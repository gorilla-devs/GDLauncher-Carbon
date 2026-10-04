pub(crate) mod publication;
use super::{InstanceData, InvalidInstanceIdError};
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
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_rt_path::InstancePath;
use futures::future::Future;
use std::{ops::Deref, path::PathBuf, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, watch},
    task::AbortHandle,
};
use tokio_util::sync::CancellationToken;

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
    fn addon_type(&self) -> instance::AddonType {
        instance::AddonType::Mods
    }
    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable>;
    fn get_install_path(&self, instance_path: &InstancePath) -> PathBuf;
    async fn post_process(
        &self,
        instance_path: &InstancePath,
        staged: &std::path::Path,
    ) -> Result<(), anyhow::Error>;
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
}

#[async_trait::async_trait]
impl<I: ResourceInstaller + ?Sized + Send> ResourceInstaller for Box<I> {
    #[inline]
    fn id(&self) -> String {
        (**self).id()
    }

    fn addon_type(&self) -> instance::AddonType {
        (**self).addon_type()
    }

    #[inline]
    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable> {
        (**self).downloadable(instance_path).await
    }

    #[inline]
    fn get_install_path(&self, instance_path: &InstancePath) -> PathBuf {
        (**self).get_install_path(instance_path)
    }

    #[inline]
    async fn post_process(
        &self,
        instance_path: &InstancePath,
        staged: &std::path::Path,
    ) -> Result<(), anyhow::Error> {
        (**self).post_process(instance_path, staged).await
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

#[derive(Clone)]
struct InstallerRollbackContext {
    ownership: Arc<InstallOwnership>,
    inner: Arc<Mutex<BoxedResourceInstaller>>,
    processed_deps: Arc<Mutex<Vec<Arc<Installer>>>>,
    instance_id: InstanceId,
    app: Arc<AppInner>,
}

impl InstallerRollbackContext {
    pub async fn rollback(&self, inciting_error: Option<&anyhow::Error>) -> anyhow::Result<()> {
        // Never wait for this error handler's own ACK. Ownership settlement is
        // recursive even when an ancestor completed its worker or had no worker.
        self.ownership.settle_children().await?;
        let parent_name = self.inner.lock().await.display_name();
        let processed_deps = self.processed_deps.lock().await;
        let instance_manager = self.app.instance_manager();
        let _operation = instance_manager.lock_instance(self.instance_id).await;
        let mut instances = instance_manager.instances.write().await;
        let Some(instance) = instances.get_mut(&self.instance_id) else {
            return Ok(());
        };
        let Ok(data) = instance.data_mut() else {
            return Ok(());
        };
        for dep in processed_deps.iter() {
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
        Ok(())
    }
}

pub struct Installer {
    pub(super) required_enabled: Option<super::shader_loader::ShaderLoaderVerification>,
    #[cfg(test)]
    download_fixture: Option<PathBuf>,
    inner: Arc<Mutex<BoxedResourceInstaller>>,
    rollback_context: Arc<Mutex<Option<InstallerRollbackContext>>>,
    abort_handle: Arc<Mutex<InstallerAbortHandle>>,
    admission: Arc<Mutex<()>>,
    ownership: Arc<InstallOwnership>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AdmissionPhase {
    New,
    Running,
    Complete,
    Unknown,
}

// Live-work ownership is independent of Task10's completed-artifact bookkeeping.
// A node is registered before any recursive work and outlives its own worker ACK.
struct InstallOwnership {
    owner: Arc<Mutex<InstallerAbortHandle>>,
    admission: Arc<Mutex<()>>,
    phase: std::sync::Mutex<AdmissionPhase>,
    cancel: CancellationToken,
    children: std::sync::Mutex<Vec<Arc<InstallOwnership>>>,
}
struct AdmissionRecord {
    node: Arc<InstallOwnership>,
    finished: bool,
}
impl AdmissionRecord {
    fn begin(node: Arc<InstallOwnership>) -> Self {
        *node.phase.lock().unwrap() = AdmissionPhase::Running;
        Self {
            node,
            finished: false,
        }
    }
    fn finish(mut self) {
        *self.node.phase.lock().unwrap() = AdmissionPhase::Complete;
        self.finished = true;
    }
}
impl Drop for AdmissionRecord {
    fn drop(&mut self) {
        if !self.finished {
            *self.node.phase.lock().unwrap() = AdmissionPhase::Unknown;
            self.node.cancel.cancel();
        }
    }
}
impl InstallOwnership {
    #[async_recursion::async_recursion]
    async fn settle(&self) -> anyhow::Result<()> {
        self.cancel.cancel();
        self.owner.lock().await.request_cancel();
        let admission = self.admission.lock().await;
        let unknown = *self.phase.lock().unwrap() == AdmissionPhase::Unknown;
        let completion = self.owner.lock().await.completion.clone();
        drop(admission);
        let own = wait_install_completion(completion).await;
        // Drain all known children even when our own completion is uncertain.
        let children = self.settle_children().await;
        if unknown {
            bail!("Installer admission ended without an acknowledgment; settlement unknown");
        }
        own?;
        children
    }
    async fn settle_children(&self) -> anyhow::Result<()> {
        let children = self.children.lock().unwrap().clone();
        for child in &children {
            child.cancel.cancel();
        }
        let mut failure = None;
        for child in children {
            if let Err(error) = child.settle().await {
                if failure.is_none() {
                    failure = Some(error);
                }
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

struct InstallerAbortHandle {
    pub handle: Option<AbortHandle>,
    pub aborted: bool,
    cancel: CancellationToken,
    completion: Option<watch::Receiver<bool>>,
}

impl InstallerAbortHandle {
    fn spawn<F>(&mut self, work: impl FnOnce(CancellationToken) -> F + Send + 'static)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let cancel = self.cancel.clone();
        let (completed, completion) = watch::channel(false);
        self.completion = Some(completion);
        let task = tokio::spawn(async move {
            work(cancel).await;
            // Acknowledgment is emitted after the entire owned operation returns.
            // Panic/abort closes the channel without authorizing filesystem rollback.
            let _ = completed.send(true);
        });
        self.handle = Some(task.abort_handle());
    }
    fn request_cancel(&mut self) -> Option<watch::Receiver<bool>> {
        self.aborted = true;
        self.cancel.cancel();
        self.completion.clone()
    }
}

async fn wait_install_completion(completion: Option<watch::Receiver<bool>>) -> anyhow::Result<()> {
    if let Some(mut completion) = completion {
        completion
            .wait_for(|finished| *finished)
            .await
            .context("Installer exited without acknowledging IO settlement")?;
    }
    Ok(())
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
        Self::with_cancel(resource_installer, CancellationToken::new())
    }

    fn with_cancel(resource_installer: BoxedResourceInstaller, cancel: CancellationToken) -> Self {
        let abort_handle = Arc::new(Mutex::new(InstallerAbortHandle {
            handle: None,
            aborted: false,
            cancel: cancel.clone(),
            completion: None,
        }));
        let admission = Arc::new(Mutex::new(()));
        let ownership = Arc::new(InstallOwnership {
            owner: abort_handle.clone(),
            admission: admission.clone(),
            phase: std::sync::Mutex::new(AdmissionPhase::New),
            cancel,
            children: std::sync::Mutex::new(Vec::new()),
        });
        Self {
            required_enabled: None,
            #[cfg(test)]
            download_fixture: None,
            inner: Arc::new(Mutex::new(resource_installer)),
            rollback_context: Arc::new(Mutex::new(None)),
            admission,
            abort_handle,
            ownership,
        }
    }

    /// Ok proves admission and every admitted descendant worker settled. Unknown
    /// admission or a missing worker ACK returns Err and forbids filesystem rollback.
    pub async fn abort(&self, inciting_error: Option<&anyhow::Error>) -> anyhow::Result<()> {
        self.ownership.settle().await?;
        self.rollback(inciting_error).await
    }

    async fn rollback(&self, inciting_error: Option<&anyhow::Error>) -> anyhow::Result<()> {
        let context = self.rollback_context.lock().await.clone();
        if let Some(context) = context {
            context.rollback(inciting_error).await?;
        }
        Ok(())
    }

    pub async fn install(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        install_deps: bool,
        replaces_mod_id: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        if let Some(id) = replaces_mod_id.as_deref() {
            app.instance_manager()
                .get_owned_mod_file(instance_id, id)
                .await?;
        }

        let download_deps = app
            .settings_manager()
            .get_settings()
            .await?
            .download_dependencies;

        let _activity = app.runtime_lifecycle.admit("addon install admission")?;
        let manager = app.instance_manager();
        let operation = manager.lock_instance(instance_id).await;
        manager.ensure_modpack_not_locked(instance_id).await?;
        if let Some(target) = &self.required_enabled {
            target.validate_target(app, instance_id).await?;
        }
        let (task, task_id, instance_path) = async {
            let (name, shortpath) = {
                let instances = manager.instances.read().await;
                let instance = instances
                    .get(&instance_id)
                    .ok_or(InvalidInstanceIdError(instance_id))?;
                (
                    instance.data()?.config.name.clone(),
                    instance.shortpath.clone(),
                )
            };

            let task = {
                let lock = self.inner.lock().await;

                if lock.is_already_installed(app, instance_id).await? {
                    // The UI's installed-state can lag behind the file cache (platform
                    // metadata still being scanned), so a plain install of a file that
                    // is already on disk is a successful no-op rather than an error.
                    // Dropping the task right after spawning completes it immediately.
                    // Explicit replacement continues into staged publication even when
                    // bytes match; it never follows the ordinary no-op branch.
                    if replaces_mod_id.is_none() {
                        if self.required_enabled.is_some() {
                            publication::enable_satisfied(&**lock, app, &operation).await?;
                        }
                        let task = VisualTask::new(Translation::InstanceTaskInstallMod {
                            mod_name: lock.display_name(),
                            instance_name: name.clone(),
                        });

                        let id = app.task_manager().spawn_task(&task).await;

                        // Nudge the UI to re-read the mods list, since its stale
                        // installed-state is the usual reason we ended up here
                        app.invalidate(INSTANCE_MODS, Some(instance_id.0.into()));

                        return Ok::<_, anyhow::Error>((None, id, None));
                    }

                    // An explicit replacement must still publish and preserve its intent.
                }

                let task = VisualTask::new(
                    if replaces_mod_id.is_some() && lock.addon_type() == instance::AddonType::Worlds
                    {
                        Translation::InstanceTaskInstallWorldCopy {
                            world_name: lock.display_name(),
                            instance_name: name.clone(),
                        }
                    } else {
                        Translation::InstanceTaskInstallMod {
                            mod_name: lock.display_name(),
                            instance_name: name.clone(),
                        }
                    },
                );

                Ok::<VisualTask, anyhow::Error>(task)
            }?;

            let instance_path = app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath);

            let id = app.task_manager().spawn_task(&task).await;
            Ok((Some(task), id, Some(instance_path)))
        }
        .await?;

        let (Some(task), Some(instance_path)) = (task, instance_path) else {
            return Ok(task_id);
        };

        let replacement = if let Some(id) = replaces_mod_id.as_deref() {
            Some(
                publication::ReplacementPlan::capture(
                    app,
                    instance_id,
                    id,
                    self.inner.lock().await.addon_type(),
                )
                .await?,
            )
        } else {
            None
        };
        drop(operation);

        let visited_ids = Arc::new(Mutex::new(Vec::new()));
        let task = Arc::new(Mutex::new(task));
        self.install_inner(
            app,
            instance_id,
            &instance_path,
            &task,
            &visited_ids,
            install_deps && download_deps,
            replacement,
        )
        .await?;

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
        replacement: Option<publication::ReplacementPlan>,
    ) -> anyhow::Result<()> {
        let admission = self.admission.lock().await;
        if self.ownership.cancel.is_cancelled() {
            bail!("Addon installation cancelled");
        }
        let record = AdmissionRecord::begin(self.ownership.clone());
        let result = self
            .admit_inner(
                app,
                instance_id,
                instance_path,
                parent_task,
                visited_ids,
                install_deps,
                replacement,
            )
            .await;
        let result = match result {
            Ok(()) => Ok(()),
            Err(error) => {
                self.ownership.cancel.cancel();
                // Admission errors occur before our own worker is spawned. Retain
                // this node's admission guard through its error cleanup, but never
                // wait for our own admission/ACK. Children do not need this guard.
                // A dropped cleanup must remain Unknown to a concurrent abort.
                let error = match self.ownership.settle_children().await {
                    Err(settlement) => {
                        error.context(format!("Installer settlement is uncertain: {settlement:#}"))
                    }
                    Ok(()) => match self.rollback(Some(&error)).await {
                        Ok(()) => error,
                        Err(rollback) => {
                            error.context(format!("Installer rollback failed: {rollback:#}"))
                        }
                    },
                };
                Err(error)
            }
        };
        record.finish();
        drop(admission);
        result
    }

    async fn admit_inner(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_path: &InstancePath,
        parent_task: &Arc<Mutex<VisualTask>>,
        visited_ids: &Arc<Mutex<Vec<String>>>,
        install_deps: bool,
        replacement: Option<publication::ReplacementPlan>,
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

        let processed_deps = Arc::new(Mutex::new(Vec::new()));
        *self.rollback_context.lock().await = Some(InstallerRollbackContext {
            ownership: self.ownership.clone(),
            inner: self.inner.clone(),
            processed_deps: processed_deps.clone(),
            instance_id,
            app: app.clone(),
        });
        if install_deps {
            let dep_error = {
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

                let mut dep_error = None;

                for dep in dep_iter {
                    if self.ownership.cancel.is_cancelled() {
                        dep_error = Some(anyhow::anyhow!("Addon installation cancelled"));
                        break;
                    }
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
                            let dep = Arc::new(Installer::with_cancel(
                                dep,
                                self.ownership.cancel.child_token(),
                            ));
                            // Register BOTH live-work and rollback identity before child admission,
                            // including a child whose own admission later returns an error.
                            self.ownership
                                .children
                                .lock()
                                .unwrap()
                                .push(dep.ownership.clone());
                            processed_deps.lock().await.push(dep.clone());
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
                                Ok(()) => {}
                            }
                        }
                    }
                }

                dep_error
            };

            if let Some(dep_error) = dep_error {
                return Err(dep_error);
            }

            if replacement.is_none()
                && self.required_enabled.is_none()
                && self
                    .inner
                    .lock()
                    .await
                    .is_already_installed(app, instance_id)
                    .await?
            {
                return Ok(());
            }
        }

        let t_download_file = {
            let lock = parent_task.lock().await;
            lock.subtask(Translation::InstanceTaskInstallModDownloadFile)
        };

        let required_enabled = self.required_enabled.clone();
        let activity = app
            .runtime_lifecycle
            .admit("addon download and publication")?;
        #[cfg(test)]
        let download_fixture = self.download_fixture.clone();
        let instance_path = instance_path.clone();
        let inner = Arc::clone(&self.inner);
        let app_clone = Arc::clone(app);
        let rollback_context = Arc::clone(&self.rollback_context);
        let parent_task = Arc::clone(parent_task);

        {
            let mut abort_handle = self.abort_handle.lock().await;
            if abort_handle.aborted || self.ownership.cancel.is_cancelled() {
                bail!("Addon installation cancelled");
            }
            {
                abort_handle.spawn(move |cancel| async move {
                    let _activity = activity;
                    // Preserve failed private staging for recovery. Explicit cancellation
                    // settles download and publication IO before rollback can inspect it.
                    let staging = tempfile::Builder::new()
                        .prefix(".addon-install-")
                        .tempdir_in(
                            app_clone
                                .settings_manager()
                                .runtime_path
                                .get_root()
                                .to_path(),
                        );
                    let r = (|| async {
                        if cancel.is_cancelled() {
                            bail!("Addon installation cancelled");
                        }
                        let staging = staging?.keep();
                        let mut downloadable = {
                            let lock = inner.lock().await;
                            lock.downloadable(&instance_path).await
                        };

                        if let Some(download) = downloadable.as_mut() {
                            download.path = staging.join("artifact");
                        }

                        parent_task
                            .lock()
                            .await
                            .edit(|data| data.state = TaskState::KnownProgress)
                            .await;

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
                                            false,
                                        );
                                    }

                                    tokio::time::sleep(Duration::from_millis(30)).await;
                                }

                                t_download_file.complete_download();
                            });

                            #[cfg(test)]
                            if let Some(source) = &download_fixture {
                                tokio::fs::create_dir_all(downloadable.path.parent().unwrap())
                                    .await?;
                                tokio::fs::copy(source, &downloadable.path).await?;
                            }
                            #[cfg(test)]
                            let acquire = download_fixture.is_none();
                            #[cfg(not(test))]
                            let acquire = true;
                            if acquire {
                                carbon_net::download_multiple(
                                    &[downloadable.clone()],
                                    DownloadOptions::builder()
                                        .concurrency(1)
                                        .cancel_token(cancel.clone())
                                        .build(),
                                )
                                .await
                                .with_context(|| {
                                    format!(
                                        "Failed to download addon file for `{:?}`",
                                        downloadable
                                    )
                                })?;
                            }

                            {
                                if cancel.is_cancelled() {
                                    bail!("Addon installation cancelled");
                                }
                                let lock = inner.lock().await;
                                publication::verify_acquired(&downloadable.path, downloadable)
                                    .await?;
                                if lock.addon_type() == instance::AddonType::Worlds {
                                    let manager = app_clone.instance_manager();
                                    let _operation = manager.lock_instance(instance_id).await;
                                    manager.ensure_modpack_not_locked(instance_id).await?;
                                    let current =
                                        publication::current_path(&app_clone, instance_id).await?;
                                    publication::filename(&**lock, &current)?;
                                    if let Some(plan) = replacement.as_ref() {
                                        let destination = publication::publish_world_version(
                                            &**lock,
                                            &app_clone,
                                            &_operation,
                                            &downloadable.path,
                                            plan,
                                        )
                                        .await?;
                                        parent_task
                                            .lock()
                                            .await
                                            .edit(|data| {
                                                if let Translation::InstanceTaskInstallWorldCopy {
                                                    world_name,
                                                    ..
                                                } = &mut data.name
                                                {
                                                    *world_name = destination
                                                        .file_name()
                                                        .unwrap()
                                                        .to_string_lossy()
                                                        .into_owned();
                                                }
                                            })
                                            .await;
                                    } else {
                                        lock.post_process(&current, &downloadable.path).await?;
                                    }
                                } else {
                                    publication::publish_file(
                                        &**lock,
                                        &app_clone,
                                        instance_id,
                                        &downloadable.path,
                                        replacement.as_ref(),
                                        required_enabled.as_ref(),
                                    )
                                    .await?;
                                }
                            }
                            if super::mods::addon_metadata(&downloadable.path)
                                .await?
                                .is_some()
                            {
                                tokio::fs::remove_file(&downloadable.path).await?;
                            }
                            tokio::fs::remove_dir(&staging).await?;
                        }

                        if cancel.is_cancelled() {
                            bail!("Addon installation cancelled");
                        }
                        // ensure the task stays alive until the mod is cached
                        app_clone
                            .meta_cache_manager()
                            .override_caching_and_wait(
                                crate::managers::metadata::cache::CacheEntityId::Instance(
                                    instance_id,
                                ),
                                true,
                                true,
                            )
                            .await?;

                        app_clone.invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
                        Ok::<_, anyhow::Error>(())
                    })()
                    .await;

                    match r {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::error!({ error = ?e }, "Error installing dependency");

                            let context = rollback_context.lock().await.clone();
                            let error = if let Some(context) = context {
                                match context.rollback(Some(&e)).await {
                                    Ok(()) => e,
                                    Err(settlement) => e.context(format!("Installer descendant settlement/rollback failed: {settlement:#}")),
                                }
                            } else { e.context("Missing installer rollback ownership") };
                            let parent_task = parent_task.lock().await;
                            parent_task.clone().fail(error).await
                        }
                    }
                });
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
    fn addon_type(&self) -> instance::AddonType {
        match self.addon_type {
            curseforge::ClassId::ResourcePacks => instance::AddonType::ResourcePacks,
            curseforge::ClassId::Worlds => instance::AddonType::Worlds,
            curseforge::ClassId::Shaders => instance::AddonType::Shaders,
            curseforge::ClassId::Datapacks => instance::AddonType::DataPacks,
            _ => instance::AddonType::Mods,
        }
    }

    fn id(&self) -> String {
        format!("curseforge:{}:{}", &self.file.mod_id, &self.file.id)
    }

    fn get_install_path(&self, instance_path: &InstancePath) -> PathBuf {
        match self.addon_type {
            curseforge::ClassId::Mods => instance_path.get_mods_path(),
            curseforge::ClassId::ResourcePacks => instance_path.get_resourcepacks_path(),
            curseforge::ClassId::Worlds => instance_path.get_saves_path(),
            curseforge::ClassId::Shaders => instance_path.get_shaderpacks_path(),
            curseforge::ClassId::Datapacks => instance_path.get_datapacks_path(),
            _ => instance_path.get_mods_path(),
        }
        .join(&self.file.file_name)
    }

    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable> {
        let install_path = self.get_install_path(instance_path);

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

    async fn post_process(
        &self,
        instance_path: &InstancePath,
        staged: &std::path::Path,
    ) -> Result<(), anyhow::Error> {
        if self.addon_type == curseforge::ClassId::Worlds {
            let name = std::path::Path::new(&self.file.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .context("invalid world archive name")?;
            let saves =
                publication::checked_folder(instance_path, instance::AddonType::Worlds).await?;
            publication::publish_world(staged, &saves, name).await?;
        }
        Ok(())
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
                        let instance_id_val = *instance_id;
                        let existing = mfcdb::instance_mod_exists_by_cf_project(
                            &app_clone.db,
                            instance_id_val,
                            mod_id as i32,
                        )
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
        publication::is_satisfied(self, app, instance_id).await
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
    fn addon_type(&self) -> instance::AddonType {
        match self.mod_info.project_type {
            modrinth::project::ProjectType::ResourcePack => instance::AddonType::ResourcePacks,
            modrinth::project::ProjectType::Shader => instance::AddonType::Shaders,
            modrinth::project::ProjectType::DataPack => instance::AddonType::DataPacks,
            _ => instance::AddonType::Mods,
        }
    }

    fn id(&self) -> String {
        format!("modrinth:{}:{}", &self.version.project_id, &self.version.id)
    }

    fn get_install_path(&self, instance_path: &InstancePath) -> PathBuf {
        match self.mod_info.project_type {
            modrinth::project::ProjectType::Mod => instance_path.get_mods_path(),
            modrinth::project::ProjectType::ResourcePack => instance_path.get_resourcepacks_path(),
            modrinth::project::ProjectType::Shader => instance_path.get_shaderpacks_path(),
            modrinth::project::ProjectType::DataPack => instance_path.get_datapacks_path(),
            _ => instance_path.get_mods_path(),
        }
        .join(&self.file.filename)
    }

    async fn downloadable(&self, instance_path: &InstancePath) -> Option<Downloadable> {
        let install_path = self.get_install_path(instance_path);

        let checksum = Checksum::Sha1(self.file.hashes.sha1.clone());
        let size = self.file.size;

        Some(
            Downloadable::new(&self.download_url, install_path)
                .with_checksum(Some(checksum))
                .with_size(size as u64),
        )
    }

    async fn post_process(
        &self,
        instance_path: &InstancePath,
        staged: &std::path::Path,
    ) -> Result<(), anyhow::Error> {
        // No post-processing required for modrinth
        Ok(())
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
                            let instance_id_val = *instance_id;
                            let project_id_owned = project_id.clone();
                            let existing = mfcdb::instance_mod_exists_by_mr_project(
                                &app_clone.db,
                                instance_id_val,
                                &project_id_owned,
                            )
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
        publication::is_satisfied(self, app, instance_id).await
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
}

impl IntoInstaller for ModrinthModInstaller {
    fn into_installer(self) -> Installer {
        Installer::new(Box::new(self) as BoxedResourceInstaller)
    }
}

#[cfg(test)]
mod tests;
