use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::info::{
    self, Instance, JavaOverride, Modpack, ModpackInfo, StandardVersion,
};
use crate::domain::instance::{self as domain, GameLogId, InstanceId};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use crate::domain::metrics::GDLMetricsEvent;
use crate::domain::modplatforms::curseforge::filters::ModFileParameters;
use crate::domain::modplatforms::modrinth::search::VersionID;
use crate::domain::runtime_path::InstancePath;
use crate::domain::vtask::VisualTaskId;
use crate::managers::instance::log::{
    format_message_as_log4j_event, GameLog, LogEntry, LogEntrySourceKind,
};
use crate::managers::instance::modpack::{packinfo, PackVersionFile};
use crate::managers::instance::schema::make_instance_config;
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
use crate::managers::minecraft::assets::get_assets_dir;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::minecraft::{curseforge, UpdateValue};
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::managers::AppInner;
use crate::util::NormalizedWalkdir;
use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
        ManagerRef,
    },
};
use anyhow::{anyhow, bail, Context};
use carbon_net::{DownloadOptions, Downloadable};
use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, Utc};
use futures::Future;
use md5::{Digest, Md5};
use std::collections::HashSet;
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{watch, Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{debug, info, trace};

pub type TSubtasks = Arc<TSubtasksInner>;

pub struct TSubtasksInner {
    pub t_request_version_info: Subtask,
    pub t_download_files: Subtask,
    pub t_scan_files: Subtask,
    pub t_generating_packinfo: Subtask,
    pub t_request_modloader_info: Subtask,
    pub t_request_minecraft_files: Subtask,
    pub t_download_java: Subtask,
    pub t_extract_java: Subtask,
    pub t_apply_staging: Subtask,
    pub t_fill_cache: Subtask,
    pub t_extract_natives: Subtask,
    pub t_reconstruct_assets: Subtask,
    pub t_forge_processors: Option<Subtask>,
    pub t_neoforge_processors: Option<Subtask>,
    pub t_finalize_import: Option<Subtask>,
}

/// This function prepares the modpack in a staging directory in the instance folder.
/// The original instane data is not modified.
///
/// It downloads and processes all required files STRICTLY belonging to the modpack.
///
/// When it's done, it creates the staging-packinfo.json and packinfo.json file and marks the modpack as installed through
/// the modpack-complete disk flag.
pub async fn process_modpack(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    deep_check: bool,
    mut config: Instance,
    instance_shortpath: String,
    task: &VisualTask,
    has_callback_task: bool,
) -> anyhow::Result<(TSubtasks, Option<StandardVersion>)> {
    let mut version: Option<StandardVersion> = None;

    let runtime_path = app.settings_manager().runtime_path.clone();
    let instance_path = runtime_path
        .get_instances()
        .get_instance_path(&instance_shortpath);

    let instance_root = instance_path.get_root();
    let setup_path = instance_root.join(".setup");
    let is_setup = setup_path.is_dir();
    let is_modpack_complete = setup_path.join("modpack-complete").exists();
    let staging_packinfo_path = setup_path.join("staging-packinfo.json");

    let staging_dir = setup_path.join("staging");

    let packinfo_path = instance_root.join("packinfo.json");
    let tmp_packinfo_path = instance_root.join("tmp-packinfo.json");
    let packinfo = match tokio::fs::read_to_string(packinfo_path).await {
        Ok(text) => Some(packinfo::parse_packinfo(&text).context("while parsing packinfo json")?),
        Err(_) => None,
    };

    let t_modpack = match is_setup && !is_modpack_complete {
        true => Some((
            task.subtask(Translation::InstanceTaskLaunchRequestModpack),
            task.subtask(Translation::InstanceTaskLaunchDownloadModpack),
            task.subtask(Translation::InstanceTaskLaunchDownloadModpackFiles),
            task.subtask(Translation::InstanceTaskLaunchExtractModpackFiles),
            task.subtask(Translation::InstanceTaskLaunchRequestAddonMetadata),
        )),
        false => None,
    };

    let t_apply_staging = task.subtask(Translation::InstanceTaskLaunchApplyStagedPatches);

    let t_request_version_info = task.subtask(Translation::InstanceTaskLaunchRequestVersions);

    let t_download_files = task.subtask(Translation::InstanceTaskLaunchDownloadFiles);
    t_download_files.set_weight(20.0);

    let t_scan_files = task.subtask(Translation::InstanceTaskLaunchCheckingFiles);
    t_scan_files.set_weight(5.0);

    let t_generating_packinfo = task.subtask(Translation::InstanceTaskGeneratingPackInfo);

    let t_request_modloader_info = task.subtask(Translation::InstanceTaskRequestModloaderInfo);

    let t_request_minecraft_files = task.subtask(Translation::InstanceTaskRequestMinecraftFiles);

    let t_download_java = task.subtask(Translation::InstanceTaskLaunchDownloadJava);

    let t_extract_java = task.subtask(Translation::InstanceTaskLaunchExtractJava);

    let t_fill_cache = task.subtask(Translation::InstanceTaskFillCache);

    let t_extract_natives = task.subtask(Translation::InstanceTaskLaunchExtractNatives);

    let t_reconstruct_assets = task.subtask(Translation::InstanceTaskReconstructAssets);

    let t_forge_processors = match is_setup {
        true => Some(task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors)),
        false => None,
    };

    let t_neoforge_processors = match is_setup {
        true => Some(task.subtask(Translation::InstanceTaskLaunchRunNeoforgeProcessors)),
        false => None,
    };

    let t_finalize_import = if has_callback_task {
        Some(task.subtask(Translation::FinalizingImport))
    } else {
        None
    };

    task.edit(|data| data.state = TaskState::KnownProgress)
        .await;

    let change_version_path = setup_path.join("change-pack-version.json");

    if let Some((
        t_request,
        t_download_packfile,
        t_download_modpack_files,
        t_extract_files,
        t_addon_metadata,
    )) = t_modpack
    {
        let mut modpack_downloads = Vec::new();

        let cffile_path = setup_path.join("curseforge");
        let mrfile_path = setup_path.join("modrinth");

        // Is this required? Can we not extract them twice? Extraction should be idempotent.
        // TODO: look into this
        let skip_overrides_path = setup_path.join("modpack-skip-overrides");
        let skip_overrides = skip_overrides_path.is_dir();

        let modpack = match tokio::fs::read_to_string(&change_version_path).await {
            Ok(text) => Some(Modpack::from(serde_json::from_str::<PackVersionFile>(
                &text,
            )?)),
            Err(_) => None,
        };

        enum Modplatform {
            Curseforge,
            Modrinth,
        }

        t_request.start_opaque();

        // If a cf or mr file is provided, we don't need to do anything.
        // In case a modpack (from a change-pack-version.json file) is provided,
        // we need to download the modpack zip file.
        let file = match (cffile_path.is_file(), mrfile_path.is_file(), &modpack) {
            (false, false, None) => {
                t_request.complete_opaque();
                None
            }
            (true, _, _) => {
                t_request.complete_opaque();
                Some(Modplatform::Curseforge)
            }
            (_, true, _) => {
                t_request.complete_opaque();
                Some(Modplatform::Modrinth)
            }
            (false, false, Some(Modpack::Curseforge(modpack))) => {
                let file = app
                    .modplatforms_manager()
                    .curseforge
                    .get_mod_file(ModFileParameters {
                        file_id: modpack.file_id as i32,
                        mod_id: modpack.project_id as i32,
                    })
                    .await?
                    .data;

                t_request.complete_opaque();

                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(UpdateValue::<(u64, u64)>::new((0, 0)));

                t_download_packfile.start_opaque();
                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let (downloaded, total) = modpack_progress_rx.borrow().0;
                            t_download_packfile.update_download(
                                downloaded as u32,
                                total as u32,
                                true,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_packfile.complete_opaque();
                });

                curseforge::download_modpack_zip(&app, &file, &cffile_path, modpack_progress_tx)
                    .await?;

                completion.await?;

                Some(Modplatform::Curseforge)
            }
            (false, false, Some(Modpack::Modrinth(modpack))) => {
                let file = app
                    .modplatforms_manager()
                    .modrinth
                    .get_version(VersionID(modpack.version_id.clone()))
                    .await?
                    .files
                    .into_iter()
                    .reduce(|a, b| if b.primary { b } else { a })
                    .ok_or_else(|| {
                        anyhow!(
                            "Modrinth project '{}' version '{}' does not have a file",
                            modpack.project_id,
                            modpack.version_id
                        )
                    })?;

                t_request.complete_opaque();

                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(UpdateValue::<(u64, u64)>::new((0, 0)));

                t_download_packfile.start_opaque();

                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let (downloaded, total) = modpack_progress_rx.borrow().0;
                            t_download_packfile.update_download(
                                downloaded as u32,
                                total as u32,
                                true,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_packfile.complete_opaque();
                });

                modrinth::download_mrpack(&app, &file, &mrfile_path, modpack_progress_tx).await?;

                completion.await?;

                Some(Modplatform::Modrinth)
            }
        };

        // Temporarily create a staging directory and download the modpack files there
        tokio::fs::create_dir_all(&staging_dir.join("instance")).await?;
        let instance_prep_path = InstancePath::new(staging_dir.clone());

        let mut skipped_mods = Vec::new();

        // Prepaers the list of modpack downloadable files and the manifest, as
        // well as extract the overrides in it
        let v: Option<StandardVersion> = match file {
            Some(Modplatform::Curseforge) => {
                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(curseforge::ProgressState::new());

                t_addon_metadata.start_opaque();

                let completion = tokio::spawn(async move {
                    let mut tracker = curseforge::ProgressState::new();

                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let progress = modpack_progress_rx.borrow();

                            tracker.extract_addon_overrides.update_from(
                                &progress.extract_addon_overrides,
                                |(completed, total)| {
                                    t_extract_files.update_items(completed as u32, total as u32);
                                },
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_extract_files.complete_opaque();
                });

                let modpack_info = curseforge::prepare_modpack_from_zip(
                    &app,
                    &cffile_path,
                    &instance_prep_path,
                    skip_overrides,
                    packinfo.as_ref(),
                    t_addon_metadata,
                    modpack_progress_tx,
                )
                .await
                .map_err(|e| {
                    tracing::error!("Error preparing modpack: {:?}", e);
                    e
                })?;

                completion.await?;

                tokio::fs::create_dir_all(skip_overrides_path).await?;

                for (downloadable, skip) in modpack_info.downloadables {
                    match skip {
                        Some(skippath) => skipped_mods.push(skippath),
                        None => modpack_downloads.push(downloadable),
                    }
                }

                let curseforge_version = modpack_info.manifest.minecraft;

                let dummy_string = daedalus::BRANDING
                    .get_or_init(daedalus::Branding::default)
                    .dummy_replace_string
                    .clone();

                let gdl_version = convert_cf_version_to_standard_version(
                    app.clone(),
                    curseforge_version,
                    dummy_string,
                )
                .await?;

                Some(gdl_version)
            }
            Some(Modplatform::Modrinth) => {
                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(modrinth::ProgressState::Idle);

                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let progress = modpack_progress_rx.borrow();
                            match *progress {
                                modrinth::ProgressState::Idle => {}
                                modrinth::ProgressState::ExtractingPackOverrides(count, total) => {
                                    t_extract_files.update_items(count as u32, total as u32)
                                }
                                modrinth::ProgressState::AcquiringPackMetadata(count, total) => {
                                    t_addon_metadata.update_items(count as u32, total as u32)
                                }
                            }
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_addon_metadata.complete_opaque();
                    t_extract_files.complete_opaque();
                });

                let modpack_info = modrinth::prepare_modpack_from_mrpack(
                    &app,
                    &mrfile_path,
                    &instance_prep_path,
                    skip_overrides,
                    packinfo.as_ref(),
                    modpack_progress_tx,
                )
                .await?;

                completion.await?;

                tokio::fs::create_dir_all(skip_overrides_path).await?;

                for (downloadable, skip) in modpack_info.downloadables {
                    match skip {
                        Some(skippath) => skipped_mods.push(skippath),
                        None => modpack_downloads.push(downloadable),
                    }
                }

                let modrinth_version = modpack_info.index.dependencies;

                let gdl_version =
                    convert_mr_version_to_standard_version(app.clone(), modrinth_version).await?;

                Some(gdl_version)
            }
            None => None,
        };

        let (progress_watch_tx, mut progress_watch_rx) =
            tokio::sync::watch::channel(carbon_net::Progress::new());

        t_download_modpack_files.start_opaque();

        // dropped when the sender is dropped
        let completion = tokio::spawn(async move {
            while progress_watch_rx.changed().await.is_ok() {
                {
                    let progress = progress_watch_rx.borrow();
                    t_download_modpack_files.update_download(
                        progress.current_size as u32,
                        progress.total_size as u32,
                        false,
                    );
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            t_download_modpack_files.complete_opaque();
        });

        let concurrency = app
            .settings_manager()
            .get_settings()
            .await?
            .concurrent_downloads;

        // Actually downloads the modpack files
        carbon_net::download_multiple(
            &modpack_downloads[..],
            DownloadOptions::builder()
                .concurrency(concurrency as usize)
                .progress_sender(progress_watch_tx)
                .deep_check(deep_check)
                .build(),
        )
        .await
        .with_context(|| {
            format!("Failed to download modpack instance files for instance {instance_id}")
        })?;

        completion.await?;

        if let Some(v) = v {
            tracing::info!("Modpack version: {v:?}");

            version = Some(v.clone());
            let path = app
                .settings_manager()
                .runtime_path
                .get_instances()
                .to_path()
                .join(instance_shortpath);

            config.modpack = modpack.map(|modpack| ModpackInfo {
                modpack,
                locked: config.modpack.map(|m| m.locked).unwrap_or(true),
            });

            if config.modpack.is_some() {
                app.instance_manager().get_modpack_info(instance_id).await?;
            }

            config.game_configuration.version = Some(GameVersion::Standard(StandardVersion {
                release: v.release.clone(),
                modloaders: v.modloaders.clone(),
            }));

            let json = make_instance_config(config.clone())?;
            tokio::fs::write(path.join("instance.json"), json).await?;

            app.instance_manager()
                .instances
                .write()
                .await
                .get_mut(&instance_id)
                .ok_or_else(|| anyhow!("Instance was deleted while loading"))?
                .data_mut()?
                .config = config;

            app.invalidate(GET_MODPACK_INFO, Some(instance_id.0.into()));
        }

        // normally there would be a problem here because we would be skipping any mods removed by users
        // but since we dont try to update those anyway its fine.
        let mut files = skipped_mods;
        // snapshot filetree before applying
        let mut walker = NormalizedWalkdir::new(&staging_dir.join("instance"))?;
        while let Some(entry) = walker.next()? {
            if entry.is_dir {
                continue;
            }
            files.push(entry.relative_path.to_string());
        }

        let snapshot = serde_json::to_string_pretty(&files)?;
        tokio::fs::write(staging_packinfo_path, snapshot).await?;

        t_generating_packinfo.start_opaque();

        let files_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        // At this point the modpack files are all in the staging directory, so that's the path we need to scan.
        // The packinfo on the other hand is in the instance folder itself.
        let packinfo =
            packinfo::scan_dir(&instance_prep_path.get_data_path(), Some(&files_refs)).await?;

        let packinfo_str = packinfo::make_packinfo(packinfo)?;
        tokio::fs::write(tmp_packinfo_path, packinfo_str).await?;

        t_generating_packinfo.complete_opaque();

        trace!("marking modpack initialization as complete");

        tracing::info!("queueing metadata caching for running instance");
        t_fill_cache.start_opaque();

        app.meta_cache_manager()
            .queue_caching(instance_id, true)
            .await;

        t_fill_cache.complete_opaque();

        trace!("queued metadata caching");
    }

    let subtasks = TSubtasksInner {
        t_request_version_info,
        t_download_files,
        t_scan_files,
        t_generating_packinfo,
        t_request_modloader_info,
        t_request_minecraft_files,
        t_download_java,
        t_apply_staging,
        t_extract_java,
        t_fill_cache,
        t_extract_natives,
        t_reconstruct_assets,
        t_forge_processors,
        t_neoforge_processors,
        t_finalize_import,
    };

    Ok((Arc::new(subtasks), version))
}

// TODO: Modpack staging is not atomic and does not track applied changes, so if the process is interrupted,
// the instance will be in an inconsistent state.
pub async fn process_modpack_staging(
    app: Arc<AppInner>,
    instance_shortpath: String,
    t_subtasks: &TSubtasks,
) -> anyhow::Result<()> {
    let runtime_path = app.settings_manager().runtime_path.clone();
    let instance_path = runtime_path
        .get_instances()
        .get_instance_path(&instance_shortpath);

    let instance_root = instance_path.get_root();
    let setup_path = instance_root.join(".setup");
    let is_first_run = setup_path.is_dir();

    let staging_dir = setup_path.join("staging");

    if staging_dir.exists() {
        t_subtasks.t_apply_staging.start_opaque();

        let change_version_path = setup_path.join("change-pack-version.json");
        let overwrite_changed = !change_version_path.exists(); // TODO

        let staging_packinfo = setup_path.join("staging-packinfo.json");

        let staged_text = tokio::fs::read_to_string(&staging_packinfo).await?;
        let staging_snapshot = serde_json::from_str::<Vec<&str>>(&staged_text)
            .context("could not parse staging snapshot")?;

        #[derive(Debug)]
        enum SkipReplaceReason {
            DeletedByUser,
            ModifiedByUser([u8; 16], [u8; 16]),
            InSaveFolder,
        }

        let mut new_files = Vec::<String>::new();
        let mut deleted_files = Vec::<String>::new();
        let mut replaced_files = Vec::<String>::new();
        let mut skipped_replacements = Vec::<(String, SkipReplaceReason)>::new();

        let packinfo_path = instance_root.join("packinfo.json");
        let packinfo = match tokio::fs::read_to_string(packinfo_path).await {
            Ok(text) => {
                Some(packinfo::parse_packinfo(&text).context("while parsing packinfo json")?)
            }
            Err(_) => None,
        };

        debug!("Applying staged instance files");
        let r: anyhow::Result<_> = async {
            if let Some(packinfo) = packinfo {
                for (oldfile, oldfilehash) in &packinfo.files {
                    let mut original_file = instance_root.join("instance").join(&oldfile[1..]);

                    trace!("Checking for replacement for packinfo file: {original_file:?}");

                    if !original_file.exists() {
                        let mut name = original_file.file_name().unwrap().to_owned();
                        name.push(".disabled");
                        original_file.set_file_name(name);

                        if !original_file.exists() {
                            // either the user deleted it or we already deleted it in the next check, skip
                            skipped_replacements
                                .push((oldfile.clone(), SkipReplaceReason::DeletedByUser));
                            continue;
                        }
                    }

                    let mut original_md5 = Md5::new();
                    let mut file = tokio::fs::File::open(&original_file).await?;
                    carbon_scheduler::buffered_digest(&mut file, |chunk| {
                        original_md5.update(chunk);
                    })
                    .await?;
                    drop(file);
                    let original_md5: [u8; 16] = original_md5.finalize().into();

                    if original_md5 != oldfilehash.md5 {
                        // the user has modified this file so we shouldn't touch it
                        skipped_replacements.push((
                            oldfile.clone(),
                            SkipReplaceReason::ModifiedByUser(oldfilehash.md5, original_md5),
                        ));
                        continue;
                    }

                    if !staging_snapshot.contains(&(&oldfile as &str)) {
                        if oldfile.starts_with("/saves") {
                            skipped_replacements
                                .push((oldfile.clone(), SkipReplaceReason::InSaveFolder));
                            continue;
                        }

                        // file is not present in new version and old version was not changed, delete
                        tokio::fs::remove_file(original_file).await?;
                        deleted_files.push(oldfile.clone());
                        continue;
                    }

                    let staged_file = staging_dir.join("instance").join(&oldfile[1..]);

                    if staged_file.is_file() {
                        // old file matches the snapshotted version and new file is present, replace
                        tokio::fs::rename(staged_file, original_file).await?;
                        replaced_files.push(oldfile.clone());
                    }
                }
            }

            for entry in walkdir::WalkDir::new(&staging_dir) {
                let entry = entry?;

                let staged_file = entry.path().to_path_buf();
                let relpath = staged_file.strip_prefix(&staging_dir).unwrap();
                let original_file = instance_root.join(relpath);

                if entry.metadata()?.is_file() && !original_file.exists() {
                    // there was no record of this file in the packinfo or it would've been moved previously,
                    // and the user has not created one in its place, add the file

                    new_files.push(relpath.to_string_lossy().to_string());
                    tokio::fs::create_dir_all(original_file.parent().unwrap()).await?;
                    tokio::fs::rename(staged_file, original_file).await?;
                }
            }

            Ok(())
        }
        .await;

        if let Err(e) = r {
            return Err(e.context("Failed to apply staged instance changes"));
        }

        trace!("Creating update audit files");
        let audit_dir = instance_root.join(".install_audit");

        // delete old audit dir if it exists
        if (audit_dir.exists()) {
            tokio::fs::remove_dir_all(&audit_dir).await?;
        }

        tokio::fs::create_dir(&audit_dir).await?;

        let audit_file = audit_dir.join("audit.txt");
        let mut audit_txt = "GDLauncher Modpack Install/Update Audit\n".to_string();

        if (!skipped_replacements.is_empty()) {
            audit_txt += "\nFiles that could not be replaced:\n";

            for (file, reason) in skipped_replacements {
                match reason {
                    SkipReplaceReason::DeletedByUser => audit_txt += &format!(" - {file}: deleted by user\n"),
                    SkipReplaceReason::ModifiedByUser(original, current) => audit_txt += &format!(
                        " - {file}: modified by user\n     original md5: {}\n     current md5:  {}\n",
                        hex::encode(original),
                        hex::encode(current),
                    ),
                    SkipReplaceReason::InSaveFolder => audit_txt += &format!(" - {file}: files in /saves will never be modified\n"),
                }
            }
        }

        if (!deleted_files.is_empty()) {
            audit_txt += "\nFiles deleted:\n";

            for file in deleted_files {
                audit_txt += &format!(" - {file}\n");
            }
        }

        if (!replaced_files.is_empty()) {
            audit_txt += "\nFiles replaced:\n";

            for file in replaced_files {
                audit_txt += &format!(" - {file}\n");
            }
        }

        if (!new_files.is_empty()) {
            audit_txt += "\nFiles created:\n";

            for file in new_files {
                audit_txt += &format!(" - {file}\n");
            }
        }

        tokio::fs::write(audit_file, audit_txt).await?;

        trace!("Cleaning up staging directory");
        tokio::fs::remove_dir_all(staging_dir).await?;
        trace!("Staging complete");
        t_subtasks.t_apply_staging.complete_opaque();

        if instance_root.join("tmp-packinfo.json").exists() {
            tokio::fs::rename(
                instance_root.join("tmp-packinfo.json"),
                instance_root.join("packinfo.json"),
            )
            .await?;
        }

        tokio::fs::write(setup_path.join("modpack-complete"), "").await?;
    }

    Ok(())
}
