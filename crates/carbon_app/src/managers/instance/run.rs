use daedalus::minecraft::{LibraryGroup, VersionInfo};

use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::{
        instance::{
            self as domain,
            info::{GameVersion, Instance, StandardVersion},
            GameLogId,
        },
        java::{JavaComponent, SystemJavaProfileName},
        runtime_path::{InstancePath, RuntimePath},
        vtask::VisualTaskId,
    },
    managers::{
        self,
        account::FullAccount,
        instance::{log::EntryType, modpacks::PrepareModpack, schema::make_instance_config},
        java::managed::Step,
        minecraft::minecraft::get_lwjgl_meta,
        vtask::{NonFailedDismissError, Subtask, TaskState, VisualTask},
        AppInner, ManagerRef,
    },
};

use std::{fmt::Debug, path::PathBuf, pin::Pin, sync::Arc, time::Duration};

use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, watch, Semaphore},
    task::JoinHandle,
};
use tracing::{debug, info};

use chrono::{DateTime, Utc};
use futures::Future;

use anyhow::{anyhow, bail, Context};

use super::{
    modloaders::PrepareModLoader, modpacks::PrepareModpackSubtasks, InstanceId, InstanceManager,
    InstanceType, InvalidInstanceIdError,
};

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

        let config = data.config.clone();

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

        let version = match config.game_configuration.version {
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
        let info = PrepareInstanceInfo {
            task_id: id,
            instance_id,
            runtime_path,
            instance_path,
            instance_shortpath,
            version,
            config,
        };
        let installation_task = tokio::spawn(async move {
            if let Ok((java, version_info, lwjgl_group)) = prepare_game_installation_task(
                Arc::clone(&app),
                &task,
                wait_task,
                info.clone(),
                callback_task,
            )
            .await
            {
                prepare_game_launch_task(
                    Arc::clone(&app),
                    task,
                    info,
                    launch_account,
                    version_info,
                    LaunchJavaInfo {
                        java,
                        lwjgl_group,
                        xms_memory,
                        xmx_memory,
                        extra_java_args,
                    },
                )
                .await;
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

#[derive(Clone)]
struct PrepareInstanceInfo {
    task_id: VisualTaskId,
    instance_id: InstanceId,
    runtime_path: RuntimePath,
    instance_path: InstancePath,
    instance_shortpath: String,
    version: Option<StandardVersion>,
    config: Instance,
}

async fn prepare_game_installation_task(
    app: Arc<AppInner>,
    task: &VisualTask,
    wait_task: Subtask,
    info: PrepareInstanceInfo,
    callback_task: Option<InstanceCallback>,
) -> anyhow::Result<(JavaComponent, VersionInfo, LibraryGroup)> {
    let mut config = info.config;
    let mut version = info.version;
    let instance_manager = app.instance_manager();
    let task = task;
    let _lock = instance_manager
        .persistence_manager
        .ensure_lock
        .acquire()
        .await
        .expect("the ensure lock semaphore should never be closed");

    let first_run_path = info.instance_path.get_root().join(".first_run_incomplete");
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

    let t_request_version_info = task.subtask(Translation::InstanceTaskLaunchRequestVersions);

    let t_download_files = task.subtask(Translation::InstanceTaskLaunchDownloadFiles);
    t_download_files.set_weight(20.0);
    let t_extract_natives = task.subtask(Translation::InstanceTaskLaunchExtractNatives);

    let t_reconstruct_assets = task.subtask(Translation::InstanceTaskReconstructAssets);

    let t_forge_processors = match is_first_run {
        true => Some(task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors)),
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
    if let Some((t_request, t_download_files, t_extract_files, t_addon_metadata)) = t_modpack {
        let subtasks = PrepareModpackSubtasks {
            t_request,
            t_extract_files,
            t_download_files,
            t_addon_metadata,
        };
        if let Some(modpack) = &config.modpack {
            let v: StandardVersion = modpack
                .prepare_modpack(
                    Arc::clone(&app),
                    info.instance_path.clone(),
                    &mut downloads,
                    subtasks,
                )
                .await
                .with_context(|| {
                    format!(
                        "Error preparing modpack `{:?}` at {} ",
                        &modpack,
                        &info.instance_path.get_root().to_string_lossy()
                    )
                })?;

            tracing::info!("Modpack version: {:?}", v);

            version = Some(v.clone());

            let path = app
                .settings_manager()
                .runtime_path
                .get_instances()
                .to_path()
                .join(info.instance_shortpath);

            config.game_configuration.version = Some(GameVersion::Standard(v.clone()));

            let json = make_instance_config(config.clone())?;
            tokio::fs::write(path.join("instance.json"), json).await?;

            instance_manager
                .instances
                .write()
                .await
                .get_mut(&info.instance_id)
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
        match prepare_java(Arc::clone(&app), &version_info, task).await? {
            Some(java) => java,
            None => return Err(anyhow!("No Java runtime available")),
        }
    };

    for modloader in version.modloaders.iter() {
        version_info = modloader
            .prepare_modloader(Arc::clone(&app), &version, version_info)
            .await?
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

    let concurrency = app
        .settings_manager()
        .get_settings()
        .await?
        .concurrent_downloads;

    carbon_net::download_multiple(downloads, progress_watch_tx, concurrency as usize).await?;

    // update mod metadata after mods are downloaded
    if is_initial_modpack_launch {
        tracing::info!("queueing metadata caching for running instance");

        app.meta_cache_manager()
            .queue_local_caching(info.instance_id, true)
            .await;

        tracing::trace!("queued metadata caching");
    }

    t_extract_natives.start_opaque();
    managers::minecraft::minecraft::extract_natives(
        &info.runtime_path,
        &version_info,
        &lwjgl_group,
        &java.arch,
    )
    .await?;
    t_extract_natives.complete_opaque();

    t_reconstruct_assets.start_opaque();
    managers::minecraft::assets::reconstruct_assets(
        &version_info.assets,
        info.runtime_path.get_assets(),
        info.instance_path.get_resources_path(),
    )
    .await?;
    t_reconstruct_assets.complete_opaque();

    let libraries_path = info.runtime_path.get_libraries();
    let game_version = version_info.id.to_string();
    let client_path = info.runtime_path.get_libraries().get_mc_client(
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
                info.instance_path.clone(),
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

    if let Some(callback_task) = callback_task {
        callback_task(
            t_finalize_import.expect("If callback_task is Some, subtask will also be Some"),
        )
        .await?;
    }

    Ok((java, version_info, lwjgl_group))
}

struct LaunchJavaInfo {
    lwjgl_group: LibraryGroup,
    java: JavaComponent,
    xms_memory: u16,
    xmx_memory: u16,
    extra_java_args: String,
}

async fn prepare_game_launch_task(
    app: Arc<AppInner>,
    task: VisualTask,
    info: PrepareInstanceInfo,
    launch_account: Option<FullAccount>,
    version_info: VersionInfo,
    java_info: LaunchJavaInfo,
) {
    let try_result: anyhow::Result<_> = (|| async {
        match launch_account {
            Some(account) => Ok(Some(
                managers::minecraft::minecraft::launch_minecraft(
                    java_info.java,
                    account,
                    java_info.xmx_memory,
                    java_info.xms_memory,
                    &java_info.extra_java_args,
                    &info.runtime_path,
                    version_info,
                    &java_info.lwjgl_group,
                    info.instance_path,
                )
                .await?,
            )),
            None => {
                let _ = app
                    .instance_manager()
                    .change_launch_state(
                        info.instance_id,
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
                    info.instance_id,
                    LaunchState::Inactive {
                        failed_task: Some(info.task_id),
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

            let (log_id, log) = app.instance_manager().create_log(info.instance_id).await;
            let _ = app
                .instance_manager()
                .change_launch_state(
                    info.instance_id,
                    LaunchState::Running(RunningInstance {
                        process_id: child.id().expect(
                            "child process id is not present even though child process was started",
                        ),
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

            let read_logs = async {
                let mut outbuf = [0u8; 1024];
                let mut errbuf = [0u8; 1024];

                loop {
                    tokio::select!(biased;
                        r = stdout.read(&mut outbuf) => match r {
                            Ok(count) if count > 0 => {
                                let utf8 = String::from_utf8_lossy(&outbuf[0..count]);
                                #[cfg(debug_assertions)]
                                {
                                    tracing::trace!("stdout: {}", utf8);
                                }
                                log.send_if_modified(|log| {
                                    log.push(EntryType::StdOut, &utf8);
                                    false
                                });

                                loop {
                                    tokio::select!(biased;
                                        _ = tokio::time::sleep(Duration::from_millis(1)) => break,
                                        r = stdout.read(&mut outbuf) => match r {
                                            Ok(count) if count > 0 => {
                                                let utf8 = String::from_utf8_lossy(&outbuf[0..count]);
                                                log.send_if_modified(|log| {
                                                    log.push(EntryType::StdOut, &utf8);
                                                    false
                                                });
                                            },
                                            Ok(_) => return Ok(()),
                                            Err(e) => return Err(e),
                                        },
                                    );
                                }
                            },
                            Ok(_) => {},
                            Err(e) => return Err(e),
                        },
                        r = stderr.read(&mut errbuf) => match r {
                            Ok(count) if count > 0 => {
                                let utf8 = String::from_utf8_lossy(&errbuf[0..count]);
                                #[cfg(debug_assertions)]
                                {
                                    tracing::trace!("stderr: {}", utf8);
                                }
                                log.send_if_modified(|log| {
                                    log.push(EntryType::StdErr, &utf8);
                                    false
                                });

                                loop {
                                    tokio::select!(biased;
                                        _ = tokio::time::sleep(Duration::from_millis(1)) => break,
                                        r = stderr.read(&mut errbuf) => match r {
                                            Ok(count) if count > 0 => {
                                                let utf8 = String::from_utf8_lossy(&errbuf[0..count]);
                                                log.send_if_modified(|log| {
                                                    log.push(EntryType::StdErr, &utf8);
                                                    false
                                                });
                                            },
                                            Ok(_) => return Ok(()),
                                            Err(e) => return Err(e),
                                        },
                                    );
                                }
                            },
                            Ok(_) => {},
                            Err(e) => return Err(e),
                        },
                    );

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
                        .update_playtime(info.instance_id, diff.num_seconds() as u64)
                        .await;
                    if let Err(e) = r {
                        tracing::error!({ error = ?e }, "error updating instance playtime");
                    }
                }
            };

            tokio::select! {
                _ = child.wait() => {},
                _ = kill_rx.recv() => drop(child.kill().await),
                // infallible, canceled by the above tasks
                _ = read_logs => {},
                _ = update_playtime => {}
            }

            let r = app
                .instance_manager()
                .update_playtime(
                    info.instance_id,
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
                    info.instance_id,
                    LaunchState::Inactive { failed_task: None },
                )
                .await;
        }
    }
}

pub async fn prepare_java(
    app: Arc<AppInner>,
    version_info: &VersionInfo,
    task: &VisualTask,
) -> anyhow::Result<Option<JavaComponent>> {
    let required_java =
        SystemJavaProfileName::from(daedalus::minecraft::MinecraftJavaProfile::try_from(
            &version_info
                .java_version
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("instance java version unsupported"))?
                .component as &str,
        )?);

    match app.java_manager().get_usable_java(required_java).await? {
        Some(path) => Ok(Some(path)),
        None => {
            let t_download_java = task.subtask(Translation::InstanceTaskLaunchDownloadJava);

            let t_extract_java = task.subtask(Translation::InstanceTaskLaunchExtractJava);
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
                        Step::Downloading(downloaded, total) => {
                            t_download_java.update_download(downloaded as u32, total as u32, true)
                        }
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
                Some(path) => Ok(Some(path)),
                None => Ok(None),
            }
        }
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
    kill_tx: mpsc::Sender<()>,
    start_time: DateTime<Utc>,
    log: GameLogId,
}

impl From<&LaunchState> for domain::LaunchState {
    fn from(value: &LaunchState) -> Self {
        match value {
            LaunchState::Inactive { failed_task } => Self::Inactive {
                failed_task: *failed_task,
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
