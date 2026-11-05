pub(crate) mod dependency;
mod graph;
mod planning;
use graph::InstallGraph;
pub(crate) use graph::LiveGraphs;
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
    dyn FnOnce(
            std::collections::HashMap<String, String>,
        ) -> Pin<
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
    /// Whether an incoming dependency declares this exact version.
    fn exact_dependency(&self) -> bool {
        true
    }
    /// This addon replaces the installed copy with these platform ids (an
    /// update). Its dependency resolution then only adds the requirements the
    /// replaced version did not already have (see dependency::Previous).
    fn replaces(&mut self, _previous: &dependency::ReplacedIds) {}
    /// Set on a dependency installed instead of the version its dependant
    /// pins, which is not for this game and loader.
    fn replaced_pin(&self) -> Option<dependency::ReplacedPin> {
        None
    }
    /// The project's name, as the user found it in search. Modrinth version
    /// names are often a bare number ("4.0").
    fn project_name(&self) -> String {
        self.display_name()
    }
    /// The version as the platform shows it next to the project's name.
    fn version_number(&self) -> String {
        self.display_name()
    }
    /// When the platform published this version, if it says.
    fn published(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }
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
    fn exact_dependency(&self) -> bool {
        (**self).exact_dependency()
    }
    fn replaces(&mut self, previous: &dependency::ReplacedIds) {
        (**self).replaces(previous)
    }
    fn replaced_pin(&self) -> Option<dependency::ReplacedPin> {
        (**self).replaced_pin()
    }
    fn project_name(&self) -> String {
        (**self).project_name()
    }
    fn version_number(&self) -> String {
        (**self).version_number()
    }
    fn published(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        (**self).published()
    }
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
    receipt: publication::ReceiptSlot,
    /// For a dependency, its graph's overlap flag ([`InstallGraph::overlapped`]):
    /// once another install ran on the instance meanwhile, a dependency this
    /// one published stays, since that install may rely on it. `None` for the
    /// addon the user picked.
    dependency_of: Option<Arc<std::sync::atomic::AtomicBool>>,
}

impl InstallerRollbackContext {
    pub async fn rollback(&self, inciting_error: Option<&anyhow::Error>) -> anyhow::Result<()> {
        // Never wait for this error handler's own ACK. Ownership settlement is
        // recursive even when an ancestor completed its worker or had no worker.
        self.ownership.settle_children().await?;
        let processed_deps = self.processed_deps.lock().await.clone();
        let mut errors = Vec::new();
        for dep in processed_deps.iter().rev() {
            if let Err(error) = Box::pin(dep.rollback(inciting_error)).await {
                errors.push(format!("{error:#}"));
            }
        }
        let shared = self
            .dependency_of
            .as_ref()
            .is_some_and(|overlapped| overlapped.load(std::sync::atomic::Ordering::Acquire));
        let kept = shared && publication::keep_new_dependency(&self.receipt).await;
        if !kept {
            if let Err(error) =
                publication::rollback_receipt(&self.app, self.instance_id, &self.receipt).await
            {
                errors.push(format!("{error:#}"));
            }
        }
        // Custom resource cleanup follows confirmed descendant settlement. Built-in
        // file rollback is exclusively receipt-based above.
        let manager = self.app.instance_manager();
        let _operation = manager.lock_instance(self.instance_id).await;
        let mut instances = manager.instances.write().await;
        if let Some(instance) = instances.get_mut(&self.instance_id) {
            if let Ok(data) = instance.data_mut() {
                if let Err(error) = self.inner.lock().await.rollback(data).await {
                    errors.push(format!("{error:#}"));
                }
            }
        }
        if !errors.is_empty() {
            bail!("Addon recovery incomplete: {}", errors.join("; "));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Installer {
    pub(super) required_enabled: Option<super::shader_loader::ShaderLoaderVerification>,
    #[cfg(test)]
    download_fixture: Option<PathBuf>,
    inner: Arc<Mutex<BoxedResourceInstaller>>,
    rollback_context: Arc<Mutex<Option<InstallerRollbackContext>>>,
    abort_handle: Arc<Mutex<InstallerAbortHandle>>,
    admission: Arc<Mutex<()>>,
    ownership: Arc<InstallOwnership>,
    receipt: publication::ReceiptSlot,
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
    async fn settle_runtime(
        &self,
        activity: &mut Option<crate::managers::runtime_lifecycle::RuntimeActivity>,
        cancel: CancellationToken,
    ) {
        let result = tokio::select! {
            result = self.wait() => result,
            _ = cancel.cancelled() => self.settle().await,
        };
        if let Err(error) = result {
            let owner = activity.take().expect("retained addon operation");
            owner.unknown(format!("Addon tree settlement unconfirmed: {error:#}"));
            // Drop retains Unknown; no rollback or new admission is authorized.
        }
    }

    // Observe normal completion without cancelling healthy siblings. The same
    // admission/ACK authority as settle applies, including Unknown rejection.
    #[async_recursion::async_recursion]
    async fn wait(&self) -> anyhow::Result<()> {
        let admission = self.admission.lock().await;
        let unknown = *self.phase.lock().unwrap() == AdmissionPhase::Unknown;
        let completion = self.owner.lock().await.completion.clone();
        drop(admission);
        let own = wait_install_completion(completion).await;
        let children = if unknown || own.is_err() {
            self.settle_children().await
        } else {
            self.wait_children().await
        };
        if unknown {
            bail!("Installer admission ended without an acknowledgment; settlement unknown");
        }
        own?;
        children
    }
    async fn wait_children(&self) -> anyhow::Result<()> {
        let children = self.children.lock().unwrap().clone();
        let mut failure = None;
        for child in children {
            if let Err(error) = child.wait().await {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
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
            receipt: Arc::new(Mutex::new(None)),
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
        let activity = app.runtime_lifecycle.admit_owned(
            "addon installation tree",
            crate::managers::runtime_lifecycle::ActivityPolicy::Installer,
        )?;
        let installer = self.clone();
        let app = app.clone();
        let (send, receive) = tokio::sync::oneshot::channel();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
            Box::pin(async move {
                let cancel = slot.as_ref().unwrap().cancellation();
                let admission =
                    installer.install_owned(&app, instance_id, install_deps, replaces_mod_id);
                tokio::pin!(admission);
                let result = tokio::select! {
                    result = &mut admission => result,
                    _ = cancel.cancelled() => {
                        installer.ownership.cancel.cancel();
                        admission.await
                    }
                };
                // Preserve the existing API's task-id timing, while the retained
                // operation continues through the existing tree ACK/rollback authority.
                let _ = send.send(result);
                installer.ownership.settle_runtime(slot, cancel).await;
            })
        });
        receive
            .await
            .context("Addon admission owner ended without result")?
    }

    async fn install_owned(
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

        let manager = app.instance_manager();
        let operation = manager.lock_instance(instance_id).await;
        manager.ensure_modpack_not_locked(instance_id).await?;
        if let Some(target) = &self.required_enabled {
            target.validate_target(app, instance_id).await?;
        }
        let (admitted, task_id) = async {
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
                    if replaces_mod_id.is_none() && !(install_deps && download_deps) {
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

                        return Ok::<_, anyhow::Error>((None, id));
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
                    } else if lock.addon_type() == instance::AddonType::DataPacks {
                        Translation::InstanceTaskStageDatapack {
                            mod_name: lock.display_name(),
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

            // Before the task shows: a refusal here (the file to update is a
            // link, is there twice or is gone) reaches the user as this
            // action's error, never as a failed task without its reason.
            let replacement = if let Some(id) = replaces_mod_id.as_deref() {
                let kind = self.inner.lock().await.addon_type();
                Some(publication::ReplacementPlan::capture(app, instance_id, id, kind).await?)
            } else {
                None
            };
            let visited_ids = Arc::new(InstallGraph::default());
            visited_ids
                .capture(app, instance_id, &instance_path)
                .await?;

            task.require_explicit_completion();
            let id = app.task_manager().spawn_task(&task).await;
            Ok((Some((task, instance_path, replacement, visited_ids)), id))
        }
        .await?;

        let Some((task, instance_path, replacement, visited_ids)) = admitted else {
            return Ok(task_id);
        };
        drop(operation);
        let task = Arc::new(Mutex::new(task));
        // A replacement keeps the replaced copy's state, so that state decides
        // whatever the caller asked: an enabled one gets the libraries its new
        // version adds, and a disabled one keeps the game as it is, as 2.0.40
        // did. A library would be published enabled and could stop the game
        // from starting, although the updated addon stays off; this holds for
        // an update and for a version picked in the addon's Versions list.
        let resolve_dependencies = match replacement.as_ref() {
            Some(plan) => plan.enabled(),
            None => install_deps,
        } && download_deps;
        if resolve_dependencies {
            if let Some(id) = replaces_mod_id.as_deref() {
                match mfcdb::get_instance_mod_update_ids(&app.db, id).await {
                    Ok(Some(row)) => self.inner.lock().await.replaces(&dependency::ReplacedIds {
                        curseforge: row.cf_project_id.zip(row.cf_file_id),
                        modrinth_version: row.mr_version_id,
                    }),
                    Ok(None) => {}
                    // Unknown: every missing requirement is resolved, as for a new install.
                    Err(error) => tracing::warn!(
                        "Could not read the platform ids of the addon an update replaces: {error:#}"
                    ),
                }
            }
            // Planning is part of the same Task6 admission authority. Abort
            // drains it, and a dropped planning future remains Unknown.
            let admission = self.admission.lock().await;
            let record = AdmissionRecord::begin(self.ownership.clone());
            let result = planning::resolve(
                self.inner.clone(),
                app,
                instance_id,
                replacement.is_some(),
                &self.ownership.cancel,
            )
            .await;
            record.finish();
            drop(admission);
            match result {
                Ok(resolution) => {
                    let subject = self.inner.lock().await.project_name();
                    let task = task.lock().await;
                    if !resolution.skipped.is_empty()
                        || !resolution.replaced.is_empty()
                        || !resolution.conflicting.is_empty()
                    {
                        // The toast names the project, not a version name like "4.0".
                        task.set_warning_subject(subject);
                    }
                    for skipped in resolution.skipped {
                        tracing::warn!(
                            dependency = %skipped.label(),
                            reason = ?skipped.reason,
                            "Installing without a required dependency that cannot be resolved or is disabled"
                        );
                        task.warn(skipped.into());
                    }
                    for conflicting in resolution.conflicting {
                        tracing::warn!(
                            dependency = %conflicting.name,
                            installed = %conflicting.installed,
                            "Installing one version of a required dependency that dependants pin at different versions"
                        );
                        task.warn(conflicting.into());
                    }
                    for replaced in resolution.replaced {
                        tracing::warn!(
                            dependency = %replaced.name,
                            pinned = %replaced.pinned,
                            installed = %replaced.installed,
                            "Installing another version of a required dependency than the pinned one"
                        );
                        task.warn(replaced.into());
                    }
                    *visited_ids.plan.lock().await = Some(resolution.plan);
                }
                Err(error) => {
                    task.lock()
                        .await
                        .clone()
                        .fail(anyhow::anyhow!("{error:#}"))
                        .await;
                    return Err(error);
                }
            }
        }
        let result = self
            .install_inner(
                app,
                instance_id,
                &instance_path,
                &task,
                &visited_ids,
                resolve_dependencies,
                replacement,
            )
            .await;
        if let Err(error) = result {
            task.lock()
                .await
                .clone()
                .fail(anyhow::anyhow!("{error:#}"))
                .await;
            return Err(error);
        }
        Ok(task_id)
    }

    #[async_recursion::async_recursion]
    async fn install_inner(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_path: &InstancePath,
        parent_task: &Arc<Mutex<VisualTask>>,
        visited_ids: &Arc<InstallGraph>,
        install_deps: bool,
        replacement: Option<publication::ReplacementPlan>,
    ) -> anyhow::Result<()> {
        let admission = self.admission.lock().await;
        if self.ownership.cancel.is_cancelled() {
            bail!("Addon installation cancelled");
        }
        let record = AdmissionRecord::begin(self.ownership.clone());
        let root = match visited_ids.enter(self.inner.lock().await.id()) {
            Ok(Some(root)) => root,
            Ok(None) => {
                record.finish();
                return Ok(());
            }
            Err(error) => {
                visited_ids.fail(&error);
                record.finish();
                return Err(error);
            }
        };
        if root {
            visited_ids.capture(app, instance_id, instance_path).await?;
        }
        let result = Box::pin(self.admit_inner(
            app,
            instance_id,
            instance_path,
            parent_task,
            visited_ids,
            install_deps,
            replacement,
            root,
        ))
        .await;
        let result = if root && result.is_ok() {
            self.retain_root_finalizer(app, instance_id, parent_task, visited_ids)
                .await
        } else {
            result
        };
        let result = match result {
            Ok(()) => Ok(()),
            Err(error) => {
                visited_ids.fail(&error);
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
        if root && result.is_ok() {
            visited_ids.planned();
            if visited_ids.is_complete() && !visited_ids.has_finalizer() {
                visited_ids.commit(app, instance_id).await?;
                parent_task.lock().await.complete();
            }
        }
        record.finish();
        drop(admission);
        result
    }

    async fn retain_root_finalizer(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        parent_task: &Arc<Mutex<VisualTask>>,
        graph: &Arc<InstallGraph>,
    ) -> anyhow::Result<()> {
        let mut owner = self.abort_handle.lock().await;
        if owner.handle.is_some() {
            return Ok(());
        }
        let context = self
            .rollback_context
            .lock()
            .await
            .clone()
            .context("Missing root installer rollback ownership")?;
        let graph = graph.clone();
        let task = parent_task.clone();
        let app = app.clone();
        graph.retain_finalizer();
        owner.spawn(move |cancel| async move {
            // This worker owns the root ACK. Only wait for descendants, never
            // for our own ACK or admission guard.
            let result = async {
                context.ownership.wait_children().await?;
                graph.wait_planned().await;
                if cancel.is_cancelled() {
                    bail!("Addon installation cancelled");
                }
                graph.require_complete()?;
                graph.commit(&app, instance_id).await?;
                Ok::<_, anyhow::Error>(())
            }
            .await;
            match result {
                Ok(()) => task.lock().await.complete(),
                Err(error) => {
                    graph.fail(&error);
                    let error = match context.rollback(Some(&error)).await {
                        Ok(()) => error,
                        Err(recovery) => error.context(format!(
                            "Installer graph settlement/rollback failed: {recovery:#}"
                        )),
                    };
                    task.lock().await.clone().fail(error).await;
                }
            }
        });
        Ok(())
    }

    async fn admit_inner(
        &self,
        app: &Arc<AppInner>,
        instance_id: InstanceId,
        instance_path: &InstancePath,
        parent_task: &Arc<Mutex<VisualTask>>,
        visited_ids: &Arc<InstallGraph>,
        install_deps: bool,
        replacement: Option<publication::ReplacementPlan>,
        root: bool,
    ) -> anyhow::Result<()> {
        let processed_deps = Arc::new(Mutex::new(Vec::new()));
        *self.rollback_context.lock().await = Some(InstallerRollbackContext {
            ownership: self.ownership.clone(),
            inner: self.inner.clone(),
            processed_deps: processed_deps.clone(),
            instance_id,
            app: app.clone(),
            receipt: self.receipt.clone(),
            dependency_of: (!root).then(|| visited_ids.overlapped.clone()),
        });
        if install_deps {
            let dep_error = {
                let (installer_name, identity) = {
                    let lock = self.inner.lock().await;
                    (lock.display_name(), lock.id())
                };
                let planned = visited_ids
                    .plan
                    .lock()
                    .await
                    .as_ref()
                    .map(|plan| plan.get(&identity).cloned().unwrap_or_default());
                let dep_iter = if let Some(resources) = planned {
                    planning::getters(resources, instance_path)
                        .await?
                        .collect::<Vec<_>>()
                } else {
                    let lock = self.inner.lock().await;
                    let instance_manager = app.instance_manager();
                    let instances = instance_manager.instances.read().await;
                    let instance = instances
                        .get(&instance_id)
                        .expect("instance should still be valid");
                    let instance_data = instance.data().expect("instance should still be valid");

                    lock.dependencies(app, instance_id, instance_data, ModChannel::Stable)
                        .collect::<Vec<_>>()
                };

                let mut dep_error = None;

                for dep in dep_iter {
                    if self.ownership.cancel.is_cancelled() {
                        dep_error = Some(anyhow::anyhow!("Addon installation cancelled"));
                        break;
                    }
                    let dep_result = dep(visited_ids.selections()).await;
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
                            visited_ids
                                .requirements
                                .lock()
                                .await
                                .push((dep.inner.lock().await.id(), dep.inner.clone()));
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
        #[cfg(test)]
        let download_fixture = self.download_fixture.clone();
        let instance_path = instance_path.clone();
        let inner = Arc::clone(&self.inner);
        let app_clone = Arc::clone(app);
        let rollback_context = Arc::clone(&self.rollback_context);
        let parent_task = Arc::clone(parent_task);
        let graph = visited_ids.clone();
        let graph_index = graph.register(
            self.inner.lock().await.id(),
            self.ownership.cancel.clone(),
            self.receipt.clone(),
        );
        let receipt = self.receipt.clone();

        {
            let mut abort_handle = self.abort_handle.lock().await;
            if abort_handle.aborted || self.ownership.cancel.is_cancelled() {
                bail!("Addon installation cancelled");
            }
            {
                abort_handle.spawn(move |cancel| async move {
                    // This attempt's private folder: its download, and the
                    // replaced version until the update commits. Explicit
                    // cancellation settles download and publication IO before
                    // rollback can inspect it; it is cleared once this worker
                    // is done (see `settle_staging` below).
                    let staging = tempfile::Builder::new()
                        .prefix(publication::STAGING_PREFIX)
                        .tempdir_in(
                            app_clone
                                .settings_manager()
                                .runtime_path
                                .get_root()
                                .to_path(),
                        )
                        .map(tempfile::TempDir::keep);
                    let staging_dir = staging.as_ref().ok().cloned();
                    let r = (|| async {
                        if cancel.is_cancelled() {
                            bail!("Addon installation cancelled");
                        }
                        let staging = staging?;
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

                            let progress = async move {
                                while progress_watch_rx.changed().await.is_ok() {
                                    let progress = progress_watch_rx.borrow_and_update();
                                    t_download_file.update_download(
                                        progress.current_size as u32, progress.total_size as u32, false,
                                    );
                                }
                                t_download_file
                            };
                            let transfer = async {
                                #[cfg(test)]
                                if let Some(source) = &download_fixture {
                                    tokio::fs::create_dir_all(downloadable.path.parent().unwrap()).await?;
                                    tokio::fs::copy(source, &downloadable.path).await?;
                                    drop(progress_watch_tx);
                                    return Ok::<(), anyhow::Error>(());
                                }
                                carbon_net::download_multiple(
                                    &[downloadable.clone()],
                                    DownloadOptions::builder().concurrency(1)
                                        .cancel_token(cancel.clone())
                                        .progress_sender(progress_watch_tx).build(),
                                ).await.map(|_| ()).with_context(|| format!("Failed to download addon file {}", downloadable.path.display()))
                            };
                            // Sender closure ends observation, never certifies transfer success.
                            // Both futures settle before publication or failure handling.
                            let (result, progress) = tokio::join!(transfer, progress);
                            result?;
                            progress.complete_download();

                            {
                                if cancel.is_cancelled() {
                                    bail!("Addon installation cancelled");
                                }
                                let lock = inner.lock().await;
                                publication::verify_acquired(&downloadable.path, downloadable)
                                    .await?;
                                drop(lock);
                                graph.ready(graph_index);
                                graph.publication_turn(graph_index, &cancel).await?;
                                #[cfg(test)]
                                {
                                    let identity = inner.lock().await.id();
                                    app_clone.instance_manager().test_hooks.pause_path("addon_graph_before_member_publish", &identity).await;
                                }
                                #[cfg(test)]
                                if replacement.is_some() { app_clone.instance_manager().test_hooks.pause("addon_graph_before_root_publish", instance_id).await; }
                                let lock = inner.lock().await;
                                if lock.addon_type() == instance::AddonType::Worlds {
                                    // Extracted before the instance gate: Play, addon
                                    // changes and scans of this instance do not wait
                                    // for a large map to unpack.
                                    let current =
                                        publication::current_path(&app_clone, instance_id).await?;
                                    let prepared = publication::prepare_world(
                                        &downloadable.path,
                                        &publication::world_archive_stem(&**lock, &current)?,
                                    )
                                    .await?;
                                    let manager = app_clone.instance_manager();
                                    let _operation = manager.lock_instance(instance_id).await;
                                    manager.ensure_modpack_not_locked(instance_id).await?;
                                    graph.validate_target(&app_clone, instance_id).await?;
                                    let current =
                                        publication::current_path(&app_clone, instance_id).await?;
                                    publication::filename(&**lock, &current)?;
                                    if let Some(plan) = replacement.as_ref() {
                                        let destinations = publication::publish_world_version_owned(
                                            &**lock,
                                            &app_clone,
                                            &_operation,
                                            prepared,
                                            plan,
                                            Some(&receipt),
                                        )
                                        .await?;
                                        let names = destinations
                                            .iter()
                                            .filter_map(|path| path.file_name())
                                            .map(|name| name.to_string_lossy().into_owned())
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        parent_task
                                            .lock()
                                            .await
                                            .edit(|data| {
                                                if let Translation::InstanceTaskInstallWorldCopy {
                                                    world_name,
                                                    ..
                                                } = &mut data.name
                                                {
                                                    *world_name = names;
                                                }
                                            })
                                            .await;
                                    } else {
                                        let saves = publication::checked_folder(&current, instance::AddonType::Worlds).await?;
                                        let reserved = prepared.reserve(&saves).await?;
                                        let names = reserved.names();
                                        let renamed = reserved.renamed();
                                        reserved.finish_owned(current.get_root(), &receipt).await?;
                                        for name in names {
                                            app_clone.meta_cache_manager().refresh_published_addon(&_operation, &saves, name, true, instance::AddonType::Worlds.to_db_string().to_owned()).await?;
                                        }
                                        // A save already had the name: the world went in
                                        // beside it, and the user is told where.
                                        if !renamed.is_empty() {
                                            let task = parent_task.lock().await;
                                            task.set_warning_subject(lock.project_name());
                                            for (name, saved_as) in renamed {
                                                task.warn(domain::vtask::TaskWarning::WorldNameTaken { name, saved_as });
                                            }
                                        }
                                    }
                                } else {
                                    publication::publish_file_owned(
                                        &**lock,
                                        &app_clone,
                                        instance_id,
                                        &downloadable.path,
                                        replacement.as_ref(),
                                        required_enabled.as_ref(),
                                        &receipt,
                                        Some(&graph),
                                    )
                                    .await?;
                                }
                            }
                            // What is left of the download (a world's archive, or a
                            // file already in place) goes now; a replacement backup
                            // stays in staging until graph success.
                            publication::settle_staging(&staging).await;
                        }

                        if downloadable.is_none() { graph.ready(graph_index); }
                        if cancel.is_cancelled() {
                            bail!("Addon installation cancelled");
                        }
                        // Keep the task alive until the instance's rows are current.
                        // Publication already hashed what it wrote, so this pass
                        // trusts unchanged files on their stamps and only holds
                        // the instance gate to snapshot and commit.
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
                        if graph.complete(graph_index) && !graph.has_finalizer() { graph.commit(&app_clone, instance_id).await?; parent_task.lock().await.complete(); }
                        Ok::<_, anyhow::Error>(())
                    })()
                    .await;

                    let failure = match r {
                        Ok(()) => None,
                        Err(e) => {
                            graph.fail(&e);
                            tracing::error!({ error = ?e }, "Error installing dependency");

                            let context = rollback_context.lock().await.clone();
                            let error = if let Some(context) = context {
                                match context.rollback(Some(&e)).await {
                                    Ok(()) => e,
                                    Err(settlement) => e.context(format!("Installer descendant settlement/rollback failed: {settlement:#}")),
                                }
                            } else { e.context("Missing installer rollback ownership") };
                            Some(error)
                        }
                    };
                    // Nothing works in staging any more: a failed or cancelled
                    // download goes, and so does the folder, unless it still
                    // holds a replaced addon (an update that commits later, or
                    // one rollback could not put back, which its error names).
                    if let Some(staging) = &staging_dir {
                        publication::settle_staging(staging).await;
                    }
                    if let Some(error) = failure {
                        if !graph.has_finalizer() {
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

// Publication-time guard for a dependency the plan downloads: resolve physical
// enabled ownership before accepting cached platform identity. Planning never
// calls this for a dependency the instance already has (dependency.rs), so it
// only refuses when a copy appeared or changed while the plan was acquired.
//
// `pin` is the Modrinth version the plan downloads and `planned` its artifact.
// Modrinth's hash lookup links a file to one version, and one jar can belong
// to several versions of a project (libjf, gputape and player-animation-library
// attach the same jar to builds for several Minecraft versions), so a copy
// linked to another version is the planned one when its bytes are.
async fn required_project(
    app: &Arc<AppInner>,
    id: InstanceId,
    cf: Option<i32>,
    mr: Option<&str>,
    pin: Option<&str>,
    planned: Option<&Downloadable>,
) -> anyhow::Result<Option<mfcdb::ModFullRow>> {
    let rows = mfcdb::get_instance_mods_full(&app.db, *id).await?;
    let mut found = None;
    let path = publication::current_path(app, id).await?;
    for row in rows.into_iter().filter(|r| {
        cf.is_some_and(|p| r.cf_project_id == Some(p))
            || mr.is_some_and(|p| r.mr_project_id.as_deref() == Some(p))
    }) {
        let kind = instance::AddonType::from_db_string(&row.addon_type)
            .context("Unknown required addon kind")?;
        let folder = publication::checked_folder(&path, kind).await?;
        let Some((physical, enabled)) = publication::selected_file(&folder, &row.filename).await?
        else {
            continue;
        };
        if !enabled || !row.enabled {
            bail!(
                "Required dependency {} is disabled. Enable it in Addons and retry.",
                row.filename
            );
        }
        if pin.is_some_and(|pin| row.mr_version_id.as_deref() != Some(pin)) {
            let planned_bytes = match planned {
                Some(planned) => publication::matches_download(&physical, planned).await?,
                None => false,
            };
            if !planned_bytes {
                bail!(
                    "Another version of a required dependency, {}, was added to the instance while this installation was running. Install the addon again: the version now installed will be kept.",
                    row.filename
                );
            }
        }
        let proof = Downloadable::new("", physical.clone())
            .with_checksum(Some(Checksum::Sha512(hex::encode(&row.sha512))));
        if !publication::matches_download(&physical, &proof).await? {
            bail!(
                "Required dependency {} has changed bytes; refresh or resolve it before retrying",
                row.filename
            );
        }
        if found.is_some() {
            bail!(
                "Multiple installed artifacts claim the required project; resolve the conflict before retrying"
            );
        }
        found = Some(row);
    }
    Ok(found)
}

async fn validate_required_resource(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    id: InstanceId,
) -> anyhow::Result<()> {
    let identity = resource.id();
    let fields: Vec<_> = identity.split(':').collect();
    let existing = match fields.as_slice() {
        ["curseforge", project, _] => {
            required_project(app, id, Some(project.parse()?), None, None, None).await?
        }
        ["modrinth", project, version] => {
            let path = publication::current_path(app, id).await?;
            let planned = resource.downloadable(&path).await;
            required_project(
                app,
                id,
                None,
                Some(project),
                Some(version),
                planned.as_ref(),
            )
            .await?
        }
        _ => None,
    };
    if let Some(row) = existing {
        if row.addon_type != resource.addon_type().to_db_string()
            || !publication::is_satisfied(resource, app, id).await?
        {
            bail!(
                "Required dependency {} does not match the selected artifact; preserve it and resolve the conflict before retrying",
                row.filename
            );
        }
    }
    let path = publication::current_path(app, id).await?;
    let folder = publication::checked_folder(&path, resource.addon_type()).await?;
    let name = publication::filename(resource, &path)?;
    if let Some((_, enabled)) = publication::selected_file(&folder, &name).await? {
        if !enabled {
            // Planning installs the addon next to it, as next to any disabled
            // copy (see dependency.rs).
            return Err(dependency::DependencyGap::disabled_file(resource, &name));
        }
        if !publication::is_satisfied(resource, app, id).await? {
            bail!("Required dependency destination {name} is occupied by another artifact");
        }
    }
    Ok(())
}

// A downloaded required dependency must satisfy the current game and the
// loaders the instance can run (see dependency::loader_fit). Installed
// dependencies never come through here.
type DependencyTarget = (
    String,
    std::collections::HashSet<domain::instance::info::ModLoader>,
);
fn select_dependency<'a, T>(
    versions: &'a [T],
    target: Option<&DependencyTarget>,
    loader_independent: bool,
    preferred: ModChannel,
    metadata: impl Fn(&T) -> (&Vec<String>, &Vec<String>, ModChannel),
) -> anyhow::Result<&'a T> {
    let Some(target) = target else {
        bail!("Cannot establish required dependency compatibility for an unknown game version");
    };
    let release = &target.0;
    let loaders = dependency::target_loaders(target);
    dependency::best_candidate(versions, preferred, |version| {
        let (games, tags, channel) = metadata(version);
        if !games.contains(release) {
            return None;
        }
        let fit = if loader_independent {
            dependency::LoaderFit::Tagged
        } else {
            dependency::loader_fit(tags, &loaders)
        };
        Some((fit, channel))
    })
    .context("No compatible required dependency for this game version and loader")
}

// curseforge
pub struct CurseforgeModInstaller {
    exact_dependency: bool,
    /// The version an update replaces.
    previous: Option<Arc<dependency::Previous>>,
    project_name: String,
    addon_type: curseforge::ClassId,
    file: curseforge::File,
    download_url: String,
    /// Stand-in for the platform APIs when resolving dependencies.
    #[cfg(test)]
    catalogue: Option<Arc<dyn dependency::Catalogue>>,
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
            exact_dependency: true,
            previous: None,
            project_name: addon.data.name.clone(),
            addon_type: addon.data.class_id.unwrap_or(curseforge::ClassId::Mods),
            file,
            download_url,
            #[cfg(test)]
            catalogue: None,
        })
    }

    pub fn from_file(file: curseforge::File, mod_info: curseforge::Mod) -> anyhow::Result<Self> {
        let download_url = file.download_url.clone().ok_or_else(|| {
            anyhow::anyhow!("mod cannot be downloaded without privileged api key")
        })?;

        Ok(Self {
            exact_dependency: true,
            previous: None,
            project_name: mod_info.name.clone(),
            addon_type: mod_info.class_id.unwrap_or(curseforge::ClassId::Mods),
            file,
            download_url,
            #[cfg(test)]
            catalogue: None,
        })
    }

    fn catalogue(&self, app: &Arc<AppInner>) -> Arc<dyn dependency::Catalogue> {
        #[cfg(test)]
        if let Some(catalogue) = &self.catalogue {
            return catalogue.clone();
        }
        Arc::new(dependency::Live(app.clone()))
    }
}

fn dependency_target(instance_data: &InstanceData) -> Option<DependencyTarget> {
    instance_data
        .config
        .game_configuration
        .version
        .clone()
        .and_then(|version| match version {
            instance::info::GameVersion::Standard(version) => {
                Some((version.release, version.modloaders))
            }
            _ => None,
        })
}

#[async_trait::async_trait]
impl ResourceInstaller for CurseforgeModInstaller {
    fn exact_dependency(&self) -> bool {
        self.exact_dependency
    }
    fn replaces(&mut self, previous: &dependency::ReplacedIds) {
        self.previous = previous
            .curseforge
            .map(|(project, file)| dependency::Previous::curseforge(project, file));
    }
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
        let request = dependency::Request {
            app: app.clone(),
            instance_id,
            target: dependency_target(instance_data),
            preferred: preferred_channel,
            catalogue: self.catalogue(app),
            previous: self.previous.clone(),
            parent_loaders: self.file.game_versions.clone(),
        };
        let mut installers: Vec<ResourceInstallerGetter> = Vec::new();
        for dep in &self.file.dependencies {
            if !matches!(
                dep.relation_type,
                curseforge::FileRelationType::RequiredDependency
            ) {
                continue;
            }
            let request = request.clone();
            let project_id = dep.mod_id;
            installers.push(Box::new(move |selections| {
                Box::pin(async move {
                    dependency::curseforge(&request, project_id, &selections)
                        .await
                        .transpose()
                })
            }));
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

    fn project_name(&self) -> String {
        self.project_name.clone()
    }

    async fn rollback(&self, _instance_data: &mut InstanceData) -> anyhow::Result<()> {
        // Installer publication receipts own rollback; a resource ID is not deletion authority.
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
    exact_dependency: bool,
    /// The version an update replaces.
    previous: Option<Arc<dependency::Previous>>,
    /// Installed instead of the pinned version of a dependant.
    replaced_pin: Option<dependency::ReplacedPin>,
    version: modrinth::version::Version,
    file: modrinth::version::VersionFile,
    mod_info: modrinth::project::Project,
    download_url: String,
    /// Stand-in for the platform APIs when resolving dependencies.
    #[cfg(test)]
    catalogue: Option<Arc<dyn dependency::Catalogue>>,
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
            exact_dependency: true,
            previous: None,
            replaced_pin: None,
            version,
            file,
            mod_info: project,
            download_url,
            #[cfg(test)]
            catalogue: None,
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
            exact_dependency: true,
            previous: None,
            replaced_pin: None,
            version,
            file,
            mod_info: project,
            download_url,
            #[cfg(test)]
            catalogue: None,
        })
    }

    fn catalogue(&self, app: &Arc<AppInner>) -> Arc<dyn dependency::Catalogue> {
        #[cfg(test)]
        if let Some(catalogue) = &self.catalogue {
            return catalogue.clone();
        }
        Arc::new(dependency::Live(app.clone()))
    }
}

#[async_trait::async_trait]
impl ResourceInstaller for ModrinthModInstaller {
    fn exact_dependency(&self) -> bool {
        self.exact_dependency
    }
    fn replaces(&mut self, previous: &dependency::ReplacedIds) {
        self.previous = previous
            .modrinth_version
            .clone()
            .map(dependency::Previous::modrinth);
    }
    fn replaced_pin(&self) -> Option<dependency::ReplacedPin> {
        self.replaced_pin.clone()
    }
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
        let request = dependency::Request {
            app: app.clone(),
            instance_id,
            target: dependency_target(instance_data),
            preferred: preferred_channel,
            catalogue: self.catalogue(app),
            previous: self.previous.clone(),
            parent_loaders: self.version.loaders.clone(),
        };
        let mut installers: Vec<ResourceInstallerGetter> = Vec::new();
        for dep in &self.version.dependencies {
            if dep.dependency_type != modrinth::version::DependencyType::Required {
                continue;
            }
            let request = request.clone();
            let dep = dep.clone();
            installers.push(Box::new(move |selections| {
                Box::pin(async move {
                    dependency::modrinth(&request, &dep, &selections)
                        .await
                        .transpose()
                })
            }));
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

    fn project_name(&self) -> String {
        self.mod_info.title.clone()
    }

    fn version_number(&self) -> String {
        self.version.version_number.clone()
    }

    fn published(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        Some(self.version.date_published)
    }

    async fn rollback(&self, _instance_data: &mut InstanceData) -> anyhow::Result<()> {
        // Installer publication receipts own rollback; a resource ID is not deletion authority.
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
