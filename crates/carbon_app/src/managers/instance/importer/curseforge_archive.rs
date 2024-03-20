use std::{
    fs,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use anyhow::anyhow;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::{
        instance::info::{CurseforgeModpack, GameVersion, Modpack},
        modplatforms::curseforge::manifest::Manifest,
        vtask::VisualTaskId,
    },
    managers::{
        instance::InstanceVersionSource,
        modplatforms::curseforge::convert_cf_version_to_standard_version, AppInner,
    },
};

use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry,
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    manifest: Manifest,
    meta: Option<CfMetadata>,
}

#[derive(Debug, Clone)]
struct CfMetadata {
    name: String,
    project_id: u32,
    file_id: u32,
    image_url: Option<String>,
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
pub struct CurseforgeArchiveImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl CurseforgeArchiveImporter {
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
            let mut file =
                fs::File::open(path2).map_err(|_| Translation::InstanceImportMrpackMalformed)?;
            let mut zip = zip::ZipArchive::new(&mut file)
                .map_err(|_| Translation::InstanceImportCfZipMalformed)?;

            let mut manifest = zip
                .by_name("manifest.json")
                .map_err(|_| Translation::InstanceImportCfZipMissingManifest)?;

            let mut data = Vec::new();
            manifest
                .read_to_end(&mut data)
                .map_err(|_| Translation::InstanceImportCfZipMalformedManifest)?;

            let manifest = serde_json::from_slice::<Manifest>(&data)
                .map_err(|_| Translation::InstanceImportCfZipMalformedManifest)?;

            let murmur2 = murmurhash32::murmurhash2({
                // drop whitespace
                data.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
                &data
            });

            Ok((manifest, murmur2))
        })
        .await?;

        let (manifest, _murmur2) = match r {
            Ok(t) => t,
            Err(reason) => {
                return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                    name,
                    reason,
                })))
            }
        };

        if manifest.manifest_type != "minecraftModpack" {
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name,
                reason: Translation::InstanceImportCfZipNotMinecraftModpack,
            })));
        }

        // does not seem to works with packs directly downloaded from curseforge. As that's already and edge case we ignore it for now
        let meta = None;
        /*
        let fp_response = app
            .modplatforms_manager()
            .curseforge
            .get_fingerprints(&[murmur2])
            .await?
            .data;

        let fp_matches = fp_response.exact_matches;

        let meta = 'remote: {
            if fp_matches.is_empty() { break 'remote None };

            let mods_response = app.modplatforms_manager()
                .curseforge
                .get_mods(ModsParameters {
                    body: ModsParametersBody {
                        mod_ids: fp_matches
                            .iter()
                            .map(|m| m.file.mod_id)
                            .collect::<Vec<_>>(),
                    },
                })
                .await?
                .data;

            let modpack_matches = fp_response
                .exact_fingerprints
                .into_iter()
                .zip(fp_matches.into_iter())
                .zip(mods_response.into_iter())
                .map(|((fingerprint, fp_match), proj)| (fingerprint, fp_match, proj))
                .collect::<Vec<_>>();

            modpack_matches.into_iter().next().map(|(_, fp_match, proj)| {
                CfMetadata {
                    name: proj.name,
                    project_id: fp_match.file.mod_id as u32,
                    file_id: fp_match.file.id as u32,
                    image_url: proj.logo.map(|logo| logo.url),
                }
            })
        };
        */

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename: name,
            path,
            manifest,
            meta,
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for CurseforgeArchiveImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_file() {
            if let Ok(Some(entry)) = self.scan_archive(app, scan_path).await {
                self.state.write().await.set_single(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
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
                            app.invalidate(GET_IMPORT_SCAN_STATUS, None);
                        }
                    })
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
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        info!("Importing target {index} - '{}'", instance.manifest.name);

        let dummy_string = daedalus::BRANDING
            .get_or_init(daedalus::Branding::default)
            .dummy_replace_string
            .clone();

        let standard_version = convert_cf_version_to_standard_version(
            app.clone(),
            instance.manifest.minecraft.clone(),
            dummy_string,
        )
        .await?;

        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.meta {
            Some(meta) => InstanceVersionSource::ModpackWithKnownVersion(
                version,
                Modpack::Curseforge(CurseforgeModpack {
                    project_id: meta.project_id,
                    file_id: meta.file_id,
                }),
                true,
            ),
            None => InstanceVersionSource::Version(version),
        };

        let icon = match &instance.meta {
            Some(CfMetadata {
                image_url: Some(image_url),
                ..
            }) => Some(
                app.instance_manager()
                    .download_icon(image_url.clone())
                    .await?,
            ),
            _ => None,
        };

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;

            async move {
                let setupdir = instance_path.join(".setup");
                tokio::fs::create_dir_all(&setupdir).await?;
                tokio::fs::copy(&instance.path, setupdir.join("curseforge")).await?;

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
