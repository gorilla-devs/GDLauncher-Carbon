use std::time::Duration;

use anyhow::bail;
use carbon_net::Downloadable;
use thiserror::Error;
use tracing::log::info;

use crate::{
    api::translation::Translation,
    domain::{
        instance::info::ModLoaderType, modplatforms::curseforge::filters::ModFileParameters,
        vtask::VisualTaskId,
    },
    managers::{vtask::VisualTask, ManagerRef},
};

use crate::db::{mod_file_cache as fcdb, mod_metadata as metadb};
use crate::{db::read_filters::IntFilter, domain::instance as domain};

use super::{Instance, InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};

impl ManagerRef<'_, InstanceManager> {
    pub async fn list_mods(self, instance_id: InstanceId) -> anyhow::Result<Vec<domain::Mod>> {
        {
            let instances = self.instances.read().await;
            if instances.get(&instance_id).is_none() {
                bail!(InvalidInstanceIdError(instance_id));
            }
        }

        let mods = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_many(vec![fcdb::WhereParam::InstanceId(IntFilter::Equals(
                *instance_id,
            ))])
            .with(fcdb::metadata::fetch().with(metadb::curseforge::fetch()))
            .exec()
            .await?
            .into_iter()
            .map(|m| domain::Mod {
                id: m.id,
                filename: m.filename,
                enabled: m.enabled,
                metadata: m
                    .metadata
                    .as_ref()
                    .map(|m| match m.modid.clone() {
                        Some(modid) => Some(domain::ModFileMetadata {
                            modid,
                            name: m.name.clone(),
                            version: m.version.clone(),
                            description: m.description.clone(),
                            authors: m.authors.clone(),
                            modloaders: m
                                .modloaders
                                .split(",")
                                // ignore unknown modloaders
                                .flat_map(|loader| ModLoaderType::try_from(loader).ok())
                                .collect::<Vec<_>>(),
                        }),
                        _ => None,
                    })
                    .flatten(),
                curseforge: m
                    .metadata
                    .map(|m| m.curseforge)
                    .flatten()
                    .flatten()
                    .map(|m| domain::CurseForgeModMetadata {
                        project_id: m.project_id as u32,
                        file_id: m.file_id as u32,
                        name: m.name,
                        urlslug: m.urlslug,
                        summary: m.summary,
                        authors: m.authors,
                    }),
            });

        Ok(mods.collect::<Vec<_>>())
    }

    pub async fn enable_mod(
        self,
        instance_id: InstanceId,
        id: String,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let shortpath = &instance.shortpath;

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .exec()
            .await?
            .ok_or_else(|| InvalidModIdError(instance_id, id))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_mods_path();

        let enabled_path = disabled_path.join(&m.filename);

        let mut disabled = m.filename.clone();
        disabled.push_str(".disabled");
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

        self.app
            .meta_cache_manager()
            .queue_local_caching(instance_id, true)
            .await;

        Ok(())
    }

    pub async fn delete_mod(self, instance_id: InstanceId, id: String) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let shortpath = &instance.shortpath;

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .exec()
            .await?
            .ok_or_else(|| InvalidModIdError(instance_id, id))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_mods_path();

        let enabled_path = disabled_path.join(&m.filename);

        let mut disabled = m.filename.clone();
        disabled.push_str(".disabled");
        disabled_path.push(disabled);

        if enabled_path.is_file() {
            tokio::fs::remove_file(enabled_path).await?;
        } else if disabled_path.is_file() {
            tokio::fs::remove_file(disabled_path).await?;
        }

        self.app
            .meta_cache_manager()
            .queue_local_caching(instance_id, true)
            .await;

        Ok(())
    }

    pub async fn install_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
        file_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        info!("downloading mod file for {project_id}/{file_id}");

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

        // TODO: check with fingerprint?
        let is_installed = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::InstanceIdFilenameEquals(
                *instance_id,
                file.file_name.clone(),
            ))
            .exec()
            .await?
            .is_some();

        if is_installed {
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

                // invalidates INSTANCE_MODS
                app.meta_cache_manager()
                    .queue_local_caching(instance_id, true)
                    .await;

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

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::{
        api::keys::instance::INSTANCE_MODS, domain::instance::info,
        managers::instance::InstanceVersionSouce,
    };

    #[tokio::test]
    async fn test_mod_metadata() -> anyhow::Result<()> {
        dbg!();
        let app = crate::setup_managers_for_test().await;
        let group = app.instance_manager().get_default_group().await?;
        let instance_id = app
            .instance_manager()
            .create_instance(
                group,
                String::from("test"),
                false,
                InstanceVersionSouce::Version(info::GameVersion::Standard(info::StandardVersion {
                    release: String::from("1.16.5"),
                    modloaders: HashSet::new(),
                })),
                String::new(),
            )
            .await?;

        app.meta_cache_manager()
            .prioritize_instance(instance_id)
            .await;

        app.instance_manager()
            .install_curseforge_mod(instance_id, 331723, 4022327)
            .await?;

        // first invalidation will happen when the mod is scanned locally
        app.wait_for_invalidation(INSTANCE_MODS).await?;

        let mods = app.instance_manager().list_mods(instance_id).await?;
        dbg!(&mods);
        assert_ne!(mods.get(0), None);

        // second invalidation will happen when the curseforge metadata is fetched
        app.wait_for_invalidation(INSTANCE_MODS).await?;

        let mods = app.instance_manager().list_mods(instance_id).await?;
        dbg!(&mods);
        assert_ne!(mods[0].curseforge, None);

        Ok(())
    }
}
