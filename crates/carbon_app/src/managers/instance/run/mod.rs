use super::log::LogProcessor;
use super::modpack::PackVersionFile;
use super::{InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};
use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::instance::info::{self, JavaOverride, Modpack, ModpackInfo, StandardVersion},
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    domain::instance::{self as domain, GameLogId},
    domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName},
    domain::metrics::GDLMetricsEvent,
    domain::vtask::VisualTaskId,
    managers::instance::log::{
        format_message_as_log4j_event, GameLog, LogEntry, LogEntrySourceKind,
    },
    managers::instance::modpack::packinfo,
    managers::instance::schema::make_instance_config,
    managers::java::java_checker::{JavaChecker, RealJavaChecker},
    managers::java::managed::Step,
    managers::minecraft::assets::get_assets_dir,
    managers::minecraft::minecraft::get_lwjgl_meta,
    managers::minecraft::modrinth,
    managers::minecraft::{curseforge, UpdateValue},
    managers::modplatforms::curseforge::convert_cf_version_to_standard_version,
    managers::modplatforms::modrinth::convert_mr_version_to_standard_version,
    managers::vtask::Subtask,
    managers::{
        self,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
        ManagerRef,
    },
    util::NormalizedWalkdir,
};
use anyhow::{anyhow, bail, Context};
use carbon_net::DownloadOptions;
use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, Utc};
use futures::Future;
use itertools::Itertools;
use md5::{Digest, Md5};
use modpack::TSubtasks;
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

mod java;
mod minecraft;
mod modpack;

#[derive(Debug)]
pub struct PersistenceManager {
    instance_download_lock: Semaphore,
    loader_install_lock: Semaphore,
    java_check_lock: Mutex<()>,
}

impl PersistenceManager {
    pub fn new() -> Self {
        Self {
            instance_download_lock: Semaphore::new(1),
            loader_install_lock: Semaphore::new(1),
            java_check_lock: Mutex::new(()),
        }
    }
}
type InstanceCallback = Box<
    dyn FnOnce(&Subtask) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> + Send,
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

        let auto_manage_java_system_profiles = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .auto_manage_java_system_profiles;

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

        let java_override = config.game_configuration.java_override.clone();

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        tracing::debug!("instance path: {:?}", instance_path);

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

        let (log_id, log) = app.instance_manager().create_log(instance_id, None).await;

        let now = Utc::now();

        let log_file_name = format!("{}_{}", now.format("%Y-%m-%d"), now.format("%H-%M-%S"));

        let logs_file_path = if launch_account.is_some() {
            Some(
                instance_path
                    .get_gdl_logs_path()
                    .join(format!("{}.log", log_file_name)),
            )
        } else {
            None
        };

        let logs_file_path_clone = logs_file_path.clone();
        let logs_file_path_clone_1 = logs_file_path.clone();

        let file_fut = logs_file_path
            .and_then(|p| p.parent().map(|p| p.to_owned()))
            .map(|p| async {
                if let Err(e) =
                    tokio::fs::create_dir_all(&logs_file_path_clone.unwrap().parent().unwrap())
                        .await
                {
                    tracing::error!({ error = ?e }, "Failed to create log directory");
                }
            })
            .map(|f| async {
                f.await;
                tokio::fs::File::create(&logs_file_path_clone_1.unwrap()).await
            });

        let mut file = match file_fut {
            Some(f) => f.await.ok(),
            None => None,
        };

        app.meta_cache_manager()
            .watch_and_prioritize(Some(instance_id))
            .await;

        let result = app.instance_manager().list_mods(instance_id).await?;
        let msg = format!(
            "Mods ({} enabled / {} disabled): {}",
            result.iter().filter(|mod_| mod_.enabled).count(),
            result.iter().filter(|mod_| !mod_.enabled).count(),
            result.into_iter().fold(String::new(), |mut acc, mod_| {
                acc.push_str("\n\t [");
                if mod_.enabled {
                    acc.push_str("x]");
                } else {
                    acc.push_str(" ]");
                }

                acc.push(' ');
                acc.push_str(&mod_.filename);

                acc
            })
        );

        log.send_modify(|log| {
            log.add_entry(LogEntry::system_message(msg.clone()));
        });
        if let Some(file) = file.as_mut() {
            file.write_all(format_message_as_log4j_event(&msg).as_bytes())
                .await?;
        }

        let installation_task = tokio::spawn(async move {
            let mut time_at_start = None;

            let instance_root = instance_path.get_root();
            let setup_path = instance_root.join(".setup");
            let is_setup = setup_path.is_dir();

            let try_result: anyhow::Result<_> = async {
                let mut downloads = Vec::new();

                let (t_subtasks, modpack_version) = modpack::process_modpack(
                    Arc::clone(&app),
                    instance_id.clone(),
                    deep_check,
                    config,
                    instance_shortpath.clone(),
                    &task,
                    callback_task.is_some(),
                )
                .await?;
                modpack::process_modpack_staging(
                    Arc::clone(&app),
                    instance_shortpath.clone(),
                    &t_subtasks,
                )
                .await?;

                let version = if modpack_version.is_some() {
                    modpack_version
                } else {
                    version
                };

                let version = match version {
                    Some(v) => v,
                    None => bail!("Instance has no associated game version and cannot be launched"),
                };

                t_subtasks.t_request_version_info.start_opaque();

                let mut version_info = app
                    .minecraft_manager()
                    .get_minecraft_version(&version.release)
                    .await
                    .map_err(|e| anyhow::anyhow!("Error getting minecraft version: {:?}", e))?;

                t_subtasks.t_request_version_info.complete_opaque();

                let java = java::check_and_install(
                    Arc::clone(&app),
                    &version_info,
                    &t_subtasks,
                    &version,
                    java_override,
                    auto_manage_java_system_profiles,
                    &log,
                    file.as_mut(),
                )
                .await?;

                let (lwjgl_group, assets_dir, version_info) = minecraft::process_minecraft(
                    Arc::clone(&app),
                    instance_id,
                    deep_check,
                    instance_shortpath,
                    &t_subtasks,
                    version_info,
                    &version,
                    &java,
                    &log,
                    file.as_mut(),
                    &mut downloads,
                )
                .await?;

                // If the setup path exists, let's delete it because installation is now complete
                if setup_path.exists() {
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
                            let t_subtasks = t_subtasks.t_finalize_import.as_ref();
                            callback_task(
                                t_subtasks
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

                    let _liveness_watch = app.instance_manager().instance_running_tracker.marker();

                    let _ = app
                        .rich_presence_manager()
                        .update_activity("Playing Minecraft".to_string())
                        .await;

                    let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

                    let start_time = Utc::now();

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

                    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take())
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
                        _ = child.wait() => {
                            tracing::info!("Instance waited");
                        },
                        _ = kill_rx.recv() => {
                            tracing::info!("Instance killed");
                            drop(child.kill().await);
                        },
                        _ = read_logs(log.clone(), stdout, stderr, file.as_mut()) => {
                            tracing::info!("Instance read logs");
                        },
                        _ = update_playtime => {
                            tracing::info!("Instance updated playtime");
                        }
                    }

                    tracing::info!("Instance exited");

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
                        let msg = format!("{exitcode}");

                        log.send_modify(|log| log.add_entry(LogEntry::system_message(msg.clone())));

                        if let Some(file) = file.as_mut() {
                            // TODO: not sure how to handle an error in here
                            let _ = file
                                .write_all(format_message_as_log4j_event(&msg).as_bytes())
                                .await;
                        }
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

            app.invalidate(GET_LOGS, Some(instance_id.0.into()));

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

            if is_setup {
                let res = app
                    .metrics_manager()
                    .track_event(GDLMetricsEvent::InstanceInstalled {
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
                    .track_event(GDLMetricsEvent::InstanceLaunched {
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
                info!("_INSTANCE_STATE_:GAME_CLOSED|{action_to_take}");
                println!("_INSTANCE_STATE_:GAME_CLOSED|{action_to_take}");
            }
            LaunchState::Running(_) => {
                // println to stdout is used by the launcher to detect when the game is closed
                info!("_INSTANCE_STATE_:GAME_LAUNCHED|{action_to_take}");
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

async fn read_logs(
    log: watch::Sender<GameLog>,
    stdout: impl AsyncReadExt + Unpin + Send + 'static,
    stderr: impl AsyncReadExt + Unpin + Send + 'static,
    file: Option<&mut File>,
) {
    let (stdout_tx, stdout_rx) = mpsc::channel::<Vec<u8>>(1000);
    let (stderr_tx, stderr_rx) = mpsc::channel::<Vec<u8>>(1000);

    let stdout_task = tokio::spawn(read_pipe(stdout, stdout_tx));
    let stderr_task = tokio::spawn(read_pipe(stderr, stderr_tx));

    process_logs(log, stdout_rx, stderr_rx, file).await;

    let _ = tokio::join!(stdout_task, stderr_task);
}

async fn read_pipe(
    mut pipe: impl AsyncReadExt + Unpin + Send + 'static,
    tx: mpsc::Sender<Vec<u8>>,
) {
    let mut buf = [0; 1024];

    loop {
        match pipe.read(&mut buf).await {
            Ok(size) if size != 0 => {
                if let Err(e) = tx.send(buf[..size].to_vec()).await {
                    tracing::error!("Failed to send data through channel: {}", e);
                    break;
                }

                tracing::trace!("Got log event from pipe");
            }
            Ok(_) => {
                tracing::trace!("Got EOF from pipe");
                break;
            }
            Err(e) => {
                tracing::error!("Failed to read from pipe: {}", e);
                break;
            }
        }
    }
}

async fn process_logs(
    log: watch::Sender<GameLog>,
    mut stdout_rx: mpsc::Receiver<Vec<u8>>,
    mut stderr_rx: mpsc::Receiver<Vec<u8>>,
    mut file: Option<&mut File>,
) {
    let mut stdout_processor = LogProcessor::new(LogEntrySourceKind::StdOut, log.clone()).await;

    let mut stderr_processor = LogProcessor::new(LogEntrySourceKind::StdErr, log).await;

    loop {
        tokio::select! {
            Some(data) = stdout_rx.recv() => {
                if let Err(e) = stdout_processor.process_data(&data, file.as_deref_mut()).await {
                    tracing::error!("Failed to process stdout data: {}", e);
                }
            }
            Some(data) = stderr_rx.recv() => {
                if let Err(e) = stderr_processor.process_data(&data, file.as_deref_mut()).await {
                    tracing::error!("Failed to process stderr data: {}", e);
                }
            }
            else => break,
        }
    }
}
