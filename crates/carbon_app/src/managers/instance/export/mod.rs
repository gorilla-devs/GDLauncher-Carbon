use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use itertools::Itertools;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zip::{
    ZipWriter,
    write::{FileOptionExtension, FileOptions},
};

use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::mod_metadata as metarepo;

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

#[cfg(test)]
mod task12_tests;

/// A linked mod as the user sees it in the instance, e.g. `mods/jei.jar`.
fn linked_mod_name(path: &Path) -> String {
    let name = |path: Option<&Path>| {
        path.and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    format!("{}/{}", name(path.parent()), name(Some(path)))
}

/// When a linked mod is checked: before the export links it, or in the final
/// re-read of every linked mod once the archive is written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LinkedRead {
    BeforeLinking,
    FinalCheck,
}

/// Confirm a mod the export links to its platform file instead of bundling
/// it still holds the bytes the link names. `stored` is the mod's row stamp.
///
/// The export's addon pass has just read every mod and re-stamped the rows
/// whose bytes matched, so a file whose stamp still equals its row's is only
/// opened here, not read again: the final re-read of every linked mod
/// ([`verify_linked_export_files`]) is the byte guard, also for a change no
/// stamp shows. Any other file is read now, so a changed mod stops the export
/// before the archive is written. A missing or unreadable mod fails here either
/// way.
///
/// Every error is one plain sentence naming the file and what to do: the
/// export window shows it as is.
async fn verify_linked_mod(path: &Path, sha512: &[u8], stored: Option<&str>) -> anyhow::Result<()> {
    let unchanged = crate::managers::metadata::cache::opened_stamp_matches(path, stored)
        .await
        .map_err(|error| linked_mod_error(path, &error, LinkedRead::BeforeLinking))?;
    if !unchanged {
        read_linked_mod(path, sha512, LinkedRead::BeforeLinking).await?;
    }
    Ok(())
}

/// The plain sentence the export window shows when a linked mod cannot be
/// opened or read.
fn linked_mod_error(path: &Path, error: &io::Error, when: LinkedRead) -> anyhow::Error {
    let name = linked_mod_name(path);
    if error.kind() == io::ErrorKind::NotFound && when == LinkedRead::BeforeLinking {
        // Also a link whose target was moved, deleted or unmounted:
        // retrying alone never helps.
        anyhow::anyhow!(
            "{name} is missing or links to a file that no longer exists. Restore it, or untick it in the export's file list, then try again."
        )
    } else if error.kind() == io::ErrorKind::NotFound {
        // It was checked before it was linked, so it went during the export.
        anyhow::anyhow!(
            "{name} was moved or deleted while the instance was being exported. Try the export again."
        )
    } else {
        anyhow::anyhow!(
            "Could not read {name} ({error}). Close any program that is using it, or untick it in the export's file list, then try again."
        )
    }
}

/// Read a linked mod and confirm it holds the bytes the link names. Returns
/// the file's stamp when nothing about it changed during the read.
async fn read_linked_mod(
    path: &Path,
    sha512: &[u8],
    when: LinkedRead,
) -> anyhow::Result<Option<String>> {
    let name = linked_mod_name(path);
    let (hash, stamp) = crate::managers::metadata::cache::read_stable_file_hash(path)
        .await
        .map_err(|error| linked_mod_error(path, &error, when))?;
    if hash != sha512 {
        anyhow::bail!(
            "{name} changed while the instance was being exported. Try the export again."
        );
    }
    Ok(stamp)
}

/// The final byte guard before an export is published: every linked mod is
/// read again so a file replaced during the export never ships as its old
/// platform file.
///
/// Files are read without the instance gate, so Play, toggles and settings
/// never wait on an export. The gate is then held only to confirm, from each
/// file's stamp, that nothing changed since it was read; a file that did is
/// read again with the gate released.
async fn verify_linked_export_files(
    app: &crate::managers::App,
    instance_id: InstanceId,
    files: &[(PathBuf, Vec<u8>)],
) -> anyhow::Result<()> {
    let mut stamps = vec![None; files.len()];
    let mut stale = (0..files.len()).collect::<Vec<_>>();
    for _ in 0..3 {
        for &index in &stale {
            let (path, sha512) = &files[index];
            #[cfg(test)]
            app.instance_manager()
                .test_hooks
                .pause("export_final_read", instance_id)
                .await;
            stamps[index] = read_linked_mod(path, sha512, LinkedRead::FinalCheck).await?;
        }
        #[cfg(test)]
        app.instance_manager()
            .test_hooks
            .pause("export_final_check", instance_id)
            .await;
        let _ownership = app.instance_manager().lock_instance(instance_id).await;
        stale.clear();
        for (index, (path, _)) in files.iter().enumerate() {
            let current = crate::managers::metadata::cache::current_file_stamp(path).await;
            if current.is_none() || current != stamps[index] {
                stale.push(index);
            }
        }
        if stale.is_empty() {
            return Ok(());
        }
    }
    anyhow::bail!(
        "{} kept changing while the instance was being exported. Close any program that is changing it, then try the export again.",
        linked_mod_name(&files[stale[0]].0)
    )
}

/// Wait for authoritative export settlement while forwarding share progress.
async fn wait_for_share_export(
    app: &crate::managers::App,
    vtask_id: VisualTaskId,
    tx: &mpsc::Sender<ShareInstanceProgress>,
    first_phase_weight: f32,
    cancel_token: &CancellationToken,
) -> anyhow::Result<()> {
    use crate::managers::vtask::TaskOutcome;

    tokio::select! {
        biased;
        _ = cancel_token.cancelled() => {
            tracing::info!("ShareInstance: cancelled during phase 1 (export)");
            anyhow::bail!("Share cancelled");
        }
        result = async {
            loop {
                match app.task_manager().get_task_outcome(vtask_id).await {
                    TaskOutcome::Running(task) => {
                        if let Progress::Known(progress) = task.progress {
                            let normalized_progress = (progress * first_phase_weight) as i32;
                            let _ = tx.send(ShareInstanceProgress::Progress(normalized_progress)).await;
                        }
                    }
                    TaskOutcome::Succeeded => return Ok(()),
                    TaskOutcome::Failed(task) => {
                        if let Progress::Failed(err) = task.progress {
                            return Err(share_export_error(&err));
                        }
                        anyhow::bail!("Export task failed");
                    }
                    TaskOutcome::Dismissed => anyhow::bail!("Export task was dismissed"),
                    TaskOutcome::Unavailable => anyhow::bail!("Export task outcome is unavailable"),
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        } => result,
    }
}

/// The error a share reports for its failed export. A share that is too large
/// or a disk that filled up keep their own messages so the share window can
/// say what to do; anything else is reported as the export's failure.
fn share_export_error(error: &anyhow::Error) -> anyhow::Error {
    if let Some(too_large) = error.downcast_ref::<InstanceTooLargeError>() {
        return too_large.clone().into();
    }
    if is_disk_full(error) {
        return InstanceShareError::NotEnoughDiskSpace.into();
    }
    anyhow::anyhow!("Export task failed: {error}")
}

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
    let instance_id_val = *instance_id;
    let cached_files = mfcdb::get_instance_export_mods(&app.db, instance_id_val).await;

    let Ok(cached_files) = cached_files else {
        tracing::warn!("Failed to query ModFileCache for instance {}", *instance_id);
        return Vec::new();
    };

    let mut mods = Vec::new();

    for row in cached_files {
        let cf_present = row.cf_project_id.is_some();
        let mr_present = row.mr_project_id.is_some();

        // Skip files that have no platform data
        if !cf_present && !mr_present {
            continue;
        }

        let name = row
            .cf_name
            .clone()
            .or_else(|| row.mr_title.clone())
            .or_else(|| row.meta_name.clone())
            .unwrap_or_else(|| row.filename.clone());

        mods.push(SharedMod {
            name,
            curseforge_project_id: row.cf_project_id,
            curseforge_file_id: row.cf_file_id,
            curseforge_slug: row.cf_urlslug.clone(),
            modrinth_project_id: row.mr_project_id.clone(),
            modrinth_version_id: row.mr_version_id.clone(),
            modrinth_slug: row.mr_urlslug.clone(),
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
        // rusqlite 0.25.4 has no array-param binding, so the `IN (list)` read is
        // fanned out into one registered per-project-id query; the results are
        // then collapsed into the same `projectId -> row` map as before.
        let mut cf_cache_results: anyhow::Result<Vec<metarepo::CfExportEnrichRow>> = Ok(Vec::new());
        for project_id in cf_project_ids {
            match metarepo::get_cf_export_enrich_by_project(&app.db, project_id).await {
                Ok(rows) => {
                    if let Ok(acc) = &mut cf_cache_results {
                        acc.extend(rows);
                    }
                }
                Err(e) => {
                    cf_cache_results = Err(e.into());
                    break;
                }
            }
        }

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
                        if let (Some(mr_project_id), Some(mr_version_id), Some(mr_urlslug)) = (
                            cf_entry.mr_project_id.as_ref(),
                            cf_entry.mr_version_id.as_ref(),
                            cf_entry.mr_urlslug.as_ref(),
                        ) {
                            shared_mod.modrinth_project_id = Some(mr_project_id.clone());
                            shared_mod.modrinth_version_id = Some(mr_version_id.clone());
                            shared_mod.modrinth_slug = Some(mr_urlslug.clone());
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
        let mut mr_cache_results: anyhow::Result<Vec<metarepo::MrExportEnrichRow>> = Ok(Vec::new());
        for project_id in mr_project_ids {
            match metarepo::get_mr_export_enrich_by_project(&app.db, &project_id).await {
                Ok(rows) => {
                    if let Ok(acc) = &mut mr_cache_results {
                        acc.extend(rows);
                    }
                }
                Err(e) => {
                    mr_cache_results = Err(e.into());
                    break;
                }
            }
        }

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
                    Some(version),
                    None,
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

        let filter = build_share_filter(&basepath, include_saves)?;

        drop(instances);

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
                None,
                Some(cancel_token.clone()),
                // Stops writing, and fails with the folders to trim, as soon as
                // the archive would pass the share limit.
                Some(MAX_SHARE_SIZE_BYTES),
            )
            .await?;

            wait_for_share_export(&app, vtask_id, &tx, first_phase_weight, &cancel_token).await?;

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

            // The export already refused to grow the archive past the limit;
            // check the finished file again before checksum, authentication
            // or upload anyway.
            let content_length = tokio::fs::metadata(&initial_tmpfile).await?.len();
            if content_length > MAX_SHARE_SIZE_BYTES {
                return Err(InstanceTooLargeError {
                    total_bytes: content_length,
                    limit_bytes: MAX_SHARE_SIZE_BYTES,
                    largest_folders: Vec::new(),
                }
                .into());
            }

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

        // Reuse the instance's own icon as the share background.
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

/// Maximum compressed size of a completed shared instance archive. Mirrors
/// enderium's authoritative `MAX_CONTENT_LENGTH`. The share export stops
/// writing as soon as its archive would grow past it; uncompressed on-disk
/// sizes are trimming diagnostics and do not decide whether an archive fits.
pub const MAX_SHARE_SIZE_BYTES: u64 = 512 * 1024 * 1024;

/// How many of the largest top-level entries to surface when a share is
/// rejected for being too large.
const SHARE_BREAKDOWN_TOP_N: usize = 5;

/// Top-level instance folders that are never included in a share: regenerable,
/// machine/world-specific caches that would only bloat the upload because the
/// recipient's install recreates them. `Distant_Horizons_server_data` is the
/// Distant Horizons LOD cache (server/multiplayer render-distance data) and is
/// commonly several GB — the usual reason a share blows past the size cap.
/// Distant Horizons' singleplayer LODs instead live inside `saves/<world>/data`
/// and are already covered by the `saves` exclusion.
const SHARE_EXCLUDED_FOLDERS: &[&str] = &["Distant_Horizons_server_data"];

/// Why an [`ArchiveSink`] stopped taking bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveStop {
    /// The archive would have grown past this many bytes.
    Limit(u64),
    /// The export was cancelled.
    Cancelled,
}

impl ArchiveStop {
    fn io_error(self) -> io::Error {
        match self {
            Self::Limit(limit) => {
                io::Error::other(format!("the archive would be larger than {limit} bytes"))
            }
            Self::Cancelled => io::Error::other("Export cancelled"),
        }
    }

    /// The error an export reports once its archive stopped for this reason.
    /// `bundled` is what the archive bundles, so a too-large share names only
    /// folders that count towards its size (linked mods are left out).
    fn into_error(self, base_path: &Path, bundled: &ExportEntry) -> anyhow::Error {
        match self {
            Self::Limit(limit) => InstanceTooLargeError::measure(base_path, bundled, limit).into(),
            Self::Cancelled => anyhow::anyhow!("Export cancelled"),
        }
    }
}

/// The file an export writes its archive to. A share passes its size limit
/// and cancel token, so an archive that cannot fit, or that the user
/// cancelled, stops at its next write instead of compressing the rest of the
/// instance into temp first. Once stopped it refuses every later write.
struct ArchiveSink<W> {
    inner: W,
    position: u64,
    limit: Option<u64>,
    cancel: Option<CancellationToken>,
    stopped: Arc<OnceLock<ArchiveStop>>,
}

impl<W> ArchiveSink<W> {
    /// Returns the sink and the handle that tells why it stopped, which stays
    /// readable after the zip writer that owns the sink is gone.
    fn new(
        inner: W,
        limit: Option<u64>,
        cancel: Option<CancellationToken>,
    ) -> (Self, Arc<OnceLock<ArchiveStop>>) {
        let stopped = Arc::new(OnceLock::new());
        let sink = Self {
            inner,
            position: 0,
            limit,
            cancel,
            stopped: stopped.clone(),
        };
        (sink, stopped)
    }

    fn stop(&self, reason: ArchiveStop) -> io::Error {
        let _ = self.stopped.set(reason);
        self.stopped.get().copied().unwrap_or(reason).io_error()
    }
}

impl<W: Write> Write for ArchiveSink<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(stop) = self.stopped.get() {
            return Err(stop.io_error());
        }
        if self
            .cancel
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(self.stop(ArchiveStop::Cancelled));
        }
        // The zip writer only seeks back to fill in headers it already wrote,
        // so the archive ends up exactly as long as its furthest write.
        if let Some(limit) = self.limit
            && self.position.saturating_add(buf.len() as u64) > limit
        {
            return Err(self.stop(ArchiveStop::Limit(limit)));
        }
        let written = self.inner.write(buf)?;
        self.position += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: io::Seek> io::Seek for ArchiveSink<W> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.position = self.inner.seek(pos)?;
        Ok(self.position)
    }
}

/// Size of a single top-level entry (folder or file) included in a share.
#[derive(Debug, Clone)]
pub struct ShareFolderSize {
    pub name: String,
    pub bytes: u64,
}

/// Returned when a share's archive would grow past `MAX_SHARE_SIZE_BYTES`.
/// The total and folder breakdown describe the uncompressed bytes the archive
/// bundles (linked mods are downloaded by the recipient and do not count), so
/// the user knows what to trim.
#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "instance archive exceeds the {limit_bytes} byte share limit ({total_bytes} uncompressed bytes)"
)]
pub struct InstanceTooLargeError {
    pub total_bytes: u64,
    pub limit_bytes: u64,
    pub largest_folders: Vec<ShareFolderSize>,
}

impl InstanceTooLargeError {
    /// Measures what `bundled` includes under `base_path`, largest first.
    fn measure(base_path: &Path, bundled: &ExportEntry, limit_bytes: u64) -> Self {
        let (total_bytes, breakdown) = compute_share_breakdown(base_path, bundled);
        Self {
            total_bytes,
            limit_bytes,
            largest_folders: breakdown.into_iter().take(SHARE_BREAKDOWN_TOP_N).collect(),
        }
    }
}

/// Whether `error` comes from a disk (or disk quota) that ran out of space.
fn is_disk_full(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.downcast_ref::<io::Error>().is_some_and(|error| {
            matches!(
                error.kind(),
                io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded
            )
        })
    })
}

/// Build the set of top-level instance entries to include in a share. Skips
/// the always-excluded cache folders (`SHARE_EXCLUDED_FOLDERS`) and, unless the
/// user opted in, the `saves` directory. Each kept entry maps to `None`,
/// meaning "include its whole subtree".
fn build_share_filter(basepath: &Path, include_saves: bool) -> anyhow::Result<ExportEntry> {
    let mut hashmap = HashMap::new();

    for entry in fs::read_dir(basepath)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Regenerable cache folders (e.g. the Distant Horizons LOD cache) only
        // bloat the upload; the recipient's install recreates them.
        if SHARE_EXCLUDED_FOLDERS.contains(&name.as_str()) {
            continue;
        }

        // `saves` holds the user's worlds and player data. Off by default
        // because it can be very large and most shares are "give me your
        // modpack, I'll start a new world"; opt in via the UI to ship it.
        if name == "saves" && !include_saves {
            continue;
        }

        hashmap.insert(name, None);
    }

    Ok(ExportEntry(hashmap))
}

/// Sum the on-disk size of every regular file a share would include, grouped by
/// top-level entry and sorted largest-first. Mirrors `zip_excluding`'s
/// inclusion rules: only entries present in `filter` are counted, a nested
/// filter counts only the children it names (so mods the export links instead
/// of bundling are left out), and symlinks are skipped (so we never follow a
/// link out of the instance or double-count).
///
/// The result only explains a refused share, so unreadable entries simply
/// count as zero.
fn compute_share_breakdown(base_path: &Path, filter: &ExportEntry) -> (u64, Vec<ShareFolderSize>) {
    fn entry_size(path: &Path, filter: Option<&ExportEntry>) -> u64 {
        let Ok(meta) = fs::symlink_metadata(path) else {
            return 0;
        };
        if meta.file_type().is_symlink() {
            return 0;
        }
        if meta.is_dir() {
            let Ok(read) = fs::read_dir(path) else {
                return 0;
            };
            read.flatten()
                .filter_map(|entry| {
                    let subfilter = match filter {
                        None => None,
                        Some(filter) => filter
                            .0
                            .get(&*entry.file_name().to_string_lossy())?
                            .as_ref(),
                    };
                    Some(entry_size(&entry.path(), subfilter))
                })
                .sum()
        } else {
            meta.len()
        }
    }

    let mut folders: Vec<ShareFolderSize> = filter
        .0
        .iter()
        .filter_map(|(name, subfilter)| {
            let bytes = entry_size(&base_path.join(name), subfilter.as_ref());
            (bytes > 0).then(|| ShareFolderSize {
                name: name.clone(),
                bytes,
            })
        })
        .collect();

    folders.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    let total = folders.iter().map(|f| f.bytes).sum();
    (total, folders)
}

#[cfg(test)]
mod share_breakdown_tests {
    use super::*;
    use crate::domain::instance::ExportEntry;

    fn filter_of(names: &[&str]) -> ExportEntry {
        ExportEntry(names.iter().map(|n| (n.to_string(), None)).collect())
    }

    #[test]
    fn sums_included_top_level_entries_and_excludes_others() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        std::fs::create_dir(base.join("mods")).unwrap();
        std::fs::write(base.join("mods").join("a.jar"), vec![0u8; 100]).unwrap();

        std::fs::create_dir_all(base.join("resourcepacks").join("nested")).unwrap();
        std::fs::write(
            base.join("resourcepacks").join("nested").join("big.zip"),
            vec![0u8; 1000],
        )
        .unwrap();

        // Not in the filter — must be ignored (mirrors the `saves` exclusion).
        std::fs::create_dir(base.join("saves")).unwrap();
        std::fs::write(base.join("saves").join("world.dat"), vec![0u8; 5000]).unwrap();

        let (total, folders) =
            compute_share_breakdown(base, &filter_of(&["mods", "resourcepacks"]));

        assert_eq!(total, 1100);
        assert_eq!(folders.len(), 2);
        // Sorted largest-first.
        assert_eq!(folders[0].name, "resourcepacks");
        assert_eq!(folders[0].bytes, 1000);
        assert_eq!(folders[1].name, "mods");
        assert_eq!(folders[1].bytes, 100);
    }

    #[test]
    fn counts_only_what_a_nested_filter_keeps() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        std::fs::create_dir_all(base.join("mods").join("sub")).unwrap();
        std::fs::write(base.join("mods").join("linked.jar"), vec![0u8; 5000]).unwrap();
        std::fs::write(base.join("mods").join("bundled.jar"), vec![0u8; 300]).unwrap();
        std::fs::write(
            base.join("mods").join("sub").join("kept.txt"),
            vec![0u8; 20],
        )
        .unwrap();
        std::fs::write(base.join("mods").join("sub").join("left.txt"), vec![0u8; 7]).unwrap();
        std::fs::create_dir(base.join("config")).unwrap();
        std::fs::write(base.join("config").join("a.toml"), vec![0u8; 40]).unwrap();
        // How the export leaves a linked mod out of what it bundles.
        let filter = ExportEntry(HashMap::from([
            (
                "mods".to_string(),
                Some(ExportEntry(HashMap::from([
                    ("bundled.jar".to_string(), None),
                    ("sub".to_string(), Some(filter_of(&["kept.txt"]))),
                ]))),
            ),
            ("config".to_string(), None),
        ]));

        let (total, folders) = compute_share_breakdown(base, &filter);

        assert_eq!(total, 360);
        let folders = folders
            .iter()
            .map(|folder| (folder.name.as_str(), folder.bytes))
            .collect::<Vec<_>>();
        assert_eq!(folders, [("mods", 320), ("config", 40)]);
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        std::fs::create_dir(base.join("mods")).unwrap();
        std::fs::write(base.join("mods").join("real.jar"), vec![0u8; 200]).unwrap();
        // A symlink inside an included folder must not be counted.
        std::os::unix::fs::symlink(
            base.join("mods").join("real.jar"),
            base.join("mods").join("link.jar"),
        )
        .unwrap();

        let (total, _) = compute_share_breakdown(base, &filter_of(&["mods"]));
        assert_eq!(total, 200);
    }

    #[test]
    fn filter_excludes_cache_folders_and_optional_saves() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        for d in ["mods", "config", "saves", "Distant_Horizons_server_data"] {
            std::fs::create_dir(base.join(d)).unwrap();
        }

        // saves excluded by default; the DH LOD cache is always excluded.
        let f = build_share_filter(base, false).unwrap();
        assert!(f.0.contains_key("mods"));
        assert!(f.0.contains_key("config"));
        assert!(!f.0.contains_key("saves"));
        assert!(!f.0.contains_key("Distant_Horizons_server_data"));

        // Opting into saves ships it, but the DH cache stays excluded.
        let f = build_share_filter(base, true).unwrap();
        assert!(f.0.contains_key("saves"));
        assert!(!f.0.contains_key("Distant_Horizons_server_data"));
    }
}

/// Files at least this large are written with ZIP64 sizes. ZIP's 32-bit size
/// fields end just under 4 GiB and DEFLATE can grow incompressible bytes a
/// little, so the switch leaves some headroom. Smaller files keep the plain
/// headers every reader understands.
const ZIP64_FILE_SIZE: u64 = u32::MAX as u64 - (u32::MAX as u64 >> 8);

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
                        let total_size = entry.metadata()?.len();
                        zip.start_file(
                            pathstr,
                            options.clone().large_file(total_size >= ZIP64_FILE_SIZE),
                        )?;
                        let mut file = File::open(entry.path())?;

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

#[cfg(test)]
mod task11_tests {
    use super::*;
    use crate::domain::instance::AddonType;
    use carbon_repos::dbtypes::DbDateTime;
    #[tokio::test]
    async fn task11_linked_export_rechecks_mutation_before_publication() {
        for (mutate, self_contained, mismatched_hash) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let app = crate::setup_managers_for_test().await;
            let (id, _, path) = super::super::mods::tests::addon_fixture(
                &app,
                "task11-export",
                AddonType::Mods,
                "mod.jar",
                b"first bytes",
            )
            .await;
            app.meta_cache_manager()
                .override_caching_and_wait(
                    crate::managers::metadata::cache::CacheEntityId::Instance(id),
                    false,
                    false,
                )
                .await
                .unwrap();
            let row = mfcdb::get_mod_files_by_instance(&app.db, *id)
                .await
                .unwrap()
                .remove(0);
            let meta = metarepo::get_metadata_by_id(&app.db, &row.metadata_id)
                .await
                .unwrap()
                .unwrap();
            let now = DbDateTime(chrono::Utc::now().fixed_offset());
            metarepo::upsert_mr_mod_cache(
                &app.db,
                if mismatched_hash {
                    "00".repeat(64)
                } else {
                    hex::encode(&meta.sha512)
                },
                "project".into(),
                "version".into(),
                "title".into(),
                "1".into(),
                "slug".into(),
                "desc".into(),
                "".into(),
                2,
                "".into(),
                "mod.jar".into(),
                "https://fixture.invalid/never-requested.jar".into(),
                now,
                row.metadata_id.clone(),
            )
            .await
            .unwrap();
            let context = crate::managers::metadata::cache::modrinth::target_compatibility(
                &app,
                crate::managers::metadata::cache::CacheEntityId::Instance(id),
            )
            .await
            .unwrap();
            metarepo::upsert_mr_update_context(
                &app.db,
                &row.metadata_id,
                &context.key("mods"),
                "",
                now,
            )
            .await
            .unwrap();
            let barrier = app
                .instance_manager()
                .test_hooks
                .arm("task11_export_before_publish", id);
            let output = path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("export.mrpack");
            let (_, worker) = modrinth_archive::export_modrinth(
                app.clone(),
                id,
                output.clone(),
                self_contained,
                ExportEntry([("mods".into(), None)].into_iter().collect()),
                Some("1".into()),
            )
            .await
            .unwrap();
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                barrier.entered.notified(),
            )
            .await
            .unwrap();
            if mutate {
                let stamp = std::fs::metadata(&path).unwrap().modified().unwrap();
                std::fs::write(&path, b"other bytes").unwrap();
                std::fs::File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_times(std::fs::FileTimes::new().set_modified(stamp))
                    .unwrap();
            }
            barrier.release.notify_one();
            worker.await.unwrap();
            assert_eq!(
                output.exists(),
                !mutate,
                "a changed linked source must not publish the old provider artifact"
            );
            if !mutate {
                let mut archive =
                    zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
                let manifest: serde_json::Value =
                    serde_json::from_reader(archive.by_name("modrinth.index.json").unwrap())
                        .unwrap();
                let use_override = self_contained || mismatched_hash;
                assert_eq!(
                    manifest["files"].as_array().unwrap().len(),
                    usize::from(!use_override)
                );
                if use_override {
                    let mut bytes = Vec::new();
                    std::io::Read::read_to_end(
                        &mut archive.by_name("overrides/mods/mod.jar").unwrap(),
                        &mut bytes,
                    )
                    .unwrap();
                    assert_eq!(bytes, b"first bytes");
                } else {
                    assert!(archive.by_name("overrides/mods/mod.jar").is_err());
                }
            }
        }
    }
}

/// What an export links after the addon pass it starts with: the pass re-reads
/// every mod, so a replaced file never ships as its old platform file, and
/// files the export does not depend on never fail it.
#[cfg(test)]
mod linked_export_scan_tests {
    use super::*;
    use crate::domain::instance::AddonType;
    use crate::managers::metadata::cache::CacheEntityId;
    use carbon_repos::dbtypes::DbDateTime;

    const OLD_URL: &str = "https://fixture.invalid/old.jar";

    /// An instance whose `mod.jar` (b"first bytes") is linked to a Modrinth
    /// file, as after installing it from Modrinth.
    async fn linked_instance(app: &crate::managers::App, name: &str) -> (InstanceId, PathBuf) {
        let (id, _, path) = super::super::mods::tests::addon_fixture(
            app,
            name,
            AddonType::Mods,
            "mod.jar",
            b"first bytes",
        )
        .await;
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let row = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.filename == "mod.jar")
            .unwrap();
        let meta = metarepo::get_metadata_by_id(&app.db, &row.metadata_id)
            .await
            .unwrap()
            .unwrap();
        let now = DbDateTime(chrono::Utc::now().fixed_offset());
        metarepo::upsert_mr_mod_cache(
            &app.db,
            hex::encode(&meta.sha512),
            "project".into(),
            "version".into(),
            "title".into(),
            "1".into(),
            "slug".into(),
            "desc".into(),
            "".into(),
            2,
            "".into(),
            "mod.jar".into(),
            OLD_URL.into(),
            now.clone(),
            row.metadata_id.clone(),
        )
        .await
        .unwrap();
        let context = crate::managers::metadata::cache::modrinth::target_compatibility(
            app,
            CacheEntityId::Instance(id),
        )
        .await
        .unwrap();
        metarepo::upsert_mr_update_context(
            &app.db,
            &row.metadata_id,
            &context.key("mods"),
            "",
            now,
        )
        .await
        .unwrap();
        (id, path)
    }

    /// Replace a file's bytes keeping its size and modification time, as some
    /// copy and sync tools do.
    fn replace_preserving_time(path: &Path, bytes: &[u8]) {
        let modified = std::fs::metadata(path).unwrap().modified().unwrap();
        std::fs::write(path, bytes).unwrap();
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
    }

    /// Run a Modrinth export and return the archive, or the task's error.
    async fn export(
        app: &crate::managers::App,
        id: InstanceId,
        path: &Path,
        filter: ExportEntry,
    ) -> Result<zip::ZipArchive<File>, String> {
        let output = path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("export.mrpack");
        let (task, worker) = modrinth_archive::export_modrinth(
            app.clone(),
            id,
            output.clone(),
            false,
            filter,
            Some("1".into()),
        )
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(20), worker)
            .await
            .expect("the export settles")
            .unwrap();
        match app.task_manager().get_task_outcome(task).await {
            crate::managers::vtask::TaskOutcome::Failed(task) => {
                assert!(!output.exists(), "a failed export publishes nothing");
                Err(format!("{:?}", task.progress))
            }
            _ => Ok(zip::ZipArchive::new(File::open(&output).unwrap()).unwrap()),
        }
    }

    fn manifest_urls(archive: &mut zip::ZipArchive<File>) -> Vec<String> {
        let manifest: serde_json::Value =
            serde_json::from_reader(archive.by_name("modrinth.index.json").unwrap()).unwrap();
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|file| file["downloads"].as_array().unwrap().clone())
            .map(|url| url.as_str().unwrap().to_owned())
            .collect()
    }

    fn override_bytes(archive: &mut zip::ZipArchive<File>, name: &str) -> Option<Vec<u8>> {
        let mut entry = archive.by_name(name).ok()?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
        Some(bytes)
    }

    fn all_mods() -> ExportEntry {
        ExportEntry([("mods".into(), None)].into_iter().collect())
    }

    #[tokio::test]
    async fn unchanged_linked_mod_ships_as_its_platform_file() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-linked-control").await;
        let mut archive = export(&app, id, &path, all_mods()).await.unwrap();
        assert_eq!(manifest_urls(&mut archive), vec![OLD_URL.to_owned()]);
        assert_eq!(override_bytes(&mut archive, "overrides/mods/mod.jar"), None);
    }

    /// An export reads a linked mod twice: in its addon pass, which confirms
    /// the mod's bytes and stamp, and in the final re-read once the archive is
    /// written. The check before linking only compares the stamp.
    #[tokio::test]
    async fn an_export_reads_an_unchanged_linked_mod_twice() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-two-reads").await;
        let before = crate::managers::metadata::cache::whole_file_reads(&path);
        let mut archive = export(&app, id, &path, all_mods()).await.unwrap();
        assert_eq!(manifest_urls(&mut archive), vec![OLD_URL.to_owned()]);
        assert_eq!(
            crate::managers::metadata::cache::whole_file_reads(&path) - before,
            2
        );
    }

    /// The check before linking reads a mod only when its stamp no longer
    /// matches its row: new bytes then stop the export before the archive is
    /// written, and a missing mod is named as missing.
    #[tokio::test]
    async fn the_check_before_linking_reads_only_mods_whose_stamp_changed() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-prelink").await;
        let row = mfcdb::get_instance_mr_export_files(&app.db, *id)
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.filename == "mod.jar")
            .unwrap();
        let reads = || crate::managers::metadata::cache::whole_file_reads(&path);

        let before = reads();
        verify_linked_mod(&path, &row.sha512, row.file_stamp.as_deref())
            .await
            .unwrap();
        assert_eq!(reads(), before, "an unchanged stamp is not read");

        verify_linked_mod(&path, &row.sha512, None).await.unwrap();
        assert_eq!(reads(), before + 1, "a row without a stamp is read");

        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::fs::write(&path, b"other bytes").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(modified + std::time::Duration::from_secs(5)),
            )
            .unwrap();
        let error = verify_linked_mod(&path, &row.sha512, row.file_stamp.as_deref())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "mods/mod.jar changed while the instance was being exported. Try the export again."
        );

        std::fs::remove_file(&path).unwrap();
        let error = verify_linked_mod(&path, &row.sha512, row.file_stamp.as_deref())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "mods/mod.jar is missing or links to a file that no longer exists. Restore it, or untick it in the export's file list, then try again."
        );
    }

    /// GPT-0084: a same-size replacement that keeps the old modification time
    /// is re-read at export and ships as its own bytes, not the old file.
    #[tokio::test]
    async fn same_size_time_preserving_replacement_does_not_keep_its_old_link() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-replaced").await;
        replace_preserving_time(&path, b"other bytes");
        // Even when nothing a stamp observes changed (Windows keeps size,
        // modification and creation time), the export reads the bytes.
        crate::managers::metadata::cache::restamp_for_test(&app, id, "mods", "mod.jar", &path)
            .await;
        let mut archive = export(&app, id, &path, all_mods()).await.unwrap();
        assert!(manifest_urls(&mut archive).is_empty());
        assert_eq!(
            override_bytes(&mut archive, "overrides/mods/mod.jar").as_deref(),
            Some(&b"other bytes"[..])
        );
    }

    /// The file is replaced while the export's pass is reading it. Where the
    /// pass sees the change (Unix change time) it re-reads the new bytes;
    /// where nothing observable changes (Windows keeps size, modification and
    /// creation time), the export's final re-read of every linked file stops
    /// it. Either way the old platform file is never shipped for new bytes.
    #[tokio::test]
    async fn replacement_during_the_export_read_never_ships_the_old_link() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-replaced-mid-read").await;
        app.meta_cache_manager()
            .mutate_before_recheck(&path, b"other bytes".to_vec());
        match export(&app, id, &path, all_mods()).await {
            Ok(mut archive) => {
                assert!(
                    manifest_urls(&mut archive).is_empty(),
                    "new bytes must not ship as the old platform file"
                );
                assert_eq!(
                    override_bytes(&mut archive, "overrides/mods/mod.jar").as_deref(),
                    Some(&b"other bytes"[..])
                );
            }
            Err(error) => {
                assert!(
                    !cfg!(unix),
                    "the change time shows the replacement on Unix: {error}"
                );
            }
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"other bytes");
    }

    /// Files the export does not link or include never fail it: a broken
    /// `*.jar` link, an unreadable jar the user left out, and a file where the
    /// resource packs folder should be.
    #[tokio::test]
    async fn unrelated_unreadable_files_do_not_fail_the_export() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-bad-files").await;
        let mods = path.parent().unwrap().to_owned();
        let root = mods.parent().unwrap().to_owned();
        std::fs::create_dir_all(root.join("resourcepacks")).unwrap();
        std::fs::write(root.join("resourcepacks/pack.zip"), b"pack").unwrap();
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        std::fs::remove_dir_all(root.join("resourcepacks")).unwrap();
        std::fs::write(root.join("resourcepacks"), b"a file where a folder belongs").unwrap();
        let locked = mods.join("locked.jar");
        std::fs::write(&locked, b"never readable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::os::unix::fs::symlink("missing-target.jar", mods.join("broken.jar")).unwrap();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let filter = ExportEntry(
            [
                (
                    "mods".to_owned(),
                    Some(ExportEntry(
                        [
                            ("mod.jar".to_owned(), None),
                            ("broken.jar".to_owned(), None),
                        ]
                        .into_iter()
                        .collect(),
                    )),
                ),
                ("resourcepacks".to_owned(), None),
            ]
            .into_iter()
            .collect(),
        );
        let result = export(&app, id, &path, filter).await;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        let mut archive = result.expect("unrelated unreadable files must not fail the export");
        assert_eq!(manifest_urls(&mut archive), vec![OLD_URL.to_owned()]);
        for name in ["overrides/mods/broken.jar", "overrides/mods/locked.jar"] {
            assert_eq!(override_bytes(&mut archive, name), None, "{name}");
        }
        assert_eq!(
            override_bytes(&mut archive, "overrides/resourcepacks").as_deref(),
            Some(&b"a file where a folder belongs"[..])
        );
        let rows = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        assert!(rows.iter().all(|row| row.filename != "broken.jar"));
        assert!(
            rows.iter().any(|row| row.filename == "pack.zip"),
            "rows of a folder that cannot be listed are kept"
        );
    }

    /// Start a Modrinth export of every mod without waiting for it.
    async fn start_export(
        app: &crate::managers::App,
        id: InstanceId,
        path: &Path,
    ) -> (VisualTaskId, tokio::task::JoinHandle<()>, PathBuf) {
        let output = path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("export.mrpack");
        let (task, worker) = modrinth_archive::export_modrinth(
            app.clone(),
            id,
            output.clone(),
            false,
            all_mods(),
            Some("1".into()),
        )
        .await
        .unwrap();
        (task, worker, output)
    }

    /// The text the export window shows for a failed export: the second cause
    /// if there is one, else the first (exportFailureMessage.ts).
    async fn shown_failure(app: &crate::managers::App, task: VisualTaskId) -> Option<String> {
        let crate::managers::vtask::TaskOutcome::Failed(task) =
            app.task_manager().get_task_outcome(task).await
        else {
            return None;
        };
        let crate::domain::vtask::Progress::Failed(error) = task.progress else {
            return None;
        };
        let error = serde_json::to_value(crate::error::FeError::from_anyhow(&error)).unwrap();
        let causes = error["cause"].as_array().unwrap().clone();
        causes
            .get(1)
            .or(causes.first())
            .map(|cause| cause["display"].as_str().unwrap().to_owned())
    }

    /// The final re-read of every linked mod runs with the instance gate free:
    /// Play, toggles and settings saves never wait for it.
    #[tokio::test]
    async fn the_final_reread_leaves_the_instance_gate_free() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-final-gate").await;
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("export_final_read", id);
        let (task, worker, output) = start_export(&app, id, &path).await;
        tokio::time::timeout(std::time::Duration::from_secs(20), pause.entered.notified())
            .await
            .expect("the export reaches its final re-read");
        let manager = app.instance_manager();
        let gate = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            manager.lock_instance(id),
        )
        .await
        .expect("the instance gate is free while linked mods are re-read");
        drop(gate);
        pause.release.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(20), worker)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(shown_failure(&app, task).await, None);
        let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert_eq!(manifest_urls(&mut archive), vec![OLD_URL.to_owned()]);
    }

    /// A linked mod that changes after its final re-read and before the gate
    /// confirms it is read again: new bytes stop the export with a message
    /// naming the file; a touch alone (same bytes) lets it finish.
    #[tokio::test]
    async fn a_mod_changed_after_its_final_reread_is_read_again() {
        for replace in [false, true] {
            let app = crate::setup_managers_for_test().await;
            let (id, path) = linked_instance(&app, &format!("export-final-check-{replace}")).await;
            let pause = app
                .instance_manager()
                .test_hooks
                .arm("export_final_check", id);
            let (task, worker, output) = start_export(&app, id, &path).await;
            tokio::time::timeout(std::time::Duration::from_secs(20), pause.entered.notified())
                .await
                .expect("the export reaches its gated check");
            let bytes = if replace {
                &b"other bytes"[..]
            } else {
                &b"first bytes"[..]
            };
            // Rewriting moves the change time (Unix); move the modification
            // time too so the change is visible on every platform.
            let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
            std::fs::write(&path, bytes).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(
                    std::fs::FileTimes::new()
                        .set_modified(modified + std::time::Duration::from_secs(5)),
                )
                .unwrap();
            pause.release.notify_one();
            tokio::time::timeout(std::time::Duration::from_secs(20), worker)
                .await
                .unwrap()
                .unwrap();
            if replace {
                assert_eq!(
                    shown_failure(&app, task).await.as_deref(),
                    Some(
                        "mods/mod.jar changed while the instance was being exported. Try the export again."
                    )
                );
                assert!(!output.exists(), "a failed export publishes nothing");
            } else {
                assert_eq!(shown_failure(&app, task).await, None);
                let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
                assert_eq!(manifest_urls(&mut archive), vec![OLD_URL.to_owned()]);
            }
        }
    }

    /// A linked mod that cannot be read fails the export with one sentence
    /// that names it and says how to get past it; the export window shows
    /// that sentence, not the bare OS error.
    #[cfg(unix)]
    #[tokio::test]
    async fn an_unreadable_linked_mod_is_named_in_the_export_window() {
        use std::os::unix::fs::PermissionsExt;
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-unreadable-linked").await;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&path).is_ok() {
            // Permission bits do not stop root.
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            return;
        }
        let (task, worker, output) = start_export(&app, id, &path).await;
        let finished = tokio::time::timeout(std::time::Duration::from_secs(20), worker).await;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        finished.unwrap().unwrap();
        let shown = shown_failure(&app, task).await.expect("the export fails");
        assert!(
            shown.starts_with("Could not read mods/mod.jar ("),
            "names the file: {shown}"
        );
        assert!(
            shown.ends_with(
                "Close any program that is using it, or untick it in the export's file list, then try again."
            ),
            "says how to get past it: {shown}"
        );
        assert!(!output.exists());
    }

    /// A linked mod that is a link into a shared folder that was later moved
    /// or unmounted fails every export the same way. The message says it is
    /// missing and how to get past it, not that it moved during the export,
    /// which a retry would never fix.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_mod_that_is_a_broken_link_is_named_as_missing() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = linked_instance(&app, "export-broken-link").await;
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(path.with_file_name("moved-away.jar"), &path).unwrap();
        let (task, worker, output) = start_export(&app, id, &path).await;
        tokio::time::timeout(std::time::Duration::from_secs(20), worker)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            shown_failure(&app, task).await.as_deref(),
            Some(
                "mods/mod.jar is missing or links to a file that no longer exists. Restore it, or untick it in the export's file list, then try again."
            )
        );
        assert!(!output.exists());
    }
}
