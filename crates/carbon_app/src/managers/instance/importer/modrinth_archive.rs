use std::{
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use anyhow::anyhow;
use sha2::{Digest, Sha512};
use tokio::sync::RwLock;

use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{GameVersion, Modpack, ModrinthModpack},
        modplatforms::modrinth::{
            search::{ProjectID, VersionHashesQuery},
            version::{HashAlgorithm, ModpackIndex},
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
    index: ModpackIndex,
    meta: Option<MrMetadata>,
}

#[derive(Debug, Clone)]
struct MrMetadata {
    name: String,
    project_id: String,
    version_id: String,
    image_url: Option<String>,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.index.name,
        }
    }
}

#[derive(Debug)]
pub struct ModrinthArchiveImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl ModrinthArchiveImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    async fn scan_archive(
        &self,
        app: &Arc<AppInner>,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        if !path.is_file() {
            return Ok(None);
        }

        let content = tokio::fs::read(&path).await?;

        let name = path
            .file_name()
            .expect("filename cannot be empty")
            .to_string_lossy()
            .to_string();

        let r = tokio::task::spawn_blocking(move || {
            let mut zip = zip::ZipArchive::new(Cursor::new(&content))
                .map_err(|_| Translation::InstanceImportMrpackMalformed)?;

            let mut manifest = zip
                .by_name("modrinth.index.json")
                .map_err(|_| Translation::InstanceImportMrpackMissingManifest)?;

            let mut data = Vec::new();
            manifest
                .read_to_end(&mut data)
                .map_err(|_| Translation::InstanceImportMrpackMalformedManifest)?;

            let manifest = serde_json::from_slice::<ModpackIndex>(&data)
                .map_err(|_| Translation::InstanceImportMrpackMalformedManifest)?;

            let sha512 = hex::encode(<[u8; 64] as From<_>>::from(
                Sha512::new_with_prefix(&content).finalize(),
            ));

            Ok((manifest, sha512))
        })
        .await?;

        let (index, sha512) = match r {
            Ok(t) => t,
            Err(reason) => {
                return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                    name,
                    reason,
                })))
            }
        };

        let version_response = app
            .modplatforms_manager()
            .modrinth
            .get_versions_from_hash(&VersionHashesQuery {
                hashes: vec![sha512.clone()],
                algorithm: HashAlgorithm::SHA512,
            })
            .await?;

        let meta = 'remote: {
            let Some(version) = version_response.get(&sha512) else {
                break 'remote None;
            };

            let project = app
                .modplatforms_manager()
                .modrinth
                .get_project(ProjectID(version.project_id.clone()))
                .await?;

            Some(MrMetadata {
                name: project.title,
                project_id: project.id,
                version_id: version.id.clone(),
                image_url: project.icon_url,
            })
        };

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename: name,
            path,
            index,
            meta,
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for ModrinthArchiveImporter {
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
                    futures.push(async move {
                        if let Ok(Some(entry)) = self.scan_archive(app, entry.path()).await {
                            self.state.write().await.push_multi(entry).await;
                        }
                    })
                }
            }
        }

        Ok(())
    }

    async fn get_default_scan_path(&self, _app: &Arc<AppInner>) -> anyhow::Result<Option<PathBuf>> {
        Ok(None)
    }

    async fn get_status(&self) -> ImportScanStatus {
        self.state.read().await.clone().into()
    }

    async fn begin_import(&self, app: &Arc<AppInner>, index: u32) -> anyhow::Result<VisualTaskId> {
        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index"))?;

        let version = GameVersion::Standard(instance.index.dependencies.clone().try_into()?);

        let instance_version_source = match &instance.meta {
            Some(meta) => InstanceVersionSource::ModpackWithKnownVersion(
                version,
                Modpack::Modrinth(ModrinthModpack {
                    project_id: meta.project_id.clone(),
                    version_id: meta.version_id.clone(),
                }),
            ),
            None => InstanceVersionSource::Version(version),
        };

        let mut use_icon = false;
        if let Some(MrMetadata {
            image_url: Some(ref image_url),
            ..
        }) = &instance.meta
        {
            app.instance_manager()
                .download_icon(image_url.clone())
                .await?;

            use_icon = true;
        }

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;

            async move {
                let setupdir = instance_path.join(".setup");
                tokio::fs::create_dir_all(&setupdir).await?;
                tokio::fs::copy(&instance.path, setupdir.join("modrinth")).await?;

                Ok(())
            }
        };

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                instance.index.name.clone(),
                use_icon,
                instance_version_source,
                String::new(),
                initializer,
            )
            .await?;

        app.instance_manager()
            .prepare_game(id, None, None)
            .await
            .map(|r| r.1)
    }
}
