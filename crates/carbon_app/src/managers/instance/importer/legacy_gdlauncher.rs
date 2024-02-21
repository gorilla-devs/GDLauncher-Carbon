use std::{collections::HashSet, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use tokio::sync::RwLock;
use tracing::trace;

use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::{
        instance::info::{
            CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
        },
        vtask::VisualTaskId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};

use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry,
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    config: LegacyGDLauncherConfig,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename.clone(),
            instance_name: value.filename,
        }
    }
}

#[derive(Debug)]
pub struct LegacyGDLauncherImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl LegacyGDLauncherImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<PathBuf> {
        let basedirs = directories::BaseDirs::new().ok_or(anyhow!("Cannot build basedirs"))?;

        // old gdl did not respect the xdg basedirs spec on linux
        #[cfg(target_os = "linux")]
        let p = basedirs.config_dir();
        #[cfg(not(target_os = "linux"))]
        let p = basedirs.data_dir();

        let mut p = p.join("gdlauncher_next");

        let override_path = p.join("override.data");
        if override_path.exists() {
            let override_path = tokio::fs::read_to_string(override_path).await;
            if let Ok(override_path) = override_path {
                let override_path = PathBuf::from(override_path);
                if override_path.is_dir() {
                    p = override_path;
                }
            }
        }

        Ok(p.join("instances"))
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let config = path.join("config.json");
        if !config.is_file() {
            return Ok(None);
        }

        let config = tokio::fs::read_to_string(config).await?;
        let config = serde_json::from_str::<LegacyGDLauncherConfig>(&config);
        let filename = path
            .file_name()
            .expect("filename cannot be empty")
            .to_string_lossy()
            .to_string();

        match config {
            Ok(config) => Ok(Some(InternalImportEntry::Valid(Importable {
                filename,
                path,
                config,
            }))),
            Err(_) => Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            }))),
        }
    }
}

#[async_trait::async_trait]
impl InstanceImporter for LegacyGDLauncherImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_dir() {
            let Ok(mut dir) = tokio::fs::read_dir(&scan_path).await else {
                return Ok(());
            };

            while let Some(path) = dir.next_entry().await? {
                if path.metadata().await?.is_dir() {
                    if let Ok(Some(entry)) = self.scan_instance(path.path()).await {
                        self.state.write().await.push_multi(entry).await;
                        app.invalidate(GET_IMPORT_SCAN_STATUS, None);
                    }
                }
            }
        } else if let Ok(Some(entry)) = self.scan_instance(scan_path).await {
            self.state.write().await.set_single(entry).await;
            app.invalidate(GET_IMPORT_SCAN_STATUS, None);
        }

        Ok(()) // TODO: invalidate on iter
    }

    async fn get_status(&self) -> ImportScanStatus {
        self.state.read().await.clone().into()
    }

    async fn begin_import(
        &self,
        app: &Arc<AppInner>,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        trace!(?index, ?name, "Beginning legacy gdl import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index"))?;

        let instance_version_source = 'version_source: {
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

            'cf: {
                if let Some(ref source) = instance.config.loader.source {
                    if source != "curseforge" {
                        break 'cf;
                    }

                    let Some(project_id) = instance.config.loader.project_id else {
                        trace!("Legacy gdl import specifies curseforge source but is missing project id, ignoring");
                        break 'cf;
                    };
                    let Some(file_id) = instance.config.loader.file_id else {
                        trace!("Legacy gdl import specifies curseforge source but is missing file id, ignoring");
                        break 'cf;
                    };

                    let curseforge_modpack = CurseforgeModpack {
                        project_id: project_id as u32,
                        file_id: file_id as u32,
                    };

                    break 'version_source InstanceVersionSource::ModpackWithKnownVersion(
                        standard_version,
                        Modpack::Curseforge(curseforge_modpack),
                    );
                }
            }

            InstanceVersionSource::Version(standard_version)
        };

        let icon = match &instance.config.background {
            Some(background) => app
                .instance_manager()
                .load_icon(instance.path.join(background))
                .await
                .ok(),
            None => None,
        };

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                let path = instance_path.join("instance");

                tokio::fs::create_dir_all(instance_path.join(".setup").join("modpack-complete"))
                    .await?;

                trace!("Copying files from legacy instance");
                // create copy-filter function in file utils for all importers
                crate::domain::runtime_path::copy_dir_filter(&instance.path, &path, |path| {
                    match path.to_str() {
                        Some("config.json" | "manifest.json" | "installing.lock" | "natives") => {
                            false
                        }
                        Some(p)
                            if Some(p)
                                == instance.config.background.as_ref().map(|x| x.as_str()) =>
                        {
                            false
                        }
                        _ => true,
                    }
                })
                .await?;

                Ok(())
            }
        };

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.filename.clone()),
                icon,
                instance_version_source,
                String::new(),
                initializer,
            )
            .await?;

        app.instance_manager()
            .prepare_game(id, None, None, true)
            .await
            .map(|r| r.1)
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
