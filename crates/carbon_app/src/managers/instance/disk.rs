use std::path::PathBuf;

use carbon_domain::translate;
use chrono::Utc;
use tokio::sync::Semaphore;

use anyhow::anyhow;

use crate::{
    domain::{
        instance::info::{GameVersion, ModLoader, ModLoaderType},
        minecraft,
    },
    managers::{
        self,
        account::{FullAccount, FullAccountType},
        vtask::{VisualTask, VisualTaskId},
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
        launch: bool,
    ) -> anyhow::Result<VisualTaskId> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            return Err(anyhow!("Instance {instance_id} is not in a valid state"))
        };

        let config = data.config.clone();

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        drop(instances);

        let version = match config.game_configuration.version {
            GameVersion::Standard(v) => v,
            GameVersion::Custom(_) => panic!("Custom versions are not supported yet"),
        };

        let task_string = match launch {
            true => "instance.task.launch",
            false => "instance.task.ensure",
        };

        let task = VisualTask::new(translate!([task_string] { "name": config.name.clone() }));

        let wait_task = task
            .subtask(translate!("instance.task.ensure.waiting"))
            .await;
        wait_task.set_weight(0.0);

        let id = self.app.task_manager().spawn_task(&task).await;

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

            let try_result: anyhow::Result<()> = (|| async {
                dbg!();
                let request_game_versions = task
                    .subtask(translate!("instance.task.ensure.requesting_game_versions"))
                    .await;
                dbg!();

                let t_request_version_info = task
                    .subtask(translate!("instance.task.ensure.request_versions"))
                    .await;
                let t_download_files = task
                    .subtask(translate!("instance.task.ensure.download_files"))
                    .await;
                t_download_files.set_weight(10.0);
                let t_extract_natives = task
                    .subtask(translate!("instance.task.ensure.extract_natives"))
                    .await;
                let t_forge_processors = task
                    .subtask(translate!("instance.task.ensure.run_forge_processors"))
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
                            minecraft::modded::merge_partial_version(forge_version, version_info);
                    }
                    _ => {}
                }
                t_request_version_info.update_items(3, 3);

                let (progress_watch_tx, mut progress_watch_rx) =
                    tokio::sync::watch::channel(carbon_net::Progress::new());

                // dropped when the sender is dropped
                tokio::spawn(async move {
                    while progress_watch_rx.changed().await.is_ok() {
                        let progress = progress_watch_rx.borrow();
                        t_download_files.update_download(
                            progress.current_count as u32,
                            progress.current_size as u32,
                        );
                        println!(
                            "Invalidate download: {} / {}",
                            progress.current_count, progress.current_size
                        );
                    }
                });

                app.minecraft_manager()
                    .download_minecraft(version_info.clone(), progress_watch_tx)
                    .await?;

                t_extract_natives.start_opaque();
                managers::minecraft::minecraft::extract_natives(&runtime_path, &version_info).await;
                t_extract_natives.complete_opaque();

                let libraries_path = runtime_path.get_libraries();
                let game_version = version_info
                    .inherits_from
                    .as_ref()
                    .unwrap_or(&version_info.id)
                    .to_string();

                let client_path = runtime_path.get_versions().get_clients_path().join(format!(
                    "{}.jar",
                    version_info.downloads.as_ref().unwrap().client.sha1
                ));

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
                    .await
                    .unwrap();
                }
                t_forge_processors.complete_opaque();

                if launch {
                    let full_account = FullAccount {
                        username: "test".to_owned(),
                        uuid: "test-uuid".to_owned(),
                        type_: FullAccountType::Offline,
                        last_used: Utc::now().into(),
                    };

                    let mut child = managers::minecraft::minecraft::launch_minecraft(
                        PathBuf::from("java"),
                        full_account,
                        2048_u16,
                        2048_u16,
                        &runtime_path,
                        version_info,
                        instance_path,
                    )
                    .await
                    .unwrap();

                    // intercept stdout

                    let _ = child.wait().await;
                    // todo return child process
                }

                Ok(())
            })()
            .await;

            if let Err(e) = try_result {
                task.fail(e).await;
            }
        });

        Ok(id)
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        time::Duration,
    };

    use crate::{
        domain::instance::info::{self, StandardVersion},
        managers::instance::InstanceVersionSouce,
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

        let task = app
            .instance_manager()
            .prepare_game(instance_id, true)
            .await?;

        app.task_manager().wait_with_log(task).await?;
        println!("Task exited");

        Ok(())
    }
}
