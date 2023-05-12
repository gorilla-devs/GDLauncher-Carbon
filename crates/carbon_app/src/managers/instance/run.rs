use crate::domain::vtask::VisualTaskId;
use daedalus::minecraft::DownloadType;
use std::path::PathBuf;
use std::time::Duration;
use tokio::{io::AsyncReadExt, sync::mpsc};

use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::{self as domain, GameLogId};
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

use super::{Instance, InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};

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
            LaunchState::Inactive => {}
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

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        let version = match config.game_configuration.version {
            GameVersion::Standard(v) => v,
            GameVersion::Custom(_) => panic!("Custom versions are not supported yet"),
        };

        let task = VisualTask::new(match &launch_account {
            Some(_) => Translation::InstanceTaskLaunch(config.name.clone()),
            None => Translation::InstanceTaskPrepare(config.name.clone()),
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

                t_request_version_info.update_items(2, 3);

                match version.modloaders.iter().next() {
                    Some(ModLoader {
                        type_: ModLoaderType::Forge,
                        version: forge_version,
                    }) => {
                        let forge_manifest = app.minecraft_manager().get_forge_manifest().await?;

                        let forge_manifest_version = forge_manifest
                            .game_versions
                            .into_iter()
                            .find(|v| v.id == version.release)
                            .ok_or_else(|| {
                                anyhow!("Could not find forge versions for {}", version.release)
                            })?
                            .loaders
                            .into_iter()
                            .find(|v| &v.id == forge_version)
                            .ok_or_else(|| {
                                anyhow!(
                                    "Could not find forge version {} for minecraft version {}",
                                    forge_version,
                                    version.release
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

                let (progress_watch_tx, mut progress_watch_rx) =
                    tokio::sync::watch::channel(carbon_net::Progress::new());

                // dropped when the sender is dropped
                tokio::spawn(async move {
                    while progress_watch_rx.changed().await.is_ok() {
                        {
                            let progress = progress_watch_rx.borrow();
                            t_download_files.update_download(
                                progress.current_count as u32,
                                progress.current_size as u32,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                });

                let mc_files = app.minecraft_manager()
                    .get_all_vanilla_files(version_info.clone())
                    .await?;

                carbon_net::download_multiple(mc_files, progress_watch_tx)
                    .await?;

                t_extract_natives.start_opaque();
                managers::minecraft::minecraft::extract_natives(&runtime_path, &version_info).await;
                t_extract_natives.complete_opaque();

                let libraries_path = runtime_path.get_libraries();
                let game_version = version_info.id.to_string();
                let client_path = runtime_path.get_versions().get_clients_path().join(format!(
                    "{}.jar",
                    version_info
                        .downloads
                        .get(&DownloadType::Client)
                        .unwrap()
                        .sha1
                ));

                if let Some(t_forge_processors) = t_forge_processors {
                    t_forge_processors.start_opaque();

                    if let Some(processors) = &version_info.processors {
                        managers::minecraft::forge::execute_processors(
                            processors,
                            version_info
                                .data
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Data entries missing"))?,
                            PathBuf::from("java"),
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
                            PathBuf::from("java"),
                            account,
                            xms_memory,
                            xmx_memory,
                            &runtime_path,
                            version_info,
                            instance_path,
                        )
                        .await?,
                    )),
                    None => Ok(None),
                }
            })()
            .await;

            match try_result {
                Err(e) => {
                    task.fail(e).await;

                    let _ = app
                        .instance_manager()
                        .change_launch_state(instance_id, LaunchState::Inactive)
                        .await;
                }
                Ok(None) => {}
                Ok(Some(mut child)) => {
                    drop(task);

                    let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

                    let (log_id, log) = app.instance_manager().create_log().await;
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

                    let _ = app
                        .instance_manager()
                        .change_launch_state(instance_id, LaunchState::Inactive)
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
        let mut instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), .. } = &mut instance else {
            bail!("change_launch_state called on invalid instance")
        };

        data.state = state;

        self.app.invalidate(GET_GROUPS, None);
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

        let Instance { type_: InstanceType::Valid(data), .. } = &instance else {
            bail!("get_launch_state called on invalid instance")
        };

        Ok((&data.state).into())
    }

    pub async fn kill_instance(self, instance_id: InstanceId) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), .. } = &instance else {
            bail!("kill_instance called on invalid instance")
        };

        let LaunchState::Running(running) = &data.state else {
            bail!("kill_instance called on instance that was not running")
        };

        running.kill_tx.send(()).await?;

        Ok(())
    }
}

pub enum LaunchState {
    Inactive,
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
            LaunchState::Inactive => Self::Inactive,
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
        println!("Task exited");
        tokio::time::sleep(std::time::Duration::from_secs(10000)).await;

        Ok(())
    }
}
