use super::{
    InstanceId, InstanceManager, InvalidInstanceIdError,
    installer::{CurseforgeModInstaller, IntoInstaller, ModrinthModInstaller},
};
use crate::api::keys::instance::INSTANCE_MODS;
use crate::domain::instance::info::{GameVersion, ModLoaderType};
use crate::domain::instance::{self as domain, info};
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
use carbon_repos::{OptionalExt, models, queries};
use chrono::{DateTime, FixedOffset, Utc};
use futures::Future;
use std::borrow::Cow;
use std::str::FromStr;
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

        let pool = self.app.db_pool.clone();
        let instance_id_val = *instance_id;
        let addon_type_filter = addon_type.map(|t| t.to_db_string().to_string());

        let mods = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;

            let mods: Vec<models::ModFileCacheWithMetadataAndImages> = if let Some(
                ref addon_type_str,
            ) = addon_type_filter
            {
                queries::metadata::ListModFileCacheWithMetadataAndImagesByInstanceAndType::fetch_all(
                    &conn,
                    instance_id_val,
                    addon_type_str,
                )?
            } else {
                queries::metadata::ListModFileCacheWithMetadataAndImagesByInstance::fetch_all(
                    &conn,
                    instance_id_val,
                )?
            };

            Ok::<_, anyhow::Error>(mods)
        })
        .await??;

        // Detect duplicated mods by grouping enabled mods by modid
        let mut modid_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for m in &mods {
            // Only consider enabled mods with modid
            if m.enabled {
                if let Some(ref modid) = m.modid {
                    *modid_counts.entry(modid.clone()).or_insert(0) += 1;
                }
            }
        }

        // Build set of modids that have duplicates (appear more than once)
        let duplicate_modids: std::collections::HashSet<String> = modid_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(modid, _)| modid)
            .collect();

        let mods = mods.into_iter().map(|m| {
            let has_cf = m.cf_project_id.is_some();
            let has_mr = m.mr_project_id.is_some();

            let has_curseforge_update = if let (Some(release_type), Some(update_paths)) =
                (m.cf_release_type, &m.cf_update_paths)
            {
                match ModChannel::try_from(release_type) {
                    Ok(channel) => {
                        !mod_sources
                            .platform_blacklist
                            .contains(&ModPlatform::Curseforge)
                            && has_update_for_paths(channel, &split_paths(update_paths))
                    }
                    Err(_) => {
                        tracing::error!(
                            "Invalid ModChannel in database for curseforge entry {}: {}",
                            &m.metadata_id,
                            release_type
                        );
                        false
                    }
                }
            } else {
                false
            };

            let has_modrinth_update = if let (Some(release_type), Some(update_paths)) =
                (m.mr_release_type, &m.mr_update_paths)
            {
                match ModChannel::try_from(release_type) {
                    Ok(channel) => {
                        !mod_sources
                            .platform_blacklist
                            .contains(&ModPlatform::Modrinth)
                            && has_update_for_paths(channel, &split_paths(update_paths))
                    }
                    Err(_) => {
                        tracing::error!(
                            "Invalid ModChannel in database for modrinth entry {}: {}",
                            &m.metadata_id,
                            release_type
                        );
                        false
                    }
                }
            } else {
                false
            };

            domain::Mod {
                id: m.id.clone(),
                filename: m.filename.clone(),
                enabled: m.enabled,
                addon_type: domain::AddonType::from_db_string(&m.addon_type)
                    .unwrap_or(domain::AddonType::Mods),
                metadata: Some(domain::ModFileMetadata {
                    id: m.metadata_id.clone(),
                    modid: m.modid.clone(),
                    name: m.name.clone(),
                    version: m.version.clone(),
                    description: m.description.clone(),
                    authors: m.authors.clone(),
                    modloaders: m
                        .modloaders
                        .split(',')
                        .flat_map(|loader| ModLoaderType::try_from(loader).ok())
                        .collect::<Vec<_>>(),
                    sha_512: m.sha512.clone(),
                    sha_1: m.sha1.clone(),
                    murmur_2: m.murmur2,
                    has_image: m.has_local_image,
                }),
                curseforge: if has_cf {
                    Some(domain::CurseForgeModMetadata {
                        project_id: m.cf_project_id.unwrap() as u32,
                        file_id: m.cf_file_id.unwrap() as u32,
                        name: m.cf_name.clone().unwrap_or_default(),
                        version: m.cf_version.clone().unwrap_or_default(),
                        urlslug: m.cf_urlslug.clone().unwrap_or_default(),
                        summary: m.cf_summary.clone().unwrap_or_default(),
                        authors: m.cf_authors.clone().unwrap_or_default(),
                        has_image: m.has_cf_image,
                    })
                } else {
                    None
                },
                modrinth: if has_mr {
                    Some(domain::ModrinthModMetadata {
                        project_id: m.mr_project_id.clone().unwrap(),
                        version_id: m.mr_version_id.clone().unwrap(),
                        title: m.mr_title.clone().unwrap_or_default(),
                        version: m.mr_version.clone().unwrap_or_default(),
                        urlslug: m.mr_urlslug.clone().unwrap_or_default(),
                        description: m.mr_description.clone().unwrap_or_default(),
                        authors: m.mr_authors.clone().unwrap_or_default(),
                        has_image: m.has_mr_image,
                    })
                } else {
                    None
                },
                has_update: has_curseforge_update || has_modrinth_update,
                is_duplicate: m.enabled
                    && m.modid
                        .as_ref()
                        .map(|modid| duplicate_modids.contains(modid))
                        .unwrap_or(false),
                file_size: m.filesize as u64,
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

        let pool = self.app.db_pool.clone();
        let id_clone = id.clone();
        let m = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            Ok::<_, anyhow::Error>(queries::metadata::FindModFileCache::fetch_optional(
                &conn, &id_clone,
            )?)
        })
        .await??
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

        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::metadata::UpdateModFileCacheEnabled::execute(&conn, &id, enabled)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        self.app
            .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn delete_mod(self, instance_id: InstanceId, id: String) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        self.ensure_modpack_not_locked(instance_id).await?;

        let shortpath = &instance.shortpath;

        let pool = self.app.db_pool.clone();
        let id_clone = id.clone();
        let m = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            Ok::<_, anyhow::Error>(queries::metadata::FindModFileCache::fetch_optional(
                &conn, &id_clone,
            )?)
        })
        .await??
        .ok_or(InvalidInstanceModIdError(instance_id, id))?;

        let mut disabled_path = {
            let instance_path = self
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(shortpath);

            domain::AddonType::from_db_string(&m.addon_type)
                .unwrap_or(domain::AddonType::Mods)
                .get_folder_path(&instance_path)
        };

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
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
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

        let project = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod(ModParameters {
                mod_id: project_id.try_into()?,
            })
            .await?;

        let (version, modloader) = match version {
            domain::info::GameVersion::Custom(_) => todo!("Unsupported"),
            domain::info::GameVersion::Standard(version) => {
                let first_modloader = version.modloaders.iter().next();

                let modloader = match project.data.class_id {
                    Some(carbon_platforms::curseforge::ClassId::Mods) | None => first_modloader,
                    _ => None,
                };

                (version.release.clone(), modloader.map(|v| v.type_.into()))
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
                    mod_loader_type: modloader,
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

        let project = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project(ProjectID(project_id.clone()))
            .await?;

        let (version, modloader) = match version {
            domain::info::GameVersion::Custom(_) => todo!("Unsupported"),
            domain::info::GameVersion::Standard(version) => {
                let first_modloader = version.modloaders.iter().next();

                let modloader = match project.project_type {
                    carbon_platforms::modrinth::project::ProjectType::Mod => first_modloader,
                    _ => None,
                };

                (
                    version.release.clone(),
                    modloader.map(|v| vec![v.type_.to_string()]),
                )
            }
        };

        let version_id = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id.clone()),
                game_versions: Some(Vec::from([version.clone()])),
                loaders: modloader,
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

        let pool = self.app.db_pool.clone();
        let id_clone = id.clone();
        let m = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            Ok::<_, anyhow::Error>(
                queries::metadata::FindModFileCacheWithMetadata::fetch_optional(&conn, &id_clone)?,
            )
        })
        .await??
        .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let cf = m
            .cf_project_id
            .map(|_| (m.cf_project_id.unwrap(), m.cf_file_id.unwrap()));

        let mr = m
            .mr_project_id
            .clone()
            .map(|project_id| (project_id, m.mr_version_id.clone().unwrap()));

        let mut versions = Vec::new();

        if let Some((cf_project_id, cf_file_id)) = &cf {
            let response = self
                .app
                .modplatforms_manager()
                .curseforge
                .get_mod_files(ModFilesParameters {
                    mod_id: *cf_project_id,
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

        if let Some((mr_project_id, mr_version_id)) = &mr {
            let response = self
                .app
                .modplatforms_manager()
                .modrinth
                .get_project_versions(ProjectVersionsFilters {
                    project_id: ProjectID(mr_project_id.clone()),
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
                            let (_, cf_file_id) = cf.as_ref().expect("curseforge metadata must be present if operating on a curseforge version");

                            if *cf_file_id == file.id {
                                break 'select;
                            }
                        }
                        RemoteVersion::Modrinth(version) => {
                            let (_, mr_version_id) = mr.as_ref().expect("modrinth metadata must be present if operating on a modrinth version");

                            if *mr_version_id == version.id {
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
        self.ensure_modpack_not_locked(instance_id).await?;

        let update = self.find_mod_update(instance_id, id.clone()).await?;

        match update {
            Some(RemoteVersion::Curseforge(file)) => {
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

        let pool = self.app.db_pool.clone();
        let id_clone = id.clone();
        let m = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            Ok::<_, anyhow::Error>(
                queries::metadata::FindModFileCacheWithMetadata::fetch_optional(&conn, &id_clone)?,
            )
        })
        .await??
        .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let (cf_project_id, cf_file_id) = m.cf_project_id.zip(m.cf_file_id).ok_or_else(|| {
            anyhow!(
                "Attempted to use update_curseforge_mod to update a mod not availible on curseforge"
            )
        })?;

        let mod_files = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: cf_project_id,
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

        if version.id == cf_file_id {
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

        let pool = self.app.db_pool.clone();
        let id_clone = id.clone();
        let m = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            Ok::<_, anyhow::Error>(
                queries::metadata::FindModFileCacheWithMetadata::fetch_optional(&conn, &id_clone)?,
            )
        })
        .await??
        .ok_or_else(|| InvalidInstanceModIdError(instance_id, id.clone()))?;

        let (mr_project_id, mr_version_id) =
            m.mr_project_id.zip(m.mr_version_id).ok_or_else(|| {
                anyhow!(
                    "Attempted to use update_modrinth_mod to update a mod not availible on modrinth"
                )
            })?;

        let mod_files = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(mr_project_id.clone()),
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

        if version.id == mr_version_id {
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

        let pool = self.app.db_pool.clone();
        let mod_id_clone = mod_id.clone();

        // First, get the metadata_id from the mod file cache
        let metadata_id = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            Ok::<_, anyhow::Error>(
                queries::metadata::FindModFileCache::fetch_optional(&conn, &mod_id_clone)?
                    .map(|m| m.metadata_id),
            )
        })
        .await??
        .ok_or(InvalidModIdError(mod_id))?;

        let pool = self.app.db_pool.clone();
        let metadata_id_clone = metadata_id.clone();

        // Fetch the appropriate image based on platformid
        let logo_image = match platformid {
            0 => {
                // Local mod image
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    Ok::<_, anyhow::Error>(
                        queries::metadata::FindLocalModImageCache::fetch_optional(
                            &conn,
                            &metadata_id_clone,
                        )?
                        .map(|m| m.data),
                    )
                })
                .await??
            }
            1 => {
                // CurseForge image
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    Ok::<_, anyhow::Error>(
                        queries::metadata::FindCurseForgeModImageCache::fetch_optional(
                            &conn,
                            &metadata_id_clone,
                        )?
                        .and_then(|m| m.data),
                    )
                })
                .await??
            }
            2 => {
                // Modrinth image
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    Ok::<_, anyhow::Error>(
                        queries::metadata::FindModrinthModImageCache::fetch_optional(
                            &conn,
                            &metadata_id_clone,
                        )?
                        .and_then(|m| m.data),
                    )
                })
                .await??
            }
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
}
