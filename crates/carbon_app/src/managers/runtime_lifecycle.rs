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
    /// A user-configured pre-launch or post-exit command. A quit, restart
    /// or update never cancels it; a blocked quit or move names it as a hook.
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
    settlement: tokio::sync::watch::Sender<Settlement>,
    reservation: Option<Arc<tokio::sync::OwnedSemaphorePermit>>,
    /// The game activity this session worker follows (`follow_spawned_game`).
    follows: Option<u64>,
    /// Watches games adopted from an earlier session (`watch_adopted_games`).
    watches_adopted: bool,
}
impl ActivityEntry {
    /// A game this core started, past its spawn: it outlives the core.
    fn running_game(&self) -> bool {
        self.policy == ActivityPolicy::Game && self.phase == ActivityPhase::Running
    }
    /// The session worker of a game this core started, or the watcher of
    /// games from an earlier session. Cancelling either hands its games over
    /// for good (logs and playtime stop being tracked), so shutdown cancels
    /// them only once nothing else is left and the core is about to exit.
    fn session_follower(&self) -> bool {
        self.follows.is_some() || self.watches_adopted
    }
    /// Whether a quit, restart or update cancels this as it begins. Running
    /// games and their followers are handed over last instead. A
    /// pre-launch or post-exit hook is the user's own script (a backup, a
    /// sync): cancelling it kills it midway, so it is left to finish and the
    /// quit prompt names it, with Quit now as the way out.
    fn cancelled_by_shutdown(&self) -> bool {
        !self.running_game() && !self.session_follower() && self.policy != ActivityPolicy::Hook
    }
    fn blocker(&self) -> RuntimeBlocker {
        use RuntimeBlockerKind as Kind;
        let kind = match (self.phase, self.policy) {
            (ActivityPhase::Unknown, _) => Kind::Unconfirmed,
            (_, ActivityPolicy::Game) => Kind::Game,
            (_, ActivityPolicy::Server) => Kind::Server,
            (_, ActivityPolicy::Installer) => Kind::Install,
            (_, ActivityPolicy::Deletion) => Kind::Deletion,
            (_, ActivityPolicy::Hook) => Kind::Hook,
            (
                _,
                ActivityPolicy::CoreWork
                | ActivityPolicy::TemporaryChild
                | ActivityPolicy::JavaProbe,
            ) => Kind::Task,
        };
        RuntimeBlocker::new(kind, &self.owner)
    }
}

/// One thing that keeps the runtime folder from being moved, or a quit,
/// restart or update from finishing, in a form the launcher can show: a kind
/// it translates and a name. For running work the name is the internal
/// owner, kept for logs; for earlier-session processes it is the folder or
/// record they belong to, and for a failed startup check or server drain
/// its error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBlocker {
    pub kind: RuntimeBlockerKind,
    pub name: String,
    /// The process ID of a process from an earlier session, which is what
    /// Task Manager and Activity Monitor list it by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// The program that holds that PID now, as the process table names it
    /// (`javaw.exe`, `java`), when it could be read. It tells the user
    /// whether the PID is a JVM before they end anything.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeBlockerKind {
    /// Minecraft started in this session is running.
    Game,
    /// A local server is running or stopping.
    Server,
    /// A download or install is in progress: an instance or modpack setup,
    /// repair, import or game preparation, or an addon, server or Java install.
    Install,
    /// An instance or server is being deleted.
    Deletion,
    /// A pre-launch or post-exit hook the user set up is still running. A
    /// quit, restart or update lets it finish rather than cancel it.
    Hook,
    /// Any other launcher work: an export, copy or scan.
    Task,
    /// Work whose end this core could not confirm, also a server drain that
    /// failed (its name is then the error). It clears if the work still
    /// settles (a child reaped after the 5 s budget); otherwise only a
    /// restart does.
    Unconfirmed,
    /// Minecraft started before the launcher last restarted is still running.
    PreviousGame,
    /// A process from an earlier session may still be running.
    PreviousProcess,
    /// A record of a process from an earlier session that cannot be read,
    /// verified or removed; its name is the file. It clears once the file is
    /// gone.
    UnreadableRecord,
    /// The startup check for running games and servers has not finished.
    StartupCheckPending,
    /// The startup check for running games and servers failed.
    StartupCheckFailed,
    /// The launcher is closing, restarting or updating.
    Shutdown,
    /// Another move of the runtime folder holds the lease.
    Relocation,
}
impl RuntimeBlocker {
    fn new(kind: RuntimeBlockerKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            pid: None,
            program: None,
        }
    }
    fn with_pid(self, pid: u32) -> Self {
        Self {
            pid: Some(pid),
            ..self
        }
    }
    fn with_program(self, program: Option<String>) -> Self {
        Self { program, ..self }
    }
}
impl std::fmt::Display for RuntimeBlocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.name)?;
        match (self.pid, &self.program) {
            (Some(pid), Some(program)) => write!(f, " (PID {pid}, {program})")?,
            (Some(pid), None) => write!(f, " (PID {pid})")?,
            _ => {}
        }
        Ok(())
    }
}

/// The core refused to start a relocation. Nothing was frozen, cancelled or
/// stopped: running games, servers and downloads carry on.
#[derive(Debug)]
pub struct RelocationRefused {
    pub blockers: Vec<RuntimeBlocker>,
    /// The launcher already cancelled this request id, or the core already
    /// refused it. Nothing is held for it; a new attempt needs a new id.
    pub cancelled: bool,
}
impl std::fmt::Display for RelocationRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.cancelled {
            return write!(
                f,
                "This move attempt was already cancelled or refused; start a new one"
            );
        }
        write!(
            f,
            "Runtime is busy; close Minecraft, stop servers and let downloads finish, then retry: {}",
            self.blockers
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}
impl std::error::Error for RelocationRefused {}

#[derive(Debug, thiserror::Error)]
pub enum CancelShutdownError {
    /// The core is on its way out; the launcher only has to see it exit.
    #[error("Shutdown is already committed")]
    Committed,
    /// Everything settled and the session workers of running games were
    /// already handed over: the request is about to report Ready.
    #[error("Shutdown is already finishing")]
    Finishing,
    #[error("No matching cancellable shutdown")]
    Missing,
}
/// A prepare-shutdown for an id the launcher cancelled or a later request
/// replaced. Nothing is frozen for it, so the launcher starts over under a
/// new id: its cancel may have landed although the answer was lost.
#[derive(Debug, thiserror::Error)]
#[error("This shutdown request was cancelled or replaced; start a new one")]
pub struct ShutdownRetired;
#[derive(Debug, thiserror::Error)]
pub enum CancelRelocationError {
    /// The lease was committed: the core is on its way out and the move goes on.
    #[error("Relocation is already committed")]
    Committed,
    #[error("No matching cancellable relocation")]
    Missing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Settlement {
    Pending,
    Complete,
    Unknown,
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
    detached_game: bool,
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
            self.owns_child = true;
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
        // The game may survive ordinary shutdown. Its enclosing worker still
        // owns readers, playtime and publication and must acknowledge settlement.
        own.policy = ActivityPolicy::CoreWork;
        own.phase = ActivityPhase::Running;
        own.follows = Some(child.id);
        Ok(())
    }
    /// Marks this activity as the watcher of games adopted from an earlier
    /// session. While any such game runs, it names the watcher's work too.
    pub fn watch_adopted_games(&self) {
        if let Some(entry) = self
            .lifecycle
            .state
            .lock()
            .unwrap()
            .activities
            .get_mut(&self.id)
        {
            entry.watches_adopted = true;
        }
    }
    pub fn unknown(&self, reason: String) {
        if let Some(entry) = self
            .lifecycle
            .state
            .lock()
            .unwrap()
            .activities
            .get_mut(&self.id)
        {
            tracing::error!("{}: {reason}", entry.owner);
            entry.phase = ActivityPhase::Unknown;
            entry.settlement.send_replace(Settlement::Unknown);
        }
    }
    /// Detachment preserves Game/Running and durable recovery evidence. It is
    /// neither child exit nor permission to run exit-only cleanup.
    pub fn detach_surviving_game(mut self) -> Result<()> {
        let state = self.lifecycle.state.lock().unwrap();
        let entry = state
            .activities
            .get(&self.id)
            .context("Game owner missing")?;
        anyhow::ensure!(
            entry.policy == ActivityPolicy::Game && entry.phase == ActivityPhase::Running,
            "Cannot detach unproven game"
        );
        self.detached_game = true;
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
        self.owns_child = true;
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
    /// Settles this activity once the worker that owned it panicked and
    /// unwound. If it never started a child process, or got ready to, the
    /// unwinding dropped everything it was doing, so it completes like a
    /// worker that returned an error: one panic must not leave an
    /// unconfirmed task that holds up every quit and refuses data folder
    /// moves until GDLauncher restarts. One that did start a child, or was
    /// already unconfirmed, stays unconfirmed: that process may still run.
    fn settle_after_panic(self) {
        let (owner, unknown) = {
            let state = self.lifecycle.state.lock().unwrap();
            match state.activities.get(&self.id) {
                Some(entry) => (entry.owner.clone(), entry.phase == ActivityPhase::Unknown),
                None => return,
            }
        };
        if self.owns_child || unknown {
            tracing::error!(
                "{owner} panicked; a program it started may still run, so its end stays unconfirmed"
            );
            // Dropping it unacknowledged keeps it Unknown.
        } else {
            tracing::error!("{owner} panicked; it started no program, so it is settled");
            self.finish();
        }
    }
}
impl Drop for RuntimeActivity {
    fn drop(&mut self) {
        if self.detached_game {
            return;
        }
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
                entry.settlement.send_replace(Settlement::Unknown);
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
            let mut state = self.lifecycle.state.lock().unwrap();
            if let Some(record) = &self.record {
                state.own_records.remove(record);
            }
            if let Some(entry) = state.activities.remove(&self.id) {
                entry.settlement.send_replace(Settlement::Complete);
            }
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
                settlement: tokio::sync::watch::channel(Settlement::Pending).0,
                reservation: None,
                follows: None,
                watches_adopted: false,
            },
        );
        Ok(RuntimeActivity {
            lifecycle: self.clone(),
            id,
            retain_on_drop: false,
            record: None,
            detached_game: false,
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
    /// cancelled never stops tracking a running game. Pre-launch and
    /// post-exit hooks are not cancelled at all: they run to their end (or
    /// their own timeout) and are named as hooks until then.
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
            if entry.cancelled_by_shutdown() {
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
                // Only the drain's own outcome is passed on. The operation wait
                // is a bounded grace period; settle_shutdown re-checks every
                // live activity under the mutex afterwards, so anything still
                // running or Unknown is still named.
                let (server, ()) = tokio::join!(tokio::spawn(drain), owned.wait_for_operations());
                let result = server.map_err(anyhow::Error::from).and_then(|r| r);
                // A request cancelled or replaced meanwhile matches nothing.
                if let Some(followers) = owned.release_followers_or_settle(&request, result) {
                    wait_for_settlement(followers, FOLLOWER_SETTLE_BUDGET).await;
                    let _ = owned.settle_shutdown(&request, Ok(()));
                }
            });
        }
        self.shutdown_status(request, purpose)
    }
    /// Gives in-flight operations up to [`OPERATION_SETTLE_BUDGET`] to settle
    /// while the drain runs. Returns early once every watched activity is
    /// Complete, or as soon as one turns Unknown or loses its sender, since the
    /// final check will report it anyway. Running games may survive and are not
    /// watched, and neither are their session workers or the watcher of games
    /// from an earlier session, which are not cancelled yet. Server runs are
    /// not watched either: the drain stops, kills and finalizes them within
    /// its own bound and fails if an exit stays unconfirmed, and a save can
    /// legitimately take longer than this budget.
    async fn wait_for_operations(&self) {
        let (receivers, budget) = {
            let state = self.state.lock().unwrap();
            #[cfg(test)]
            let budget = state.settle_budget.unwrap_or(OPERATION_SETTLE_BUDGET);
            #[cfg(not(test))]
            let budget = OPERATION_SETTLE_BUDGET;
            let receivers: Vec<_> = state
                .activities
                .values()
                .filter(|entry| {
                    entry.policy != ActivityPolicy::Server
                        && !entry.running_game()
                        && !entry.session_follower()
                })
                .map(|entry| entry.settlement.subscribe())
                .collect();
            (receivers, budget)
        };
        wait_for_settlement(receivers, budget).await;
    }
    /// Ends the drain of `request`. Once the drain succeeded and nothing but
    /// running games, their session workers and the watcher of games from an
    /// earlier session is left of this core's work, it cancels those workers
    /// and the watcher and returns their settlements: the caller waits for
    /// them and then settles. Otherwise it settles right away, under the same
    /// lock as the check, so the attempt ends unresolved with every running
    /// game still tracked. A request cancelled or replaced meanwhile matches
    /// nothing and is left alone.
    fn release_followers_or_settle(
        &self,
        request: &str,
        drain_result: Result<()>,
    ) -> Option<Vec<tokio::sync::watch::Receiver<Settlement>>> {
        let mut state = self.state.lock().unwrap();
        let draining = state
            .shutdown
            .as_ref()
            .is_some_and(|s| s.request_id == request && s.phase == ShutdownPhase::Draining);
        if !draining {
            return None;
        }
        let only_followers_left = state
            .activities
            .values()
            .all(|entry| entry.running_game() || entry.session_follower());
        let has_followers = state
            .activities
            .values()
            .any(|entry| entry.session_follower());
        if drain_result.is_ok() && only_followers_left && has_followers {
            let followers = state
                .activities
                .values()
                .filter(|entry| entry.session_follower())
                .map(|entry| {
                    entry.cancel.cancel();
                    entry.settlement.subscribe()
                })
                .collect();
            state.shutdown.as_mut().unwrap().released = true;
            return Some(followers);
        }
        Self::settle_locked(&mut state, drain_result);
        None
    }
    #[cfg(test)]
    pub(crate) fn set_settle_budget(&self, budget: std::time::Duration) {
        self.state.lock().unwrap().settle_budget = Some(budget);
    }
    /// Lets the launcher go back to normal after a quit, restart or update
    /// that the user chose not to force: admission opens again, and a drain
    /// still running for `request` no longer changes anything: it does not
    /// settle, and it leaves alone every server it had not asked to stop yet
    /// (also one started after this). Work that was cancelled stays
    /// cancelled and servers that were asked to stop stay stopped, but
    /// running games keep being tracked, as their session workers are
    /// cancelled last. The id is retired either way.
    pub fn cancel_shutdown(&self, request: &str) -> std::result::Result<(), CancelShutdownError> {
        let mut state = self.state.lock().unwrap();
        let result = match &state.shutdown {
            Some(status) if status.request_id == request => match status.phase {
                ShutdownPhase::Committed => return Err(CancelShutdownError::Committed),
                ShutdownPhase::Ready => return Err(CancelShutdownError::Finishing),
                ShutdownPhase::Draining if status.released => {
                    return Err(CancelShutdownError::Finishing);
                }
                ShutdownPhase::Draining | ShutdownPhase::Unresolved => {
                    status.cancel.cancel();
                    state.shutdown = None;
                    Ok(())
                }
            },
            _ => Err(CancelShutdownError::Missing),
        };
        state.retire_shutdown(request);
        result
    }
    pub async fn prepare_shutdown_and_commit<F>(
        self: &Arc<Self>,
        request: &str,
        purpose: ShutdownPurpose,
        drain: F,
    ) -> Result<()>
    where
        F: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.prepare_shutdown(request, purpose, drain)?;
        loop {
            let status = self.shutdown_status(request, purpose)?;
            match status.phase {
                ShutdownPhase::Ready | ShutdownPhase::Committed => {
                    return self.commit_prepared_shutdown(request, purpose);
                }
                ShutdownPhase::Unresolved => bail!(
                    "Shutdown unresolved; supervision retained: {:?}",
                    status.blockers
                ),
                ShutdownPhase::Draining => {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await
                }
            }
        }
    }
    /// The handle a server drain for `request` checks and reports through.
    /// A request that is no longer the current one gives a handle that is
    /// already cancelled, so a drain that starts late touches nothing.
    pub fn shutdown_drain(self: &Arc<Self>, request: &str) -> ShutdownDrain {
        let state = self.state.lock().unwrap();
        let cancel = match state.shutdown.as_ref().filter(|s| s.request_id == request) {
            Some(status) => status.cancel.clone(),
            None => {
                let cancel = tokio_util::sync::CancellationToken::new();
                cancel.cancel();
                cancel
            }
        };
        ShutdownDrain {
            owner: Some((Arc::downgrade(self), request.to_owned())),
            cancel,
        }
    }
    /// The request's status. While it drains, its blockers are what is still
    /// running right now, so the launcher can say what it is waiting for.
    pub fn shutdown_status(
        &self,
        request: &str,
        purpose: ShutdownPurpose,
    ) -> Result<ShutdownStatus> {
        let state = self.state.lock().unwrap();
        let mut status = state
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
    /// Final blocker check follows retained operation ACKs and server drain.
    /// Child absence alone never acknowledges a producer or its entered writes.
    /// Only this core's own work counts. Processes adopted from an earlier
    /// session keep running whether the core stays or goes, as games this
    /// core started do, so they never refuse a quit, restart or update.
    /// The session workers of running games are named only once nothing
    /// else is left: until `release_followers_or_settle` cancels them they
    /// are not in the way, they just still have to settle.
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
        // Like settle_shutdown, only this core's own work counts.
        let current_blockers = state.activities.values().any(|entry| !entry.running_game());
        if current_blockers {
            bail!("Runtime ownership changed or remains unsettled before shutdown commit");
        }
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
    pub fn admission_frozen(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.shutdown.is_some() || state.relocation.is_some()
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
        let result = match &state.relocation {
            Some((id, true)) if id == request => return Err(CancelRelocationError::Committed),
            Some((id, false)) if id == request => {
                state.relocation = None;
                Ok(())
            }
            _ => Err(CancelRelocationError::Missing),
        };
        state.retire_relocation(request);
        result
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

/// Borrowed setup cannot be moved into a retained worker. Only a returned future
/// acknowledges its IO/destructors; abandonment conservatively retains Unknown.
/// A panic settles the activity unless it started a child process
/// ([`RuntimeActivity::settle_after_panic`]), then carries on unwinding.
pub async fn run_borrowed_owned<T>(
    activity: RuntimeActivity,
    work: impl std::future::Future<Output = T>,
) -> T {
    let outcome = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(work)).await;
    match outcome {
        Ok(result) => {
            activity.finish();
            result
        }
        Err(panic) => {
            activity.settle_after_panic();
            std::panic::resume_unwind(panic)
        }
    }
}

/// Cancellation is selected only while waiting to enter a gate/permit. Once
/// entered, callers keep ownership until actual writes and cleanup return.
pub async fn wait_or_cancel<T>(
    cancel: &tokio_util::sync::CancellationToken,
    wait: impl std::future::Future<Output = T>,
) -> Result<T> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => bail!("Runtime operation cancelled while queued"),
        result = wait => Ok(result),
    }
}

/// A lookup that writes nothing the user keeps (a platform API call whose
/// only write is the HTTP cache) is dropped as soon as its operation is
/// cancelled, so a stalled network never holds up a quit, restart or update.
/// An already cancelled operation never starts it. Work that writes files
/// stays behind [`run_owned_phase`].
pub async fn lookup_or_cancel<T>(
    cancel: &tokio_util::sync::CancellationToken,
    lookup: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => bail!("Runtime operation cancelled while waiting for a reply"),
        result = lookup => result,
    }
}

/// A lazy next-phase boundary inside an already-owned operation. Once started,
/// this future must settle; cancellation must never drop its entered IO.
pub async fn run_owned_phase<T, F, W>(
    cancel: &tokio_util::sync::CancellationToken,
    start: F,
) -> Result<T>
where
    F: FnOnce() -> W,
    W: std::future::Future<Output = Result<T>>,
{
    anyhow::ensure!(
        !cancel.is_cancelled(),
        "Runtime operation cancelled before next phase"
    );
    start().await
}

/// Retain the SAME admitted operation across requester loss. The factory may
/// transfer its activity slot to the next retained worker; otherwise its normal
/// return (including handled errors) explicitly acknowledges complete owned work.
/// A panic settles an activity still in the slot unless it started a child
/// process ([`RuntimeActivity::settle_after_panic`]); the task still ends in
/// that panic. A raw abort drops an unacknowledged owner and retains Unknown.
pub fn spawn_retained<T, F>(mut activity: RuntimeActivity, work: F) -> tokio::task::JoinHandle<T>
where
    T: Send + 'static,
    F: Send
        + 'static
        + for<'a> FnOnce(
            &'a mut Option<RuntimeActivity>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>,
{
    activity.external();
    tokio::spawn(async move {
        let mut activity = Some(activity);
        // The work, and its borrow of the slot, is dropped by the end of
        // this statement, also when it panicked.
        let outcome =
            futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(work(&mut activity)))
                .await;
        match outcome {
            Ok(result) => {
                if let Some(activity) = activity {
                    activity.finish();
                }
                result
            }
            Err(panic) => {
                if let Some(activity) = activity {
                    activity.settle_after_panic();
                }
                std::panic::resume_unwind(panic)
            }
        }
    })
}

#[derive(Debug, Default, Clone)]
pub struct ChildProgress {
    pub processors: u32,
}

#[derive(Debug, thiserror::Error)]
#[error("Temporary child cleanup is unconfirmed; preserve installation and ownership evidence")]
pub struct ChildSettlementUnknown;

pub fn child_settlement_unknown(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.downcast_ref::<ChildSettlementUnknown>().is_some()
            || cause
                .downcast_ref::<std::io::Error>()
                .and_then(|e| e.get_ref())
                .is_some_and(|e| e.is::<ChildSettlementUnknown>())
    })
}

/// The launcher's own record of a child process could not be kept in the
/// data folder: the ownership record before the spawn, or the started
/// child's identity after it. It says nothing about the program, so a caller
/// must never read it as a verdict on that program (a Java probe that fails
/// this way is inconclusive, not proof of a broken Java).
#[derive(Debug, thiserror::Error)]
#[error(
    "{stage}. Check that the drive holding the GDLauncher data folder is writable and has free space"
)]
pub struct ChildBookkeepingFailed {
    stage: &'static str,
    #[source]
    source: anyhow::Error,
}

impl ChildBookkeepingFailed {
    fn before_spawn(source: impl Into<anyhow::Error>) -> Self {
        Self {
            stage: "GDLauncher could not write its process record to the data folder before starting the program",
            source: source.into(),
        }
    }
    fn after_spawn(source: impl Into<anyhow::Error>) -> Self {
        Self {
            stage: "GDLauncher could not record the started program in the data folder",
            source: source.into(),
        }
    }
}

/// Whether `error` comes from [`ChildBookkeepingFailed`], directly or carried
/// in the `io::Error` that [`tracked_output`] returns.
pub fn child_bookkeeping_failed(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.is::<ChildBookkeepingFailed>()
            || cause
                .downcast_ref::<std::io::Error>()
                .and_then(|e| e.get_ref())
                .is_some_and(|e| e.is::<ChildBookkeepingFailed>())
    })
}

/// Owns an external command independently of the future invoking it. Cancellation
/// or wait failure leaves an explicit uncertain writer; timeout/kill is not reap.
pub async fn tracked_output(
    lifecycle: &Arc<RuntimeLifecycle>,
    command: &mut tokio::process::Command,
) -> std::io::Result<std::process::Output> {
    tracked_output_reserved(lifecycle, command, ActivityPolicy::TemporaryChild, None).await
}

pub async fn tracked_output_reserved(
    lifecycle: &Arc<RuntimeLifecycle>,
    command: &mut tokio::process::Command,
    policy: ActivityPolicy,
    reservation: Option<Arc<tokio::sync::OwnedSemaphorePermit>>,
) -> std::io::Result<std::process::Output> {
    tracked_output_observed(lifecycle, command, policy, reservation, None).await
}

pub async fn tracked_output_observed(
    lifecycle: &Arc<RuntimeLifecycle>,
    command: &mut tokio::process::Command,
    policy: ActivityPolicy,
    reservation: Option<Arc<tokio::sync::OwnedSemaphorePermit>>,
    progress: Option<tokio::sync::watch::Sender<ChildProgress>>,
) -> std::io::Result<std::process::Output> {
    let mut activity = lifecycle
        .admit_owned("temporary child", policy)
        .map_err(std::io::Error::other)?;
    lifecycle
        .state
        .lock()
        .unwrap()
        .activities
        .get_mut(&activity.id)
        .unwrap()
        .reservation = reservation;
    // No caller feeds input. An inherited stdin is the core's own, a pipe
    // the launcher never writes to or closes, so a hook that pauses or asks
    // for input (`pause`, `set /p`, `read`) would wait there until its
    // timeout; an empty one answers at once.
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if let Err(error) = activity.external_child() {
        // No spawn was attempted; cleanup of a partial intent is proven safe.
        activity.finish();
        return Err(std::io::Error::other(error));
    }
    let child = match activity.spawn(policy, || command.spawn()) {
        Ok(child) => child,
        Err(error) => {
            activity.finish();
            return Err(error);
        }
    };
    let record_result = activity
        .record_child_pid(child.id())
        .map_err(std::io::Error::other);
    if record_result.is_err() {
        activity.cancellation().cancel();
    }
    let output =
        temporary_child::collect_observed(activity, temporary_child::NativeChild(child), progress)
            .await;
    // Preserve an unconfirmed-cleanup error so installation owners cannot mistake
    // a record-write failure for permission to delete a still-owned directory.
    let output = output?;
    record_result?;
    Ok(output)
}

mod temporary_child;

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
        assert_eq!(result.phase, ShutdownPhase::Ready);
        assert!(result.blockers.is_empty());
        l.commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
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
        assert_eq!(result.phase, ShutdownPhase::Unresolved);
        assert_eq!(
            result.blockers,
            [RuntimeBlocker::new(
                RuntimeBlockerKind::Task,
                "game preparation"
            )]
        );
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

    /// A process record that cannot be written is the launcher's own
    /// failure. tracked_output says so with a typed error, before it spawns
    /// anything, so callers never read it as a verdict on the program.
    #[tokio::test]
    async fn pre_spawn_record_failure_is_a_typed_bookkeeping_error() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("__gdl_runtime_writers"), "not a directory").unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let ran = root.path().join("ran");
        let mut command = if cfg!(windows) {
            let mut command = tokio::process::Command::new("cmd");
            command
                .arg("/C")
                .arg(format!("type nul > \"{}\"", ran.display()));
            command
        } else {
            let mut command = tokio::process::Command::new("sh");
            command.arg("-c").arg(format!("touch '{}'", ran.display()));
            command
        };
        let error =
            tracked_output_reserved(&lifecycle, &mut command, ActivityPolicy::JavaProbe, None)
                .await
                .unwrap_err();
        let error = anyhow::Error::from(error);
        assert!(child_bookkeeping_failed(&error), "{error:#}");
        assert!(!child_settlement_unknown(&error));
        assert!(format!("{error:#}").contains("data folder"), "{error:#}");
        assert!(!ran.exists(), "nothing is spawned without a record");

        let mut activity = lifecycle.admit("test").unwrap();
        let error = anyhow::Error::from(activity.external_child().unwrap_err());
        assert!(child_bookkeeping_failed(&error), "{error:#}");
        drop(activity);
        lifecycle.begin_relocation("move").unwrap();
        assert!(!child_bookkeeping_failed(&anyhow::Error::from(
            std::io::Error::other("ordinary spawn failure")
        )));
    }

    /// A started child whose identity cannot be recorded is the same
    /// bookkeeping failure.
    #[test]
    fn post_spawn_record_failure_is_a_typed_bookkeeping_error() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let mut activity = lifecycle.admit("test").unwrap();
        activity.external_child().unwrap();
        let record = activity.record.clone().unwrap();
        std::fs::remove_file(&record).unwrap();
        std::fs::create_dir(&record).unwrap();
        let error = activity
            .record_child_pid(Some(std::process::id()))
            .unwrap_err();
        assert!(child_bookkeeping_failed(&anyhow::Error::from(
            std::io::Error::other(error)
        )));
        std::fs::remove_dir(&record).unwrap();
        activity.finish();
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
        assert_eq!(status.phase, ShutdownPhase::Ready);
        assert!(status.blockers.is_empty());
        lifecycle
            .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
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

#[cfg(test)]
mod task18_settlement_tests {
    use super::*;
    #[tokio::test]
    async fn task18_shutdown_waits_for_whole_operation_then_readies() {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let operation = lifecycle
            .admit_owned("paused real writer", ActivityPolicy::Installer)
            .unwrap();
        let cancel = operation.cancellation();
        lifecycle
            .prepare_shutdown("quit18", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::task::yield_now().await;
        assert!(cancel.is_cancelled());
        assert_eq!(
            lifecycle
                .shutdown_status("quit18", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            ShutdownPhase::Draining
        );
        assert!(
            lifecycle
                .commit_prepared_shutdown("quit18", ShutdownPurpose::Quit)
                .is_err()
        );
        operation.finish();
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            lifecycle
                .shutdown_status("quit18", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            ShutdownPhase::Ready
        );
    }
    #[tokio::test]
    async fn task18_running_game_keeps_core_writer_blocking() {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let operation = lifecycle
            .admit_owned("game core writer", ActivityPolicy::CoreWork)
            .unwrap();
        let mut game = lifecycle
            .admit_owned("proven game", ActivityPolicy::Game)
            .unwrap();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        operation.follow_spawned_game(&game).unwrap();
        let cancel = operation.cancellation();
        lifecycle
            .prepare_shutdown("quit18", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        // Nothing else runs, so the worker is cancelled right after the drain.
        tokio::time::timeout(std::time::Duration::from_secs(1), cancel.cancelled())
            .await
            .expect("core logging/finalization needs its own cancellation and settlement");
        assert!(!game.cancellation().is_cancelled());
        assert!(
            lifecycle
                .commit_prepared_shutdown("quit18", ShutdownPurpose::Quit)
                .is_err(),
            "the worker still has to settle"
        );
        operation.finish();
        phase(
            &lifecycle,
            "quit18",
            ShutdownPurpose::Quit,
            ShutdownPhase::Ready,
        )
        .await;
    }
    async fn phase(
        lifecycle: &RuntimeLifecycle,
        request: &str,
        purpose: ShutdownPurpose,
        expected: ShutdownPhase,
    ) {
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if lifecycle
                    .shutdown_status(request, purpose)
                    .is_ok_and(|status| status.phase == expected)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("controlled lifecycle phase");
    }
    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    #[tokio::test]
    async fn task18_retained_handoff_waiter_loss_keeps_same_owner_gate_and_real_writes() {
        let lifecycle = ready();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("session");
        let worker_path = path.clone();
        let gate = Arc::new(tokio::sync::Mutex::new(()));
        let guard = gate.clone().lock_owned().await;
        let activity = lifecycle
            .admit_owned("retained producer", ActivityPolicy::CoreWork)
            .unwrap();
        let identity = activity.id;
        let (arrive, arrived) = tokio::sync::oneshot::channel();
        let (handoff, handed) = tokio::sync::oneshot::channel();
        let (worker_arrive, worker_arrived) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let producer = spawn_retained(activity, move |slot| {
            Box::pin(async move {
                tokio::fs::write(&worker_path, b"Preparing").await.unwrap();
                arrive.send(()).unwrap();
                handed.await.unwrap();
                let worker = spawn_retained(slot.take().unwrap(), move |slot| {
                    Box::pin(async move {
                        assert_eq!(slot.as_ref().unwrap().id, identity);
                        assert!(slot.as_ref().unwrap().cancellation().is_cancelled());
                        worker_arrive.send(()).unwrap();
                        released.await.unwrap();
                        tokio::fs::write(worker_path, b"Committed").await.unwrap();
                    })
                });
                drop(guard); // actual producer gate releases immediately after worker spawn
                drop(worker); // requester loss never aborts the retained owner
            })
        });
        arrived.await.unwrap();
        drop(producer);
        assert!(gate.try_lock().is_err());
        lifecycle
            .prepare_shutdown("handoff", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert!(lifecycle.admit("fresh work").is_err());
        handoff.send(()).unwrap();
        worker_arrived.await.unwrap();
        assert!(gate.try_lock().is_ok());
        assert_eq!(lifecycle.state.lock().unwrap().activities.len(), 1);
        assert!(
            lifecycle
                .state
                .lock()
                .unwrap()
                .activities
                .contains_key(&identity)
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"Preparing");
        assert!(
            lifecycle
                .commit_prepared_shutdown("handoff", ShutdownPurpose::Quit)
                .is_err()
        );
        release.send(()).unwrap();
        phase(
            &lifecycle,
            "handoff",
            ShutdownPurpose::Quit,
            ShutdownPhase::Ready,
        )
        .await;
        assert_eq!(std::fs::read(path).unwrap(), b"Committed");
        assert!(
            lifecycle
                .commit_prepared_shutdown("handoff", ShutdownPurpose::Update)
                .is_err()
        );
        lifecycle
            .commit_prepared_shutdown("handoff", ShutdownPurpose::Quit)
            .unwrap();
    }
    #[tokio::test]
    async fn task18_surviving_game_detaches_only_after_core_io_and_preserves_evidence() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let core = lifecycle
            .admit_owned("game core", ActivityPolicy::CoreWork)
            .unwrap();
        let mut game = lifecycle
            .admit_owned("proven game", ActivityPolicy::Game)
            .unwrap();
        game.external_child().unwrap();
        let record = game.record.clone().unwrap();
        std::fs::write(&record, b"controlled recovery identity").unwrap();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        core.follow_spawned_game(&game).unwrap();
        let game_id = game.id;
        let game_cancel = game.cancellation();
        let output = root.path().join("playtime");
        let worker_output = output.clone();
        let (arrive, arrived) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        drop(spawn_retained(core, move |slot| {
            Box::pin(async move {
                slot.as_ref().unwrap().cancellation().cancelled().await;
                arrive.send(()).unwrap();
                released.await.unwrap();
                tokio::fs::write(worker_output, b"settled playtime and logs")
                    .await
                    .unwrap();
                game.detach_surviving_game().unwrap();
            })
        }));
        lifecycle
            .prepare_shutdown("quit-game", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        arrived.await.unwrap();
        assert!(!game_cancel.is_cancelled());
        assert!(!output.exists());
        assert!(
            lifecycle
                .commit_prepared_shutdown("quit-game", ShutdownPurpose::Quit)
                .is_err()
        );
        release.send(()).unwrap();
        phase(
            &lifecycle,
            "quit-game",
            ShutdownPurpose::Quit,
            ShutdownPhase::Ready,
        )
        .await;
        assert_eq!(std::fs::read(output).unwrap(), b"settled playtime and logs");
        assert_eq!(
            std::fs::read(record).unwrap(),
            b"controlled recovery identity"
        );
        assert_eq!(
            lifecycle.state.lock().unwrap().activities[&game_id].phase,
            ActivityPhase::Running
        );
        lifecycle
            .commit_prepared_shutdown("quit-game", ShutdownPurpose::Quit)
            .unwrap();
        // The surviving game still refuses a move, and the refusal changes nothing.
        let refused = lifecycle.begin_relocation("move-game").unwrap_err();
        assert!(
            refused.blockers.contains(&RuntimeBlocker::new(
                RuntimeBlockerKind::Game,
                "proven game"
            )),
            "{refused}"
        );
        let state = lifecycle.state.lock().unwrap();
        assert!(state.relocation.is_none());
        assert_eq!(
            state.shutdown.as_ref().unwrap().phase,
            ShutdownPhase::Committed
        );
        assert_eq!(state.activities[&game_id].phase, ActivityPhase::Running);
    }
    #[tokio::test]
    async fn task18_unknown_worker_and_late_game_transition_block_commit() {
        let lifecycle = ready();
        let activity = lifecycle
            .admit_owned("lost worker", ActivityPolicy::CoreWork)
            .unwrap();
        let (send, received) = tokio::sync::oneshot::channel();
        let worker = spawn_retained(activity, move |_| {
            Box::pin(async move {
                send.send(()).unwrap();
                std::future::pending::<()>().await;
            })
        });
        received.await.unwrap();
        worker.abort();
        assert!(worker.await.is_err());
        lifecycle
            .prepare_shutdown("lost", ShutdownPurpose::Update, async { Ok(()) })
            .unwrap();
        phase(
            &lifecycle,
            "lost",
            ShutdownPurpose::Update,
            ShutdownPhase::Unresolved,
        )
        .await;
        assert!(
            lifecycle
                .commit_prepared_shutdown("lost", ShutdownPurpose::Update)
                .is_err()
        );
        let lifecycle = ready();
        let mut game = lifecycle.admit_owned("game", ActivityPolicy::Game).unwrap();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        lifecycle
            .prepare_shutdown("late", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        phase(
            &lifecycle,
            "late",
            ShutdownPurpose::Quit,
            ShutdownPhase::Ready,
        )
        .await;
        game.settling(); // natural-exit finalization after the ready snapshot
        assert!(
            lifecycle
                .commit_prepared_shutdown("late", ShutdownPurpose::Quit)
                .is_err()
        );
        game.finish();
        lifecycle
            .commit_prepared_shutdown("late", ShutdownPurpose::Quit)
            .unwrap();
    }
    #[tokio::test]
    async fn task18_signal_commit_waiter_loss_and_failed_drain_never_commit_early() {
        let lifecycle = ready();
        let owner = lifecycle
            .admit_owned("entered write", ActivityPolicy::CoreWork)
            .unwrap();
        let child = lifecycle.clone();
        let waiter = tokio::spawn(async move {
            child
                .prepare_shutdown_and_commit("signal", ShutdownPurpose::Quit, async { Ok(()) })
                .await
        });
        phase(
            &lifecycle,
            "signal",
            ShutdownPurpose::Quit,
            ShutdownPhase::Draining,
        )
        .await;
        waiter.abort();
        let _ = waiter.await;
        owner.finish();
        phase(
            &lifecycle,
            "signal",
            ShutdownPurpose::Quit,
            ShutdownPhase::Ready,
        )
        .await;
        assert!(!lifecycle.shutdown_committed());
        lifecycle
            .prepare_shutdown_and_commit("signal", ShutdownPurpose::Quit, async {
                panic!("repeat drain must not start")
            })
            .await
            .unwrap();
        assert!(lifecycle.shutdown_committed());
        let lifecycle = ready();
        assert!(
            lifecycle
                .prepare_shutdown_and_commit("failure", ShutdownPurpose::Update, async {
                    anyhow::bail!("controlled server drain failure")
                })
                .await
                .is_err()
        );
        assert!(!lifecycle.shutdown_committed());
    }

    #[tokio::test]
    async fn task18_borrowed_setup_normal_error_drop_waits_destructors_and_unknown_blocks() {
        for fail in [false, true] {
            let lifecycle = ready();
            let root = tempfile::tempdir().unwrap();
            let temporary = carbon_rt_path::TempPath::new(root.path().into())
                .maketmpdir()
                .await
                .unwrap();
            let path = temporary.to_path_buf();
            let owner = lifecycle
                .admit_owned("borrowed setup", ActivityPolicy::CoreWork)
                .unwrap();
            let result = run_borrowed_owned(owner, async move {
                let temporary = temporary;
                tokio::fs::write(temporary.join("entered"), b"settled write").await?;
                if fail {
                    tokio::fs::write(&*temporary, b"directory rejects file IO").await?;
                }
                Ok::<_, anyhow::Error>(())
            })
            .await;
            assert_eq!(result.is_err(), fail);
            assert!(
                !path.exists(),
                "synchronous TempEntry destructor must precede ACK"
            );
            lifecycle.begin_relocation("settled").unwrap();
        }
        let lifecycle = ready();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("entered-write");
        let worker_path = path.clone();
        let owner = lifecycle
            .admit_owned("abandoned borrowed setup", ActivityPolicy::CoreWork)
            .unwrap();
        let (arrive, arrived) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let (done, finished) = tokio::sync::oneshot::channel();
        let requester = tokio::spawn(run_borrowed_owned(owner, async move {
            tokio::task::spawn_blocking(move || {
                std::fs::write(&worker_path, b"entered").unwrap();
                arrive.send(()).unwrap();
                released.recv().unwrap();
                std::fs::write(worker_path, b"actually settled").unwrap();
                done.send(()).unwrap();
            })
            .await
            .unwrap();
        }));
        arrived.await.unwrap();
        requester.abort();
        let _ = requester.await;
        lifecycle
            .prepare_shutdown("abandoned", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        phase(
            &lifecycle,
            "abandoned",
            ShutdownPurpose::Quit,
            ShutdownPhase::Unresolved,
        )
        .await;
        assert_eq!(std::fs::read(&path).unwrap(), b"entered");
        release.send(()).unwrap();
        finished.await.unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"actually settled");
        assert!(
            lifecycle
                .commit_prepared_shutdown("abandoned", ShutdownPurpose::Quit)
                .is_err(),
            "lost borrowed ACK cannot be manufactured after the fact"
        );
    }
    #[tokio::test]
    async fn task18_queued_cancellation_does_not_wait_for_unrelated_permit() {
        let lifecycle = ready();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let held = semaphore.clone().acquire_owned().await.unwrap();
        let waiting = semaphore.clone();
        let activity = lifecycle
            .admit_owned("queued preparation", ActivityPolicy::CoreWork)
            .unwrap();
        let task = spawn_retained(activity, move |slot| {
            Box::pin(async move {
                let cancel = slot.as_ref().unwrap().cancellation();
                wait_or_cancel(&cancel, waiting.acquire_owned()).await
            })
        });
        lifecycle
            .prepare_shutdown("queue", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert!(task.await.unwrap().is_err());
        assert_eq!(semaphore.available_permits(), 0);
        phase(
            &lifecycle,
            "queue",
            ShutdownPurpose::Quit,
            ShutdownPhase::Ready,
        )
        .await;
        lifecycle
            .commit_prepared_shutdown("queue", ShutdownPurpose::Quit)
            .unwrap();
        drop(held);
    }
}
