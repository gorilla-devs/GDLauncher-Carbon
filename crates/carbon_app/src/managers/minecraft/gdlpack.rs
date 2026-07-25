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

use super::modrinth::{
    MAX_EXTRACTED_OVERRIDE_BYTES, MAX_HASHED_ENTRY_BYTES, copy_bounded, is_symlink_mode,
    secure_path_join,
};

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

/// Picks the hash a resolved file's download is verified against.
///
/// `platform_sha1` describes exactly what the platform's CDN will serve, so it
/// wins when present. It often isn't: CurseForge's fingerprint results carry a
/// SHA-1 only sometimes, and a gdlpack written by another launcher's exporter
/// need not populate `sha1` at all, since Modrinth resolution only needs
/// `sha512` and CurseForge only needs `murmur2`. Each remaining digest is
/// therefore taken from the manifest in strength order, and an absent hash
/// yields `None` — attaching an empty one would fail every download it was
/// meant to protect, with a mismatch no retry could clear.
fn download_checksum(platform_sha1: &str, manifest: &FileHashes) -> Option<carbon_net::Checksum> {
    if !platform_sha1.is_empty() {
        Some(carbon_net::Checksum::Sha1(platform_sha1.to_string()))
    } else if !manifest.sha512.is_empty() {
        Some(carbon_net::Checksum::Sha512(manifest.sha512.clone()))
    } else if !manifest.sha1.is_empty() {
        Some(carbon_net::Checksum::Sha1(manifest.sha1.clone()))
    } else {
        None
    }
}

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
            // Find the file that matches our hash. `sha512` is the key Modrinth
            // echoes back from the query, so it carries the gdlpack manifest's
            // casing, while `file.hashes.sha512` carries Modrinth's own — compare
            // the two without regard to hex case.
            for file in &version.files {
                if file.hashes.sha512.eq_ignore_ascii_case(&sha512) {
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
    // Tracks a platform API call that errored (as opposed to a successful "not found"), so a
    // transient outage is surfaced as retryable instead of being misreported as a missing file.
    let mut platform_error: Option<anyhow::Error> = None;

    if platform_files.is_empty() {
        debug!("No platform files to resolve");
    } else {
        // Update progress - starting resolution
        progress_sender.send_modify(|state| {
            *state = ProgressState::ResolvingFiles(0, total_files);
        });

        // Resolve from Modrinth first — the primary source.
        debug!("Resolving {} files from Modrinth", platform_files.len());
        let modrinth_results = match batch_resolve_modrinth(app, &platform_files).await {
            Ok(results) => {
                debug!("Modrinth resolved {} files", results.len());
                results
            }
            Err(e) => {
                warn!("Modrinth batch resolution failed: {}", e);
                platform_error = Some(e.context("Modrinth file resolution failed"));
                HashMap::new()
            }
        };

        // Update progress - Modrinth done
        progress_sender.send_modify(|state| {
            *state = ProgressState::ResolvingFiles(total_files / 2, total_files);
        });

        // Collect the files Modrinth didn't resolve, to look up on CurseForge.
        let not_in_modrinth: Vec<FileHashes> = platform_files
            .iter()
            .filter(|h| !modrinth_results.contains_key(&h.sha512))
            .cloned()
            .collect();

        // Resolve the remainder from CurseForge.
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
                    if platform_error.is_none() {
                        platform_error = Some(e.context("CurseForge file resolution failed"));
                    }
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

        // Build downloadables from the resolved files.
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

                    let downloadable = Downloadable::new(&file.download_url, target_path)
                        .with_checksum(download_checksum(&file.sha1, hashes))
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
        // Scan the overrides first: a file bundled in overrides must install even when a
        // platform API errored, since it never needed the platform. Only a file that is on
        // neither platform AND missing from the overrides is a real failure (handled per-file
        // below, where a platform error is surfaced as retryable).
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

                        // Read file and compute SHA512. Bounded per-entry (not
                        // cumulative -- `contents` is dropped at the end of each
                        // iteration, so there is nothing to accumulate): a
                        // decompression bomb masquerading as an override must not
                        // be read into memory without limit just to hash it, and an
                        // entry over the limit can never legitimately be the small
                        // unresolved file we are matching against anyway.
                        let mut contents = Vec::new();
                        if copy_bounded(&mut entry, &mut contents, MAX_HASHED_ENTRY_BYTES).is_err()
                        {
                            continue;
                        }

                        use sha2::{Digest, Sha512};
                        let hash = Sha512::digest(&contents);
                        let hash_hex = hex::encode(hash);

                        // `hash_hex` is lowercase by construction; `hashes.sha512`
                        // is whatever the gdlpack manifest declared.
                        if hash_hex.eq_ignore_ascii_case(&hashes.sha512) {
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
                // On neither platform and not in the overrides. If a platform API errored, the
                // file may resolve once it recovers, so surface the retryable cause; otherwise
                // it is genuinely missing from the pack.
                if let Some(err) = platform_error.take() {
                    return Err(err.context(
                        "A platform API was temporarily unavailable while resolving modpack files; please retry",
                    ));
                }
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
                // Cumulative across the whole pass, not per entry: without this,
                // many entries each just under a per-entry cap could still add up
                // to an unbounded amount written to disk.
                let mut extracted_bytes: u64 = 0;

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

                        if is_symlink_mode(entry.unix_mode()) {
                            // A symlink entry materialized as a regular file would end
                            // up containing the link's target path text instead of
                            // real data.
                            tracing::warn!(
                                "Skipping gdlpack override entry `{}`: symlinks are not extracted",
                                rel_path
                            );
                            continue;
                        }

                        let target = match secure_path_join(&instance_data_path, &rel_path) {
                            Ok(p) => p,
                            Err(e) => {
                                tracing::warn!(
                                    "Skipping gdlpack override entry with unsafe path `{}`: {}",
                                    rel_path,
                                    e
                                );
                                continue;
                            }
                        };

                        if let Some(parent) = target.parent() {
                            std::fs::create_dir_all(parent)?;
                        }

                        let mut outfile = std::fs::File::create(&target)?;
                        let remaining_budget =
                            MAX_EXTRACTED_OVERRIDE_BYTES.saturating_sub(extracted_bytes);
                        extracted_bytes +=
                            copy_bounded(&mut entry, &mut outfile, remaining_budget)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_hashes(sha512: &str, sha1: &str) -> FileHashes {
        FileHashes {
            sha512: sha512.to_string(),
            sha1: sha1.to_string(),
            murmur2: 1,
        }
    }

    #[test]
    fn the_platform_sha1_wins_when_the_platform_supplied_one() {
        match download_checksum("platform-sha1", &manifest_hashes("mr-sha512", "mr-sha1")) {
            Some(carbon_net::Checksum::Sha1(hash)) => assert_eq!(hash, "platform-sha1"),
            other => panic!("expected the platform SHA-1, got {other:?}"),
        }
    }

    #[test]
    fn the_manifest_sha512_covers_a_platform_that_reported_no_sha1() {
        // CurseForge fingerprint matches carry a SHA-1 only sometimes.
        match download_checksum("", &manifest_hashes("mr-sha512", "mr-sha1")) {
            Some(carbon_net::Checksum::Sha512(hash)) => assert_eq!(hash, "mr-sha512"),
            other => panic!("expected the manifest SHA-512, got {other:?}"),
        }
    }

    #[test]
    fn the_manifest_sha1_is_the_last_resort() {
        match download_checksum("", &manifest_hashes("", "mr-sha1")) {
            Some(carbon_net::Checksum::Sha1(hash)) => assert_eq!(hash, "mr-sha1"),
            other => panic!("expected the manifest SHA-1, got {other:?}"),
        }
    }

    /// A foreign exporter may populate only `murmur2`, which resolves on
    /// CurseForge without any digest. Verification is then skipped rather than
    /// run against an empty hash, which would fail every such download.
    #[test]
    fn a_file_with_no_digest_anywhere_is_left_unverified() {
        assert!(download_checksum("", &manifest_hashes("", "")).is_none());
    }
}
