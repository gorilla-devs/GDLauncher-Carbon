use std::path::PathBuf;
use std::sync::Arc;

use crate::api::translation::Translation;
use crate::domain::instance::info::{CurseforgeModpack, GameVersion, Modpack};
use crate::managers::instance::InstanceVersionSource;
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::AppInner;
use crate::{domain::vtask::VisualTaskId, managers::instance::anyhow};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, trace};

use crate::domain::modplatforms::curseforge::manifest::{Manifest, Minecraft, ModLoaders};

use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry, GET_IMPORT_SCAN_STATUS,
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    manifest: Option<Manifest>,
    meta: CfMetadata,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMetadata {
    name: String,
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    installed_modpack: Option<InstalledModpack>,
    base_mod_loader: Option<BaseModLoader>,
    game_version: String,
    last_played: Option<String>,
    is_unlocked: bool,
    profile_image_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct InstalledModpack {
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BaseModLoader {
    name: String,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.meta.name,
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
        let userdirs = directories::UserDirs::new().ok_or(anyhow!("Cannot build basedirs"))?;

        #[cfg(not(target_os = "windows"))]
        let p = userdirs.document_dir();
        #[cfg(target_os = "windows")]
        let p = userdirs.home_dir();

        Ok(p.join("curseforge").join("minecraft").join("Instances"))
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

        let config = tokio::fs::read_to_string(config).await?;
        let config = serde_json::from_str::<CfMetadata>(&config);

        let filename = path
            .file_name()
            .expect("filename cannot be empty")
            .to_string_lossy()
            .to_string();

        let manifest = tokio::fs::read_to_string(manifest_path)
            .await
            .ok()
            .map(|text| serde_json::from_str::<Manifest>(&text).ok())
            .flatten();

        let merged = config.map(|config| Importable {
            filename: filename.clone(),
            path,
            manifest,
            meta: config,
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
            CfMetadata {
                profile_image_path: Some(image),
                ..
            } => app
                .instance_manager()
                .load_icon(PathBuf::from(image.clone()))
                .await
                .ok(),
            CfMetadata {
                installed_modpack: Some(installed_modpack),
                ..
            } => Some(
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
                        Some(p) if p.starts_with("profileImage") => false,
                        _ => true,
                    }
                })
                .await?;

                Ok(())
            }
        };

        let dummy_string = daedalus::BRANDING
            .get_or_init(daedalus::Branding::default)
            .dummy_replace_string
            .clone();

        let standard_version = convert_cf_version_to_standard_version(
            app.clone(),
            Minecraft {
                version: instance.meta.game_version.clone(),
                mod_loaders: instance
                    .meta
                    .base_mod_loader
                    .as_ref()
                    .map(|loader| ModLoaders {
                        id: {
                            let mut name = &loader.name as &str;

                            if name.starts_with("fabric") || name.starts_with("quilt") {
                                if let Some((name2, _)) = name.rsplit_once('-') {
                                    name = name2;
                                }
                            }

                            dbg!(name).to_string()
                        },
                        primary: true,
                    })
                    .into_iter()
                    .collect(),
            },
            dummy_string,
        )
        .await?;

        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.manifest {
            Some(manifest) => InstanceVersionSource::ModpackWithKnownVersion(
                version,
                Modpack::Curseforge(CurseforgeModpack {
                    project_id: instance.meta.project_id,
                    file_id: instance.meta.file_id,
                }),
                instance.meta.is_unlocked,
            ),
            None => InstanceVersionSource::Version(version),
        };

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.meta.name.clone()),
                icon,
                None,
                None,
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
