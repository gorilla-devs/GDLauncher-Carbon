use std::{collections::HashSet, path::PathBuf, sync::Arc};

use tokio::{
    fs::create_dir_all,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};
use tracing::info;

use crate::{
    api::{instance::import::FEEntity, keys},
    domain::{
        instance::{
            info::{
                CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
            },
            InstanceId,
        },
        vtask::VisualTaskId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};

use super::InstanceImporter;

#[derive(Debug, Default)]
pub struct LegacyGDLauncherImporter {
    results: Mutex<Vec<LegacyGDLauncherConfigWrapper>>,
}

#[async_trait::async_trait]
impl InstanceImporter for LegacyGDLauncherImporter {
    type Config = LegacyGDLauncherConfigWrapper;

    async fn scan(&mut self, app: Arc<AppInner>) -> anyhow::Result<()> {
        let mut old_gdl_base_path = directories::BaseDirs::new()
            .ok_or(anyhow::anyhow!("Cannot build basedirs"))?
            .data_dir()
            .join("gdlauncher_next");

        let override_path = old_gdl_base_path.join("override.data");

        if override_path.exists() {
            let override_path = tokio::fs::read_to_string(override_path).await;

            if let Ok(override_path) = override_path {
                let override_path = PathBuf::from(override_path);

                if override_path.exists() {
                    old_gdl_base_path = override_path;
                }
            }
        }

        let instances_path = old_gdl_base_path.join("instances");

        let Ok(mut all_instances) = tokio::fs::read_dir(&instances_path).await else {
            return Ok(());
        };

        self.results.lock().await.clear();
        app.invalidate(
            keys::instance::GET_IMPORTABLE_INSTANCES,
            Some(serde_json::to_value(FEEntity::LegacyGDLauncher)?),
        );

        while let Some(child) = all_instances.next_entry().await? {
            if child.metadata().await?.is_dir() {
                let config = child.path().join("config.json");
                if !config.exists() {
                    continue;
                }

                let config = tokio::fs::read_to_string(config).await?;
                let Ok(config): Result<LegacyGDLauncherConfig, serde_json::Error> = serde_json::from_str(&config) else {
                    tracing::info!(
                        "Failed to parse legacy gdlauncher config: {}",
                        child.path().display()
                    );
                    continue;
                };

                let mut lock = self.results.lock().await;

                lock.push(LegacyGDLauncherConfigWrapper {
                    name: child.file_name().into_string().unwrap(),
                    full_path: child.path(),
                    config,
                });

                app.invalidate(
                    keys::instance::GET_IMPORTABLE_INSTANCES,
                    Some(serde_json::to_value(FEEntity::LegacyGDLauncher)?),
                );
            }
        }

        Ok(())
    }

    async fn get_available(&self) -> anyhow::Result<Vec<super::ImportableInstance>> {
        let mut instances = Vec::new();

        let lock = self.results.lock().await;
        for instance in lock.iter() {
            instances.push(super::ImportableInstance {
                name: instance.name.clone(),
            });
        }

        Ok(instances)
    }

    async fn import(&self, app: Arc<AppInner>, index: u32) -> anyhow::Result<VisualTaskId> {
        let lock = self.results.lock().await;
        let instance = lock
            .get(index as usize)
            .ok_or(anyhow::anyhow!("No importable instance at index {index}"))?;

        if let Some(ref background) = instance.config.background {
            app.instance_manager()
                .load_icon(instance.full_path.join(background))
                .await?;
        }

        let instance_version_source = 'a: {
            let modloader = match &*instance.config.loader.loader_type {
                "forge" => Some(ModLoaderType::Forge),
                "fabric" => Some(ModLoaderType::Fabric),
                _ => None,
            }
            .and_then(|loader_type| {
                let Some(ref loader_version) = instance.config.loader.loader_version else {
                        return None;
                    };

                Some(ModLoader {
                    type_: loader_type,
                    version: loader_version.to_owned(),
                })
            })
            .map(|modloader| HashSet::from_iter(vec![modloader]))
            .unwrap_or_default();

            let standard_version = GameVersion::Standard(StandardVersion {
                release: instance.config.loader.mc_version.clone(),
                modloaders: modloader,
            });

            if let Some(ref source) = instance.config.loader.source {
                if source != "curseforge" {
                    break 'a InstanceVersionSource::Version(standard_version);
                }

                let Some(project_id) = instance.config.loader.project_id else {
                        return Err(anyhow::anyhow!("Missing project id"));

                };
                let Some(file_id) = instance.config.loader.file_id else {
                        return Err(anyhow::anyhow!("Missing file id"));
                };

                let curseforge_modpack = CurseforgeModpack {
                    project_id,
                    file_id,
                };

                break 'a InstanceVersionSource::Modpack(Modpack::Curseforge(curseforge_modpack));
            } else {
                break 'a InstanceVersionSource::Version(standard_version);
            }
        };

        let created_instance_id = app
            .instance_manager()
            .create_instance(
                app.instance_manager().get_default_group().await?,
                instance.name.clone(),
                instance.config.background.is_some(),
                instance_version_source,
                "".to_string(),
            )
            .await?;

        let (_, task_id) = app
            .instance_manager()
            .prepare_game(created_instance_id, None)
            .await?;

        let walked_dir = walkdir::WalkDir::new(&instance.full_path)
            .into_iter()
            .filter_map(|entry| {
                let Ok(entry) = entry else {
                    return None;
                };

                let Some(file_name) = entry.file_name().to_str() else {
                    return None;
                };

                match file_name {
                    "config.json" => None,
                    _ => {
                        if let Some(ref background) = instance.config.background {
                            if file_name == background {
                                return None;
                            }
                        }

                        Some(entry)
                    }
                }
            });

        let instances_path = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        let instance_path = instances_path.join(
            &app.instance_manager()
                .instances
                .read()
                .await
                .get(&created_instance_id)
                .unwrap()
                .shortpath,
        );

        for entry in walked_dir {
            let is_dir = entry.file_type().is_dir();
            let path = entry.path();
            let relative_path = path.strip_prefix(&instance.full_path).unwrap();

            let destination = instance_path.join(relative_path);

            if destination.exists() {
                // TODO: Check checksum
                continue;
            }

            if is_dir {
                create_dir_all(destination).await?;
            } else {
                let mut file = tokio::fs::File::open(path).await?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).await?;

                let mut file = tokio::fs::File::create(destination).await?;
                file.write_all(&buffer).await?;
            }
        }

        Ok(task_id)
    }
}

#[derive(Debug)]
pub struct LegacyGDLauncherConfigWrapper {
    name: String,
    full_path: PathBuf,
    config: LegacyGDLauncherConfig,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyGDLauncherConfig {
    loader: _Loader,
    time_played: u64,
    last_played: Option<u64>,
    background: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct _Loader {
    loader_type: String,
    loader_version: Option<String>,
    mc_version: String,
    #[serde(rename = "fileID")]
    file_id: Option<u32>,
    #[serde(rename = "projectID")]
    project_id: Option<u32>,
    source: Option<String>,
    source_name: Option<String>,
}

mod test {
    use crate::managers::instance::importer::InstanceImporter;

    #[tokio::test]
    async fn test_legacy_gdlauncher_importer() {
        let app = crate::setup_managers_for_test().await;

        let mut importer = super::LegacyGDLauncherImporter::default();
        importer.scan(app.app.clone()).await.unwrap();

        let instances = importer.get_available().await.unwrap();

        for (index, _) in instances.iter().enumerate() {
            importer
                .import(app.app.clone(), index as u32)
                .await
                .unwrap();
        }
    }
}
