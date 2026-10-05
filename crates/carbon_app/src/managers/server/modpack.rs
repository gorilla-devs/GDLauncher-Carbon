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
pub(super) fn is_save_path(rel: &str) -> bool {
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
    cancel: &tokio_util::sync::CancellationToken,
) -> anyhow::Result<ServerPackResult> {
    anyhow::ensure!(!cancel.is_cancelled(), "Server pack preparation cancelled");
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
    let file_info = crate::managers::runtime_lifecycle::wait_or_cancel(cancel, async {
        app.modplatforms_manager
            .curseforge
            .get_mod_file(ModFileParameters {
                mod_id: project_id as i32,
                file_id: server_pack_file_id as i32,
            })
            .await
            .context("Failed to fetch server pack file info")
    })
    .await??;

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

    anyhow::ensure!(!cancel.is_cancelled(), "Server archive download cancelled");
    let temp_dir = tempfile::Builder::new()
        .prefix("gdl-server-pack-")
        .tempdir()?;
    let zip_path = temp_dir.path().join(&file_info.data.file_name);

    let mut request = app.reqwest_client.get(download_url);
    // CurseForge rejects unauthenticated CDN downloads; this fetch streams directly
    // instead of going through carbon_net's downloader, so attach the key here too.
    if let Some(api_key) = carbon_net::curseforge_cdn_auth(download_url) {
        request = request.header("x-api-key", api_key);
    }

    let response = crate::managers::runtime_lifecycle::wait_or_cancel(cancel, async {
        request
            .send()
            .await
            .context("Failed to download server pack")
    })
    .await??;

    if !response.status().is_success() {
        bail!("Failed to download server pack: HTTP {}", response.status());
    }

    // Stream download with progress tracking (throttled to avoid flooding WS invalidation)
    let total_size = response.content_length();
    let mut progress = ArchiveProgress::new(&t_download, total_size);
    let downloaded: u64;
    let mut file = tokio::fs::File::create(&zip_path)
        .await
        .context("Failed to create temp file")?;

    let mut stream = response.bytes_stream();
    downloaded = copy_archive_stream(&mut stream, &mut file, cancel, |bytes| {
        progress.report(bytes);
    })
    .await?;
    drop(file);
    progress.finish(downloaded);

    info!(
        "Server pack downloaded ({} bytes), extracting...",
        downloaded
    );

    anyhow::ensure!(!cancel.is_cancelled(), "Server pack extraction cancelled");
    // --- Extract server pack (subtask already created upfront) ---
    t_extract.start_opaque();

    let data_path = server_path.get_data_path();
    tokio::fs::create_dir_all(&data_path)
        .await
        .context("Failed to create server data directory")?;

    let zip_path_clone = zip_path.clone();
    let data_path_clone = data_path.clone();
    let skipped_links =
        crate::managers::runtime_lifecycle::run_owned_phase(cancel, || async move {
            tokio::task::spawn_blocking(move || {
                extract_zip_to_dir(&zip_path_clone, &data_path_clone)
            })
            .await?
            .context("Failed to extract server pack")
        })
        .await?;
    warn_skipped_links(task, skipped_links);
    t_extract.complete_opaque();

    anyhow::ensure!(
        !cancel.is_cancelled(),
        "Server pack preparation cancelled after extraction settled"
    );
    // Detect game version from extracted files
    let game_version =
        detect_game_version_from_files(&data_path, &file_info.data.game_versions).await;

    // Detect modloader from extracted files
    let (modloader_type, modloader_version) = detect_modloader_from_files(&data_path).await?;

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
    cancel: &tokio_util::sync::CancellationToken,
) -> anyhow::Result<ServerPackResult> {
    use carbon_platforms::modrinth::search::VersionID;

    anyhow::ensure!(!cancel.is_cancelled(), "Server pack preparation cancelled");
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
    let version = crate::managers::runtime_lifecycle::wait_or_cancel(cancel, async {
        app.modplatforms_manager
            .modrinth
            .get_version(VersionID(version_id.to_string()))
            .await
            .context("Failed to fetch Modrinth version info")
    })
    .await??;

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

    anyhow::ensure!(!cancel.is_cancelled(), "Server archive download cancelled");
    let temp_dir = tempfile::Builder::new().prefix("gdl-mrpack-").tempdir()?;
    let mrpack_path = temp_dir.path().join(&mrpack_file.filename);

    let response = crate::managers::runtime_lifecycle::wait_or_cancel(cancel, async {
        app.reqwest_client
            .get(&mrpack_file.url)
            .send()
            .await
            .context("Failed to download mrpack")
    })
    .await??;

    if !response.status().is_success() {
        bail!("Failed to download mrpack: HTTP {}", response.status());
    }

    // Stream download with progress (throttled)
    let total_size = response.content_length();
    let mut progress = ArchiveProgress::new(&t_download_mrpack, total_size);
    let downloaded: u64;
    let mut file = tokio::fs::File::create(&mrpack_path)
        .await
        .context("Failed to create temp file")?;

    let mut stream = response.bytes_stream();
    downloaded = copy_archive_stream(&mut stream, &mut file, cancel, |bytes| {
        progress.report(bytes);
    })
    .await?;
    drop(file);
    progress.finish(downloaded);

    info!("Mrpack downloaded ({} bytes), processing...", downloaded);

    let data_path = server_path.get_data_path();
    tokio::fs::create_dir_all(&data_path)
        .await
        .context("Failed to create server data directory")?;

    anyhow::ensure!(
        !cancel.is_cancelled(),
        "Server pack index preparation cancelled"
    );
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

    anyhow::ensure!(
        !cancel.is_cancelled(),
        "Server pack preparation cancelled after index read settled"
    );
    // Filter files for server compatibility, and refuse anything aimed at a
    // world/save path. A well-formed mrpack should never list save data, but
    // we don't trust the input — the cost of being wrong here is a destroyed
    // user world.
    // One walk serves the index and the overrides (see `SaveProtection::read`).
    let protection = {
        let data_path = data_path.clone();
        tokio::task::spawn_blocking(move || super::world::SaveProtection::read(&data_path))
            .await??
    };
    let mut server_files = Vec::new();
    let mut destinations =
        super::world::Destinations::new(&data_path, super::world::filesystem_resolves);
    for file in &index.files {
        if file
            .env
            .as_ref()
            .is_some_and(|env| matches!(env.server, ModrinthEnvironmentSupport::Unsupported))
        {
            continue;
        }
        if let Some(destination) =
            pack_destination(&protection, &mut destinations, &file.path, false)?
        {
            server_files.push((file, destination));
        }
    }
    let mut skipped_links = destinations.into_skipped_links();
    info!(
        "Filtered {} server-compatible files out of {} total",
        server_files.len(),
        index.files.len()
    );
    let mut downloadables = Vec::with_capacity(server_files.len());
    for (modpack_file, file_path) in &server_files {
        let download_url = modpack_file
            .downloads
            .first()
            .context("Modpack file has no download URLs")?;
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        downloadables.push(mrpack_downloadable(modpack_file, download_url, file_path));
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
                t_download_files.update_download_progress(&progress, false);
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        t_download_files.complete_opaque();
    });

    let download_result = carbon_net::download_multiple(
        &downloadables,
        carbon_net::DownloadOptions::builder()
            .cancel_token(cancel.clone())
            .concurrency(concurrency as usize)
            .progress_sender(progress_tx)
            .build(),
    )
    .await
    .context("Failed to download modpack files");
    progress_task.await?;
    download_result?;
    anyhow::ensure!(!cancel.is_cancelled(), "Server pack preparation cancelled");

    // --- Extract overrides (subtask already created upfront) ---
    t_extract.start_opaque();

    let mrpack_path_clone = mrpack_path.clone();
    let data_path_clone = data_path.clone();
    skipped_links.extend(
        tokio::task::spawn_blocking(move || {
            extract_mrpack_overrides_protected(
                &mrpack_path_clone,
                &data_path_clone,
                &protection,
                super::world::filesystem_resolves,
            )
        })
        .await?
        .context("Failed to extract overrides from mrpack")?,
    );
    warn_skipped_links(task, skipped_links);
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

/// Where a pack entry goes, or `None` when it must not be written: a world or
/// a folder the save check could not open (see
/// [`super::world::SaveProtection`]), the operator's own files, `eula.txt`,
/// or a path through a link, one the filesystem cannot place or a name this
/// OS cannot store (`:` on Windows). The last are skipped with a warning, as
/// 2.0.40 skipped unsafe paths, so one odd entry does not fail a whole
/// install; a link is also recorded in `destinations` to tell the user (see
/// [`warn_skipped_links`]). A path that leaves the server folder is an
/// error; the caller decides whether that fails the install.
fn pack_destination(
    protection: &super::world::SaveProtection,
    destinations: &mut super::world::Destinations<'_>,
    raw: &str,
    is_dir: bool,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    let data = destinations.data();
    super::world::contained_relative(data, raw)?;
    let (relative, path) = match destinations.effective(raw) {
        Ok(found) => found,
        Err(error) => {
            warn!("Skipping server pack entry `{raw}`: {error:#}");
            if let Some(through) = error.downcast_ref::<super::world::ThroughLink>() {
                // A world, or an operator file the pack would not replace
                // anyway, is no news.
                let link = &through.link;
                if !protection.protects_effective(link)
                    && !is_preserved_config_file(&link.to_ascii_lowercase())
                {
                    destinations.skip_link(link.clone());
                }
            }
            return Ok(None);
        }
    };
    // Lowercase identifies a canonical operator name only. Preserve it only
    // when filesystem lookup resolves that name to this same checked path
    // (or cannot tell); an unrelated case-distinct Linux file keeps its own
    // destination.
    let canonical = relative.to_ascii_lowercase();
    let operator_file = !is_dir
        && is_preserved_config_file(&canonical)
        && path.try_exists()?
        && destinations
            .effective(&canonical)
            .map_or(true, |(_, canonical)| canonical == path);
    if protection.protects_effective(&relative)
        || (!is_dir && relative.eq_ignore_ascii_case("eula.txt"))
        || operator_file
    {
        return Ok(None);
    }
    Ok(Some(path))
}

/// Tells the user, after the install, which links in the server folder pack
/// files were not written through: GDLauncher does not write through links
/// (see [`super::world::Destinations::effective`]), and Repair does not
/// delete these (a `config/` shared with another server).
fn warn_skipped_links(task: &VisualTask, links: std::collections::BTreeSet<String>) {
    for folder in links {
        task.warn(crate::domain::vtask::TaskWarning::PackFilesSkippedAtLink { path: folder });
    }
}

/// [`pack_destination`] of one entry, as a test asks for it.
#[cfg(test)]
fn pack_destination_with(
    protection: &super::world::SaveProtection,
    data: &Path,
    raw: &str,
    is_dir: bool,
    lookup: super::world::ComponentLookup,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    let mut destinations = super::world::Destinations::new(data, lookup);
    pack_destination(protection, &mut destinations, raw, is_dir)
}
#[cfg(test)]
fn pack_in(
    protection: &super::world::SaveProtection,
    data: &Path,
    raw: &str,
    is_dir: bool,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    pack_destination_with(
        protection,
        data,
        raw,
        is_dir,
        super::world::filesystem_resolves,
    )
}

#[cfg(test)]
fn extract_mrpack_overrides(
    mrpack_path: &Path,
    data_path: &Path,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    extract_mrpack_overrides_with(mrpack_path, data_path, super::world::filesystem_resolves)
}
#[cfg(test)]
fn extract_mrpack_overrides_with(
    mrpack_path: &Path,
    data_path: &Path,
    lookup: super::world::ComponentLookup,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    let protection = super::world::SaveProtection::read(data_path)?;
    extract_mrpack_overrides_protected(mrpack_path, data_path, &protection, lookup)
}
/// Extracts an mrpack's overrides. Returns the links pack files were not
/// written through (see [`pack_destination`]).
fn extract_mrpack_overrides_protected(
    mrpack_path: &Path,
    data_path: &Path,
    protection: &super::world::SaveProtection,
    lookup: super::world::ComponentLookup,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    let mut destinations = super::world::Destinations::new(data_path, lookup);
    let file = std::fs::File::open(mrpack_path)?;
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

            let out_path = match pack_destination(
                protection,
                &mut destinations,
                relative,
                entry.is_dir(),
            ) {
                Ok(Some(out_path)) => out_path,
                Ok(None) => continue,
                Err(error) => {
                    warn!(
                        "Skipping mrpack override entry with unsafe path `{relative}`: {error:#}"
                    );
                    continue;
                }
            };

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

    Ok(destinations.into_skipped_links())
}

/// Extract a ZIP file into a target directory, handling single-root-folder
/// layouts. Returns the links pack files were not written through (see
/// [`pack_destination`]).
fn extract_zip_to_dir(
    zip_path: &Path,
    target_dir: &Path,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    extract_zip_to_dir_with(zip_path, target_dir, super::world::filesystem_resolves)
}
fn extract_zip_to_dir_with(
    zip_path: &Path,
    target_dir: &Path,
    lookup: super::world::ComponentLookup,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    let protection = super::world::SaveProtection::read(target_dir)?;
    let mut destinations = super::world::Destinations::new(target_dir, lookup);
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Check if all entries share a single root directory
    let mut root_dirs = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = match super::world::normalized_relative(target_dir, entry.name()) {
            Ok(name) => name,
            Err(_) => continue,
        };
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
                .map(|e| {
                    super::world::normalized_relative(target_dir, e.name())
                        .is_ok_and(|name| name.starts_with(&format!("{}/", root)))
                        && !e.is_dir()
                })
                .unwrap_or(false)
        });
        if has_subfiles && !protection.protects(&root) {
            Some(format!("{}/", root))
        } else {
            None
        }
    } else {
        None
    };

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = match super::world::normalized_relative(target_dir, entry.name()) {
            Ok(name) => name,
            Err(_) => continue,
        };

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

        let out_path =
            match pack_destination(&protection, &mut destinations, &relative, entry.is_dir()) {
                Ok(Some(out_path)) => out_path,
                Ok(None) => continue,
                Err(error) => {
                    warn!("Skipping server pack entry with unsafe path `{relative}`: {error:#}");
                    continue;
                }
            };

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

    Ok(destinations.into_skipped_links())
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

/// Read the newest loader version directory under `libraries/<vendor_path>/`
/// that actually contains the platform argument file, mirroring
/// `modloader_install::find_loader_args_file`'s own evidence requirement so
/// pack-detection never accepts a version the launch-time lookup would not.
///
/// Modern Forge (1.17+) and every NeoForge version install into a Maven-style
/// tree with the loader version as the leaf directory
/// (`libraries/net/neoforged/neoforge/21.1.77/`), and ship no versioned jar at
/// the data root at all. Server packs distributed pre-installed therefore have
/// to be recognised from this path.
async fn loader_version_from_libraries(data_path: &Path, vendor_path: &str) -> Option<String> {
    let file_name = super::modloader_install::platform_args_file_name();

    let vendor_dir = vendor_path
        .split('/')
        .fold(data_path.join("libraries"), |acc, segment| {
            acc.join(segment)
        });

    let mut versions: Vec<String> = Vec::new();
    let mut entries = tokio::fs::read_dir(&vendor_dir).await.ok()?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if !entry.path().is_dir() {
            continue;
        }
        // A version directory can exist without the installer having
        // finished (e.g. an interrupted extraction) — require the argfile
        // itself, not just the directory, before accepting the version.
        if !entry.path().join(file_name).exists() {
            continue;
        }
        if let Some(version) = entry.file_name().to_str() {
            versions.push(version.to_string());
        }
    }

    versions.sort();
    versions.pop()
}

/// Extract the loader version from a root-level jar such as
/// `neoforge-21.1.77-installer.jar` or `forge-1.20.1-47.4.10-universal.jar`.
fn loader_version_from_jar_name(name: &str, prefix: &str) -> Option<String> {
    let rest = name.strip_prefix(prefix)?.strip_suffix(".jar")?;
    // Installer/universal/shim jars all encode the same version; strip the role
    // suffix so we are left with something the installer URLs accept.
    let rest = ["-installer", "-universal", "-shim", "-server"]
        .iter()
        .find_map(|suffix| rest.strip_suffix(suffix))
        .unwrap_or(rest);

    (!rest.is_empty()).then(|| rest.to_string())
}

/// Detect modloader type and version from extracted server pack files.
///
/// Handles both shapes a server pack arrives in: already installed (the loader
/// unpacked under `libraries/`, or a self-contained Fabric/Quilt launcher jar),
/// and bare (only the installer jar shipped at the root).
///
/// Errors if both a Forge and a NeoForge tree are present with a valid argfile:
/// that is not a state any real install produces, and silently preferring one
/// risks installing/launching the wrong loader entirely.
async fn detect_modloader_from_files(
    data_path: &Path,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    // Pre-installed Forge/NeoForge — check before the root-jar scan, since these
    // packs frequently ship no versioned jar at the root whatsoever.
    let neoforge = loader_version_from_libraries(data_path, "net/neoforged/neoforge").await;
    let forge = loader_version_from_libraries(data_path, "net/minecraftforge/forge").await;

    match (neoforge, forge) {
        (Some(neoforge_version), Some(forge_version)) => bail!(
            "Server pack has both NeoForge ({neoforge_version}) and Forge ({forge_version}) \
             installed under libraries/ — ambiguous modloader, refusing to guess"
        ),
        (Some(version), None) => return Ok((Some("neoforge".to_string()), Some(version))),
        (None, Some(version)) => return Ok((Some("forge".to_string()), Some(version))),
        (None, None) => {}
    }

    // Fabric and Quilt ship a self-contained launcher jar with no version in the
    // name. The loader is already installed, so a missing version is fine here.
    if data_path.join("fabric-server-launch.jar").exists() {
        return Ok((Some("fabric".to_string()), None));
    }

    if data_path.join("quilt-server-launch.jar").exists() {
        return Ok((Some("quilt".to_string()), None));
    }

    // Bare pack: the loader still has to be installed from a root-level jar.
    if let Ok(mut entries) = tokio::fs::read_dir(data_path).await {
        let mut forge_version = None;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(version) = loader_version_from_jar_name(&name, "neoforge-") {
                return Ok((Some("neoforge".to_string()), Some(version)));
            }
            if forge_version.is_none() {
                forge_version = loader_version_from_jar_name(&name, "forge-");
            }
        }
        if let Some(version) = forge_version {
            return Ok((Some("forge".to_string()), Some(version)));
        }
    }

    Ok((None, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_pack_eula_is_never_extracted_verbatim() {
        // A pack's own eula.txt must never auto-accept the EULA on the
        // operator's behalf, even on a brand new install where the
        // "already exists" guard that protects the other preserved config
        // files doesn't apply yet (this is a fresh, empty target dir).
        use std::io::Write;

        let src_dir = tempfile::tempdir().unwrap();
        let zip_path = src_dir.path().join("pack.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("eula.txt", zip::write::FileOptions::<()>::default())
                .unwrap();
            zip.write_all(b"eula=true\n").unwrap();
            zip.start_file("server.jar", zip::write::FileOptions::<()>::default())
                .unwrap();
            zip.write_all(b"jar").unwrap();
            zip.finish().unwrap();
        }

        let target_dir = tempfile::tempdir().unwrap();
        extract_zip_to_dir(&zip_path, target_dir.path()).unwrap();

        assert!(
            !target_dir.path().join("eula.txt").exists(),
            "a pack's eula.txt must never be written, even on a fresh install"
        );
        assert!(target_dir.path().join("server.jar").exists());
    }

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

    /// Layout produced by `neoforge-<ver>-installer.jar --installServer`, which
    /// is what a pre-installed NeoForge server pack contains: libraries tree,
    /// run scripts, and no versioned jar at the data root.
    fn write_preinstalled_neoforge(data_path: &Path, version: &str) {
        let loader_dir = data_path
            .join("libraries/net/neoforged/neoforge")
            .join(version);
        std::fs::create_dir_all(&loader_dir).unwrap();
        std::fs::write(loader_dir.join("unix_args.txt"), "-p libraries/foo.jar").unwrap();
        std::fs::write(loader_dir.join("win_args.txt"), "-p libraries/foo.jar").unwrap();
        std::fs::write(data_path.join("run.sh"), "#!/bin/sh").unwrap();
        std::fs::write(data_path.join("user_jvm_args.txt"), "-Xmx4G").unwrap();
        std::fs::create_dir_all(data_path.join("mods")).unwrap();
    }

    #[tokio::test]
    async fn detects_preinstalled_neoforge_server_pack() {
        // Regression: NeoForge server packs ship no `neoforge-*.jar` at the
        // root, so filename sniffing reported them as vanilla. The server then
        // booted vanilla and modded clients were rejected with "you are trying
        // to connect to a server that is not running NeoForge".
        let dir = tempfile::tempdir().unwrap();
        write_preinstalled_neoforge(dir.path(), "21.1.77");

        let (loader, version) = detect_modloader_from_files(dir.path()).await.unwrap();

        assert_eq!(loader.as_deref(), Some("neoforge"));
        assert_eq!(version.as_deref(), Some("21.1.77"));
    }

    #[tokio::test]
    async fn detects_preinstalled_forge_server_pack() {
        let dir = tempfile::tempdir().unwrap();
        let loader_dir = dir
            .path()
            .join("libraries/net/minecraftforge/forge/1.20.1-47.4.10");
        std::fs::create_dir_all(&loader_dir).unwrap();
        std::fs::write(loader_dir.join("unix_args.txt"), "-p libraries/foo.jar").unwrap();
        std::fs::write(loader_dir.join("win_args.txt"), "-p libraries/foo.jar").unwrap();

        let (loader, version) = detect_modloader_from_files(dir.path()).await.unwrap();

        assert_eq!(loader.as_deref(), Some("forge"));
        assert_eq!(version.as_deref(), Some("1.20.1-47.4.10"));
    }

    #[tokio::test]
    async fn detects_bare_neoforge_installer_jar() {
        // Some packs ship only the installer and expect it to be run.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("neoforge-21.1.77-installer.jar"), b"jar").unwrap();

        let (loader, version) = detect_modloader_from_files(dir.path()).await.unwrap();

        assert_eq!(loader.as_deref(), Some("neoforge"));
        // The `-installer` suffix must not leak into the version, or the maven
        // URL built from it 404s.
        assert_eq!(version.as_deref(), Some("21.1.77"));
    }

    #[tokio::test]
    async fn detects_fabric_and_quilt_launcher_jars() {
        let fabric = tempfile::tempdir().unwrap();
        std::fs::write(fabric.path().join("fabric-server-launch.jar"), b"jar").unwrap();
        assert_eq!(
            detect_modloader_from_files(fabric.path())
                .await
                .unwrap()
                .0
                .as_deref(),
            Some("fabric")
        );

        let quilt = tempfile::tempdir().unwrap();
        std::fs::write(quilt.path().join("quilt-server-launch.jar"), b"jar").unwrap();
        assert_eq!(
            detect_modloader_from_files(quilt.path())
                .await
                .unwrap()
                .0
                .as_deref(),
            Some("quilt")
        );
    }

    #[tokio::test]
    async fn reports_no_modloader_for_a_vanilla_server_pack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("server.jar"), b"jar").unwrap();
        std::fs::create_dir_all(dir.path().join("world")).unwrap();

        let (loader, version) = detect_modloader_from_files(dir.path()).await.unwrap();

        assert_eq!(loader, None);
        assert_eq!(version, None);
    }

    #[tokio::test]
    async fn loader_version_dir_without_argfile_is_not_detected() {
        // Regression: a version directory can exist without the installer
        // having finished (e.g. an interrupted extraction) — the old scan
        // accepted any directory under libraries/, so this used to falsely
        // report the loader as installed.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("libraries/net/neoforged/neoforge/21.1.77"))
            .unwrap();
        // No unix_args.txt / win_args.txt written into the version dir.

        let (loader, version) = detect_modloader_from_files(dir.path()).await.unwrap();

        assert_eq!(loader, None);
        assert_eq!(version, None);
    }

    #[tokio::test]
    async fn ambiguous_forge_and_neoforge_trees_are_rejected() {
        // A data dir with both loader trees fully installed is not a state
        // any real install produces — treat it as corrupt rather than
        // silently preferring NeoForge.
        let dir = tempfile::tempdir().unwrap();
        write_preinstalled_neoforge(dir.path(), "21.1.77");
        let forge_dir = dir
            .path()
            .join("libraries/net/minecraftforge/forge/1.20.1-47.4.10");
        std::fs::create_dir_all(&forge_dir).unwrap();
        std::fs::write(forge_dir.join("unix_args.txt"), "-p libraries/foo.jar").unwrap();
        std::fs::write(forge_dir.join("win_args.txt"), "-p libraries/foo.jar").unwrap();

        let result = detect_modloader_from_files(dir.path()).await;

        assert!(
            result.is_err(),
            "expected an ambiguity error, got {result:?}"
        );
    }

    #[test]
    fn jar_name_version_strips_role_suffixes() {
        let cases = [
            ("neoforge-21.1.77.jar", "neoforge-", "21.1.77"),
            ("neoforge-21.1.77-installer.jar", "neoforge-", "21.1.77"),
            (
                "forge-1.20.1-47.4.10-universal.jar",
                "forge-",
                "1.20.1-47.4.10",
            ),
            ("forge-1.20.1-47.4.10-shim.jar", "forge-", "1.20.1-47.4.10"),
        ];

        for (name, prefix, expected) in cases {
            assert_eq!(
                loader_version_from_jar_name(name, prefix).as_deref(),
                Some(expected),
                "parsing {name}"
            );
        }

        // A NeoForge jar must not be mistaken for a Forge one.
        assert_eq!(
            loader_version_from_jar_name("neoforge-21.1.77.jar", "forge-"),
            None
        );
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

fn mrpack_downloadable(
    file: &carbon_platforms::modrinth::version::ModrinthFile,
    url: &str,
    path: &Path,
) -> carbon_net::Downloadable {
    carbon_net::Downloadable::new(url, path)
        .with_size(u64::from(file.file_size))
        .with_checksum(Some(carbon_net::Checksum::Sha512(
            file.hashes.sha512.clone(),
        )))
}
#[cfg(test)]
mod task06_tests {
    use super::*;
    #[test]
    fn task06_mrpack_descriptor_keeps_integrity() {
        let file: carbon_platforms::modrinth::version::ModrinthFile = serde_json::from_value(serde_json::json!({
            "path": "mods/fixture.jar", "hashes": {"sha1": "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3", "sha512": "ee26b0dd4af7e749aa1a8ee3c10ae9923f618980772e473f8819a5d4940e0db27ac185f8a0e1d5f84f88bc887fd67b143732c304cc5fa9ad8e6f57f50028a8ff"},
            "downloads": ["https://fixture.invalid/artifact"], "fileSize": 4
        })).unwrap();
        let d = mrpack_downloadable(&file, &file.downloads[0], Path::new("mods/fixture.jar"));
        assert_eq!(d.size, Some(4));
        assert!(
            matches!(d.checksum, Some(carbon_net::Checksum::Sha512(hash)) if hash == file.hashes.sha512)
        );
    }
}

#[cfg(test)]
mod task07_save_tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn task07_custom_and_metadata_worlds_survive_pack_extraction() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("server");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            data.join("server.properties"),
            "level-name=AdventureMap\nserver-port=25577\n",
        )
        .unwrap();
        for world in ["AdventureMap", "ArchivedMap", "world", "DIM-1"] {
            std::fs::create_dir(data.join(world)).unwrap();
            std::fs::write(data.join(world).join("level.dat"), "PROGRESSED").unwrap();
        }
        let zip_path = root.path().join("pack.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        for name in [
            "AdventureMap/level.dat",
            "ArchivedMap/level.dat",
            "world/level.dat",
            "DIM-1/level.dat",
            "server.properties",
            "config/test.txt",
            "dimension-config/test.txt",
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"BUNDLED").unwrap();
        }
        zip.finish().unwrap();
        extract_zip_to_dir(&zip_path, &data).unwrap();
        let mrpack_path = root.path().join("pack.mrpack");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&mrpack_path).unwrap());
        for name in [
            "overrides/AdventureMap/level.dat",
            "server-overrides/AdventureMap/level.dat",
            "server-overrides/ArchivedMap/level.dat",
            "server-overrides/world/level.dat",
            "server-overrides/server.properties",
            "server-overrides/config/test.txt",
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"BUNDLED").unwrap();
        }
        zip.finish().unwrap();
        extract_mrpack_overrides(&mrpack_path, &data).unwrap();
        for world in ["AdventureMap", "ArchivedMap", "world", "DIM-1"] {
            assert_eq!(
                std::fs::read_to_string(data.join(world).join("level.dat")).unwrap(),
                "PROGRESSED",
                "{world}"
            );
        }
        assert_eq!(
            std::fs::read_to_string(data.join("config/test.txt")).unwrap(),
            "BUNDLED"
        );
        assert_eq!(
            std::fs::read_to_string(data.join("dimension-config/test.txt")).unwrap(),
            "BUNDLED"
        );
        assert_eq!(
            std::fs::read_to_string(data.join("server.properties")).unwrap(),
            "level-name=AdventureMap\nserver-port=25577\n"
        );
    }
}

#[cfg(test)]
mod odd_entry_tests {
    use super::*;
    use std::io::Write;

    fn archive(path: &Path, prefix: &str, names: &[&str]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for name in names {
            zip.start_file(
                format!("{prefix}{name}"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"BUNDLED").unwrap();
        }
        zip.finish().unwrap();
    }

    /// A path the filesystem resolves to no entry it lists (as Windows does
    /// for a name ending in a dot), here only for `ghost`.
    fn ghost_resolves(parent: &Path, component: &str, entries: &[String]) -> anyhow::Result<bool> {
        Ok(component == "ghost"
            || super::super::world::filesystem_resolves(parent, component, entries)?)
    }

    /// One odd entry (a path that leaves the server folder, one through a
    /// folder linked elsewhere, a name the filesystem cannot place) is
    /// skipped with a warning, as 2.0.40 skipped unsafe paths, and the rest
    /// of the pack is still installed; on a Repair the pack's mods were
    /// already deleted, so failing here left the server without them.
    #[test]
    fn odd_entries_are_skipped_and_the_rest_installed() {
        for mr in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let data = root.path().join("data");
            std::fs::create_dir_all(&data).unwrap();
            let shared = root.path().join("shared-config");
            std::fs::create_dir_all(&shared).unwrap();
            std::fs::write(shared.join("shared.toml"), b"OPERATOR").unwrap();
            super::super::world::link_folder(&shared, &data.join("config"));
            let names = [
                "../escape.txt",
                "ghost/a.txt",
                "config/shared.toml",
                "config/other.toml",
                "kubejs/startup.js",
                "mods/pack.jar",
            ];
            let zip = root.path().join("pack.zip");
            archive(&zip, if mr { "overrides/" } else { "" }, &names);
            let skipped_links = if mr {
                extract_mrpack_overrides_with(&zip, &data, ghost_resolves).unwrap()
            } else {
                extract_zip_to_dir_with(&zip, &data, ghost_resolves).unwrap()
            };
            assert_eq!(
                skipped_links.into_iter().collect::<Vec<_>>(),
                vec!["config"],
                "mr={mr}: the link is told once"
            );
            for written in ["kubejs/startup.js", "mods/pack.jar"] {
                assert_eq!(
                    std::fs::read(data.join(written)).unwrap(),
                    b"BUNDLED",
                    "mr={mr} {written}"
                );
            }
            assert!(!root.path().join("escape.txt").exists());
            assert!(!data.join("ghost").exists());
            assert_eq!(
                std::fs::read(shared.join("shared.toml")).unwrap(),
                b"OPERATOR",
                "nothing is written through a link"
            );
            assert!(!shared.join("other.toml").exists());
        }
    }

    /// The links pack files were not written through reach the user once the
    /// install has succeeded.
    #[tokio::test]
    async fn skipped_links_are_reported_when_the_install_succeeds() {
        let app = crate::setup_managers_for_test().await;
        let task = VisualTask::new(Translation::ServerTaskInstallFromModpack {
            server_name: "Server".into(),
        });
        task.require_explicit_completion();
        app.task_manager().spawn_task(&task).await;
        warn_skipped_links(&task, ["config".to_owned()].into());
        task.complete();
        drop(task);
        let warnings = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let warnings = app.task_manager().get_task_warnings().await;
                if !warnings.is_empty() {
                    break warnings;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            warnings[0].warnings,
            vec![crate::domain::vtask::TaskWarning::PackFilesSkippedAtLink {
                path: "config".into()
            }]
        );
    }

    /// Pack files aimed at a linked world are refused like those aimed at any
    /// world, which needs no telling; only a link the pack had files for that
    /// are missing now is told.
    #[test]
    fn a_linked_world_is_not_reported_as_skipped() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        std::fs::create_dir_all(&data).unwrap();
        let world = root.path().join("elsewhere");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(data.join("server.properties"), "level-name=survival\n").unwrap();
        super::super::world::link_folder(&world, &data.join("survival"));
        let zip = root.path().join("pack.zip");
        archive(&zip, "", &["survival/level.dat", "mods/pack.jar"]);
        let skipped_links = extract_zip_to_dir(&zip, &data).unwrap();
        assert!(skipped_links.is_empty(), "{skipped_links:?}");
        assert!(!world.join("level.dat").exists());
        assert!(data.join("mods/pack.jar").is_file());
    }

    /// One extraction of a pack that spells a folder two ways keeps each
    /// file in the folder its entry names on a case-sensitive volume, in any
    /// order.
    #[test]
    fn a_case_sensitive_volume_keeps_case_distinct_folders_apart() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir(data.join("Probe")).unwrap();
        let insensitive = data.join("probe").exists();
        std::fs::remove_dir(data.join("Probe")).unwrap();
        if insensitive {
            return; // Only a case-sensitive volume has two such folders.
        }
        for mr in [false, true] {
            let zip = root.path().join("pack.zip");
            archive(
                &zip,
                if mr { "overrides/" } else { "" },
                &[
                    "Config/x.toml",
                    "Config/y.toml",
                    "config/z.toml",
                    "config/w.toml",
                ],
            );
            if mr {
                extract_mrpack_overrides(&zip, &data).unwrap();
            } else {
                extract_zip_to_dir(&zip, &data).unwrap();
            }
            for written in [
                "Config/x.toml",
                "Config/y.toml",
                "config/z.toml",
                "config/w.toml",
            ] {
                assert!(data.join(written).is_file(), "mr={mr} {written}");
            }
            assert!(!data.join("Config/w.toml").exists(), "mr={mr}");
            std::fs::remove_dir_all(data.join("Config")).unwrap();
            std::fs::remove_dir_all(data.join("config")).unwrap();
        }
    }

    /// A pack file with `:` in its name is written as 2.0.40 wrote it outside
    /// Windows, where it is an ordinary character. Windows reads it as a
    /// stream, so skips it instead of failing the whole install; only a path
    /// that leaves the server folder is an error for the caller.
    #[test]
    fn a_colon_in_a_pack_path_does_not_fail_the_install() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let protection = super::super::world::SaveProtection::read(data).unwrap();
        let destination = pack_in(&protection, data, "mods/foo:bar.jar", false).unwrap();
        if cfg!(windows) {
            assert_eq!(destination, None);
        } else {
            assert_eq!(destination, Some(data.join("mods").join("foo:bar.jar")));
        }
        assert!(pack_in(&protection, data, "../escape.jar", false).is_err());
    }
}

#[cfg(test)]
mod task07r1_alias_tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn task07r1_r1_effective_save_config_eula_aliases_are_preserved() {
        for mr in [false, true] {
            for prefix in ["./", "config/../", "Wrapper/config/../"] {
                let root = tempfile::tempdir().unwrap();
                let data = root.path().join("data");
                std::fs::create_dir_all(data.join("AdventureMap")).unwrap();
                std::fs::write(data.join("AdventureMap/level.dat"), "PROGRESSED").unwrap();
                std::fs::write(
                    data.join("server.properties"),
                    "level-name=AdventureMap\nserver-port=25577\n",
                )
                .unwrap();
                let zip = root.path().join("pack.zip");
                let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
                let outer = if mr { "server-overrides/" } else { "" };
                let effective_prefix = if mr && prefix.starts_with("Wrapper") {
                    "config/../"
                } else {
                    prefix
                };
                for (path, bytes) in [
                    ("AdventureMap/level.dat", "BUNDLED"),
                    ("server.properties", "level-name=world"),
                    ("eula.txt", "eula=true"),
                ] {
                    writer
                        .start_file(
                            format!("{outer}{effective_prefix}{path}"),
                            zip::write::SimpleFileOptions::default(),
                        )
                        .unwrap();
                    writer.write_all(bytes.as_bytes()).unwrap();
                }
                let control = if !mr && prefix.starts_with("Wrapper") {
                    "Wrapper/config/control.txt"
                } else {
                    "config/control.txt"
                };
                writer
                    .start_file(
                        format!("{outer}{control}"),
                        zip::write::SimpleFileOptions::default(),
                    )
                    .unwrap();
                writer.write_all(b"ordinary").unwrap();
                writer.finish().unwrap();
                if mr {
                    extract_mrpack_overrides(&zip, &data).unwrap();
                } else {
                    extract_zip_to_dir(&zip, &data).unwrap();
                }
                assert_eq!(
                    std::fs::read_to_string(data.join("AdventureMap/level.dat")).unwrap(),
                    "PROGRESSED",
                    "mr={mr} prefix={prefix}"
                );
                assert_eq!(
                    std::fs::read_to_string(data.join("server.properties")).unwrap(),
                    "level-name=AdventureMap\nserver-port=25577\n"
                );
                assert!(!data.join("eula.txt").exists());
                assert_eq!(
                    std::fs::read_to_string(data.join("config/control.txt")).unwrap(),
                    "ordinary"
                );
            }
        }
    }
}

#[cfg(test)]
#[test]
fn task07r1_r1_index_destinations_share_effective_save_and_operator_policy() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("AdventureMap")).unwrap();
    std::fs::write(
        root.path().join("server.properties"),
        "level-name=AdventureMap",
    )
    .unwrap();
    let protection = super::world::SaveProtection::read(root.path()).unwrap();
    for raw in [
        "./AdventureMap/level.dat",
        "config/../AdventureMap/level.dat",
        "./server.properties",
        "config/../server.properties",
        "./eula.txt",
        "config/../eula.txt",
    ] {
        assert!(
            pack_in(&protection, root.path(), raw, false)
                .unwrap()
                .is_none(),
            "index raw alias {raw}"
        );
    }
    assert_eq!(
        pack_in(
            &protection,
            root.path(),
            "config/../config/options.txt",
            false
        )
        .unwrap(),
        Some(root.path().join("config/options.txt"))
    );
    assert!(pack_in(&protection, root.path(), "../escape", false).is_err());
}

#[cfg(all(test, target_os = "linux"))]
mod task07r2_operator_tests {
    use super::*;
    use std::io::Write;

    const OPERATOR_FILES: [&str; 7] = [
        "server.properties",
        "whitelist.json",
        "ops.json",
        "banned-players.json",
        "banned-ips.json",
        "usercache.json",
        "icon.png",
    ];
    const ORIGINAL: &[u8] = b"level-name=world\nserver-port=25577\n";
    const BUNDLED: &[u8] = b"level-name=world\nserver-port=25599\n";

    fn insensitive_lookup(
        _parent: &Path,
        component: &str,
        entries: &[String],
    ) -> anyhow::Result<bool> {
        // Inject only component lookup. The same production resolver and archive
        // writers still select and mutate real temporary files on Linux.
        Ok(entries
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(component)))
    }
    fn policy(insensitive: bool) -> super::super::world::ComponentLookup {
        if insensitive {
            insensitive_lookup
        } else {
            super::super::world::filesystem_resolves
        }
    }

    #[test]
    fn task07r2_operator_case_equivalence_preserves_real_cf_mr_archive_destinations() {
        for mr in [false, true] {
            for insensitive in [true, false] {
                for disk_upper in [true, false] {
                    for archive_upper in [false, true] {
                        for canonical in OPERATOR_FILES {
                            let root = tempfile::tempdir().unwrap();
                            let data = root.path().join("data");
                            std::fs::create_dir(&data).unwrap();
                            let disk = if disk_upper {
                                canonical.to_uppercase()
                            } else {
                                canonical.into()
                            };
                            let archive_name = if archive_upper {
                                canonical.to_uppercase()
                            } else {
                                canonical.into()
                            };
                            std::fs::write(data.join(&disk), ORIGINAL).unwrap();
                            let zip = root.path().join("pack.zip");
                            let mut writer =
                                zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
                            let prefix = if mr { "server-overrides/" } else { "" };
                            for (name, bytes) in [
                                (&*archive_name, BUNDLED),
                                ("config/ordinary.txt", b"ordinary"),
                                ("EULA.TXT", b"eula=true"),
                            ] {
                                writer
                                    .start_file(
                                        format!("{prefix}{name}"),
                                        zip::write::SimpleFileOptions::default(),
                                    )
                                    .unwrap();
                                writer.write_all(bytes).unwrap();
                            }
                            writer.finish().unwrap();
                            if mr {
                                extract_mrpack_overrides_with(&zip, &data, policy(insensitive))
                                    .unwrap();
                            } else {
                                extract_zip_to_dir_with(&zip, &data, policy(insensitive)).unwrap();
                            }
                            let expected = if !insensitive && disk_upper && archive_upper {
                                BUNDLED
                            } else {
                                ORIGINAL
                            };
                            assert_eq!(
                                std::fs::read(data.join(&disk)).unwrap(),
                                expected,
                                "operator preservation mr={mr} insensitive={insensitive} disk={disk} archive={archive_name}"
                            );
                            if !insensitive && disk_upper != archive_upper {
                                assert_eq!(
                                    std::fs::read(data.join(&archive_name)).unwrap(),
                                    BUNDLED,
                                    "distinct Linux destination stays writable"
                                );
                            }
                            assert_eq!(
                                std::fs::read(data.join("config/ordinary.txt")).unwrap(),
                                b"ordinary"
                            );
                            assert!(!data.join("EULA.TXT").exists());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn task07r2_index_operator_policy_uses_canonical_destination_equivalence() {
        for insensitive in [true, false] {
            for disk_upper in [true, false] {
                for archive_upper in [false, true] {
                    for canonical in OPERATOR_FILES {
                        let root = tempfile::tempdir().unwrap();
                        let disk = if disk_upper {
                            canonical.to_uppercase()
                        } else {
                            canonical.into()
                        };
                        let archive_name = if archive_upper {
                            canonical.to_uppercase()
                        } else {
                            canonical.into()
                        };
                        std::fs::write(root.path().join(&disk), ORIGINAL).unwrap();
                        let protection =
                            super::super::world::SaveProtection::read(root.path()).unwrap();
                        let result = pack_destination_with(
                            &protection,
                            root.path(),
                            &archive_name,
                            false,
                            policy(insensitive),
                        )
                        .unwrap();
                        let protected = insensitive || (!disk_upper && !archive_upper);
                        assert_eq!(
                            result.is_none(),
                            protected,
                            "index insensitive={insensitive} disk={disk} archive={archive_name}"
                        );
                        if let Some(path) = result {
                            assert_eq!(path, root.path().join(&archive_name));
                        }
                        assert_eq!(std::fs::read(root.path().join(&disk)).unwrap(), ORIGINAL);
                        assert!(
                            pack_destination_with(
                                &protection,
                                root.path(),
                                "EULA.TXT",
                                false,
                                policy(insensitive)
                            )
                            .unwrap()
                            .is_none()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn task07r2_native_case_distinct_files_remain_independent() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("server.properties"), ORIGINAL).unwrap();
        std::fs::write(
            root.path().join("SERVER.PROPERTIES"),
            b"unrelated uppercase content",
        )
        .unwrap();
        let protection = super::super::world::SaveProtection::read(root.path()).unwrap();
        assert!(
            pack_in(&protection, root.path(), "server.properties", false)
                .unwrap()
                .is_none()
        );
        let destination = pack_in(&protection, root.path(), "SERVER.PROPERTIES", false)
            .unwrap()
            .unwrap();
        std::fs::write(destination, BUNDLED).unwrap();
        assert_eq!(
            std::fs::read(root.path().join("server.properties")).unwrap(),
            ORIGINAL
        );
        assert_eq!(
            std::fs::read(root.path().join("SERVER.PROPERTIES")).unwrap(),
            BUNDLED
        );
    }
}

/// Shared direct archive writer. The owner retains entered writes and flushes.

#[cfg(test)]
mod task18_r1_stream_tests {
    use super::*;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::io::AsyncWrite;
    struct PausedWriter {
        file: tokio::fs::File,
        entered: Option<tokio::sync::oneshot::Sender<()>>,
        release: Option<tokio::sync::oneshot::Receiver<()>>,
    }
    impl AsyncWrite for PausedWriter {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            if let Some(entered) = self.entered.take() {
                let _ = entered.send(());
            }
            if let Some(release) = self.release.as_mut() {
                use std::future::Future;
                if Pin::new(release).poll(cx).is_pending() {
                    return Poll::Pending;
                }
                self.release = None;
            }
            Pin::new(&mut self.file).poll_write(cx, bytes)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.file).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.file).poll_shutdown(cx)
        }
    }
    fn stream(
        rx: tokio::sync::mpsc::Receiver<Result<Vec<u8>, std::io::Error>>,
    ) -> Pin<Box<dyn futures::Stream<Item = Result<Vec<u8>, std::io::Error>> + Send>> {
        Box::pin(futures::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        }))
    }
    #[tokio::test]
    async fn task18_r1_archive_pending_body_cancels_without_new_phase() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("archive");
        let mut file = tokio::fs::File::create(&path).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let cancel = tokio_util::sync::CancellationToken::new();
        let token = cancel.clone();
        let mut worker = tokio::spawn(async move {
            copy_archive_stream(&mut stream(rx), &mut file, &token, |_| {}).await
        });
        tokio::task::yield_now().await;
        cancel.cancel();
        let result = tokio::time::timeout(std::time::Duration::from_millis(100), &mut worker).await;
        if result.is_err() {
            drop(tx);
            let _ = worker.await;
            panic!("cancelled archive remained waiting for its next chunk");
        }
        assert!(
            result.unwrap().unwrap().is_err(),
            "cancelled archive must not authorize extraction"
        );
        assert!(std::fs::read(path).unwrap().is_empty());
    }
    #[tokio::test]
    async fn task18_r1_archive_cancel_awaits_entered_real_write_and_flush() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("archive");
        let file = tokio::fs::File::create(&path).await.unwrap();
        let (entered, wait_entered) = tokio::sync::oneshot::channel();
        let (release, wait_release) = tokio::sync::oneshot::channel();
        let mut writer = PausedWriter {
            file,
            entered: Some(entered),
            release: Some(wait_release),
        };
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.send(Ok(b"entered archive bytes".to_vec()))
            .await
            .unwrap();
        let cancel = tokio_util::sync::CancellationToken::new();
        let token = cancel.clone();
        let mut worker = tokio::spawn(async move {
            copy_archive_stream(&mut stream(rx), &mut writer, &token, |_| {}).await
        });
        wait_entered.await.unwrap();
        cancel.cancel();
        tokio::task::yield_now().await;
        assert!(
            !worker.is_finished(),
            "entered write must settle before cancellation returns"
        );
        release.send(()).unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_millis(150), &mut worker).await;
        if result.is_err() {
            drop(tx);
            let _ = worker.await;
            panic!("completed entered write then ignored cancellation at next chunk boundary");
        }
        assert!(result.unwrap().unwrap().is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"entered archive bytes");
    }
    #[tokio::test]
    async fn task18_r1_archive_healthy_completion_retains_bytes_and_progress() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("archive");
        let mut file = tokio::fs::File::create(&path).await.unwrap();
        let mut progress = Vec::new();
        let mut stream = futures::stream::iter([
            Ok::<_, std::io::Error>(b"first".to_vec()),
            Ok(b"second".to_vec()),
        ]);
        assert_eq!(
            copy_archive_stream(
                &mut stream,
                &mut file,
                &tokio_util::sync::CancellationToken::new(),
                |n| progress.push(n)
            )
            .await
            .unwrap(),
            11
        );
        drop(file);
        assert_eq!(std::fs::read(path).unwrap(), b"firstsecond");
        assert_eq!(progress, vec![5, 11]);
    }
}

async fn copy_archive_stream<S, B, E, W>(
    stream: &mut S,
    file: &mut W,
    cancel: &tokio_util::sync::CancellationToken,
    mut progress: impl FnMut(u64),
) -> anyhow::Result<u64>
where
    S: futures::Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: Into<anyhow::Error>,
    W: tokio::io::AsyncWrite + Unpin,
{
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    let result: anyhow::Result<u64> = async {
        let mut downloaded = 0u64;
        loop {
            // Only pending transport waits are cancelable. An entered write is
            // always awaited before observing cancellation at the next boundary.
            let chunk =
                crate::managers::runtime_lifecycle::wait_or_cancel(cancel, stream.next()).await?;
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = chunk.map_err(Into::into)?;
            file.write_all(chunk.as_ref()).await?;
            downloaded = downloaded
                .checked_add(chunk.as_ref().len() as u64)
                .context("Server archive byte count exceeds u64")?;
            progress(downloaded);
        }
        Ok(downloaded)
    }
    .await;
    // Drain entered file work before returning cancellation/error; caller drops
    // the handle before its owned private archive directory is cleaned up.
    let flushed = file.flush().await;
    let closed = file.shutdown().await;
    let downloaded = result?;
    flushed?;
    closed?;
    Ok(downloaded)
}

#[cfg(test)]
mod task18_r2_phase_tests {
    use super::*;
    use crate::managers::runtime_lifecycle::{
        ActivityPolicy, RuntimeLifecycle, ShutdownPurpose, run_owned_phase, spawn_retained,
    };
    use std::io::{Read, Write};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    async fn phase_case(hash: bool, cancel_during_io: bool) {
        let root = tempfile::tempdir().unwrap();
        let zip_path = root.path().join("pack.zip");
        let target = root.path().join("data");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        for (name, bytes) in [
            ("first.txt", b"first".as_slice()),
            ("second.txt", b"second".as_slice()),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        let source = root.path().join("archive-to-hash");
        std::fs::write(&source, b"real hash input across a controlled boundary").unwrap();
        let expected = {
            use sha2::Digest;
            sha2::Sha512::digest(std::fs::read(&source).unwrap()).to_vec()
        };
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let owner = lifecycle
            .admit_owned("pack phase fixture", ActivityPolicy::CoreWork)
            .unwrap();
        let cancel = owner.cancellation();
        let (entered, wait_entered) = tokio::sync::oneshot::channel();
        let (release, wait_release) = std::sync::mpsc::channel();
        let phases = Arc::new(AtomicUsize::new(0));
        let observed = phases.clone();
        let data = target.clone();
        let worker = spawn_retained(owner, move |_| {
            Box::pin(async move {
                let prep_target = data.clone();
                let digest = run_owned_phase(&cancel, || async move {
                    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
                        if hash {
                            use sha2::Digest;
                            let mut file = std::fs::File::open(source)?;
                            let mut first = [0u8; 4];
                            file.read_exact(&mut first)?;
                            let mut digest = sha2::Sha512::new();
                            digest.update(first);
                            entered.send(()).unwrap();
                            wait_release.recv().unwrap();
                            std::io::copy(&mut file, &mut digest)?;
                            Ok(digest.finalize().to_vec())
                        } else {
                            std::fs::create_dir_all(&prep_target)?;
                            entered.send(()).unwrap();
                            wait_release.recv().unwrap();
                            std::fs::create_dir_all(prep_target.join("settled-mkdir"))?;
                            Ok(Vec::new())
                        }
                    })
                    .await?
                })
                .await?;
                if hash {
                    assert_eq!(
                        digest, expected,
                        "entered complete hash must settle even when cancelled"
                    );
                }
                run_owned_phase(&cancel, || async move {
                    observed.fetch_add(1, Ordering::SeqCst);
                    tokio::fs::create_dir_all(&data).await?;
                    tokio::task::spawn_blocking(move || extract_zip_to_dir(&zip_path, &data))
                        .await?
                        .map(drop)
                })
                .await
            })
        });
        wait_entered.await.unwrap();
        if cancel_during_io {
            lifecycle
                .prepare_shutdown("phase", ShutdownPurpose::Quit, async { Ok(()) })
                .unwrap();
        }
        tokio::task::yield_now().await;
        assert!(
            !worker.is_finished(),
            "entered preparatory IO must remain owned"
        );
        if cancel_during_io {
            assert!(
                lifecycle
                    .commit_prepared_shutdown("phase", ShutdownPurpose::Quit)
                    .is_err()
            );
        }
        release.send(()).unwrap();
        let result = worker.await.unwrap();
        if cancel_during_io {
            assert_eq!(
                phases.load(Ordering::SeqCst),
                0,
                "cancelled preparatory IO admitted a new extraction phase"
            );
            assert!(result.is_err());
            assert!(!target.join("first.txt").exists());
        } else {
            result.unwrap();
            assert_eq!(phases.load(Ordering::SeqCst), 1);
            assert_eq!(std::fs::read(target.join("first.txt")).unwrap(), b"first");
            assert_eq!(std::fs::read(target.join("second.txt")).unwrap(), b"second");
        }
        if !hash {
            assert!(target.join("settled-mkdir").is_dir());
        }
    }
    #[tokio::test]
    async fn task18_r2_cancel_during_hash_refuses_next_extraction() {
        phase_case(true, true).await;
    }
    #[tokio::test]
    async fn task18_r2_cancel_during_directory_io_refuses_next_extraction() {
        phase_case(false, true).await;
    }
    #[tokio::test]
    async fn task18_r2_healthy_hash_directory_and_extraction_complete() {
        phase_case(true, false).await;
        phase_case(false, false).await;
    }
}

/// Byte counter publication is separate from stream IO and preserves its final flush.
struct ArchiveProgress<'a> {
    task: &'a crate::managers::vtask::Subtask,
    total: Option<u64>,
    last_reported: u64,
    threshold: u64,
}
impl<'a> ArchiveProgress<'a> {
    fn new(task: &'a crate::managers::vtask::Subtask, total: Option<u64>) -> Self {
        match total {
            Some(total) => task.update_download(0, total, false),
            None => task.start_opaque(),
        }
        Self {
            task,
            total,
            last_reported: 0,
            threshold: total.map(|n| (n / 200).max(64 * 1024)).unwrap_or(0),
        }
    }
    fn report(&mut self, current: u64) {
        if let Some(total) = self.total {
            if current.saturating_sub(self.last_reported) >= self.threshold {
                self.task.update_download(current, total, false);
                self.last_reported = current;
            }
        }
    }
    fn finish(self, current: u64) {
        match self.total {
            Some(total) => {
                self.task.update_download(current, total, false);
                self.task.complete_download();
            }
            None => self.task.complete_opaque(),
        }
    }
}
#[cfg(test)]
mod task21_stream_tests {
    use super::*;
    #[tokio::test]
    async fn task21_stream_progress_native_width_final_flush_and_opaque() {
        let task = VisualTask::new(Translation::Test);
        let download = task.subtask(Translation::Test);
        let mut progress = ArchiveProgress::new(&download, Some(5_368_709_120));
        progress.report(4_294_967_296);
        assert_eq!(
            task.downloaded_bytes().await,
            (4_294_967_296, 5_368_709_120)
        );
        progress.finish(4_294_967_297);
        assert_eq!(task.downloaded_bytes().await.0, 4_294_967_297);
        assert_eq!(task.progress_float().await, 1.0);
        let mut unknown = ArchiveProgress::new(&download, None);
        unknown.report(5_368_709_120);
        assert_eq!(task.progress_float().await, 0.0);
        unknown.finish(5_368_709_120);
        assert_eq!(task.progress_float().await, 1.0);
    }
}
