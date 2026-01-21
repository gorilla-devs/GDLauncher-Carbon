use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{GameVersion, ModLoader, ModLoaderType, StandardVersion},
        vtask::VisualTaskId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};
use anyhow::anyhow;
use carbon_platforms::gdlauncher::manifest::schema::v1::{Manifest, ModloaderType};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;

use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry,
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    manifest: Manifest,
    /// Icon data extracted from archive (bytes, extension)
    icon_data: Option<(Vec<u8>, String)>,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.manifest.name,
        }
    }
}

#[derive(Debug)]
pub struct GdlpackImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl GdlpackImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    async fn scan_archive(
        &self,
        _app: &Arc<AppInner>,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        if !path.is_file() {
            return Ok(None);
        }

        let name = path
            .file_name()
            .expect("filename cannot be empty")
            .to_string_lossy()
            .to_string();

        let path2 = path.clone();
        let r = tokio::task::spawn_blocking(move || {
            let file =
                fs::File::open(&path2).map_err(|_| Translation::InstanceImportGdlpackMalformed)?;

            let mut zip = zip::ZipArchive::new(file)
                .map_err(|_| Translation::InstanceImportGdlpackMalformed)?;

            // Parse manifest
            let manifest = {
                let mut manifest_file = zip
                    .by_name("gdlpack.json")
                    .map_err(|_| Translation::InstanceImportGdlpackMissingManifest)?;

                let mut data = Vec::new();
                manifest_file
                    .read_to_end(&mut data)
                    .map_err(|_| Translation::InstanceImportGdlpackMalformedManifest)?;

                serde_json::from_slice::<Manifest>(&data)
                    .map_err(|_| Translation::InstanceImportGdlpackMalformedManifest)?
            };

            // Extract icon if present
            let icon_data = if let Some(icon_path) = &manifest.icon {
                if let Ok(mut icon_file) = zip.by_name(icon_path) {
                    let mut icon_bytes = Vec::new();
                    if icon_file.read_to_end(&mut icon_bytes).is_ok() {
                        let ext = std::path::Path::new(icon_path)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("png")
                            .to_string();
                        Some((icon_bytes, ext))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Ok((manifest, icon_data, path2))
        })
        .await?;

        let (manifest, icon_data, path) = match r {
            Ok(t) => t,
            Err(reason) => {
                return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                    name,
                    reason,
                })));
            }
        };

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename: name,
            path,
            manifest,
            icon_data,
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for GdlpackImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_file() {
            if let Ok(Some(entry)) = self.scan_archive(app, scan_path).await {
                self.state.write().await.set_single(entry).await;
            }
        } else if scan_path.is_dir() {
            let Ok(mut dir) = tokio::fs::read_dir(&scan_path).await else {
                return Ok(());
            };

            let mut futures = Vec::new();

            while let Some(entry) = dir.next_entry().await? {
                if entry.metadata().await?.is_file() {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase());

                    // Only scan .gdlpack files
                    if ext == Some("gdlpack".to_string()) {
                        futures.push(async move {
                            if let Ok(Some(entry)) = self.scan_archive(app, entry.path()).await {
                                self.state.write().await.push_multi(entry).await;
                            }
                        })
                    }
                }
            }

            futures::future::join_all(futures).await;
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
        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index"))?;

        // Convert GDLPack modloaders to GDLauncher format
        let modloaders: HashSet<ModLoader> = instance
            .manifest
            .dependencies
            .modloaders
            .iter()
            .map(|ml| ModLoader {
                type_: match ml.type_ {
                    ModloaderType::Forge => ModLoaderType::Forge,
                    ModloaderType::Neoforge => ModLoaderType::Neoforge,
                    ModloaderType::Fabric => ModLoaderType::Fabric,
                    ModloaderType::Quilt => ModLoaderType::Quilt,
                },
                version: ml.version.clone(),
            })
            .collect();

        let version = GameVersion::Standard(StandardVersion {
            release: instance.manifest.dependencies.minecraft.clone(),
            modloaders,
        });

        let instance_version_source = InstanceVersionSource::Version(version);

        // Set up icon from extracted data
        let icon = instance.icon_data.as_ref().map(|(bytes, ext)| {
            (format!("icon.{}", ext), bytes.clone())
        });

        let instance_path_for_copy = instance.path.clone();

        let initializer = move |instance_path: PathBuf| {
            let gdlpack_path = instance_path_for_copy;

            async move {
                let setupdir = instance_path.join(".setup");
                tokio::fs::create_dir_all(&setupdir).await?;
                // Copy the gdlpack file to .setup/gdlpack for processing during prepare_game
                tokio::fs::copy(&gdlpack_path, setupdir.join("gdlpack")).await?;

                Ok(())
            }
        };

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.manifest.name.clone()),
                icon,
                None,
                None,
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
