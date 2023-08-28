use crate::domain::instance::info::{Modpack, StandardVersion};
use crate::domain::java::SystemJavaProfileName;
use crate::domain::modplatforms::curseforge::filters::ModFileParameters;
use crate::domain::modplatforms::modrinth::search::VersionID;
use crate::domain::vtask::VisualTaskId;
use crate::managers::java::managed::Step;
use crate::managers::minecraft::curseforge;
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use crate::managers::minecraft::modrinth;
use crate::managers::vtask::Subtask;

use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;

use std::sync::{Arc, atomic};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{debug, info};

use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::{self as domain, GameLogId};
use crate::managers::instance::log::{EntryType, GameLog};
use crate::managers::instance::schema::make_instance_config;
use chrono::{DateTime, Utc};
use futures::Future;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinHandle;

use anyhow::{anyhow, bail};

use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
        ManagerRef,
    },
};

use super::{InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};

#[derive(Debug)]
pub struct PersistenceManager {
    ensure_lock: Semaphore,
}

impl PersistenceManager {
    pub fn new() -> Self {
        Self {
            ensure_lock: Semaphore::new(1),
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
    ) -> anyhow::Result<(JoinHandle<()>, VisualTaskId)> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &mut instance.type_ else {
            return Err(anyhow!("Instance {instance_id} is not in a valid state"));
        };

        match &data.state {
            LaunchState::Inactive { .. } => {}
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

        let wait_task = task.subtask(Translation::InstanceTaskLaunchWaiting);
        wait_task.set_weight(0.0);

        let id = self.app.task_manager().spawn_task(&task).await;

        data.state = LaunchState::Preparing(id);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_INSTANCES_UNGROUPED, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

        let app = self.app.clone();
        let instance_shortpath = instance.shortpath.clone();
        let installation_task = tokio::spawn(async move {
            let instance_manager = app.instance_manager();
            let task = task;
            let _lock = instance_manager
                .persistence_manager
                .ensure_lock
                .acquire()
                .await
                .expect("the ensure lock semaphore should never be closed");

            let try_result: anyhow::Result<_> = (|| async {
                let first_run_path = instance_path.get_root().join(".first_run_incomplete");
                let is_first_run = first_run_path.is_file();

                let t_modpack = match is_first_run {
                    true => Some((
                        task.subtask(Translation::InstanceTaskLaunchRequestModpack),
                        task.subtask(Translation::InstanceTaskLaunchDownloadModpackFiles),
                        task.subtask(Translation::InstanceTaskLaunchExtractModpackFiles),
                        task.subtask(Translation::InstanceTaskLaunchDownloadAddonMetadata),
                    )),
                    false => None,
                };

                let t_request_version_info = task
                    .subtask(Translation::InstanceTaskLaunchRequestVersions);

                let t_download_files = task
                    .subtask(Translation::InstanceTaskLaunchDownloadFiles);
                t_download_files.set_weight(20.0);
                let t_extract_natives = task
                    .subtask(Translation::InstanceTaskLaunchExtractNatives);

                let t_reconstruct_assets = task
                    .subtask(Translation::InstanceTaskReconstructAssets);

                let t_forge_processors = match is_first_run {
                    true => Some(
                        task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors),
                    ),
                    false => None,
                };

                let t_finalize_import = if callback_task.is_some() {
                    Some(task.subtask(Translation::FinalizingImport))
                } else {
                    None
                };

                task.edit(|data| data.state = TaskState::KnownProgress)
                    .await;

                wait_task.complete_opaque();

                let mut downloads = Vec::new();

                let mut is_initial_modpack_launch = false;
                if let Some((t_request, t_download_files, t_extract_files, t_addon_metadata)) =
                    t_modpack
                {
                    if let Some(modpack) = &config.modpack {
                        let v: StandardVersion = match modpack {
                            Modpack::Curseforge(modpack) => {
                                t_request.start_opaque();
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
                                    tokio::sync::watch::channel(curseforge::ProgressState::new());

                                tokio::spawn(async move {
                                    let mut tracker = curseforge::ProgressState::new();

                                    while modpack_progress_rx.changed().await.is_ok() {
                                        {
                                            let progress = modpack_progress_rx.borrow();

                                            tracker.download_addon_zip.update_from(&progress.download_addon_zip, |(downloaded, total)| {
                                                t_download_files.update_download(downloaded as u32, total as u32, true);
                                            });

                                            tracker.extract_addon_overrides.update_from(&progress.extract_addon_overrides, |(completed, total)| {
                                                t_extract_files.update_items(completed as u32, total as u32);
                                            });

                                            tracker.acquire_addon_metadata.update_from(&progress.acquire_addon_metadata, |(completed, total)| {
                                                t_addon_metadata.update_items(completed as u32, total as u32);
                                            });
                                        }

                                        tokio::time::sleep(Duration::from_millis(200)).await;
                                    }
                                });

                                let modpack_info = curseforge::prepare_modpack_from_addon(
                                    &app,
                                    &file,
                                    instance_path.clone(),
                                    modpack_progress_tx,
                                )
                                .await?;

                                downloads.extend(modpack_info.downloadables);

                                modpack_info.manifest.minecraft.try_into()?
                            }
                            Modpack::Modrinth(modpack) =>  {
                                t_request.start_opaque();
                                let file = app
                                    .modplatforms_manager()
                                    .modrinth
                                    .get_version(VersionID(modpack.version_id.clone()))
                                    .await?
                                    .files
                                    .into_iter()
                                    .reduce(|a, b| {
                                        if b.primary {
                                            b
                                        } else {
                                            a
                                        }
                                    })
                                    .ok_or_else(|| anyhow!("Modrinth project '{}' version '{}' does not have a file", modpack.project_id, modpack.version_id))?;
                                t_request.complete_opaque();

                                let (modpack_progress_tx, mut modpack_progress_rx) =
                                    tokio::sync::watch::channel(modrinth::ProgressState::Idle);

                                tokio::spawn(async move {
                                    while modpack_progress_rx.changed().await.is_ok() {
                                        {
                                            let progress = modpack_progress_rx.borrow();
                                            match *progress {
                                                modrinth::ProgressState::Idle => {}
                                                modrinth::ProgressState::DownloadingMRPack(downloaded, total) => {
                                                    t_download_files
                                                        .update_download(downloaded as u32, total as u32, true)
                                                }
                                                modrinth::ProgressState::ExtractingPackOverrides(count, total) => {
                                                    t_extract_files.update_items(count as u32, total as u32)
                                                }
                                                modrinth::ProgressState::AcquiringPackMetadata(count, total) => {
                                                    t_addon_metadata
                                                        .update_items(count as u32, total as u32)
                                                }
                                            }
                                        }

                                        tokio::time::sleep(Duration::from_millis(200)).await;
                                    }

                                    t_download_files.complete_download();
                                });

                                let modpack_info = modrinth::prepare_modpack_from_file(&app, &file, instance_path.clone(), modpack_progress_tx).await?;

                                downloads.extend(modpack_info.downloadables);

                                modpack_info.index.dependencies.try_into()?

                            }
                        };



                        tracing::info!("Modpack version: {:?}", v);

                        version = Some(v.clone());

                        let path = app
                            .settings_manager()
                            .runtime_path
                            .get_instances()
                            .to_path()
                            .join(instance_shortpath);

                        config.game_configuration.version =
                            Some(GameVersion::Standard(StandardVersion {
                                release: v.release.clone(),
                                modloaders: v.modloaders.clone(),
                            }));

                        let json = make_instance_config(config.clone())?;
                        tokio::fs::write(path.join("instance.json"), json).await?;

                        instance_manager.instances.write().await
                            .get_mut(&instance_id)
                            .ok_or_else(|| anyhow!("Instance was deleted while loading"))?
                            .data_mut()?
                            .config = config;

                        is_initial_modpack_launch = true;
                    }
                }

                let version = match version {
                    Some(v) => v,
                    None => bail!("Instance has no associated game version and cannot be launched"),
                };

                t_request_version_info.update_items(0, 3);
                let manifest = app.minecraft_manager().get_minecraft_manifest().await?;
                t_request_version_info.update_items(1, 3);

                let manifest_version = manifest
                    .versions
                    .into_iter()
                    .find(|v| v.id == version.release)
                    .ok_or_else(|| anyhow!("Could not find game version {}", version.release))?;

                let mut version_info = app
                    .minecraft_manager()
                    .get_minecraft_version(manifest_version.clone())
                    .await?;

                let lwjgl_group = get_lwjgl_meta(
                    &app.reqwest_client,
                    &version_info,
                    &app.minecraft_manager().meta_base_url,
                )
                .await?;

                t_request_version_info.update_items(2, 3);

                let java = {
                    let required_java = SystemJavaProfileName::from(
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

                    tracing::debug!("Required java: {:?}", required_java);

                    let usable_java = app.java_manager().get_usable_java(required_java).await?;

                    tracing::debug!("Usable java: {:?}", usable_java);

                    match usable_java {
                        Some(path) => path,
                        None => {
                            let t_download_java = task
                                .subtask(Translation::InstanceTaskLaunchDownloadJava);

                            let t_extract_java = task
                                .subtask(Translation::InstanceTaskLaunchExtractJava);
                            t_download_java.set_weight(0.0);
                            t_extract_java.set_weight(0.0);

                            let (progress_watch_tx, mut progress_watch_rx) = watch::channel(Step::Idle);

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
                                        Step::Downloading(downloaded, total) => t_download_java.update_download(downloaded as u32, total as u32, true),
                                        Step::Extracting(count, total) => {
                                            if !dl_completed {
                                                t_download_java.complete_download();
                                                dl_completed = true;
                                            }

                                            t_extract_java.update_items(count as u32, total as u32);
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
                                .require_java_install(required_java, true, Some(progress_watch_tx))
                                .await?;

                            match path {
                                Some(path) => path,
                                None => return Ok(None),
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
                            let forge_manifest = app.minecraft_manager().get_forge_manifest().await?;

                            let forge_version =
                                match forge_version.strip_prefix(&format!("{}-", version.release)) {
                                    None => forge_version.clone(),
                                    Some(sub) => sub.to_string(),
                                };

                            let forge_manifest_version = forge_manifest
                                .game_versions
                                .into_iter()
                                .find(|v| v.id == version.release)
                                .ok_or_else(|| {
                                    anyhow!("Could not find forge versions for {}", version.release)
                                })?
                                .loaders
                                .into_iter()
                                .find(|v| v.id == format!("{}-{}", version.release, forge_version))
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Could not find forge version {}-{} for minecraft version {}",
                                        version.release,
                                        forge_version,
                                        version.release,
                                    )
                                })?;

                            let forge_version = crate::managers::minecraft::forge::get_version(
                                &app.reqwest_client,
                                forge_manifest_version,
                            )
                            .await?;

                            version_info =
                                daedalus::modded::merge_partial_version(forge_version, version_info);
                        }
                        ModLoader {
                            type_: ModLoaderType::Fabric,
                            version: fabric_version,
                        } => {
                            let fabric_manifest = app.minecraft_manager().get_fabric_manifest().await?;

                            let fabric_version =
                                match fabric_version.strip_prefix(&format!("{}-", version.release)) {
                                    None => fabric_version.clone(),
                                    Some(sub) => sub.to_string(),
                                };

                            let dummy_string = daedalus::BRANDING
                                .get_or_init(daedalus::Branding::default)
                                .dummy_replace_string
                                .clone();

                            let supported = fabric_manifest
                                    .game_versions
                                    .iter()
                                    .any(|v| v.id == version.release);

                            if !supported {
                                return Err(anyhow!("Fabric does not support version {}", version.release));
                            }

                            let fabric_manifest_version = fabric_manifest
                                .game_versions
                                .into_iter()
                                .find(|v| v.id == dummy_string)
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Could not find fabric metadata template using {}",
                                        dummy_string
                                    )
                                })?
                                .loaders
                                .into_iter()
                                .find(|v| v.id == fabric_version)
                                .ok_or_else(|| {
                                    anyhow!("Could not find fabric version {}", fabric_version)
                                })?;

                            let fabric_version = crate::managers::minecraft::fabric::replace_template(
                                &crate::managers::minecraft::fabric::get_version(
                                    &app.reqwest_client,
                                    fabric_manifest_version,
                                )
                                .await?,
                                &version.release,
                                &dummy_string,
                            );

                            version_info =
                                daedalus::modded::merge_partial_version(fabric_version, version_info);
                        }
                        ModLoader {
                            type_: ModLoaderType::Quilt,
                            version: quilt_version,
                        } => {
                            let quilt_manifest = app.minecraft_manager().get_quilt_manifest().await?;

                            let quilt_version =
                                match quilt_version.strip_prefix(&format!("{}-", version.release)) {
                                    None => quilt_version.clone(),
                                    Some(sub) => sub.to_string(),
                                };

                            let dummy_string = daedalus::BRANDING
                                .get_or_init(daedalus::Branding::default)
                                .dummy_replace_string
                                .clone();

                            let supported = quilt_manifest
                                    .game_versions
                                    .iter()
                                    .any(|v| v.id == version.release);

                            if !supported {
                                return Err(anyhow!("Quilt does not support version {}", version.release));
                            }

                            let quilt_manifest_version = quilt_manifest
                                .game_versions
                                .into_iter()
                                .find(|v| v.id == dummy_string)
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Could not find quilt metadata template using {}",
                                        dummy_string
                                    )
                                })?
                                .loaders
                                .into_iter()
                                .find(|v| v.id == quilt_version)
                                .ok_or_else(|| {
                                    anyhow!("Could not find quilt version {}", quilt_version)
                                })?;

                            let quilt_version = crate::managers::minecraft::quilt::replace_template(
                                &crate::managers::minecraft::quilt::get_version(
                                    &app.reqwest_client,
                                    quilt_manifest_version,
                                )
                                .await?,
                                &version.release,
                                &dummy_string,
                            );

                            version_info =
                                daedalus::modded::merge_partial_version(quilt_version, version_info);
                        }
                    }
                }

                t_request_version_info.update_items(3, 3);

                downloads.extend(
                    app.minecraft_manager()
                        .get_all_version_info_files(version_info.clone(), &java.arch)
                        .await?,
                );

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

                let concurrency = app.settings_manager().get_settings().await?.concurrent_downloads;

                carbon_net::download_multiple(downloads, progress_watch_tx, concurrency as usize).await?;

                // update mod metadata after mods are downloaded
                if is_initial_modpack_launch {
                    tracing::info!("queueing metadata caching for running instance");

                    app.meta_cache_manager()
                        .queue_local_caching(instance_id, true)
                        .await;

                    tracing::trace!("queued metadata caching");
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
                    &version_info.assets,
                    runtime_path.get_assets(),
                    instance_path.get_resources_path(),
                ).await?;
                t_reconstruct_assets.complete_opaque();

                let libraries_path = runtime_path.get_libraries();
                let game_version = version_info.id.to_string();
                let client_path = runtime_path.get_libraries().get_mc_client(
                    version_info
                        .inherits_from
                        .as_ref()
                        .unwrap_or(&version_info.id),
                );

                if let Some(t_forge_processors) = &t_forge_processors {
                    t_forge_processors.start_opaque();

                    if let Some(processors) = &version_info.processors {
                        managers::minecraft::forge::execute_processors(
                            processors,
                            version_info
                                .data
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Data entries missing"))?,
                            PathBuf::from(&java.path),
                            instance_path.clone(),
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

                let _ = tokio::fs::remove_file(first_run_path).await;

                match launch_account {
                    Some(account) => Ok(Some(
                        managers::minecraft::minecraft::launch_minecraft(
                            java,
                            account,
                            xmx_memory,
                            xms_memory,
                            &extra_java_args,
                            &runtime_path,
                            version_info,
                            &lwjgl_group,
                            instance_path,
                        )
                        .await?,
                    )),
                    None => {
                        if let Some(callback_task) = callback_task {
                            callback_task(t_finalize_import.expect("If callback_task is Some, subtask will also be Some")).await?;
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
            })()
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
                    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
                    let stopping = Arc::new(AtomicBool::new(false));

                    let start_time = Utc::now();

                    let (log_id, log) = app.instance_manager().create_log(instance_id).await;
                    let _ = app.instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Running(RunningInstance {
                                process_id: child.id().expect("child process id is not present even though child process was started"),
                                kill_tx,
                                stop_tx,
                                start_time,
                                log: log_id,
                                stopping: stopping.clone(),
                            }),
                        )
                        .await;

                    let (Some(mut stdout), Some(mut stderr)) =
                        (child.stdout.take(), child.stderr.take())
                    else {
                        panic!("stdout and stderr are not availible even though the child process was created with both enabled");
                    };

                    let read_logs = async {
                        async fn read_step<'a>(
                            log: &'a watch::Sender<GameLog>,
                            entry_type: EntryType,
                            stream: &'a mut (impl AsyncReadExt + Unpin),
                        ) -> io::Result<impl Future<Output = io::Result<()>> + 'a>
                        {
                            let mut buf = [0u8; 1024];
                            stream.read(&mut buf[..]).await.map(|count| async move {
                                if count > 0 {
                                    let utf8 = String::from_utf8_lossy(&buf[0..count]);
                                    log.send_if_modified(|log| {
                                        log.push(entry_type, &*utf8);
                                        false
                                    });

                                    loop {
                                        tokio::select!(biased;
                                            _ = tokio::time::sleep(Duration::from_millis(1)) => break,
                                            count = stream.read(&mut buf[..]) => count.map(|count| {
                                                if count > 0 {
                                                    let utf8 = String::from_utf8_lossy(&buf[0..count]);
                                                    log.send_if_modified(|log| {
                                                        log.push(entry_type, &*utf8);
                                                        false
                                                    });
                                                }
                                            })?
                                        );
                                    }
                                }

                                Ok(())
                            })
                        }

                        loop {
                            let r = async {
                                tokio::select!(biased;
                                    cont = read_step(&log, EntryType::StdOut, &mut stdout) => cont?.await,
                                    cont = read_step(&log, EntryType::StdErr, &mut stderr) => cont?.await,
                                )
                            }.await;

                            if let Err(e) = r {
                                tracing::error!({ error = ?e }, "game log reader died");
                            }

                            log.send_if_modified(|_| true);
                        }
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
                                .update_playtime(instance_id, diff.num_seconds() as u64)
                                .await;
                            if let Err(e) = r {
                                tracing::error!({ error = ?e }, "error updating instance playtime");
                            }
                        }
                    };

                    let child_pid = child.id();
                    let (chainkill_tx, mut chainkill_rx) = mpsc::channel::<()>(1);

                    if let Some(child_pid) = child_pid {
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = chainkill_rx.recv() => {},
                                v = stop_rx.recv() => if v.is_some() {
                                    stopping.store(true, atomic::Ordering::Relaxed);

                                    let r = crate::reaper::stop_process(child_pid);

                                    match r {
                                        Ok(()) => tracing::info!("sent stop signal to game child process"),
                                        Err(e) => tracing::error!({ error = ?e }, "failed to stop child process"),
                                    }
                                },
                            }
                        });
                    }

                    tokio::select! {
                        _ = child.wait() => {},
                        _ = kill_rx.recv() => {
                            let _ = child.kill();
                            let _ = child.wait().await;
                        },
                        // infallible, canceled by the above tasks
                        _ = read_logs => {},
                        _ = update_playtime => {}
                    }

                    let _ = chainkill_tx.send(()).await;

                    let r = app
                        .instance_manager()
                        .update_playtime(
                            instance_id,
                            (Utc::now() - last_stored_time).num_seconds() as u64,
                        )
                        .await;
                    if let Err(e) = r {
                        tracing::error!({ error = ?e }, "error updating instance playtime");
                    }

                    if let Ok(exitcode) = child.wait().await {
                        log.send_modify(|log| log.push(EntryType::System, &exitcode.to_string()));
                    }

                    let _ = app.rich_presence_manager().stop_activity().await;

                    let _ = app
                        .instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Inactive { failed_task: None },
                        )
                        .await;
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

        debug!("changing state of instance {instance_id} to {state:?}");
        instance.data_mut()?.state = state;
        self.app.invalidate(GET_INSTANCES_UNGROUPED, None);
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

    pub async fn stop_instance(self, instance_id: InstanceId) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let LaunchState::Running(running) = &instance.data()?.state else {
            bail!("stop_instance called on instance that was not running")
        };

        if running.stopping.load(atomic::Ordering::Relaxed) {
            bail!("stop_instance called on instance that was already stopping")
        }

        info!("stopping instance {instance_id}");
        running.stop_tx.send(()).await?;

        Ok(())
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
            }
        )
    }
}

pub struct RunningInstance {
    process_id: u32,
    stop_tx: mpsc::Sender<()>,
    kill_tx: mpsc::Sender<()>,
    start_time: DateTime<Utc>,
    log: GameLogId,
    stopping: Arc<AtomicBool>,
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
            .prepare_game(instance_id, Some(account), None)
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
            let new_lines = log.get_region(idx..);
            idx = log.len();
            for line in new_lines {
                tracing::info!("[{:?}]: {}", line.type_, line.text);
            }
        }

        Ok(())
    }
}
