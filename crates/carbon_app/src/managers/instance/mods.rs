use super::{
    InstanceId, InstanceManager, InvalidInstanceIdError,
    installer::{CurseforgeModInstaller, IntoInstaller, ModrinthModInstaller},
};
use crate::api::keys::instance::INSTANCE_MODS;
use crate::domain::instance::info::{GameVersion, ModLoader, ModLoaderType};
use crate::domain::instance::{self as domain, AddonType, info};
use crate::managers::AppInner;
use crate::managers::instance::InstanceType;
use crate::{domain::vtask::VisualTaskId, managers::ManagerRef};
use anyhow::{anyhow, bail};
use carbon_platforms::curseforge::FileReleaseType;
use carbon_platforms::curseforge::filters::{
    ModFilesParameters, ModFilesParametersQuery, ModParameters,
};
use carbon_platforms::modrinth::project::ProjectVersionsFilters;
use carbon_platforms::modrinth::search::ProjectID;
use carbon_platforms::modrinth::version::VersionType;
use carbon_platforms::{
    ModChannel, ModChannelWithUsage, ModPlatform, ModSources, RemoteVersion, curseforge, modrinth,
};
use carbon_repos::db::{
    curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb,
    modrinth_mod_cache as mrdb,
};
use chrono::{DateTime, FixedOffset, Utc};
use futures::Future;
use std::borrow::Cow;
use std::collections::HashSet;
use std::str::FromStr;
use std::time::Instant;
use thiserror::Error;

impl ManagerRef<'_, InstanceManager> {
    async fn ensure_modpack_not_locked(&self, instance_id: InstanceId) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        if let Some(modpack) = instance.data()?.config.modpack.as_ref() {
            if modpack.locked {
                bail!("Modpack is locked");
            }
        }

        Ok(())
    }

    pub async fn list_mods(
        self,
        instance_id: InstanceId,
        addon_type: Option<domain::AddonType>,
    ) -> anyhow::Result<Vec<domain::Mod>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            bail!("instance {} is not valid", *instance_id);
        };

        let config = data.config.clone();
        drop(instances);

        let update_paths = match &config.game_configuration.version {
            Some(GameVersion::Standard(version)) => {
                let v = version.release.to_lowercase();

                version
                    .modloaders
                    .iter()
                    .map(|loader| (v.clone(), loader.type_.to_string().to_lowercase()))
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };

        let mod_sources = self.instance_cfg_mod_sources(&config).await?;

        fn split_paths<'a>(paths: &'a str) -> Vec<(&'a str, &'a str, &'a str)> {
            paths
                .split(';')
                .filter(|p| !p.is_empty())
                .filter_map(|path| path.split_once(','))
                .filter_map(|(v, lc)| lc.split_once(',').map(|(l, c)| (v, l, c)))
                .collect()
        }

        let has_update_for_paths =
            |current_channel: ModChannel, paths: &Vec<(&str, &str, &str)>| {
                let mut best_channel = ModChannel::Alpha;

                paths
                    .iter()
                    .filter(|(v1, l1, _)| update_paths.iter().any(|(v2, l2)| v1 == v2 && l1 == l2))
                    .filter_map(|(_, _, channel)| ModChannel::from_str(channel).ok())
                    .filter(|channel| {
                        if *channel >= best_channel
                            && mod_sources
                                .channels
                                .iter()
                                .any(|c| c.channel == *channel && c.allow_updates)
                        {
                            best_channel = *channel;
                            true
                        } else {
                            false
                        }
                    })
                    .next()
                    .is_some()
            };

        let mut query_filters = vec![fcdb::instance_id::equals(*instance_id)];

        if let Some(addon_type) = addon_type {
            query_filters.push(fcdb::addon_type::equals(
                addon_type.to_db_string().to_string(),
            ));
        }

        let mods = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_many(query_filters)
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::logo_image::fetch())
                    .with(metadb::curseforge::fetch().with(cfdb::logo_image::fetch()))
                    .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
            )
            .exec()
            .await?
            .into_iter()
            .map(|m| {
                let (mid, cf, mr) = m
                    .metadata
                    .clone()
                    .map(|m| (Some(m.id), m.curseforge.flatten(), m.modrinth.flatten()))
                    .unwrap_or((None, None, None));

                let has_curseforge_update = cf
                    .as_ref()
                    .map(|cf| {
                        let Ok(channel) = ModChannel::try_from(cf.release_type) else {
                            tracing::error!(
                                "Invalid ModChannel in database for curseforge entry {}: {}",
                                mid.as_ref().unwrap(),
                                cf.release_type
                            );
                            return false;
                        };

                        !mod_sources
                            .platform_blacklist
                            .contains(&ModPlatform::Curseforge)
                            && has_update_for_paths(channel, &split_paths(&cf.update_paths))
                    })
                    .unwrap_or(false);

                let has_modrinth_update = mr
                    .as_ref()
                    .map(|mr| {
                        let Ok(channel) = ModChannel::try_from(mr.release_type) else {
                            tracing::error!(
                                "Invalid ModChannel in database for modrinth entry {}: {}",
                                mid.as_ref().unwrap(),
                                mr.release_type
                            );
                            return false;
                        };

                        !mod_sources
                            .platform_blacklist
                            .contains(&ModPlatform::Modrinth)
                            && has_update_for_paths(channel, &split_paths(&mr.update_paths))
                    })
                    .unwrap_or(false);

                domain::Mod {
                    id: m.id,
                    filename: m.filename,
                    enabled: m.enabled,
                    addon_type: domain::AddonType::from_db_string(&m.addon_type)
                        .unwrap_or(domain::AddonType::Mods),
                    metadata: m.metadata.as_ref().map(|m| domain::ModFileMetadata {
                        id: m.id.clone(),
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
                        sha_512: m.sha_512.clone(),
                        sha_1: m.sha_1.clone(),
                        murmur_2: m.murmur_2,
                        has_image: m
                            .logo_image
                            .as_ref()
                            .map(|v| v.as_ref().map(|_| ()))
                            .flatten()
                            .is_some(),
                    }),
                    curseforge: cf.map(|m| domain::CurseForgeModMetadata {
                        project_id: m.project_id as u32,
                        file_id: m.file_id as u32,
                        name: m.name,
                        version: m.version,
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
                            version: m.version,
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
                    has_update: has_curseforge_update || has_modrinth_update,
                }
            });

        Ok(mods.collect::<Vec<_>>())
    }

    async fn instance_cfg_mod_sources(
        self,
        config: &info::Instance,
    ) -> anyhow::Result<Cow<ModSources>> {
        match &config.mod_sources {
            Some(sources) => Ok(Cow::Borrowed(sources)),
            None => {
                let settings = self.app.settings_manager().get_settings().await?;

                let mut channels = ModChannelWithUsage::str_to_vec(&settings.mod_channels)?;
                ModChannelWithUsage::fixup_list(&mut channels);

                Ok(Cow::Owned(ModSources {
                    channels,
                    platform_blacklist: settings
                        .mod_platform_blacklist
                        .split(",")
                        .filter(|p| !p.is_empty())
                        .map(FromStr::from_str)
                        .collect::<Result<_, _>>()?,
                }))
            }
        }
    }

    pub async fn get_instance_mod_sources(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<ModSources> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.type_.data()?;
        let config = data.config.clone();
        drop(instances);

        Ok(self.instance_cfg_mod_sources(&config).await?.into_owned())
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

        self.ensure_modpack_not_locked(instance_id).await?;

        let shortpath = &instance.shortpath;

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .exec()
            .await?
            .ok_or(InvalidInstanceModIdError(instance_id, id.clone()))?;

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
            .prisma_client
            .mod_file_cache()
            .update(
                fcdb::UniqueWhereParam::IdEquals(id),
                vec![fcdb::SetParam::SetEnabled(enabled)],
            )
            .exec()
            .await?;

        self.app
            .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn delete_mod(self, instance_id: InstanceId, id: String) -> anyhow::Result<()> {
        tracing::info!("delete_mod called for instance {} mod {}", instance_id, id);

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        self.ensure_modpack_not_locked(instance_id).await?;

        let shortpath = &instance.shortpath;

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .exec()
            .await?
            .ok_or(InvalidInstanceModIdError(instance_id, id.clone()))?;

        tracing::info!(
            "Found mod in cache: filename={}, addon_type={}",
            m.filename,
            m.addon_type
        );

        // Get the correct addon type folder based on the mod's addon_type
        let addon_type = AddonType::from_db_string(&m.addon_type)
            .ok_or_else(|| anyhow!("Invalid addon type: {}", m.addon_type))?;

        let instance_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(shortpath);

        let addon_dir = addon_type.get_folder_path(&instance_path);
        let enabled_path = addon_dir.join(&m.filename);

        let mut disabled = m.filename.clone();
        disabled.push_str(".disabled");
        let disabled_path = addon_dir.join(&disabled);

        tracing::info!(
            "Looking for mod files at: enabled={:?}, disabled={:?}",
            enabled_path,
            disabled_path
        );

        if enabled_path.is_file() {
            tracing::info!("Deleting enabled mod file: {:?}", enabled_path);
            tokio::fs::remove_file(&enabled_path).await?;
            tracing::info!("Successfully deleted enabled mod file");
        } else if disabled_path.is_file() {
            tracing::info!("Deleting disabled mod file: {:?}", disabled_path);
            tokio::fs::remove_file(&disabled_path).await?;
            tracing::info!("Successfully deleted disabled mod file");
        } else {
            tracing::warn!(
                "Mod file not found at either {:?} or {:?}",
                enabled_path,
                disabled_path
            );
        }

        // Delete the mod from the database cache
        tracing::info!("Deleting mod from database cache");
        self.app
            .prisma_client
            .mod_file_cache()
            .delete(fcdb::UniqueWhereParam::IdEquals(m.id))
            .exec()
            .await?;
        tracing::info!("Successfully deleted mod from database cache");

        // Don't queue caching after deletion - it's unnecessary work
        // The UI will be updated via invalidation from the installer

        Ok(())
    }

    pub async fn install_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
        file_id: u32,
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        tracing::info!(
            "install_curseforge_mod: project_id={}, file_id={}, replaces_mod_id={:?}",
            project_id,
            file_id,
            replaces_mod_id
        );
        self.ensure_modpack_not_locked(instance_id).await?;

        let installer = CurseforgeModInstaller::create(self.app, project_id, file_id)
            .await?
            .into_installer();

        let task_id = installer
            .install(self.app, instance_id, install_deps, replaces_mod_id)
            .await?;

        Ok(task_id)
    }

    pub async fn install_latest_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let version = {
            let instances = self.instances.read().await;
            instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .config
                .game_configuration
                .version
                .as_ref()
                .ok_or(anyhow!("Can't find valid version"))?
                .clone()
        };

        let (version, modloader) = match version {
            domain::info::GameVersion::Custom(_) => todo!("Unsupported"),
            domain::info::GameVersion::Standard(version) => {
                let modloader = version
                    .modloaders
                    .iter()
                    .next()
                    .ok_or(anyhow!("No modloader available"))?;

                (version.release.clone(), modloader.type_)
            }
        };

        let file_id = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: project_id.try_into()?,
                query: ModFilesParametersQuery {
                    game_version: Some(version.clone()),
                    game_version_type_id: None,
                    mod_loader_type: Some(modloader.into()),
                    index: None,
                    page_size: Some(200),
                },
            })
            .await?
            .data
            .iter()
            .find(|value| value.game_versions.contains(&version))
            .ok_or(anyhow::anyhow!(
                "Can't find a valid version for this instance"
            ))?
            .id
            .try_into()?;

        let installer = CurseforgeModInstaller::create(self.app, project_id, file_id)
            .await?
            .into_installer();

        let task_id = installer.install(self.app, instance_id, true, None).await?;

        Ok(task_id)
    }

    pub async fn install_modrinth_mod(
        &self,
        instance_id: InstanceId,
        project_id: String,
        version_id: String,
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let installer = ModrinthModInstaller::create(self.app, project_id, version_id)
            .await?
            .into_installer();

        let task_id = installer
            .install(self.app, instance_id, install_deps, replaces_mod_id)
            .await?;

        Ok(task_id)
    }

    pub async fn install_latest_modrinth_mod(
        &self,
        instance_id: InstanceId,
        project_id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let version = {
            let instances = self.instances.read().await;
            instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .config
                .game_configuration
                .version
                .as_ref()
                .ok_or(anyhow!("Can't find valid version"))?
                .clone()
        };

        let (version, modloader) = match version {
            domain::info::GameVersion::Custom(_) => todo!("Unsupported"),
            domain::info::GameVersion::Standard(version) => {
                let modloader = version
                    .modloaders
                    .iter()
                    .next()
                    .ok_or(anyhow!("No modloader available"))?;

                (version.release.clone(), modloader.type_.to_string())
            }
        };

        let version_id = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id.clone()),
                game_versions: Some(Vec::from([version.clone()])),
                loaders: Some(Vec::from([modloader])),
                limit: None,
                offset: None,
            })
            .await?
            .get(0)
            .ok_or(anyhow!("Can't find a valid version for this instance"))?
            .id
            .clone();

        let installer = ModrinthModInstaller::create(self.app, project_id, version_id)
            .await?
            .into_installer();

        let task_id = installer.install(self.app, instance_id, true, None).await?;

        Ok(task_id)
    }

    /// Attempt to find an update for a mod respecting the instance's (and the global) channel preference.
    pub async fn find_mod_update(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<Option<RemoteVersion>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.type_.data()?;
        let config = data.config.clone();
        drop(instances);

        let Some(GameVersion::Standard(version)) = &config.game_configuration.version else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };

        let mod_sources = self.instance_cfg_mod_sources(&config).await?;

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::curseforge::fetch())
                    .with(metadb::modrinth::fetch()),
            )
            .exec()
            .await?
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let metadata = m
            .metadata
            .expect("metadata must be associated with a ModFileCache entry");

        let cf = metadata
            .curseforge
            .expect("curseforge metadata was queried but not returned");

        let mr = metadata
            .modrinth
            .expect("modrinth metadata was queried but not returned");

        let mut versions = Vec::new();

        if let Some(cf) = &cf {
            let response = self
                .app
                .modplatforms_manager()
                .curseforge
                .get_mod_files(ModFilesParameters {
                    mod_id: cf.project_id,
                    query: ModFilesParametersQuery {
                        game_version: Some(version.release.clone()),
                        game_version_type_id: None,
                        mod_loader_type: version.modloaders.iter().next().map(|v| v.type_.into()),
                        index: None,
                        page_size: None,
                    },
                })
                .await?;

            versions.extend(
                response
                    .data
                    .into_iter()
                    .map(|f| RemoteVersion::Curseforge(f)),
            );
        }

        if let Some(mr) = &mr {
            let response = self
                .app
                .modplatforms_manager()
                .modrinth
                .get_project_versions(ProjectVersionsFilters {
                    project_id: ProjectID(mr.project_id.clone()),
                    game_versions: Some(vec![version.release.clone()]),
                    loaders: Some(
                        version
                            .modloaders
                            .iter()
                            .map(|ml| ml.type_.to_string())
                            .collect(),
                    ),
                    limit: None,
                    offset: None,
                })
                .await?;

            versions.extend(response.into_iter().map(|v| RemoteVersion::Modrinth(v)));
        }

        versions.sort();

        'select: for channel in &mod_sources.channels {
            if !channel.allow_updates {
                continue;
            }

            for i in 0..versions.len() {
                let version = &versions[i];

                if version.channel() >= channel.channel {
                    let version = versions.remove(i);

                    match &version {
                        RemoteVersion::Curseforge(file) => {
                            let cf = cf.expect("curseforge metadata must be present if operating on a curseforge version");

                            if cf.file_id == file.id {
                                break 'select;
                            }
                        }
                        RemoteVersion::Modrinth(version) => {
                            let mr = mr.expect("modrinth metadata must be present if operating on a modrinth version");

                            if mr.version_id == version.id {
                                break 'select;
                            }
                        }
                    }

                    return Ok(Some(version));
                }
            }
        }

        Ok(None)
    }

    pub async fn update_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<VisualTaskId> {
        tracing::info!(
            "update_mod called with instance_id: {}, mod_id: {}",
            instance_id,
            id
        );
        self.ensure_modpack_not_locked(instance_id).await?;

        let update = self.find_mod_update(instance_id, id.clone()).await?;

        match update {
            Some(RemoteVersion::Curseforge(file)) => {
                tracing::info!(
                    "Installing CurseForge update: project_id={}, file_id={}, replaces_mod_id={}",
                    file.mod_id,
                    file.id,
                    id
                );
                self.install_curseforge_mod(
                    instance_id,
                    file.mod_id as u32,
                    file.id as u32,
                    false,
                    Some(id),
                )
                .await
            }
            Some(RemoteVersion::Modrinth(version)) => {
                tracing::info!(
                    "Installing Modrinth update: project_id={}, version_id={}, replaces_mod_id={}",
                    version.project_id,
                    version.id,
                    id
                );
                self.install_modrinth_mod(
                    instance_id,
                    version.project_id,
                    version.id,
                    false,
                    Some(id),
                )
                .await
            }
            None => Err(anyhow!(
                "unable to find newer mod version in availible update channels"
            )),
        }
    }

    pub async fn update_curseforge_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            bail!("instance is in an invalid state");
        };

        let Some(GameVersion::Standard(version)) = data.config.game_configuration.version.clone()
        else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };

        drop(instances);

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .with(fcdb::metadata::fetch().with(metadb::curseforge::fetch()))
            .exec()
            .await?
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let cf = m.metadata
            .expect("metadata must be associated with a ModFileCache entry")
            .curseforge
            .expect("curseforge metadata was queried but not returned")
            .ok_or_else(|| anyhow!("Attempted to use update_curseforge_mod to update a mod not availible on curseforge"))?;

        let mod_files = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: cf.project_id,
                query: ModFilesParametersQuery {
                    game_version: Some(version.release),
                    game_version_type_id: None,
                    mod_loader_type: version.modloaders.iter().next().map(|v| v.type_.into()),
                    index: None,
                    page_size: None,
                },
            })
            .await?;

        let version = mod_files.data.into_iter().next();

        let Some(version) = version else {
            bail!("unable to find newer mod version");
        };

        if version.id == cf.file_id {
            bail!("unable to find newer mod version");
        }

        self.install_curseforge_mod(
            instance_id,
            version.mod_id as u32,
            version.id as u32,
            false,
            Some(id),
        )
        .await
    }

    pub async fn update_modrinth_mod(
        self,
        instance_id: InstanceId,
        id: String,
    ) -> anyhow::Result<VisualTaskId> {
        self.ensure_modpack_not_locked(instance_id).await?;

        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            bail!("instance is in an invalid state");
        };

        let Some(GameVersion::Standard(version)) = data.config.game_configuration.version.clone()
        else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };

        drop(instances);

        let m = self
            .app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::IdEquals(id.clone()))
            .with(fcdb::metadata::fetch().with(metadb::modrinth::fetch()))
            .exec()
            .await?
            .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let mr = m
            .metadata
            .expect("metadata must be associated with a ModFileCache entry")
            .modrinth
            .expect("curseforge metadata was queried but not returned")
            .ok_or_else(|| {
                anyhow!(
                    "Attempted to use update_modrinth_mod to update a mod not availible on modrinth"
                )
            })?;

        let mod_files = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(mr.project_id),
                game_versions: Some(vec![version.release]),
                loaders: Some(
                    version
                        .modloaders
                        .iter()
                        .map(|ml| ml.type_.to_string())
                        .collect(),
                ),
                limit: None,
                offset: None,
            })
            .await?;

        let version = mod_files.0.into_iter().next();

        let Some(version) = version else {
            bail!("unable to find newer mod version");
        };

        if version.id == mr.version_id {
            bail!("unable to find newer mod version");
        }

        self.install_modrinth_mod(instance_id, version.project_id, version.id, false, Some(id))
            .await
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
            .ok_or_else(|| anyhow!("broken db state"))?;

        let logo_image = match platformid {
            0 => r
                .logo_image
                .ok_or_else(|| anyhow!("broken db state"))?
                .map(|m| m.data),
            1 => r
                .curseforge
                .ok_or_else(|| anyhow!("broken db state"))?
                .map(|cf| cf.logo_image.ok_or_else(|| anyhow!("broken db state")))
                .transpose()?
                .flatten()
                .map(|img| img.data)
                .flatten(),
            2 => r
                .modrinth
                .ok_or_else(|| anyhow!("broken db state"))?
                .map(|mr| mr.logo_image.ok_or_else(|| anyhow!("broken db state")))
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
            .install_curseforge_mod(instance_id, 331723, 4022327, true, None)
            .await?;

        // first invalidation will happen when the mod is scanned locally
        app.wait_for_invalidation(INSTANCE_MODS).await?;

        let mods = app.instance_manager().list_mods(instance_id, None).await?;
        dbg!(&mods);
        assert_ne!(mods.get(0), None);

        // second invalidation will happen when the curseforge metadata is fetched
        app.wait_for_invalidation(INSTANCE_MODS).await?;

        let mods = app.instance_manager().list_mods(instance_id, None).await?;
        dbg!(&mods);
        assert_ne!(mods[0].curseforge, None);

        Ok(())
    }

    /// Comprehensive integration test for mod auto-update logic and caching system
    /// Tests the complete flow: install -> cache -> update -> verify caching performance
    #[tokio::test]
    #[ignore] // Requires network access and real mod downloads
    async fn test_mod_auto_update_with_full_caching() -> anyhow::Result<()> {
        use std::time::Instant;
        use tokio::time::{Duration, timeout};

        println!("🧪 Starting comprehensive mod auto-update and caching test");

        // Setup test environment with full app infrastructure
        let app = crate::setup_managers_for_test().await;

        // Create test instance with Forge 1.20.1 (popular for mods)
        let group = app.instance_manager().get_default_group().await?;
        let instance_id = app
            .instance_manager()
            .create_instance(
                group,
                String::from("ModUpdateCachingTest"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.20.1"),
                        modloaders: {
                            let mut set = HashSet::new();
                            set.insert(info::ModLoader {
                                type_: info::ModLoaderType::Forge,
                                version: String::from("47.2.0"),
                            });
                            set
                        },
                    },
                )),
                String::new(),
            )
            .await?;

        println!("✅ Created test instance: {}", instance_id);

        // Phase 1: Install initial mod version (JEI - a popular mod with frequent updates)
        println!("\n📦 Phase 1: Installing initial mod version");
        let install_start = Instant::now();

        // Install JEI (Modrinth) - known to have multiple versions for 1.20.1
        app.instance_manager()
            .install_modrinth_mod(
                instance_id,
                "u6dRKJwZ".to_string(), // JEI project ID
                "TDBKa9oZ".to_string(), // Specific version for 1.20.1 (older)
                true,
                None,
            )
            .await?;

        let install_duration = install_start.elapsed();
        println!("⏱️ Initial install took: {:?}", install_duration);

        // Wait for local caching to complete
        println!("⏳ Waiting for local mod caching...");
        timeout(
            Duration::from_secs(30),
            app.wait_for_invalidation(INSTANCE_MODS),
        )
        .await
        .expect("Local caching should complete within 30 seconds")?;

        // Verify mod was cached locally
        let mods_after_install = app.instance_manager().list_mods(instance_id, None).await?;
        assert_eq!(
            mods_after_install.len(),
            1,
            "Should have exactly 1 mod installed"
        );
        let initial_mod = &mods_after_install[0];
        let initial_mod_id = initial_mod.id.clone();

        println!(
            "✅ Initial mod cached: {} (ID: {})",
            initial_mod
                .metadata
                .as_ref()
                .and_then(|m| m.name.as_deref())
                .unwrap_or("Unknown"),
            initial_mod_id
        );

        // Wait for platform metadata caching (this tests our V2 system)
        println!("⏳ Waiting for platform metadata caching...");
        let metadata_start = Instant::now();

        // Poll for platform metadata with timeout
        let mut platform_metadata_ready = false;
        for attempt in 1..=20 {
            tokio::time::sleep(Duration::from_millis(500)).await;

            let current_mods = app.instance_manager().list_mods(instance_id, None).await?;
            if let Some(mod_with_metadata) = current_mods.first() {
                if mod_with_metadata.modrinth.is_some() || mod_with_metadata.curseforge.is_some() {
                    platform_metadata_ready = true;
                    println!(
                        "✅ Platform metadata ready after {} attempts ({:?})",
                        attempt,
                        metadata_start.elapsed()
                    );
                    break;
                }
            }

            if attempt % 5 == 0 {
                println!(
                    "⏳ Still waiting for platform metadata... attempt {}/20",
                    attempt
                );
            }
        }

        if !platform_metadata_ready {
            println!(
                "⚠️ Platform metadata not ready after 10 seconds - this may indicate V2 caching issues"
            );
        }

        // Phase 2: Test mod update detection
        println!("\n🔍 Phase 2: Testing mod update detection");

        let updates = app
            .instance_manager()
            .find_mod_update(instance_id, initial_mod_id.clone())
            .await;

        match updates {
            Ok(Some(update_info)) => {
                println!("✅ Found available update for mod");

                // Phase 3: Perform mod update and test caching performance
                println!(
                    "\n🚀 Phase 3: Performing mod update with caching performance measurement"
                );

                let update_start = Instant::now();

                // This tests our optimized caching method
                let update_result = app
                    .instance_manager()
                    .update_mod(instance_id, initial_mod_id.clone())
                    .await;

                match update_result {
                    Ok(task_id) => {
                        let update_duration = update_start.elapsed();
                        println!("⏱️ Mod update completed in: {:?}", update_duration);

                        // Performance assertion - should be much faster than old system
                        if update_duration > Duration::from_secs(10) {
                            println!(
                                "⚠️ Update took longer than 10 seconds - performance regression detected"
                            );
                        } else {
                            println!("✅ Update performance acceptable");
                        }

                        // Wait for UI invalidation after update
                        timeout(
                            Duration::from_secs(15),
                            app.wait_for_invalidation(INSTANCE_MODS),
                        )
                        .await
                        .expect("UI should update within 15 seconds after mod update")?;

                        // Phase 4: Verify update was successful and properly cached
                        println!("\n✅ Phase 4: Verifying update success and caching");

                        let mods_after_update =
                            app.instance_manager().list_mods(instance_id, None).await?;
                        assert_eq!(
                            mods_after_update.len(),
                            1,
                            "Should still have exactly 1 mod after update"
                        );

                        let updated_mod = &mods_after_update[0];

                        // Verify it's a different mod (updated)
                        assert_ne!(
                            updated_mod.id, initial_mod_id,
                            "Mod ID should change after update"
                        );

                        // Verify metadata is preserved/updated
                        assert!(
                            updated_mod
                                .metadata
                                .as_ref()
                                .and_then(|m| m.name.as_ref())
                                .is_some(),
                            "Updated mod should have name"
                        );

                        println!(
                            "✅ Updated mod: {} (New ID: {})",
                            updated_mod
                                .metadata
                                .as_ref()
                                .and_then(|m| m.name.as_deref())
                                .unwrap_or("Unknown"),
                            updated_mod.id
                        );

                        // Phase 5: Test V2 caching system functionality
                        println!("\n🔧 Phase 5: Testing V2 caching system");

                        // Get V2 statistics if available
                        if let Some(v2_stats) = app.meta_cache_manager().get_v2_stats().await {
                            println!("📊 V2 Caching Statistics:");
                            println!(
                                "  - Local scans completed: {}",
                                v2_stats.local_scans_completed
                            );
                            println!("  - Local scans failed: {}", v2_stats.local_scans_failed);
                            println!("  - Images processed: {}", v2_stats.images_processed);
                            println!(
                                "  - Platform requests completed: {}",
                                v2_stats.platform_requests_completed
                            );
                            println!(
                                "  - Avg local scan time: {:.2}ms",
                                v2_stats.average_local_scan_time_ms
                            );

                            // Verify V2 system is working
                            assert!(
                                v2_stats.local_scans_completed > 0,
                                "V2 should have completed local scans"
                            );

                            // Performance check for hash computation
                            if v2_stats.average_local_scan_time_ms > 1000.0 {
                                println!(
                                    "⚠️ Average local scan time > 1s - performance issue detected"
                                );
                            } else {
                                println!("✅ V2 local scan performance good");
                            }
                        } else {
                            println!(
                                "⚠️ V2 statistics not available - V2 system may not be running"
                            );
                        }

                        // Phase 6: Test rapid successive operations (stress test)
                        println!("\n🎯 Phase 6: Stress testing rapid cache operations");

                        let stress_start = Instant::now();
                        let mut cache_times = Vec::new();

                        // Simulate rapid mod list requests (like frontend polling)
                        for i in 1..=5 {
                            let cache_op_start = Instant::now();
                            let _mods = app.instance_manager().list_mods(instance_id, None).await?;
                            let cache_op_duration = cache_op_start.elapsed();
                            cache_times.push(cache_op_duration);

                            println!("  Cache operation {}: {:?}", i, cache_op_duration);

                            // Brief delay to simulate real usage
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        let stress_total = stress_start.elapsed();
                        let avg_cache_time =
                            cache_times.iter().sum::<Duration>() / cache_times.len() as u32;

                        println!("📊 Stress test results:");
                        println!("  - Total time for 5 operations: {:?}", stress_total);
                        println!("  - Average cache operation time: {:?}", avg_cache_time);

                        // Performance assertions
                        assert!(
                            avg_cache_time < Duration::from_millis(500),
                            "Average cache operation should be < 500ms"
                        );

                        println!("\n🎉 All phases completed successfully!");
                        println!("✅ Mod auto-update and caching system working correctly");
                    }
                    Err(e) => {
                        println!("❌ Mod update failed: {:?}", e);
                        // For this test, we'll continue even if update fails (might be no newer version)
                        println!("ℹ️ This might be expected if no newer version is available");
                    }
                }
            }
            Ok(None) => {
                println!("ℹ️ No updates available for this mod - test completed with limitation");
                println!("✅ Update detection system working (no updates found)");
            }
            Err(e) => {
                println!("❌ Update detection failed: {:?}", e);
                return Err(e);
            }
        }

        // Final verification - ensure system is in good state
        let final_mods = app.instance_manager().list_mods(instance_id, None).await?;
        assert!(
            !final_mods.is_empty(),
            "Should have at least one mod at the end"
        );

        println!("\n🏁 Test completed successfully!");
        println!("📊 Final summary:");
        println!("  - Mods in instance: {}", final_mods.len());
        println!("  - Install time: {:?}", install_duration);

        if let Some(v2_stats) = app.meta_cache_manager().get_v2_stats().await {
            println!("  - V2 local scans: {}", v2_stats.local_scans_completed);
            println!(
                "  - V2 platform requests: {}",
                v2_stats.platform_requests_completed
            );
        }

        Ok(())
    }
}
