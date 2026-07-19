use crate::api::translation::Translation;
use crate::managers::minecraft::modrinth::secure_path_join;
use crate::managers::vtask::VisualTask;
use anyhow::{Context, bail};
use carbon_platforms::curseforge::filters::ModFileParameters;
use carbon_platforms::modrinth::version::ModrinthEnvironmentSupport;
use carbon_rt_path::ServerPath;
use std::path::Path;
use tracing::{info, warn};

/// True if a relative path inside the server data dir points at world/save
/// data. These paths are NEVER written to during install or reinstall — we
/// will not destroy a user's world even if a malformed modpack tries to ship
/// one.
fn is_save_path(rel: &str) -> bool {
    // Accept both / and \ from zip entry names.
    let normalized = rel.replace('\\', "/");
    let first = normalized.split('/').next().unwrap_or("");
    let lower = first.to_ascii_lowercase();

    // Standard vanilla + Forge custom dimensions live at the data root:
    //   world, world_nether, world_the_end, world_<custom>
    //   DIM-1, DIM1, DIM<id> (legacy Forge)
    //   saves/ (rare on server, included for instance-style server packs)
    //   playerdata, stats, advancements (sometimes hoisted to root by plugins)
    if lower == "saves"
        || lower == "world"
        || lower.starts_with("world_")
        || lower == "playerdata"
        || lower == "stats"
        || lower == "advancements"
    {
        return true;
    }

    // Legacy Forge dimension folder: DIM<id> or DIM-<id>, where <id> is an
    // integer. Match strictly so we don't sweep in unrelated dirs like
    // "dimension-config" or "dimskin".
    if let Some(rest) = lower.strip_prefix("dim") {
        let rest = rest.strip_prefix('-').unwrap_or(rest);
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    false
}

/// True if a relative path inside the server data dir is a top-level user
/// config/state file we should preserve on reinstall. Caller should still
/// allow the write when the destination doesn't exist (fresh install).
fn is_preserved_config_file(rel: &str) -> bool {
    matches!(
        rel,
        "server.properties"
            | "eula.txt"
            | "whitelist.json"
            | "ops.json"
            | "banned-players.json"
            | "banned-ips.json"
            | "usercache.json"
            | "icon.png"
    )
}

#[derive(Debug)]
pub enum ServerModpackSource {
    Curseforge {
        project_id: u32,
        file_id: u32,
        server_pack_file_id: u32,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
}

/// Result of processing a server pack — contains detected metadata.
pub struct ServerPackResult {
    pub game_version: String,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
}

/// Process a CurseForge server pack: download the server pack ZIP and extract it.
pub async fn process_curseforge_server_pack(
    app: &crate::managers::AppInner,
    server_path: &ServerPath,
    project_id: u32,
    server_pack_file_id: u32,
    task: &VisualTask,
) -> anyhow::Result<ServerPackResult> {
    // Create all subtasks upfront so the total weight is fixed and progress
    // only moves forward. The display_name isn't known yet — use a placeholder
    // that we'll replace once we've fetched the file info.
    let t_download = task.subtask(Translation::ServerTaskDownloadServerPack {
        server_name: String::new(),
    });
    t_download.set_weight(10.0);
    t_download.start_opaque(); // Show activity while fetching file info
    let t_extract = task.subtask(Translation::ServerTaskExtractServerPack);
    t_extract.set_weight(3.0);

    // Fetch the server pack file info
    let file_info = app
        .modplatforms_manager
        .curseforge
        .get_mod_file(ModFileParameters {
            mod_id: project_id as i32,
            file_id: server_pack_file_id as i32,
        })
        .await
        .context("Failed to fetch server pack file info")?;

    let download_url = file_info
        .data
        .download_url
        .as_ref()
        .context("Server pack file has no download URL")?;

    // Update the subtask name with the actual server pack display name
    t_download.update(|data| {
        data.name = Translation::ServerTaskDownloadServerPack {
            server_name: file_info.data.display_name.clone(),
        };
    });

    info!("Downloading CurseForge server pack from {}", download_url);

    let temp_dir = std::env::temp_dir().join(format!("gdl-server-pack-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .context("Failed to create temp directory")?;
    let zip_path = temp_dir.join(&file_info.data.file_name);

    let mut request = app.reqwest_client.get(download_url);
    // CurseForge rejects unauthenticated CDN downloads; this fetch streams directly
    // instead of going through carbon_net's downloader, so attach the key here too.
    if let Some(api_key) = carbon_net::curseforge_cdn_auth(download_url) {
        request = request.header("x-api-key", api_key);
    }

    let response = request
        .send()
        .await
        .context("Failed to download server pack")?;

    if !response.status().is_success() {
        bail!("Failed to download server pack: HTTP {}", response.status());
    }

    // Stream download with progress tracking (throttled to avoid flooding WS invalidation)
    let total_size = response.content_length().map(|v| v as u32);
    match total_size {
        Some(total) => t_download.update_download(0, total, false),
        None => t_download.start_opaque(),
    }

    let mut downloaded: u32 = 0;
    let mut last_reported: u32 = 0;
    let report_threshold = total_size.map(|t| (t / 200).max(64 * 1024)).unwrap_or(0);
    let mut file = tokio::fs::File::create(&zip_path)
        .await
        .context("Failed to create temp file")?;

    use tokio::io::AsyncWriteExt;
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Error reading download chunk")?
    {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u32;
        if let Some(total) = total_size {
            if downloaded - last_reported >= report_threshold {
                t_download.update_download(downloaded, total, false);
                last_reported = downloaded;
            }
        }
    }
    file.flush().await?;
    match total_size {
        Some(_) => t_download.complete_download(),
        None => t_download.complete_opaque(),
    }

    info!(
        "Server pack downloaded ({} bytes), extracting...",
        downloaded
    );

    // --- Extract server pack (subtask already created upfront) ---
    t_extract.start_opaque();

    let data_path = server_path.get_data_path();
    tokio::fs::create_dir_all(&data_path)
        .await
        .context("Failed to create server data directory")?;

    let zip_path_clone = zip_path.clone();
    let data_path_clone = data_path.clone();
    tokio::task::spawn_blocking(move || extract_zip_to_dir(&zip_path_clone, &data_path_clone))
        .await?
        .context("Failed to extract server pack")?;
    t_extract.complete_opaque();

    // Detect game version from extracted files
    let game_version =
        detect_game_version_from_files(&data_path, &file_info.data.game_versions).await;

    // Detect modloader from extracted files
    let (modloader_type, modloader_version) = detect_modloader_from_files(&data_path).await;

    info!(
        "CurseForge server pack processed: game_version={}, modloader={:?}",
        game_version, modloader_type
    );

    Ok(ServerPackResult {
        game_version,
        modloader_type,
        modloader_version,
    })
}

/// Process a Modrinth modpack for server use: download the mrpack,
/// filter for server-compatible files, and extract.
pub async fn process_modrinth_server_pack(
    app: &crate::managers::App,
    server_path: &ServerPath,
    project_id: &str,
    version_id: &str,
    task: &VisualTask,
) -> anyhow::Result<ServerPackResult> {
    use carbon_platforms::modrinth::search::VersionID;

    // Create all subtasks upfront so the total weight is fixed from the start
    // and progress only moves forward. Use a placeholder name for now.
    let t_download_mrpack = task.subtask(Translation::ServerTaskDownloadServerPack {
        server_name: String::new(),
    });
    t_download_mrpack.set_weight(5.0);
    t_download_mrpack.start_opaque(); // Show activity while fetching version info
    let t_download_files = task.subtask(Translation::ServerTaskDownloadModpackFiles);
    t_download_files.set_weight(10.0);
    let t_extract = task.subtask(Translation::ServerTaskExtractModpackOverrides);

    // Fetch version info
    let version = app
        .modplatforms_manager
        .modrinth
        .get_version(VersionID(version_id.to_string()))
        .await
        .context("Failed to fetch Modrinth version info")?;

    // Find the primary file (the .mrpack)
    let mrpack_file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .context("Modrinth version has no files")?;

    // Update the download subtask name with the actual server name
    t_download_mrpack.update(|data| {
        data.name = Translation::ServerTaskDownloadServerPack {
            server_name: version.name.clone(),
        };
    });

    info!("Downloading Modrinth mrpack from {}", mrpack_file.url);

    let temp_dir = std::env::temp_dir().join(format!("gdl-mrpack-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .context("Failed to create temp directory")?;
    let mrpack_path = temp_dir.join(&mrpack_file.filename);

    let response = app
        .reqwest_client
        .get(&mrpack_file.url)
        .send()
        .await
        .context("Failed to download mrpack")?;

    if !response.status().is_success() {
        bail!("Failed to download mrpack: HTTP {}", response.status());
    }

    // Stream download with progress (throttled)
    let total_size = response.content_length().map(|v| v as u32);
    match total_size {
        Some(total) => t_download_mrpack.update_download(0, total, false),
        None => t_download_mrpack.start_opaque(),
    }

    let mut downloaded: u32 = 0;
    let mut last_reported: u32 = 0;
    let report_threshold = total_size.map(|t| (t / 200).max(64 * 1024)).unwrap_or(0);
    let mut file = tokio::fs::File::create(&mrpack_path)
        .await
        .context("Failed to create temp file")?;

    use tokio::io::AsyncWriteExt;
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Error reading mrpack download chunk")?
    {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u32;
        if let Some(total) = total_size {
            if downloaded - last_reported >= report_threshold {
                t_download_mrpack.update_download(downloaded, total, false);
                last_reported = downloaded;
            }
        }
    }
    file.flush().await?;
    match total_size {
        Some(_) => t_download_mrpack.complete_download(),
        None => t_download_mrpack.complete_opaque(),
    }

    info!("Mrpack downloaded ({} bytes), processing...", downloaded);

    let data_path = server_path.get_data_path();
    tokio::fs::create_dir_all(&data_path)
        .await
        .context("Failed to create server data directory")?;

    // Read the modrinth.index.json from the archive
    let mrpack_path_clone = mrpack_path.clone();
    let index: carbon_platforms::modrinth::version::ModpackIndex =
        tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let file = std::fs::File::open(&mrpack_path_clone)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let index_file = archive.by_name("modrinth.index.json")?;
            let index: carbon_platforms::modrinth::version::ModpackIndex =
                serde_json::from_reader(index_file)?;
            Ok(index)
        })
        .await?
        .context("Failed to read modrinth.index.json")?;

    // Filter files for server compatibility, and refuse anything aimed at a
    // world/save path. A well-formed mrpack should never list save data, but
    // we don't trust the input — the cost of being wrong here is a destroyed
    // user world.
    let server_files: Vec<_> = index
        .files
        .iter()
        .filter(|file| {
            if is_save_path(&file.path) {
                warn!(
                    "Skipping save-path entry from mrpack index: {} (would overwrite user data)",
                    file.path
                );
                return false;
            }
            file.env.as_ref().map_or(true, |env| {
                !matches!(env.server, ModrinthEnvironmentSupport::Unsupported)
            })
        })
        .collect();

    info!(
        "Filtered {} server-compatible files out of {} total",
        server_files.len(),
        index.files.len()
    );

    // --- Download modpack files concurrently (subtask already created upfront) ---
    let mut downloadables = Vec::with_capacity(server_files.len());
    for modpack_file in &server_files {
        let download_url = modpack_file
            .downloads
            .first()
            .context("Modpack file has no download URLs")?;

        let file_path = secure_path_join(&data_path, &modpack_file.path)?;
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        downloadables.push(carbon_net::Downloadable::new(
            download_url.as_str(),
            &file_path,
        ));
    }

    let concurrency = app
        .settings_manager()
        .get_settings()
        .await?
        .concurrent_downloads;

    let (progress_tx, mut progress_rx) = tokio::sync::watch::channel(carbon_net::Progress::new());

    t_download_files.start_opaque();

    let progress_task = tokio::spawn(async move {
        while progress_rx.changed().await.is_ok() {
            {
                let progress = progress_rx.borrow();
                t_download_files.update_download(
                    progress.current_size as u32,
                    progress.total_size as u32,
                    false,
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        t_download_files.complete_opaque();
    });

    carbon_net::download_multiple(
        &downloadables,
        carbon_net::DownloadOptions::builder()
            .concurrency(concurrency as usize)
            .progress_sender(progress_tx)
            .build(),
    )
    .await
    .context("Failed to download modpack files")?;

    let _ = progress_task.await;

    // --- Extract overrides (subtask already created upfront) ---
    t_extract.start_opaque();

    let mrpack_path_clone = mrpack_path.clone();
    let data_path_clone = data_path.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let file = std::fs::File::open(&mrpack_path_clone)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();

            // Extract from both overrides/ and server-overrides/
            let strip_prefix = if name.starts_with("server-overrides/") {
                Some("server-overrides/")
            } else if name.starts_with("overrides/") {
                Some("overrides/")
            } else {
                None
            };

            if let Some(prefix) = strip_prefix {
                let relative = &name[prefix.len()..];
                if relative.is_empty() {
                    continue;
                }

                // Same hard refusal as the CF extractor: never write into
                // world/save paths from a modpack override.
                if is_save_path(relative) {
                    tracing::warn!(
                        "Skipping save-path entry from mrpack overrides: {} (would overwrite user data)",
                        relative
                    );
                    continue;
                }

                let out_path = match secure_path_join(&data_path_clone, relative) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(
                            "Skipping mrpack override entry with unsafe path `{}`: {}",
                            relative,
                            e
                        );
                        continue;
                    }
                };

                if !entry.is_dir()
                    && is_preserved_config_file(relative)
                    && out_path.exists()
                {
                    continue;
                }

                if entry.is_dir() {
                    std::fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let mut outfile = std::fs::File::create(&out_path)?;
                    std::io::copy(&mut entry, &mut outfile)?;
                }
            }
        }

        Ok(())
    })
    .await?
    .context("Failed to extract overrides from mrpack")?;
    t_extract.complete_opaque();

    // Extract game version and modloader from index dependencies
    let game_version = index.dependencies.minecraft.clone().unwrap_or_else(|| {
        version
            .game_versions
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string())
    });

    let (modloader_type, modloader_version) = if let Some(ref v) = index.dependencies.forge {
        (Some("forge".to_string()), Some(v.clone()))
    } else if let Some(ref v) = index.dependencies.neoforge {
        (Some("neoforge".to_string()), Some(v.clone()))
    } else if let Some(ref v) = index.dependencies.fabric_loader {
        (Some("fabric".to_string()), Some(v.clone()))
    } else if let Some(ref v) = index.dependencies.quilt_loader {
        (Some("quilt".to_string()), Some(v.clone()))
    } else {
        (None, None)
    };

    info!(
        "Modrinth server pack processed: game_version={}, modloader={:?}",
        game_version, modloader_type
    );

    Ok(ServerPackResult {
        game_version,
        modloader_type,
        modloader_version,
    })
}

/// Extract a ZIP file into a target directory, handling single-root-folder layouts.
fn extract_zip_to_dir(zip_path: &Path, target_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Check if all entries share a single root directory
    let mut root_dirs = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name();
        if let Some(first_component) = name.split('/').next() {
            if !first_component.is_empty() {
                root_dirs.insert(first_component.to_string());
            }
        }
    }

    let strip_root = if root_dirs.len() == 1 {
        let root = root_dirs.into_iter().next().unwrap();
        let has_subfiles = (0..archive.len()).any(|i| {
            archive
                .by_index(i)
                .map(|e| e.name().starts_with(&format!("{}/", root)) && !e.is_dir())
                .unwrap_or(false)
        });
        if has_subfiles {
            Some(format!("{}/", root))
        } else {
            None
        }
    } else {
        None
    };

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        let relative = if let Some(ref prefix) = strip_root {
            if let Some(stripped) = name.strip_prefix(prefix.as_str()) {
                stripped.to_string()
            } else {
                continue;
            }
        } else {
            name.clone()
        };

        if relative.is_empty() {
            continue;
        }

        // Hard refuse to write into world/save paths under any circumstances.
        // Even a fresh install loses nothing — modpack-bundled worlds are
        // niche and the risk to user data on reinstall isn't worth it.
        if is_save_path(&relative) {
            warn!(
                "Skipping save-path entry from server pack: {} (would overwrite user data)",
                relative
            );
            continue;
        }

        let out_path = match secure_path_join(target_dir, &relative) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Skipping server pack entry with unsafe path `{}`: {}",
                    relative, e
                );
                continue;
            }
        };

        // Don't clobber user-customized config files if they already exist
        // (server.properties, eula.txt, op/whitelist/banned lists, icon).
        if !entry.is_dir() && is_preserved_config_file(&relative) && out_path.exists() {
            continue;
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }

    Ok(())
}

/// Detect game version from CurseForge file metadata.
async fn detect_game_version_from_files(_data_path: &Path, cf_game_versions: &[String]) -> String {
    for version in cf_game_versions {
        if version.contains('.')
            && !version.contains('-')
            && !version.to_lowercase().contains("forge")
        {
            return version.clone();
        }
    }

    cf_game_versions
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

/// Detect modloader type from extracted server pack files.
async fn detect_modloader_from_files(data_path: &Path) -> (Option<String>, Option<String>) {
    let fabric_jar = data_path.join("fabric-server-launch.jar");
    if fabric_jar.exists() {
        return (Some("fabric".to_string()), None);
    }

    if let Ok(mut entries) = tokio::fs::read_dir(data_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("neoforge-") && name.ends_with(".jar") {
                let version = name
                    .strip_prefix("neoforge-")
                    .and_then(|s| s.strip_suffix(".jar"))
                    .map(|s| s.to_string());
                return (Some("neoforge".to_string()), version);
            }
        }
    }

    if let Ok(mut entries) = tokio::fs::read_dir(data_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("forge-") && name.ends_with(".jar") {
                let version = name
                    .strip_prefix("forge-")
                    .and_then(|s| s.strip_suffix(".jar"))
                    .map(|s| s.strip_suffix("-installer").unwrap_or(s))
                    .map(|s| s.to_string());
                return (Some("forge".to_string()), version);
            }
        }
    }

    let quilt_jar = data_path.join("quilt-server-launch.jar");
    if quilt_jar.exists() {
        return (Some("quilt".to_string()), None);
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_path_protection_covers_world_dirs() {
        // Vanilla worlds and Forge custom dimensions
        assert!(is_save_path("world"));
        assert!(is_save_path("world/level.dat"));
        assert!(is_save_path("world_nether"));
        assert!(is_save_path("world_nether/region/r.0.0.mca"));
        assert!(is_save_path("world_the_end"));
        assert!(is_save_path("world_aether/level.dat"));
        assert!(is_save_path("world_twilightforest/level.dat"));
        assert!(is_save_path("world_my_custom_dim_42/level.dat"));

        // Legacy Forge dimension folders at the data root
        assert!(is_save_path("DIM-1"));
        assert!(is_save_path("DIM1/region/r.0.0.mca"));
        assert!(is_save_path("dim-1/region/r.0.0.mca"));

        // Instance-style save folder (rare on server but cheap to cover)
        assert!(is_save_path("saves"));
        assert!(is_save_path("saves/myworld/level.dat"));

        // Plugin-hoisted player data
        assert!(is_save_path("playerdata/uuid.dat"));
        assert!(is_save_path("stats/uuid.json"));
        assert!(is_save_path("advancements/uuid.json"));

        // Backslash separators (zip entries on Windows-authored packs)
        assert!(is_save_path("world\\level.dat"));
    }

    #[test]
    fn save_path_protection_does_not_overmatch() {
        // These look superficially close but must NOT be classified as saves
        assert!(!is_save_path("mods/somemod.jar"));
        assert!(!is_save_path("libraries/foo.jar"));
        assert!(!is_save_path("config/somemod.toml"));
        assert!(!is_save_path("defaultconfigs/foo.toml"));
        assert!(!is_save_path("server.jar"));
        assert!(!is_save_path("server.properties"));
        assert!(!is_save_path("eula.txt"));
        assert!(!is_save_path("whitelist.json"));
        // "dim" alone is not a dimension folder
        assert!(!is_save_path("dim"));
        // Non-DIM names that happen to start with d/i/m are not saves
        assert!(!is_save_path("dimension-config"));
    }

    #[test]
    fn preserved_configs_are_listed() {
        assert!(is_preserved_config_file("server.properties"));
        assert!(is_preserved_config_file("eula.txt"));
        assert!(is_preserved_config_file("whitelist.json"));
        assert!(is_preserved_config_file("ops.json"));
        assert!(is_preserved_config_file("banned-players.json"));
        assert!(is_preserved_config_file("banned-ips.json"));
        assert!(is_preserved_config_file("usercache.json"));
        assert!(is_preserved_config_file("icon.png"));

        // Nested files are not matched — caller passes top-level relative paths
        assert!(!is_preserved_config_file("config/server.properties"));
        assert!(!is_preserved_config_file("mods/foo.jar"));
    }
}
