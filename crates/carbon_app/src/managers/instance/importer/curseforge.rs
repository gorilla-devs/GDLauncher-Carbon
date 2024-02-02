use std::path::PathBuf;
use std::sync::Arc;

use crate::api::translation::Translation;
use crate::domain::instance::info::{CurseforgeModpack, GameVersion, Modpack};
use crate::managers::instance::InstanceVersionSource;
use crate::managers::AppInner;
use crate::{domain::vtask::VisualTaskId, managers::instance::anyhow};
use tokio::sync::RwLock;
use tracing::{info, trace};

use crate::domain::modplatforms::curseforge::manifest::Manifest;

use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry, GET_IMPORT_SCAN_STATUS,
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    manifest: Manifest,
    meta: Option<CfMetadata>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CfMetadata {
    name: String,
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    #[serde(rename = "installedModpack")]
    installed_modpack: Option<InstalledModpack>,
    last_played: Option<String>,
    background: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct InstalledModpack {
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
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
pub struct CurseforgeImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl CurseforgeImporter {
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

        let mut p = p.join("curseforge");
        Ok(p)
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let config: PathBuf = path.join("minecraftinstance.json");
        if !config.is_file() {
            return Ok(None);
        }

        let manifest_path: PathBuf = path.join("manifest.json");
        if !config.is_file() {
            return Ok(None);
        }

        let config = tokio::fs::read_to_string(config).await?;
        let config = serde_json::from_str::<CfMetadata>(&config);
        let filename = path
            .file_name()
            .expect("filename cannot be empty")
            .to_string_lossy()
            .to_string();

        let manifest = tokio::fs::read_to_string(manifest_path).await?;
        let manifest = serde_json::from_str::<Manifest>(&manifest);

        let merged = manifest.map(|manifest| Importable {
            filename: filename.clone(),
            path,
            manifest,
            meta: config.ok(),
        });
        match merged {
            Ok(i) => Ok(Some(InternalImportEntry::Valid(i))),
            Err(_) => Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            }))),
        }
    }
}

#[async_trait::async_trait]
impl InstanceImporter for CurseforgeImporter {
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

        Ok(())
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
        trace!(?index, ?name, "Beginning curseforge import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let icon = match &instance.meta {
            Some(CfMetadata {
                installed_modpack: Some(installed_modpack),
                ..
            }) => Some(
                app.instance_manager()
                    .download_icon(
                        installed_modpack
                            .thumbnail_url
                            .as_ref()
                            .unwrap()
                            .to_string(),
                    )
                    .await?,
            ),
            _ => None,
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
                        Some("minecraftinstance.json" | "manifest.json" | "modelist.html") => false,
                        Some(p)
                            if Some(p)
                                == instance
                                    .meta
                                    .as_ref()
                                    .unwrap()
                                    .background
                                    .to_owned()
                                    .as_deref() =>
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

        let version = GameVersion::Standard(instance.manifest.minecraft.clone().try_into()?);

        let instance_version_source = match &instance.meta {
            Some(meta) => InstanceVersionSource::ModpackWithKnownVersion(
                version,
                Modpack::Curseforge(CurseforgeModpack {
                    project_id: meta.project_id,
                    file_id: meta.file_id,
                }),
            ),
            None => InstanceVersionSource::Version(version),
        };

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.manifest.name.clone()),
                icon,
                instance_version_source,
                String::new(),
                initializer,
            )
            .await?;

        app.instance_manager()
            .prepare_game(id, None, None, false)
            .await
            .map(|r| r.1)
    }
}
