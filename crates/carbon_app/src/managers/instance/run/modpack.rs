use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::info::{
    self, Instance, JavaOverride, Modpack, ModpackInfo, StandardVersion,
};
use crate::domain::instance::{self as domain, GameLogId, InstanceId};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use crate::domain::metrics::GDLMetricsEvent;
use crate::domain::vtask::VisualTaskId;
use crate::managers::AppInner;
use crate::managers::instance::log::{
    GameLog, LogEntry, LogEntrySourceKind, format_message_as_log4j_event,
};
use crate::managers::instance::modpack::{
    PackVersionFile, RepairMarkerFile, apply_plan, disk_scan, normalize_cleanup_path, packinfo,
    walk_untracked_files,
};
use crate::managers::instance::schema::make_instance_config;
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
use crate::managers::minecraft::assets::get_assets_dir;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::minecraft::{UpdateValue, curseforge, gdlpack};
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::util::NormalizedWalkdir;
use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self, ManagerRef,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
    },
};
use anyhow::{Context, anyhow, bail};
use carbon_net::{DownloadOptions, Downloadable};
use carbon_parsing::log::{LogParser, ParsedItem};
use carbon_platforms::curseforge::filters::ModFileParameters;
use carbon_platforms::modrinth::search::VersionID;
use carbon_rt_path::InstancePath;
use chrono::{DateTime, Local, Utc};
use futures::Future;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{info, trace};

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
) -> anyhow::Result<(TSubtasks, Option<StandardVersion>, Option<RepairMarkerFile>)> {
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

    // Absent marker (including a `.setup` left behind by an older build that
    // never wrote one) means an ordinary version change — unchanged from
    // before this file read the marker at all.
    let repair_marker_path = setup_path.join("repair");
    let repair_options: Option<RepairMarkerFile> =
        match tokio::fs::read_to_string(&repair_marker_path).await {
            Ok(s) => Some(serde_json::from_str(&s).context("while parsing repair marker json")?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };

    // Named for what it's used for below (skip-optimisation oracle for the
    // platform prep fns), not merely what it holds. A repair judges "needs
    // download" against what is actually on disk right now, not against the
    // record — a corrupt or missing file must be re-fetched even when the
    // record says it was fine.
    let skip_oracle = match &repair_options {
        Some(_) => {
            Some(disk_scan::scan_instance_as_packinfo(&instance_path.get_data_path()).await?)
        }
        None => match tokio::fs::read_to_string(&packinfo_path).await {
            Ok(text) => {
                Some(packinfo::parse_packinfo(&text).context("while parsing packinfo json")?)
            }
            Err(_) => None,
        },
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

    // Fresh modpack installs may not have the version in config until
    // modpack processing resolves it, so is_setup must keep creating the
    // subtasks unconditionally; loader-keyed creation covers every launch
    // after that so regeneration has a progress subtask to report through.
    let has_loader = |wanted: ModLoaderType| {
        matches!(
            &config.game_configuration.version,
            Some(GameVersion::Standard(v))
                if v.modloaders.iter().any(|m| m.type_ == wanted)
        )
    };

    let t_forge_processors = match is_setup || has_loader(ModLoaderType::Forge) {
        true => Some(task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors)),
        false => None,
    };

    let t_neoforge_processors = match is_setup || has_loader(ModLoaderType::Neoforge) {
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
        let gdlpack_path = setup_path.join("gdlpack");

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
            GDLPack,
        }

        t_request.start_opaque();

        // If a cf, mr, or gdlpack file is provided, we don't need to do anything.
        // In case a modpack (from a change-pack-version.json file) is provided,
        // we need to download the modpack zip file.
        let file = match (
            cffile_path.is_file(),
            mrfile_path.is_file(),
            gdlpack_path.is_file(),
            &modpack,
        ) {
            (false, false, false, None) => {
                t_request.complete_opaque();
                None
            }
            (true, _, _, _) => {
                t_request.complete_opaque();
                Some(Modplatform::Curseforge)
            }
            (_, true, _, _) => {
                t_request.complete_opaque();
                Some(Modplatform::Modrinth)
            }
            (_, _, true, _) => {
                t_request.complete_opaque();
                Some(Modplatform::GDLPack)
            }
            (false, false, false, Some(Modpack::Curseforge(modpack))) => {
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
            (false, false, false, Some(Modpack::Modrinth(modpack))) => {
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
                    skip_oracle.as_ref(),
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
                    skip_oracle.as_ref(),
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
            Some(Modplatform::GDLPack) => {
                // gdlpack's own skip predicate (`gdlpack.rs`'s
                // `existing_packinfo`/`skip_path` lookup) only checks
                // whether a path is PRESENT in the oracle, never whether its
                // hash matches — unlike the curseforge/modrinth prep fns.
                // Under an ordinary version change that is merely a missed
                // optimisation (the record is trusted anyway); under repair
                // the oracle is a live disk scan, so a merely-*existing*
                // damaged file would be skip-optimised as-is and its
                // corrupt hash promoted into packinfo as canonical —
                // laundering the corruption instead of fixing it. No
                // `Modpack` variant can select GDLPack today (making this
                // unreachable in practice), but refuse outright rather than
                // leave a live trap for whenever one can.
                if repair_options.is_some() {
                    bail!("repair is not supported for GDLPack-installed instances");
                }

                let (modpack_progress_tx, mut modpack_progress_rx) =
                    tokio::sync::watch::channel(gdlpack::ProgressState::Idle);

                let completion = tokio::spawn(async move {
                    while modpack_progress_rx.changed().await.is_ok() {
                        {
                            let progress = *modpack_progress_rx.borrow();
                            match progress {
                                gdlpack::ProgressState::Idle => {}
                                gdlpack::ProgressState::ResolvingFiles(count, total) => {
                                    t_addon_metadata.update_items(count as u32, total as u32)
                                }
                                gdlpack::ProgressState::ExtractingOverrides(count, total) => {
                                    t_extract_files.update_items(count as u32, total as u32)
                                }
                            }
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_addon_metadata.complete_opaque();
                    t_extract_files.complete_opaque();
                });

                let modpack_info = gdlpack::prepare_modpack_from_gdlpack(
                    &app,
                    &gdlpack_path,
                    &instance_prep_path,
                    skip_overrides,
                    skip_oracle.as_ref(),
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

                Some(modpack_info.version)
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
            // Atomically replace the live config (matching update_instance/update_playtime) so a
            // crash mid-write cannot leave a partial instance.json that fails to parse on startup.
            app.settings_manager()
                .runtime_path
                .get_temp()
                .write_file_atomic(path.join("instance.json"), json)
                .await?;

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

        // Only generate staging-packinfo if we actually processed a modpack
        // (i.e., file was Some). Otherwise this is just a version/modloader change
        // and we should not touch the existing mods.
        if file.is_some() {
            // normally there would be a problem here because we would be skipping any mods removed by users
            // but since we dont try to update those anyway its fine.
            //
            // Cloned rather than moved: `skipped_mods` is walked again below
            // to merge its hashes into the freshly scanned packinfo, since
            // `scan_dir` never staged these paths and so cannot see them.
            let mut files = skipped_mods.clone();
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
            let mut packinfo =
                packinfo::scan_dir(&instance_prep_path.get_data_path(), Some(&files_refs)).await?;

            // Skip-optimised files were never staged, so the scan cannot see
            // them. "Skipped" means the oracle's hash matched the manifest's,
            // so the oracle entry IS the target hash — merge it, or every
            // unchanged file falls out of the record (the stale-survivor bug).
            for skipped in &skipped_mods {
                if packinfo.files.contains_key(skipped) {
                    continue;
                }
                let Some(hashes) = skip_oracle.as_ref().and_then(|o| o.files.get(skipped)) else {
                    bail!(
                        "skip-optimised path {skipped} has no oracle entry — refusing to write an incomplete packinfo"
                    );
                };
                packinfo.files.insert(skipped.clone(), hashes.clone());
            }

            let packinfo_str = packinfo::make_packinfo(packinfo)?;
            tokio::fs::write(tmp_packinfo_path, packinfo_str).await?;

            t_generating_packinfo.complete_opaque();
        } else {
            t_generating_packinfo.complete_opaque();
        }

        trace!("marking modpack initialization as complete");

        tracing::info!("queueing metadata caching for running instance");
        t_fill_cache.start_opaque();

        app.meta_cache_manager()
            .queue_caching(
                crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                true,
            )
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

    Ok((Arc::new(subtasks), version, repair_options))
}

// TODO: Modpack staging is not atomic and does not track applied changes, so if the process is interrupted,
// the instance will be in an inconsistent state.
pub async fn process_modpack_staging(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    instance_shortpath: String,
    t_subtasks: &TSubtasks,
    repair_options: Option<RepairMarkerFile>,
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
        let staging_packinfo = setup_path.join("staging-packinfo.json");

        // Check if staging-packinfo.json exists - if not, this was a version/modloader-only change
        // and we should skip staging entirely to avoid deleting existing mods
        if !staging_packinfo.exists() {
            trace!(
                "No staging-packinfo.json found, skipping staging (version/modloader-only change)"
            );
            tokio::fs::remove_dir_all(&staging_dir).await?;
            tokio::fs::write(setup_path.join("modpack-complete"), "").await?;
            t_subtasks.t_apply_staging.complete_opaque();
            return Ok(());
        }

        t_subtasks.t_apply_staging.start_opaque();

        let old_packinfo =
            match tokio::fs::read_to_string(instance_root.join("packinfo.json")).await {
                Ok(s) => Some(packinfo::parse_packinfo(&s)?),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
        let target_packinfo = packinfo::parse_packinfo(
            &tokio::fs::read_to_string(instance_root.join("tmp-packinfo.json")).await?,
        )?;

        // Staged set: files physically present under .setup/staging/instance,
        // as packinfo-style keys.
        let mut staged = HashSet::new();
        let mut walker = NormalizedWalkdir::new(&staging_dir.join("instance"))?;
        while let Some(entry) = walker.next()? {
            if entry.is_dir {
                continue;
            }
            // Mirrors packinfo::scan_dir's own `.disabled` stripping
            // (packinfo/scan.rs:30-32): a pack that ships an override
            // disabled by default stages it under the `.disabled` name, but
            // tmp-packinfo.json (built by scan_dir) keys it under the
            // enabled name. `staged` has to match that key shape, or a path
            // whose only staged copy is `.disabled`-suffixed looks unstaged
            // to the planner and a fresh install of that path errors
            // permanently (MissingStagedSource).
            let mut key = entry.relative_path.to_string();
            if key.ends_with(".disabled") {
                key.truncate(key.len() - ".disabled".len());
            }
            staged.insert(key);
        }

        let universe: BTreeSet<String> = old_packinfo
            .iter()
            .flat_map(|p| p.files.keys().cloned())
            .chain(target_packinfo.files.keys().cloned())
            .collect();
        let disk = disk_scan::scan_disk_state(&instance_root.join("instance"), &universe).await?;

        // Absent marker -> ordinary version-change reconciliation, unchanged
        // from before repair mode existed. Present -> re-reconcile every
        // pack-tracked path against `target` alone (see
        // `apply_plan::decide_repair`), regardless of what `old` says.
        let mode = match &repair_options {
            Some(options) => apply_plan::ApplyMode::Repair {
                re_enable_disabled: options.re_enable_disabled,
            },
            None => apply_plan::ApplyMode::VersionChange,
        };

        let entries = apply_plan::plan(apply_plan::PlanInputs {
            old: old_packinfo.as_ref(),
            target: &target_packinfo,
            staged: &staged,
            disk: &disk,
            mode,
        })?;

        execute_plan(&entries, &instance_root, &staging_dir).await?;

        // Repair-only: paths the user explicitly ticked for removal in the
        // preview, applied after the plan so a cleanup can never race the
        // pack's own reconciliation of the same path.
        let user_removed = match &repair_options {
            Some(options) => {
                apply_user_cleanup(
                    &options.cleanup_paths,
                    old_packinfo.as_ref(),
                    &target_packinfo,
                    &instance_root,
                )
                .await
            }
            None => Vec::new(),
        };

        trace!("Creating update audit files");
        let audit_dir = instance_root.join(".install_audit");

        // delete old audit dir if it exists
        if (audit_dir.exists()) {
            tokio::fs::remove_dir_all(&audit_dir).await?;
        }

        tokio::fs::create_dir(&audit_dir).await?;

        let audit_file = audit_dir.join("audit.txt");
        let audit_txt = render_audit(&entries, &user_removed);
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

        // Trigger caching now that modpack installation is complete
        app.meta_cache_manager()
            .watch_and_prioritize(Some(
                crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
            ))
            .await;
    }

    Ok(())
}

/// Carries out every [`apply_plan::PlanEntry`] against the real filesystem.
/// Pure mechanics — every decision (including whether a path even needs
/// touching) was already made by [`apply_plan::plan`]; this only performs
/// the rename/remove that `entry.action` names.
async fn execute_plan(
    entries: &[apply_plan::PlanEntry],
    instance_root: &Path,
    staging_dir: &Path,
) -> anyhow::Result<()> {
    use apply_plan::PlanAction;
    for entry in entries {
        let rel = &entry.path[1..];
        let live = instance_root.join("instance").join(rel);
        let staged = staging_dir.join("instance").join(rel);
        let twin = disabled_sibling(&live);
        match entry.action {
            PlanAction::Keep => {}
            PlanAction::Delete => {
                tokio::fs::remove_file(&live).await?;
            }
            PlanAction::Replace => {
                let (source, is_disabled) =
                    resolve_staged(&staged).ok_or_else(|| missing_staged_error(&entry.path))?;
                if is_disabled {
                    // The target now ships this path disabled by default:
                    // land it under the twin spelling, then drop the
                    // previously-enabled live copy so only one spelling of
                    // the file survives on disk.
                    tokio::fs::rename(&source, &twin).await?;
                    tokio::fs::remove_file(&live).await?;
                } else {
                    tokio::fs::rename(&source, &live).await?;
                }
            }
            PlanAction::Create => {
                let (source, is_disabled) =
                    resolve_staged(&staged).ok_or_else(|| missing_staged_error(&entry.path))?;
                // A pack-shipped-disabled path must land disabled, not
                // enabled — the pack's own default is preserved.
                let dest = if is_disabled { &twin } else { &live };
                tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
                tokio::fs::rename(&source, dest).await?;
            }
            PlanAction::ReplaceDisabled => {
                let (source, _) =
                    resolve_staged(&staged).ok_or_else(|| missing_staged_error(&entry.path))?;
                tokio::fs::rename(&source, &twin).await?;
            }
            PlanAction::ReEnable => {
                if let Some((source, _)) = resolve_staged(&staged) {
                    tokio::fs::create_dir_all(live.parent().unwrap()).await?;
                    tokio::fs::rename(&source, &live).await?;
                    let _ = tokio::fs::remove_file(&twin).await;
                } else {
                    tokio::fs::rename(&twin, &live).await?;
                }
            }
        }
    }
    Ok(())
}

/// Deletes every path in a repair's `cleanup_paths` from the live instance
/// data dir, on the user's own explicit request from the repair preview.
/// Runs after [`execute_plan`] so a cleanup can never race the pack's own
/// reconciliation of the same path.
///
/// **Never bails.** `repair_modpack` already rejected a syntactically
/// invalid path before this pipeline ever started (see
/// [`crate::managers::instance::modpack::normalize_cleanup_path`]'s doc), so
/// by the time execution reaches here every remaining failure mode — a
/// tracked path, a symlink-widened escape, a plain I/O error — is either an
/// adversarial input or a benign race, never a normal user mistake worth
/// aborting the whole apply for. A single bad entry after [`execute_plan`]
/// has already run would otherwise skip the audit write and the
/// `packinfo.json` promotion entirely and leave `.setup/repair` in place, so
/// every future relaunch re-enters repair and re-fails at the identical
/// path forever. Every rejection is `tracing::warn!`-logged and the path is
/// skipped instead: it simply stays on disk, absent from the returned list
/// (and so absent from the audit's "removed at user request" section too).
///
/// **Walk-membership design.** A byte-exact string comparison against
/// `/saves`, a `.disabled` suffix, or a packinfo key is not enough on its
/// own: Windows and default-configuration macOS resolve paths
/// case-insensitively (`/Saves/...` reaches the same file as `/saves/...`,
/// `/Mods/tracked.jar` the same as `/mods/tracked.jar`), and Windows also
/// silently drops a trailing dot/space — any of these would pass every
/// string check here yet still delete the real `/saves` or pack-tracked
/// file once the OS's own path resolution runs inside `remove_file`. Rather
/// than chase every OS-specific aliasing rule with case-folds, this closes
/// the whole class structurally: [`walk_untracked_files`] walks the REAL
/// instance data dir once and returns the ground-truth set of untracked
/// files, each keyed by the raw spelling **the walk itself observed** (never
/// a spelling derived from user input) and mapped to that file's own real
/// [`PathBuf`]. A `cleanup_paths` entry (already syntax-validated by
/// [`normalize_cleanup_path`]) is honored only if it is an *exact* member of
/// that set — **the `remove_file` target is always the walked entry's own
/// `PathBuf`, never rebuilt from the user's string.** That is what makes
/// alias divergence structurally impossible: whatever spelling the user
/// typed, it only ever earns the deletion of a directory entry the walk
/// itself enumerated and classified untracked — there is no code path left
/// where a user-supplied string is turned into a deletion target on its
/// own, so no OS path-resolution quirk can make the two diverge.
///
/// The canonicalize-parent containment check below is kept as defense in
/// depth against a *walked* entry reached through a symlink somewhere in
/// the instance tree (`NormalizedWalkdir` follows a symlinked subdirectory
/// during traversal, same as `fs::metadata`) — `remove_file` itself never
/// follows a symlink in the final path component, so only the parent chain
/// needs checking. A path already absent from disk, or whose parent
/// directory no longer exists at all (a benign race between the walk and
/// this loop), is not a failure: the user's intent — this path gone — is
/// already satisfied, though still `tracing::warn!`-logged rather than
/// silently passed over, since by this point the walk itself just proved
/// the entry existed a moment ago. Returns exactly the paths actually
/// removed, for [`render_audit`]'s `user_removed`.
async fn apply_user_cleanup(
    cleanup_paths: &[String],
    old_packinfo: Option<&packinfo::PackInfo>,
    target_packinfo: &packinfo::PackInfo,
    instance_root: &Path,
) -> Vec<String> {
    if cleanup_paths.is_empty() {
        // Avoid walking the whole instance tree for nothing — the common
        // case today, since the repair preview UI that produces a non-empty
        // list doesn't exist yet (`RepairModpack/index.tsx` always sends `[]`).
        return Vec::new();
    }

    let instance_data = instance_root.join("instance");
    let canonical_data = match tokio::fs::canonicalize(&instance_data).await {
        Ok(p) => p,
        Err(e) => {
            // The staging apply that runs immediately before this already
            // requires this directory to exist — this should never happen,
            // but "never bail" means treating even this as skip-all rather
            // than propagating an error that would ALSO lose the audit
            // write and packinfo promotion, the exact failure mode this
            // function exists to avoid.
            tracing::warn!(
                "skipping all repair cleanup: failed to canonicalize instance data dir {instance_data:?}: {e}"
            );
            return Vec::new();
        }
    };

    let deletable = walk_untracked_files(&instance_data, old_packinfo, target_packinfo).await;

    let mut user_removed = Vec::new();
    for path in cleanup_paths {
        let Some(key) = normalize_cleanup_path(path) else {
            tracing::warn!("skipping repair cleanup of syntactically invalid path {path:?}");
            continue;
        };

        let Some(real_path) = deletable.get(&key) else {
            tracing::warn!(
                "skipping repair cleanup of {key}: not an exact match for any untracked file \
                 currently on disk (either it doesn't exist, or only a differently-spelled \
                 alias of it does)"
            );
            continue;
        };

        let Some(parent) = real_path.parent() else {
            tracing::warn!("skipping repair cleanup of {key}: path has no parent directory");
            continue;
        };
        let canonical_parent = match tokio::fs::canonicalize(parent).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "skipping repair cleanup of {key}: failed to canonicalize parent directory \
                     (the walk found it moments ago, so this is likely a race): {e}"
                );
                continue;
            }
        };
        if !canonical_parent.starts_with(&canonical_data) {
            tracing::warn!(
                "skipping repair cleanup of {key}: resolves outside the instance data dir \
                 once its parent directory's symlinks are followed"
            );
            continue;
        }

        match tokio::fs::remove_file(real_path).await {
            Ok(()) => user_removed.push(key),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!(
                    "repair cleanup of {key}: the walk found it moments ago but it is gone now \
                     (likely a race) — treating as already satisfied"
                );
            }
            Err(e) => {
                tracing::warn!("skipping repair cleanup of {key}: failed to remove: {e}");
            }
        }
    }
    user_removed
}

/// Resolves the physical staged file backing a plan entry, which may sit
/// under its bare name or — when the target ships this path disabled by
/// default — under the `.disabled`-suffixed name. `packinfo::scan_dir`
/// strips that suffix when keying `tmp-packinfo.json`
/// (`packinfo/scan.rs:30-32`), so a [`apply_plan::PlanEntry::path`] never
/// carries it even when the only staged copy does; this is where that gets
/// reconciled against what is physically on disk. The bare spelling wins
/// when (implausibly) both exist. `Some((path, true))` means the resolved
/// copy is the disabled spelling.
fn resolve_staged(staged_bare: &Path) -> Option<(PathBuf, bool)> {
    if staged_bare.is_file() {
        return Some((staged_bare.to_path_buf(), false));
    }
    let twin = disabled_sibling(staged_bare);
    twin.is_file().then_some((twin, true))
}

/// The planner already required a staged source to exist (via the same
/// path-normalised `staged` set) before choosing an action that needs one,
/// so `resolve_staged` failing here means that invariant broke, not a
/// normal runtime condition — still handled as a proper error rather than a
/// panic, since a real filesystem is involved.
fn missing_staged_error(path: &str) -> anyhow::Error {
    anyhow!(
        "planner chose an action requiring a staged source for {path}, but neither the bare nor \
         .disabled-suffixed staged copy exists on disk"
    )
}

/// `<name>` -> `<name>.disabled`, the on-disk convention for a disabled mod.
fn disabled_sibling(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap().to_owned();
    name.push(".disabled");
    path.with_file_name(name)
}

/// Renders the plain-text install audit `apps/desktop/e2e-tests/helpers/installAudit.ts`
/// parses. Pure and unit-tested (`staging_test::render_audit_golden`) — every
/// byte of an existing section is a format contract with that parser, so a
/// change here must stay in lockstep with it. `entries` are expected
/// path-sorted (guaranteed by [`apply_plan::plan`]'s own output), which is
/// what makes each section's line order deterministic.
fn render_audit(entries: &[apply_plan::PlanEntry], user_removed: &[String]) -> String {
    use apply_plan::{PlanAction, PlanReason};
    let mut skipped = String::new();
    let mut deleted = String::new();
    let mut replaced = String::new();
    let mut created = String::new();
    let mut unchanged = String::new();
    let mut re_enabled = String::new();

    for e in entries {
        let file = &e.path;
        match (&e.action, &e.reason) {
            (PlanAction::Keep, PlanReason::DeletedByUser) => {
                skipped += &format!(" - {file}: deleted by user\n")
            }
            (PlanAction::Keep, PlanReason::ModifiedByUser { original, current })
            | (PlanAction::Keep, PlanReason::DroppedButModified { original, current }) => {
                skipped += &format!(
                    " - {file}: modified by user\n     original md5: {}\n     current md5:  {}\n",
                    hex::encode(original),
                    hex::encode(current),
                )
            }
            (PlanAction::Keep, PlanReason::InSaveFolder) => {
                skipped += &format!(" - {file}: files in /saves will never be modified\n")
            }
            (PlanAction::Keep, PlanReason::DisabledByUser) => {
                skipped += &format!(" - {file}: disabled by user\n")
            }
            (PlanAction::Keep, PlanReason::PreservedExisting) => {
                skipped += &format!(" - {file}: already present\n")
            }
            (PlanAction::Keep, _) => unchanged += &format!(" - {file}\n"),
            (PlanAction::Delete, _) => deleted += &format!(" - {file}\n"),
            (PlanAction::Replace, _) | (PlanAction::ReplaceDisabled, _) => {
                replaced += &format!(" - {file}\n")
            }
            (PlanAction::Create, _) => created += &format!(" - {file}\n"),
            (PlanAction::ReEnable, _) => re_enabled += &format!(" - {file}\n"),
        }
    }

    let mut audit = "GDLauncher Modpack Install/Update Audit\n".to_string();
    audit += "\nFiles that could not be replaced:\n";
    audit += &skipped;
    audit += "\nFiles deleted:\n";
    audit += &deleted;
    audit += "\nFiles replaced:\n";
    audit += &replaced;
    audit += "\nFiles created:\n";
    audit += &created;
    audit += "\nFiles unchanged:\n";
    audit += &unchanged;
    audit += "\nFiles re-enabled:\n";
    audit += &re_enabled;
    audit += "\nFiles removed at user request:\n";
    for file in user_removed {
        audit += &format!(" - {file}\n");
    }
    audit
}

#[cfg(test)]
#[path = "staging_test.rs"]
mod staging_test;
