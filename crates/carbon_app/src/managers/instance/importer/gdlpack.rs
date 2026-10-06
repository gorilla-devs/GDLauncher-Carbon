use crate::{
    domain::{
        instance::info::{GameVersion, ModLoader, ModLoaderType, StandardVersion},
        vtask::VisualTaskId,
    },
    managers::minecraft::gdlpack::archive::{AdmittedArchive, inspect},
    managers::{AppInner, instance::InstanceVersionSource},
};
use anyhow::anyhow;
use carbon_platforms::gdlauncher::manifest::schema::v1::{Manifest, ModloaderType};
use std::{collections::HashSet, path::PathBuf, sync::Arc};
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
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.manifest.name,
            warnings: Vec::new(),
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
            inspect(&path2).map(|(manifest, _)| (manifest, path2))
        })
        .await?;

        let (manifest, path) = match r {
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
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for GdlpackImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_file() {
            if let Some(entry) = self.scan_archive(app, scan_path).await? {
                self.state.write().await.set_single(entry).await;
            }
        } else if scan_path.is_dir() {
            let mut dir = super::common::scan_directory(&scan_path).await?;

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
                            if let Some(entry) = self.scan_archive(app, entry.path()).await? {
                                self.state.write().await.push_multi(entry).await;
                            }
                            Ok::<_, anyhow::Error>(())
                        })
                    }
                }
            }

            for result in futures::future::join_all(futures).await {
                result?;
            }
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

        let admitted = AdmittedArchive::capture(
            instance.path.clone(),
            app.settings_manager().runtime_path.get_temp().to_path(),
        )
        .await?;

        // Convert GDLPack modloaders to GDLauncher format
        let modloaders: HashSet<ModLoader> = admitted
            .manifest()
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
            release: admitted.manifest().dependencies.minecraft.clone(),
            modloaders,
        });

        let instance_version_source = InstanceVersionSource::Version(version);

        // Set up icon from extracted data
        let icon = admitted
            .icon()
            .as_ref()
            .map(|(bytes, ext)| (format!("icon.{}", ext), bytes.clone()));

        let pack_name = admitted.manifest().name.clone();
        let initializer =
            move |instance_path: PathBuf| admitted.install(instance_path.join(".setup"));

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or(pack_name),
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
