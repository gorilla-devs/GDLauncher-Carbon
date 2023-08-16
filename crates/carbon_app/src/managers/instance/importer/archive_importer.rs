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

use anyhow::Context;
use sha2::{digest::Digest, Sha512};
use std::{
    io::{Read, Seek},
    path::PathBuf,
    sync::Arc,
};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::domain::modplatforms::modrinth::{search::VersionHashesQuery, version::HashAlgorithm};

use super::{Entity, InstanceImporter};

#[derive(Debug, Clone)]
pub enum CurseForgeImportedModpack {
    Managed {
        project_id: u32,
        file_id: u32,
        archive_path: PathBuf,
        icon: Option<String>,
        name: String,
        file_name: String,
    },
    Unmanaged {
        archive_path: PathBuf,
    },
}

impl From<CurseForgeImportedModpack> for Modpack {
    fn from(value: CurseForgeImportedModpack) -> Self {
        match value {
            CurseForgeImportedModpack::Managed {
                project_id,
                file_id,
                archive_path,
                ..
            } => Modpack::Curseforge(CurseforgeModpack::LocalManaged {
                project_id,
                file_id,
                archive_path: archive_path.to_string_lossy().to_string(),
            }),
            CurseForgeImportedModpack::Unmanaged { archive_path } => {
                Modpack::Curseforge(CurseforgeModpack::Unmanaged {
                    archive_path: archive_path.to_string_lossy().to_string(),
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct CurseForgeManifestWrapper {
    full_path: PathBuf,
    manifest: curseforge::manifest::Manifest,
    modpack: CurseForgeImportedModpack,
    icon: Option<String>,
}

#[derive(Debug, Default)]
pub struct CurseForgeZipImporter {
    results: Mutex<Vec<CurseForgeManifestWrapper>>,
}

#[derive(Debug)]
pub struct MrpackIndexWrapper {
    full_path: PathBuf,
    index: modrinth::version::ModpackIndex,
}

#[derive(Debug, Default)]
pub struct MrpackImporter {
    results: Mutex<Vec<MrpackIndexWrapper>>,
}

#[async_trait::async_trait]
impl InstanceImporter for CurseForgeZipImporter {
    type Config = CurseForgeManifestWrapper;

    async fn scan(&mut self, app: Arc<AppInner>, scan_paths: Vec<PathBuf>) -> anyhow::Result<()> {
        if !scan_paths.is_empty() {
            self.results.lock().await.clear();
            app.invalidate(
                keys::instance::GET_IMPORTABLE_INSTANCES,
                Some(serde_json::to_value(FEEntity::CurseForgeZip)?),
            );
        }
        for path in scan_paths {
            let file_path_clone = path.clone();

            // make sure this is a valid modpack and check if it's known by curseforge
            let (manifest, murmur2, sha1_hash) = spawn_blocking(move || {
                let mut file = std::fs::File::open(&file_path_clone).with_context(|| {
                    format!("Error reading `{}`", file_path_clone.to_string_lossy())
                })?;

                let (sha1_hash, archive_murmur2) = {
                    let mut content = Vec::new();
                    let _archive_size = file.read_to_end(&mut content).with_context(|| {
                        format!(
                            "Error reading archive `{}`",
                            file_path_clone.to_string_lossy()
                        )
                    })?;
                    file.rewind().with_context(|| {
                        format!(
                            "Error reading archive `{}`",
                            file_path_clone.to_string_lossy()
                        )
                    })?;
                    use sha1::Digest;
                    let mut hasher = sha1::Sha1::new();
                    hasher.update(&content);
                    let hash = hasher.finalize();
                    let sha1_hash = hex::encode(hash);
                    let archive_murmur2 = murmurhash32::murmurhash2({
                        // drop whitespace
                        content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
                        &content
                    });
                    (sha1_hash, archive_murmur2)
                };

                let mut archive = zip::ZipArchive::new(file).with_context(|| {
                    format!(
                        "Error reading archive `{}`",
                        file_path_clone.to_string_lossy()
                    )
                })?;

                let (manifest, murmur2) = {
                    let mut file = archive.by_name("manifest.json").with_context(|| {
                        format!(
                            "Error reading `manifest.json` from `{}`",
                            file_path_clone.to_string_lossy()
                        )
                    })?;
                    let mut content = Vec::new();
                    let _size = file.read_to_end(&mut content)?;
                    let manifest: curseforge::manifest::Manifest = serde_json::from_slice(&content)
                        .with_context(|| {
                            format!(
                                "Error parsing `manifest.json` from `{}`",
                                file_path_clone.to_string_lossy()
                            )
                        })?;
                    let murmur2 = murmurhash32::murmurhash2({
                        // drop whitespace
                        // content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
                        &content
                    });
                    tracing::debug!(
                        "`{}/manifest.json` murmur2 hash: `{}`",
                        file_path_clone.to_string_lossy(),
                        &murmur2
                    );
                    (manifest, murmur2)
                };

                Ok::<_, anyhow::Error>((manifest, murmur2, sha1_hash))
            })
            .await??;

            if manifest.manifest_type != "minecraftModpack" {
                return Err(anyhow::anyhow!(format!(
                    "Invalid manifest type `{}`",
                    &manifest.manifest_type
                )));
            }

            let fp_response = app
                .modplatforms_manager()
                .curseforge
                .get_fingerprints(&[murmur2])
                .await?
                .data;
            let fp_matches = fp_response.exact_matches;
            let mods_response = if !fp_matches.is_empty() {
                Some(
                    app.modplatforms_manager()
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
                        .data,
                )
            } else {
                None
            };
            let modpack_matches = if let Some(mods_response) = mods_response {
                Some(
                    fp_response
                        .exact_fingerprints
                        .into_iter()
                        .zip(fp_matches.into_iter())
                        .zip(mods_response.into_iter())
                        .map(|((fingerprint, fp_match), proj)| (fingerprint, fp_match, proj))
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };

            let (modpack, icon) = modpack_matches
                .and_then(|modpack_matches| {
                    modpack_matches.first().map(|(_, fp_match, proj)| {
                        let icon = proj.logo.as_ref().map(|logo| logo.url.clone());
                        (
                            CurseForgeImportedModpack::Managed {
                                project_id: fp_match.file.mod_id as u32,
                                file_id: fp_match.file.id as u32,
                                archive_path: path.clone(),
                                icon: icon.clone(),
                                name: proj.name.clone(),
                                file_name: fp_match
                                    .file
                                    .display_name
                                    .strip_suffix(".zip")
                                    .map(|name| name.to_string())
                                    .unwrap_or_else(|| fp_match.file.display_name.clone()),
                            },
                            icon,
                        )
                    })
                })
                .unwrap_or_else(|| {
                    (
                        CurseForgeImportedModpack::Unmanaged {
                            archive_path: path.clone(),
                        },
                        None,
                    )
                });

            let mut lock = self.results.lock().await;
            lock.push(CurseForgeManifestWrapper {
                full_path: path.clone(),
                manifest,
                modpack,
                icon,
            });
        }

        Ok(())
    }

    async fn get_default_scan_path(&self, _app: Arc<AppInner>) -> anyhow::Result<Option<PathBuf>> {
        Ok(None)
    }

    async fn get_available(&self) -> anyhow::Result<Vec<super::ImportableInstance>> {
        let instances = self
            .results
            .lock()
            .await
            .iter()
            .map(|instance| {
                let (name, icon, known_remote) = match &instance.modpack {
                    CurseForgeImportedModpack::Managed {
                        icon, file_name, ..
                    } => (file_name.clone(), icon.clone(), true),
                    CurseForgeImportedModpack::Unmanaged { .. } => {
                        (format!("{} - {}", &instance.manifest.name, &instance.manifest.version), None, false)
                    }
                };
                super::ImportableInstance {
                    entity: Entity::CurseForgeZip,
                    name,
                    icon,
                    import_once: false,
                }
            })
            .collect();
        Ok(instances)
    }

    async fn import(
        &self,
        app: Arc<AppInner>,
        index: u32,
        name: &str,
    ) -> anyhow::Result<VisualTaskId> {
        let lock = self.results.lock().await;
        let instance = lock
            .get(index as usize)
            .ok_or(anyhow::anyhow!("No importable instance at index {index}"))?;

        let use_icon = if let CurseForgeImportedModpack::Managed { icon: Some(icon_url), .. } = &instance.modpack {
            app.instance_manager().download_icon(icon_url.clone()).await?;
            true
        } else {
            false
        };

        let install_source = InstanceVersionSource::Modpack(instance.modpack.clone().into());

        let created_instance_id = app
            .instance_manager()
            .create_instance(
                app.instance_manager().get_default_group().await?,
                name.to_string(),
                use_icon,
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

    async fn scan(&mut self, app: Arc<AppInner>, scan_paths: Vec<PathBuf>) -> anyhow::Result<()> {
        if !scan_paths.is_empty() {
            self.results.lock().await.clear();
            app.invalidate(
                keys::instance::GET_IMPORTABLE_INSTANCES,
                Some(serde_json::to_value(FEEntity::MRPack)?),
            );
        }
        for path in scan_paths {
            let file_path_clone = path.clone();

            // make sure this is a valid modpack
            let index = spawn_blocking(move || {
                let file = std::fs::File::open(&file_path_clone).with_context(|| {
                    format!("Error reading `{}`", file_path_clone.to_string_lossy())
                })?;
                let mut archive = zip::ZipArchive::new(file).with_context(|| {
                    format!(
                        "Error reading archive `{}`",
                        file_path_clone.to_string_lossy()
                    )
                })?;
                let index: modrinth::version::ModpackIndex = {
                    let file = archive.by_name("modrinth.index.json").with_context(|| {
                        format!(
                            "Error reading `modrinth.index.json` from `{}`",
                            file_path_clone.to_string_lossy()
                        )
                    })?;
                    serde_json::from_reader(file).with_context(|| {
                        format!(
                            "Error parsing `modrinth.index.json` from `{}`",
                            file_path_clone.to_string_lossy()
                        )
                    })?
                };

                Ok::<_, anyhow::Error>(index)
            })
            .await??;

            let mut lock = self.results.lock().await;
            lock.push(MrpackIndexWrapper {
                full_path: path.clone(),
                index,
            });
        }

        Ok(())
    }

    async fn get_default_scan_path(&self, _app: Arc<AppInner>) -> anyhow::Result<Option<PathBuf>> {
        Ok(None)
    }

    async fn get_available(&self) -> anyhow::Result<Vec<super::ImportableInstance>> {
        let instances = self
            .results
            .lock()
            .await
            .iter()
            .map(|instance| super::ImportableInstance {
                entity: Entity::MRPack,
                name: instance.index.name.clone(),
                icon: None,
                import_once: false,
            })
            .collect();
        Ok(instances)
    }

    async fn import(
        &self,
        app: Arc<AppInner>,
        index: u32,
        name: &str,
    ) -> anyhow::Result<VisualTaskId> {
        let lock = self.results.lock().await;
        let instance = lock
            .get(index as usize)
            .ok_or(anyhow::anyhow!("No importable instance at index {index}"))?;
        let content = tokio::fs::read(&instance.full_path)
            .await
            .with_context(|| format!("Error reading `{}`", instance.full_path.to_string_lossy()))?;
        let sha512 = tokio::task::spawn_blocking(move || {
            hex::encode(<[u8; 64] as From<_>>::from(
                Sha512::new_with_prefix(content).finalize(),
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
