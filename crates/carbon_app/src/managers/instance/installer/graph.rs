//! Per-attempt planning and semantic publication order, separate from IO settlement.
use super::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

/// The install graphs running on each instance. Two that run at once may
/// each come to rely on a dependency the other published and has not
/// committed yet (the second one plans after the first published it, or finds
/// it in place), so a late failure of one must not remove it.
#[derive(Debug, Default)]
pub(crate) struct LiveGraphs(std::sync::Mutex<HashMap<InstanceId, Vec<LiveGraph>>>);

/// A registered graph: alive while the graph itself is (its admission or a
/// worker still runs), and its overlap flag.
#[derive(Debug)]
struct LiveGraph {
    alive: std::sync::Weak<()>,
    overlapped: Arc<AtomicBool>,
}

impl LiveGraphs {
    /// Registers `graph`, starting on `id`; with another one live there, both
    /// are marked as overlapping.
    fn join(&self, id: InstanceId, graph: &InstallGraph) {
        let mut graphs = self.0.lock().unwrap();
        let live = graphs.entry(id).or_default();
        live.retain(|other| other.alive.strong_count() > 0);
        if !live.is_empty() {
            graph.overlapped.store(true, Ordering::Release);
        }
        for other in live.iter() {
            other.overlapped.store(true, Ordering::Release);
        }
        live.push(LiveGraph {
            alive: Arc::downgrade(&graph.alive),
            overlapped: graph.overlapped.clone(),
        });
    }
}

#[derive(Default)]
struct State {
    ids: HashMap<String, String>,
    ready: Vec<bool>,
    worker_ids: Vec<String>,
    done: Vec<bool>,
    planned: bool,
    finalizer: bool,
    error: Option<String>,
    cancels: Vec<CancellationToken>,
}
#[derive(Default)]
pub(super) struct InstallGraph {
    state: std::sync::Mutex<State>,
    changed: Notify,
    receipts: std::sync::Mutex<Vec<publication::ReceiptSlot>>,
    snapshot: Mutex<
        Option<(
            PathBuf,
            Option<domain::instance::info::GameVersion>,
            Option<domain::instance::info::ModpackInfo>,
        )>,
    >,
    pub plan: Mutex<Option<planning::Plan>>,
    pub requirements: Mutex<Vec<(String, Arc<Mutex<BoxedResourceInstaller>>)>>,
    /// Set once another graph ran on the instance at the same time as this
    /// one (see [`LiveGraphs`]). Its rollback then keeps the dependencies it
    /// published.
    pub overlapped: Arc<AtomicBool>,
    /// Held only here: the graph counts as running while it lives.
    alive: Arc<()>,
}
impl InstallGraph {
    pub fn retain_finalizer(&self) {
        self.state.lock().unwrap().finalizer = true;
    }
    pub fn has_finalizer(&self) -> bool {
        self.state.lock().unwrap().finalizer
    }
    pub fn require_complete(&self) -> anyhow::Result<()> {
        let state = self.state.lock().unwrap();
        if let Some(error) = &state.error {
            bail!("Required dependency graph failed: {error}");
        }
        if !state.planned || !state.done.iter().all(|done| *done) {
            bail!("Required dependency graph did not complete");
        }
        Ok(())
    }
    pub async fn wait_planned(&self) {
        loop {
            let changed = self.changed.notified();
            if self.state.lock().unwrap().planned {
                return;
            }
            changed.await;
        }
    }
    pub fn enter(&self, id: String) -> anyhow::Result<Option<bool>> {
        let mut state = self.state.lock().unwrap();
        // Platform identities are project/version; custom resource IDs are opaque.
        let project = id
            .rsplit_once(':')
            .map_or(id.clone(), |(project, _)| project.to_owned());
        if let Some(existing) = state.ids.get(&project) {
            if existing != &id {
                bail!("Conflicting required versions: {existing} and {id}");
            }
            return Ok(None);
        }
        if state.ids.len() >= 512 {
            bail!("Required dependency graph exceeds 512 unique projects");
        }
        let root = state.ids.is_empty();
        state.ids.insert(project, id);
        Ok(Some(root))
    }
    pub fn register(
        &self,
        identity: String,
        cancel: CancellationToken,
        receipt: publication::ReceiptSlot,
    ) -> usize {
        self.receipts.lock().unwrap().push(receipt);
        let mut state = self.state.lock().unwrap();
        let index = state.ready.len();
        state.ready.push(false);
        state.worker_ids.push(identity);
        state.done.push(false);
        state.cancels.push(cancel);
        index
    }
    pub fn pending(&self, identity: &str) -> bool {
        let state = self.state.lock().unwrap();
        state
            .worker_ids
            .iter()
            .position(|id| id == identity)
            .is_some_and(|index| !state.done[index])
    }
    pub fn selections(&self) -> std::collections::HashMap<String, String> {
        self.state.lock().unwrap().ids.clone()
    }
    pub fn planned(&self) {
        self.state.lock().unwrap().planned = true;
        self.changed.notify_waiters();
    }
    pub fn ready(&self, index: usize) {
        self.state.lock().unwrap().ready[index] = true;
        self.changed.notify_waiters();
    }
    pub fn is_complete(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.planned && state.done.iter().all(|done| *done) && state.error.is_none()
    }
    pub fn complete(&self, index: usize) -> bool {
        let mut state = self.state.lock().unwrap();
        state.done[index] = true;
        self.changed.notify_waiters();
        state.planned && state.done.iter().all(|done| *done) && state.error.is_none()
    }
    pub async fn capture(
        &self,
        app: &Arc<AppInner>,
        id: InstanceId,
        path: &InstancePath,
    ) -> anyhow::Result<()> {
        let mut snapshot = self.snapshot.lock().await;
        if snapshot.is_some() {
            return Ok(());
        }
        let manager = app.instance_manager();
        let instances = manager.instances.read().await;
        let data = instances
            .get(&id)
            .ok_or(InvalidInstanceIdError(id))?
            .data()?;
        *snapshot = Some((
            path.get_root(),
            data.config.game_configuration.version.clone(),
            data.config.modpack.clone(),
        ));
        manager.addon_graphs.join(id, self);
        Ok(())
    }
    pub async fn validate_target(&self, app: &Arc<AppInner>, id: InstanceId) -> anyhow::Result<()> {
        if let Some(error) = self.state.lock().unwrap().error.clone() {
            bail!("Dependency graph failed: {error}");
        }
        let snapshot = self.snapshot.lock().await;
        if let Some((root, version, pack)) = snapshot.as_ref() {
            if publication::current_path(app, id).await?.get_root() != *root {
                bail!("Instance root changed during addon acquisition");
            }
            let manager = app.instance_manager();
            let instances = manager.instances.read().await;
            let data = instances
                .get(&id)
                .ok_or(InvalidInstanceIdError(id))?
                .data()?;
            if !super::super::mods::same_addon_game_version(
                version,
                &data.config.game_configuration.version,
            ) || pack != &data.config.modpack
            {
                bail!("Instance game or pack target changed during addon acquisition");
            }
        }
        Ok(())
    }
    pub async fn commit(&self, app: &Arc<AppInner>, id: InstanceId) -> anyhow::Result<()> {
        let manager = app.instance_manager();
        let _operation = manager.lock_instance(id).await;
        manager.ensure_modpack_not_locked(id).await?;
        self.validate_target(app, id).await?;
        for (_, resource) in self.requirements.lock().await.iter() {
            let resource = resource.lock().await;
            validate_required_resource(&**resource, app, id).await?;
            if !resource.is_already_installed(app, id).await? {
                bail!(
                    "Required dependency {} is not usable at graph completion",
                    resource.display_name()
                );
            }
        }
        let receipts = self.receipts.lock().unwrap().clone();
        // Validate every receipt before releasing ANY recovery authority.
        for slot in &receipts {
            if let Some(receipt) = slot.lock().await.as_ref() {
                receipt.validate_commit().await?;
            }
        }
        let mut committed = Vec::new();
        for slot in receipts {
            if let Some(receipt) = slot.lock().await.take() {
                committed.push(receipt);
            }
        }
        // The verified graph is now committed. Cleanup failure retains coherent
        // installed bytes; it must not partly roll back a committed graph.
        for receipt in committed {
            receipt.commit().await?;
        }
        Ok(())
    }
    pub fn fail(&self, error: &anyhow::Error) {
        let mut state = self.state.lock().unwrap();
        if state.error.is_none() {
            state.error = Some(format!("{error:#}"));
        }
        for cancel in &state.cancels {
            cancel.cancel();
        }
        self.changed.notify_waiters();
    }
    pub async fn publication_turn(
        &self,
        index: usize,
        cancel: &CancellationToken,
    ) -> anyhow::Result<()> {
        loop {
            let changed = self.changed.notified();
            {
                let state = self.state.lock().unwrap();
                if let Some(error) = &state.error {
                    bail!("Required dependency graph failed: {error}");
                }
                if cancel.is_cancelled() {
                    bail!("Addon installation cancelled");
                }
                if state.planned
                    && state.ready.iter().all(|ready| *ready)
                    && state.done[..index].iter().all(|done| *done)
                {
                    return Ok(());
                }
            }
            tokio::select! {
                _ = changed => {},
                _ = cancel.cancelled() => bail!("Addon installation cancelled"),
            }
        }
    }
}
