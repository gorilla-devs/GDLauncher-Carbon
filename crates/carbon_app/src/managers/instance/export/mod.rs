use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zip::{
    ZipWriter,
    write::{FileOptionExtension, FileOptions},
};

use carbon_repos::db::{
    curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb,
    modrinth_mod_cache as mrdb,
};

use crate::{
    api::translation::Translation,
    domain::{
        instance::{ExportEntry, ExportTarget, InstanceId},
        vtask::{Progress, VisualTaskId},
    },
    managers::{
        ManagerRef,
        account::gdl_account::{
            InstanceShareError, ShareMetadata, SharedMod, WaitForShareInstanceResponse,
        },
        vtask::{Subtask, VisualTask},
    },
};

use super::InvalidInstanceIdError;

mod curseforge_archive;
pub mod gdlauncher_archive;
mod modrinth_archive;

/// Extract basic metadata from a GDLPack zip file
/// Returns ShareMetadata with basic pack info (no mods list since they're hash-based)
fn extract_gdlpack_metadata(zip_path: &Path) -> anyhow::Result<ShareMetadata> {
    use carbon_platforms::gdlauncher::manifest::schema::v1::Manifest as GdlpackManifest;
    use std::io::Read as StdRead;

    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut manifest_file = archive.by_name("gdlpack.json")?;
    let mut manifest_content = String::new();
    manifest_file.read_to_string(&mut manifest_content)?;
    drop(manifest_file);

    let manifest: GdlpackManifest = serde_json::from_str(&manifest_content)?;

    // Extract minecraft version
    let minecraft_version = Some(manifest.dependencies.minecraft);

    // Extract modloader info from the primary modloader
    let (modloader_type, modloader_version) = manifest
        .dependencies
        .modloaders
        .iter()
        .find(|ml| ml.primary)
        .or_else(|| manifest.dependencies.modloaders.first())
        .map(|ml| {
            let loader_type = match ml.type_ {
                carbon_platforms::gdlauncher::manifest::schema::v1::ModloaderType::Forge => "forge",
                carbon_platforms::gdlauncher::manifest::schema::v1::ModloaderType::Neoforge => {
                    "neoforge"
                }
                carbon_platforms::gdlauncher::manifest::schema::v1::ModloaderType::Fabric => {
                    "fabric"
                }
                carbon_platforms::gdlauncher::manifest::schema::v1::ModloaderType::Quilt => "quilt",
            };
            (Some(loader_type.to_string()), Some(ml.version.clone()))
        })
        .unwrap_or((None, None));

    // GDLPack files are hash-based, we don't have mod metadata here
    // The mods list will be empty - enrichment happens on the backend
    let mods: Vec<SharedMod> = Vec::new();

    Ok(ShareMetadata {
        minecraft_version,
        modloader_type,
        modloader_version,
        mods,
    })
}

/// Build SharedMod list from the instance's ModFileCache
/// This is used for sharing to get accurate mod metadata with platform IDs
async fn build_shared_mods_from_cache(
    app: &crate::managers::App,
    instance_id: InstanceId,
) -> Vec<SharedMod> {
    // Query ModFileCache for all mods in this instance with their platform metadata
    let cached_files = app
        .prisma_client
        .mod_file_cache()
        .find_many(vec![
            fcdb::instance_id::equals(*instance_id),
            fcdb::addon_type::equals("mods".to_string()),
        ])
        .with(
            fcdb::metadata::fetch()
                .with(metadb::curseforge::fetch())
                .with(metadb::modrinth::fetch()),
        )
        .exec()
        .await;

    let Ok(cached_files) = cached_files else {
        tracing::warn!("Failed to query ModFileCache for instance {}", *instance_id);
        return Vec::new();
    };

    let mut mods = Vec::new();

    for cached_file in cached_files {
        let Some(metadata) = cached_file.metadata else {
            continue;
        };

        let metadata = *metadata;

        // Try to build SharedMod from CurseForge or Modrinth data
        let curseforge = metadata.curseforge.flatten();
        let modrinth = metadata.modrinth.flatten();

        // Skip files that have no platform data
        if curseforge.is_none() && modrinth.is_none() {
            continue;
        }

        let name = curseforge
            .as_ref()
            .map(|cf| cf.name.clone())
            .or_else(|| modrinth.as_ref().map(|mr| mr.title.clone()))
            .or_else(|| metadata.name.clone())
            .unwrap_or_else(|| cached_file.filename.clone());

        mods.push(SharedMod {
            name,
            curseforge_project_id: curseforge.as_ref().map(|cf| cf.project_id),
            curseforge_file_id: curseforge.as_ref().map(|cf| cf.file_id),
            curseforge_slug: curseforge.as_ref().map(|cf| cf.urlslug.clone()),
            modrinth_project_id: modrinth.as_ref().map(|mr| mr.project_id.clone()),
            modrinth_version_id: modrinth.as_ref().map(|mr| mr.version_id.clone()),
            modrinth_slug: modrinth.as_ref().map(|mr| mr.urlslug.clone()),
        });
    }

    mods
}

/// Enrich ShareMetadata mods with names and slugs from the database caches
async fn enrich_share_metadata(
    app: &crate::managers::App,
    mut metadata: ShareMetadata,
) -> ShareMetadata {
    // --- Enrich from CurseForge cache ---
    let cf_project_ids: Vec<i32> = metadata
        .mods
        .iter()
        .filter_map(|m| m.curseforge_project_id)
        .collect();

    if !cf_project_ids.is_empty() {
        let cf_cache_results = app
            .prisma_client
            .curse_forge_mod_cache()
            .find_many(vec![cfdb::project_id::in_vec(cf_project_ids)])
            .with(cfdb::metadata::fetch().with(metadb::modrinth::fetch()))
            .exec()
            .await;

        if let Ok(cf_entries) = cf_cache_results {
            let cf_map: HashMap<i32, _> = cf_entries
                .into_iter()
                .map(|entry| (entry.project_id, entry))
                .collect();

            for shared_mod in &mut metadata.mods {
                if let Some(cf_project_id) = shared_mod.curseforge_project_id {
                    if let Some(cf_entry) = cf_map.get(&cf_project_id) {
                        shared_mod.name = cf_entry.name.clone();
                        shared_mod.curseforge_slug = Some(cf_entry.urlslug.clone());

                        // Check for Modrinth cross-reference via metadata
                        if let Some(Some(mr_data)) =
                            cf_entry.metadata.as_ref().and_then(|m| m.modrinth.as_ref())
                        {
                            shared_mod.modrinth_project_id = Some(mr_data.project_id.clone());
                            shared_mod.modrinth_version_id = Some(mr_data.version_id.clone());
                            shared_mod.modrinth_slug = Some(mr_data.urlslug.clone());
                        }
                    }
                }
            }
        }
    }

    // --- Enrich Modrinth-only mods (not already enriched via CurseForge) ---
    let mr_project_ids: Vec<String> = metadata
        .mods
        .iter()
        .filter(|m| m.curseforge_project_id.is_none())
        .filter_map(|m| m.modrinth_project_id.clone())
        .collect();

    if !mr_project_ids.is_empty() {
        let mr_cache_results = app
            .prisma_client
            .modrinth_mod_cache()
            .find_many(vec![mrdb::project_id::in_vec(mr_project_ids)])
            .exec()
            .await;

        if let Ok(mr_entries) = mr_cache_results {
            let mr_map: HashMap<String, _> = mr_entries
                .into_iter()
                .map(|entry| (entry.project_id.clone(), entry))
                .collect();

            for shared_mod in &mut metadata.mods {
                if shared_mod.curseforge_project_id.is_some() {
                    continue; // Already enriched via CurseForge path
                }
                if let Some(mr_project_id) = &shared_mod.modrinth_project_id {
                    if let Some(mr_entry) = mr_map.get(mr_project_id) {
                        shared_mod.name = mr_entry.title.clone();
                        shared_mod.modrinth_slug = Some(mr_entry.urlslug.clone());
                    }
                }
            }
        }
    }

    metadata
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareInstanceProgress {
    Progress(i32),
}

#[derive(Debug)]
pub struct InstanceExportManager {}

impl InstanceExportManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, InstanceExportManager> {
    pub async fn export_instance(
        self,
        instance_id: InstanceId,
        target: ExportTarget,
        save_path: PathBuf,
        self_contained_addons_bundling: bool,
        filter: ExportEntry,
        version: String,
    ) -> anyhow::Result<VisualTaskId> {
        match target {
            ExportTarget::Curseforge => curseforge_archive::export_curseforge(
                self.app.clone(),
                instance_id,
                save_path,
                self_contained_addons_bundling,
                filter,
                Some(version),
            )
            .await
            .map(|v| v.0),
            ExportTarget::Modrinth => modrinth_archive::export_modrinth(
                self.app.clone(),
                instance_id,
                save_path,
                self_contained_addons_bundling,
                filter,
                Some(version),
            )
            .await
            .map(|v| v.0),
            ExportTarget::Gdlauncher => {
                gdlauncher_archive::export_gdlauncher(
                    self.app.clone(),
                    instance_id,
                    save_path,
                    self_contained_addons_bundling,
                    filter,
                    None,
                )
                .await
            }
        }
    }

    pub async fn share_instance(
        self,
        instance_id: InstanceId,
        title: Option<String>,
        expiration_days: Option<i32>,
        max_downloads: Option<i32>,
        include_saves: bool,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<(
        mpsc::Receiver<ShareInstanceProgress>,
        tokio::task::JoinHandle<Result<String, anyhow::Error>>,
    )> {
        let app = self.app.clone();
        let title = title.clone();
        let expiration_days = expiration_days;
        let max_downloads = max_downloads;

        let tmpdir = app
            .settings_manager()
            .runtime_path
            .get_temp()
            .maketmpdir()
            .await?;

        let instance_manager = app.instance_manager();
        let instances = instance_manager.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let basepath = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath)
            .get_data_path();

        let mut hashmap = HashMap::new();

        for entry in fs::read_dir(basepath)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy().to_string();

            // `saves` holds the user's worlds and player data. Off by default
            // because (a) it can be very large, (b) most shares are "give me
            // your modpack, I'll start a new world" — opt-in via the UI when
            // the user actually wants their world to travel with the share.
            if name == "saves" && !include_saves {
                continue;
            }

            hashmap.insert(name, None);
        }

        drop(instances);

        let filter = ExportEntry(hashmap);

        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let handle = tokio::spawn(async move {
            let phases_count = 4;
            let mut current_phase = 0;

            // Calculate phase weights as floats
            let first_phase_weight = 25.0; // 25% for export
            let second_phase_weight = 25.0; // 25% for checksum
            let third_phase_weight = 10.0; // 10% for getting URL
            let fourth_phase_weight = 40.0; // 40% for upload
            let mut total_progress = 0.0;

            // First phase: Export the instance
            tx.send(ShareInstanceProgress::Progress(0)).await?;

            let rand_uuid = Uuid::new_v4();
            let initial_tmpfile = tmpdir.join(format!("{}.gdlpack", rand_uuid));
            let vtask_id = gdlauncher_archive::export_gdlauncher(
                app.clone(),
                instance_id,
                initial_tmpfile.clone(),
                false,
                filter,
                Some(cancel_token.clone()),
            )
            .await?;

            // Wait for export task to complete while reporting progress
            loop {
                if cancel_token.is_cancelled() {
                    tracing::info!("ShareInstance: cancelled during phase 1 (export)");
                    anyhow::bail!("Share cancelled");
                }
                let vtask = app.task_manager().get_task(vtask_id).await;
                if let Some(vtask) = vtask {
                    match &vtask.progress {
                        Progress::Known(progress) => {
                            // Scale progress to phase weight (0-25%)
                            let normalized_progress = (progress * first_phase_weight) as i32;
                            let _ = tx
                                .send(ShareInstanceProgress::Progress(normalized_progress))
                                .await;
                        }
                        Progress::Failed(err) => {
                            anyhow::bail!("Export task failed: {err}");
                        }
                        _ => {}
                    }

                    // Check if task is complete
                    if vtask.progress == Progress::Known(1.0) {
                        break;
                    }
                } else {
                    // Task no longer exists, assume it's done
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Update total progress after first phase
            total_progress += first_phase_weight;
            tx.send(ShareInstanceProgress::Progress(total_progress as i32))
                .await?;

            if cancel_token.is_cancelled() {
                tracing::info!(
                    "ShareInstance: cancelled between phase 1 (export) and phase 2 (checksum)"
                );
                anyhow::bail!("Share cancelled");
            }

            // Extract metadata from the GDLPack zip and the instance's mod cache
            let metadata = {
                let zip_path = initial_tmpfile.clone();
                let mut basic_metadata =
                    tokio::task::spawn_blocking(move || extract_gdlpack_metadata(&zip_path))
                        .await?
                        .unwrap_or_default();

                // Build mods list from the instance's ModFileCache (not from the manifest)
                // This gives us accurate platform IDs for the share preview
                basic_metadata.mods = build_shared_mods_from_cache(&app, instance_id).await;

                // Enrich with names/slugs from database (fills in any missing data)
                enrich_share_metadata(&app, basic_metadata).await
            };

            // Second phase: Calculate the checksum
            current_phase += 1;

            let content_length = tokio::fs::metadata(&initial_tmpfile).await?.len();
            let sha256_checksum = {
                use base64::{Engine as _, engine::general_purpose};
                use sha2::{Digest, Sha256};
                let mut file = tokio::fs::File::open(&initial_tmpfile).await?;
                let mut hasher = Sha256::new();
                let mut buffer = vec![0; 1024 * 1024]; // 1MB buffer

                // Get the total file size for progress calculation
                let total_size = content_length;
                let mut bytes_processed = 0;

                carbon_scheduler::buffered_digest(&mut file, |chunk| {
                    bytes_processed += chunk.len() as u64;
                    hasher.update(chunk);

                    // Update progress
                    let phase_progress =
                        (bytes_processed as f64 / total_size as f64) * second_phase_weight;
                    let current_progress = total_progress + phase_progress as f32;

                    let _ =
                        tx.blocking_send(ShareInstanceProgress::Progress(current_progress as i32));
                })
                .await?;

                general_purpose::STANDARD.encode(hasher.finalize())
            };

            // Update total progress after second phase
            total_progress += second_phase_weight as f32;
            tx.send(ShareInstanceProgress::Progress(total_progress as i32))
                .await?;

            if cancel_token.is_cancelled() {
                tracing::info!(
                    "ShareInstance: cancelled between phase 2 (checksum) and phase 3 (presigned URL)"
                );
                anyhow::bail!("Share cancelled");
            }

            let Some(gdl_account_uuid) = app
                .settings_manager()
                .get_settings()
                .await?
                .gdl_account_uuid
            else {
                return Err(anyhow::anyhow!(
                    "no gdl account found for instance {instance_id}"
                ));
            };

            // Third phase: Get the presigned upload URL
            current_phase += 1;

            let presigned_response = app
                .account_manager()
                .get_presigned_upload_url(
                    gdl_account_uuid.clone(),
                    content_length,
                    sha256_checksum.clone(),
                    title,
                    expiration_days,
                    max_downloads,
                    metadata,
                )
                .await?;

            // Update total progress after third phase
            total_progress += third_phase_weight;
            tx.send(ShareInstanceProgress::Progress(total_progress as i32))
                .await?;

            if cancel_token.is_cancelled() {
                tracing::info!(
                    "ShareInstance: cancelled between phase 3 (presigned URL) and phase 4 (upload)"
                );
                anyhow::bail!("Share cancelled");
            }

            // Fourth phase: Upload the file
            current_phase += 1;

            let file = tokio::fs::File::open(&initial_tmpfile).await?;

            let (upload_tx, mut upload_rx) = tokio::sync::mpsc::channel::<i32>(1);

            let tx_clone = tx.clone();
            let final_progress = total_progress;
            tokio::spawn(async move {
                while let Some(progress) = upload_rx.recv().await {
                    let scaled_progress =
                        final_progress + (progress as f32 * fourth_phase_weight / 100.0);
                    let _ = tx_clone
                        .send(ShareInstanceProgress::Progress(scaled_progress as i32))
                        .await;
                }
            });

            app.account_manager()
                .upload_share_instance(
                    gdl_account_uuid,
                    presigned_response.url,
                    file,
                    content_length,
                    sha256_checksum,
                    upload_tx,
                    cancel_token,
                )
                .await?;

            Ok(presigned_response.file_key.clone())
        });

        Ok((rx, handle))
    }

    pub async fn wait_for_share_instance(
        self,
        file_key: String,
        instance_id: Option<InstanceId>,
    ) -> anyhow::Result<WaitForShareInstanceResponse> {
        tracing::info!(
            "wait_for_share_instance called with file_key={}, instance_id={:?}",
            file_key,
            instance_id
        );

        let Some(gdl_account_uuid) = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .gdl_account_uuid
        else {
            return Err(anyhow::anyhow!("no gdl account found"));
        };

        let response = self
            .app
            .account_manager()
            .wait_for_share_instance(gdl_account_uuid.clone(), file_key)
            .await?;

        // Upload instance background if available. The icon is reused as the
        // share background, but the user didn't necessarily choose it with
        // sharing in mind — so format/size issues are downgraded to a warning
        // and the share stays. Only content-moderation signals roll back the
        // share, since those indicate the uploaded image itself is not
        // acceptable and the user should reshare with a different icon.
        if let Some(instance_id) = instance_id {
            if let Err(e) = self
                .upload_instance_background(&gdl_account_uuid, &response.share_code, instance_id)
                .await
            {
                let is_moderation_rejection =
                    e.downcast_ref::<InstanceShareError>().is_some_and(|se| {
                        matches!(
                            se,
                            InstanceShareError::ImageRejectedByModeration
                                | InstanceShareError::ModerationUnavailable
                                | InstanceShareError::ModerationRateLimited
                        )
                    });

                if is_moderation_rejection {
                    if let Err(del_err) = self
                        .app
                        .account_manager()
                        .delete_share(gdl_account_uuid.clone(), response.share_code.clone())
                        .await
                    {
                        tracing::error!(
                            "Failed to roll back share {} after background rejection: {}",
                            response.share_code,
                            del_err
                        );
                    }
                    return Err(e);
                }

                tracing::warn!(
                    "Failed to upload instance background for share {}: {}",
                    response.share_code,
                    e
                );
            }
        }

        Ok(response)
    }

    /// Upload the instance background to the share (best-effort)
    async fn upload_instance_background(
        &self,
        gdl_account_uuid: &str,
        share_code: &str,
        instance_id: InstanceId,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Attempting to upload background for share {} from instance {:?}",
            share_code,
            instance_id
        );

        // Use the existing instance_icon method which handles path resolution correctly
        let Some((_icon_name, image_data)) = self
            .app
            .instance_manager()
            .instance_icon(instance_id)
            .await?
        else {
            tracing::info!(
                "Instance {:?} has no custom icon, skipping background upload",
                instance_id
            );
            return Ok(());
        };

        // Upload to enderium
        self.app
            .account_manager()
            .upload_share_background(
                gdl_account_uuid.to_string(),
                share_code.to_string(),
                image_data,
            )
            .await?;

        tracing::info!(
            "Uploaded background for share {} from instance {}",
            share_code,
            instance_id
        );

        Ok(())
    }
}
enum ZipMode<'a, W: io::Write + io::Seek, T: FileOptionExtension + Clone> {
    Count(&'a mut u32, &'a mut u64),
    Create(
        &'a mut ZipWriter<W>,
        FileOptions<'a, T>,
        tokio::sync::watch::Sender<(u64, u64)>,
    ),
}

fn zip_excluding<W: io::Write + io::Seek, T: FileOptionExtension + Clone>(
    mut mode: ZipMode<W, T>,
    base_path: &Path,
    prefix: &str,
    filter: &ExportEntry,
) -> anyhow::Result<()> {
    // Create shared counters for the entire operation
    let mut bytes_accumulated = 0;
    let mut files_completed = 0;
    let mut last_notify = std::time::Instant::now();
    let notify_interval = std::time::Duration::from_millis(50);

    fn walk_recursive<W: io::Write + io::Seek, T: FileOptionExtension + Clone>(
        mode: &mut ZipMode<W, T>,
        path: &Path,
        prefix: &str,
        relpath: &[&str],
        filter: Option<&ExportEntry>,
        bytes_accumulated: &mut u64,
        files_completed: &mut u64,
        last_notify: &mut std::time::Instant,
        notify_interval: std::time::Duration,
    ) -> anyhow::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();

            let Some(subfilter) = filter
                .as_ref()
                .map(|f| f.0.get(&*name))
                .unwrap_or(Some(&None))
            else {
                continue;
            };

            let pathstr =
                String::from(prefix) + "/" + &relpath.iter().chain([&*name].iter()).join("/");

            if entry.metadata()?.is_dir() {
                let relpath = &[relpath, &[&*name][..]].concat()[..];
                walk_recursive(
                    mode,
                    &entry.path(),
                    prefix,
                    relpath,
                    subfilter.as_ref(),
                    bytes_accumulated,
                    files_completed,
                    last_notify,
                    notify_interval,
                )?;
            } else if entry.metadata()?.is_symlink() {
                continue;
            } else {
                match mode {
                    ZipMode::Count(counter, size) => {
                        **counter += 1;
                        **size += entry.metadata()?.len();
                    }
                    ZipMode::Create(zip, options, notify) => {
                        zip.start_file(pathstr, options.clone())?;
                        let mut file = File::open(entry.path())?;
                        let total_size = entry.metadata()?.len();

                        // Adaptive buffer: use file size or cap at 1MB
                        let buffer_size = std::cmp::min(total_size as usize, 1024 * 1024); // 1MB max
                        let mut buffer = vec![0; buffer_size];
                        let mut bytes_copied = 0;

                        loop {
                            let bytes_read = file.read(&mut buffer)?;
                            if bytes_read == 0 {
                                break;
                            }

                            zip.write_all(&buffer[..bytes_read])?;
                            bytes_copied += bytes_read as u64;
                            *bytes_accumulated += bytes_read as u64;

                            // Report progress every 200ms
                            let now = std::time::Instant::now();
                            if now.duration_since(*last_notify) >= notify_interval {
                                let _ = notify.send((*bytes_accumulated, *files_completed));
                                *bytes_accumulated = 0;
                                *files_completed = 0;
                                *last_notify = now;
                            }
                        }

                        // Increment file counter after each completed file
                        *files_completed += 1;

                        // Only send notification if we've exceeded the interval
                        let now = std::time::Instant::now();
                        if now.duration_since(*last_notify) >= notify_interval {
                            let _ = notify.send((*bytes_accumulated, *files_completed));
                            *bytes_accumulated = 0;
                            *files_completed = 0;
                            *last_notify = now;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Pass the shared counters to the recursive function
    let result = walk_recursive(
        &mut mode,
        base_path,
        prefix,
        &[],
        Some(filter),
        &mut bytes_accumulated,
        &mut files_completed,
        &mut last_notify,
        notify_interval,
    );

    // Send any final accumulated data
    if let ZipMode::Create(_, _, notify) = &mode {
        if bytes_accumulated > 0 || files_completed > 0 {
            let _ = notify.send((bytes_accumulated, files_completed));
        }
    }

    result
}
