use anyhow::bail;
use thiserror::Error;

use crate::db::{
    curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb,
    modrinth_mod_cache as mrdb,
};
use crate::domain::instance as domain;
use crate::domain::instance::info::ModLoaderType;
use crate::{domain::vtask::VisualTaskId, managers::ManagerRef};

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
            .find_many(vec![fcdb::instance_id::equals(*instance_id)])
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::logo_image::fetch())
                    .with(metadb::curseforge::fetch().with(cfdb::logo_image::fetch()))
                    .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
            )
            .exec()
            .await?
            .into_iter()
            .map(|m| domain::Mod {
                id: m.id,
                filename: m.filename,
                enabled: m.enabled,
                metadata: m.metadata.as_ref().map(|m| domain::ModFileMetadata {
                    modid: m.modid.clone(),
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
                    has_image: m
                        .logo_image
                        .as_ref()
                        .map(|v| v.as_ref().map(|_| ()))
                        .flatten()
                        .is_some(),
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
                        has_image: m
                            .logo_image
                            .flatten()
                            .as_ref()
                            .map(|row| row.data.as_ref().map(|_| ()))
                            .flatten()
                            .is_some(),
                    }),
                modrinth: m.metadata.and_then(|m| m.modrinth).flatten().map(|m| {
                    domain::ModrinthModMetadata {
                        project_id: m.project_id,
                        version_id: m.version_id,
                        title: m.title,
                        urlslug: m.urlslug,
                        description: m.description,
                        authors: m.authors,
                        has_image: m
                            .logo_image
                            .flatten()
                            .as_ref()
                            .map(|row| row.data.as_ref().map(|_| ()))
                            .flatten()
                            .is_some(),
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
            .ok_or(InvalidInstanceModIdError(instance_id, id))?;

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
            .queue_caching(instance_id, true)
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
            .ok_or(InvalidInstanceModIdError(instance_id, id))?;

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
            .queue_caching(instance_id, true)
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

    pub async fn get_mod_icon(
        &self,
        instance_id: InstanceId,
        mod_id: String,
        platformid: i32,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let instances = self.instances.read().await;
        let _ = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let r = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(mod_id.clone()))
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::logo_image::fetch())
                    .with(metadb::curseforge::fetch().with(cfdb::logo_image::fetch()))
                    .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
            )
            .exec()
            .await?
            .ok_or(InvalidModIdError(mod_id))?
            .metadata
            .ok_or_else(|| anyhow::anyhow!("broken db state"))?;

        let logo_image = match platformid {
            0 => r
                .logo_image
                .ok_or_else(|| anyhow::anyhow!("broken db state"))?
                .map(|m| m.data),
            1 => r
                .curseforge
                .ok_or_else(|| anyhow::anyhow!("broken db state"))?
                .map(|cf| {
                    cf.logo_image
                        .ok_or_else(|| anyhow::anyhow!("broken db state"))
                })
                .transpose()?
                .flatten()
                .map(|img| img.data)
                .flatten(),
            2 => r
                .modrinth
                .ok_or_else(|| anyhow::anyhow!("broken db state"))?
                .map(|mr| {
                    mr.logo_image
                        .ok_or_else(|| anyhow::anyhow!("broken db state"))
                })
                .transpose()?
                .flatten()
                .map(|img| img.data)
                .flatten(),
            _ => bail!("unsupported platform"),
        };

        Ok(logo_image)
    }
}

#[derive(Error, Debug)]
#[error("invalid mod id '{1}' given for instance '{0}'")]
pub struct InvalidInstanceModIdError(InstanceId, String);

#[derive(Error, Debug)]
#[error("invalid mod id '{0}'")]
pub struct InvalidModIdError(String);

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::managers::instance::InstanceVersionSource;
    use crate::{api::keys::instance::INSTANCE_MODS, domain::instance::info};

    #[tokio::test]
    #[ignore]
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
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.16.5"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        app.meta_cache_manager()
            .cache_with_priority(instance_id)
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
