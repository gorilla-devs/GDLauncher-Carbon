use std::{
    ffi::{OsStr, OsString},
    io::Cursor,
    time::Duration,
};

use anyhow::bail;
use carbon_net::Downloadable;
use md5::{Digest, Md5};
use thiserror::Error;

use crate::domain::instance as domain;
use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::{modplatforms::curseforge::filters::ModFileParameters, vtask::VisualTaskId},
    managers::{metadata, vtask::VisualTask, ManagerRef},
};

use super::{Instance, InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError, Mod};

impl ManagerRef<'_, InstanceManager> {
    pub async fn enable_mod(
        self,
        instance_id: InstanceId,
        id: String,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let mut instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &mut instance else {
            bail!("enable_mod called on invalid instance");
        };

        let m = data
            .mods
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| InvalidModIdError(instance_id, id.clone()))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_mods_path();

        let enabled_path = disabled_path.join(&m.filename);

        let mut disabled = m.filename.clone();
        disabled.push(OsStr::new(".disabled"));
        disabled_path.push(disabled);

        if enabled {
            if enabled_path.exists() {
                bail!("mod is already enabled");
            }

            if !disabled_path.exists() {
                bail!("mod does not exist on disk");
            }

            tokio::fs::rename(disabled_path, enabled_path).await?;
        } else {
            if disabled_path.exists() {
                bail!("mod is already disabled");
            }

            if !enabled_path.exists() {
                bail!("mod does not exist on disk");
            }

            tokio::fs::rename(enabled_path, disabled_path).await?;
        }

        m.enabled = !m.enabled;
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn delete_mod(self, instance_id: InstanceId, id: String) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let mut instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &mut instance else {
            bail!("enable_mod called on invalid instance");
        };

        let (i, m) = data
            .mods
            .iter_mut()
            .enumerate()
            .find(|(_, m)| m.id == id)
            .ok_or_else(|| InvalidModIdError(instance_id, id.clone()))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_mods_path();

        let enabled_path = disabled_path.join(&m.filename);

        let mut disabled = m.filename.clone();
        disabled.push(OsStr::new(".disabled"));
        disabled_path.push(disabled);

        if enabled_path.is_file() {
            tokio::fs::remove_file(enabled_path).await?;
        } else if disabled_path.is_file() {
            tokio::fs::remove_file(disabled_path).await?;
        }

        data.mods.remove(i);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn install_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
        file_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        let file = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_file(ModFileParameters {
                mod_id: project_id as i32,
                file_id: file_id as i32,
            })
            .await?
            .data;

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &instance else {
            bail!("install_mod called on invalid instance");
        };

        // TODO: check with fingerprint once local meta cache is done
        if data
            .mods
            .iter()
            .any(|m| m.filename.to_string_lossy() == file.file_name)
        {
            bail!("mod is already installed");
        }

        let downloadable = Downloadable::new(
            file.download_url.ok_or_else(|| {
                anyhow::anyhow!("mod cannot be downloaded without privileged api key")
            })?,
            self.app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath)
                .get_mods_path()
                .join(&file.file_name),
        );

        let task = VisualTask::new(Translation::InstanceTaskInstallMod {
            mod_name: file.display_name,
            instance_name: data.config.name.clone(),
        });

        let id = self.app.task_manager().spawn_task(&task).await;

        drop(instances);

        let t_download_file = task
            .subtask(Translation::InstanceTaskInstallModDownloadFile)
            .await;

        let app = self.app.clone();
        tokio::spawn(async move {
            let r = (|| async {
                let (progress_watch_tx, mut progress_watch_rx) =
                    tokio::sync::watch::channel(carbon_net::Progress::new());

                // dropped when the sender is dropped
                tokio::spawn(async move {
                    while progress_watch_rx.changed().await.is_ok() {
                        {
                            let progress = progress_watch_rx.borrow();
                            t_download_file.update_download(
                                progress.current_size as u32,
                                progress.total_size as u32,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_file.complete_download();
                });

                carbon_net::download_file(&downloadable, Some(progress_watch_tx)).await?;

                let instance_manager = app.instance_manager();
                let mut instances = instance_manager.instances.write().await;
                let mut instance = instances
                    .get_mut(&instance_id)
                    .ok_or(InvalidInstanceIdError(instance_id))?;

                let Instance { type_: InstanceType::Valid(data), .. } = &mut instance else {
                    bail!("install_mod called on invalid instance");
                };

                let file_data = tokio::fs::read(downloadable.path).await?;
                let md5hash = Md5::new_with_prefix(&file_data).finalize();

                let metadata = tokio::task::spawn_blocking(|| {
                    metadata::mods::parse_metadata(Cursor::new(file_data))
                })
                .await??
                .ok_or_else(|| {
                    anyhow::anyhow!("downloaded curseforge mod did not have any metadata")
                })?;

                let id = hex::encode(md5hash);

                data.mods.push(Mod {
                    id,
                    filename: OsString::from(file.file_name),
                    enabled: true,
                    modloaders: metadata.modloaders.clone().unwrap_or_else(|| vec![domain::info::ModLoaderType::Forge]),
                    metadata,
                });

                app.invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
                Ok::<_, anyhow::Error>(())
            })()
            .await;

            match r {
                Ok(()) => {}
                Err(e) => task.fail(e).await,
            }
        });

        Ok(id)
    }
}

#[derive(Error, Debug)]
#[error("invalid mod id '{1}' given for instance '{0}'")]
pub struct InvalidModIdError(InstanceId, String);
