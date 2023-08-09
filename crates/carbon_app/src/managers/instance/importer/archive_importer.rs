use crate::{
    api::{instance::import::FEEntity, keys},
    domain::{
        instance::info::{CurseforgeModpack, Modpack, ModrinthModpack},
        modplatforms::{
            curseforge::{
                self,
                filters::{ModsParameters, ModsParametersBody},
            },
            modrinth::{self, search::ProjectID},
        },
        vtask::VisualTaskId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};

use anyhow::anyhow;
use sha2::{digest::Digest, Sha512};
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::domain::modplatforms::modrinth::{search::VersionHashesQuery, version::HashAlgorithm};

use super::InstanceImporter;

#[derive(Debug)]
pub struct CurseForgeManifestWrapper {
    full_path: PathBuf,
    manifest: curseforge::manifest::Manifest,
}

#[derive(Debug, Default)]
pub struct CurseForgeZipImporter {
    scan_result: Mutex<Option<CurseForgeManifestWrapper>>,
}

#[derive(Debug)]
pub struct MrpackIndexWrapper {
    full_path: PathBuf,
    index: modrinth::version::ModpackIndex,
}

#[derive(Debug, Default)]
pub struct MrpackImporter {
    scan_result: Mutex<Option<MrpackIndexWrapper>>,
}

#[async_trait::async_trait]
impl InstanceImporter for CurseForgeZipImporter {
    type Config = CurseForgeManifestWrapper;

    async fn scan(&mut self, app: Arc<AppInner>, path: Option<PathBuf>) -> anyhow::Result<()> {
        let Some(path) = path else {
            return Err(anyhow!("No scan path provided. Scan path required for CurseForge Zip Importer"));
        };

        let file_path_clone = path.clone();

        // make sure this is a valid modpack
        let manifest = spawn_blocking(move || {
            let file = std::fs::File::open(file_path_clone)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let manifest: curseforge::manifest::Manifest = {
                let file = archive.by_name("manifest.json")?;
                serde_json::from_reader(file)?
            };

            Ok::<_, anyhow::Error>(manifest)
        })
        .await??;

        if manifest.manifest_type != "minecraftModpack" {
            return Err(anyhow::anyhow!(format!(
                "Invalid manifest type `{}`",
                &manifest.manifest_type
            )));
        }

        let mut lock = self.scan_result.lock().await;
        *lock = Some(CurseForgeManifestWrapper {
            full_path: path.clone(),
            manifest,
        });
        app.invalidate(
            keys::instance::GET_IMPORTABLE_INSTANCES,
            Some(serde_json::to_value(FEEntity::CurseForgeZip(
                path.to_string_lossy().to_string(),
            ))?),
        );

        Ok(())
    }

    async fn get_available(&self) -> anyhow::Result<Vec<super::ImportableInstance>> {
        let instances = self
            .scan_result
            .lock()
            .await
            .as_ref()
            .map_or_else(Vec::new, |instance| {
                vec![super::ImportableInstance {
                    name: instance.manifest.name.clone(),
                }]
            });
        Ok(instances)
    }

    async fn import(
        &self,
        app: Arc<AppInner>,
        _index: u32,
        name: &str,
    ) -> anyhow::Result<VisualTaskId> {
        let lock = self.scan_result.lock().await;
        let instance = lock
            .as_ref()
            .ok_or(anyhow!("No importable instance available"))?;

        let mut content = tokio::fs::read(&instance.full_path).await?;
        let murmur2 = tokio::task::spawn_blocking(move || {
            murmurhash32::murmurhash2({
                // curseforge's weird api
                content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
                &content
            })
        })
        .await?;
        let fp_response = app
            .modplatforms_manager()
            .curseforge
            .get_fingerprints(&[murmur2])
            .await?
            .data;
        let mods_response = app
            .modplatforms_manager()
            .curseforge
            .get_mods(ModsParameters {
                body: ModsParametersBody {
                    mod_ids: fp_response
                        .exact_matches
                        .iter()
                        .map(|m| m.file.mod_id)
                        .collect::<Vec<_>>(),
                },
            })
            .await?
            .data;
        let matches = fp_response
            .exact_fingerprints
            .into_iter()
            .zip(fp_response.exact_matches.into_iter())
            .zip(mods_response.into_iter())
            .map(|((fingerprint, fp_match), proj)| (fingerprint, fp_match, proj))
            .collect::<Vec<_>>();
        let (modpack, icon) = matches
            .first()
            .map(|(_, fp_match, proj)| {
                (
                    Modpack::Curseforge(CurseforgeModpack::LocalManaged {
                        project_id: fp_match.file.mod_id,
                        file_id: fp_match.file.id,
                        archive_path: instance.full_path.to_string_lossy().to_string(),
                    }),
                    proj.logo.as_ref().map(|logo| logo.url.clone()),
                )
            })
            .unwrap_or_else(|| {
                (
                    Modpack::Curseforge(CurseforgeModpack::Unmanaged {
                        archive_path: instance.full_path.to_string_lossy().to_string(),
                    }),
                    None,
                )
            });

        if let Some(icon_url) = &icon {
            app.instance_manager()
                .download_icon(icon_url.clone())
                .await?;
        }

        let install_source = InstanceVersionSource::Modpack(modpack);

        let created_instance_id = app
            .instance_manager()
            .create_instance(
                app.instance_manager().get_default_group().await?,
                name.to_string(),
                icon.is_some(),
                install_source,
                "".to_string(),
            )
            .await?;

        let (_, task_id) = app
            .instance_manager()
            .prepare_game(created_instance_id, None, None)
            .await?;

        Ok(task_id)
    }
}

#[async_trait::async_trait]
impl InstanceImporter for MrpackImporter {
    type Config = MrpackIndexWrapper;

    async fn scan(&mut self, app: Arc<AppInner>, path: Option<PathBuf>) -> anyhow::Result<()> {
        let Some(path) = path else {
            return Err(anyhow!("No scan path provided. Scan path required for CurseForge Zip Importer"));
        };

        let file_path_clone = path.clone();

        // make sure this is a valid modpack
        let index = spawn_blocking(move || {
            let file = std::fs::File::open(file_path_clone)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let index: modrinth::version::ModpackIndex = {
                let file = archive.by_name("modrinth.index.json")?;
                serde_json::from_reader(file)?
            };

            Ok::<_, anyhow::Error>(index)
        })
        .await??;

        let mut lock = self.scan_result.lock().await;
        *lock = Some(MrpackIndexWrapper {
            full_path: path.clone(),
            index,
        });
        app.invalidate(
            keys::instance::GET_IMPORTABLE_INSTANCES,
            Some(serde_json::to_value(FEEntity::MRPack(
                path.to_string_lossy().to_string(),
            ))?),
        );

        Ok(())
    }

    async fn get_available(&self) -> anyhow::Result<Vec<super::ImportableInstance>> {
        let instances = self
            .scan_result
            .lock()
            .await
            .as_ref()
            .map_or_else(Vec::new, |instance| {
                vec![super::ImportableInstance {
                    name: instance.index.name.clone(),
                }]
            });
        Ok(instances)
    }

    async fn import(
        &self,
        app: Arc<AppInner>,
        _index: u32,
        name: &str,
    ) -> anyhow::Result<VisualTaskId> {
        let lock = self.scan_result.lock().await;
        let instance = lock
            .as_ref()
            .ok_or(anyhow!("No importable instance available"))?;
        let content = tokio::fs::read(&instance.full_path).await?;
        let sha512 = tokio::task::spawn_blocking(move || {
            hex::encode(<[u8; 64] as From<_>>::from(
                Sha512::new_with_prefix(&content).finalize(),
            ))
        })
        .await?;

        let version_response = app
            .modplatforms_manager()
            .modrinth
            .get_versions_from_hash(&VersionHashesQuery {
                hashes: vec![sha512.clone()],
                algorithm: HashAlgorithm::SHA512,
            })
            .await?;

        let (modpack, icon) = match version_response.get(&sha512) {
            Some(version) => {
                let project = app
                    .modplatforms_manager()
                    .modrinth
                    .get_project(ProjectID(version.project_id.clone()))
                    .await?;
                (
                    Modpack::Modrinth(ModrinthModpack::LocalManaged {
                        project_id: project.id.clone(),
                        version_id: version.id.clone(),
                        mrpack_path: instance.full_path.to_string_lossy().to_string(),
                    }),
                    project.icon_url,
                )
            }
            None => (
                Modpack::Modrinth(ModrinthModpack::Unmanaged {
                    mrpack_path: instance.full_path.to_string_lossy().to_string(),
                }),
                None,
            ),
        };

        if let Some(icon_url) = &icon {
            app.instance_manager()
                .download_icon(icon_url.clone())
                .await?;
        }

        let install_source = InstanceVersionSource::Modpack(modpack);

        let created_instance_id = app
            .instance_manager()
            .create_instance(
                app.instance_manager().get_default_group().await?,
                name.to_string(),
                icon.is_some(),
                install_source,
                "".to_string(),
            )
            .await?;

        let (_, task_id) = app
            .instance_manager()
            .prepare_game(created_instance_id, None, None)
            .await?;
        Ok(task_id)
    }
}
