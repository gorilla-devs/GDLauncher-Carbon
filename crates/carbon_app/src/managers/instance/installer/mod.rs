use anyhow::bail;
use carbon_net::{Checksum, Downloadable};
use std::{ffi::OsString, pin::Pin, sync::Arc, time::Duration};

use crate::{
    api::{keys::instance::INSTANCE_DETAILS, translation::Translation},
    domain::{
        self,
        instance::{self, InstanceId},
        modplatforms::curseforge::filters::{
            ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters,
        },
        vtask::VisualTaskId,
    },
    managers::{instance::Mod, metadata, vtask::VisualTask, AppInner},
};

use super::{Instance, InstanceData, InstanceType, InvalidInstanceIdError};

type PinedResourceInstaller = Pin<Box<dyn ResourceInstaller + Send>>;
pub struct DependencyIterator {
    iter: Pin<Box<dyn Iterator<Item = anyhow::Result<PinedResourceInstaller>> + Send>>,
}

impl Iterator for DependencyIterator {
    type Item = anyhow::Result<PinedResourceInstaller>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl DependencyIterator {
    pub fn new<I>(iter: I) -> Self
    where
        I: Iterator<Item = anyhow::Result<PinedResourceInstaller>> + Send,
    {
        Self {
            iter: Box::pin(iter),
        }
    }
}

pub enum ResourceFingerprint {
    BigInt(u64),
    Hash(String),
}

#[async_trait::async_trait]
pub trait ResourceInstaller: Sync {
    async fn downloadable(&self, instance_id: &str) -> Downloadable;
    async fn dependencies(&self) -> DependencyIterator;
    fn fingerprint(&self) -> ResourceFingerprint;
    fn filename(&self) -> String;
    fn display_name(&self) -> String;
    async fn perform_install(
        &self,
        file_data: &Vec<u8>,
        instance_data: &mut InstanceData,
    ) -> anyhow::Result<()>;
}

pub struct InstallResult {
    task: VisualTaskId,
    dependency_tasks: Vec<VisualTaskId>,
}

#[async_trait::async_trait]
pub trait InstallResource: Sync {
    async fn install(
        &self,
        app: Arc<AppInner>,
        instance_id: &InstanceId,
    ) -> anyhow::Result<InstallResult>;
}

// #[async_trait::async_trait]
// impl InstallResource for Pin<Box<dyn ResourceInstaller + Send>> {
//     async fn install(
//         &self,
//         app: Arc<AppInner>,
//         instance_id: &InstanceId,
//     ) -> anyhow::Result<(VisualTaskId, Vec<anyhow::Result<VisualTaskId>>)> {
//         self.install(app, instance_id).await
//     }
// }

#[async_trait::async_trait]
impl<T> InstallResource for T
where
    T: ResourceInstaller + ?Sized,
{
    async fn install(
        &self,
        app: Arc<AppInner>,
        instance_id: &InstanceId,
    ) -> anyhow::Result<InstallResult> {
        let (task, task_id, shortpath) = {
            let instance_manager = app.instance_manager();
            let mut instances = instance_manager.instances.write().await;
            let mut instance = instances
                .get_mut(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id.clone()))?;

            let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &mut instance else {
                bail!("install_mod called on invalid instance");
            };

            // TODO: check with fingerprint once local meta cache is done
            let file_name = self.filename();
            if data
                .mods
                .iter()
                .any(|m| m.filename.to_string_lossy() == file_name)
            {
                bail!("mod is already installed");
            }

            let task = VisualTask::new(Translation::InstanceTaskInstallMod {
                mod_name: self.display_name(),
                instance_name: data.config.name.clone(),
            });

            let id = app.task_manager().spawn_task(&task).await;
            (task, id, shortpath)
        };

        let dep_results = Vec::new();

        let dep_iter = self.dependencies().await;

        for dep in dep_iter {
            match dep {
                Err(err) => {
                    unimplemented!();
                    self.rollback();
                    // dep_results.push(Err(err));
                }
                Ok(dep) => {
                    let app_clone = Arc::clone(&app);
                    let install_future = dep.install(app_clone, instance_id);
                    let results = install_future.await;
                    match results {
                        Err(err) => {
                            unimplemented!();
                            // self.rollback;
                        }
                        Ok(mut results) => {
                            dep_results.push(results.task);
                            dep_results.append(&mut results.dependency_tasks);
                        }
                    }
                }
            }
        }

        let t_download_file = task
            .subtask(Translation::InstanceTaskInstallModDownloadFile)
            .await;

        let downloadable = self.downloadable(shortpath).await;

        tokio::spawn(async move {
            let r = (|| async {
                let (progress_watch_tx, mut progress_watch_rx) =
                    tokio::sync::watch::channel(carbon_net::Progress::new());

                // dropped when the sender is dropped
                tokio::spawn(async move {
                    while progress_watch_rx.changed().await.is_ok() {
                        {
                            let progress = progress_watch_rx.borrow();
                            t_download_file.update_download(
                                progress.current_size as u32,
                                progress.total_size as u32,
                            );
                        }

                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }

                    t_download_file.complete_download();
                });

                carbon_net::download_file(&downloadable, Some(progress_watch_tx)).await?;

                let instance_manager = app.instance_manager();
                let mut instances = instance_manager.instances.write().await;
                let mut instance = instances
                    .get_mut(&instance_id)
                    .ok_or(InvalidInstanceIdError(instance_id.clone()))?;

                let Instance { type_: InstanceType::Valid(data), .. } = &mut instance else {
                    bail!("install_mod called on invalid instance");
                };

                let file_data = tokio::fs::read(downloadable.path).await?;

                self.perform_install(&file_data, data).await?;

                app.invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
                Ok::<_, anyhow::Error>(())
            })()
            .await;

            match r {
                Ok(()) => {}
                Err(e) => task.fail(e).await,
            }
        });

        Ok((task_id, dep_results))
    }
}

pub struct CurseforgeModInstaller {
    app: Arc<AppInner>,
    file: crate::domain::modplatforms::curseforge::File,
    download_url: String,
}

impl CurseforgeModInstaller {
    pub async fn create(app: Arc<AppInner>, project_id: u32, file_id: u32) -> anyhow::Result<Self> {
        let file = app
            .modplatforms_manager()
            .curseforge
            .get_mod_file(ModFileParameters {
                mod_id: project_id as i32,
                file_id: file_id as i32,
            })
            .await?
            .data;

        let download_url = file.download_url.clone().ok_or_else(|| {
            anyhow::anyhow!("mod cannot be downloaded without privileged api key")
        })?;

        Ok(Self {
            app,
            file,
            download_url,
        })
    }

    pub fn from_file(
        app: Arc<AppInner>,
        file: crate::domain::modplatforms::curseforge::File,
    ) -> anyhow::Result<Self> {
        let download_url = file.download_url.clone().ok_or_else(|| {
            anyhow::anyhow!("mod cannot be downloaded without privileged api key")
        })?;

        Ok(Self {
            app,
            file,
            download_url,
        })
    }
}

#[async_trait::async_trait]
impl ResourceInstaller for CurseforgeModInstaller {
    async fn downloadable(&self, instance_id: &str) -> Downloadable {
        let install_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(instance_id)
            .get_mods_path()
            .join(&self.file.file_name);

        let checksums = &self
            .file
            .hashes
            .iter()
            .map(|hash| match hash.algo {
                crate::domain::modplatforms::curseforge::HashAlgo::Sha1 => {
                    Checksum::Sha1(hash.value.clone())
                }
                crate::domain::modplatforms::curseforge::HashAlgo::Md5 => {
                    Checksum::Md5(hash.value.clone())
                }
            })
            .collect::<Vec<_>>();

        Downloadable::new(&self.download_url, install_path).with_checksum(checksums.get(0).cloned())
    }

    async fn dependencies(&self) -> DependencyIterator {
        let mut installers = Vec::new();
        for dep in self.file.dependencies {
            let file = self
                .app
                .modplatforms_manager()
                .curseforge
                .get_mod_files(ModFilesParameters {
                    mod_id: dep.mod_id,
                    query: ModFilesParametersQuery {
                        game_version: None,
                        game_version_type_id: None,
                        index: None,
                        page_size: None,
                        mod_loader_type: None,
                    },
                })
                .await
                .and_then(|res| {
                    res.data
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("no files found"))
                });
            match file {
                Err(err) => {
                    installers.push(Err(err));
                }
                Ok(file) => {
                    let app_clone = Arc::clone(&self.app);
                    let installer = CurseforgeModInstaller::from_file(app_clone, file.clone()).map(
                        |installer| Box::pin(installer) as Pin<Box<dyn ResourceInstaller + Send>>,
                    );
                    installers.push(installer);
                }
            }
        }
        DependencyIterator::new(installers.into_iter())
    }

    fn fingerprint(&self) -> ResourceFingerprint {
        ResourceFingerprint::BigInt(self.file.file_fingerprint)
    }

    fn filename(&self) -> String {
        self.file.file_name.clone()
    }

    fn display_name(&self) -> String {
        self.file.display_name.clone()
    }

    async fn perform_install(
        &self,
        file_data: &Vec<u8>,
        instance_data: &mut InstanceData,
    ) -> anyhow::Result<()> {
        use md5::Digest;
        let md5hash = md5::Md5::new_with_prefix(&file_data).finalize();

        let metadata = tokio::task::spawn_blocking(|| {
            metadata::mods::parse_metadata(std::io::Cursor::new(file_data))
        })
        .await??
        .ok_or_else(|| anyhow::anyhow!("downloaded curseforge mod did not have any metadata"))?;

        let id = hex::encode(md5hash);

        instance_data.mods.push(Mod {
            id,
            filename: OsString::from(&self.file.file_name),
            enabled: true,
            modloaders: metadata
                .modloaders
                .clone()
                .unwrap_or_else(|| vec![instance::info::ModLoaderType::Forge]),
            metadata,
        });
        Ok(())
    }
}
