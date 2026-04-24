use crate::managers::App;
use crate::managers::instance::modpack::packinfo::PackInfo;
use anyhow::{Context, anyhow};
use carbon_net::Downloadable;
use carbon_platforms::gdlauncher::manifest::schema::v1::{
    FileHashes, Manifest, ModloaderType, PackFile,
};
use carbon_platforms::modrinth::search::VersionHashesQuery;
use carbon_platforms::modrinth::version::HashAlgorithm;
use carbon_rt_path::InstancePath;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, trace, warn};

use super::modrinth::secure_path_join;

use crate::domain::instance::info::{ModLoader, ModLoaderType, StandardVersion};

#[derive(Debug, Copy, Clone)]
pub enum ProgressState {
    Idle,
    ResolvingFiles(u64, u64),
    ExtractingOverrides(u64, u64),
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub manifest: Manifest,
    pub version: StandardVersion,
    pub downloadables: Vec<(Downloadable, Option<String>)>,
}

/// Resolved file info from platform lookup
struct ResolvedFile {
    download_url: String,
    relative_path: String,
    sha1: String,
    size: u64,
}

/// Batch size for API calls (both platforms support up to 1000)
const BATCH_SIZE: usize = 1000;

/// Batch resolve files from Modrinth using SHA512 hashes
/// Returns a map of SHA512 -> (download_url, relative_path)
async fn batch_resolve_modrinth(
    app: &App,
    hashes: &[FileHashes],
) -> anyhow::Result<HashMap<String, ResolvedFile>> {
    let mut results = HashMap::new();

    for chunk in hashes.chunks(BATCH_SIZE) {
        let sha512_hashes: Vec<String> = chunk.iter().map(|h| h.sha512.clone()).collect();

        let versions = app
            .modplatforms_manager()
            .modrinth
            .get_versions_from_hash(&VersionHashesQuery {
                hashes: sha512_hashes,
                algorithm: HashAlgorithm::SHA512,
            })
            .await?;

        for (sha512, version) in versions {
            // Find the file that matches our hash
            for file in &version.files {
                if file.hashes.sha512 == sha512 {
                    let relative_path = format!("mods/{}", file.filename);
                    results.insert(
                        sha512.clone(),
                        ResolvedFile {
                            download_url: file.url.to_string(),
                            relative_path,
                            sha1: file.hashes.sha1.clone(),
                            size: file.size as u64,
                        },
                    );
                    break;
                }
            }
        }
    }

    Ok(results)
}

/// Batch resolve files from CurseForge using murmur2 fingerprints
/// Returns a map of murmur2 -> (download_url, relative_path, sha1)
async fn batch_resolve_curseforge(
    app: &App,
    hashes: &[FileHashes],
) -> anyhow::Result<HashMap<u32, ResolvedFile>> {
    let mut results = HashMap::new();

    for chunk in hashes.chunks(BATCH_SIZE) {
        let fingerprints: Vec<u32> = chunk.iter().map(|h| h.murmur2).collect();

        let matches = app
            .modplatforms_manager()
            .curseforge
            .get_fingerprints(&fingerprints)
            .await?;

        for exact_match in matches.data.exact_matches {
            if let Some(download_url) = &exact_match.file.download_url {
                // Use file_fingerprint as the key since that's what we looked up by
                results.insert(
                    exact_match.file.file_fingerprint,
                    ResolvedFile {
                        download_url: download_url.clone(),
                        relative_path: format!("mods/{}", exact_match.file.file_name),
                        // CurseForge doesn't always provide SHA1, use empty string as fallback
                        sha1: exact_match
                            .file
                            .hashes
                            .iter()
                            .find(|h| {
                                matches!(h.algo, carbon_platforms::curseforge::HashAlgo::Sha1)
                            })
                            .map(|h| h.value.clone())
                            .unwrap_or_default(),
                        size: exact_match.file.file_length as u64,
                    },
                );
            }
        }
    }

    Ok(results)
}

pub async fn prepare_modpack_from_gdlpack(
    app: &App,
    gdlpack_path: &Path,
    instance_path: &InstancePath,
    skip_overrides: bool,
    existing_packinfo: Option<&PackInfo>,
    progress_sender: watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    debug!("Preparing GDLPack modpack from {:?}", gdlpack_path);

    // Parse the manifest
    let manifest = tokio::task::spawn_blocking({
        let gdlpack_path = gdlpack_path.to_path_buf();
        move || {
            let file = std::fs::File::open(&gdlpack_path)?;
            let mut archive = zip::ZipArchive::new(file)?;

            let mut manifest_file = archive
                .by_name("gdlpack.json")
                .context("Missing gdlpack.json manifest")?;

            let mut manifest_data = Vec::new();
            manifest_file.read_to_end(&mut manifest_data)?;

            let manifest: Manifest =
                serde_json::from_slice(&manifest_data).context("Failed to parse gdlpack.json")?;

            Ok::<_, anyhow::Error>(manifest)
        }
    })
    .await??;

    // Collect all platform files that need resolution
    let platform_files: Vec<FileHashes> = manifest
        .entries
        .iter()
        .filter_map(|file| match file {
            PackFile::Platform(pf) => Some(pf.hashes.clone()),
            PackFile::Optional(_) => None,
        })
        .collect();

    let total_files = platform_files.len() as u64;
    let mut downloadables: Vec<(Downloadable, Option<String>)> = Vec::new();
    let mut unresolved_platform_files: Vec<FileHashes> = Vec::new();

    if platform_files.is_empty() {
        debug!("No platform files to resolve");
    } else {
        // Update progress - starting resolution
        progress_sender.send_modify(|state| {
            *state = ProgressState::ResolvingFiles(0, total_files);
        });

        // Step 1: Batch resolve from Modrinth (primary source)
        debug!("Resolving {} files from Modrinth", platform_files.len());
        let modrinth_results = match batch_resolve_modrinth(app, &platform_files).await {
            Ok(results) => {
                debug!("Modrinth resolved {} files", results.len());
                results
            }
            Err(e) => {
                warn!("Modrinth batch resolution failed: {}", e);
                HashMap::new()
            }
        };

        // Update progress - Modrinth done
        progress_sender.send_modify(|state| {
            *state = ProgressState::ResolvingFiles(total_files / 2, total_files);
        });

        // Step 2: Collect files not found in Modrinth for CurseForge lookup
        let not_in_modrinth: Vec<FileHashes> = platform_files
            .iter()
            .filter(|h| !modrinth_results.contains_key(&h.sha512))
            .cloned()
            .collect();

        // Step 3: Batch resolve remaining from CurseForge
        let curseforge_results = if !not_in_modrinth.is_empty() {
            debug!(
                "Resolving {} remaining files from CurseForge",
                not_in_modrinth.len()
            );
            match batch_resolve_curseforge(app, &not_in_modrinth).await {
                Ok(results) => {
                    debug!("CurseForge resolved {} files", results.len());
                    results
                }
                Err(e) => {
                    warn!("CurseForge batch resolution failed: {}", e);
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        // Update progress - resolution complete
        progress_sender.send_modify(|state| {
            *state = ProgressState::ResolvingFiles(total_files, total_files);
        });

        // Step 4: Build downloadables from resolved files
        for hashes in &platform_files {
            // Try Modrinth first, then CurseForge
            let resolved = modrinth_results
                .get(&hashes.sha512)
                .or_else(|| curseforge_results.get(&hashes.murmur2));

            match resolved {
                Some(file) => {
                    let target_path =
                        secure_path_join(instance_path.get_data_path(), &file.relative_path)?;

                    // Check if file already exists in packinfo (for updates)
                    let skip_path = existing_packinfo.and_then(|pi| {
                        let search_path = format!("/{}", file.relative_path);
                        pi.files
                            .iter()
                            .find(|(p, _)| **p == search_path)
                            .map(|(p, _)| p.clone())
                    });

                    // Prefer SHA1 from platform, fall back to manifest
                    let sha1 = if !file.sha1.is_empty() {
                        file.sha1.clone()
                    } else {
                        hashes.sha1.clone()
                    };

                    let downloadable = Downloadable::new(&file.download_url, target_path)
                        .with_checksum(Some(carbon_net::Checksum::Sha1(sha1)))
                        .with_size(file.size);

                    downloadables.push((downloadable, skip_path));
                }
                None => {
                    // File not found on either platform - will check overrides
                    warn!(
                        "Platform resolution failed for SHA512: {}, will check overrides",
                        hashes.sha512
                    );
                    unresolved_platform_files.push(hashes.clone());
                }
            }
        }
    }

    // Verify that unresolved platform files exist in overrides
    if !unresolved_platform_files.is_empty() {
        let found_in_overrides = tokio::task::spawn_blocking({
            let gdlpack_path = gdlpack_path.to_path_buf();
            let overrides_dir = manifest.overrides.clone();
            let client_overrides = manifest.client_overrides.clone();
            let unresolved = unresolved_platform_files.clone();

            move || -> anyhow::Result<Vec<bool>> {
                let file = std::fs::File::open(&gdlpack_path)?;
                let mut archive = zip::ZipArchive::new(file)?;

                let override_prefix = format!("{}/", overrides_dir);
                let client_override_prefix = client_overrides
                    .as_ref()
                    .map(|d| format!("{}/", d))
                    .unwrap_or_default();

                let mut results = Vec::with_capacity(unresolved.len());

                for hashes in &unresolved {
                    let mut found = false;

                    // Scan override files for matching SHA512
                    for i in 0..archive.len() {
                        let mut entry = match archive.by_index(i) {
                            Ok(e) => e,
                            Err(_) => continue,
                        };
                        let entry_name = entry.name().to_string();

                        // Check if it's an override file
                        let is_override = entry_name.starts_with(&override_prefix)
                            || (!client_override_prefix.is_empty()
                                && entry_name.starts_with(&client_override_prefix));

                        if !is_override || entry.is_dir() {
                            continue;
                        }

                        // Read file and compute SHA512
                        let mut contents = Vec::new();
                        if entry.read_to_end(&mut contents).is_err() {
                            continue;
                        }

                        use sha2::{Digest, Sha512};
                        let hash = Sha512::digest(&contents);
                        let hash_hex = hex::encode(hash);

                        if hash_hex == hashes.sha512 {
                            found = true;
                            trace!(
                                "Found unresolved platform file in overrides: {} (SHA512: {})",
                                entry_name, hashes.sha512
                            );
                            break;
                        }
                    }

                    results.push(found);
                }

                Ok(results)
            }
        })
        .await??;

        // Check if any unresolved files were NOT found in overrides
        for (i, found) in found_in_overrides.iter().enumerate() {
            if !found {
                return Err(anyhow!(
                    "Required file could not be resolved from platforms or found in overrides. SHA512: {}",
                    unresolved_platform_files[i].sha512
                ));
            }
        }

        debug!(
            "{} platform files will be extracted from overrides instead of downloading",
            unresolved_platform_files.len()
        );
    }

    // Extract overrides
    if !skip_overrides {
        tokio::task::spawn_blocking({
            let gdlpack_path = gdlpack_path.to_path_buf();
            let instance_data_path = instance_path.get_data_path().to_path_buf();
            let overrides_dir = manifest.overrides.clone();
            let client_overrides = manifest.client_overrides.clone();
            let progress_sender = progress_sender.clone();

            move || -> anyhow::Result<()> {
                let file = std::fs::File::open(&gdlpack_path)?;
                let mut archive = zip::ZipArchive::new(file)?;

                // Count override files
                let override_prefix = format!("{}/", overrides_dir);
                let client_override_prefix = client_overrides
                    .as_ref()
                    .map(|d| format!("{}/", d))
                    .unwrap_or_default();

                let override_count = (0..archive.len())
                    .filter(|&i| {
                        if let Ok(entry) = archive.by_index(i) {
                            let name = entry.name();
                            name.starts_with(&override_prefix)
                                || (!client_override_prefix.is_empty()
                                    && name.starts_with(&client_override_prefix))
                        } else {
                            false
                        }
                    })
                    .count() as u64;

                let mut extracted = 0u64;

                // Extract main overrides
                for i in 0..archive.len() {
                    let mut entry = archive.by_index(i)?;
                    let entry_name = entry.name().to_string();

                    let relative_path = if entry_name.starts_with(&override_prefix) {
                        entry_name
                            .strip_prefix(&override_prefix)
                            .map(|s| s.to_string())
                    } else if !client_override_prefix.is_empty()
                        && entry_name.starts_with(&client_override_prefix)
                    {
                        // Client overrides (for client-side installation)
                        entry_name
                            .strip_prefix(&client_override_prefix)
                            .map(|s| s.to_string())
                    } else {
                        None
                    };

                    if let Some(rel_path) = relative_path {
                        if rel_path.is_empty() || entry.is_dir() {
                            continue;
                        }

                        let target = instance_data_path.join(&rel_path);

                        if let Some(parent) = target.parent() {
                            std::fs::create_dir_all(parent)?;
                        }

                        let mut outfile = std::fs::File::create(&target)?;
                        std::io::copy(&mut entry, &mut outfile)?;

                        extracted += 1;
                        progress_sender.send_modify(|state| {
                            *state = ProgressState::ExtractingOverrides(extracted, override_count);
                        });
                    }
                }

                Ok(())
            }
        })
        .await??;
    }

    // Build StandardVersion from manifest
    let modloaders: HashSet<ModLoader> = manifest
        .dependencies
        .modloaders
        .iter()
        .map(|ml| ModLoader {
            type_: match ml.type_ {
                ModloaderType::Forge => ModLoaderType::Forge,
                ModloaderType::Neoforge => ModLoaderType::Neoforge,
                ModloaderType::Fabric => ModLoaderType::Fabric,
                ModloaderType::Quilt => ModLoaderType::Quilt,
            },
            version: ml.version.clone(),
        })
        .collect();

    let version = StandardVersion {
        release: manifest.dependencies.minecraft.clone(),
        modloaders,
    };

    Ok(ModpackInfo {
        manifest,
        version,
        downloadables,
    })
}
