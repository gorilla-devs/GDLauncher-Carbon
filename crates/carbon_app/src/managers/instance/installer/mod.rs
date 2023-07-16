use anyhow::{bail, Context};
use carbon_net::{Checksum, Downloadable};
use std::{
    cell::RefCell, collections::HashSet, ffi::OsString, ops::Deref, path::PathBuf, pin::Pin,
    sync::Arc, time::Duration,
};
use tokio::{sync::Mutex, task::AbortHandle};

use crate::{
    api::{keys::instance::INSTANCE_DETAILS, translation::Translation},
    domain::{
        self,
        instance::{self, InstanceId},
        modplatforms::curseforge::{
            self,
            filters::{
                ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters,
            },
        },
        runtime_path::InstancePath,
        vtask::VisualTaskId,
    },
    managers::{instance::Mod, metadata, vtask::VisualTask, AppInner},
};

use super::{Instance, InstanceData, InstanceType, InvalidInstanceIdError};

use futures::future::{BoxFuture, Future};

type BoxedResourceInstaller = Box<dyn ResourceInstaller + Send>;
type ResourceInstallerGetter = Box<
    dyn FnOnce() -> Pin<Box<dyn Future<Output = anyhow::Result<BoxedResourceInstaller>> + Send>>
        + Send,
>;

pub struct DependencyIterator<'iter> {
    iter: Box<dyn Iterator<Item = ResourceInstallerGetter> + Send + 'iter>,
}

impl<'iter> Iterator for DependencyIterator<'iter> {
    type Item = ResourceInstallerGetter;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'iter> DependencyIterator<'iter> {
    pub fn new<I>(iter: I) -> Self
    where
        I: Iterator<Item = ResourceInstallerGetter> + Send + 'iter,
    {
        Self {
            iter: Box::new(iter),
        }
    }
}

pub enum ResourceFingerprint {
    BigInt(u64),
    Hash(String),
}

#[async_trait::async_trait]
pub trait ResourceInstaller: Sync {
    /// a unique ID to identify dependency loops
    fn id(&self) -> String;
    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable>;
    fn dependencies(&self, app: &Arc<AppInner>, instance_data: &InstanceData)
        -> DependencyIterator;
    fn is_already_installed(&self, instance_data: &InstanceData) -> bool;
    fn display_name(&self) -> String;
    async fn perform_install(
        &self,
        instance_data: &mut InstanceData,
        downloadable: Option<Downloadable>,
    ) -> anyhow::Result<()>;
    async fn rollback(&self, instance_data: &mut InstanceData) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<I: ResourceInstaller + ?Sized + Send> ResourceInstaller for Box<I> {
    #[inline]
    fn id(&self) -> String {
        (**self).id()
    }

    #[inline]
    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable> {
        (**self).downloadable(instance_path).await
    }

    #[inline]
    fn dependencies(
        &self,
        app: &Arc<AppInner>,
        instance_data: &InstanceData,
    ) -> DependencyIterator {
        (**self).dependencies(app, instance_data)
    }

    #[inline]
    fn is_already_installed(&self, instance_data: &InstanceData) -> bool {
        (**self).is_already_installed(instance_data)
    }
    #[inline]
    fn display_name(&self) -> String {
        (**self).display_name()
    }

    #[inline]
    async fn perform_install(
        &self,
        instance_data: &mut InstanceData,
        downloadable: Option<Downloadable>,
    ) -> anyhow::Result<()> {
        (**self).perform_install(instance_data, downloadable).await
    }

    #[inline]
    async fn rollback(&self, instance_data: &mut InstanceData) -> anyhow::Result<()> {
        (**self).rollback(instance_data).await
    }
}

pub struct InstallResult {
    pub task: VisualTaskId,
    pub dependency_tasks: Vec<VisualTaskId>,
}

struct InstallerRollbackContext {
    inner: Arc<Mutex<BoxedResourceInstaller>>,
    processed_deps: Arc<Mutex<Vec<Installer>>>,
    instance_id: InstanceId,
    app: Arc<AppInner>,
}

impl InstallerRollbackContext {
    pub async fn rollback(&self, inciting_error: Option<&anyhow::Error>) {
        let instance_manager = self.app.instance_manager();
        let mut instances = instance_manager.instances.write().await;
        let instance = instances
            .get_mut(&self.instance_id)
            .expect("rollback should be called only when operating on a valid instance");

        let data = instance
            .data_mut()
            .expect("rollback should be called only when operating on a valid instance");

        let parent_name = {
            let lock = self.inner.lock().await;
            lock.display_name()
        };

        let processed_deps = self.processed_deps.lock().await;

        for dep in processed_deps.iter() {
            let abort_handle = dep.abort_handle.lock().await;
            if let Some(handle) = &abort_handle.handle {
                handle.abort();
            }
            let lock = dep.inner.lock().await;
            match lock.rollback(data).await {
                Ok(_) => {}
                Err(err) => {
                    // report this error but continue with others
                    tracing::error!({error = ?err, inciting_error = ?inciting_error},
                        "Error rolling back install of {name:?} during rollback of {parent:?} install",
                        name = lock.display_name(),
                        parent = parent_name
                    );
                }
            }
        }
    }
}

pub struct Installer {
    inner: Arc<Mutex<BoxedResourceInstaller>>,
    rollback_context: Arc<Mutex<Option<InstallerRollbackContext>>>,
    abort_handle: Arc<Mutex<InstallerAbortHandle>>,
}

struct InstallerAbortHandle {
    pub handle: Option<AbortHandle>,
    pub aborted: bool,
}

impl Deref for Installer {
    type Target = Arc<Mutex<BoxedResourceInstaller>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub trait IntoInstaller: Sized {
    fn into_installer(self) -> Installer;
}

impl Installer {
    pub fn new(resource_installer: BoxedResourceInstaller) -> Self {
        Self {
            inner: Arc::new(Mutex::new(resource_installer)),
            rollback_context: Arc::new(Mutex::new(None)),
            abort_handle: Arc::new(Mutex::new(InstallerAbortHandle {
                handle: None,
                aborted: false,
            })),
        }
    }

    pub async fn abort(&self, inciting_error: Option<&anyhow::Error>) {
        {
            let mut abort_handle = self.abort_handle.lock().await;
            if let Some(handle) = &abort_handle.handle {
                if !handle.is_finished() {
                    handle.abort();
                }
            }
            abort_handle.aborted = true;
        }

        self.rollback(inciting_error).await;
    }

    async fn rollback(&self, inciting_error: Option<&anyhow::Error>) {
        let lock = self.rollback_context.lock().await;
        if let Some(context) = &*lock {
            context.rollback(inciting_error).await;
        }
    }

    pub async fn install(
        &self,
        app: &Arc<AppInner>,
        instance_id: &InstanceId,
    ) -> anyhow::Result<VisualTaskId> {
        let (task, task_id, instance_path) = async {
            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.write().await;
            let instance = instances
                .get(instance_id)
                .ok_or(InvalidInstanceIdError(*instance_id))?;

            let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &instance else {
                bail!("install called with invalid instance");
            };

            let task = {
                let lock = self.inner.lock().await;

                if lock.is_already_installed(data) {
                    bail!("resource is already installed");
                }

                let task = VisualTask::new(Translation::InstanceTaskInstallMod {
                    mod_name: lock.display_name(),
                    instance_name: data.config.name.clone(),
                });

                Ok::<VisualTask, anyhow::Error>(task)
            }?;

            let instance_path = app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(shortpath);

            let id = app.task_manager().spawn_task(&task).await;
            Ok((task, id, instance_path))
        }
        .await?;
        let visited_ids = Arc::new(Mutex::new(Vec::new()));
        let task = Arc::new(Mutex::new(task));
        self.install_inner(app, instance_id, &instance_path, &task, &visited_ids)
            .await?;

        Ok(task_id)
    }

    #[async_recursion::async_recursion]
    async fn install_inner(
        &self,
        app: &Arc<AppInner>,
        instance_id: &InstanceId,
        instance_path: &InstancePath,
        parent_task: &Arc<Mutex<VisualTask>>,
        visited_ids: &Arc<Mutex<Vec<String>>>,
    ) -> anyhow::Result<()> {
        {
            let mut lock = visited_ids.lock().await;
            let installer_id = self.inner.lock().await.id();
            if !lock.iter().any(|id| id == &installer_id) {
                // not found, add ourselves
                lock.push(installer_id);
            } else {
                // already being installed
                return Ok(());
            }
        }

        let (dep_error, processed_deps) = {
            let lock = self.inner.lock().await;
            let installer_name = lock.display_name();
            let dep_iter = {
                let instance_manager = app.instance_manager();
                let instances = instance_manager.instances.read().await;
                let instance = instances
                    .get(instance_id)
                    .expect("instance should still be valid");
                let instance_data = instance.data().expect("instance should still be valid");

                lock.dependencies(app, instance_data)
            };

            let mut processed_deps = Vec::new();
            let mut dep_error = None;

            for dep in dep_iter {
                let dep_result = dep().await;
                match dep_result {
                    Err(err) => {
                        dep_error = Some(err.context(format!(
                            "Error processing dependencies for `{}`",
                            installer_name
                        )));
                        break;
                    }
                    Ok(dep) => {
                        let dep_name = dep.display_name();
                        let dep = Installer::new(dep);
                        let install_future = dep.install_inner(
                            app,
                            instance_id,
                            instance_path,
                            parent_task,
                            visited_ids,
                        );
                        let results = install_future.await;
                        match results {
                            Err(err) => {
                                dep_error = Some(err.context(format!(
                                    "Error installing dependency `{}` for `{}`",
                                    dep_name, installer_name
                                )));
                                break;
                            }
                            Ok(()) => {
                                processed_deps.push(dep);
                            }
                        }
                    }
                }
            }

            (dep_error, processed_deps)
        };

        {
            let mut lock = self.rollback_context.lock().await;
            *lock = Some(InstallerRollbackContext {
                inner: Arc::clone(&self.inner),
                processed_deps: Arc::new(Mutex::new(processed_deps)),
                instance_id: *instance_id,
                app: Arc::clone(app),
            })
        }

        if let Some(dep_error) = dep_error {
            self.rollback(Some(&dep_error)).await;
            return Err(dep_error);
        }

        let t_download_file = {
            let lock = parent_task.lock().await;
            lock.subtask(Translation::InstanceTaskInstallModDownloadFile)
                .await
        };

        let instance_id = *instance_id;
        let instance_path = instance_path.clone();
        let inner = Arc::clone(&self.inner);
        let app_clone = Arc::clone(app);
        let rollback_context = Arc::clone(&self.rollback_context);
        let parent_task = Arc::clone(parent_task);

        {
            let mut abort_handle = self.abort_handle.lock().await;
            if !abort_handle.aborted {
                let task_handle = tokio::spawn(async move {
                    let r = (|| async {
                        let downloadable = {
                            let lock = inner.lock().await;
                            lock.downloadable(&instance_path).await
                        };

                        if let Some(downloadable) = &downloadable {
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

                            carbon_net::download_file(downloadable, Some(progress_watch_tx)).await?
                        }

                        {
                            // context to drop instance lock after install attempt
                            let instance_manager = app_clone.instance_manager();
                            let mut instances = instance_manager.instances.write().await;
                            let instance = instances
                                .get_mut(&instance_id)
                                .expect("instance should still be valid");

                            let data = instance.data_mut().expect("instance should still be valid");
                            let lock = inner.lock().await;

                            lock.perform_install(data, downloadable).await
                        }?;

                        app_clone.invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
                        Ok::<_, anyhow::Error>(())
                    })()
                    .await;

                    match r {
                        Ok(()) => {}
                        Err(e) => {
                            let rollback_lock = rollback_context.lock().await;
                            rollback_lock
                                .as_ref()
                                .expect("valid rollback context in spawned task")
                                .rollback(Some(&e))
                                .await;

                            let parent_task = parent_task.lock().await;
                            parent_task.clone().fail(e).await
                        }
                    }
                });

                abort_handle.handle = Some(task_handle.abort_handle());
            }
        }
        Ok(())
    }
}

pub struct CurseforgeModInstaller {
    file: crate::domain::modplatforms::curseforge::File,
    download_url: String,
    applied_data: Arc<Mutex<Option<(Mod, Downloadable)>>>,
}

impl CurseforgeModInstaller {
    pub async fn create(
        app: &Arc<AppInner>,
        project_id: u32,
        file_id: u32,
    ) -> anyhow::Result<Self> {
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
            file,
            download_url,
            applied_data: Arc::new(Mutex::new(None)),
        })
    }

    pub fn from_file(file: crate::domain::modplatforms::curseforge::File) -> anyhow::Result<Self> {
        let download_url = file.download_url.clone().ok_or_else(|| {
            anyhow::anyhow!("mod cannot be downloaded without privileged api key")
        })?;

        Ok(Self {
            file,
            download_url,
            applied_data: Arc::new(Mutex::new(None)),
        })
    }
}

#[async_trait::async_trait]
impl ResourceInstaller for CurseforgeModInstaller {
    fn id(&self) -> String {
        format!("curseforge:{}:{}", &self.file.mod_id, &self.file.id)
    }

    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable> {
        let install_path = instance_path.get_mods_path().join(&self.file.file_name);

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

        Some(
            Downloadable::new(&self.download_url, install_path)
                .with_checksum(checksums.get(0).cloned()),
        )
    }

    fn dependencies(
        &self,
        app: &Arc<AppInner>,
        instance_data: &InstanceData,
    ) -> DependencyIterator {
        let game_version = instance_data
            .config
            .game_configuration
            .version
            .clone()
            .and_then(|ver| match ver {
                instance::info::GameVersion::Standard(version) => {
                    Some((version.release, version.modloaders))
                }
                _ => None,
            });

        let mut installers: Vec<ResourceInstallerGetter> = Vec::new();
        for dep in &self.file.dependencies {
            let app_clone = Arc::clone(app);
            let mod_id = dep.mod_id;
            let game_version = game_version.clone();

            if let curseforge::FileRelationType::RequiredDependency = dep.relation_type {
                installers.push(Box::new(move || {
                    Box::pin(async move {
                        app_clone
                            .modplatforms_manager()
                            .curseforge
                            .get_mod_files(ModFilesParameters {
                                mod_id,
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
                                // select an appropriate file based on game version and loader, or
                                // the first file if that fails
                                if let Some((release, modloaders)) = game_version {
                                    let modloader_strings: Vec<String> = modloaders
                                        .iter()
                                        .filter_map(|modloader| match modloader.type_ {
                                            domain::instance::info::ModLoaderType::Forge => {
                                                Some("forge".to_string())
                                            }
                                            domain::instance::info::ModLoaderType::Fabric => {
                                                Some("fabric".to_string())
                                            }
                                            domain::instance::info::ModLoaderType::Quilt => {
                                                Some("quilt".to_string())
                                            }
                                            domain::instance::info::ModLoaderType::Unknown => None,
                                        })
                                        .collect();
                                    if let Some(file) = res.data.iter().find(|&file| {
                                        let has_release = file.game_versions.contains(&release);
                                        let has_one_of_our_modloaders =
                                            file.game_versions.iter().any(|ver| {
                                                modloader_strings.contains(&ver.to_lowercase())
                                            });
                                        has_release && has_one_of_our_modloaders
                                    }) {
                                        Ok(file.clone())
                                    } else {
                                        res.data
                                            .first()
                                            .cloned()
                                            .ok_or_else(|| anyhow::anyhow!("no files found"))
                                    }
                                } else {
                                    res.data
                                        .first()
                                        .cloned()
                                        .ok_or_else(|| anyhow::anyhow!("no files found"))
                                }
                            })
                            .and_then(|file| {
                                // let app_clone = Arc::clone(&self.app);
                                CurseforgeModInstaller::from_file(file)
                                    .map(|installer| Box::new(installer) as BoxedResourceInstaller)
                            })
                    })
                }));
            }
        }
        DependencyIterator::new(installers.into_iter())
    }

    fn is_already_installed(&self, instance_data: &InstanceData) -> bool {
        // TODO: check with fingerprint once local meta cache is done
        let file_name = &self.file.file_name;
        instance_data
            .mods
            .iter()
            .any(|m| &m.filename.to_string_lossy() == file_name)
    }

    fn display_name(&self) -> String {
        self.file.display_name.clone()
    }

    async fn perform_install(
        &self,
        instance_data: &mut InstanceData,
        downloadable: Option<Downloadable>,
    ) -> anyhow::Result<()> {
        let Some(downloadable) = downloadable else {
            return Err(anyhow::anyhow!("Perform install called before file was downloaded."))
        };

        tracing::info!(
            "Installing {} from {}",
            self.file.display_name,
            downloadable.path.to_string_lossy()
        );

        let file_data = tokio::fs::read(&downloadable.path).await.with_context(|| {
            format!(
                "failed to read file: `{}`",
                &downloadable.path.to_string_lossy()
            )
        })?;

        use md5::Digest;
        let md5hash = md5::Md5::new_with_prefix(&file_data).finalize();

        let metadata = tokio::task::spawn_blocking(|| {
            metadata::mods::parse_metadata(std::io::Cursor::new(file_data))
        })
        .await??
        .ok_or_else(|| anyhow::anyhow!("downloaded curseforge mod did not have any metadata"))?;

        let id = hex::encode(md5hash);

        let mod_data = Mod {
            id,
            filename: OsString::from(&self.file.file_name),
            enabled: true,
            modloaders: metadata
                .modloaders
                .clone()
                .unwrap_or_else(|| vec![instance::info::ModLoaderType::Forge]),
            metadata,
        };

        instance_data.mods.push(mod_data.clone());
        *self.applied_data.lock().await = Some((mod_data, downloadable));
        Ok(())
    }

    async fn rollback(&self, instance_data: &mut InstanceData) -> anyhow::Result<()> {
        let mut lock = self.applied_data.lock().await;
        if let Some((applied_mod_data, downloadable)) = &*lock {
            instance_data
                .mods
                .retain(|mod_data| mod_data.id != applied_mod_data.id);

            match tokio::fs::try_exists(&downloadable.path).await {
                Ok(true) => {
                    tokio::fs::remove_file(&downloadable.path).await?;
                }
                Ok(false) => {
                    // not downloaded yet
                    // NOOP
                }
                Err(_) => {
                    // no confirmation of path, not downloaded yet
                    // NOOP
                }
            }
        }

        *lock = None;

        Ok(())
    }
}

impl IntoInstaller for CurseforgeModInstaller {
    fn into_installer(self) -> Installer {
        Installer::new(Box::new(self) as BoxedResourceInstaller)
    }
}
