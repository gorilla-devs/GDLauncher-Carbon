use anyhow::bail;
use thiserror::Error;

use crate::domain::instance::info::ModLoaderType;
use crate::{domain::vtask::VisualTaskId, managers::ManagerRef};

use crate::db::{mod_file_cache as fcdb, mod_metadata as metadb};
use crate::{db::read_filters::IntFilter, domain::instance as domain};

use super::{
    installer::{CurseforgeModInstaller, IntoInstaller, ModrinthModInstaller},
    InstanceId, InstanceManager, InvalidInstanceIdError,
};

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
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::curseforge::fetch())
                    .with(metadb::modrinth::fetch()),
            )
            .exec()
            .await?
            .into_iter()
            .map(|m| domain::Mod {
                id: m.id,
                filename: m.filename,
                enabled: m.enabled,
                metadata: m.metadata.as_ref().and_then(|m| {
                    m.modid.clone().map(|modid| domain::ModFileMetadata {
                        modid,
                        name: m.name.clone(),
                        version: m.version.clone(),
                        description: m.description.clone(),
                        authors: m.authors.clone(),
                        modloaders: m
                            .modloaders
                            .split(',')
                            // ignore unknown modloaders
                            .flat_map(|loader| ModLoaderType::try_from(loader).ok())
                            .collect::<Vec<_>>(),
                    })
                }),
                curseforge: m
                    .metadata
                    .clone()
                    .and_then(|m| m.curseforge)
                    .flatten()
                    .map(|m| domain::CurseForgeModMetadata {
                        project_id: m.project_id as u32,
                        file_id: m.file_id as u32,
                        name: m.name,
                        urlslug: m.urlslug,
                        summary: m.summary,
                        authors: m.authors,
                    }),
                modrinth: m.metadata.and_then(|m| m.modrinth).flatten().map(|m| {
                    domain::ModrinthModMetadata {
                        project_id: m.project_id,
                        version_id: m.version_id,
                        title: m.title,
                        filename: m.filename,
                        urlslug: m.urlslug,
                        description: m.description,
                        authors: m.authors,
                        sha512: m.sha_512,
                        sha1: m.sha_1,
                    }
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
            .ok_or(InvalidModIdError(instance_id, id))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(shortpath)
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
            .ok_or(InvalidModIdError(instance_id, id))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(shortpath)
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
        let installer = CurseforgeModInstaller::create(self.app, project_id, file_id)
            .await?
            .into_installer();

        let task_id = installer.install(self.app, instance_id).await?;

        Ok(task_id)
    }

    pub async fn install_modrinth_mod(
        &self,
        instance_id: InstanceId,
        project_id: String,
        version_id: String,
    ) -> anyhow::Result<VisualTaskId> {
        let installer = ModrinthModInstaller::create(self.app, project_id, version_id)
            .await?
            .into_installer();

        let task_id = installer.install(self.app, instance_id).await?;

        Ok(task_id)
    }
}

#[derive(Error, Debug)]
#[error("invalid mod id '{1}' given for instance '{0}'")]
pub struct InvalidModIdError(InstanceId, String);

// #[cfg(test)]
// mod test {
//     use crate::managers::instance::InstanceVersionSource;
//     use std::collections::HashSet;

//     use crate::{api::keys::instance::INSTANCE_MODS, domain::instance::info};

//     #[tokio::test]
//     async fn test_mod_metadata() -> anyhow::Result<()> {
//         dbg!();
//         let app = crate::setup_managers_for_test().await;
//         let group = app.instance_manager().get_default_group().await?;
//         let instance_id = app
//             .instance_manager()
//             .create_instance(
//                 group,
//                 String::from("test"),
//                 false,
//                 InstanceVersionSource::Version(info::GameVersion::Standard(
//                     info::StandardVersion {
//                         release: String::from("1.16.5"),
//                         modloaders: HashSet::new(),
//                     },
//                 )),
//                 String::new(),
//             )
//             .await?;

//         app.meta_cache_manager()
//             .prioritize_instance(instance_id)
//             .await;

//         app.instance_manager()
//             .install_curseforge_mod(instance_id, 331723, 4022327)
//             .await?;

//         // first invalidation will happen when the mod is scanned locally
//         app.wait_for_invalidation(INSTANCE_MODS).await?;

//         let mods = app.instance_manager().list_mods(instance_id).await?;
//         dbg!(&mods);
//         assert_ne!(mods.get(0), None);

//         // second invalidation will happen when the curseforge metadata is fetched
//         app.wait_for_invalidation(INSTANCE_MODS).await?;

//         let mods = app.instance_manager().list_mods(instance_id).await?;
//         dbg!(&mods);
//         assert_ne!(mods[0].curseforge, None);

//         Ok(())
//     }
// }
