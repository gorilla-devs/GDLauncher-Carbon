use super::log::{CappedLogFile, LogProcessor};
use super::modpack::PackVersionFile;
use super::{Instance, InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};
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
        GameLog, LogEntry, LogEntrySourceKind, format_message_as_log4j_event,
    },
    managers::instance::modpack::packinfo,
    managers::instance::schema::make_instance_config,
    managers::java::java_checker::{JavaChecker, RealJavaChecker},
    managers::java::managed::Step,
    managers::minecraft::assets::get_assets_dir,
    managers::minecraft::minecraft::get_lwjgl_meta,
    managers::minecraft::modrinth,
    managers::minecraft::{UpdateValue, curseforge},
    managers::modplatforms::curseforge::convert_cf_version_to_standard_version,
    managers::modplatforms::modrinth::convert_mr_version_to_standard_version,
    managers::orphan_pid,
    managers::vtask::Subtask,
    managers::{
        self, ManagerRef,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
    },
    util::NormalizedWalkdir,
};
use anyhow::{Context, anyhow, bail};
use carbon_net::DownloadOptions;
use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, Utc};
use futures::Future;
use itertools::Itertools;
use md5::{Digest, Md5};
use modpack::TSubtasks;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{debug, info, trace, warn};

mod java;
mod minecraft;
mod modpack;

#[derive(thiserror::Error, Debug)]
#[error(
    "Minecraft needs an estimated {needed_mb} MB ({requested_mb} MB heap + JVM overhead) but only {available_mb} MB is available"
)]
pub struct InsufficientMemoryError {
    pub instance_id: i32,
    pub requested_mb: u64,
    pub needed_mb: u64,
    pub available_mb: u64,
}

impl crate::error::FeErrorCode for InsufficientMemoryError {
    fn error_code(&self) -> &'static str {
        "INSUFFICIENT_MEMORY"
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "instance_id": self.instance_id,
            "requested_mb": self.requested_mb,
            "needed_mb": self.needed_mb,
            "available_mb": self.available_mb
        }))
    }
}

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
/// Maximum time a user-configured pre/post-launch hook may run before it is killed. Generous
/// enough for real setup scripts, but bounds a hook that never exits so it cannot wedge the
/// launch (or, for a pre-launch hook, the whole launch queue) until the app is restarted.
const HOOK_TIMEOUT: Duration = Duration::from_secs(300);

type InstanceCallback = Box<
    dyn FnOnce(&Subtask) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> + Send,
>;

/// Clamp the stored i32 heap settings (MB) into the u16 the JVM args use, saturating instead of
/// wrapping: a raw `as u16` cast would turn e.g. 66000 MB into 464 MB and silently hand the JVM
/// a tiny heap.
fn clamp_heap_mb(xms: i32, xmx: i32) -> (u16, u16) {
    (
        xms.clamp(0, u16::MAX as i32) as u16,
        xmx.clamp(0, u16::MAX as i32) as u16,
    )
}

/// Split a user-configured hook command line into program + arguments. shlex uses POSIX
/// backslash escaping, so on Windows a path like `C:\tools\setup.bat` would lose its separators
/// (or fail to parse on a trailing one); escape backslashes first there so they survive the
/// split, mirroring how the wrapper command is handled in `launch_minecraft`.
fn split_hook_command(raw: &str) -> Option<Vec<String>> {
    #[cfg(target_os = "windows")]
    let escaped = raw.replace('\\', "\\\\");
    #[cfg(target_os = "windows")]
    let raw: &str = &escaped;
    shlex::split(raw)
}

impl ManagerRef<'_, InstanceManager> {
    /// Resolve the effective memory (xms, xmx) for an instance.
    /// Uses instance-level override if set, otherwise falls back to global settings.
    pub async fn get_effective_memory(self, instance_id: InstanceId) -> anyhow::Result<(u16, u16)> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            return Err(anyhow!("Instance {instance_id} is not in a valid state"));
        };

        match data.config.game_configuration.memory {
            Some(memory) => Ok(memory),
            None => self
                .app
                .settings_manager()
                .get_settings()
                .await
                .map(|c| clamp_heap_mb(c.xms, c.xmx)),
        }
    }

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
            LaunchState::Queued(task_id) | LaunchState::Preparing(task_id) => {
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
                .map(|c| clamp_heap_mb(c.xms, c.xmx))?,
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

        data.state = LaunchState::Queued(id);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

        let app = self.app.clone();
        let instance_shortpath = instance.shortpath.clone();

        drop(data);
        drop(instance);
        drop(instances);

        // Capture datetime once to ensure log entry and file name match exactly
        let now = Local::now();

        let (log_id, log) = if launch_account.is_some() {
            let (id, sender) = app
                .instance_manager()
                .create_log(instance_id, Some(now))
                .await;
            (Some(id), Some(sender))
        } else {
            (None, None)
        };

        let log_file_name = format!("{}", now.format("%Y-%m-%d_%H-%M-%S"));

        let logs_file_path = if launch_account.is_some() {
            let gdl_logs_path = instance_path.get_gdl_logs_path();

            // Same retention as the launcher's own __gdl_logs__; these files previously
            // accumulated forever. Deleting can involve huge files, so keep the blocking
            // fs work off the async runtime.
            let cleanup_path = gdl_logs_path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                crate::logger::cleanup_old_logs(&cleanup_path, 10);
            })
            .await;

            Some(gdl_logs_path.join(format!("{}.log", log_file_name)))
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

        // Logging the mod list is best-effort: a failure to read the mod list or write the log
        // line must not abort the launch and leave the instance stuck in Queued with no task to
        // drive it out. The installation task spawned below is what actually drives the state.
        match app.instance_manager().list_mods(instance_id, None).await {
            Ok(result) => {
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

                if let Some(file) = file.as_mut() {
                    if let Some(log) = log.as_ref() {
                        log.send_modify(|log| {
                            log.add_entry(LogEntry::system_message(msg.clone()));
                        });
                    }
                    if let Err(e) = file
                        .write_all(format_message_as_log4j_event(&msg).as_bytes())
                        .await
                    {
                        tracing::warn!({ error = ?e }, "Failed to write mod list to log file");
                    }
                }
            }
            Err(e) => {
                tracing::warn!({ error = ?e }, "Failed to list mods for the launch log");
            }
        }

        let installation_task = tokio::spawn(async move {
            let mut time_at_start = None;

            let instance_root = instance_path.get_root();
            let setup_path = instance_root.join(".setup");
            let is_setup = setup_path.is_dir();

            // Acquire semaphore FIRST - this is where queuing happens
            // Instance stays in Queued state until we get the lock
            let instance_manager = app.instance_manager();
            let download_guard = instance_manager
                .persistence_manager
                .instance_download_lock
                .acquire()
                .await
                .expect("Semaphore should not be closed");

            // Now that we have the lock, transition from Queued to Preparing
            {
                let instance_manager_ref = app.instance_manager();
                let mut instances = instance_manager_ref.instances.write().await;
                if let Some(instance) = instances.get_mut(&instance_id) {
                    if let InstanceType::Valid(data) = &mut instance.type_ {
                        data.state = LaunchState::Preparing(id);
                    }
                }
            }
            app.invalidate(GET_GROUPS, None);
            app.invalidate(GET_ALL_INSTANCES, None);
            app.invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

            let try_result: anyhow::Result<_> = async {
                let mut downloads = Vec::new();

                let (t_subtasks, modpack_version, repair_options) = modpack::process_modpack(
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
                    instance_id,
                    instance_shortpath.clone(),
                    &t_subtasks,
                    repair_options,
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
                    log.as_ref(),
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
                    log.as_ref(),
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
                            let mut split = split_hook_command(&pre_launch_hook)
                                .ok_or_else(|| anyhow::anyhow!("Failed to parse pre-launch hook"))?
                                .into_iter();

                            let main_command = split
                                .next()
                                .ok_or_else(|| anyhow::anyhow!("Pre-launch hook is empty"))?;

                            let pre_launch_command = tokio::time::timeout(
                                HOOK_TIMEOUT,
                                tokio::process::Command::new(main_command)
                                    .args(split)
                                    .current_dir(instance_path.get_data_path())
                                    .kill_on_drop(true)
                                    .output(),
                            )
                            .await
                            .map_err(|_| {
                                anyhow::anyhow!(
                                    "Pre-launch hook did not finish within {HOOK_TIMEOUT:?}"
                                )
                            })?
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

            // Downloading, installing and spawning the process are done; release the global
            // download permit before the game-session wait so other instances can be prepared
            // and launched concurrently instead of queuing behind an already-running game.
            drop(download_guard);

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

                    let Some(process_id) = child.id() else {
                        // Process exited before we could capture its PID.
                        // Surface as a launch error rather than panicking the
                        // task (which would leave the instance stuck in
                        // Preparing).
                        tracing::error!(
                            "Process exited before PID could be captured (instance {})",
                            *instance_id
                        );
                        let _ = app
                            .instance_manager()
                            .change_launch_state(
                                instance_id,
                                LaunchState::Inactive {
                                    failed_task: Some(id),
                                },
                            )
                            .await;
                        return;
                    };

                    // Record the pid so a future `InstanceManager::scan_instances`
                    // pass can find this game JVM again and adopt it, if the
                    // core exits without going through the kill_rx/child.wait()
                    // path below — the user closing the launcher, a crash, a
                    // force-quit, Windows TerminateProcess. Best-effort: never
                    // blocks or fails the launch on write failure.
                    orphan_pid::write_pid_file(
                        &instance_path.get_root(),
                        super::PID_FILE_NAME,
                        process_id,
                    )
                    .await;

                    let Some(running_log_id) = log_id else {
                        tracing::error!("log_id missing when launching instance {}", *instance_id);
                        let _ = app
                            .instance_manager()
                            .change_launch_state(
                                instance_id,
                                LaunchState::Inactive {
                                    failed_task: Some(id),
                                },
                            )
                            .await;
                        return;
                    };

                    let _ = app
                        .instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Running(RunningInstance {
                                process_id,
                                kill_tx: Some(kill_tx),
                                start_time,
                                log: Some(running_log_id),
                                // This core owns the process, so the loop
                                // below keeps its own `last_stored_time` and
                                // banks against a real `child.wait()`.
                                playtime: None,
                            }),
                        )
                        .await;

                    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take())
                    else {
                        tracing::error!(
                            "Failed to capture stdout/stderr from child process for instance {}",
                            *instance_id
                        );
                        let _ = app
                            .instance_manager()
                            .change_launch_state(
                                instance_id,
                                LaunchState::Inactive {
                                    failed_task: Some(id),
                                },
                            )
                            .await;
                        return;
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
                            tracing::debug!("Instance waited");
                        },
                        _ = kill_rx.recv() => {
                            tracing::debug!("Instance killed");
                            if let Err(e) = child.kill().await {
                                tracing::warn!(
                                    "Failed to kill child process for instance {}: {}",
                                    *instance_id,
                                    e
                                );
                            }
                        },
                        _ = read_logs(log.as_ref().expect("log must exist when launching game"), stdout, stderr, file.as_mut()) => {
                            tracing::debug!("Instance read logs");
                        },
                        _ = update_playtime => {
                            tracing::debug!("Instance updated playtime");
                        }
                    }

                    tracing::debug!("Instance exited");

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

                        if let Some(file) = file.as_mut() {
                            if let Some(log) = log.as_ref() {
                                // TODO: not sure how to handle an error in here
                                log.send_modify(|log| {
                                    log.add_entry(LogEntry::system_message(msg.clone()))
                                });
                            }
                            let _ = file
                                .write_all(format_message_as_log4j_event(&msg).as_bytes())
                                .await;
                        }
                    }

                    // The game process has exited by this point regardless of
                    // whether the select above resolved via a natural exit or a
                    // kill signal — `child.wait()` above blocks until it has.
                    // The pidfile no longer refers to anything a future startup
                    // reap needs to clean up. Best-effort, removed exactly once
                    // here on both paths.
                    orphan_pid::remove_pid_file(&instance_path.get_root(), super::PID_FILE_NAME)
                        .await;

                    let _ = app.rich_presence_manager().stop_activity().await;

                    if let Some(post_exit_hook) = post_exit_hook.filter(|v| !v.is_empty()) {
                        match split_hook_command(&post_exit_hook)
                            .ok_or_else(|| anyhow::anyhow!("Failed to parse post-exit hook"))
                            .map(|v| v.into_iter())
                        {
                            Ok(mut split) => match split.next() {
                                Some(main_command) => {
                                    let post_exit_command = tokio::time::timeout(
                                        HOOK_TIMEOUT,
                                        tokio::process::Command::new(main_command)
                                            .args(split)
                                            .current_dir(instance_path.get_data_path())
                                            .kill_on_drop(true)
                                            .output(),
                                    )
                                    .await;

                                    match post_exit_command {
                                        Err(_) => {
                                            tracing::warn!(
                                                "Post-exit hook did not finish within {HOOK_TIMEOUT:?}; killed it"
                                            );
                                        }
                                        Ok(Ok(post_exit_command)) => {
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
                                        Ok(Err(e)) => {
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

            // Drop the log sender so the receiver sees the channel as closed
            // This must happen BEFORE invalidation so the frontend sees active: false
            drop(log);

            // Flush and close the log file before invalidation so file size is accurate
            if let Some(mut f) = file.take() {
                let _ = f.flush().await;
                drop(f);
            }

            app.invalidate(GET_LOGS, Some(instance_id.0.into()));

            let now = Utc::now();
            let offset_in_sec = Local::now().offset().local_minus_utc();

            let mods = app
                .instance_manager()
                .list_mods(instance_id, None)
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
            LaunchState::Queued(_) | LaunchState::Preparing(_) | LaunchState::Deleting => (),
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

        match &running.kill_tx {
            // Owned by this core: the run task holds the child handle, kills
            // it, and does all the bookkeeping afterwards (pidfile removal,
            // playtime, exit code, post-exit hook, transition to Inactive).
            Some(kill_tx) => {
                kill_tx.send(()).await?;
            }
            // Adopted: there is no run task and no handle, so the pid is
            // signalled directly and everything that task would have done has
            // to happen here instead. Narrowed to "still a live java process"
            // first — the same guard the server manager's kill relies on, and
            // the only thing standing between this and a pid that has been
            // reused by something unrelated.
            None => {
                let process_id = running.process_id;
                let shortpath = instance.shortpath.clone();
                drop(instances);

                let pid = Pid::from_u32(process_id);
                let mut system = System::new();
                system.refresh_processes(ProcessesToUpdate::Some(&[pid]));

                if orphan_pid::is_live_java_process(&system, process_id) {
                    if let Some(process) = system.process(pid) {
                        if !process.kill() {
                            bail!("failed to signal adopted game process {process_id}");
                        }
                    }
                    // The game was alive right up to the signal above, so this
                    // last interval was genuinely played — the one case where
                    // an adopted session's final seconds can be banked exactly,
                    // rather than dropped as the poller has to drop them.
                    self.observe_adopted_alive(instance_id, Utc::now()).await;
                    self.bank_adopted_playtime(instance_id).await;
                } else {
                    // Already gone, or the number now belongs to something
                    // else. Either way there is nothing to stop and nothing to
                    // bank — it stopped playing at an unknown earlier moment —
                    // and the instance still has to come back to Inactive: the
                    // poller would eventually do it, but not before the user
                    // has been told their click did nothing.
                    warn!(
                        "adopted game process {process_id} for instance {instance_id} was already gone"
                    );
                }

                self.release_adopted_instance(instance_id, &shortpath).await;
            }
        }

        Ok(())
    }

    /// Return an adopted instance to Inactive: remove the pidfile that was the
    /// only record of its process, and tell the frontend.
    ///
    /// Deliberately not through `change_launch_state`, which prints
    /// `_INSTANCE_STATE_:GAME_CLOSED` for Electron to act on. An adopted
    /// session never printed the matching `GAME_LAUNCHED` — doing so at
    /// startup would fire the user's launcher-action-on-game-launch (hiding
    /// or closing the window they just opened) for a game they launched
    /// earlier — so it must not print the close either, or the two go out of
    /// step.
    pub async fn release_adopted_instance(self, instance_id: InstanceId, shortpath: &str) {
        let root = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(shortpath)
            .get_root();
        orphan_pid::remove_pid_file(&root, super::PID_FILE_NAME).await;

        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.get_mut(&instance_id) {
            if let Ok(data) = instance.data_mut() {
                data.state = LaunchState::Inactive { failed_task: None };
            }
        }
        drop(instances);

        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));
    }

    /// Record that an adopted instance's process was seen alive at `now`, and
    /// write its accrued playtime out if enough has built up to be worth a
    /// write.
    ///
    /// The two halves are separate on purpose. Liveness is observed every
    /// `ADOPTED_POLL_INTERVAL` so the UI reacts quickly to a game closing;
    /// writing is far rarer, because each write rewrites `instance.json` and
    /// invalidates the instance's frontend queries, and doing that every few
    /// seconds for the length of a play session is churn with nothing to show
    /// for it. Nothing is lost by writing late — `last_alive_at` remembers how
    /// far the session got, and the release path banks up to it.
    async fn observe_adopted_alive(self, instance_id: InstanceId, now: DateTime<Utc>) {
        let seconds = {
            let mut instances = self.instances.write().await;
            let Some(playtime) = adopted_playtime_mut(&mut instances, instance_id) else {
                return;
            };

            playtime.last_alive_at = now;

            if (now - playtime.banked_at).num_seconds() < ADOPTED_PLAYTIME_BANK_SECS {
                return;
            }

            let seconds = adopted_playtime_secs(playtime.banked_at, now);
            playtime.banked_at = now;
            seconds
        };

        self.write_adopted_playtime(instance_id, seconds).await;
    }

    /// Write out everything an adopted instance accrued up to the last moment
    /// its process was seen alive, for a session that is ending: the poller
    /// having found the process gone, or Stop having just signalled it.
    ///
    /// Banks to `last_alive_at`, never to now. For Stop those are the same
    /// instant (it confirms the process alive immediately before signalling
    /// it); for a game that exited on its own they differ by up to one poll
    /// interval, and that interval is dropped rather than credited.
    async fn bank_adopted_playtime(self, instance_id: InstanceId) {
        let seconds = {
            let mut instances = self.instances.write().await;
            let Some(playtime) = adopted_playtime_mut(&mut instances, instance_id) else {
                return;
            };

            let up_to = playtime.last_alive_at;
            let seconds = adopted_playtime_secs(playtime.banked_at, up_to);
            playtime.banked_at = up_to;
            seconds
        };

        self.write_adopted_playtime(instance_id, seconds).await;
    }

    /// The slow half of banking, deliberately outside the instance lock:
    /// `update_playtime` takes that same lock itself, and it is what rewrites
    /// `instance.json`.
    async fn write_adopted_playtime(self, instance_id: InstanceId, seconds: u32) {
        if seconds == 0 {
            return;
        }

        if let Err(e) = self.update_playtime(instance_id, seconds).await {
            warn!("error banking playtime for adopted instance {instance_id}: {e:?}");
        }
    }

    /// Watch adopted instances until every one of them has exited.
    ///
    /// An adopted game has no `child.wait()` behind it, so without this its
    /// instance would read Running forever — including long after the user
    /// quit Minecraft normally, which is strictly worse than showing it
    /// Inactive. Polling is the only option available: the process belongs to
    /// no parent this core can wait on.
    ///
    /// Runs only while at least one adopted instance is left, and stops on its
    /// own once none are — a normally-launched game is watched by its own run
    /// task and never needs this.
    pub fn watch_adopted_instances(self, instance_ids: Vec<InstanceId>) {
        let app = self.app.clone();
        let liveness = self.instance_running_tracker.marker();

        tokio::spawn(async move {
            // Moved in rather than dropped when this function returns: it has
            // to live exactly as long as the loop below, which is exactly as
            // long as some adopted instance is still running. Without it,
            // work that backs off during a game — mod-metadata caching, off
            // `any_instance_running` — would keep running through an adopted
            // session, competing with the game the user is actually playing.
            // A normally-launched game holds the equivalent marker in its own
            // run task.
            let _liveness = liveness;
            let mut remaining = instance_ids;

            loop {
                tokio::time::sleep(ADOPTED_POLL_INTERVAL).await;

                // Re-read the map each tick rather than trusting the list:
                // the user may have stopped one through `kill_instance` in
                // the meantime, and it is no longer adopted (or no longer
                // running) if so.
                let still_adopted: Vec<(InstanceId, u32, String)> = {
                    let manager = app.instance_manager();
                    let instances = manager.instances.read().await;
                    remaining
                        .iter()
                        .filter_map(|id| {
                            let instance = instances.get(id)?;
                            let InstanceType::Valid(data) = &instance.type_ else {
                                return None;
                            };
                            let LaunchState::Running(running) = &data.state else {
                                return None;
                            };
                            running
                                .is_adopted()
                                .then(|| (*id, running.process_id, instance.shortpath.clone()))
                        })
                        .collect()
                };

                if still_adopted.is_empty() {
                    break;
                }

                let pids: Vec<Pid> = still_adopted
                    .iter()
                    .map(|(_, pid, _)| Pid::from_u32(*pid))
                    .collect();
                let mut system = System::new();
                system.refresh_processes(ProcessesToUpdate::Some(&pids));

                let observed_at = Utc::now();

                remaining.clear();
                for (instance_id, pid, shortpath) in still_adopted {
                    if should_release_adopted(orphan_pid::is_live_java_process(&system, pid)) {
                        info!(
                            "adopted game process {pid} for instance {instance_id} has exited; instance is inactive again"
                        );
                        // Banked up to the last tick that saw it alive, not to
                        // now: it died at some unknowable point in between.
                        // Must run before the release, which drops the
                        // bookkeeping along with the rest of the running state.
                        app.instance_manager()
                            .bank_adopted_playtime(instance_id)
                            .await;
                        app.instance_manager()
                            .release_adopted_instance(instance_id, &shortpath)
                            .await;
                    } else {
                        app.instance_manager()
                            .observe_adopted_alive(instance_id, observed_at)
                            .await;
                        remaining.push(instance_id);
                    }
                }
            }
        });
    }
}

/// How often adopted instances are checked for still being alive, and so how
/// long the Running badge can lag a game the user has just quit. Kept short:
/// the cost is a targeted `/proc` read per adopted instance, and a stale badge
/// is the one thing about an adopted session a user actually watches.
///
/// Deliberately not also the write cadence — see `ADOPTED_PLAYTIME_BANK_SECS`.
const ADOPTED_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// How much playtime an adopted instance accrues before it is written out,
/// matching the launch task's own 60s tick so an adopted session is no
/// chattier on disk than a launched one.
///
/// Separate from `ADOPTED_POLL_INTERVAL` because each write rewrites
/// `instance.json` and invalidates the instance's frontend queries; doing that
/// at the polling rate would be twelve rewrites a minute, for hours, to record
/// something nothing reads until the session ends.
const ADOPTED_PLAYTIME_BANK_SECS: i64 = 60;

/// The playtime bookkeeping of an adopted running instance, or `None` for
/// anything else — a missing instance, an invalid one, one that is not
/// running, or one this core launched itself (whose launch task does its own
/// accounting).
fn adopted_playtime_mut(
    instances: &mut HashMap<InstanceId, Instance>,
    instance_id: InstanceId,
) -> Option<&mut AdoptedPlaytime> {
    let instance = instances.get_mut(&instance_id)?;
    let InstanceType::Valid(data) = &mut instance.type_ else {
        return None;
    };
    let LaunchState::Running(running) = &mut data.state else {
        return None;
    };
    running.playtime.as_mut()
}

/// Whether an adopted instance should go back to Inactive, given whether its
/// recorded pid is still a live java process.
///
/// Split out from the polling loop for the same reason `reconcile_pid` is
/// split out from the sysinfo lookup: the decision is then testable without a
/// real process table behind it.
pub fn should_release_adopted(is_live_java: bool) -> bool {
    !is_live_java
}

/// Whole seconds of playtime to bank for an adopted instance last banked at
/// `banked_at` and observed still alive at `now`.
///
/// Clamped rather than cast straight to `u32`. A system clock that steps
/// backwards mid-session — an NTP correction, a user changing the date, a
/// laptop resuming with a stale RTC — makes the difference negative, and
/// `as u32` would turn a few seconds of drift into billions of seconds of
/// playtime that nobody played. The upper clamp is unreachable in practice
/// and is there so the cast is total rather than conditionally correct.
pub fn adopted_playtime_secs(banked_at: DateTime<Utc>, now: DateTime<Utc>) -> u32 {
    (now - banked_at).num_seconds().clamp(0, u32::MAX as i64) as u32
}

impl InstanceManager {
    /// Best-effort kill of every currently running game process, meant for
    /// the core process itself being terminated (SIGTERM/SIGINT/Ctrl+C) so
    /// games get a kill signal instead of being silently orphaned. Bounded
    /// to `SHUTDOWN_TIMEOUT` for the whole operation — including a stalled
    /// kill — so a caller awaiting this can never hang past that; a timeout
    /// here is only logged. It does not change what the caller does next
    /// (main.rs exits the process either way, relying on the pidfile-based
    /// reap in `scan_instances` on next launch as the fallback for whatever
    /// didn't get killed in time). Mirrors `ServerManager::shutdown_running`.
    ///
    /// Fire-and-forget: this only sends the kill signal on `kill_tx` and does
    /// not wait for the game process to actually exit before returning — the
    /// pidfile reap on the next launch is the safety net for anything left
    /// alive past this call (or past `SHUTDOWN_TIMEOUT`).
    pub async fn shutdown_running(&self) {
        const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

        let outcome = tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
            let instances = self.instances.read().await;

            let kills = instances.values().filter_map(|instance| {
                let InstanceType::Valid(data) = &instance.type_ else {
                    return None;
                };
                match &data.state {
                    // Adopted instances have no channel and are skipped: this
                    // stops what this core started, and it did not start them.
                    LaunchState::Running(running) => running
                        .kill_tx
                        .as_ref()
                        .map(|kill_tx| (instance.shortpath.as_str(), kill_tx)),
                    _ => None,
                }
            });

            futures::future::join_all(kills.map(|(shortpath, kill_tx)| async move {
                if let Err(e) = kill_tx.send(()).await {
                    warn!("Failed to signal shutdown to instance {}: {}", shortpath, e);
                }
            }))
            .await;
        })
        .await;

        if outcome.is_err() {
            warn!(
                "shutdown_running did not finish within {:?}; proceeding with core exit anyway",
                SHUTDOWN_TIMEOUT
            );
        }
    }
}

pub enum LaunchState {
    Inactive { failed_task: Option<VisualTaskId> },
    Queued(VisualTaskId),
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
                Self::Queued(_) => "Queued",
                Self::Preparing(_) => "Preparing",
                Self::Running(_) => "Running",
                Self::Deleting => "Deleting",
            }
        )
    }
}

pub struct RunningInstance {
    process_id: u32,
    /// `None` when the game was adopted: there is no spawning task to signal,
    /// so stopping goes through the pid instead (`kill_instance`).
    kill_tx: Option<mpsc::Sender<()>>,
    start_time: DateTime<Utc>,
    /// `None` when the game was adopted: the stdout pipe belonged to the core
    /// that spawned it, and died with that process. Nothing can re-attach to
    /// it, so an adopted session has no log at all rather than an empty one.
    log: Option<GameLogId>,
    /// `None` when this core owns the process — the launch task keeps the
    /// equivalent (`last_stored_time`) in its own stack frame, where it also
    /// has a `child.wait()` to bank the final, exact interval against. An
    /// adopted session has neither, so its bookkeeping has to live here, where
    /// both the poller and Stop can reach it.
    playtime: Option<AdoptedPlaytime>,
}

/// Playtime bookkeeping for a session this core adopted.
///
/// Two instants rather than one, because the two things they answer happen at
/// different rates: the process is *watched* often enough for the UI to feel
/// responsive, and *written* rarely enough not to rewrite `instance.json`
/// every few seconds for the length of a play session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdoptedPlaytime {
    /// The last instant the process was seen alive. This — never "now" — is
    /// how far a session that ended between two polls may be banked: it died
    /// at some unknowable moment after this, and crediting past it would
    /// invent playtime nobody played.
    last_alive_at: DateTime<Utc>,
    /// The instant up to which playtime has already been written to
    /// `instance.json`. Always <= `last_alive_at`.
    banked_at: DateTime<Utc>,
}

impl RunningInstance {
    /// A game found already running at startup, belonging to a session this
    /// core did not spawn (`InstanceManager::reconcile_running_instances`).
    ///
    /// Everything that needs a handle to the child process is absent by
    /// construction rather than left to a caller to remember: no channel to
    /// signal, no log, and — since only `child.wait()` ever produced them —
    /// no exit code and no post-exit hook either. Stop is the one action such
    /// an instance still offers.
    ///
    /// `now` is when the adoption happened, and playtime is banked only from
    /// there. Deliberately not `start_time`: the previous core recorded
    /// playtime up to its own last tick before it died, and banking from the
    /// launch would count all of that a second time. Everything between that
    /// tick and this moment is lost instead — the launcher was not running to
    /// observe it, and undercounting is the side to err on.
    pub fn adopted(process_id: u32, start_time: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        Self {
            process_id,
            kill_tx: None,
            start_time,
            log: None,
            playtime: Some(AdoptedPlaytime {
                last_alive_at: now,
                banked_at: now,
            }),
        }
    }

    /// Whether this core owns the process behind the instance. `false` means
    /// it was adopted from a previous session.
    pub fn is_adopted(&self) -> bool {
        self.kill_tx.is_none()
    }

    pub fn process_id(&self) -> u32 {
        self.process_id
    }
}

impl From<&LaunchState> for domain::LaunchState {
    fn from(value: &LaunchState) -> Self {
        match value {
            LaunchState::Inactive { failed_task } => Self::Inactive {
                failed_task: failed_task.clone(),
            },
            LaunchState::Queued(t) => Self::Queued(*t),
            LaunchState::Preparing(t) => Self::Preparing(*t),
            LaunchState::Running(running) => Self::Running {
                start_time: running.start_time,
                log_id: running.log,
                // Reported explicitly rather than left for the frontend to
                // infer from a missing log: "adopted" is the reason, and a
                // caller that had to guess it from `log_id == None` would be
                // wrong the moment any other cause of a missing log appears.
                adopted: running.is_adopted(),
            },
            LaunchState::Deleting => Self::Deleting,
        }
    }
}

async fn read_logs(
    log: &watch::Sender<GameLog>,
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
    log: &watch::Sender<GameLog>,
    mut stdout_rx: mpsc::Receiver<Vec<u8>>,
    mut stderr_rx: mpsc::Receiver<Vec<u8>>,
    file: Option<&mut File>,
) {
    let mut file = match file {
        Some(file) => {
            let already_written = file.metadata().await.map(|m| m.len()).unwrap_or(0);
            Some(CappedLogFile::new(file, already_written))
        }
        None => None,
    };

    let mut stdout_processor = LogProcessor::new(LogEntrySourceKind::StdOut, log).await;

    let mut stderr_processor = LogProcessor::new(LogEntrySourceKind::StdErr, log).await;

    loop {
        tokio::select! {
            Some(data) = stdout_rx.recv() => {
                if let Err(e) = stdout_processor.process_data(&data, file.as_mut()).await {
                    tracing::error!("Failed to process stdout data: {}", e);
                }
            }
            Some(data) = stderr_rx.recv() => {
                if let Err(e) = stderr_processor.process_data(&data, file.as_mut()).await {
                    tracing::error!("Failed to process stderr data: {}", e);
                }
            }
            else => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::instance::{Instance, InstanceData};

    fn dummy_config() -> info::Instance {
        info::Instance {
            name: "test".to_string(),
            icon: info::InstanceIcon::Default,
            date_created: Utc::now(),
            date_updated: Utc::now(),
            last_played: None,
            seconds_played: 0,
            modpack: None,
            game_configuration: info::GameConfig {
                version: None,
                global_java_args: true,
                extra_java_args: None,
                memory: None,
                java_override: None,
                game_resolution: None,
            },
            pre_launch_hook: None,
            post_exit_hook: None,
            wrapper_command: None,
            mod_sources: None,
            notes: String::new(),
        }
    }

    fn instance_data_with_state(state: LaunchState) -> InstanceData {
        InstanceData {
            favorite: false,
            config: dummy_config(),
            state,
            modpack_update_curseforge: None,
            modpack_update_modrinth: None,
            icon_revision: None,
        }
    }

    // --- shutdown_running ---------------------------------------------

    #[tokio::test]
    async fn shutdown_running_kills_a_running_instances_handle() {
        let manager = InstanceManager::new();

        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

        manager.instances.write().await.insert(
            InstanceId(1),
            Instance {
                shortpath: "test-instance".to_string(),
                type_: InstanceType::Valid(instance_data_with_state(LaunchState::Running(
                    RunningInstance {
                        process_id: 4242,
                        kill_tx: Some(kill_tx),
                        start_time: Utc::now(),
                        log: Some(GameLogId(1)),
                        playtime: None,
                    },
                ))),
            },
        );

        manager.shutdown_running().await;

        // `shutdown_running` sends on the instance's `kill_tx` — receiving on
        // it confirms it found the running instance and drove it through the
        // same kill path `kill_instance` uses.
        assert!(
            kill_rx.try_recv().is_ok(),
            "expected shutdown_running to send a kill signal to the running instance"
        );
    }

    #[tokio::test]
    async fn shutdown_running_ignores_inactive_instances_and_returns_promptly() {
        let manager = InstanceManager::new();

        manager.instances.write().await.insert(
            InstanceId(1),
            Instance {
                shortpath: "test-instance".to_string(),
                type_: InstanceType::Valid(instance_data_with_state(LaunchState::Inactive {
                    failed_task: None,
                })),
            },
        );

        // Must return promptly (well within the 3s bound) when nothing is
        // running, and must not panic on a non-`Running` state.
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.shutdown_running(),
        )
        .await
        .expect("shutdown_running must not hang when no instance is running");
    }

    #[tokio::test]
    async fn shutdown_running_with_no_instances_at_all_returns_promptly() {
        let manager = InstanceManager::new();

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.shutdown_running(),
        )
        .await
        .expect("shutdown_running must not hang with an empty instance map");
    }

    #[tokio::test]
    async fn shutdown_running_leaves_an_adopted_instance_alone() {
        let manager = InstanceManager::new();

        manager.instances.write().await.insert(
            InstanceId(1),
            Instance {
                shortpath: "adopted-instance".to_string(),
                type_: InstanceType::Valid(instance_data_with_state(LaunchState::Running(
                    RunningInstance::adopted(4242, Utc::now(), Utc::now()),
                ))),
            },
        );

        // Nothing to send on, and nothing that should be sent: this is the
        // "stop what this core started" call, and an adopted game was started
        // by a session that has already ended. Reaching for `kill_tx`
        // unconditionally would panic instead of returning.
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.shutdown_running(),
        )
        .await
        .expect("shutdown_running must not hang on an adopted instance");

        let instances = manager.instances.read().await;
        let InstanceType::Valid(data) = &instances[&InstanceId(1)].type_ else {
            panic!("instance became invalid")
        };
        assert!(
            matches!(&data.state, LaunchState::Running(r) if r.is_adopted()),
            "an adopted instance must still be running after shutdown_running"
        );
    }

    // --- adopted liveness ------------------------------------------------

    #[test]
    fn adopted_instance_is_released_once_its_pid_stops_being_a_live_jvm() {
        assert!(
            should_release_adopted(false),
            "a pid that is gone (or no longer java) must release the instance back to Inactive"
        );
        assert!(
            !should_release_adopted(true),
            "a pid that is still a live JVM must keep the instance Running"
        );
    }

    // --- adopted playtime -------------------------------------------------

    #[test]
    fn adopted_playtime_counts_whole_seconds_since_the_last_bank() {
        let banked_at = Utc::now();

        assert_eq!(
            adopted_playtime_secs(banked_at, banked_at + chrono::Duration::seconds(30)),
            30
        );
        // Sub-second remainders are dropped, not rounded up: the bookmark
        // moves to `now` either way, so the fraction is picked up by the next
        // interval rather than lost.
        assert_eq!(
            adopted_playtime_secs(banked_at, banked_at + chrono::Duration::milliseconds(1_500)),
            1
        );
        assert_eq!(adopted_playtime_secs(banked_at, banked_at), 0);
    }

    #[test]
    fn adopted_playtime_never_invents_time_when_the_clock_steps_backwards() {
        let banked_at = Utc::now();

        // An NTP correction or a manual date change mid-session. A bare
        // `as u32` here would credit ~4.3 billion seconds of playtime.
        assert_eq!(
            adopted_playtime_secs(banked_at, banked_at - chrono::Duration::seconds(5)),
            0
        );
        assert_eq!(
            adopted_playtime_secs(banked_at, banked_at - chrono::Duration::days(365)),
            0
        );
    }

    #[test]
    fn only_an_adopted_instance_carries_playtime_bookkeeping() {
        // The launch task keeps its own `last_stored_time` and banks against a
        // real `child.wait()`, so an owned instance must not also be banked
        // through the adopted path — that would double-count every interval.
        let (kill_tx, _kill_rx) = mpsc::channel::<()>(1);
        let owned = RunningInstance {
            process_id: 4242,
            kill_tx: Some(kill_tx),
            start_time: Utc::now(),
            log: Some(GameLogId(1)),
            playtime: None,
        };
        assert!(owned.playtime.is_none());

        // An adopted one starts both clocks at adoption, never at launch: the
        // previous core already recorded up to its own last tick.
        let launched_at = Utc::now() - chrono::Duration::hours(2);
        let adopted_at = Utc::now();
        let adopted = RunningInstance::adopted(4242, launched_at, adopted_at);
        assert_eq!(
            adopted.playtime,
            Some(AdoptedPlaytime {
                last_alive_at: adopted_at,
                banked_at: adopted_at
            })
        );
        assert_eq!(adopted.start_time, launched_at);
    }

    #[test]
    fn adopted_playtime_is_written_far_less_often_than_liveness_is_polled() {
        // The two rates are independent on purpose: the badge has to react
        // quickly, `instance.json` does not have to be rewritten at that rate.
        // If these ever converge, a play session starts costing a file rewrite
        // and a query invalidation every few seconds for hours.
        assert!(
            ADOPTED_PLAYTIME_BANK_SECS > ADOPTED_POLL_INTERVAL.as_secs() as i64,
            "banking must be rarer than polling"
        );
        // And a lagging badge is what a user actually notices, so the poll has
        // to stay comfortably sub-second-scale-perceptible.
        assert!(
            ADOPTED_POLL_INTERVAL <= Duration::from_secs(5),
            "an adopted instance must not look Running for long after its game exits"
        );
    }
}
