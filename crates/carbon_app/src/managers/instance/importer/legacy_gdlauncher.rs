use std::{collections::HashSet, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use tokio::sync::RwLock;

use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{
            CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
        },
        vtask::VisualTaskId,
    },
    managers::{
        instance::InstanceVersionSource,
        AppInner,
    },
};

use super::{InstanceImporter, ImportScanStatus, ImportableInstance, InvalidImportEntry};

#[derive(Debug)]
enum State {
    None,
    Single(ImportEntry),
    Multi(Vec<ImportEntry>),
}

#[derive(Debug, Clone)]
enum ImportEntry {
    Valid(Importable),
    Invalid(InvalidImportEntry),
}

#[derive(Debug, Clone)]
struct Importable {
    name: String,
    path: PathBuf,
    config: LegacyGDLauncherConfig,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            name: value.name,
        }
    }
}

impl From<ImportEntry> for super::ImportEntry {
    fn from(value: ImportEntry) -> Self {
        match value {
            ImportEntry::Valid(v) => Self::Valid(v.into()),
            ImportEntry::Invalid(v) => Self::Invalid(v),
        }
    }
}

#[derive(Debug)]
pub struct LegacyGDLauncherImporter {
    state: RwLock<State>,
}

impl LegacyGDLauncherImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(State::None),
        }
    }

    async fn scan_instance(&self, path: PathBuf) -> anyhow::Result<Option<ImportEntry>> {
        let config = path.join("config.json");
        if !config.is_file() {
            return Ok(None)
        }

        let config = tokio::fs::read_to_string(config).await?;
        let config = serde_json::from_str::<LegacyGDLauncherConfig>(&config);
        let name = path.file_name().expect("filename cannot be empty").to_string_lossy().to_string();

        match config {
            Ok(config) => {
                Ok(Some(ImportEntry::Valid(Importable {
                    name,
                    path,
                    config,
                })))
            },
            Err(_) => {
                Ok(Some(ImportEntry::Invalid(InvalidImportEntry {
                    name,
                    reason: Translation::InstanceImportLegacyBadConfigFile,
                })))
            }
        }
    }
}

#[async_trait::async_trait]
impl InstanceImporter for LegacyGDLauncherImporter {
    async fn scan(&self, _app: &Arc<AppInner>, mut scan_path: PathBuf) -> anyhow::Result<()> {
        if !scan_path.is_dir() {
            return Ok(())
        }

        let override_path = scan_path.join("override.data");

        if override_path.exists() {
            let override_path = tokio::fs::read_to_string(override_path).await;

            if let Ok(override_path) = override_path {
                let override_path = PathBuf::from(override_path);

                if override_path.is_dir() {
                    scan_path = override_path;
                }
            }
        }

        let instances_path = scan_path.join("instances");

        if instances_path.is_dir() {
            let Ok(mut instances) = tokio::fs::read_dir(&instances_path).await else {
                return Ok(())
            };

            while let Some(instance) = instances.next_entry().await? {
                if instance.metadata().await?.is_dir() {
                    if let Ok(Some(entity)) = self.scan_instance(instance.path()).await {
                        let mut state = self.state.write().await;

                        match &mut *state {
                            State::Multi(vec) => vec.push(entity),
                            state => *state = State::Multi(vec![entity]),
                        }
                    }
                }
            }
        } else {
            if let Ok(Some(entity)) = self.scan_instance(scan_path).await {
                *self.state.write().await = State::Single(entity);
            }
        }

        Ok(()) // TODO: invalidate on iter
    }

    async fn get_default_scan_path(&self, _app: &Arc<AppInner>) -> anyhow::Result<Option<PathBuf>> {
        Ok(Some(
            directories::BaseDirs::new()
                .ok_or(anyhow!("Cannot build basedirs"))?
                .data_dir()
                .join("gdlauncher_next"),
        ))
    }

    async fn get_status(&self) -> ImportScanStatus {
        match &*self.state.read().await {
            State::None => ImportScanStatus::NoResults,
            State::Single(result) => ImportScanStatus::SingleResult(result.clone().into()),
            State::Multi(results) => ImportScanStatus::MultiResult(results.iter().map(|r| r.clone().into()).collect()),
        }
    }

    async fn begin_import(&self, app: &Arc<AppInner>, index: u32) -> anyhow::Result<VisualTaskId> {
        let instance = match &*self.state.read().await {
            State::Single(ImportEntry::Valid(entry)) if index == 0 => Some(entry.clone()),
            State::Multi(entries) => entries.get(index as usize).map(|r| match r {
                ImportEntry::Valid(entry) => Some(entry.clone()),
                _ => None,
            }).flatten(),
            _ => None,
        };

        let instance = instance.ok_or_else(|| anyhow!("invalid importable instance index"))?;

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
                        return Err(anyhow!("Missing project id"));

                };
                let Some(file_id) = instance.config.loader.file_id else {
                        return Err(anyhow!("Missing file id"));
                };

                let curseforge_modpack = CurseforgeModpack {
                    project_id: project_id as u32,
                    file_id: file_id as u32,
                };

                break 'a InstanceVersionSource::ModpackWithKnownVersion(standard_version, Modpack::Curseforge(curseforge_modpack));
            } else {
                break 'a InstanceVersionSource::Version(standard_version);
            }
        };

        if let Some(ref background) = instance.config.background {
            app.instance_manager()
                .load_icon(instance.path.join(background))
                .await?;
        }

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                let path = instance_path.join("instance");
                tokio::fs::write(instance_path.join(".first_run_incomplete"), "skip-modpack-init").await?;

                // create copy-filter function in file utils for all importers
                crate::domain::runtime_path::copy_dir_filter(
                    &instance.path,
                    &path,
                    |path| match path.to_str() {
                        Some(
                            | "config.json"
                            | "manifest.json"
                            | "installing.lock"
                            | "natives"
                        ) => false,
                        Some(p) if Some(p) == instance.config.background.as_ref().map(|x| x.as_str()) => false,
                        _ => true,
                    },
                ).await?;

                Ok(())
            }
        };

        let id = app.instance_manager().create_instance_ext(
            app.instance_manager().get_default_group().await?,
            instance.name.clone(),
            instance.config.background.is_some(),
            instance_version_source,
            String::new(),
            initializer,
        ).await?;

        app.instance_manager().prepare_game(id, None, None).await.map(|r| r.1)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyGDLauncherConfig {
    loader: _Loader,
    time_played: u64,
    last_played: Option<u64>,
    background: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct _Loader {
    loader_type: String,
    loader_version: Option<String>,
    mc_version: String,
    #[serde(rename = "fileID")]
    file_id: Option<i32>,
    #[serde(rename = "projectID")]
    project_id: Option<i32>,
    source: Option<String>,
    source_name: Option<String>,
}
