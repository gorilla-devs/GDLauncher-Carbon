//! Relocation admission owns external writers independently of UI/map state.
//! Lock order: briefly acquire lifecycle mutex before instance/server operation gates;
//! never hold it across awaits. Abandoned external ownership remains a blocker.
use anyhow::{Context, Result, bail};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// How long shutdown waits for in-flight operations other than server runs,
/// concurrently with the server drain. The drain's own worst case (30 s
/// grace, kill, log settle) is about 42 s; handing running games over
/// afterwards adds up to [`FOLLOWER_SETTLE_BUDGET`]. Electron polls for 60 s,
/// and shows the user what it waits for after 3 s.
const OPERATION_SETTLE_BUDGET: std::time::Duration = std::time::Duration::from_secs(15);
/// How long shutdown waits for the session workers of running games, and for
/// the watcher of games from an earlier session, once they are cancelled.
/// They only stop reading logs, bank playtime and hand the game over, which
/// takes about the 2 s log grace. It runs after the drain, so a quit that
/// cannot finish never stops tracking a running game.
const FOLLOWER_SETTLE_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);
/// How many cancelled or replaced shutdown request ids are remembered.
const RETIRED_SHUTDOWNS_KEPT: usize = 32;
#[derive(Debug)]
pub struct RuntimeLifecycle {
    state: Mutex<State>,
    child_records: Option<std::path::PathBuf>,
    process_observer: Arc<dyn ProcessAbsenceObserver>,
}
trait ProcessAbsenceObserver: std::fmt::Debug + Send + Sync {
    fn confirmed_absent(&self, pid: u32, start: u64) -> bool;
}
#[derive(Debug)]
struct RealProcessAbsenceObserver;
impl ProcessAbsenceObserver for RealProcessAbsenceObserver {
    fn confirmed_absent(&self, pid: u32, start: u64) -> bool {
        recorded_process_confirmed_absent(pid, start)
    }
}
impl Default for RuntimeLifecycle {
    fn default() -> Self {
        Self {
            state: Default::default(),
            child_records: None,
            process_observer: Arc::new(RealProcessAbsenceObserver),
        }
    }
}
#[derive(Debug, Default)]
struct State {
    ready: bool,
    failure: Option<String>,
    relocation: Option<(String, bool)>,
    next_id: u64,
    activities: HashMap<u64, ActivityEntry>,
    shutdown: Option<ShutdownStatus>,
    adopted: HashMap<(u32, u64), String>,
    adopted_games: std::collections::HashSet<(u32, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityPolicy {
    CoreWork,
    Server,
    Game,
    TemporaryChild,
    /// Shuts down like CoreWork; a blocked move names it as a download or
    /// install, so producers that set up, repair or import instances use it.
    Installer,
    /// Shuts down like CoreWork; a blocked quit or move names it as a
    /// deletion, which can take a while when files go to the Recycle Bin.
    Deletion,
    Hook,
    JavaProbe,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityPhase {
    Preparing,
    Running,
    Settling,
    Unknown,
}
#[derive(Debug)]
struct ActivityEntry {
    owner: String,
    policy: ActivityPolicy,
    phase: ActivityPhase,
    cancel: tokio_util::sync::CancellationToken,
}
/// Relocation is not a shutdown purpose: it goes through the refuse-first
/// `begin_relocation` lease, which never freezes, cancels or drains anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShutdownPurpose {
    Quit,
    Update,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShutdownPhase {
    Draining,
    Ready,
    Committed,
    Unresolved,
}
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownStatus {
    pub request_id: String,
    pub purpose: ShutdownPurpose,
    pub phase: ShutdownPhase,
    /// While draining, what is still running; once unresolved, what kept it
    /// from finishing.
    pub blockers: Vec<RuntimeBlocker>,
    /// The drain of this or a replaced request asked local servers to stop
    /// ([`ShutdownDrain::servers_asked_to_stop`]). They stay stopped when the
    /// user goes back to the launcher.
    pub servers_stopped: bool,
    /// The session workers of running games were cancelled: their games are
    /// handed over and the request can no longer be cancelled unless it
    /// still ends unresolved.
    #[serde(skip)]
    released: bool,
    /// Cancelled once the request is cancelled or replaced, so a drain still
    /// running for it leaves alone every server it has not asked to stop yet.
    #[serde(skip)]
    cancel: tokio_util::sync::CancellationToken,
}
/// What the server drain of a quit, restart or update knows about its
/// request while it runs ([`RuntimeLifecycle::shutdown_drain`]).
#[derive(Debug, Clone, Default)]
pub struct ShutdownDrain {
    owner: Option<(std::sync::Weak<RuntimeLifecycle>, String)>,
    cancel: tokio_util::sync::CancellationToken,
}
impl ShutdownDrain {
    /// The user kept GDLauncher open, or a later request replaced this one:
    /// servers that were not asked to stop yet are left running.
    pub fn cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
    /// Records that local servers were asked to stop for this request, so
    /// the launcher can say they have to be started again.
    pub fn servers_asked_to_stop(&self) {
        let Some((lifecycle, request)) = &self.owner else {
            return;
        };
        let Some(lifecycle) = lifecycle.upgrade() else {
            return;
        };
        let mut state = lifecycle.state.lock().unwrap();
        if let Some(status) = state
            .shutdown
            .as_mut()
            .filter(|status| status.request_id == *request)
        {
            status.servers_stopped = true;
        }
    }
}

#[derive(Debug)]
pub struct RuntimeActivity {
    lifecycle: Arc<RuntimeLifecycle>,
    id: u64,
    retain_on_drop: bool,
    record: Option<std::path::PathBuf>,
}
impl RuntimeActivity {
    /// Final no-await spawn authorization shares the freeze mutex. The caller
    /// persists intent first; failed authorization must explicitly finish it.
    pub fn spawn<T>(
        &mut self,
        policy: ActivityPolicy,
        spawn: impl FnOnce() -> std::io::Result<T>,
    ) -> std::io::Result<T> {
        let mut state = self.lifecycle.state.lock().unwrap();
        if state.shutdown.is_some() || state.relocation.is_some() {
            return Err(std::io::Error::other(
                "Runtime admission frozen before child spawn",
            ));
        }
        let result = spawn();
        if result.is_ok() {
            let entry = state.activities.get_mut(&self.id).expect("owned activity");
            entry.policy = policy;
            entry.phase = ActivityPhase::Running;
        }
        result
    }
    pub fn follow_spawned_game(&self, child: &RuntimeActivity) -> Result<()> {
        if !Arc::ptr_eq(&self.lifecycle, &child.lifecycle) {
            bail!("Foreign game ownership");
        }
        let mut state = self.lifecycle.state.lock().unwrap();
        let entry = state
            .activities
            .get(&child.id)
            .context("Game ownership disappeared")?;
        if entry.policy != ActivityPolicy::Game || entry.phase != ActivityPhase::Running {
            bail!("Game spawn is not proven");
        }
        let own = state
            .activities
            .get_mut(&self.id)
            .context("Preparation ownership disappeared")?;
        own.policy = ActivityPolicy::Game;
        own.phase = ActivityPhase::Running;
        own.follows = Some(child.id);
        Ok(())
    }
    pub fn cancellation(&self) -> tokio_util::sync::CancellationToken {
        self.lifecycle.state.lock().unwrap().activities[&self.id]
            .cancel
            .clone()
    }
    pub fn settling(&self) {
        if let Some(entry) = self
            .lifecycle
            .state
            .lock()
            .unwrap()
            .activities
            .get_mut(&self.id)
        {
            entry.phase = ActivityPhase::Settling;
        }
    }
    /// Call immediately before spawning an external writer. Drop/cancellation is
    /// no longer completion. Only observed exit (or a proven spawn failure) clears it.
    pub fn external(&mut self) {
        self.retain_on_drop = true;
    }
    /// All external writers retain a durable relocation record. Persist intent
    /// before spawn so a core crash cannot turn an orphan into apparent quiescence.
    pub fn external_child(&mut self) -> Result<()> {
        if let Some(root) = &self.lifecycle.child_records {
            std::fs::create_dir_all(root)?;
            let record = root.join(format!("{}.json", uuid::Uuid::new_v4()));
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&record)?;
            self.record = Some(record);
            file.sync_all()?; // empty record is explicitly Unknown, never completion
            #[cfg(unix)]
            {
                std::fs::File::open(root)?.sync_all()?;
                if let Some(parent) = root.parent() {
                    std::fs::File::open(parent)?.sync_all()?;
                }
            }
        }
        self.external();
        Ok(())
    }
    pub fn record_child_pid(&self, pid: Option<u32>) -> Result<()> {
        let Some(record) = &self.record else {
            return Ok(());
        };
        let Some(pid) = pid else {
            return Ok(());
        };
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
        let start = system.process(Pid::from_u32(pid)).map(|p| p.start_time());
        {
            let start = start.unwrap_or(0);
            let bytes = serde_json::to_vec(&(pid, start))?;
            // A partial record remains Unknown on next startup; never removes authority.
            let mut file = std::fs::OpenOptions::new().write(true).open(record)?;
            std::io::Write::write_all(&mut file, &bytes)?;
            file.sync_all()?;
        }
        Ok(())
    }
    pub fn finish(mut self) {
        self.retain_on_drop = false;
    }
}
impl Drop for RuntimeActivity {
    fn drop(&mut self) {
        if self.retain_on_drop {
            if let Some(entry) = self
                .lifecycle
                .state
                .lock()
                .unwrap()
                .activities
                .get_mut(&self.id)
            {
                entry.phase = ActivityPhase::Unknown;
            }
        } else {
            if let Some(record) = &self.record {
                if let Err(error) = std::fs::remove_file(record) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        tracing::warn!(
                            "Confirmed completion, but ownership record could not be removed: {error}"
                        );
                    }
                }
            }
            self.lifecycle
                .state
                .lock()
                .unwrap()
                .activities
                .remove(&self.id);
        }
    }
}

fn relocation_process_absent(observed_start: Option<u64>, _recorded_start: u64) -> bool {
    // Epoch start times may change after a wall-clock correction (Task22/M23).
    // This is the absence probe only: a present PID is never absence proof.
    // Whether a present PID was reused is `classify_observed`'s decision.
    observed_start.is_none()
}

fn confirmed_absent(pid: u32, observed_start: Option<u64>, recorded_start: u64) -> bool {
    if !relocation_process_absent(observed_start, recorded_start) {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        // A partial/permission-limited sysinfo refresh must not free a live owner.
        if std::fs::metadata("/proc/self").is_err() {
            return false;
        }
        matches!(std::fs::metadata(format!("/proc/{pid}")), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
    }
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
            fn GetLastError() -> u32;
            fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }
        if pid == 0 {
            return false;
        }
        // Read-only process query: access denied means Unknown, never Exited.
        unsafe {
            let handle = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
            if handle.is_null() {
                return GetLastError() == 87;
            } // ERROR_INVALID_PARAMETER: absent PID
            let mut code = 259;
            let queried = GetExitCodeProcess(handle, &mut code);
            CloseHandle(handle);
            queried != 0 && code != 259 // STILL_ACTIVE
        }
    }
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
            fn __error() -> *mut i32;
        }
        if pid == 0 || pid > i32::MAX as u32 {
            return false;
        }
        // Signal zero only queries existence; EPERM and other errors remain Unknown.
        unsafe { kill(pid as i32, 0) == -1 && *__error() == 3 } // ESRCH
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        false
    }
}

pub(crate) fn recorded_process_confirmed_absent(pid: u32, start: u64) -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
    confirmed_absent(
        pid,
        system.process(Pid::from_u32(pid)).map(|p| p.start_time()),
        start,
    )
}

impl RuntimeLifecycle {
    pub fn with_runtime_path(root: &std::path::Path) -> Self {
        Self {
            state: Mutex::new(State::default()),
            child_records: Some(root.join("__gdl_runtime_writers")),
            process_observer: Arc::new(RealProcessAbsenceObserver),
        }
    }
    /// Adopts the durable child records an earlier session left behind and
    /// removes those whose process is confirmed gone. Each record is handled
    /// on its own: one that cannot be read, decoded, verified or removed is
    /// left in place and listed as unresolved, which blocks only relocation.
    /// Only a failure to list the folder itself is an error.
    pub async fn reconcile_children(&self) -> Result<()> {
        let Some(root) = &self.child_records else {
            return Ok(());
        };
        let listing = || format!("Cannot list {}", root.display());
        let mut entries = match tokio::fs::read_dir(root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(anyhow::Error::from(error).context(listing())),
        };
        while let Some(entry) = entries.next_entry().await.with_context(listing)? {
            let path = entry.path();
            if let Err(error) = self.reconcile_child(&path).await {
                self.unresolved_evidence(path, error);
            }
        }
        Ok(())
    }
    async fn reconcile_child(&self, path: &Path) -> Result<()> {
        // Records are regular `*.json` files. Anything else (.DS_Store,
        // desktop.ini, a sync tool's temporary file, a folder) was never
        // written by the launcher and says nothing about a process.
        match tokio::fs::symlink_metadata(path).await {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        if path.extension().is_none_or(|extension| extension != "json") {
            return Ok(());
        }
        if self.state.lock().unwrap().own_records.contains(path) {
            return Ok(());
        }
        // A record removed since the folder was listed is gone.
        let Some(bytes) = read_unless_gone(path).await? else {
            return Ok(());
        };
        while let Some(entry) = entries.next_entry().await? {
            let bytes = tokio::fs::read(entry.path()).await?;
            let (pid, start): (u32, u64) = serde_json::from_slice(&bytes)
                .with_context(|| format!("Unconfirmed external writer ownership at {}; after independently confirming the child has exited, remove this record and restart to retry", entry.path().display()))?;
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
            let observed = system.process(Pid::from_u32(pid)).map(|p| p.start_time());
            if confirmed_absent(pid, observed, start) {
                tokio::fs::remove_file(entry.path()).await?;
            } else {
                self.state
                    .lock()
                    .unwrap()
                    .adopted
                    .insert((pid, start), entry.path().display().to_string());
            }
        }
        Ok(())
    }
    /// Whether `record` names a process that ended or whose PID another
    /// process now holds.
    fn record_gone(&self, record: ProcessRecord, require_java: bool) -> bool {
        self.process_observer.status(record, require_java) == IdentityStatus::GoneOrReused
    }
    /// What the server or game marker `record` names right now: the JVM it
    /// was written for (`Same`), a process that ended or a PID another
    /// process holds (`GoneOrReused`), or something that cannot be told
    /// apart (`Unknown`). Server code asks before every start, delete or
    /// file change, so a marker never outlives its process for long.
    pub(crate) fn marker_status(&self, record: ProcessRecord) -> IdentityStatus {
        self.process_observer.status(record, true)
    }
    /// Forgets a server marker the user resolved from the launcher: its
    /// process was stopped, or the user cleared a marker that could not be
    /// verified. The caller has already removed the marker file. This drops
    /// it from the unreadable list, drops the process it named (`record`,
    /// when it could be read) from the adopted processes, and removes the
    /// copy `reconcile_root` kept of it, so a move of the runtime folder no
    /// longer waits on it and the next start does not adopt it again. A
    /// copy that cannot be removed stays listed and keeps refusing a move.
    pub(crate) fn release_marker(&self, marker: &Path, record: Option<ProcessRecord>) {
        let released: Vec<ProcessRecord> = {
            let mut state = self.state.lock().unwrap();
            state.unresolved.remove(marker);
            let Some(record) = record else {
                return;
            };
            let matched: Vec<_> = state
                .adopted
                .keys()
                .copied()
                .filter(|adopted| same_process(*adopted, record))
                .chain(std::iter::once(record))
                .collect();
            for adopted in &matched {
                state.forget_adopted(adopted);
            }
            matched
        };
        let Some(root) = &self.child_records else {
            return;
        };
        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                tracing::warn!(
                    "Cannot list {} to remove the copy of {}: {error}",
                    root.display(),
                    marker.display()
                );
                return;
            }
        };
        let prefix = format!("reconciled-{}-", released[0].pid);
        for entry in entries.flatten() {
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".json"))
            {
                continue;
            }
            // A copy that cannot be read is left for the startup check.
            let Some(copy) = std::fs::read(&path)
                .ok()
                .and_then(|bytes| decode_child_record(&bytes).ok())
            else {
                continue;
            };
            if !released.iter().any(|record| same_process(copy, *record)) {
                continue;
            }
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    self.state.lock().unwrap().unresolved.remove(&path);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.state.lock().unwrap().unresolved.remove(&path);
                }
                Err(error) => self.unresolved_evidence(path, error.into()),
            }
        }
    }
    /// Keeps a record or marker that cannot be read, decoded, verified or
    /// removed exactly where it is, and lists it so a move of the runtime
    /// folder is refused until `refresh_adopted` finds it resolved. It never
    /// blocks quit, restart or update: those leave earlier-session processes
    /// alone anyway. `path` is always that file, never the folder holding it.
    fn unresolved_evidence(&self, path: PathBuf, error: anyhow::Error) {
        tracing::warn!(
            "Cannot verify the process record {}: {error:#}. It is kept and only blocks moving the runtime folder until it is deleted",
            path.display()
        );
        self.state.lock().unwrap().unresolved.insert(path);
    }
    pub fn admit(self: &Arc<Self>, owner: impl Into<String>) -> Result<RuntimeActivity> {
        let mut state = self.state.lock().unwrap();
        if state.relocation.is_some() || state.shutdown.is_some() {
            bail!("Runtime shutdown or relocation is in progress; retry after resolution");
        }
        state.next_id += 1;
        let id = state.next_id;
        state.activities.insert(
            id,
            ActivityEntry {
                owner: owner.into(),
                policy: ActivityPolicy::CoreWork,
                phase: ActivityPhase::Preparing,
                cancel: tokio_util::sync::CancellationToken::new(),
            },
        );
        Ok(RuntimeActivity {
            lifecycle: self.clone(),
            id,
            retain_on_drop: false,
            record: None,
        })
    }
    /// [`Self::admit`] for work a blocked quit or move names by `policy`
    /// (a deletion, say) rather than as a generic task. Dropping it still
    /// completes it.
    pub fn admit_as(
        self: &Arc<Self>,
        owner: impl Into<String>,
        policy: ActivityPolicy,
    ) -> Result<RuntimeActivity> {
        let activity = self.admit(owner)?;
        self.state
            .lock()
            .unwrap()
            .activities
            .get_mut(&activity.id)
            .unwrap()
            .policy = policy;
        Ok(activity)
    }
    /// Whole-operation producer ownership for Task18. Dropping a producer,
    /// including before spawn, retains Unknown; only explicit finish settles it.
    pub fn admit_owned(
        self: &Arc<Self>,
        owner: impl Into<String>,
        policy: ActivityPolicy,
    ) -> Result<RuntimeActivity> {
        let mut activity = self.admit(owner)?;
        activity.external();
        self.state
            .lock()
            .unwrap()
            .activities
            .get_mut(&activity.id)
            .unwrap()
            .policy = policy;
        Ok(activity)
    }
    /// Ends the startup check. A failure (a runtime folder that cannot be
    /// listed, a failed instance scan or server load) is kept for the session
    /// and refuses only a move of the runtime folder, naming the error.
    pub fn reconciliation_finished(&self, result: Result<()>) {
        let mut state = self.state.lock().unwrap();
        match result {
            Ok(()) if state.failure.is_none() => state.ready = true,
            Ok(()) => {}
            Err(error) => {
                state.ready = false;
                state.failure = Some(format!("{error:#}"));
            }
        }
    }
    /// Refuse-first relocation lease. It checks every writer before it changes
    /// anything, so a refusal leaves games, servers, downloads and admission
    /// exactly as they were. The same request id re-joins its own lease, also
    /// after commit, so a retry after a lost ACK or a slow exit finishes it.
    /// A refused id is retired like a cancelled one: the launcher drops it on
    /// a refusal, so an earlier duplicate of it that the core only gets to
    /// later cannot take a lease once the blockers are gone.
    pub fn begin_relocation(&self, request: &str) -> std::result::Result<(), RelocationRefused> {
        let mut state = self.state.lock().unwrap();
        if state
            .relocation
            .as_ref()
            .is_some_and(|(id, _)| id == request)
        {
            return Ok(());
        }
        // A cancelled id never gets a lease, so a request that reaches the
        // core only after its sender cancelled it cannot freeze admission.
        if state.cancelled_relocations.iter().any(|id| id == request) {
            return Err(RelocationRefused {
                blockers: vec![],
                cancelled: true,
            });
        }
        let blockers = state.relocation_blockers();
        if !blockers.is_empty() {
            state.retire_relocation(request);
            return Err(RelocationRefused {
                blockers,
                cancelled: false,
            });
        }
        state.relocation = Some((request.to_owned(), false));
        Ok(())
    }
    /// Read-only: what would refuse `begin_relocation(request)` right now.
    /// Lets the launcher refuse before its slow folder scan; it changes
    /// nothing, so the lease itself still decides.
    pub fn relocation_blockers(&self, request: &str) -> Vec<RuntimeBlocker> {
        let state = self.state.lock().unwrap();
        if state
            .relocation
            .as_ref()
            .is_some_and(|(id, _)| id == request)
        {
            return vec![];
        }
        state.relocation_blockers()
    }
    /// Starts a quit, restart or update. It never waits on the startup check
    /// or on records from an earlier session: quitting leaves those processes
    /// exactly as they are, so a check that is still running, has failed or
    /// found a record it cannot read keeps only a move of the runtime folder
    /// from starting (`begin_relocation`).
    ///
    /// Freezes admission and cancels this core's work, except running games,
    /// their session workers and the watcher of games from an earlier
    /// session: those are cancelled only once everything else has settled
    /// (`release_followers_or_settle`), so an attempt that ends unresolved or is
    /// cancelled never stops tracking a running game.
    ///
    /// The same request re-joins its own attempt, and retries it once it is
    /// unresolved. A new request of either purpose takes the place of an
    /// unresolved one, whose id is then retired; one that is still draining,
    /// ready or committed is never replaced. Returns whether a drain has to
    /// run.
    pub fn begin_shutdown(&self, request: &str, purpose: ShutdownPurpose) -> Result<bool> {
        let mut state = self.state.lock().unwrap();
        if request.is_empty() {
            bail!("Missing shutdown request id");
        }
        if state.retired_shutdowns.iter().any(|id| id == request) {
            return Err(ShutdownRetired.into());
        }
        let mut servers_stopped = false;
        if let Some(status) = &mut state.shutdown {
            if status.request_id == request {
                if status.purpose != purpose {
                    bail!("Another request owns runtime shutdown");
                }
                if status.phase == ShutdownPhase::Unresolved {
                    status.phase = ShutdownPhase::Draining;
                    status.blockers.clear();
                    return Ok(true);
                }
                return Ok(false);
            }
            if status.phase != ShutdownPhase::Unresolved {
                bail!("Another request owns runtime shutdown");
            }
            // Its drain has ended; the servers it stopped stay stopped.
            servers_stopped = status.servers_stopped;
            status.cancel.cancel();
            let replaced = status.request_id.clone();
            state.retire_shutdown(&replaced);
        }
        // A relocation lease never pins quit, restart or update: it was only
        // granted with nothing running, and admission has stayed frozen since.
        state.shutdown = Some(ShutdownStatus {
            request_id: request.into(),
            purpose,
            phase: ShutdownPhase::Draining,
            blockers: vec![],
            servers_stopped,
            released: false,
            cancel: tokio_util::sync::CancellationToken::new(),
        });
        for entry in state.activities.values() {
            if !entry.running_game() && !entry.session_follower() {
                entry.cancel.cancel();
            }
        }
        Ok(true)
    }
    pub fn prepare_shutdown<F>(
        self: &Arc<Self>,
        request: &str,
        purpose: ShutdownPurpose,
        drain: F,
    ) -> Result<ShutdownStatus>
    where
        F: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        if self.begin_shutdown(request, purpose)? {
            let owned = self.clone();
            let request = request.to_owned();
            tokio::spawn(async move {
                let result = drain.await;
                owned.refresh_adopted();
                let _ = owned.settle_shutdown(&request, result);
            });
        }
        self.shutdown_status(request, purpose)
    }
    pub fn shutdown_status(
        &self,
        request: &str,
        purpose: ShutdownPurpose,
    ) -> Result<ShutdownStatus> {
        self.state
            .lock()
            .unwrap()
            .shutdown
            .as_ref()
            .filter(|s| s.request_id == request && s.purpose == purpose)
            .cloned()
            .context("No matching shutdown request")?;
        if status.phase == ShutdownPhase::Draining {
            let (mut others, followers) = state.shutdown_blockers();
            if status.released {
                others.extend(followers);
            }
            status.blockers = others;
        }
        Ok(status)
    }
    /// Task18 must replace the explicit pending component with its reviewed
    /// producer settlement. A zero child snapshot cannot implement that participant.
    pub fn settle_shutdown(
        &self,
        request: &str,
        server_result: Result<()>,
    ) -> Result<ShutdownStatus> {
        let mut state = self.state.lock().unwrap();
        let status = state
            .shutdown
            .as_ref()
            .filter(|s| s.request_id == request)
            .context("No matching shutdown request")?;
        if status.phase != ShutdownPhase::Draining {
            return Ok(status.clone());
        }
        Ok(Self::settle_locked(&mut state, server_result))
    }
    /// [`Self::settle_shutdown`] for the draining request in `state`.
    ///
    /// A server drain that failed is over: it already asked, waited, killed
    /// and gave up (an exit it could not confirm, a supervisor that went
    /// away, a marker it could not retire). Trying the same request again
    /// only repeats that, so the failure is named as Unconfirmed, which the
    /// launcher does not wait on, rather than as a server still stopping.
    fn settle_locked(state: &mut State, server_result: Result<()>) -> ShutdownStatus {
        let (mut blockers, followers) = state.shutdown_blockers();
        if let Err(error) = server_result {
            blockers.push(RuntimeBlocker::new(
                RuntimeBlockerKind::Unconfirmed,
                format!("{error:#}"),
            ));
        }
        if blockers.is_empty() {
            blockers = followers;
        }
        if purpose != ShutdownPurpose::Relocation {
            blockers.push(
                "Installer shutdown participant pending Task18 (including Java probes and hooks)"
                    .into(),
            );
        }
        let status = state.shutdown.as_mut().unwrap();
        status.phase = if blockers.is_empty() {
            ShutdownPhase::Ready
        } else {
            ShutdownPhase::Unresolved
        };
        status.blockers = blockers;
        status.clone()
    }
    pub fn commit_prepared_shutdown(&self, request: &str, purpose: ShutdownPurpose) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let status = state
            .shutdown
            .as_mut()
            .filter(|s| s.request_id == request && s.purpose == purpose)
            .context("No matching shutdown request")?;
        if !matches!(
            status.phase,
            ShutdownPhase::Ready | ShutdownPhase::Committed
        ) {
            bail!("Shutdown remains unresolved: {:?}", status.blockers);
        }
        status.phase = ShutdownPhase::Committed;
        Ok(())
    }
    /// Called only after the existing instance reconciler verifies and publishes
    /// an adopted game. Recovered generic child records cannot grant this policy.
    pub(crate) fn adopted_game_published(&self, pid: u32, start: u64) {
        self.state
            .lock()
            .unwrap()
            .adopted_games
            .insert((pid, start));
    }
    pub fn shutdown_committed(&self) -> bool {
        let state = self.state.lock().unwrap();
        state
            .shutdown
            .as_ref()
            .is_some_and(|s| s.phase == ShutdownPhase::Committed)
            || state
                .relocation
                .as_ref()
                .is_some_and(|(_, committed)| *committed)
    }

    /// Releases an uncommitted lease, which restores admission. A committed
    /// lease is reported as such so the caller knows the move must go on.
    /// Either way an uncommitted id is remembered as cancelled, so a
    /// prepare-relocation with it that arrives later is refused.
    pub fn cancel_relocation(
        &self,
        request: &str,
    ) -> std::result::Result<(), CancelRelocationError> {
        let mut state = self.state.lock().unwrap();
        if state
            .relocation
            .as_ref()
            .is_some_and(|(id, shutdown)| id == request && !shutdown)
        {
            state.relocation = None;
            Ok(())
        } else {
            bail!("No matching cancellable relocation")
        }
    }
    pub fn commit_shutdown(&self, request: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        match state.relocation.as_mut() {
            Some((id, shutdown)) if id == request => {
                *shutdown = true;
                Ok(())
            }
            _ => bail!("Shutdown requires the matching relocation lease"),
        }
    }
    pub fn relocation_shutdown(&self) -> bool {
        self.state
            .lock()
            .unwrap()
            .relocation
            .as_ref()
            .is_some_and(|(_, shutdown)| *shutdown)
    }
    /// Snapshot process identities outside the admission mutex. Removal only ever
    /// affects captured identities; a concurrent admission still blocks begin_relocation.
    /// Each unresolved file is checked again, so Try again moves once the
    /// user deleted it, once a launch of its instance replaced it and the
    /// game exited, or once it reads again and names a process that is gone.
    /// A game or server record whose PID a program other than Java now holds
    /// is gone. For each record that stays, the program now holding its PID
    /// is remembered, so a refused move names it next to the PID.
    pub fn refresh_adopted(&self) {
        let (captured, unresolved): (Vec<_>, Vec<_>) = {
            let state = self.state.lock().unwrap();
            (
                state
                    .adopted
                    .keys()
                    .map(|record| {
                        let java = state.java_records.contains(record)
                            || state.adopted_games.contains(record);
                        (*record, java)
                    })
                    .collect(),
                state.unresolved.iter().cloned().collect(),
            )
        };
        let inspected: Vec<_> = captured
            .into_iter()
            .map(|(record, java)| (record, self.process_observer.inspect(record, java)))
            .collect();
        let resolved: Vec<_> = unresolved
            .into_iter()
            .filter(|(pid, start)| self.process_observer.confirmed_absent(*pid, *start))
            .collect();
        let mut state = self.state.lock().unwrap();
        for (record, (status, program)) in inspected {
            if status == IdentityStatus::GoneOrReused {
                state.forget_adopted(&record);
                continue;
            }
            // A record released while it was checked stays forgotten, and a
            // program this look did not see is never named.
            match program {
                Some(program) if state.adopted.contains_key(&record) => {
                    state.programs.insert(record, program);
                }
                _ => {
                    state.programs.remove(&record);
                }
            }
        }
        for path in resolved {
            state.unresolved.remove(&path);
        }
    }
    fn persist_reconciled_process(&self, pid: u32, start: u64) -> Result<()> {
        let Some(root) = &self.child_records else {
            return Ok(());
        };
        std::fs::create_dir_all(root)?;
        let record = root.join(format!("reconciled-{pid}-{start}.json"));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(record)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        std::io::Write::write_all(&mut file, &serde_json::to_vec(&(pid, start))?)?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            std::fs::File::open(root)?.sync_all()?;
            if let Some(parent) = root.parent() {
                std::fs::File::open(parent)?.sync_all()?;
            }
        }
        Ok(())
    }

    /// Runs before best-effort UI reconciliation can discard corrupt pidfiles.
    /// Each folder's marker is handled on its own: one that cannot be read or
    /// parsed stays in place and is listed as unresolved, which blocks only
    /// relocation, and the other folders are still reconciled. Only a failure
    /// to list `root` itself is an error.
    pub async fn reconcile_root(&self, root: &Path, marker: &str) -> Result<()> {
        let listing = || format!("Cannot list {}", root.display());
        let mut entries = match tokio::fs::read_dir(root).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(anyhow::Error::from(e).context(listing())),
        };
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let record = super::orphan_pid::read_pid_file(&entry.path(), marker)
                .await
                .with_context(|| format!("Cannot reconcile {}", entry.path().display()))?;
            if let Some((pid, expected)) = record {
                let mut system = System::new();
                system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
                let observed = system.process(Pid::from_u32(pid)).map(|p| p.start_time());
                if !confirmed_absent(pid, observed, expected.unwrap_or(0)) {
                    // Best-effort UI reconciliation can discard legacy/mismatched
                    // pidfiles. Preserve relocation ownership before allowing it.
                    self.persist_reconciled_process(pid, expected.unwrap_or(0))?;
                    self.state.lock().unwrap().adopted.insert(
                        (pid, expected.unwrap_or(0)),
                        format!(
                            "{} (present or unconfirmed process)",
                            entry.path().display()
                        ),
                    );
                }
            }
        }
        Ok(())
    }
}

/// Owns an external command independently of the future invoking it. Cancellation
/// or wait failure leaves an explicit uncertain writer; timeout/kill is not reap.
pub async fn tracked_output(
    lifecycle: &Arc<RuntimeLifecycle>,
    command: &mut tokio::process::Command,
) -> std::io::Result<std::process::Output> {
    let mut activity = lifecycle
        .admit("installer or hook child")
        .map_err(std::io::Error::other)?;
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    activity.external_child().map_err(std::io::Error::other)?;
    let child = match activity.spawn(ActivityPolicy::TemporaryChild, || command.spawn()) {
        Ok(child) => child,
        Err(error) => {
            activity.finish();
            return Err(error);
        }
    };
    activity
        .record_child_pid(child.id())
        .map_err(std::io::Error::other)?;
    let result = child.wait_with_output().await;
    if result.is_ok() {
        activity.finish();
    }
    result
}

#[cfg(test)]
pub(crate) mod tests_support {
    use std::cell::Cell;

    thread_local! {
        static CHILD_RECORD_FAULT: Cell<bool> = const { Cell::new(false) };
    }

    /// Until dropped, writing a started child's identity into its record
    /// fails on this thread, as on a drive that just ran out of space.
    pub(crate) struct ChildRecordsFail(());

    pub(crate) fn fail_child_records() -> ChildRecordsFail {
        CHILD_RECORD_FAULT.with(|fault| fault.set(true));
        ChildRecordsFail(())
    }

    impl Drop for ChildRecordsFail {
        fn drop(&mut self) {
            CHILD_RECORD_FAULT.with(|fault| fault.set(false));
        }
    }

    pub(super) fn child_record_fault() -> std::io::Result<()> {
        if CHILD_RECORD_FAULT.with(Cell::get) {
            return Err(std::io::ErrorKind::StorageFull.into());
        }
        Ok(())
    }

    /// What a Java probe gets from [`super::tracked_output`] when the data
    /// folder refuses its process record.
    pub(crate) fn bookkeeping_error() -> anyhow::Error {
        let error = super::ChildBookkeepingFailed::before_spawn(std::io::Error::from(
            std::io::ErrorKind::StorageFull,
        ));
        anyhow::Error::from(std::io::Error::other(error))
            .context("Cannot execute java checker command")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    #[test]
    fn task07_freeze_cancels_preparation_and_serializes_spawn() {
        let l = ready();
        let mut activity = l.admit("queued server").unwrap();
        let cancel = activity.cancellation();
        assert!(l.begin_shutdown("quit", ShutdownPurpose::Quit).unwrap());
        assert!(cancel.is_cancelled());
        let called = std::cell::Cell::new(false);
        assert!(
            activity
                .spawn(ActivityPolicy::Server, || {
                    called.set(true);
                    Ok(())
                })
                .is_err()
        );
        assert!(!called.get());
        assert!(l.admit("late start").is_err());
        assert!(!l.begin_shutdown("quit", ShutdownPurpose::Quit).unwrap());
        assert!(l.begin_shutdown("quit", ShutdownPurpose::Update).is_err());
        assert!(l.cancel_relocation("stale").is_err());
        drop(activity);
        let result = l.settle_shutdown("quit", Ok(())).unwrap();
        assert_eq!(result.phase, ShutdownPhase::Unresolved);
        assert!(result.blockers.iter().any(|b| b.contains("Task18")));
        assert!(
            l.commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
                .is_err()
        );
    }
    #[test]
    fn task07_game_survival_is_proven_at_spawn_and_finalization_blocks() {
        let l = ready();
        let preparation = l.admit("game preparation").unwrap();
        let mut child = l.admit("game child").unwrap();
        assert!(preparation.follow_spawned_game(&child).is_err());
        child.external();
        child.spawn(ActivityPolicy::Game, || Ok(())).unwrap(); // injected no-process spawn
        preparation.follow_spawned_game(&child).unwrap();
        l.begin_shutdown("quit", ShutdownPurpose::Quit).unwrap();
        assert!(!child.cancellation().is_cancelled());
        // The session worker is cancelled last (release_followers_or_settle), so an
        // attempt that ends here still tracks the game.
        assert!(!preparation.cancellation().is_cancelled());
        let result = l.settle_shutdown("quit", Ok(())).unwrap();
        assert_eq!(result.blockers.len(), 1); // only the explicit missing Task18 participant
        child.finish();
        preparation.settling();
        assert_eq!(
            l.state.lock().unwrap().activities[&preparation.id].phase,
            ActivityPhase::Settling
        );
    }
    #[test]
    fn task07_relocation_commit_retries_and_unknown_retains_evidence() {
        let l = ready();
        let mut child = l.admit("owned server").unwrap();
        child.external();
        drop(child);
        let refused = l.begin_relocation("move").unwrap_err();
        assert_eq!(
            refused.blockers,
            [RuntimeBlocker::new(
                RuntimeBlockerKind::Unconfirmed,
                "owned server"
            )]
        );
        assert!(!l.admission_frozen());
        assert!(l.commit_shutdown("move").is_err());
        let l = ready();
        l.begin_relocation("move").unwrap();
        l.commit_shutdown("move").unwrap();
        // A retry after a lost ACK or a slow exit re-joins the same lease.
        l.begin_relocation("move").unwrap();
        l.commit_shutdown("move").unwrap();
        assert!(l.shutdown_committed());
        assert!(matches!(
            l.cancel_relocation("move"),
            Err(CancelRelocationError::Committed)
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn runtime_lifecycle_missing_snapshot_is_not_absence_when_proc_exists() {
        assert!(!confirmed_absent(std::process::id(), None, 0));
        assert!(confirmed_absent(u32::MAX, None, 0));
    }
    #[test]
    fn runtime_lifecycle_pre_spawn_record_failure_releases_admission() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("__gdl_runtime_writers"), "not a directory").unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let mut activity = lifecycle.admit("test").unwrap();
        assert!(activity.external_child().is_err());
        drop(activity);
        lifecycle.begin_relocation("move").unwrap();
    }

    /// A data folder on a volume that does not sync folders (a VirtualBox
    /// shared folder answers EINVAL, network mounts ENOTSUP or ENOTTY) still
    /// gets process records, so Play, servers and Java checks work there as
    /// they did before records existed. The records folder's own entry in
    /// the data folder is synced only when the folder is new, so a Java
    /// check does not pay for it every time. A real I/O error still refuses
    /// the start.
    #[cfg(unix)]
    #[test]
    fn process_records_work_where_folders_cannot_be_synced() {
        use crate::util::durable::test_hooks::FolderSyncs;
        let root = tempfile::tempdir().unwrap();
        let writers = root.path().join("__gdl_runtime_writers");
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));

        let syncs = FolderSyncs::failing_with(root.path(), libc::EINVAL);
        let mut first = lifecycle.admit("java check").unwrap();
        first.external_child().unwrap();
        first.record_child_pid(Some(std::process::id())).unwrap();
        let record = first.record.clone().unwrap();
        assert_eq!(
            decode_child_record(&std::fs::read(&record).unwrap())
                .unwrap()
                .pid,
            std::process::id()
        );
        let mut second = lifecycle.admit("java check").unwrap();
        second.external_child().unwrap();
        assert_eq!(
            syncs.synced(),
            [writers.clone(), root.path().to_path_buf(), writers.clone()],
            "the data folder is synced once, when the records folder is new"
        );
        drop(syncs);
        first.finish();
        second.finish();
        assert!(!record.exists());

        let _syncs = FolderSyncs::failing_with(root.path(), libc::EIO);
        let mut refused = lifecycle.admit("server").unwrap();
        let error = anyhow::Error::from(refused.external_child().unwrap_err());
        assert!(child_bookkeeping_failed(&error), "{error:#}");
        refused.finish();
        lifecycle.begin_relocation("move").unwrap();
    }

    /// A game or server whose identity cannot be recorded once it started
    /// stays tracked: its activity keeps running with the identity in
    /// memory, nothing reads it as unconfirmed, and its exit settles it as
    /// usual. The empty record stays behind as Unknown evidence in case the
    /// launcher crashes before that, and the log says why it stayed empty.
    #[tracing_test::traced_test]
    #[test]
    fn a_started_program_whose_record_cannot_be_written_stays_tracked() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let mut activity = lifecycle.admit("game 1").unwrap();
        activity.external_child().unwrap();
        activity
            .spawn(ActivityPolicy::Game, || Ok::<_, std::io::Error>(()))
            .unwrap();
        let record = activity.record.clone().unwrap();
        assert!(activity.child_identity().is_none());
        {
            let _fault = tests_support::fail_child_records();
            activity.record_spawned_child(Some(std::process::id()));
        }
        assert!(logs_contain(
            "could not record the started program in the data folder"
        ));
        assert!(logs_contain(
            &std::io::Error::from(std::io::ErrorKind::StorageFull).to_string()
        ));
        assert_eq!(
            activity.child_identity().map(|identity| identity.pid),
            Some(std::process::id())
        );
        assert!(std::fs::read(&record).unwrap().is_empty());
        assert_eq!(
            lifecycle.state.lock().unwrap().activities[&activity.id].phase,
            ActivityPhase::Running
        );
        activity.finish();
        assert!(!record.exists());
        lifecycle.begin_relocation("move").unwrap();
    }

    #[test]
    fn runtime_lifecycle_wall_clock_mismatch_is_not_exit_proof() {
        assert!(
            !relocation_process_absent(Some(1_700_000_100), 1_700_000_000),
            "a still-present process can report a changed epoch start after a clock correction"
        );
        assert!(!relocation_process_absent(Some(0), 1_700_000_000));
        assert!(!relocation_process_absent(
            Some(1_700_000_000),
            1_700_000_000
        ));
        assert!(relocation_process_absent(None, 1_700_000_000));
    }

    #[tokio::test]
    async fn runtime_lifecycle_abandoned_child_record_survives_restart() {
        let root = tempfile::tempdir().unwrap();
        let l = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        let mut child = l.admit("installer").unwrap();
        child.external_child().unwrap();
        let record = child.record.clone().unwrap();
        drop(child);
        let restarted = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        // The empty intent is kept as evidence; it does not fail the check.
        restarted.reconcile_children().await.unwrap();
        restarted.reconciliation_finished(Ok(()));
        assert_eq!(
            restarted.begin_relocation("one").unwrap_err().blockers,
            [RuntimeBlocker::new(
                RuntimeBlockerKind::UnreadableRecord,
                record.display().to_string()
            )]
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("__gdl_runtime_writers"))
                .unwrap()
                .count(),
            1
        );
        assert!(std::fs::read(&record).unwrap().is_empty());
        // Only a move waits on it: quitting leaves that process alone anyway.
        restarted
            .prepare_shutdown_and_commit("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .await
            .unwrap();
        assert!(record.exists());
    }
    #[tokio::test]
    async fn runtime_lifecycle_confirmed_child_clears_durable_ownership() {
        let root = tempfile::tempdir().unwrap();
        let l = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        let mut child = l.admit("installer").unwrap();
        child.external_child().unwrap();
        child.finish();
        let restarted = RuntimeLifecycle::with_runtime_path(root.path());
        restarted.reconciliation_finished(restarted.reconcile_children().await);
        restarted.begin_relocation("one").unwrap();
    }
    #[tokio::test]
    async fn runtime_lifecycle_corrupt_pidfile_blocks_before_best_effort_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let instance = root.path().join("instance");
        std::fs::create_dir(&instance).unwrap();
        std::fs::write(instance.join(".pid"), "malformed").unwrap();
        let l = Arc::new(RuntimeLifecycle::default());
        l.reconcile_root(root.path(), ".pid").await.unwrap();
        l.reconciliation_finished(Ok(()));
        assert_eq!(
            l.begin_relocation("one").unwrap_err().blockers,
            [RuntimeBlocker::new(
                RuntimeBlockerKind::UnreadableRecord,
                instance.join(".pid").display().to_string()
            )]
        );
        assert_eq!(
            std::fs::read_to_string(instance.join(".pid")).unwrap(),
            "malformed"
        );
        l.prepare_shutdown_and_commit("update", ShutdownPurpose::Update, async { Ok(()) })
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(instance.join(".pid")).unwrap(),
            "malformed"
        );
    }

    #[test]
    fn runtime_lifecycle_startup_and_failure_block() {
        let l = RuntimeLifecycle::default();
        assert!(l.begin_relocation("one").is_err());
        l.reconciliation_finished(Err(anyhow::anyhow!("unreadable pidfile")));
        l.reconciliation_finished(Ok(()));
        assert!(l.begin_relocation("one").is_err());
    }
    #[test]
    fn runtime_lifecycle_external_drop_does_not_prove_exit() {
        let l = ready();
        let mut writer = l.admit("installer").unwrap();
        writer.external();
        drop(writer);
        assert!(l.begin_relocation("one").is_err());
    }
    #[test]
    fn runtime_lifecycle_confirmed_completion_and_reentry() {
        let l = ready();
        let mut writer = l.admit("game and post-exit hooks").unwrap();
        writer.external();
        assert!(l.begin_relocation("busy").is_err());
        writer.finish();
        // The launcher asks again under a new id once a move was refused.
        l.begin_relocation("one").unwrap();
        assert!(l.admit("queued preparation").is_err());
        assert!(l.cancel_relocation("stale").is_err());
        l.cancel_relocation("one").unwrap();
        drop(l.admit("another game").unwrap());
        l.begin_relocation("two").unwrap();
        l.commit_shutdown("two").unwrap();
        assert!(l.cancel_relocation("two").is_err());
        assert!(l.admit("automatic server restart").is_err());
    }
    #[test]
    fn runtime_lifecycle_launch_vs_relocation_has_one_winner() {
        let l = ready();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        std::thread::scope(|scope| {
            let launch = scope.spawn(|| {
                barrier.wait();
                l.admit("launch")
            });
            barrier.wait();
            let moved = l.begin_relocation("one").is_ok();
            let launch = launch.join().unwrap();
            assert_ne!(moved, launch.is_ok());
        });
    }
    #[test]
    fn runtime_lifecycle_adopted_remains_independent_of_ui_state() {
        let l = ready();
        l.state
            .lock()
            .unwrap()
            .adopted
            .insert((42, 100), "adopted".into());
        assert!(l.begin_relocation("one").is_err());
        assert!(l.commit_shutdown("one").is_err());
    }
}

#[cfg(test)]
mod task07_request_tests {
    use super::*;
    #[derive(Debug)]
    struct InjectedAbsence(std::sync::atomic::AtomicBool);
    impl ProcessAbsenceObserver for InjectedAbsence {
        fn confirmed_absent(&self, _pid: u32, _start: u64) -> bool {
            self.0.load(std::sync::atomic::Ordering::SeqCst)
        }
    }
    #[derive(Debug)]
    struct InjectedObservation {
        start: Option<u64>,
        authority_absent: bool,
    }
    impl ProcessAbsenceObserver for InjectedObservation {
        fn confirmed_absent(&self, pid: u32, recorded: u64) -> bool {
            assert_eq!((pid, recorded), (42, 10));
            relocation_process_absent(self.start, recorded) && self.authority_absent
        }
    }
    /// A record from an earlier session whose PID is still held (the same
    /// process, a reused PID or an orphaned server JVM) refuses a move every
    /// time, and never a quit or an update: quitting leaves it running either
    /// way, so it is not counted, not signalled and not forgotten.
    #[tokio::test]
    async fn task07r1_r6_live_reused_unknown_block_only_relocation_and_adopted_game_survives_quit()
    {
        for start in [Some(10), Some(99), None] {
            let lifecycle = Arc::new(RuntimeLifecycle {
                process_observer: Arc::new(InjectedObservation {
                    start,
                    authority_absent: false,
                }),
                ..Default::default()
            });
            lifecycle.reconciliation_finished(Ok(()));
            lifecycle
                .state
                .lock()
                .unwrap()
                .adopted
                .insert((42, 10), "recovered owner".into());
            lifecycle.adopted_game_published(42, 10);
            for _ in 0..2 {
                lifecycle
                    .prepare_shutdown("move", ShutdownPurpose::Relocation, async { Ok(()) })
                    .unwrap();
                tokio::task::yield_now().await;
                let status = lifecycle
                    .shutdown_status("move", ShutdownPurpose::Relocation)
                    .unwrap();
                assert_eq!(
                    status.phase,
                    ShutdownPhase::Unresolved,
                    "observation {start:?}"
                );
                assert_eq!(status.blockers, ["recovered owner"]);
                assert!(lifecycle.admit("late writer").is_err());
                assert!(
                    lifecycle
                        .state
                        .lock()
                        .unwrap()
                        .adopted_games
                        .contains(&(42, 10))
                );
            }
        }
        let lifecycle = Arc::new(RuntimeLifecycle {
            process_observer: Arc::new(InjectedObservation {
                start: Some(10),
                authority_absent: false,
            }),
            ..Default::default()
        });
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
            .state
            .lock()
            .unwrap()
            .adopted
            .insert((42, 10), "proven adopted game".into());
        lifecycle.adopted_game_published(42, 10);
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::task::yield_now().await;
        let status = lifecycle
            .shutdown_status("quit", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert_eq!(status.blockers.len(), 1);
        assert!(status.blockers[0].contains("Task18"));
        assert!(
            lifecycle
                .state
                .lock()
                .unwrap()
                .adopted
                .contains_key(&(42, 10))
        );
    }
    #[tokio::test]
    async fn task07r1_r6_relocation_retry_refreshes_confirmed_recovered_absence() {
        let observed = Arc::new(InjectedAbsence(std::sync::atomic::AtomicBool::new(false)));
        let lifecycle = Arc::new(RuntimeLifecycle {
            process_observer: observed.clone(),
            ..Default::default()
        });
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
            .state
            .lock()
            .unwrap()
            .adopted
            .insert((42, 10), "recovered server".into());
        lifecycle
            .prepare_shutdown("move", ShutdownPurpose::Relocation, async { Ok(()) })
            .unwrap();
        tokio::task::yield_now().await;
        assert_eq!(
            refused.blockers,
            [
                RuntimeBlocker::new(RuntimeBlockerKind::PreviousProcess, "recovered server")
                    .with_pid(42)
            ]
        );
        assert!(!lifecycle.admission_frozen());
        observed.0.store(true, std::sync::atomic::Ordering::SeqCst);
        lifecycle.refresh_adopted();
        // Try again is a new attempt with a new id.
        lifecycle
            .begin_relocation("retry")
            .expect("confirmed absence must clear the recovered blocker on the actual retry path");
        lifecycle.commit_shutdown("retry").unwrap();
        assert!(lifecycle.shutdown_committed());
    }
    #[tokio::test]
    async fn task07_request_worker_survives_waiter_loss_and_retry_is_single_owner() {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let (done, wait) = tokio::sync::oneshot::channel();
        let first = lifecycle
            .prepare_shutdown("request", ShutdownPurpose::Quit, async move {
                wait.await?;
                Ok(())
            })
            .unwrap();
        assert_eq!(first.phase, ShutdownPhase::Draining);
        drop(first); // response/HTTP consumer disappears
        let retry = lifecycle
            .prepare_shutdown("request", ShutdownPurpose::Quit, async {
                panic!("retry must not start a second drain")
            })
            .unwrap();
        assert_eq!(retry.phase, ShutdownPhase::Draining);
        assert!(lifecycle.admit("queued start").is_err());
        done.send(()).unwrap();
        tokio::task::yield_now().await;
        assert_eq!(
            lifecycle
                .shutdown_status("request", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            ShutdownPhase::Ready
        );
        lifecycle
            .commit_prepared_shutdown("request", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(
            lifecycle
                .shutdown_status("request", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            ShutdownPhase::Committed
        );
    }
}
