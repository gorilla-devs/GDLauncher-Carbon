use crate::domain::instance::info::{self, JavaOverride, Modpack, ModpackInfo, StandardVersion};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use crate::domain::metrics::Event;
use crate::domain::modplatforms::curseforge::filters::ModFileParameters;
use crate::domain::modplatforms::modrinth::search::VersionID;
use crate::domain::runtime_path::InstancePath;
use crate::domain::vtask::VisualTaskId;
use crate::managers::instance::modpack::packinfo;
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
use crate::managers::minecraft::assets::get_assets_dir;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::minecraft::{curseforge, UpdateValue};
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::util::NormalizedWalkdir;

use super::modpack::PackVersionFile;
use super::{InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};
use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::{self as domain, GameLogId};
use crate::managers::instance::log::{GameLog, LogEntry, LogEntrySourceKind};
use crate::managers::instance::schema::make_instance_config;
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
use chrono::{DateTime, Local, Utc};
use futures::Future;
use itertools::Itertools;
use md5::{Digest, Md5};
use std::collections::HashSet;
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinHandle;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{debug, info, trace};

#[derive(Debug)]
pub struct PersistenceManager {
    instance_download_lock: Semaphore,
    loader_install_lock: Semaphore,
}

impl PersistenceManager {
    pub fn new() -> Self {
        Self {
            instance_download_lock: Semaphore::new(1),
            loader_install_lock: Semaphore::new(1),
        }
    }
}
type InstanceCallback = Box<
    dyn FnOnce(Subtask) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> + Send,
>;

impl ManagerRef<'_, InstanceManager> {
    #[tracing::instrument(skip(self, callback_task))]
    pub async fn prepare_game(
        self,
        instance_id: InstanceId,
        launch_account: Option<FullAccount>,
        callback_task: Option<InstanceCallback>,
        deep_check: bool,
    ) -> anyhow::Result<(JoinHandle<()>, VisualTaskId)> {
        let initial_time = Utc::now();

        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &mut instance.type_ else {
            return Err(anyhow!("Instance {instance_id} is not in a valid state"));
        };

        match &data.state {
            LaunchState::Inactive { .. } => {}
            LaunchState::Deleting => {
                bail!("cannot prepare an instance that is being deleted");
            }
            LaunchState::Preparing(task_id) => {
                // dismiss the existing task if its a failure, return if its still in progress.
                let r = self.app.task_manager().dismiss_task(*task_id).await;

                if let Err(e) = r {
                    if e.is::<NonFailedDismissError>() {
                        bail!("cannot prepare an instance that is already being prepared");
                    }
                }
            }
            LaunchState::Running(_) => {
                bail!("cannot prepare an instance that is already running");
            }
        }

        let mut config = data.config.clone();

        let (xms_memory, xmx_memory) = match config.game_configuration.memory {
            Some(memory) => memory,
            None => self
                .app
                .settings_manager()
                .get_settings()
                .await
                .map(|c| (c.xms as u16, c.xmx as u16))?,
        };

        let global_java_args = match config.game_configuration.global_java_args {
            true => self
                .app
                .settings_manager()
                .get_settings()
                .await
                .map(|c| c.java_custom_args)
                .unwrap_or(String::new()),
            false => String::new(),
        };

        let extra_java_args = global_java_args
            + " "
            + config
                .game_configuration
                .extra_java_args
                .as_ref()
                .map(|s| s as &str)
                .unwrap_or("");

        let game_resolution = match config.game_configuration.game_resolution.as_ref() {
            Some(res) => match res {
                info::GameResolution::Custom(w, h) => Some((*w, *h)),
                info::GameResolution::Standard(w, h) => Some((*w, *h)),
            },
            None => {
                let settings = self.app.settings_manager().get_settings().await?;
                settings.game_resolution.and_then(|res_str| {
                    let split_res = res_str
                        .split_once(':')
                        .and_then(|(_, res)| res.split_once('x'))
                        .and_then(|(w, h)| {
                            w.parse::<u16>()
                                .ok()
                                .and_then(|w| h.parse::<u16>().ok().map(|h| (w, h)))
                        });

                    match split_res {
                        Some((w, h)) => Some((w, h)),
                        None => None,
                    }
                })
            }
        };

        let pre_launch_hook = match config.pre_launch_hook.as_ref() {
            Some(hook) => Some(hook.clone()),
            None => {
                let settings = self.app.settings_manager().get_settings().await?;
                settings.pre_launch_hook.clone()
            }
        };

        let post_exit_hook = match config.post_exit_hook.as_ref() {
            Some(hook) => Some(hook.clone()),
            None => {
                let settings = self.app.settings_manager().get_settings().await?;
                settings.post_exit_hook.clone()
            }
        };

        let wrapper_command = match config.wrapper_command.as_ref() {
            Some(cmd) => Some(cmd.clone()),
            None => {
                let settings = self.app.settings_manager().get_settings().await?;
                settings.wrapper_command.clone()
            }
        };

        let java_component_override = match config.game_configuration.java_override.as_ref() {
            Some(path) => match path {
                JavaOverride::Path(value) => {
                    if let Some(value) = value {
                        RealJavaChecker::get_bin_info(
                            &RealJavaChecker,
                            &PathBuf::from(value),
                            JavaComponentType::Custom,
                        )
                        .await
                        .ok()
                    } else {
                        None
                    }
                }
                JavaOverride::Profile(value) => {
                    if let Ok(all_profiles) = self.app.java_manager().get_java_profiles().await {
                        if let Ok(all_javas) = self.app.java_manager().get_available_javas().await {
                            all_profiles.iter().find_map(|profile| {
                                value.as_ref().and_then(|v| {
                                    if &profile.name == v {
                                        let Some(java_id) = profile.java_id.as_ref() else {
                                            return None;
                                        };
                                        all_javas.iter().find_map(|javas| {
                                            javas.1.iter().find_map(|java| {
                                                if &java.id == java_id {
                                                    Some(java.component.clone())
                                                } else {
                                                    None
                                                }
                                            })
                                        })
                                    } else {
                                        None
                                    }
                                })
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            },
            None => None,
        };

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        tracing::debug!("instance path: {:?}", instance_path);

        let mut version = match config.game_configuration.version {
            Some(GameVersion::Standard(ref v)) => Some(v.clone()),
            Some(GameVersion::Custom(_)) => bail!("Custom versions are not supported yet"),
            None if config.modpack.as_ref().is_some() => None,
            None => bail!("Instance has no associated game version and cannot be launched"),
        };

        let task = VisualTask::new(match &launch_account {
            Some(_) => Translation::InstanceTaskLaunch {
                name: config.name.clone(),
            },
            None => Translation::InstanceTaskPrepare {
                name: config.name.clone(),
            },
        });

        let id = self.app.task_manager().spawn_task(&task).await;

        data.state = LaunchState::Preparing(id);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

        let app = self.app.clone();
        let instance_shortpath = instance.shortpath.clone();

        drop(data);
        drop(instance);
        drop(instances);

        let installation_task = tokio::spawn(async move {
            let instance_manager = app.instance_manager();
            let task = task;
            let instance_root = instance_path.get_root();
            let setup_path = instance_root.join(".setup");
            let is_first_run = setup_path.is_dir();

            let mut time_at_start = None;

            let try_result: anyhow::Result<_> = async {
                let do_modpack_install =
                    is_first_run && !setup_path.join("modpack-complete").is_dir();

                let staging_dir = setup_path.join("staging");
                let do_modpack_staging =
                    do_modpack_install && !setup_path.join("staging.json").exists();

                let packinfo_path = instance_root.join("packinfo.json");
                let packinfo = match tokio::fs::read_to_string(packinfo_path).await {
                    Ok(text) => Some(
                        packinfo::parse_packinfo(&text).context("while parsing packinfo json")?,
                    ),
                    Err(_) => None,
                };

                let t_modpack = match do_modpack_staging {
                    true => Some((
                        task.subtask(Translation::InstanceTaskLaunchRequestModpack),
                        task.subtask(Translation::InstanceTaskLaunchDownloadModpack),
                        task.subtask(Translation::InstanceTaskLaunchDownloadModpackFiles),
                        task.subtask(Translation::InstanceTaskLaunchExtractModpackFiles),
                        task.subtask(Translation::InstanceTaskLaunchDownloadAddonMetadata),
                    )),
                    false => None,
                };

                let t_apply_staging =
                    task.subtask(Translation::InstanceTaskLaunchApplyStagedPatches);

                let t_request_version_info =
                    task.subtask(Translation::InstanceTaskLaunchRequestVersions);

                let t_download_files = task.subtask(Translation::InstanceTaskLaunchDownloadFiles);
                t_download_files.set_weight(20.0);

                let t_generating_packinfo =
                    task.subtask(Translation::InstanceTaskGeneratingPackInfo);

                let t_fill_cache = task.subtask(Translation::InstanceTaskFillCache);

                let t_extract_natives = task.subtask(Translation::InstanceTaskLaunchExtractNatives);

                let t_reconstruct_assets = task.subtask(Translation::InstanceTaskReconstructAssets);

                let t_forge_processors = match is_first_run {
                    true => Some(task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors)),
                    false => None,
                };

                let t_neoforge_processors = match is_first_run {
                    true => {
                        Some(task.subtask(Translation::InstanceTaskLaunchRunNeoforgeProcessors))
                    }
                    false => None,
                };

                let t_finalize_import = if callback_task.is_some() {
                    Some(task.subtask(Translation::FinalizingImport))
                } else {
                    None
                };

                task.edit(|data| data.state = TaskState::KnownProgress)
                    .await;

                let dummy_string = daedalus::BRANDING
                    .get_or_init(daedalus::Branding::default)
                    .dummy_replace_string
                    .clone();

                let mut downloads = Vec::new();

                let change_version_path = setup_path.join("change-pack-version.json");

                if let Some((
                    t_request,
                    t_download_packfile,
                    t_download_files,
                    t_extract_files,
                    t_addon_metadata,
                )) = t_modpack
                {
                    let mut downloads = Vec::new();

                    let cffile_path = setup_path.join("curseforge");
                    let mrfile_path = setup_path.join("modrinth");
                    let skip_overrides_path = setup_path.join("modpack-skip-overlays");
                    let skip_overrides = skip_overrides_path.is_dir();

                    let modpack = match tokio::fs::read_to_string(&change_version_path).await {
                        Ok(text) => Some(Modpack::from(serde_json::from_str::<PackVersionFile>(
                            &text,
                        )?)),
                        Err(_) => config.modpack.as_ref().map(|m| m.modpack.clone()),
                    };

                    enum Modplatform {
                        Curseforge,
                        Modrinth,
                    }

                    t_request.start_opaque();

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

                            tokio::spawn(async move {
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

                                t_download_packfile.complete_download();
                            });

                            curseforge::download_modpack_zip(
                                &app,
                                &file,
                                &cffile_path,
                                modpack_progress_tx,
                            )
                            .await?;

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

                            tokio::spawn(async move {
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

                                t_download_packfile.complete_download();
                            });

                            modrinth::download_mrpack(
                                &app,
                                &file,
                                &mrfile_path,
                                modpack_progress_tx,
                            )
                            .await?;

                            Some(Modplatform::Modrinth)
                        }
                    };

                    tokio::fs::create_dir_all(&staging_dir.join("instance")).await?;

                    let instance_prep_path = InstancePath::new(staging_dir.clone());

                    let mut skipped_mods = Vec::new();

                    let v: Option<StandardVersion> = match file {
                        Some(Modplatform::Curseforge) => {
                            let (modpack_progress_tx, mut modpack_progress_rx) =
                                tokio::sync::watch::channel(curseforge::ProgressState::new());

                            t_addon_metadata.start_opaque();

                            tokio::spawn(async move {
                                let mut tracker = curseforge::ProgressState::new();

                                while modpack_progress_rx.changed().await.is_ok() {
                                    {
                                        let progress = modpack_progress_rx.borrow();

                                        tracker.extract_addon_overrides.update_from(
                                            &progress.extract_addon_overrides,
                                            |(completed, total)| {
                                                t_extract_files
                                                    .update_items(completed as u32, total as u32);
                                            },
                                        );
                                    }

                                    tokio::time::sleep(Duration::from_millis(200)).await;
                                }
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

                            tokio::fs::create_dir_all(skip_overrides_path).await?;

                            for (downloadable, skip) in modpack_info.downloadables {
                                match skip {
                                    Some(skippath) => skipped_mods.push(skippath),
                                    None => downloads.push(downloadable),
                                }
                            }

                            let curseforge_version = modpack_info.manifest.minecraft;

                            let gdl_version = convert_cf_version_to_standard_version(
                                app.clone(),
                                curseforge_version,
                                dummy_string.clone(),
                            )
                            .await?;

                            Some(gdl_version)
                        }
                        Some(Modplatform::Modrinth) => {
                            let (modpack_progress_tx, mut modpack_progress_rx) =
                                tokio::sync::watch::channel(modrinth::ProgressState::Idle);

                            tokio::spawn(async move {
                                while modpack_progress_rx.changed().await.is_ok() {
                                    {
                                        let progress = modpack_progress_rx.borrow();
                                        match *progress {
                                            modrinth::ProgressState::Idle => {}
                                            modrinth::ProgressState::ExtractingPackOverrides(
                                                count,
                                                total,
                                            ) => t_extract_files
                                                .update_items(count as u32, total as u32),
                                            modrinth::ProgressState::AcquiringPackMetadata(
                                                count,
                                                total,
                                            ) => t_addon_metadata
                                                .update_items(count as u32, total as u32),
                                        }
                                    }

                                    tokio::time::sleep(Duration::from_millis(200)).await;
                                }
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

                            tokio::fs::create_dir_all(skip_overrides_path).await?;

                            for (downloadable, skip) in modpack_info.downloadables {
                                match skip {
                                    Some(skippath) => skipped_mods.push(skippath),
                                    None => downloads.push(downloadable),
                                }
                            }

                            let modrinth_version = modpack_info.index.dependencies;

                            let gdl_version = convert_mr_version_to_standard_version(
                                app.clone(),
                                modrinth_version,
                            )
                            .await?;

                            Some(gdl_version)
                        }
                        None => None,
                    };

                    let (progress_watch_tx, mut progress_watch_rx) =
                        tokio::sync::watch::channel(carbon_net::Progress::new());

                    t_download_files.start_opaque();

                    // dropped when the sender is dropped
                    tokio::spawn(async move {
                        while progress_watch_rx.changed().await.is_ok() {
                            {
                                let progress = progress_watch_rx.borrow();
                                t_download_files.update_download(
                                    progress.current_size as u32,
                                    progress.total_size as u32,
                                    false,
                                );
                            }

                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }

                        t_download_files.complete_download();
                    });

                    let concurrency = app
                        .settings_manager()
                        .get_settings()
                        .await?
                        .concurrent_downloads;

                    carbon_net::download_multiple(
                        &downloads[..],
                        Some(progress_watch_tx),
                        concurrency as usize,
                        deep_check,
                        false,
                    )
                    .await?;

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

                        config.game_configuration.version =
                            Some(GameVersion::Standard(StandardVersion {
                                release: v.release.clone(),
                                modloaders: v.modloaders.clone(),
                            }));

                        let json = make_instance_config(config.clone())?;
                        tokio::fs::write(path.join("instance.json"), json).await?;

                        instance_manager
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
                    tokio::fs::write(setup_path.join("staging.json"), snapshot).await?;
                }

                if staging_dir.exists() {
                    t_apply_staging.start_opaque();

                    let overwrite_changed = !change_version_path.exists(); // TODO

                    let staged_text =
                        tokio::fs::read_to_string(setup_path.join("staging.json")).await?;
                    let staging_snapshot = serde_json::from_str::<Vec<&str>>(&staged_text)
                        .context("could not parse staging snapshot")?;

                    debug!("Applying staged instance files");
                    let r: anyhow::Result<_> = async {
                        if let Some(packinfo) = packinfo {
                            for (oldfile, oldfilehash) in &packinfo.files {
                                let mut original_file =
                                    instance_root.join("instance").join(&oldfile[1..]);

                                if !original_file.exists() {
                                    let mut name = original_file.file_name().unwrap().to_owned();
                                    name.push(".disabled");
                                    original_file.set_file_name(name);

                                    if !original_file.exists() {
                                        // either the user deleted it or we already deleted it in the next check, skip
                                        continue;
                                    }
                                }

                                let original_conent = tokio::fs::read(&original_file).await?;
                                let original_md5: [u8; 16] = Md5::digest(&original_conent).into();

                                if original_md5 != oldfilehash.md5 {
                                    // the user has modified this file so we shouldn't touch it
                                    continue;
                                }

                                if !staging_snapshot.contains(&(&oldfile as &str)) {
                                    // file is not present in new version and old version was not changed, delete
                                    tokio::fs::remove_file(original_file).await?;
                                    continue;
                                }

                                let staged_file = staging_dir.join("instance").join(&oldfile[1..]);

                                if staged_file.is_file() {
                                    // old file matches the snapshotted version and new file is present, replace
                                    tokio::fs::rename(staged_file, original_file).await?;
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

                    trace!("Cleaning up staging directory");
                    tokio::fs::remove_dir_all(staging_dir).await?;
                    trace!("Staging complete");
                }

                t_apply_staging.complete_opaque();

                let version = match version {
                    Some(v) => v,
                    None => bail!("Instance has no associated game version and cannot be launched"),
                };

                t_request_version_info.update_items(0, 2);

                let mut version_info = app
                    .minecraft_manager()
                    .get_minecraft_version(&version.release)
                    .await
                    .map_err(|e| anyhow::anyhow!("Error getting minecraft version: {:?}", e))?;

                let lwjgl_group = get_lwjgl_meta(
                    Arc::clone(&app.prisma_client),
                    &app.reqwest_client,
                    &version_info,
                    &app.minecraft_manager().meta_base_url,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Error getting lwjgl meta: {:?}", e))?;

                t_request_version_info.update_items(1, 2);

                let java = {
                    if let Some(java_component_override) = java_component_override {
                        java_component_override
                    } else {
                        let mut required_java = SystemJavaProfileName::from(
                            daedalus::minecraft::MinecraftJavaProfile::try_from(
                                &version_info
                                    .java_version
                                    .as_ref()
                                    .ok_or_else(|| {
                                        anyhow::anyhow!("instance java version unsupported")
                                    })?
                                    .component as &str,
                            )?,
                        );

                        // Forge 1.16.5 requires an older java 8 version so we inject the legacy fixed 1 profile
                        if &version.release == "1.16.5"
                            && *&version
                                .modloaders
                                .iter()
                                .find(|v| v.type_ == ModLoaderType::Forge)
                                .is_some()
                        {
                            required_java = SystemJavaProfileName::LegacyFixed1;
                        }

                        tracing::debug!("Required java: {:?}", required_java);

                        let auto_manage_java = app
                            .settings_manager()
                            .get_settings()
                            .await?
                            .auto_manage_java;

                        let usable_java = app
                            .java_manager()
                            .get_usable_java_for_profile_name(required_java)
                            .await?;

                        tracing::debug!("Usable java: {:?}", usable_java);

                        match usable_java {
                            Some(path) => path,
                            None => {
                                if !auto_manage_java {
                                    return bail!(
                                        "No usable java found and auto manage java is disabled"
                                    );
                                }

                                let t_download_java =
                                    task.subtask(Translation::InstanceTaskLaunchDownloadJava);

                                let t_extract_java =
                                    task.subtask(Translation::InstanceTaskLaunchExtractJava);
                                t_download_java.set_weight(0.0);
                                t_extract_java.set_weight(0.0);

                                let (progress_watch_tx, mut progress_watch_rx) =
                                    watch::channel(Step::Idle);

                                // dropped when the sender is dropped
                                tokio::spawn(async move {
                                    let mut started = false;
                                    let mut dl_completed = false;

                                    while progress_watch_rx.changed().await.is_ok() {
                                        let step = progress_watch_rx.borrow();

                                        if !started && !matches!(*step, Step::Idle) {
                                            t_download_java.set_weight(10.0);
                                            t_extract_java.set_weight(3.0);
                                            started = true;
                                        }

                                        match *step {
                                            Step::Downloading(downloaded, total) => t_download_java
                                                .update_download(
                                                    downloaded as u32,
                                                    total as u32,
                                                    true,
                                                ),
                                            Step::Extracting(count, total) => {
                                                if !dl_completed {
                                                    t_download_java.complete_download();
                                                    dl_completed = true;
                                                }

                                                t_extract_java
                                                    .update_items(count as u32, total as u32);
                                            }

                                            Step::Done => {
                                                t_download_java.complete_download();
                                                t_extract_java.complete_items();
                                            }

                                            Step::Idle => {}
                                        }

                                        // this is already debounced in setup_managed
                                    }
                                });

                                let path = app
                                    .java_manager()
                                    .require_java_install(
                                        required_java,
                                        true,
                                        Some(progress_watch_tx),
                                    )
                                    .await?;

                                match path {
                                    Some(path) => path,
                                    None => return Ok(None),
                                }
                            }
                        }
                    }
                };

                for modloader in version.modloaders.iter() {
                    match modloader {
                        ModLoader {
                            type_: ModLoaderType::Forge,
                            version: forge_version,
                        } => {
                            if forge_version.is_empty() {
                                anyhow::bail!("Forge version is empty");
                            }

                            let forge_version = crate::managers::minecraft::forge::get_version(
                                app.prisma_client.clone(),
                                &app.reqwest_client,
                                forge_version,
                                &app.minecraft_manager().meta_base_url,
                            )
                            .await?;

                            version_info = daedalus::modded::merge_partial_version(
                                forge_version,
                                version_info,
                            );
                        }
                        ModLoader {
                            type_: ModLoaderType::Neoforge,
                            version: neoforge_version,
                        } => {
                            if neoforge_version.is_empty() {
                                anyhow::bail!("Neoforge version is empty");
                            }

                            let neoforge_version =
                                crate::managers::minecraft::neoforge::get_version(
                                    app.prisma_client.clone(),
                                    &app.reqwest_client,
                                    neoforge_version,
                                    &app.minecraft_manager().meta_base_url,
                                )
                                .await?;

                            version_info = daedalus::modded::merge_partial_version(
                                neoforge_version,
                                version_info,
                            );
                        }
                        ModLoader {
                            type_: ModLoaderType::Fabric,
                            version: fabric_version,
                        } => {
                            if fabric_version.is_empty() {
                                anyhow::bail!("Fabric version is empty");
                            }

                            let fabric_version =
                                crate::managers::minecraft::fabric::replace_template(
                                    &crate::managers::minecraft::fabric::get_version(
                                        app.prisma_client.clone(),
                                        &app.reqwest_client,
                                        &fabric_version,
                                        &app.minecraft_manager().meta_base_url,
                                    )
                                    .await?,
                                    &version.release,
                                    &dummy_string,
                                );

                            version_info = daedalus::modded::merge_partial_version(
                                fabric_version,
                                version_info,
                            );
                        }
                        ModLoader {
                            type_: ModLoaderType::Quilt,
                            version: quilt_version,
                        } => {
                            if quilt_version.is_empty() {
                                anyhow::bail!("Quilt version is empty");
                            }

                            let quilt_version = crate::managers::minecraft::quilt::replace_template(
                                &crate::managers::minecraft::quilt::get_version(
                                    app.prisma_client.clone(),
                                    &app.reqwest_client,
                                    &quilt_version,
                                    &app.minecraft_manager().meta_base_url,
                                )
                                .await?,
                                &version.release,
                                &dummy_string,
                            );

                            version_info = daedalus::modded::merge_partial_version(
                                quilt_version,
                                version_info,
                            );
                        }
                    }
                }

                t_request_version_info.update_items(2, 2);

                downloads.extend(
                    app.minecraft_manager()
                        .get_all_version_info_files(version_info.clone(), &java.arch)
                        .await?,
                );

                let concurrency = app
                    .settings_manager()
                    .get_settings()
                    .await?
                    .concurrent_downloads;

                let download_required = carbon_net::download_multiple(
                    &downloads[..],
                    None,
                    concurrency as usize,
                    deep_check,
                    true,
                )
                .await?;

                if download_required {
                    let wait_task = task.subtask(Translation::InstanceTaskLaunchWaitDownloadFiles);
                    wait_task.set_weight(0.0);

                    let _lock = instance_manager
                        .persistence_manager
                        .instance_download_lock
                        .acquire()
                        .await
                        .unwrap();

                    let (progress_watch_tx, mut progress_watch_rx) =
                        tokio::sync::watch::channel(carbon_net::Progress::new());

                    // dropped when the sender is dropped
                    tokio::spawn(async move {
                        while progress_watch_rx.changed().await.is_ok() {
                            {
                                let progress = progress_watch_rx.borrow();
                                t_download_files.update_download(
                                    progress.current_size as u32,
                                    progress.total_size as u32,
                                    false,
                                );
                            }

                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }

                        t_download_files.complete_download();
                    });

                    carbon_net::download_multiple(
                        &downloads[..],
                        Some(progress_watch_tx),
                        concurrency as usize,
                        deep_check,
                        false,
                    )
                    .await?;
                }

                // update mod metadata and add modpack complete flag after mods are downloaded
                if is_first_run {
                    trace!("marking modpack initialization as complete");

                    if do_modpack_install {
                        t_generating_packinfo.start_opaque();

                        let staging_path = setup_path.join("staging.json");

                        let staged_text;
                        let mut filter = None;

                        if staging_path.exists() {
                            staged_text = tokio::fs::read_to_string(staging_path).await?;
                            filter =
                                Some(serde_json::from_str::<Vec<&str>>(&staged_text).context(
                                    "could not parse staging snapshot for packinfo creation",
                                )?);
                        }

                        let packinfo =
                            packinfo::scan_dir(&instance_path.get_data_path(), filter.as_ref())
                                .await?;
                        let packinfo_str = packinfo::make_packinfo(packinfo)?;
                        tokio::fs::write(
                            instance_path.get_root().join("packinfo.json"),
                            packinfo_str,
                        )
                        .await?;

                        t_generating_packinfo.complete_opaque();
                    }

                    tokio::fs::create_dir_all(setup_path.join("modpack-complete")).await?;

                    tracing::info!("queueing metadata caching for running instance");

                    t_fill_cache.start_opaque();

                    app.meta_cache_manager()
                        .queue_caching(instance_id, true)
                        .await;

                    t_fill_cache.complete_opaque();

                    trace!("queued metadata caching");
                }

                t_extract_natives.start_opaque();
                managers::minecraft::minecraft::extract_natives(
                    &runtime_path,
                    &version_info,
                    &lwjgl_group,
                    &java.arch,
                )
                .await?;
                t_extract_natives.complete_opaque();

                t_reconstruct_assets.start_opaque();
                managers::minecraft::assets::reconstruct_assets(
                    Arc::clone(&app.prisma_client),
                    app.reqwest_client.clone(),
                    &version_info.asset_index,
                    runtime_path.get_assets(),
                    instance_path.get_resources_path(),
                )
                .await?;
                t_reconstruct_assets.complete_opaque();

                let libraries_path = runtime_path.get_libraries();
                let game_version = version_info.id.to_string();
                let client_path = runtime_path.get_libraries().get_mc_client(
                    version_info
                        .inherits_from
                        .as_ref()
                        .unwrap_or(&version_info.id),
                );
                let assets_dir = get_assets_dir(
                    app.prisma_client.clone(),
                    app.reqwest_client.clone(),
                    &version_info.asset_index,
                    runtime_path.get_assets(),
                    instance_path.get_resources_path(),
                )
                .await
                .unwrap();

                for modloader in version.modloaders.iter() {
                    let instance_path = instance_path.clone();
                    let client_path = client_path.clone();
                    let game_version = game_version.clone();
                    let libraries_path = libraries_path.clone();

                    match modloader {
                        ModLoader {
                            type_: ModLoaderType::Forge,
                            ..
                        } => {
                            if let Some(t_forge_processors) = &t_forge_processors {
                                t_forge_processors.start_opaque();

                                let _lock = instance_manager
                                    .persistence_manager
                                    .loader_install_lock
                                    .acquire()
                                    .await
                                    .unwrap();

                                if let Some(processors) = &version_info.processors {
                                    managers::minecraft::forge::execute_processors(
                                        processors,
                                        version_info.data.as_ref().ok_or_else(|| {
                                            anyhow::anyhow!("Data entries missing")
                                        })?,
                                        PathBuf::from(&java.path),
                                        instance_path,
                                        client_path,
                                        game_version,
                                        libraries_path,
                                        Some(Box::new(|current, total| {
                                            t_forge_processors.update_items(current, total);
                                        })),
                                    )
                                    .await?;
                                }

                                t_forge_processors.complete_opaque();
                            }
                        }
                        ModLoader {
                            type_: ModLoaderType::Neoforge,
                            ..
                        } => {
                            if let Some(t_neoforge_processors) = &t_neoforge_processors {
                                t_neoforge_processors.start_opaque();

                                let _lock = instance_manager
                                    .persistence_manager
                                    .loader_install_lock
                                    .acquire()
                                    .await
                                    .unwrap();

                                if let Some(processors) = &version_info.processors {
                                    managers::minecraft::neoforge::execute_processors(
                                        processors,
                                        version_info.data.as_ref().ok_or_else(|| {
                                            anyhow::anyhow!("Data entries missing")
                                        })?,
                                        PathBuf::from(&java.path),
                                        instance_path.clone(),
                                        client_path,
                                        game_version,
                                        libraries_path,
                                        Some(Box::new(|current, total| {
                                            t_neoforge_processors.update_items(current, total);
                                        })),
                                    )
                                    .await?;
                                }

                                t_neoforge_processors.complete_opaque();
                            }
                        }
                        _ => {}
                    }
                }

                if is_first_run {
                    tokio::fs::remove_dir_all(setup_path).await?;
                }

                match launch_account {
                    Some(account) => {
                        if let Some(pre_launch_hook) = pre_launch_hook.filter(|v| !v.is_empty()) {
                            let mut split = shlex::split(&pre_launch_hook)
                                .ok_or_else(|| anyhow::anyhow!("Failed to parse pre-launch hook"))?
                                .into_iter();

                            let main_command = split
                                .next()
                                .ok_or_else(|| anyhow::anyhow!("Pre-launch hook is empty"))?;

                            let pre_launch_command = tokio::process::Command::new(main_command)
                                .args(split)
                                .current_dir(instance_path.get_data_path())
                                .output()
                                .await
                                .map_err(|e| {
                                    anyhow::anyhow!("Pre-launch hook failed to start: {:?}", e)
                                })?;

                            if !pre_launch_command.status.success() {
                                return Err(anyhow::anyhow!(
                                    "Pre-launch hook failed with status: {:?} \n{}",
                                    pre_launch_command.status,
                                    String::from_utf8(pre_launch_command.stderr)
                                        .unwrap_or_default()
                                ));
                            }

                            tracing::info!(
                                "Pre-launch hook completed successfully {}",
                                String::from_utf8(pre_launch_command.stdout).unwrap_or_default()
                            );
                        }

                        Ok(Some(
                            managers::minecraft::minecraft::launch_minecraft(
                                java,
                                account,
                                xmx_memory,
                                xms_memory,
                                game_resolution,
                                &extra_java_args,
                                &runtime_path,
                                version_info,
                                &lwjgl_group,
                                instance_path.clone(),
                                assets_dir,
                                wrapper_command,
                            )
                            .await?,
                        ))
                    }
                    None => {
                        if let Some(callback_task) = callback_task {
                            callback_task(
                                t_finalize_import
                                    .expect("If callback_task is Some, subtask will also be Some"),
                            )
                            .await?;
                        }

                        let _ = app
                            .instance_manager()
                            .change_launch_state(
                                instance_id,
                                LaunchState::Inactive { failed_task: None },
                            )
                            .await;

                        Ok(None)
                    }
                }
            }
            .await;

            match try_result {
                Err(e) => {
                    task.fail(e).await;

                    let _ = app
                        .instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Inactive {
                                failed_task: Some(id),
                            },
                        )
                        .await;
                }
                Ok(None) => {}
                Ok(Some(mut child)) => {
                    drop(task);

                    let _ = app
                        .rich_presence_manager()
                        .update_activity("Playing Minecraft".to_string())
                        .await;

                    let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

                    let start_time = Utc::now();

                    let (log_id, log) = app.instance_manager().create_log(instance_id).await;
                    let _ = app.instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Running(RunningInstance {
                                process_id: child.id().expect("child process id is not present even though child process was started"),
                                kill_tx,
                                start_time,
                                log: log_id,
                            }),
                        )
                        .await;

                    let (Some(mut stdout), Some(mut stderr)) =
                        (child.stdout.take(), child.stderr.take())
                    else {
                        panic!("stdout and stderr are not availible even though the child process was created with both enabled");
                    };

                    let mut last_stored_time = start_time;
                    let update_playtime = async {
                        loop {
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            let now = Utc::now();
                            let diff = now - last_stored_time;
                            last_stored_time = now;
                            let r = app
                                .instance_manager()
                                .update_playtime(instance_id, diff.num_seconds() as u32)
                                .await;
                            if let Err(e) = r {
                                tracing::error!({ error = ?e }, "error updating instance playtime");
                            }
                        }
                    };

                    time_at_start = Some(Utc::now());

                    tokio::select! {
                        _ = child.wait() => {},
                        _ = kill_rx.recv() => drop(child.kill().await),
                        // infallible, canceled by the above tasks
                        _ = read_logs(&log, &mut stdout,&mut  stderr) => {},
                        _ = update_playtime => {}
                    }

                    let r = app
                        .instance_manager()
                        .update_playtime(
                            instance_id,
                            (Utc::now() - last_stored_time).num_seconds() as u32,
                        )
                        .await;

                    if let Err(e) = r {
                        tracing::error!({ error = ?e }, "error updating instance playtime");
                    }

                    if let Ok(exitcode) = child.wait().await {
                        log.send_modify(|log| {
                            log.add_entry(LogEntry::system_message(format!("{exitcode}")))
                        });
                    }

                    let _ = app.rich_presence_manager().stop_activity().await;

                    if let Some(post_exit_hook) = post_exit_hook.filter(|v| !v.is_empty()) {
                        match shlex::split(&post_exit_hook)
                            .ok_or_else(|| anyhow::anyhow!("Failed to parse post-exit hook"))
                            .map(|v| v.into_iter())
                        {
                            Ok(mut split) => match split.next() {
                                Some(main_command) => {
                                    let post_exit_command =
                                        tokio::process::Command::new(main_command)
                                            .args(split)
                                            .current_dir(instance_path.get_data_path())
                                            .output()
                                            .await;

                                    match post_exit_command {
                                        Ok(post_exit_command) => {
                                            if !post_exit_command.status.success() {
                                                tracing::error!(
                                                    "Post-exit hook failed with status: {:?} \n{}",
                                                    post_exit_command.status,
                                                    String::from_utf8(post_exit_command.stderr)
                                                        .unwrap_or_default()
                                                );
                                            } else {
                                                tracing::info!(
                                                    "Post-exit hook completed successfully {}",
                                                    String::from_utf8(post_exit_command.stdout)
                                                        .unwrap_or_default()
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Post-exit hook failed to start: {:?}",
                                                e
                                            );
                                        }
                                    }
                                }
                                None => {
                                    tracing::error!("Post-exit hook is empty");
                                }
                            },
                            Err(e) => {
                                tracing::error!("Post-exit hook failed to parse: {:?}", e);
                            }
                        }
                    }

                    let _ = app
                        .instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Inactive { failed_task: None },
                        )
                        .await;
                }
            }

            let now = Utc::now();
            let offset_in_sec = Local::now().offset().local_minus_utc();

            let mods = app
                .instance_manager()
                .list_mods(instance_id)
                .await
                .unwrap_or_default()
                .len();

            let Ok(instance_details) = app.instance_manager().instance_details(instance_id).await
            else {
                return;
            };

            if is_first_run {
                let res = app
                    .metrics_manager()
                    .track_event(Event::InstanceInstalled {
                        mods_count: mods as u32,
                        modloader_name: instance_details
                            .modloaders
                            .get(0)
                            .cloned()
                            .map(|v| v.type_.to_string()),
                        modloader_version: instance_details
                            .modloaders
                            .get(0)
                            .cloned()
                            .map(|v| v.version),
                        modplatform: instance_details.modpack.map(|v| v.modpack.to_string()),
                        version: instance_details.version.unwrap_or(String::from("unknown")),
                        seconds_taken: (now - initial_time).num_seconds() as u32,
                    })
                    .await;

                if let Err(e) = res {
                    tracing::error!({ error = ?e }, "failed to track instance installed event");
                }
            } else {
                let Some(time_at_start) = time_at_start else {
                    tracing::error!("time_at_start is None even though this is not the first run");
                    return;
                };

                let res = app
                    .metrics_manager()
                    .track_event(Event::InstanceLaunched {
                        mods_count: mods as u32,
                        modloader_name: instance_details
                            .modloaders
                            .get(0)
                            .cloned()
                            .map(|v| v.type_.to_string()),
                        modloader_version: instance_details
                            .modloaders
                            .get(0)
                            .cloned()
                            .map(|v| v.version),
                        modplatform: instance_details.modpack.map(|v| v.modpack.to_string()),
                        version: instance_details.version.unwrap_or(String::from("unknown")),
                        xmx_memory: xmx_memory as u32,
                        xms_memory: xms_memory as u32,
                        time_to_start_secs: (now - time_at_start).num_seconds() as u64,
                        timestamp_start: initial_time.timestamp(),
                        timestamp_end: now.timestamp(),
                        timezone_offset: offset_in_sec / 60 / 60,
                    })
                    .await;

                if let Err(e) = res {
                    tracing::error!({ error = ?e }, "failed to track instance installed event");
                }
            }
        });

        Ok((installation_task, id))
    }

    async fn change_launch_state(
        self,
        instance_id: InstanceId,
        state: LaunchState,
    ) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let action_to_take = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .launcher_action_on_game_launch;

        match &state {
            LaunchState::Inactive { .. } => {
                // println to stdout is used by the launcher to detect when the game is closed
                println!("_INSTANCE_STATE_:GAME_CLOSED|{action_to_take}");
            }
            LaunchState::Running(_) => {
                // println to stdout is used by the launcher to detect when the game is closed
                println!("_INSTANCE_STATE_:GAME_LAUNCHED|{action_to_take}");
            }
            LaunchState::Preparing(_) | LaunchState::Deleting => (),
        };

        debug!("changing state of instance {instance_id} to {state:?}");
        instance.data_mut()?.state = state;
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

        Ok(())
    }

    pub async fn get_launch_state(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<domain::LaunchState> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        Ok((&instance.data()?.state).into())
    }

    pub async fn kill_instance(self, instance_id: InstanceId) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let LaunchState::Running(running) = &instance.data()?.state else {
            bail!("kill_instance called on instance that was not running")
        };

        info!("killing instance {instance_id}");
        running.kill_tx.send(()).await?;

        Ok(())
    }
}

pub enum LaunchState {
    Inactive { failed_task: Option<VisualTaskId> },
    Preparing(VisualTaskId),
    Running(RunningInstance),
    Deleting,
}

impl Debug for LaunchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Inactive { .. } => "Inactive",
                Self::Preparing(_) => "Preparing",
                Self::Running(_) => "Running",
                Self::Deleting => "Deleting",
            }
        )
    }
}

pub struct RunningInstance {
    process_id: u32,
    kill_tx: mpsc::Sender<()>,
    start_time: DateTime<Utc>,
    log: GameLogId,
}

impl From<&LaunchState> for domain::LaunchState {
    fn from(value: &LaunchState) -> Self {
        match value {
            LaunchState::Inactive { failed_task } => Self::Inactive {
                failed_task: failed_task.clone(),
            },
            LaunchState::Preparing(t) => Self::Preparing(*t),
            LaunchState::Running(RunningInstance {
                start_time, log, ..
            }) => Self::Running {
                start_time: *start_time,
                log_id: *log,
            },
            LaunchState::Deleting => Self::Deleting,
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::domain;
    use chrono::Utc;

    use crate::{
        api::keys,
        domain::instance::info::{self, StandardVersion},
        managers::{account::FullAccount, instance::InstanceVersionSource},
    };

    //#[tokio::test(flavor = "multi_thread", worker_threads = 12)]
    async fn test_launch() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let instance_id = app
            .instance_manager()
            .create_instance(
                app.instance_manager().get_default_group().await?,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(StandardVersion {
                    release: String::from("1.16.5"),
                    modloaders: HashSet::new(),
                })),
                String::new(),
            )
            .await?;

        let account = FullAccount {
            username: String::from("test"),
            uuid: String::from("very real uuid"),
            type_: crate::managers::account::FullAccountType::Offline,
            last_used: Utc::now().into(),
        };

        app.instance_manager()
            .prepare_game(instance_id, Some(account), None, true)
            .await?;

        let task = match app.instance_manager().get_launch_state(instance_id).await? {
            domain::LaunchState::Preparing(taskid) => taskid,
            _ => unreachable!(),
        };

        app.task_manager().wait_with_log(task).await?;
        app.wait_for_invalidation(keys::instance::INSTANCE_DETAILS)
            .await?;
        tracing::info!("Task exited");
        let log_id = match app.instance_manager().get_launch_state(instance_id).await? {
            domain::LaunchState::Inactive { .. } => {
                tracing::info!("Game not running");
                return Ok(());
            }
            domain::LaunchState::Running { log_id, .. } => log_id,
            _ => unreachable!(),
        };

        let mut log = app.instance_manager().get_log(log_id).await?;

        let mut idx = 0;
        while log.changed().await.is_ok() {
            let log = log.borrow();
            let new_lines = log.get_span(idx..);
            idx = log.len();
            for line in new_lines {
                tracing::info!("[{:?}]: {}", line.source_kind, line.message);
            }
        }

        Ok(())
    }
}

/// Reads `stdout` and `stderr`, sending each whole line to the log.
async fn read_logs(
    log: &watch::Sender<GameLog>,
    mut stdout: impl AsyncReadExt + Unpin,
    mut stderr: impl AsyncReadExt + Unpin,
) {
    let mut stdout_line_buf = String::with_capacity(1024);
    let mut stderr_line_buf = String::with_capacity(1024);

    loop {
        // TODO: should we still dispatch modifications on a
        // time based window like before? Traffic should be low enough
        // to not need it
        let modified = tokio::select! { biased;
            modified = read_pipe(
                    &log,
                    LogEntrySourceKind::StdErr,
                    &mut stderr,
                    &mut stdout_line_buf
                ) => { modified }
            modified = read_pipe(
                    &log,
                    LogEntrySourceKind::StdOut,
                    &mut stdout,
                    &mut stderr_line_buf
                ) => { modified }
        };

        log.send_if_modified(|_| modified);
    }
}

/// Performs a single poll for data from the given pipe.
///
/// Returns `true` when a line was fully received, at which point it is
/// safe to flush to the log.
///
/// This function will modify the [`GameLog`], but not notify watchers.
/// It is the responsibility of the caller to ensure notification happens
/// at some point in the future if this function returns `true`.
async fn read_pipe(
    log: &watch::Sender<GameLog>,
    kind: LogEntrySourceKind,
    mut pipe: impl AsyncReadExt + Unpin,
    line_buf: &mut String,
) -> bool {
    let mut buf = [0; 1024];

    match pipe.read(&mut buf).await {
        Ok(size) if size != 0 => {
            let utf8 = String::from_utf8_lossy(&buf[..size]);

            line_buf.push_str(&utf8);

            match carbon_parsing::log::parse_log_entry(line_buf) {
                Ok((rest, entry)) => {
                    log.send_if_modified(|log| {
                        log.add_entry((kind, entry).into());

                        false
                    });

                    let rest = rest.to_owned();
                    *line_buf = rest;

                    true
                }
                // do nothing, wait for more bytes
                Err(nom::Err::Incomplete(_)) => false,
                Err(err) => {
                    tracing::error!("failed to parse log entry:\n{err:#?}");

                    log.send_if_modified(|log| {
                        log.add_entry(LogEntry::system_error(
                            "failed to parse log entry from {kind:?}",
                        ));

                        false
                    });

                    true
                }
            }
        }
        Ok(_) => false,
        Err(err) => {
            tracing::error!("failed to read stdout into the log:\n{err:#?}");

            // We might have missed some data, including a
            // `\n`, so give up on this line and flush it
            log.send_if_modified(|log| {
                log.add_entry(LogEntry::system_error(format!(
                    "failed to receive data from `{kind:?}: {}`",
                    line_buf.as_str()
                )));
                line_buf.clear();

                false
            });

            true
        }
    }
}
