use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::{
    domain::instance::{
        info::{
            CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
        },
        GroupId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};

use super::InstanceImporter;

#[derive(Debug, Default)]
pub struct LegacyGDLauncherImporter {
    results: Vec<LegacyGDLauncherConfigWrapper>,
}

#[async_trait::async_trait]
impl InstanceImporter for LegacyGDLauncherImporter {
    type Config = LegacyGDLauncherConfigWrapper;

    async fn scan(&mut self, app: Arc<AppInner>) -> anyhow::Result<()> {
        let old_gdl_base_path = directories::BaseDirs::new()
            .ok_or(anyhow::anyhow!("Cannot build basedirs"))?
            .data_dir()
            .join("gdlauncher_next")
            .join("instances");

        let mut all_instances = tokio::fs::read_dir(&old_gdl_base_path).await?;

        while let Some(child) = all_instances.next_entry().await? {
            if child.metadata().await?.is_dir() {
                let config = child.path().join("config.json");
                if !config.exists() {
                    continue;
                }

                let config = tokio::fs::read_to_string(config).await?;
                let config: LegacyGDLauncherConfig = serde_json::from_str(&config)?;

                self.results.push(LegacyGDLauncherConfigWrapper {
                    name: child.file_name().into_string().unwrap(),
                    config,
                });
            }
        }

        Ok(())
    }

    async fn get_available(&self) -> anyhow::Result<Vec<super::ImportableInstance>> {
        let mut instances = Vec::new();

        for instance in &self.results {
            instances.push(super::ImportableInstance {
                name: instance.name.clone(),
            });
        }

        Ok(instances)
    }

    async fn import(&self, app: Arc<AppInner>, index: u32) -> anyhow::Result<()> {
        let instance = self
            .results
            .get(index as usize)
            .ok_or(anyhow::anyhow!("No importable instance at index {index}"))?;

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
                false,
                instance_version_source,
                "".to_string(),
            )
            .await?;

        app.instance_manager()
            .prepare_game(created_instance_id, None)
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct LegacyGDLauncherConfigWrapper {
    name: String,
    config: LegacyGDLauncherConfig,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyGDLauncherConfig {
    loader: _Loader,
    time_played: u64,
    last_played: u64,
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
