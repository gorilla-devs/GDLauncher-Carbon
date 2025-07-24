use super::{Instance, InstanceData, InstanceType, InvalidInstanceIdError};
use crate::{
    api::{
        keys::instance::{INSTANCE_DETAILS, INSTANCE_MODS},
        translation::Translation,
    },
    domain::{
        self,
        instance::{self, InstanceId},
        vtask::VisualTaskId,
    },
    managers::{
        AppInner,
        instance::Mod,
        vtask::{TaskState, VisualTask},
    },
};
use anyhow::{Context, bail};
use carbon_net::{Checksum, DownloadOptions, Downloadable};
use carbon_platforms::{
    ModChannel,
    curseforge::{
        self,
        filters::{
            ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters,
            ModsParameters, ModsParametersBody,
        },
    },
    modrinth::{
        self,
        project::ProjectVersionsFilters,
        search::{ProjectID, VersionID},
    },
};
use carbon_repos::db::{
    curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb,
    modrinth_mod_cache as mrdb,
};
use carbon_rt_path::InstancePath;
use futures::future::Future;
use std::{ops::Deref, pin::Pin, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::AbortHandle};

type BoxedResourceInstaller = Box<dyn ResourceInstaller + Send>;
type ResourceInstallerGetter = Box<
    dyn FnOnce() -> Pin<
            Box<dyn Future<Output = Option<anyhow::Result<BoxedResourceInstaller>>> + Send>,
        > + Send,
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
    fn dependencies(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_data: &InstanceData,
        preferred_channel: ModChannel,
    ) -> DependencyIterator;
    async fn is_already_installed(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
    ) -> anyhow::Result<bool>;
    fn display_name(&self) -> String;
    async fn rollback(&self, instance_data: &mut InstanceData) -> anyhow::Result<()>;

    /// Get the addon type for this installer
    fn get_addon_type(&self) -> crate::domain::instance::AddonType {
        // Default to mods if not implemented
        crate::domain::instance::AddonType::Mods
    }
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
        instance_id: InstanceId,
        instance_data: &InstanceData,
        preferred_channel: ModChannel,
    ) -> DependencyIterator {
        (**self).dependencies(app, instance_id, instance_data, preferred_channel)
    }

    #[inline]
    async fn is_already_installed(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
    ) -> anyhow::Result<bool> {
        (**self).is_already_installed(app, instance_id).await
    }
    #[inline]
    fn display_name(&self) -> String {
        (**self).display_name()
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
        instance_id: InstanceId,
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        tracing::info!(
            "Installer::install called with replaces_mod_id: {:?}",
            replaces_mod_id
        );
        let download_deps = app
            .settings_manager()
            .get_settings()
            .await?
            .download_dependencies;

        tracing::info!(
            "🚀 INSTALLER: Starting install for instance {:?}",
            instance_id
        );
        tracing::info!("   - install_deps: {}", install_deps);
        tracing::info!("   - download_deps: {}", download_deps);
        tracing::info!("   - replaces_mod_id: {:?}", replaces_mod_id);

        let (task, task_id, instance_path) = async {
            tracing::info!("🔧 INSTALLER: Creating task and getting instance path");

            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.write().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            tracing::info!("   - Found instance in manager");

            let Instance {
                type_: InstanceType::Valid(data),
                shortpath,
                ..
            } = &instance
            else {
                bail!("install called with invalid instance");
            };
            tracing::info!("   - Instance is valid, shortpath: {:?}", shortpath);

            let task = {
                let lock = self.inner.lock().await;
                tracing::info!("   - Acquired installer lock");

                if lock.is_already_installed(app, instance_id).await? {
                    bail!("resource is already installed");
                }
                tracing::info!("   - Resource not already installed, proceeding");

                let task = VisualTask::new(Translation::InstanceTaskInstallMod {
                    mod_name: lock.display_name(),
                    instance_name: data.config.name.clone(),
                });
                tracing::info!("   - Created visual task: {:?}", lock.display_name());

                Ok::<VisualTask, anyhow::Error>(task)
            }?;

            let instance_path = app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(shortpath);
            tracing::info!("   - Instance path: {:?}", instance_path);

            let id = app.task_manager().spawn_task(&task).await;
            tracing::info!("   - Task spawned with ID: {:?}", id);

            Ok((task, id, instance_path))
        }
        .await?;
        let visited_ids = Arc::new(Mutex::new(Vec::new()));

        // Store the task in a scope to ensure it gets dropped
        {
            let task_arc = Arc::new(Mutex::new(task));

            tracing::info!("📥 INSTALLER: Starting install_inner for installer");

            self.install_inner(
                app,
                instance_id,
                &instance_path,
                &task_arc,
                &visited_ids,
                install_deps && download_deps,
                replaces_mod_id,
            )
            .await?;

            tracing::info!("✅ INSTALLER: install_inner completed, dropping task");

            // Extract the task from the Arc and drop it explicitly to ensure completion
            if let Ok(task) = Arc::try_unwrap(task_arc) {
                drop(task.into_inner());
                tracing::info!("🗑️ INSTALLER: Task dropped successfully");
            } else {
                tracing::warn!("⚠️ INSTALLER: Could not unwrap task Arc - still has references");
            }
        }

        tracing::info!(
            "🎉 INSTALLER: Install completed successfully, returning task ID: {:?}",
            task_id
        );
        Ok(task_id)
    }

    #[async_recursion::async_recursion]
    async fn install_inner(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_path: &InstancePath,
        parent_task: &Arc<Mutex<VisualTask>>,
        visited_ids: &Arc<Mutex<Vec<String>>>,
        install_deps: bool,
        replaces_mod_id: Option<String>,
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

        if install_deps {
            let (dep_error, processed_deps) = {
                let lock = self.inner.lock().await;
                let installer_name = lock.display_name();
                let dep_iter = {
                    let instance_manager = app.instance_manager();
                    let instances = instance_manager.instances.read().await;
                    let instance = instances
                        .get(&instance_id)
                        .expect("instance should still be valid");
                    let instance_data = instance.data().expect("instance should still be valid");

                    lock.dependencies(app, instance_id, instance_data, ModChannel::Stable)
                };

                let mut processed_deps = Vec::new();
                let mut dep_error = None;

                for dep in dep_iter {
                    let dep_result = dep().await;
                    let Some(dep_result) = dep_result else {
                        continue;
                    };

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
                                true,
                                None,
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

            // Only check if already installed when NOT replacing a mod
            // When updating (replaces_mod_id is Some), we want to proceed even if a file with the same name exists
            if replaces_mod_id.is_none() {
                let is_installed = self
                    .inner
                    .lock()
                    .await
                    .is_already_installed(app, instance_id)
                    .await?;

                if is_installed {
                    tracing::info!("Mod is already installed, skipping installation");
                    return Ok(());
                }
            } else {
                tracing::info!(
                    "Skipping is_already_installed check because we're replacing mod: {:?}",
                    replaces_mod_id
                );
            }

            let mut lock = self.rollback_context.lock().await;
            *lock = Some(InstallerRollbackContext {
                inner: Arc::clone(&self.inner),
                processed_deps: Arc::new(Mutex::new(processed_deps)),
                instance_id,
                app: Arc::clone(app),
            });

            drop(lock);

            if let Some(dep_error) = dep_error {
                self.rollback(Some(&dep_error)).await;
                return Err(dep_error);
            }
        }

        let t_download_file = {
            let lock = parent_task.lock().await;
            lock.subtask(Translation::InstanceTaskInstallModDownloadFile)
        };

        let instance_path = instance_path.clone();
        let inner = Arc::clone(&self.inner);
        let app_clone = Arc::clone(app);
        let rollback_context = Arc::clone(&self.rollback_context);
        let parent_task = Arc::clone(parent_task);

        {
            let mut abort_handle = self.abort_handle.lock().await;
            if !abort_handle.aborted {
                let task_handle = tokio::spawn(async move {
                    tracing::info!("🔄 SPAWNED TASK: Starting mod installation task");
                    let start_time = std::time::Instant::now();

                    let r = (|| async {
                        tracing::info!("🔧 SPAWNED TASK: Beginning installation process");
                        let downloadable = {
                            tracing::info!("🔒 SPAWNED TASK: Acquiring inner lock for downloadable");
                            let lock = inner.lock().await;
                            let result = lock.downloadable(&instance_path).await;
                            tracing::info!("📦 SPAWNED TASK: Got downloadable result: {:?}", result.is_some());
                            result
                        };

                        tracing::info!("📊 SPAWNED TASK: Setting task state to KnownProgress");
                        parent_task
                            .lock()
                            .await
                            .edit(|data| data.state = TaskState::KnownProgress)
                            .await;

                        if let Some(downloadable) = &downloadable {
                            tracing::info!("⬇️ SPAWNED TASK: Processing downloadable: {:?}", downloadable.path);
                            {
                                tracing::info!("🌐 SPAWNED TASK: Starting real download process");
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
                                                false,
                                            );
                                        }

                                        tokio::time::sleep(Duration::from_millis(30)).await;
                                    }

                                    t_download_file.complete_download();
                                });

                                // Add timeout to prevent hanging in test environments
                                let download_result = tokio::time::timeout(
                                    std::time::Duration::from_secs(10),
                                    carbon_net::download_multiple(
                                        &[downloadable.clone()],
                                        DownloadOptions::builder().concurrency(1).build(),
                                    )
                                ).await;
                                match download_result {
                                    Ok(Ok(_)) => {
                                        tracing::info!("Download completed successfully");
                                    }
                                    Ok(Err(e)) => {
                                        return Err(e).with_context(|| {
                                            format!("Failed to download addon file for `{:?}`", downloadable)
                                        });
                                    }
                                    Err(_) => {
                                        return Err(anyhow::anyhow!("Download timed out after 10 seconds"));
                                    }
                                }
                            }
                        }

                        // Cache just the new mod file instead of forcing a full instance scan
                        if let Some(ref downloadable) = downloadable {
                            tracing::info!(
                                "💾 SPAWNED TASK: Caching single mod file at path: {:?}",
                                downloadable.path
                            );
                            let addon_type = {
                                tracing::info!("🔒 SPAWNED TASK: Getting addon type from inner lock");
                                let lock = inner.lock().await;
                                let addon_type = lock.get_addon_type();
                                tracing::info!("🏷️ SPAWNED TASK: Addon type: {:?}", addon_type);
                                addon_type
                            };

                            tracing::info!("🚀 SPAWNED TASK: Calling cache_single_mod_file");
                            // Extract platform metadata from installer context
                            let platform_metadata = {
                                let lock = inner.lock().await;
                                let installer_id = lock.id();
                                tracing::debug!("📋 SPAWNED TASK: Installer ID: {}", installer_id);
                                // Parse the installer ID to determine platform and extract metadata
                                if installer_id.starts_with("curseforge:") {
                                    // Format: "curseforge:project_id:file_id"
                                    let parts: Vec<&str> = installer_id.split(':').collect();
                                    if parts.len() == 3 {
                                        if let (Ok(project_id), Ok(file_id)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                                            tracing::info!("🏷️ SPAWNED TASK: Detected CurseForge mod: project_id={}, file_id={}", project_id, file_id);
                                            Some(crate::managers::metadata::cache::PlatformMetadata::CurseForge { project_id, file_id })
                                        } else {
                                            tracing::warn!("⚠️ SPAWNED TASK: Failed to parse CurseForge IDs from: {}", installer_id);
                                            None
                                        }
                                    } else {
                                        tracing::warn!("⚠️ SPAWNED TASK: Invalid CurseForge installer ID format: {}", installer_id);
                                        None
                                    }
                                } else if installer_id.starts_with("modrinth:") {
                                    // Format: "modrinth:project_id:version_id"
                                    let parts: Vec<&str> = installer_id.split(':').collect();
                                    if parts.len() == 3 {
                                        let project_id = parts[1].to_string();
                                        let version_id = parts[2].to_string();
                                        tracing::info!("🏷️ SPAWNED TASK: Detected Modrinth mod: project_id={}, version_id={}", project_id, version_id);
                                        Some(crate::managers::metadata::cache::PlatformMetadata::Modrinth { project_id, version_id })
                                    } else {
                                        tracing::warn!("⚠️ SPAWNED TASK: Invalid Modrinth installer ID format: {}", installer_id);
                                        None
                                    }
                                } else {
                                    tracing::info!("ℹ️ SPAWNED TASK: Unknown installer type: {}", installer_id);
                                    None
                                }
                            };

                            app_clone
                                .meta_cache_manager()
                                .cache_single_mod_file(instance_id, &downloadable.path, addon_type, &app_clone.prisma_client, platform_metadata)
                                .await?;
                            tracing::info!("✅ SPAWNED TASK: Single mod file cached successfully");
                        } else {
                            tracing::warn!("⚠️ SPAWNED TASK: No downloadable found, falling back to full caching");
                            // Fallback to full caching if no downloadable
                            tracing::info!("🚀 SPAWNED TASK: Calling override_caching_and_wait");
                            let cache_manager = app_clone.meta_cache_manager();
                            cache_manager
                                .override_caching_and_wait(instance_id, cache_manager)
                                .await?;
                            tracing::info!("✅ SPAWNED TASK: Full caching completed");
                        }

                        // Delete the old mod AFTER the new one is cached to prevent it from disappearing
                        if let Some(id) = replaces_mod_id {
                            tracing::info!("🗑️ SPAWNED TASK: Attempting to delete old mod with id: {}", id);
                            if let Err(e) = app_clone
                                .instance_manager()
                                .delete_mod(instance_id, id.clone())
                                .await
                            {
                                tracing::error!("❌ SPAWNED TASK: Failed to delete old mod {}: {:?}", id, e);
                                // Continue anyway - the new mod is already installed
                            } else {
                                tracing::info!("✅ SPAWNED TASK: Successfully deleted old mod: {}", id);
                            }
                        } else {
                            tracing::info!("ℹ️ SPAWNED TASK: No mod to replace (replaces_mod_id is None)");
                        }

                        tracing::info!("🔄 SPAWNED TASK: Invalidating instance mods cache");
                        app_clone.invalidate(INSTANCE_MODS, Some(instance_id.0.into()));

                        let elapsed = start_time.elapsed();
                        tracing::info!("🏁 SPAWNED TASK: Mod installation task completed in {:?}", elapsed);

                        Ok::<_, anyhow::Error>(())
                    })()
                    .await;

                    match r {
                        Ok(()) => {
                            tracing::info!(
                                "🎉 SPAWNED TASK: Task completed successfully, no errors"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "💥 SPAWNED TASK: Error installing dependency: {:?}",
                                e
                            );

                            let rollback_lock = rollback_context.lock().await;

                            if let Some(rollback_lock) = rollback_lock.as_ref() {
                                tracing::info!(
                                    "🔄 SPAWNED TASK: Rolling back changes due to error"
                                );
                                rollback_lock.rollback(Some(&e)).await;
                            } else {
                                tracing::error!(
                                    "❌ SPAWNED TASK: Invalid rollback context in spawned task"
                                );
                            }

                            tracing::info!("🚨 SPAWNED TASK: Failing parent task due to error");
                            let parent_task = parent_task.lock().await;
                            parent_task.clone().fail(e).await
                        }
                    }

                    tracing::info!(
                        "🏁 SPAWNED TASK: Task ending - this should trigger task completion"
                    );
                });

                abort_handle.handle = Some(task_handle.abort_handle());
            }
        }
        Ok(())
    }
}

// curseforge
pub struct CurseforgeModInstaller {
    addon_type: curseforge::ClassId,
    file: curseforge::File,
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

        let addon = app
            .modplatforms_manager()
            .curseforge
            .get_mod(ModParameters {
                mod_id: project_id as i32,
            })
            .await?;

        let download_url = file.download_url.clone().ok_or_else(|| {
            anyhow::anyhow!("mod cannot be downloaded without privileged api key")
        })?;

        Ok(Self {
            addon_type: addon.data.class_id.unwrap_or(curseforge::ClassId::Mods),
            file,
            download_url,
            applied_data: Arc::new(Mutex::new(None)),
        })
    }

    pub fn from_file(file: curseforge::File, mod_info: curseforge::Mod) -> anyhow::Result<Self> {
        let download_url = file.download_url.clone().ok_or_else(|| {
            anyhow::anyhow!("mod cannot be downloaded without privileged api key")
        })?;

        Ok(Self {
            addon_type: mod_info.class_id.unwrap_or(curseforge::ClassId::Mods),
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
        let install_path = match self.addon_type {
            curseforge::ClassId::Mods => instance_path.get_mods_path(),
            curseforge::ClassId::ResourcePacks => instance_path.get_resourcepacks_path(),
            curseforge::ClassId::Worlds => instance_path.get_saves_path(),
            curseforge::ClassId::Shaders => instance_path.get_shaderpacks_path(),
            curseforge::ClassId::Datapacks => instance_path.get_datapacks_path(),
            _ => instance_path.get_mods_path(),
        }
        .join(&self.file.file_name);

        let checksums = &self
            .file
            .hashes
            .iter()
            .map(|hash| match hash.algo {
                curseforge::HashAlgo::Sha1 => Checksum::Sha1(hash.value.clone()),
                curseforge::HashAlgo::Md5 => Checksum::Md5(hash.value.clone()),
            })
            .collect::<Vec<_>>();

        let size = &self.file.file_length;

        Some(
            Downloadable::new(&self.download_url, install_path)
                .with_checksum(checksums.get(0).cloned())
                .with_size(*size as u64),
        )
    }

    fn dependencies(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_data: &InstanceData,
        preferred_channel: ModChannel,
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
                        let existing = app_clone
                            .prisma_client
                            .mod_file_cache()
                            .find_first(vec![
                                fcdb::instance_id::equals(*instance_id),
                                fcdb::metadata::is(vec![metadb::curseforge::is(vec![
                                    cfdb::project_id::equals(mod_id),
                                ])]),
                            ])
                            .exec()
                            .await;

                        if let Ok(Some(_)) = existing {
                            return None;
                        }

                        let platform = &app_clone.modplatforms_manager().curseforge;

                        let files = platform.get_mod_files(ModFilesParameters {
                            mod_id,
                            query: ModFilesParametersQuery {
                                game_version: None,
                                game_version_type_id: None,
                                index: None,
                                page_size: None,
                                mod_loader_type: None,
                            },
                        });

                        let mods = platform.get_mod(ModParameters {
                            mod_id: mod_id as i32,
                        });

                        let r = tokio::try_join!(files, mods)
                            .and_then(|(files, mod_info)| {
                                // select an appropriate file based on game version and loader, or
                                // the first file if that fails
                                if let Some((release, modloaders)) = game_version {
                                    let modloader_strings: Vec<String> = modloaders
                                        .iter()
                                        .map(|modloader| match modloader.type_ {
                                            domain::instance::info::ModLoaderType::Neoforge => {
                                                "neoforge".to_string()
                                            }
                                            domain::instance::info::ModLoaderType::Forge => {
                                                "forge".to_string()
                                            }
                                            domain::instance::info::ModLoaderType::Fabric => {
                                                "fabric".to_string()
                                            }
                                            domain::instance::info::ModLoaderType::Quilt => {
                                                "quilt".to_string()
                                            }
                                        })
                                        .collect();

                                    let mut matching_versions = files
                                        .data
                                        .iter()
                                        .filter(|&file| {
                                            let has_release = file.game_versions.contains(&release);
                                            let has_one_of_our_modloaders =
                                                file.game_versions.iter().any(|ver| {
                                                    modloader_strings.contains(&ver.to_lowercase())
                                                });
                                            has_release && has_one_of_our_modloaders
                                        })
                                        .peekable();

                                    let mut matched_version = matching_versions.peek().map(|f| *f);
                                    let mut matched_channel = ModChannel::Alpha;

                                    for version in matching_versions {
                                        let channel = ModChannel::from(version.release_type);

                                        if channel > matched_channel {
                                            matched_version = Some(version);
                                            matched_channel = channel;
                                        }

                                        if channel >= preferred_channel {
                                            break;
                                        }
                                    }

                                    if let Some(file) = matched_version {
                                        Ok((file.clone(), mod_info))
                                    } else {
                                        files
                                            .data
                                            .first()
                                            .cloned()
                                            .ok_or_else(|| anyhow::anyhow!("no files found"))
                                            .map(|file| (file, mod_info))
                                    }
                                } else {
                                    files
                                        .data
                                        .first()
                                        .cloned()
                                        .ok_or_else(|| anyhow::anyhow!("no files found"))
                                        .map(|file| (file, mod_info))
                                }
                            })
                            .and_then(|(file, mod_info)| {
                                CurseforgeModInstaller::from_file(file, mod_info.data)
                                    .map(|installer| Box::new(installer) as BoxedResourceInstaller)
                            });

                        Some(r)
                    })
                }));
            }
        }
        DependencyIterator::new(installers.into_iter())
    }

    async fn is_already_installed(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
    ) -> anyhow::Result<bool> {
        use carbon_repos::db::mod_file_cache as fcdb;

        // TODO: check with fingerprint?
        let is_installed = app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::InstanceIdFilenameEquals(
                *instance_id,
                self.file.file_name.clone(),
            ))
            .exec()
            .await?
            .is_some();

        Ok(is_installed)
    }

    fn display_name(&self) -> String {
        self.file.display_name.clone()
    }

    async fn rollback(&self, _instance_data: &mut InstanceData) -> anyhow::Result<()> {
        let mut lock = self.applied_data.lock().await;
        if let Some((_, downloadable)) = &*lock {
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

    fn get_addon_type(&self) -> crate::domain::instance::AddonType {
        match self.addon_type {
            curseforge::ClassId::Mods => crate::domain::instance::AddonType::Mods,
            curseforge::ClassId::ResourcePacks => crate::domain::instance::AddonType::ResourcePacks,
            curseforge::ClassId::Shaders => crate::domain::instance::AddonType::Shaders,
            curseforge::ClassId::Datapacks => crate::domain::instance::AddonType::DataPacks,
            curseforge::ClassId::Worlds => crate::domain::instance::AddonType::Worlds,
            _ => crate::domain::instance::AddonType::Mods,
        }
    }
}

impl IntoInstaller for CurseforgeModInstaller {
    fn into_installer(self) -> Installer {
        Installer::new(Box::new(self) as BoxedResourceInstaller)
    }
}

// modrinth
pub struct ModrinthModInstaller {
    version: modrinth::version::Version,
    file: modrinth::version::VersionFile,
    mod_info: modrinth::project::Project,
    download_url: String,
    applied_data: Arc<Mutex<Option<(Mod, Downloadable)>>>,
}

impl ModrinthModInstaller {
    pub async fn create(
        app: &Arc<AppInner>,
        project_id: String,
        version_id: String,
    ) -> anyhow::Result<Self> {
        let platform = &app.modplatforms_manager().modrinth;

        let version = platform.get_version(VersionID(version_id.clone()));
        let project = platform.get_project(ProjectID(project_id.clone()));

        let (version, project) = tokio::try_join!(version, project)?;

        let file = version
            .files
            .iter()
            .reduce(|a, b| if b.primary { b } else { a })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Modrinth project '{}' version '{}' does not have a file",
                    &project_id,
                    &version_id
                )
            })?;

        let download_url = file.url.clone();

        Ok(Self {
            version,
            file,
            mod_info: project,
            download_url,
            applied_data: Arc::new(Mutex::new(None)),
        })
    }

    pub fn from_version(
        version: modrinth::version::Version,
        project: modrinth::project::Project,
    ) -> anyhow::Result<Self> {
        let file = version
            .files
            .iter()
            .reduce(|a, b| if b.primary { b } else { a })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Modrinth project '{}' version '{}' does not have a file",
                    &version.project_id,
                    &version.id
                )
            })?;

        let download_url = file.url.clone();

        Ok(Self {
            version,
            file,
            mod_info: project,
            download_url,
            applied_data: Arc::new(Mutex::new(None)),
        })
    }
}

#[async_trait::async_trait]
impl ResourceInstaller for ModrinthModInstaller {
    fn id(&self) -> String {
        format!("modrinth:{}:{}", &self.version.project_id, &self.version.id)
    }

    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable> {
        use modrinth::project::ProjectType;
        let install_path = match self.mod_info.project_type {
            ProjectType::Mod => instance_path.get_mods_path(),
            ProjectType::ResourcePack => instance_path.get_resourcepacks_path(),
            ProjectType::Shader => instance_path.get_shaderpacks_path(),
            ProjectType::DataPack => instance_path.get_datapacks_path(),
            _ => instance_path.get_mods_path(),
        }
        .join(&self.file.filename);

        let checksum = Checksum::Sha1(self.file.hashes.sha1.clone());
        let size = self.file.size;

        Some(
            Downloadable::new(&self.download_url, install_path)
                .with_checksum(Some(checksum))
                .with_size(size as u64),
        )
    }

    fn dependencies(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_data: &InstanceData,
        preferred_channel: ModChannel,
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
        for dep in &self.version.dependencies {
            let app_clone = Arc::clone(app);
            let version_id = dep.version_id.clone();
            let project_id = dep.project_id.clone();
            let game_version = game_version.clone();

            if let modrinth::version::DependencyType::Required = dep.dependency_type {
                if let Some(project_id) = project_id {
                    installers.push(Box::new(move || {
                        Box::pin(async move {
                            let existing = app_clone
                                .prisma_client
                                .mod_file_cache()
                                .find_first(vec![
                                    fcdb::instance_id::equals(*instance_id),
                                    fcdb::metadata::is(vec![metadb::modrinth::is(vec![
                                        mrdb::project_id::equals(project_id.clone()),
                                    ])]),
                                ])
                                .exec()
                                .await;

                            if let Ok(Some(_)) = existing {
                                return None;
                            }

                            if let Some(version_id) = version_id {
                                let platform = &app_clone.modplatforms_manager().modrinth;

                                let version = platform.get_version(VersionID(version_id));

                                let project = platform.get_project(ProjectID(project_id.clone()));

                                return Some(tokio::try_join!(version, project).and_then(
                                    |(version, project)| {
                                        ModrinthModInstaller::from_version(version, project).map(
                                            |installer| {
                                                Box::new(installer) as BoxedResourceInstaller
                                            },
                                        )
                                    },
                                ));
                            }

                            let platform = &app_clone.modplatforms_manager().modrinth;

                            let versions = platform.get_project_versions(ProjectVersionsFilters {
                                project_id: ProjectID(project_id.clone()),
                                game_versions: None,
                                loaders: None,
                                limit: None,
                                offset: None,
                            });

                            let project = platform.get_project(ProjectID(project_id));

                            Some(
                                tokio::try_join!(versions, project)
                                    .and_then(|(versions, project)| {
                                        if let Some((release, modloaders)) = game_version {
                                            let modloader_strings: Vec<String> = modloaders
                                                .iter()
                                                .map(|modloader| {
                                                    match modloader.type_ {
                                                domain::instance::info::ModLoaderType::Neoforge => {
                                                    "neoforge".to_string()
                                                }
                                                domain::instance::info::ModLoaderType::Forge => {
                                                    "forge".to_string()
                                                }
                                                domain::instance::info::ModLoaderType::Fabric => {
                                                    "fabric".to_string()
                                                }
                                                domain::instance::info::ModLoaderType::Quilt => {
                                                    "quilt".to_string()
                                                }
                                            }
                                                })
                                                .collect();

                                            let mut matching_versions = versions
                                                .iter()
                                                .filter(|&version| {
                                                    let has_release =
                                                        version.game_versions.contains(&release);
                                                    let has_one_of_our_modloaders =
                                                        version.loaders.iter().any(|loader| {
                                                            modloader_strings
                                                                .contains(&loader.to_lowercase())
                                                        });
                                                    has_release && has_one_of_our_modloaders
                                                })
                                                .peekable();

                                            let mut matched_version =
                                                matching_versions.peek().map(|f| *f);
                                            let mut matched_channel = ModChannel::Alpha;

                                            for version in matching_versions {
                                                let channel =
                                                    ModChannel::from(version.version_type);

                                                if channel > matched_channel {
                                                    matched_version = Some(version);
                                                    matched_channel = channel;
                                                }

                                                if channel >= preferred_channel {
                                                    break;
                                                }
                                            }

                                            if let Some(version) = matched_version {
                                                Ok((version.clone(), project))
                                            } else {
                                                versions
                                                    .first()
                                                    .cloned()
                                                    .ok_or_else(|| {
                                                        anyhow::anyhow!("no versions found")
                                                    })
                                                    .map(|version| (version, project))
                                            }
                                        } else {
                                            versions
                                                .first()
                                                .cloned()
                                                .ok_or_else(|| anyhow::anyhow!("no versions found"))
                                                .map(|version| (version, project))
                                        }
                                    })
                                    .and_then(|(version, project)| {
                                        ModrinthModInstaller::from_version(version, project).map(
                                            |installer| {
                                                Box::new(installer) as BoxedResourceInstaller
                                            },
                                        )
                                    }),
                            )
                        })
                    }));
                }
            }
        }
        DependencyIterator::new(installers.into_iter())
    }

    async fn is_already_installed(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
    ) -> anyhow::Result<bool> {
        use carbon_repos::db::mod_file_cache as fcdb;

        // TODO: check with fingerprint?
        let is_installed = app
            .prisma_client
            .mod_file_cache()
            .find_unique(fcdb::UniqueWhereParam::InstanceIdFilenameEquals(
                *instance_id,
                self.file.filename.clone(),
            ))
            .exec()
            .await?
            .is_some();

        Ok(is_installed)
    }

    fn display_name(&self) -> String {
        self.version.name.clone()
    }

    async fn rollback(&self, _instance_data: &mut InstanceData) -> anyhow::Result<()> {
        let mut lock = self.applied_data.lock().await;
        if let Some((_applied_mod_data, downloadable)) = &*lock {
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

    fn get_addon_type(&self) -> crate::domain::instance::AddonType {
        use modrinth::project::ProjectType;
        match self.mod_info.project_type {
            ProjectType::Mod => crate::domain::instance::AddonType::Mods,
            ProjectType::ResourcePack => crate::domain::instance::AddonType::ResourcePacks,
            ProjectType::Shader => crate::domain::instance::AddonType::Shaders,
            ProjectType::DataPack => crate::domain::instance::AddonType::DataPacks,
            ProjectType::Modpack | ProjectType::Plugin | ProjectType::Unknown => {
                crate::domain::instance::AddonType::Mods
            }
        }
    }
}

impl IntoInstaller for ModrinthModInstaller {
    fn into_installer(self) -> Installer {
        Installer::new(Box::new(self) as BoxedResourceInstaller)
    }
}
