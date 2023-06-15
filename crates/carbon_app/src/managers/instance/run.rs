use crate::domain::instance::info::{Modpack, StandardVersion};
use crate::domain::java::SystemJavaProfileName;
use crate::domain::modplatforms::curseforge::filters::ModFileParameters;
use crate::domain::vtask::VisualTaskId;
use crate::managers::minecraft::curseforge::{self, ProgressState};
use crate::managers::minecraft::minecraft::get_lwjgl_meta;
use daedalus::minecraft::DownloadType;
use std::path::PathBuf;

use std::time::Duration;
use tokio::{io::AsyncReadExt, sync::mpsc};

use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::{self as domain, GameLogId, InstanceSettingsUpdate};
use crate::managers::instance::log::EntryType;
use chrono::{DateTime, Utc};
use tokio::sync::Semaphore;

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

impl ManagerRef<'_, InstanceManager> {
    pub async fn prepare_game(
        self,
        instance_id: InstanceId,
        launch_account: Option<FullAccount>,
    ) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &mut instance.type_ else {
            return Err(anyhow!("Instance {instance_id} is not in a valid state"))
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
                .get()
                .await
                .map(|c| (c.xms as u16, c.xmx as u16))?,
        };

        let global_java_args = match config.game_configuration.global_java_args {
            true => self
                .app
                .settings_manager()
                .get()
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

        let mut version = match config.game_configuration.version {
            Some(GameVersion::Standard(v)) => Some(v),
            Some(GameVersion::Custom(_)) => bail!("Custom versions are not supported yet"),
            None if config.modpack.is_some() => None,
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

        let wait_task = task.subtask(Translation::InstanceTaskLaunchWaiting).await;
        wait_task.set_weight(0.0);

        let id = self.app.task_manager().spawn_task(&task).await;

        data.state = LaunchState::Preparing(id);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_INSTANCES_UNGROUPED, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some((*instance_id).into()));

        let app = self.app.clone();
        tokio::spawn(async move {
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
                        task.subtask(Translation::InstanceTaskLaunchRequestModpack)
                            .await,
                        task.subtask(Translation::InstanceTaskLaunchDownloadModpackFiles)
                            .await,
                        task.subtask(Translation::InstanceTaskLaunchExtractModpackFiles)
                            .await,
                        task.subtask(Translation::InstanceTaskLaunchDownloadAddonMetadata)
                            .await,
                    )),
                    false => None,
                };

                let t_request_version_info = task
                    .subtask(Translation::InstanceTaskLaunchRequestVersions)
                    .await;

                let t_download_files = task
                    .subtask(Translation::InstanceTaskLaunchDownloadFiles)
                    .await;
                t_download_files.set_weight(20.0);
                let t_extract_natives = task
                    .subtask(Translation::InstanceTaskLaunchExtractNatives)
                    .await;

                let t_forge_processors = match is_first_run {
                    true => Some(
                        task.subtask(Translation::InstanceTaskLaunchRunForgeProcessors)
                            .await,
                    ),
                    false => None,
                };

                task.edit(|data| data.state = TaskState::KnownProgress)
                    .await;

                wait_task.complete_opaque();

                let mut downloads = Vec::new();

                if let Some((t_request, t_download_files, t_extract_files, t_addon_metadata)) =
                    t_modpack
                {
                    if let Some(modpack) = &config.modpack {
                        let modpack = match modpack {
                            Modpack::Curseforge(modpack) => modpack,
                        };

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
                            tokio::sync::watch::channel(ProgressState::Idle);

                        tokio::spawn(async move {
                            while modpack_progress_rx.changed().await.is_ok() {
                                {
                                    let progress = modpack_progress_rx.borrow();
                                    match *progress {
                                        ProgressState::Idle => {}
                                        ProgressState::DownloadingAddonZip(downloaded, total) => {
                                            t_download_files
                                                .update_download(downloaded as u32, total as u32)
                                        }
                                        ProgressState::ExtractingAddonOverrides(count, total) => {
                                            t_extract_files.update_items(count as u32, total as u32)
                                        }
                                        ProgressState::AcquiringAddonsMetadata(count, total) => {
                                            t_addon_metadata
                                                .update_items(count as u32, total as u32)
                                        }
                                    }
                                }

                                tokio::time::sleep(Duration::from_millis(200)).await;
                            }

                            t_download_files.complete_download();
                        });

                        let modpack_info = curseforge::prepare_modpack(
                            &app,
                            &file,
                            instance_path.clone(),
                            modpack_progress_tx,
                        )
                        .await?;

                        downloads.extend(modpack_info.downloadables);
                        let v: StandardVersion = modpack_info.manifest.minecraft.try_into()?;
                        version = Some(v.clone());

                        app.instance_manager()
                            .update_instance(InstanceSettingsUpdate {
                                instance_id,
                                version: Some(v.release.clone()),
                                modloader: Some(v.modloaders.iter().next().cloned()),
                                name: None,
                                use_loaded_icon: None,
                                notes: None,
                                global_java_args: None,
                                extra_java_args: None,
                                memory: None,
                            })
                            .await?;
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

                    match app.java_manager().get_usable_java(required_java).await? {
                        Some(path) => path,
                        None => {
                            let t_install_java = task
                                .subtask(Translation::InstanceTaskLaunchInstallJava)
                                .await;
                            t_install_java.set_weight(0.0);
                            t_install_java.start_opaque();
                            let path = app
                                .java_manager()
                                .require_java_install(required_java)
                                .await?;
                            t_install_java.complete_opaque();

                            match path {
                                Some(path) => path,
                                None => return Ok(None),
                            }
                        }
                    }
                };

                match version.modloaders.iter().next() {
                    Some(ModLoader {
                        type_: ModLoaderType::Forge,
                        version: forge_version,
                    }) => {
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
                    _ => {}
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
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_files.complete_download();
                });

                carbon_net::download_multiple(downloads, progress_watch_tx).await?;

                // scan instances again offtask to pick up modpack mods
                let app2 = app.clone();
                tokio::spawn(async move {
                    let _ = app2.instance_manager().scan_instances().await;
                });

                t_extract_natives.start_opaque();
                managers::minecraft::minecraft::extract_natives(
                    &runtime_path,
                    &version_info,
                    &lwjgl_group,
                    &java.arch,
                )
                .await?;
                t_extract_natives.complete_opaque();

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

                    let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

                    let (log_id, log) = app.instance_manager().create_log(instance_id).await;
                    let _ = app.instance_manager()
                        .change_launch_state(
                            instance_id,
                            LaunchState::Running(RunningInstance {
                                process_id: child.id().expect("child process id is not present even though child process was started"),
                                kill_tx,
                                start_time: Utc::now(),
                                log: log_id,
                            }),
                        )
                        .await;

                    let (Some(mut stdout), Some(mut stderr)) = (child.stdout.take(), child.stderr.take()) else {
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
                                            log.push(EntryType::StdOut, &*utf8);
                                            false
                                        });

                                        loop {
                                            tokio::select!(biased;
                                                _ = tokio::time::sleep(Duration::from_millis(1)) => break,
                                                r = stdout.read(&mut outbuf) => match r {
                                                    Ok(count) if count > 0 => {
                                                        let utf8 = String::from_utf8_lossy(&outbuf[0..count]);
                                                        log.send_if_modified(|log| {
                                                            log.push(EntryType::StdOut, &*utf8);
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
                                            log.push(EntryType::StdErr, &*utf8);
                                            false
                                        });

                                        loop {
                                            tokio::select!(biased;
                                                _ = tokio::time::sleep(Duration::from_millis(1)) => break,
                                                r = stderr.read(&mut errbuf) => match r {
                                                    Ok(count) if count > 0 => {
                                                        let utf8 = String::from_utf8_lossy(&errbuf[0..count]);
                                                        log.send_if_modified(|log| {
                                                            log.push(EntryType::StdErr, &*utf8);
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

                    tokio::select! {
                        _ = child.wait() => {},
                        _ = kill_rx.recv() => drop(child.kill().await),
                        // canceled by one of the others being selected
                        _ = read_logs => {},
                    }

                    if let Ok(exitcode) = child.wait().await {
                        log.send_modify(|log| log.push(EntryType::System, &exitcode.to_string()));
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
        });

        Ok(())
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

        running.kill_tx.send(()).await?;

        Ok(())
    }
}

pub enum LaunchState {
    Inactive { failed_task: Option<VisualTaskId> },
    Preparing(VisualTaskId),
    Running(RunningInstance),
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
        managers::{account::FullAccount, instance::InstanceVersionSouce},
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
                InstanceVersionSouce::Version(info::GameVersion::Standard(StandardVersion {
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
            .prepare_game(instance_id, Some(account))
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
