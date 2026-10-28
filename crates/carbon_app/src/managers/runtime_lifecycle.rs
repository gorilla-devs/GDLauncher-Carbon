//! Relocation admission owns external writers independently of UI/map state.
//! Lock order: briefly acquire lifecycle mutex before instance/server operation gates;
//! never hold it across awaits. Abandoned external ownership remains a relocation
//! blocker. Quit, restart and update wait only for this core's own work: records
//! from an earlier session, readable or not, never keep the launcher from closing.
use super::orphan_pid::{self, IdentityStatus, ProcessRecord};
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
    fn observation(&self, _record: ProcessRecord) -> orphan_pid::ProcessObservation {
        orphan_pid::ProcessObservation::Unknown
    }
    /// What `record` names now: the same process, one that ended or whose
    /// PID another process holds, or something that cannot be told apart.
    /// `require_java` is set for game and server markers, whose process is
    /// always a JVM. Fixtures that only model absence keep everything that
    /// is not confirmed absent.
    fn status(&self, record: ProcessRecord, _require_java: bool) -> IdentityStatus {
        if self.confirmed_absent(record.pid, record.start_time.unwrap_or(0)) {
            IdentityStatus::GoneOrReused
        } else {
            IdentityStatus::Unknown
        }
    }
    /// [`Self::status`] with the name of the program that holds the PID
    /// now (`javaw.exe`, `java`), read in the same look at the process
    /// table. Fixtures that do not model program names give none.
    fn inspect(
        &self,
        record: ProcessRecord,
        require_java: bool,
    ) -> (IdentityStatus, Option<String>) {
        (self.status(record, require_java), None)
    }
}
#[derive(Debug)]
struct RealProcessAbsenceObserver;
impl ProcessAbsenceObserver for RealProcessAbsenceObserver {
    fn observation(&self, record: ProcessRecord) -> orphan_pid::ProcessObservation {
        orphan_pid::observe_process(record.pid)
    }
    fn status(&self, record: ProcessRecord, require_java: bool) -> IdentityStatus {
        recorded_identity_status(record, require_java)
    }
    fn inspect(
        &self,
        record: ProcessRecord,
        require_java: bool,
    ) -> (IdentityStatus, Option<String>) {
        recorded_identity(record, require_java)
    }
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
    adopted: HashMap<ProcessRecord, String>,
    adopted_games: std::collections::HashSet<ProcessRecord>,
    /// Adopted records that came from a game or server marker, or from the
    /// copy kept of one. Their process is a JVM, so a PID that a process
    /// other than Java now holds has been reused. Child records of hooks and
    /// installers are not in it: those processes are not Java.
    java_records: std::collections::HashSet<ProcessRecord>,
    /// The program that held each adopted record's PID when
    /// `refresh_adopted` last looked, so a refused move can name it next
    /// to the PID. Missing when it could not be read.
    programs: HashMap<ProcessRecord, String>,
    /// Process records and markers from an earlier session that could not be
    /// read, decoded, verified or removed. Each may describe a process that
    /// still runs, so the file stays where it is and keeps the runtime folder
    /// from being moved until it is gone or names a process that has ended.
    /// Nothing else waits on them.
    unresolved: std::collections::BTreeSet<PathBuf>,
    /// Records `external_child` created in this session. A startup reconcile
    /// running beside a spawn skips them: they belong to this core's own
    /// activities, not to an earlier session.
    own_records: std::collections::HashSet<PathBuf>,
    /// Relocation ids the launcher cancelled or the core refused. The
    /// launcher never reuses either, so a prepare-relocation for one of them
    /// that the core only gets to now (its sender gave up waiting and asked
    /// again) must not grant a lease that nobody owns.
    cancelled_relocations: std::collections::VecDeque<String>,
    /// Shutdown request ids the launcher cancelled, or that a later request
    /// replaced. A prepare-shutdown for one of them that the core only gets
    /// to now must not freeze the core again for a request nobody owns.
    retired_shutdowns: std::collections::VecDeque<String>,
    /// Tests shorten [`OPERATION_SETTLE_BUDGET`] with this.
    #[cfg(test)]
    settle_budget: Option<std::time::Duration>,
}
/// How many cancelled or refused relocation ids are remembered. Each move
/// attempt uses one, so this covers far more retries than a session sees.
const CANCELLED_RELOCATIONS_KEPT: usize = 32;
impl State {
    /// Every writer and every unverified state that keeps the runtime folder
    /// from being moved, in admission order. Empty means the move may start.
    fn relocation_blockers(&self) -> Vec<RuntimeBlocker> {
        use RuntimeBlockerKind as Kind;
        let mut blockers = vec![];
        if !self.ready {
            blockers.push(match &self.failure {
                Some(failure) => RuntimeBlocker::new(Kind::StartupCheckFailed, failure),
                None => RuntimeBlocker::new(Kind::StartupCheckPending, ""),
            });
        }
        if self.shutdown.is_some() {
            blockers.push(RuntimeBlocker::new(Kind::Shutdown, ""));
        }
        if self.relocation.is_some() {
            blockers.push(RuntimeBlocker::new(Kind::Relocation, ""));
        }
        for entry in self.activities_in_order() {
            // A game's session worker is named by the game itself while that
            // game is registered; it is listed on its own once the game is gone.
            if entry
                .follows
                .is_some_and(|game| self.activities.contains_key(&game))
            {
                continue;
            }
            // Likewise the watcher of games from an earlier session is named
            // by those games while any of them is still running.
            if entry.watches_adopted && !self.adopted_games.is_empty() {
                continue;
            }
            blockers.push(entry.blocker());
        }
        let mut adopted: Vec<_> = self.adopted.iter().collect();
        adopted.sort_by(|(a, a_label), (b, b_label)| (a_label, a.pid).cmp(&(b_label, b.pid)));
        for (record, label) in adopted {
            let kind = if self.adopted_games.contains(record) {
                Kind::PreviousGame
            } else {
                Kind::PreviousProcess
            };
            blockers.push(
                RuntimeBlocker::new(kind, label)
                    .with_pid(record.pid)
                    .with_program(self.programs.get(record).cloned()),
            );
        }
        for path in &self.unresolved {
            blockers.push(RuntimeBlocker::new(
                Kind::UnreadableRecord,
                path.display().to_string(),
            ));
        }
        blockers
    }
    fn activities_in_order(&self) -> impl Iterator<Item = &ActivityEntry> {
        let mut activities: Vec<_> = self.activities.iter().collect();
        activities.sort_by_key(|(id, _)| **id);
        activities.into_iter().map(|(_, entry)| entry)
    }
    /// What keeps a quit, restart or update from finishing: this core's own
    /// work, split into the session workers of games and the watcher of
    /// games from an earlier session (`followers`) and everything else.
    /// Running games are never listed: they outlive the core.
    fn shutdown_blockers(&self) -> (Vec<RuntimeBlocker>, Vec<RuntimeBlocker>) {
        let (mut others, mut followers) = (vec![], vec![]);
        for entry in self.activities_in_order() {
            if entry.running_game() {
                continue;
            }
            if entry.session_follower() {
                followers.push(entry.blocker());
            } else {
                others.push(entry.blocker());
            }
        }
        (others, followers)
    }
    /// Records a process from an earlier session under the label a refused
    /// move shows for it. `java` marks a game or server JVM.
    fn adopt(&mut self, record: ProcessRecord, label: String, java: bool) {
        self.adopted.insert(record, label);
        if java {
            self.java_records.insert(record);
        }
    }
    fn forget_adopted(&mut self, record: &ProcessRecord) {
        self.adopted.remove(record);
        self.adopted_games.remove(record);
        self.java_records.remove(record);
        self.programs.remove(record);
    }
    /// Remembers an id that will never hold a lease, dropping the oldest
    /// once the bound is reached.
    fn retire_relocation(&mut self, request: &str) {
        if self.cancelled_relocations.iter().any(|id| id == request) {
            return;
        }
        if self.cancelled_relocations.len() == CANCELLED_RELOCATIONS_KEPT {
            self.cancelled_relocations.pop_front();
        }
        self.cancelled_relocations.push_back(request.to_owned());
    }
    /// Remembers a shutdown request id that must never freeze the core
    /// again, dropping the oldest once the bound is reached.
    fn retire_shutdown(&mut self, request: &str) {
        if self.retired_shutdowns.iter().any(|id| id == request) {
            return;
        }
        if self.retired_shutdowns.len() == RETIRED_SHUTDOWNS_KEPT {
            self.retired_shutdowns.pop_front();
        }
        self.retired_shutdowns.push_back(request.to_owned());
    }
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
    child_identity: Mutex<Option<ProcessRecord>>,
    /// A child process was started under this activity, or is about to be
    /// (`external_child`). It may outlive the code that owns the activity.
    owns_child: bool,
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
    pub fn external_child(&mut self) -> std::result::Result<(), ChildBookkeepingFailed> {
        let lifecycle = self.lifecycle.clone();
        if let Some(root) = &lifecycle.child_records {
            self.persist_intent(root)
                .map_err(ChildBookkeepingFailed::before_spawn)?;
        }
        self.external();
        self.owns_child = true;
        Ok(())
    }
    fn persist_intent(&mut self, root: &Path) -> Result<()> {
        let new_folder = create_records_folder(root)?;
        let record = root.join(format!("{}.json", uuid::Uuid::new_v4()));
        // Claimed before the file exists, so the startup reconcile that
        // runs beside the startup Java scan never reads this still-empty
        // intent as a record from an earlier session.
        self.lifecycle
            .state
            .lock()
            .unwrap()
            .own_records
            .insert(record.clone());
        let file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&record)
        {
            Ok(file) => file,
            Err(error) => {
                self.lifecycle
                    .state
                    .lock()
                    .unwrap()
                    .own_records
                    .remove(&record);
                return Err(error.into());
            }
        };
        self.record = Some(record);
        // The empty record is explicitly Unknown, never completion.
        sync_new_record(&file, root, new_folder)
    }
    /// Writes a started child's identity into its record. Private: a game or
    /// server goes through [`Self::record_spawned_child`], which cannot fail
    /// the start, and a temporary child through [`tracked_output_observed`],
    /// which stops the child when this fails.
    fn record_child_pid(
        &self,
        pid: Option<u32>,
    ) -> std::result::Result<(), ChildBookkeepingFailed> {
        let Some(pid) = pid else {
            return Ok(());
        };
        self.record_child_observed(pid, orphan_pid::observe_process(pid))
            .map_err(ChildBookkeepingFailed::after_spawn)
    }
    fn record_child_observed(
        &self,
        pid: u32,
        observed: orphan_pid::ProcessObservation,
    ) -> Result<()> {
        let identity = orphan_pid::capture_observed(pid, observed);
        *self.child_identity.lock().unwrap() = identity;
        if let (Some(record), Some(identity)) = (&self.record, identity) {
            persist_child_record(record, identity)?;
        }
        // No identity leaves the pre-spawn empty intent Unknown.
        Ok(())
    }
    /// [`Self::record_child_pid`] for a game or server that has started.
    /// A record that cannot be written leaves the empty intent on disk, which
    /// a later start reads as Unknown, as when no identity can be captured.
    /// The program runs on and stays supervised through its live handle and
    /// the identity kept in memory: failing the start here would drop a
    /// running game or server untracked, free to be started a second time.
    pub fn record_spawned_child(&self, pid: Option<u32>) {
        if let Err(error) = self.record_child_pid(pid) {
            // Through anyhow, so the line names the cause (disk full, access
            // denied) and not only what failed.
            tracing::warn!(
                "{:#}; the started program stays tracked",
                anyhow::Error::from(error)
            );
        }
    }
    pub fn child_identity(&self) -> Option<ProcessRecord> {
        *self.child_identity.lock().unwrap()
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

/// Versioned durable records cannot accidentally decode as old epoch tuples.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ChildRecord {
    version: u8,
    process: ProcessRecord,
}
fn encode_child_record(process: ProcessRecord) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&ChildRecord {
        version: 1,
        process,
    })?)
}
fn decode_child_record(bytes: &[u8]) -> Result<ProcessRecord> {
    if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b'[') {
        let (pid, start): (u32, u64) = serde_json::from_slice(bytes)?;
        return Ok(ProcessRecord::legacy(pid, start));
    }
    let record: ChildRecord = serde_json::from_slice(bytes)?;
    anyhow::ensure!(record.version == 1, "Unknown durable child record version");
    record.process.validate_identity()
}
/// The file's bytes, or `None` once it has been removed.
async fn read_unless_gone(path: &Path) -> Result<Option<Vec<u8>>> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn persist_child_record(path: &std::path::Path, identity: ProcessRecord) -> Result<()> {
    #[cfg(test)]
    tests_support::child_record_fault()?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?;
    std::io::Write::write_all(&mut file, &encode_child_record(identity)?)?;
    crate::util::durable::sync_file_plain(&file)?;
    Ok(())
}
/// Creates the folder of durable child records, and says whether it is new.
fn create_records_folder(root: &Path) -> Result<bool> {
    let new_folder = !root.is_dir();
    std::fs::create_dir_all(root)?;
    Ok(new_folder)
}
/// Syncs a record just created in `root`: the file, the folder's entry for
/// it, and the folder's own entry in the data folder when it is new.
///
/// Plain syncs: these records name programs that may outlive the launcher,
/// never the computer. A power cut ends every program they could name, so
/// F_FULLFSYNC's drive-cache flush on Apple platforms would buy nothing and
/// cost every Java check, hook and launch a full flush. Windows and Linux
/// have no lighter flush (see [`crate::util::durable`]), so there a record
/// still gets a full flush of its file, twice per child (the empty record,
/// then the identity), as before. That flush is also what lets a record
/// written just before the computer stopped be read and cleared at the next
/// start, rather than kept as unreadable, which holds up moving the data
/// folder until it is deleted.
/// A volume that does not sync folders (a VirtualBox shared folder, some
/// network mounts) leaves the entries to the system.
fn sync_new_record(file: &std::fs::File, root: &Path, new_folder: bool) -> Result<()> {
    use crate::util::durable;
    durable::sync_file_plain(file)?;
    durable::sync_dir_plain(root)?;
    if new_folder {
        if let Some(parent) = root.parent() {
            durable::sync_dir_plain(parent)?;
        }
    }
    Ok(())
}
/// What a process record from an earlier session names right now.
///
/// `GoneOrReused` means the process ended, or its PID belongs to another
/// process: one that is not Java when `require_java` (game and server
/// markers), one with a different boot/ticks identity on Linux, or one with
/// a different creation time on Windows and macOS. `Same` and `Unknown`
/// keep the record. On Linux a legacy record without boot/ticks identity
/// whose epoch start differs from a live Java process stays `Unknown`: a
/// wall-clock step moves that epoch (M23).
pub(crate) fn recorded_identity_status(
    record: ProcessRecord,
    require_java: bool,
) -> IdentityStatus {
    recorded_identity(record, require_java).0
}
/// [`recorded_identity_status`] with the name of the program holding the
/// PID, from the same observation.
fn recorded_identity(
    record: ProcessRecord,
    require_java: bool,
) -> (IdentityStatus, Option<String>) {
    let (observed, program) = orphan_pid::observe_process_named(record.pid);
    let status = classify_observed(
        record,
        observed,
        require_java,
        orphan_pid::EPOCH_START_TIMES_ARE_STABLE,
        || recorded_process_confirmed_absent(record.pid, record.start_time.unwrap_or(0)),
    );
    (status, program)
}
/// [`recorded_identity_status`] for a given observation and platform rule.
/// A process sysinfo did not list counts as ended only when the native
/// probe (`/proc`, `kill(pid, 0)`, `OpenProcess`) confirms no process holds
/// the PID: a partial or permission-limited listing is not absence.
fn classify_observed(
    record: ProcessRecord,
    observed: orphan_pid::ProcessObservation,
    require_java: bool,
    epoch_is_stable: bool,
    confirmed_absent: impl FnOnce() -> bool,
) -> IdentityStatus {
    match orphan_pid::identity_status_with(record, observed, require_java, epoch_is_stable) {
        IdentityStatus::GoneOrReused
            if observed == orphan_pid::ProcessObservation::Absent && !confirmed_absent() =>
        {
            IdentityStatus::Unknown
        }
        status => status,
    }
}
/// Copies `reconcile_root` keeps of game and server markers. Their process
/// is a JVM, unlike other child records (hooks, installers, Java probes).
fn is_marker_copy(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("reconciled-"))
}
/// Whether two records describe the same process, also when one of them
/// was upgraded with the Linux boot/ticks identity verified for it.
fn same_process(a: ProcessRecord, b: ProcessRecord) -> bool {
    a == b
        || (a.pid == b.pid
            && (a.identity.is_none() || b.identity.is_none())
            && a.start_time.is_some()
            && a.start_time == b.start_time)
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
        let record = decode_child_record(&bytes)?;
        let require_java = is_marker_copy(path);
        if self.record_gone(record, require_java) {
            if let Some(current) = read_unless_gone(path).await? {
                anyhow::ensure!(
                    decode_child_record(&current)? == record,
                    "Durable child identity changed; ownership retained"
                );
                match tokio::fs::remove_file(path).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let mut state = self.state.lock().unwrap();
            state.forget_adopted(&record);
            return Ok(());
        }
        self.state
            .lock()
            .unwrap()
            .adopt(record, path.display().to_string(), require_java);
        if let Some(upgraded) = orphan_pid::verified_record(
            record,
            self.process_observer.observation(record),
            require_java,
        ) {
            if upgraded != record {
                // The process is adopted either way. A failed upgrade may
                // have left the file truncated, so it is listed as well.
                persist_child_record(path, upgraded)?;
                let mut state = self.state.lock().unwrap();
                state.forget_adopted(&record);
                state.adopt(upgraded, path.display().to_string(), require_java);
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
            child_identity: Mutex::new(None),
            owns_child: false,
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
    pub(crate) fn adopted_record_published(&self, record: ProcessRecord) {
        let mut state = self.state.lock().unwrap();
        state
            .adopted
            .entry(record)
            // The blocker carries the PID; a folder label replaces this.
            .or_default();
        state.adopted_games.insert(record);
    }
    pub(crate) fn adopted_game_published(&self, pid: u32, start: u64) {
        self.adopted_record_published(ProcessRecord::legacy(pid, start));
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
            .filter(|path| self.evidence_resolved(path))
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
    /// Whether an unresolved file no longer stands for a process that may
    /// still run. It changes nothing on disk: the next start's check still
    /// removes what is stale. A file that now reads as a process which may
    /// be running stays listed; it can be a game this session started, as a
    /// launch rewrites its instance's marker, and it clears once that ends.
    fn evidence_resolved(&self, path: &Path) -> bool {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            // A file under a folder that is gone, or that is now a plain
            // file, cannot exist either.
            Err(error) => {
                return matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                );
            }
        };
        let child_record = self
            .child_records
            .as_deref()
            .is_some_and(|root| path.parent() == Some(root));
        let record = if child_record {
            decode_child_record(&bytes).ok()
        } else {
            std::str::from_utf8(&bytes)
                .ok()
                .and_then(|text| orphan_pid::parse_process_file(text).ok())
        };
        // Markers and their copies name a JVM; other child records do not.
        let java = !child_record || is_marker_copy(path);
        record.is_some_and(|record| self.record_gone(record, java))
    }
    /// Where `persist_reconciled_process` keeps a recovered process so a
    /// restart still knows about it after its marker is cleaned up.
    fn reconciled_record_path(&self, identity: ProcessRecord) -> Option<PathBuf> {
        let root = self.child_records.as_ref()?;
        let token = match identity.identity {
            Some(orphan_pid::ProcessIdentity::LinuxBootTicks {
                boot_id,
                start_ticks,
            }) => format!("linux-{boot_id}-{start_ticks}"),
            None => format!(
                "epoch-{}",
                identity
                    .start_time
                    .map(|start| start.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ),
        };
        Some(root.join(format!("reconciled-{}-{token}.json", identity.pid)))
    }
    fn persist_reconciled_process(&self, identity: ProcessRecord) -> Result<()> {
        let (Some(root), Some(record)) = (
            self.child_records.as_ref(),
            self.reconciled_record_path(identity),
        ) else {
            return Ok(());
        };
        let new_folder = create_records_folder(root)?;
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&record)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = decode_child_record(&std::fs::read(&record)?)?;
                anyhow::ensure!(
                    existing == identity,
                    "Reconciled child identity changed; ownership retained"
                );
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        std::io::Write::write_all(&mut file, &encode_child_record(identity)?)?;
        sync_new_record(&file, root, new_folder)
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
        while let Some(entry) = entries.next_entry().await.with_context(listing)? {
            let file_type = entry.file_type().await;
            self.reconcile_folder(&entry.path(), file_type, marker)
                .await;
        }
        Ok(())
    }
    /// One entry of `reconcile_root`, given the type its listing reported.
    /// Only the marker file is ever listed as unresolved, never the instance
    /// or server folder: the move refusal tells the user to delete what it
    /// lists, and that folder holds their worlds.
    async fn reconcile_folder(
        &self,
        folder: &Path,
        file_type: std::io::Result<std::fs::FileType>,
        marker: &str,
    ) {
        match file_type {
            Ok(file_type) if !file_type.is_dir() => return,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            // Some network and FUSE file systems leave the type out of the
            // listing, and the stat that stands in for it can fail. Reading
            // the marker decides instead.
            Err(error) => tracing::debug!(
                "Cannot tell whether {} is a folder ({error}); checking for its process marker",
                folder.display()
            ),
        }
        // A marker removed since the folder was listed reads as none, and so
        // does one under an entry that turns out not to be a folder.
        let record = match orphan_pid::read_process_file(folder, marker).await {
            Ok(Some(record)) => record,
            Ok(None) => return,
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotADirectory) =>
            {
                return;
            }
            Err(error) => {
                self.unresolved_evidence(orphan_pid::pid_file_path(folder, marker), error);
                return;
            }
        };
        // A game or server marker names a JVM: a PID held by anything else,
        // or by a process with another identity, was reused.
        if self.record_gone(record, true) {
            return;
        }
        let record =
            orphan_pid::verified_record(record, self.process_observer.observation(record), true)
                .unwrap_or(record);
        // The folder is what the launcher shows the user.
        self.state
            .lock()
            .unwrap()
            .adopt(record, folder.display().to_string(), true);
        if let Err(error) = self.persist_reconciled_process(record) {
            // This session still refuses a move on the process itself.
            // The copy that carries it across a restart is what is unsure.
            if let Some(path) = self.reconciled_record_path(record) {
                self.unresolved_evidence(path, error);
            }
        }
    }
}

/// Waits up to `budget` for every settlement to be Complete. Returns early
/// as soon as one turns Unknown or loses its sender: the final check after
/// it names whatever remains, so this only bounds the grace period.
async fn wait_for_settlement(
    receivers: Vec<tokio::sync::watch::Receiver<Settlement>>,
    budget: std::time::Duration,
) {
    let wait =
        futures::future::try_join_all(receivers.into_iter().map(|mut receiver| async move {
            loop {
                match *receiver.borrow_and_update() {
                    Settlement::Complete => return Ok(()),
                    Settlement::Unknown => bail!("Runtime operation settlement is unknown"),
                    Settlement::Pending => {}
                }
                receiver
                    .changed()
                    .await
                    .context("Runtime operation lost its terminal acknowledgement")?;
            }
        }));
    match tokio::time::timeout(budget, wait).await {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            tracing::warn!("Shutdown found an unsettled runtime operation: {error:#}")
        }
        Err(_) => tracing::warn!(
            "Runtime operations were still active {budget:?} into shutdown; the final check names any that remain"
        ),
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
            .insert(ProcessRecord::legacy(42, 100), "adopted".into());
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
            for (published, kind) in [
                (false, RuntimeBlockerKind::PreviousProcess),
                (true, RuntimeBlockerKind::PreviousGame),
            ] {
                for purpose in [ShutdownPurpose::Quit, ShutdownPurpose::Update] {
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
                        .insert(ProcessRecord::legacy(42, 10), "recovered owner".into());
                    if published {
                        lifecycle.adopted_game_published(42, 10);
                    }
                    for attempt in 0..2 {
                        // What /prepare-relocation runs: refresh, then the refuse-first lease.
                        lifecycle.refresh_adopted();
                        let refused = lifecycle
                            .begin_relocation(&format!("move {attempt}"))
                            .unwrap_err();
                        assert_eq!(
                            refused.blockers,
                            [RuntimeBlocker::new(kind, "recovered owner").with_pid(42)],
                            "observation {start:?}"
                        );
                        {
                            let state = lifecycle.state.lock().unwrap();
                            assert!(state.relocation.is_none() && state.shutdown.is_none());
                        }
                        drop(lifecycle.admit("launch after refused move").unwrap());
                    }
                    let request = format!("{purpose:?}");
                    lifecycle
                        .prepare_shutdown(&request, purpose, async { Ok(()) })
                        .unwrap();
                    let status = loop {
                        let status = lifecycle.shutdown_status(&request, purpose).unwrap();
                        if status.phase != ShutdownPhase::Draining {
                            break status;
                        }
                        tokio::task::yield_now().await;
                    };
                    assert_eq!(status.phase, ShutdownPhase::Ready, "{start:?} {purpose:?}");
                    assert!(status.blockers.is_empty(), "{:?}", status.blockers);
                    lifecycle
                        .commit_prepared_shutdown(&request, purpose)
                        .unwrap();
                    assert!(lifecycle.shutdown_committed());
                    assert!(
                        lifecycle
                            .state
                            .lock()
                            .unwrap()
                            .adopted
                            .contains_key(&ProcessRecord::legacy(42, 10)),
                        "the record stays evidence for a later move"
                    );
                }
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
            .insert(ProcessRecord::legacy(42, 10), "proven adopted game".into());
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
                .contains_key(&ProcessRecord::legacy(42, 10))
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
            .insert(ProcessRecord::legacy(42, 10), "recovered server".into());
        lifecycle.refresh_adopted();
        let refused = lifecycle.begin_relocation("move").unwrap_err();
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

/// Relocation refuses before it changes anything. A move attempted while a
/// game, server, download or unverified process exists must leave the game's
/// session worker, the servers and admission exactly as they were.
#[cfg(test)]
mod relocation_refusal_tests {
    use super::*;
    use RuntimeBlockerKind as Kind;
    use tokio_util::sync::CancellationToken;

    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    fn blocker(kind: Kind, name: &str) -> RuntimeBlocker {
        RuntimeBlocker::new(kind, name)
    }
    /// No lease, no freeze, no cancelled token, and new work is still admitted.
    fn assert_nothing_changed(lifecycle: &Arc<RuntimeLifecycle>, tokens: &[CancellationToken]) {
        {
            let state = lifecycle.state.lock().unwrap();
            assert!(state.relocation.is_none(), "refusal took a lease");
            assert!(state.shutdown.is_none(), "refusal froze shutdown");
        }
        assert!(!lifecycle.admission_frozen());
        assert!(
            tokens.iter().all(|token| !token.is_cancelled()),
            "refusal cancelled running work"
        );
        drop(lifecycle.admit("launch after refused move").unwrap());
    }

    #[test]
    fn running_game_refuses_once_and_keeps_its_session_worker() {
        let lifecycle = ready();
        // Admitted like InstanceManager::prepare_game: downloads first.
        let worker = lifecycle
            .admit_owned("instance 3 preparation/session", ActivityPolicy::Installer)
            .unwrap();
        // Each Try again is a new attempt under a new id, as in the launcher.
        assert_eq!(
            lifecycle.begin_relocation("move 1").unwrap_err().blockers,
            [blocker(Kind::Install, "instance 3 preparation/session")]
        );
        let mut game = lifecycle.admit("game 3").unwrap();
        game.external();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        worker.follow_spawned_game(&game).unwrap();
        let tokens = [worker.cancellation(), game.cancellation()];

        let refused = lifecycle.begin_relocation("move 2").unwrap_err();
        // The worker is the game's own reader, so the game names both.
        assert_eq!(refused.blockers, [blocker(Kind::Game, "game 3")]);
        assert_nothing_changed(&lifecycle, &tokens);

        // Closing the game ends the game entry; a worker still settling is named.
        game.finish();
        let refused = lifecycle.begin_relocation("move 3").unwrap_err();
        assert_eq!(
            refused.blockers,
            [blocker(Kind::Task, "instance 3 preparation/session")]
        );
        assert_nothing_changed(&lifecycle, &tokens);

        worker.finish();
        lifecycle.begin_relocation("move 4").unwrap();
    }

    #[test]
    fn each_writer_kind_refuses_with_its_own_blocker() {
        let lifecycle = ready();
        let mut server = lifecycle.admit("server 7 run").unwrap();
        server.external();
        server.spawn(ActivityPolicy::Server, || Ok(())).unwrap();
        let install = lifecycle
            .admit_owned("addon installation tree", ActivityPolicy::Installer)
            .unwrap();
        let lost = lifecycle.admit("lost deletion").unwrap();
        lost.unknown("controlled".into());
        let task = lifecycle.admit("instance export").unwrap();
        lifecycle
            .state
            .lock()
            .unwrap()
            .adopted
            .insert(ProcessRecord::legacy(42, 10), "recovered server".into());
        lifecycle.adopted_record_published(ProcessRecord::legacy(43, 10));
        let tokens = [
            server.cancellation(),
            install.cancellation(),
            lost.cancellation(),
            task.cancellation(),
        ];

        let refused = lifecycle.begin_relocation("move").unwrap_err();
        assert_eq!(
            refused.blockers,
            [
                blocker(Kind::Server, "server 7 run"),
                blocker(Kind::Install, "addon installation tree"),
                blocker(Kind::Unconfirmed, "lost deletion"),
                blocker(Kind::Task, "instance export"),
                // A game published without a folder is named by its PID alone.
                blocker(Kind::PreviousGame, "").with_pid(43),
                blocker(Kind::PreviousProcess, "recovered server").with_pid(42),
            ]
        );
        assert!(refused.to_string().contains("server 7 run"), "{refused}");
        assert_nothing_changed(&lifecycle, &tokens);
    }

    #[test]
    fn startup_check_refuses_while_pending_and_names_a_failure() {
        let pending = Arc::new(RuntimeLifecycle::default());
        assert_eq!(
            pending.begin_relocation("move").unwrap_err().blockers,
            [blocker(Kind::StartupCheckPending, "")]
        );
        assert_nothing_changed(&pending, &[]);

        let failed = Arc::new(RuntimeLifecycle::default());
        failed.reconciliation_finished(Err(anyhow::anyhow!("Cannot reconcile /data/instances/A")));
        failed.reconciliation_finished(Ok(()));
        assert_eq!(
            failed.begin_relocation("move").unwrap_err().blockers,
            [blocker(
                Kind::StartupCheckFailed,
                "Cannot reconcile /data/instances/A"
            )]
        );
        assert_nothing_changed(&failed, &[]);
    }

    #[test]
    fn a_quit_in_progress_or_another_move_refuses_without_taking_over() {
        let lifecycle = ready();
        lifecycle
            .begin_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(
            lifecycle.begin_relocation("move").unwrap_err().blockers,
            [blocker(Kind::Shutdown, "")]
        );
        assert!(lifecycle.state.lock().unwrap().relocation.is_none());

        let lifecycle = ready();
        lifecycle.begin_relocation("first").unwrap();
        assert_eq!(
            lifecycle.begin_relocation("second").unwrap_err().blockers,
            [blocker(Kind::Relocation, "")]
        );
        assert!(matches!(
            lifecycle.cancel_relocation("second"),
            Err(CancelRelocationError::Missing)
        ));
        lifecycle.cancel_relocation("first").unwrap();
        assert_nothing_changed(&lifecycle, &[]);
    }

    #[test]
    fn a_confirmed_gone_record_clears_and_the_move_commits() {
        #[derive(Debug)]
        struct Gone;
        impl ProcessAbsenceObserver for Gone {
            fn confirmed_absent(&self, _pid: u32, _start: u64) -> bool {
                true
            }
        }
        let lifecycle = Arc::new(RuntimeLifecycle {
            process_observer: Arc::new(Gone),
            ..Default::default()
        });
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle.adopted_record_published(ProcessRecord::legacy(42, 10));
        assert!(lifecycle.begin_relocation("busy").is_err());
        lifecycle.refresh_adopted();
        lifecycle.begin_relocation("move").unwrap();
        assert!(lifecycle.admit("late launch").is_err());
        assert!(!lifecycle.shutdown_committed());
        lifecycle.commit_shutdown("move").unwrap();
        assert!(lifecycle.shutdown_committed());
        lifecycle.begin_relocation("move").unwrap();
        assert!(matches!(
            lifecycle.cancel_relocation("move"),
            Err(CancelRelocationError::Committed)
        ));
        // A committed lease is not cancelled, so a retry still re-joins it.
        lifecycle.begin_relocation("move").unwrap();
    }

    /// A prepare-relocation the core gets to only after the launcher gave up
    /// on it and cancelled the id must not grant a lease nobody owns: that
    /// lease would refuse every launch and every later move until a restart.
    #[test]
    fn a_cancelled_id_never_gets_a_lease() {
        // Cancelled before the core saw the prepare: nothing was held.
        let lifecycle = ready();
        assert!(matches!(
            lifecycle.cancel_relocation("late"),
            Err(CancelRelocationError::Missing)
        ));
        let refused = lifecycle.begin_relocation("late").unwrap_err();
        assert!(refused.cancelled);
        assert!(refused.blockers.is_empty());
        assert!(refused.to_string().contains("cancelled"), "{refused}");
        assert_nothing_changed(&lifecycle, &[]);

        // Released after a grant: a duplicate prepare cannot take it again.
        lifecycle.begin_relocation("released").unwrap();
        lifecycle.cancel_relocation("released").unwrap();
        assert!(
            lifecycle
                .begin_relocation("released")
                .unwrap_err()
                .cancelled
        );
        assert_nothing_changed(&lifecycle, &[]);

        // A new attempt has a new id and is decided on what runs.
        lifecycle.begin_relocation("fresh").unwrap();
        lifecycle.cancel_relocation("fresh").unwrap();
    }

    /// The launcher's first prepare goes unanswered, its retry with the same
    /// id is refused while a download finishes, and the user sees the busy
    /// view. If the first prepare reaches the core only once the download is
    /// done, it must not take a lease nobody owns: that lease would refuse
    /// every launch and every later move until a restart.
    #[test]
    fn a_refused_id_never_gets_a_lease_later() {
        let lifecycle = ready();
        let install = lifecycle
            .admit_owned("addon installation tree", ActivityPolicy::Installer)
            .unwrap();
        let refused = lifecycle.begin_relocation("stalled").unwrap_err();
        assert!(!refused.cancelled);
        assert_eq!(
            refused.blockers,
            [blocker(Kind::Install, "addon installation tree")]
        );
        install.finish();

        let late = lifecycle.begin_relocation("stalled").unwrap_err();
        assert!(late.cancelled, "{late}");
        assert!(late.blockers.is_empty());
        assert_nothing_changed(&lifecycle, &[]);

        // Try again uses a new id and is decided on what runs now.
        lifecycle.begin_relocation("try again").unwrap();
        lifecycle.cancel_relocation("try again").unwrap();
        assert_nothing_changed(&lifecycle, &[]);
    }

    #[test]
    fn cancelled_ids_are_remembered_up_to_a_bound() {
        let lifecycle = ready();
        for attempt in 0..=CANCELLED_RELOCATIONS_KEPT {
            let _ = lifecycle.cancel_relocation(&format!("attempt {attempt}"));
        }
        let _ = lifecycle.cancel_relocation("attempt 1");
        let state = lifecycle.state.lock().unwrap();
        assert_eq!(
            state.cancelled_relocations.len(),
            CANCELLED_RELOCATIONS_KEPT
        );
        assert!(
            !state
                .cancelled_relocations
                .iter()
                .any(|id| id == "attempt 0")
        );
        drop(state);
        lifecycle.begin_relocation("attempt 0").unwrap();
    }

    /// The check before the folder scan answers like begin_relocation would,
    /// without taking the lease, and the caller's own lease is not a blocker.
    #[test]
    fn the_blocker_check_takes_no_lease() {
        let lifecycle = ready();
        let task = lifecycle.admit("instance export").unwrap();
        assert_eq!(
            lifecycle.relocation_blockers("move"),
            [blocker(Kind::Task, "instance export")]
        );
        assert_nothing_changed(&lifecycle, &[task.cancellation()]);
        task.finish();
        assert!(lifecycle.relocation_blockers("move").is_empty());
        assert_nothing_changed(&lifecycle, &[]);

        lifecycle.begin_relocation("move").unwrap();
        assert!(lifecycle.relocation_blockers("move").is_empty());
        assert_eq!(
            lifecycle.relocation_blockers("other"),
            [blocker(Kind::Relocation, "")]
        );
    }

    /// A game left running by an earlier session is one entry, not two: its
    /// watcher is part of that game until the game is gone.
    #[test]
    fn an_adopted_game_names_its_watcher() {
        let lifecycle = ready();
        lifecycle
            .state
            .lock()
            .unwrap()
            .adopted
            .insert(ProcessRecord::legacy(42, 10), "/data/instances/Pack".into());
        lifecycle.adopted_record_published(ProcessRecord::legacy(42, 10));
        let watcher = lifecycle
            .admit_owned("adopted game core watcher", ActivityPolicy::CoreWork)
            .unwrap();
        watcher.watch_adopted_games();
        assert_eq!(
            lifecycle.begin_relocation("move 1").unwrap_err().blockers,
            [blocker(Kind::PreviousGame, "/data/instances/Pack").with_pid(42)]
        );
        // The game is gone but its watcher is still finishing up.
        {
            let mut state = lifecycle.state.lock().unwrap();
            state.adopted.clear();
            state.adopted_games.clear();
        }
        assert_eq!(
            lifecycle.begin_relocation("move 2").unwrap_err().blockers,
            [blocker(Kind::Task, "adopted game core watcher")]
        );
        watcher.finish();
        lifecycle.begin_relocation("move 3").unwrap();
    }

    /// A lease that could not be released or whose exit was not observed must
    /// not keep the user from quitting, restarting or updating.
    #[tokio::test]
    async fn quit_is_never_pinned_by_a_relocation_lease() {
        for committed in [false, true] {
            let lifecycle = ready();
            lifecycle.begin_relocation("move").unwrap();
            if committed {
                lifecycle.commit_shutdown("move").unwrap();
            }
            lifecycle
                .prepare_shutdown_and_commit("quit", ShutdownPurpose::Quit, async { Ok(()) })
                .await
                .unwrap();
            assert!(lifecycle.shutdown_committed());
        }
    }
}

/// The operation grace period runs beside the server drain. A local server may
/// take up to the drain's own bound to save and stop, and that is not a reason
/// to refuse the quit, restart or update once it has stopped.
#[cfg(test)]
mod shutdown_drain_budget_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::Instant;

    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    /// A running local server as start_server_impl and the supervisor leave it:
    /// admitted, spawned as Server/Running, retained until finalize_run.
    fn server_run(lifecycle: &Arc<RuntimeLifecycle>, owner: &str) -> RuntimeActivity {
        let mut run = lifecycle.admit(owner).unwrap();
        run.external();
        run.spawn(ActivityPolicy::Server, || Ok(())).unwrap();
        run
    }
    /// Polls like Electron does and returns every phase seen plus the final status.
    async fn settle(
        lifecycle: &RuntimeLifecycle,
        request: &str,
        purpose: ShutdownPurpose,
    ) -> (Vec<ShutdownPhase>, ShutdownStatus) {
        let mut seen = vec![];
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let status = lifecycle.shutdown_status(request, purpose).unwrap();
            if seen.last() != Some(&status.phase) {
                seen.push(status.phase);
            }
            if status.phase != ShutdownPhase::Draining {
                return (seen, status);
            }
            assert!(Instant::now() < deadline, "shutdown never settled");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn slow_server_stop_readies(stop: Duration) {
        let lifecycle = ready();
        let run = server_run(&lifecycle, "slow server");
        let started = Instant::now();
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async move {
                // 'stop' sent; the world saves, the process exits, finalize_run
                // publishes the logs and finishes the run.
                tokio::time::sleep(stop).await;
                run.finish();
                Ok(())
            })
            .unwrap();
        let (seen, status) = settle(&lifecycle, "quit", ShutdownPurpose::Quit).await;
        assert_eq!(
            seen,
            [ShutdownPhase::Draining, ShutdownPhase::Ready],
            "{:?}",
            status.blockers
        );
        assert!(status.blockers.is_empty());
        assert!(started.elapsed() >= stop);
        lifecycle
            .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn a_server_that_takes_25_seconds_to_stop_does_not_fail_the_quit() {
        slow_server_stop_readies(Duration::from_secs(25)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_hung_server_killed_after_the_grace_period_does_not_fail_the_quit() {
        // 30 s graceful stop, 1 s + 5 s kill, 5 s log settle.
        slow_server_stop_readies(Duration::from_secs(41)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn an_operation_that_never_settles_still_blocks_after_the_budget() {
        for policy in [ActivityPolicy::Installer, ActivityPolicy::CoreWork] {
            let lifecycle = ready();
            let operation = lifecycle.admit_owned("stuck installer", policy).unwrap();
            let started = Instant::now();
            lifecycle
                .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
                .unwrap();
            tokio::time::sleep(OPERATION_SETTLE_BUDGET - Duration::from_secs(1)).await;
            assert_eq!(
                lifecycle
                    .shutdown_status("quit", ShutdownPurpose::Quit)
                    .unwrap()
                    .phase,
                ShutdownPhase::Draining,
                "the operation gets the whole budget"
            );
            let (_, status) = settle(&lifecycle, "quit", ShutdownPurpose::Quit).await;
            assert_eq!(status.phase, ShutdownPhase::Unresolved);
            assert!(started.elapsed() >= OPERATION_SETTLE_BUDGET);
            assert!(
                status
                    .blockers
                    .iter()
                    .any(|blocker| blocker.name == "stuck installer"),
                "{:?}",
                status.blockers
            );
            assert!(
                lifecycle
                    .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
                    .is_err()
            );
            operation.finish();
        }
    }

    #[tokio::test(start_paused = true)]
    async fn an_operation_that_settles_after_the_budget_but_before_the_drain_is_not_a_blocker() {
        let lifecycle = ready();
        let run = server_run(&lifecycle, "slow server");
        let operation = lifecycle
            .admit_owned("late writer", ActivityPolicy::CoreWork)
            .unwrap();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(18)).await;
            operation.finish();
        });
        lifecycle
            .prepare_shutdown("update", ShutdownPurpose::Update, async move {
                tokio::time::sleep(Duration::from_secs(25)).await;
                run.finish();
                Ok(())
            })
            .unwrap();
        let (seen, status) = settle(&lifecycle, "update", ShutdownPurpose::Update).await;
        assert_eq!(
            seen,
            [ShutdownPhase::Draining, ShutdownPhase::Ready],
            "{:?}",
            status.blockers
        );
        lifecycle
            .commit_prepared_shutdown("update", ShutdownPurpose::Update)
            .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_drain_names_its_error_and_the_server_it_left_running() {
        for drain_fails in [true, false] {
            let lifecycle = ready();
            let run = server_run(&lifecycle, "unconfirmed server");
            let started = Instant::now();
            lifecycle
                .prepare_shutdown("quit", ShutdownPurpose::Quit, async move {
                    if drain_fails {
                        anyhow::bail!(
                            "Server forced exit unconfirmed; files and ownership retained"
                        );
                    }
                    Ok(())
                })
                .unwrap();
            let (_, status) = settle(&lifecycle, "quit", ShutdownPurpose::Quit).await;
            assert_eq!(status.phase, ShutdownPhase::Unresolved);
            assert!(
                started.elapsed() < Duration::from_secs(1),
                "the drain's verdict is not delayed by the operation budget"
            );
            assert!(
                status.blockers.contains(&RuntimeBlocker::new(
                    RuntimeBlockerKind::Server,
                    "unconfirmed server"
                )),
                "a server run the drain did not finish is still named: {:?}",
                status.blockers
            );
            // A failed drain is not a server still stopping: retrying the
            // request only repeats it, so it is named as unconfirmed, which
            // the launcher does not wait on.
            assert_eq!(
                status.blockers.iter().any(|blocker| {
                    blocker.kind == RuntimeBlockerKind::Unconfirmed
                        && blocker.name.contains("Server forced exit unconfirmed")
                }),
                drain_fails,
                "{:?}",
                status.blockers
            );
            assert!(
                !status.blockers.iter().any(|blocker| {
                    blocker.kind == RuntimeBlockerKind::Server
                        && blocker.name.contains("Server forced exit unconfirmed")
                }),
                "{:?}",
                status.blockers
            );
            assert!(
                !status
                    .blockers
                    .iter()
                    .any(|blocker| blocker.name.contains("remain active")),
                "{:?}",
                status.blockers
            );
            assert!(
                lifecycle
                    .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
                    .is_err()
            );
            run.finish();
        }
    }

    #[tokio::test(start_paused = true)]
    async fn an_operation_turning_unknown_resolves_the_request_promptly() {
        let lifecycle = ready();
        let operation = lifecycle
            .admit_owned("lost worker", ActivityPolicy::CoreWork)
            .unwrap();
        let held = lifecycle
            .admit_owned("busy installer", ActivityPolicy::Installer)
            .unwrap();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            operation.unknown("worker supervision lost".into());
        });
        let started = Instant::now();
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let (_, status) = settle(&lifecycle, "quit", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert!(
            started.elapsed() < OPERATION_SETTLE_BUDGET,
            "Unknown does not wait out the budget"
        );
        assert!(
            status.blockers.contains(&RuntimeBlocker::new(
                RuntimeBlockerKind::Unconfirmed,
                "lost worker"
            )),
            "{:?}",
            status.blockers
        );
        assert!(
            status.blockers.contains(&RuntimeBlocker::new(
                RuntimeBlockerKind::Install,
                "busy installer"
            )),
            "{:?}",
            status.blockers
        );
        held.finish();
    }
}

/// A quit, restart or update that cannot finish never leaves the core
/// frozen and never stops tracking a running game: the user can cancel it
/// and keep using the launcher, or start another attempt in its place.
#[cfg(test)]
mod failed_shutdown_tests {
    use super::*;
    use std::time::Duration;

    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    /// Polls like Electron does until the request leaves Draining.
    async fn settled(
        lifecycle: &RuntimeLifecycle,
        request: &str,
        purpose: ShutdownPurpose,
    ) -> ShutdownStatus {
        tokio::time::timeout(Duration::from_secs(120), async {
            loop {
                let status = lifecycle.shutdown_status(request, purpose).unwrap();
                if status.phase != ShutdownPhase::Draining {
                    return status;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("shutdown never settled")
    }
    /// A long copy or Recycle Bin move: it does not look at its cancellation.
    fn stuck(lifecycle: &Arc<RuntimeLifecycle>) -> RuntimeActivity {
        lifecycle
            .admit_owned("stuck copy", ActivityPolicy::CoreWork)
            .unwrap()
    }
    fn admission_open(lifecycle: &Arc<RuntimeLifecycle>) -> bool {
        let Ok(mut launch) = lifecycle.admit("launch after cancel") else {
            return false;
        };
        let spawned = launch.spawn(ActivityPolicy::Game, || Ok(())).is_ok();
        launch.finish();
        spawned
    }
    /// A game this core started and its session worker, as instance/run
    /// sets them up: the worker detaches the game when cancelled, and
    /// otherwise sees it exit and sets the instance back to Inactive.
    struct Session {
        worker: tokio::task::JoinHandle<&'static str>,
        cancel: tokio_util::sync::CancellationToken,
        exit: tokio::sync::oneshot::Sender<()>,
    }
    fn session(lifecycle: &Arc<RuntimeLifecycle>, instance: u32) -> Session {
        let follower = lifecycle.admit(format!("game {instance} session")).unwrap();
        let mut game = lifecycle
            .admit_owned(format!("game {instance}"), ActivityPolicy::Game)
            .unwrap();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        follower.follow_spawned_game(&game).unwrap();
        let cancel = follower.cancellation();
        let (exit, exited) = tokio::sync::oneshot::channel();
        let worker = spawn_retained(follower, move |slot| {
            let cancel = slot.as_ref().unwrap().cancellation();
            Box::pin(async move {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        game.detach_surviving_game().unwrap();
                        "detached"
                    }
                    _ = exited => {
                        game.finish();
                        "inactive"
                    }
                }
            })
        });
        Session {
            worker,
            cancel,
            exit,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn cancelling_an_unresolved_quit_reopens_admission_and_retires_its_id() {
        let lifecycle = ready();
        let copy = stuck(&lifecycle);
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let status = settled(&lifecycle, "quit-1", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert_eq!(
            status.blockers,
            [RuntimeBlocker::new(RuntimeBlockerKind::Task, "stuck copy")]
        );
        assert!(
            !admission_open(&lifecycle),
            "an unresolved quit freezes admission"
        );

        assert!(matches!(
            lifecycle.cancel_shutdown("someone else"),
            Err(CancelShutdownError::Missing)
        ));
        assert!(lifecycle.admission_frozen());
        lifecycle.cancel_shutdown("quit-1").unwrap();
        assert!(!lifecycle.admission_frozen());
        assert!(
            admission_open(&lifecycle),
            "Play, installs and spawns work again"
        );
        assert!(
            lifecycle
                .shutdown_status("quit-1", ShutdownPurpose::Quit)
                .is_err()
        );

        // A late duplicate of the cancelled request cannot freeze the core again.
        assert!(matches!(
            lifecycle.cancel_shutdown("quit-1"),
            Err(CancelShutdownError::Missing)
        ));
        assert!(
            lifecycle
                .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async { Ok(()) })
                .is_err()
        );
        assert!(!lifecycle.admission_frozen());
        copy.finish();
    }

    #[tokio::test(start_paused = true)]
    async fn a_drain_that_finishes_after_the_cancel_changes_nothing() {
        let lifecycle = ready();
        let (finish_drain, drain_finished) = tokio::sync::oneshot::channel::<()>();
        let (drained, drain_done) = tokio::sync::oneshot::channel::<()>();
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async move {
                let _ = drain_finished.await;
                let _ = drained.send(());
                Ok(())
            })
            .unwrap();
        tokio::task::yield_now().await;
        assert_eq!(
            lifecycle
                .shutdown_status("quit-1", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            ShutdownPhase::Draining
        );
        lifecycle.cancel_shutdown("quit-1").unwrap();
        finish_drain.send(()).unwrap();
        drain_done.await.unwrap();
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert!(lifecycle.state.lock().unwrap().shutdown.is_none());
        assert!(admission_open(&lifecycle));
    }

    #[tokio::test]
    async fn a_committed_request_cannot_be_cancelled() {
        let lifecycle = ready();
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert_eq!(
            settled(&lifecycle, "quit-1", ShutdownPurpose::Quit)
                .await
                .phase,
            ShutdownPhase::Ready
        );
        assert!(matches!(
            lifecycle.cancel_shutdown("quit-1"),
            Err(CancelShutdownError::Finishing)
        ));
        lifecycle
            .commit_prepared_shutdown("quit-1", ShutdownPurpose::Quit)
            .unwrap();
        assert!(matches!(
            lifecycle.cancel_shutdown("quit-1"),
            Err(CancelShutdownError::Committed)
        ));
        assert!(lifecycle.shutdown_committed());
        assert!(lifecycle.admission_frozen());
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_request_replaces_only_an_unresolved_one() {
        let lifecycle = ready();
        let copy = stuck(&lifecycle);
        lifecycle
            .prepare_shutdown("update-1", ShutdownPurpose::Update, async { Ok(()) })
            .unwrap();
        assert_eq!(
            settled(&lifecycle, "update-1", ShutdownPurpose::Update)
                .await
                .phase,
            ShutdownPhase::Unresolved
        );
        // Closing the window after a failed update install.
        lifecycle
            .prepare_shutdown("quit-2", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert!(
            lifecycle
                .shutdown_status("update-1", ShutdownPurpose::Update)
                .is_err()
        );
        assert!(
            lifecycle
                .begin_shutdown("update-1", ShutdownPurpose::Update)
                .is_err(),
            "the replaced request is retired"
        );
        // While it drains, it is not replaced.
        assert!(
            lifecycle
                .begin_shutdown("update-3", ShutdownPurpose::Update)
                .is_err()
        );
        assert_eq!(
            settled(&lifecycle, "quit-2", ShutdownPurpose::Quit)
                .await
                .phase,
            ShutdownPhase::Unresolved
        );
        copy.finish();
        // The same request retries once it is unresolved.
        lifecycle
            .prepare_shutdown("quit-2", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert_eq!(
            settled(&lifecycle, "quit-2", ShutdownPurpose::Quit)
                .await
                .phase,
            ShutdownPhase::Ready
        );
        assert!(
            lifecycle
                .begin_shutdown("update-4", ShutdownPurpose::Update)
                .is_err(),
            "a ready request is not replaced"
        );
        lifecycle
            .commit_prepared_shutdown("quit-2", ShutdownPurpose::Quit)
            .unwrap();
        assert!(
            lifecycle
                .begin_shutdown("update-5", ShutdownPurpose::Update)
                .is_err(),
            "a committed request is not replaced"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn servers_stopped_by_a_replaced_attempt_are_still_reported() {
        let lifecycle = ready();
        let mut server = lifecycle.admit("server 7 run").unwrap();
        server.external();
        server.spawn(ActivityPolicy::Server, || Ok(())).unwrap();
        let copy = stuck(&lifecycle);
        let draining = lifecycle.clone();
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async move {
                // As ServerManager::shutdown_running_for does once it sent 'stop'.
                draining.shutdown_drain("quit-1").servers_asked_to_stop();
                server.finish();
                Ok(())
            })
            .unwrap();
        let status = settled(&lifecycle, "quit-1", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert!(status.servers_stopped);
        lifecycle
            .prepare_shutdown("update-2", ShutdownPurpose::Update, async { Ok(()) })
            .unwrap();
        assert!(
            lifecycle
                .shutdown_status("update-2", ShutdownPurpose::Update)
                .unwrap()
                .servers_stopped
        );
        copy.finish();
    }

    /// The note that servers were stopped is only shown once the drain
    /// asked one to stop: a server that is running while the drain still
    /// waits for something else was not stopped yet.
    #[tokio::test(start_paused = true)]
    async fn servers_count_as_stopped_only_once_the_drain_asked_them() {
        let lifecycle = ready();
        let mut server = lifecycle.admit("server 7 run").unwrap();
        server.external();
        server.spawn(ActivityPolicy::Server, || Ok(())).unwrap();
        let (asked, ask) = tokio::sync::oneshot::channel::<()>();
        let draining = lifecycle.clone();
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async move {
                let _ = ask.await;
                draining.shutdown_drain("quit-1").servers_asked_to_stop();
                server.finish();
                Ok(())
            })
            .unwrap();
        tokio::task::yield_now().await;
        let status = lifecycle
            .shutdown_status("quit-1", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(status.phase, ShutdownPhase::Draining);
        assert!(!status.servers_stopped);
        asked.send(()).unwrap();
        let status = settled(&lifecycle, "quit-1", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Ready);
        assert!(status.servers_stopped);
    }

    /// Keep GDLauncher open, or a later request taking the place of an
    /// unresolved one, tells a drain still running for the old request to
    /// leave alone the servers it has not asked to stop yet. A drain that
    /// starts after its request is gone touches nothing.
    #[tokio::test(start_paused = true)]
    async fn a_cancelled_or_replaced_request_tells_its_drain_to_stop_asking() {
        let lifecycle = ready();
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, std::future::pending())
            .unwrap();
        let drain = lifecycle.shutdown_drain("quit-1");
        assert!(!drain.cancelled());
        lifecycle.cancel_shutdown("quit-1").unwrap();
        assert!(drain.cancelled());
        assert!(lifecycle.shutdown_drain("quit-1").cancelled());
        // Reporting for a request that is gone changes nothing.
        drain.servers_asked_to_stop();

        let copy = stuck(&lifecycle);
        lifecycle
            .prepare_shutdown("quit-2", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let retried = lifecycle.shutdown_drain("quit-2");
        assert_eq!(
            settled(&lifecycle, "quit-2", ShutdownPurpose::Quit)
                .await
                .phase,
            ShutdownPhase::Unresolved
        );
        // A retry of the same request keeps its handle live.
        lifecycle
            .begin_shutdown("quit-2", ShutdownPurpose::Quit)
            .unwrap();
        assert!(!retried.cancelled());
        lifecycle.cancel_shutdown("quit-2").unwrap();
        assert!(retried.cancelled());

        lifecycle
            .prepare_shutdown("quit-3", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let unresolved = lifecycle.shutdown_drain("quit-3");
        assert_eq!(
            settled(&lifecycle, "quit-3", ShutdownPurpose::Quit)
                .await
                .phase,
            ShutdownPhase::Unresolved
        );
        lifecycle
            .begin_shutdown("update-4", ShutdownPurpose::Update)
            .unwrap();
        assert!(unresolved.cancelled(), "a replaced request is cancelled");
        assert!(!lifecycle.shutdown_drain("update-4").cancelled());
        assert!(
            !lifecycle
                .shutdown_status("update-4", ShutdownPurpose::Update)
                .unwrap()
                .servers_stopped
        );
        copy.finish();
    }

    /// A retired id is refused with its own error, which the route reports
    /// as `cancelled: true` so the launcher starts over under a new id.
    #[test]
    fn a_retired_id_is_refused_as_such() {
        let lifecycle = ready();
        lifecycle
            .begin_shutdown("quit-1", ShutdownPurpose::Quit)
            .unwrap();
        lifecycle.cancel_shutdown("quit-1").unwrap();
        let error = lifecycle
            .begin_shutdown("quit-1", ShutdownPurpose::Quit)
            .unwrap_err();
        assert!(error.is::<ShutdownRetired>(), "{error:#}");
        let error = lifecycle
            .begin_shutdown("", ShutdownPurpose::Quit)
            .unwrap_err();
        assert!(!error.is::<ShutdownRetired>());
        assert!(!lifecycle.admission_frozen());
    }

    #[tokio::test(start_paused = true)]
    async fn a_refused_attempt_keeps_tracking_running_games_until_the_retry_can_finish() {
        let lifecycle = ready();
        let game = session(&lifecycle, 3);
        let watcher = lifecycle
            .admit_owned("adopted game core watcher", ActivityPolicy::CoreWork)
            .unwrap();
        watcher.watch_adopted_games();
        let watcher_cancel = watcher.cancellation();
        let copy = stuck(&lifecycle);

        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        // While it drains, only what keeps it from finishing is listed.
        assert_eq!(
            lifecycle
                .shutdown_status("quit-1", ShutdownPurpose::Quit)
                .unwrap()
                .blockers,
            [RuntimeBlocker::new(RuntimeBlockerKind::Task, "stuck copy")]
        );
        let status = settled(&lifecycle, "quit-1", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert_eq!(
            status.blockers,
            [RuntimeBlocker::new(RuntimeBlockerKind::Task, "stuck copy")],
            "the session worker and watcher are not in the way"
        );
        assert!(!game.cancel.is_cancelled(), "the game is still tracked");
        assert!(!watcher_cancel.is_cancelled());

        // Keep GDLauncher open: the game exits later and is seen exiting.
        lifecycle.cancel_shutdown("quit-1").unwrap();
        game.exit.send(()).unwrap();
        assert_eq!(game.worker.await.unwrap(), "inactive");

        // Another quit once the copy is done hands the next game over.
        let next = session(&lifecycle, 4);
        lifecycle
            .prepare_shutdown("quit-2", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let status = settled(&lifecycle, "quit-2", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert!(!next.cancel.is_cancelled());
        copy.finish();
        lifecycle
            .prepare_shutdown("quit-2", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), next.cancel.cancelled())
            .await
            .expect("the session worker is cancelled once nothing else is left");
        assert!(watcher_cancel.is_cancelled());
        watcher.finish();
        assert_eq!(next.worker.await.unwrap(), "detached");
        let status = settled(&lifecycle, "quit-2", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Ready, "{:?}", status.blockers);
        lifecycle
            .commit_prepared_shutdown("quit-2", ShutdownPurpose::Quit)
            .unwrap();
        drop(next.exit);
    }

    /// The end of a drain decides under one lock whether running games are
    /// handed over, so work that ends between the check and the settlement
    /// can never leave an unreleased session worker named as the blocker.
    #[tokio::test]
    async fn a_drain_hands_games_over_only_when_it_succeeded_and_nothing_else_is_left() {
        let lifecycle = ready();
        let game = session(&lifecycle, 8);
        let copy = stuck(&lifecycle);
        let status = |lifecycle: &RuntimeLifecycle| {
            lifecycle
                .shutdown_status("quit-1", ShutdownPurpose::Quit)
                .unwrap()
        };

        lifecycle
            .begin_shutdown("quit-1", ShutdownPurpose::Quit)
            .unwrap();
        assert!(
            lifecycle
                .release_followers_or_settle("quit-1", Ok(()))
                .is_none()
        );
        assert!(!game.cancel.is_cancelled());
        assert_eq!(status(&lifecycle).phase, ShutdownPhase::Unresolved);
        assert_eq!(
            status(&lifecycle).blockers,
            [RuntimeBlocker::new(RuntimeBlockerKind::Task, "stuck copy")]
        );

        // A server whose exit stayed unconfirmed keeps the games tracked too.
        copy.finish();
        assert!(
            lifecycle
                .begin_shutdown("quit-1", ShutdownPurpose::Quit)
                .unwrap()
        );
        assert!(
            lifecycle
                .release_followers_or_settle("quit-1", Err(anyhow::anyhow!("exit unconfirmed")))
                .is_none()
        );
        assert!(!game.cancel.is_cancelled());
        assert_eq!(
            status(&lifecycle).blockers,
            [RuntimeBlocker::new(
                RuntimeBlockerKind::Unconfirmed,
                "exit unconfirmed"
            )]
        );

        // Another request's drain changes nothing.
        assert!(
            lifecycle
                .begin_shutdown("quit-1", ShutdownPurpose::Quit)
                .unwrap()
        );
        assert!(
            lifecycle
                .release_followers_or_settle("quit-0", Ok(()))
                .is_none()
        );
        assert_eq!(status(&lifecycle).phase, ShutdownPhase::Draining);

        let followers = lifecycle
            .release_followers_or_settle("quit-1", Ok(()))
            .expect("nothing else is left, so the game is handed over");
        assert_eq!(followers.len(), 1);
        assert!(game.cancel.is_cancelled());
        assert_eq!(status(&lifecycle).phase, ShutdownPhase::Draining);
        assert_eq!(game.worker.await.unwrap(), "detached");
        wait_for_settlement(followers, FOLLOWER_SETTLE_BUDGET).await;
        assert_eq!(
            lifecycle.settle_shutdown("quit-1", Ok(())).unwrap().phase,
            ShutdownPhase::Ready
        );
        drop(game.exit);
    }

    #[tokio::test(start_paused = true)]
    async fn a_session_worker_that_does_not_settle_after_its_release_is_named() {
        let lifecycle = ready();
        let follower = lifecycle.admit("game 5 session").unwrap();
        let mut game = lifecycle
            .admit_owned("game 5", ActivityPolicy::Game)
            .unwrap();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        follower.follow_spawned_game(&game).unwrap();
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let status = settled(&lifecycle, "quit-1", ShutdownPurpose::Quit).await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert_eq!(
            status.blockers,
            [RuntimeBlocker::new(
                RuntimeBlockerKind::Task,
                "game 5 session"
            )]
        );
        assert!(follower.cancellation().is_cancelled());
        // Its game is already handed over, so going back is still allowed
        // once the attempt ended unresolved.
        lifecycle.cancel_shutdown("quit-1").unwrap();
        follower.finish();
        drop(game);
    }

    #[tokio::test(start_paused = true)]
    async fn a_request_whose_games_were_handed_over_is_finishing() {
        let lifecycle = ready();
        let (settle, settled_rx) = tokio::sync::oneshot::channel::<()>();
        let follower = lifecycle.admit("game 6 session").unwrap();
        let mut game = lifecycle
            .admit_owned("game 6", ActivityPolicy::Game)
            .unwrap();
        game.spawn(ActivityPolicy::Game, || Ok(())).unwrap();
        follower.follow_spawned_game(&game).unwrap();
        let cancel = follower.cancellation();
        let worker = spawn_retained(follower, move |_| {
            Box::pin(async move {
                let _ = settled_rx.await;
                game.detach_surviving_game().unwrap();
            })
        });
        lifecycle
            .prepare_shutdown("quit-1", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), cancel.cancelled())
            .await
            .unwrap();
        assert!(matches!(
            lifecycle.cancel_shutdown("quit-1"),
            Err(CancelShutdownError::Finishing)
        ));
        settle.send(()).unwrap();
        worker.await.unwrap();
        assert_eq!(
            settled(&lifecycle, "quit-1", ShutdownPurpose::Quit)
                .await
                .phase,
            ShutdownPhase::Ready
        );
    }

    #[test]
    fn a_deletion_is_named_as_one() {
        let lifecycle = ready();
        let deletion = lifecycle
            .admit_owned("delete instance 12", ActivityPolicy::Deletion)
            .unwrap();
        assert_eq!(
            lifecycle.relocation_blockers("move"),
            [RuntimeBlocker::new(
                RuntimeBlockerKind::Deletion,
                "delete instance 12"
            )]
        );
        drop(
            lifecycle
                .admit_as("server deletion", ActivityPolicy::Deletion)
                .unwrap(),
        );
        assert_eq!(
            lifecycle.relocation_blockers("move").len(),
            1,
            "dropping it completes it"
        );
        deletion.finish();
    }
}

#[cfg(test)]
mod task22_tests {
    use super::*;
    /// The real classification on an injected observation. Injected Absent
    /// means the native probe confirmed no process holds the PID.
    #[derive(Debug)]
    struct SyntheticObserver(Mutex<orphan_pid::ProcessObservation>);
    impl ProcessAbsenceObserver for SyntheticObserver {
        fn confirmed_absent(&self, _: u32, _: u64) -> bool {
            panic!("Stable route required")
        }
        fn observation(&self, _: ProcessRecord) -> orphan_pid::ProcessObservation {
            *self.0.lock().unwrap()
        }
        fn status(&self, record: ProcessRecord, require_java: bool) -> IdentityStatus {
            classify_observed(
                record,
                self.observation(record),
                require_java,
                orphan_pid::EPOCH_START_TIMES_ARE_STABLE,
                || true,
            )
        }
    }
    fn record() -> ProcessRecord {
        ProcessRecord {
            pid: 4242,
            start_time: Some(1700000000),
            identity: Some(orphan_pid::ProcessIdentity::LinuxBootTicks {
                boot_id: uuid::Uuid::parse_str("cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80").unwrap(),
                start_ticks: 123456,
            }),
        }
    }
    fn observed(record: ProcessRecord, epoch: u64) -> orphan_pid::ProcessObservation {
        orphan_pid::ProcessObservation::Present {
            process: orphan_pid::LiveProc {
                is_java: true,
                start_time: epoch,
            },
            identity: record.identity,
        }
    }
    #[test]
    fn task22_lifecycle_clock_unknown_stop_and_captured_release_controls() {
        let record = record();
        let observer = Arc::new(SyntheticObserver(Mutex::new(observed(record, 1700000030))));
        let lifecycle = Arc::new(RuntimeLifecycle {
            process_observer: observer.clone(),
            ..Default::default()
        });
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle.adopted_record_published(record);
        assert!(lifecycle.begin_relocation("clock".into()).is_err());
        assert!(
            lifecycle
                .state
                .lock()
                .unwrap()
                .adopted
                .contains_key(&record)
        );
        *observer.0.lock().unwrap() = orphan_pid::ProcessObservation::Unknown;
        lifecycle.refresh_adopted();
        assert!(lifecycle.begin_relocation("unknown".into()).is_err());
        // A requested Stop changes no authority; only observed exit/reuse releases.
        let mut replacement = record;
        replacement.identity = Some(orphan_pid::ProcessIdentity::LinuxBootTicks {
            boot_id: uuid::Uuid::new_v4(),
            start_ticks: 123456,
        });
        lifecycle.adopted_record_published(replacement);
        *observer.0.lock().unwrap() = observed(replacement, 1700000000);
        lifecycle.refresh_adopted();
        // Another boot/ticks identity behind the PID is reuse on every platform.
        assert!(
            !lifecycle
                .state
                .lock()
                .unwrap()
                .adopted
                .contains_key(&record)
        );
        assert!(
            lifecycle
                .state
                .lock()
                .unwrap()
                .adopted
                .contains_key(&replacement)
        );
        *observer.0.lock().unwrap() = orphan_pid::ProcessObservation::Absent;
        lifecycle.refresh_adopted();
        assert!(lifecycle.begin_relocation("gone".into()).is_ok());
    }
    #[tokio::test]
    async fn task22_durable_child_restart_clock_reuse_unknown_and_legacy_controls() {
        let root = tempfile::tempdir().unwrap();
        let record = record();
        let observer = Arc::new(SyntheticObserver(Mutex::new(observed(record, 1700000030))));
        let mut lifecycle = RuntimeLifecycle::with_runtime_path(root.path());
        lifecycle.process_observer = observer.clone();
        let path = lifecycle.child_records.as_ref().unwrap();
        std::fs::create_dir_all(path).unwrap();
        let file = path.join("owned.json");
        std::fs::write(&file, encode_child_record(record).unwrap()).unwrap();
        lifecycle.reconcile_children().await.unwrap();
        assert!(file.exists());
        assert!(
            lifecycle
                .state
                .lock()
                .unwrap()
                .adopted
                .contains_key(&record)
        );
        *observer.0.lock().unwrap() = orphan_pid::ProcessObservation::Unknown;
        lifecycle.reconcile_children().await.unwrap();
        assert!(file.exists());
        let mut reused = record;
        reused.identity = Some(orphan_pid::ProcessIdentity::LinuxBootTicks {
            boot_id: uuid::Uuid::new_v4(),
            start_ticks: 123456,
        });
        *observer.0.lock().unwrap() = observed(reused, 1700000000);
        lifecycle.reconcile_children().await.unwrap();
        // Another boot/ticks identity behind the PID is reuse on every platform.
        assert!(!file.exists());
        let legacy = ProcessRecord::legacy(4242, 1700000000);
        let legacy_bytes = serde_json::to_vec(&(legacy.pid, legacy.start_time.unwrap())).unwrap();
        std::fs::write(&file, &legacy_bytes).unwrap();
        *observer.0.lock().unwrap() = observed(record, 1700000030);
        lifecycle.reconcile_children().await.unwrap();
        if orphan_pid::EPOCH_START_TIMES_ARE_STABLE {
            // A creation time on Windows or macOS does not move with the
            // clock: another start time is another process.
            assert!(!file.exists());
            std::fs::write(&file, &legacy_bytes).unwrap();
        } else {
            // On Linux a clock step can explain it (M23): kept, unconfirmed.
            assert!(file.exists());
            assert!(
                lifecycle
                    .state
                    .lock()
                    .unwrap()
                    .adopted
                    .contains_key(&legacy)
            );
        }
        *observer.0.lock().unwrap() = observed(record, 1700000000);
        lifecycle.reconcile_children().await.unwrap();
        assert_eq!(
            decode_child_record(&std::fs::read(file).unwrap()).unwrap(),
            record
        );
    }
    #[test]
    fn task22_durable_malformed_version_has_no_legacy_fallback() {
        assert_eq!(
            decode_child_record(b"[4242,1700000000]").unwrap(),
            ProcessRecord::legacy(4242, 1700000000)
        );
        for bytes in [b"".as_slice(), b"{}", b"{\"version\":2,\"process\":{\"pid\":4242,\"start_time\":1700000000,\"identity\":null}}", b"{\"version\":1,\"process\":{\"pid\":4242,\"start_time\":1700000000,\"identity\":{\"kind\":\"unknown\"}}}"] { assert!(decode_child_record(bytes).is_err()); }
    }
}

#[cfg(test)]
mod task22_spawn_tests {
    use super::*;
    #[test]
    fn task22_owned_capture_persists_same_token_and_unconfirmed_intent() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let mut activity = lifecycle.admit("synthetic owned spawn").unwrap();
        activity.external_child().unwrap();
        let identity = orphan_pid::ProcessIdentity::LinuxBootTicks {
            boot_id: uuid::Uuid::parse_str("cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80").unwrap(),
            start_ticks: 123456,
        };
        activity
            .record_child_observed(
                4242,
                orphan_pid::ProcessObservation::Present {
                    process: orphan_pid::LiveProc {
                        is_java: true,
                        start_time: 1700000000,
                    },
                    identity: Some(identity),
                },
            )
            .unwrap();
        let record = activity.child_identity().unwrap();
        assert_eq!(record.identity, Some(identity));
        let path = activity.record.clone().unwrap();
        assert_eq!(
            decode_child_record(&std::fs::read(&path).unwrap()).unwrap(),
            record
        );
        activity.finish();
        assert!(!path.exists());
        let mut uncertain = lifecycle.admit("synthetic unconfirmed spawn").unwrap();
        uncertain.external_child().unwrap();
        uncertain
            .record_child_observed(4242, orphan_pid::ProcessObservation::Unknown)
            .unwrap();
        assert!(uncertain.child_identity().is_none());
        let path = uncertain.record.clone().unwrap();
        drop(uncertain);
        assert!(decode_child_record(&std::fs::read(path).unwrap()).is_err());
        assert!(lifecycle.begin_relocation("unknown".into()).is_err());
    }
}

#[cfg(test)]
mod task22_metadata_tests {
    use super::*;
    fn record(epoch: u64) -> ProcessRecord {
        ProcessRecord {
            pid: 4242,
            start_time: Some(epoch),
            identity: Some(orphan_pid::ProcessIdentity::LinuxBootTicks {
                boot_id: uuid::Uuid::parse_str("cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80").unwrap(),
                start_ticks: 123456,
            }),
        }
    }
    #[test]
    fn task22_stable_owner_metadata_does_not_split_game_survival_authority() {
        let lifecycle = RuntimeLifecycle::default();
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
            .state
            .lock()
            .unwrap()
            .adopted
            .insert(record(1700000000), "previous durable owner".into());
        lifecycle.adopted_record_published(record(1700000030));
        lifecycle
            .begin_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
        assert_eq!(
            lifecycle.settle_shutdown("quit", Ok(())).unwrap().phase,
            ShutdownPhase::Ready
        );
        assert_eq!(lifecycle.state.lock().unwrap().adopted.len(), 1);
    }
    #[test]
    fn task22_reconciled_durable_record_is_idempotent_for_same_stable_token() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = RuntimeLifecycle::with_runtime_path(root.path());
        lifecycle
            .persist_reconciled_process(record(1700000000))
            .unwrap();
        lifecycle
            .persist_reconciled_process(record(1700000030))
            .unwrap();
        assert_eq!(
            std::fs::read_dir(lifecycle.child_records.as_ref().unwrap())
                .unwrap()
                .count(),
            1
        );
    }
}

#[cfg(test)]
mod task22_invalid_token_tests {
    use super::*;
    #[test]
    fn task22_nil_boot_durable_extension_is_malformed_without_fallback() {
        let bytes = br#"{"version":1,"process":{"pid":4242,"start_time":1700000000,"identity":{"kind":"LinuxBootTicks","boot_id":"00000000-0000-0000-0000-000000000000","start_ticks":123456}}}"#;
        assert!(
            decode_child_record(bytes).is_err(),
            "Malformed stable token must remain Unknown, never prove reuse or gain epoch fallback"
        );
    }
}

/// Records and markers the startup check cannot resolve keep only a move of
/// the runtime folder from starting. Quit, restart and update never wait on
/// them, on a failed check or on a check that is still running.
#[cfg(test)]
mod ownership_evidence_tests {
    use super::*;
    use RuntimeBlockerKind as Kind;

    /// A PID some process still holds: never confirmed gone, identity unknown.
    #[derive(Debug)]
    struct NeverGone;
    impl ProcessAbsenceObserver for NeverGone {
        fn confirmed_absent(&self, _pid: u32, _start: u64) -> bool {
            false
        }
    }
    fn never_gone(root: &Path) -> Arc<RuntimeLifecycle> {
        Arc::new(RuntimeLifecycle {
            process_observer: Arc::new(NeverGone),
            ..RuntimeLifecycle::with_runtime_path(root)
        })
    }
    fn unreadable(path: &Path) -> RuntimeBlocker {
        RuntimeBlocker::new(Kind::UnreadableRecord, path.display().to_string())
    }
    /// Every file under `root` with its bytes; folders by name only.
    fn snapshot(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        let mut files: Vec<_> = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = path.is_file().then(|| std::fs::read(&path).unwrap());
                (path, bytes)
            })
            .collect();
        files.sort();
        files
    }

    /// (a) A runtime folder that could not be listed fails the startup check
    /// for the session. A move is refused and names that folder; quitting
    /// and installing an update still go through.
    #[tokio::test]
    async fn a_failed_startup_check_refuses_a_move_but_not_quit_or_update() {
        for purpose in [ShutdownPurpose::Quit, ShutdownPurpose::Update] {
            let root = tempfile::tempdir().unwrap();
            // A file where the instances folder should be cannot be listed.
            let instances = root.path().join("instances");
            std::fs::write(&instances, b"not a folder").unwrap();
            let lifecycle = Arc::new(RuntimeLifecycle::default());
            let error = lifecycle
                .reconcile_root(&instances, ".gdl_instance.pid")
                .await
                .unwrap_err();
            lifecycle.reconciliation_finished(Err(error));
            lifecycle.reconciliation_finished(Ok(()));

            let refused = lifecycle.begin_relocation("move").unwrap_err();
            assert_eq!(refused.blockers.len(), 1, "{refused}");
            assert_eq!(refused.blockers[0].kind, Kind::StartupCheckFailed);
            assert!(
                refused.blockers[0]
                    .name
                    .contains(&instances.display().to_string()),
                "{refused}"
            );
            assert!(
                refused
                    .to_string()
                    .contains(&instances.display().to_string())
            );

            lifecycle
                .prepare_shutdown_and_commit("shutdown", purpose, async { Ok(()) })
                .await
                .unwrap();
            assert_eq!(
                lifecycle
                    .shutdown_status("shutdown", purpose)
                    .unwrap()
                    .phase,
                ShutdownPhase::Committed
            );
        }
    }

    /// (b) Quitting in the first seconds, before the startup check is done,
    /// still stops servers and closes, through Electron's prepare/commit and
    /// through the core's own termination signal handler alike.
    #[tokio::test]
    async fn a_quit_while_the_startup_check_runs_goes_through() {
        let drained = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        let flag = drained.clone();
        let status = lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async move {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();
        assert_eq!(status.phase, ShutdownPhase::Draining);
        let status = loop {
            let status = lifecycle
                .shutdown_status("quit", ShutdownPurpose::Quit)
                .unwrap();
            if status.phase != ShutdownPhase::Draining {
                break status;
            }
            tokio::task::yield_now().await;
        };
        assert_eq!(status.phase, ShutdownPhase::Ready);
        // Servers are still asked to stop and awaited, as on any quit.
        assert!(drained.load(std::sync::atomic::Ordering::SeqCst));
        lifecycle
            .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
        assert!(lifecycle.shutdown_committed());

        // main.rs: SIGTERM/SIGINT run the same handshake under this id.
        let signalled = Arc::new(RuntimeLifecycle::default());
        signalled
            .prepare_shutdown_and_commit("core-termination-signal", ShutdownPurpose::Quit, async {
                Ok(())
            })
            .await
            .unwrap();
        assert!(signalled.shutdown_committed());

        // A move still waits for the check to finish.
        let pending = Arc::new(RuntimeLifecycle::default());
        assert_eq!(
            pending.begin_relocation("move").unwrap_err().blockers,
            [RuntimeBlocker::new(Kind::StartupCheckPending, "")]
        );
    }

    /// (d) One bad entry in __gdl_runtime_writers no longer fails the whole
    /// check. Files the launcher never writes are skipped, records it cannot
    /// read are kept in place and named, and a valid record is still adopted.
    #[tokio::test]
    async fn bad_writer_entries_are_kept_and_named_while_valid_records_are_adopted() {
        let root = tempfile::tempdir().unwrap();
        let writers = root.path().join("__gdl_runtime_writers");
        std::fs::create_dir_all(&writers).unwrap();
        // Left empty by a crash between the spawn intent and the PID write.
        let empty = writers.join("3f1c2b7e-7a51-4c55-9a0e-2d4b6f0c8e11.json");
        std::fs::write(&empty, b"").unwrap();
        let garbage = writers.join("8b0d7c3a-15e2-4f4b-a6a3-91e5c2d7f402.json");
        std::fs::write(&garbage, b"{\"version\":1,\"proc").unwrap();
        // Finder's folder metadata, never a record.
        std::fs::write(writers.join(".DS_Store"), b"\0\0\0\x01Bud1").unwrap();
        // A folder, even one named like a record.
        std::fs::create_dir(writers.join("folder.json")).unwrap();
        let live = writers.join("c9a4e5f0-2b1d-4c3e-8f7a-6d5b4c3a2e10.json");
        let record = ProcessRecord::legacy(4242, 1_700_000_000);
        std::fs::write(&live, encode_child_record(record).unwrap()).unwrap();
        let before = snapshot(&writers);

        let lifecycle = never_gone(root.path());
        let mut expected_unresolved = vec![unreadable(&empty), unreadable(&garbage)];
        expected_unresolved.sort_by(|a, b| a.name.cmp(&b.name));
        for run in 0..2 {
            lifecycle.reconcile_children().await.unwrap();
            assert_eq!(snapshot(&writers), before, "run {run} changed a file");
            {
                let state = lifecycle.state.lock().unwrap();
                assert_eq!(state.adopted.len(), 1, "run {run}");
                assert_eq!(state.adopted[&record], live.display().to_string());
            }
            lifecycle.reconciliation_finished(Ok(()));
            let refused = lifecycle
                .begin_relocation(&format!("move {run}"))
                .unwrap_err();
            let mut expected = vec![
                RuntimeBlocker::new(Kind::PreviousProcess, live.display().to_string())
                    .with_pid(4242),
            ];
            expected.extend(expected_unresolved.iter().cloned());
            assert_eq!(refused.blockers, expected, "run {run}");
        }

        // None of it keeps the user from quitting or updating.
        lifecycle
            .prepare_shutdown_and_commit("update", ShutdownPurpose::Update, async { Ok(()) })
            .await
            .unwrap();
        assert_eq!(snapshot(&writers), before);
    }

    /// The way out the move refusal gives: once the user deletes a record it
    /// named, Try again no longer lists it.
    #[tokio::test]
    async fn a_deleted_unreadable_record_clears_on_the_next_attempt() {
        let root = tempfile::tempdir().unwrap();
        let writers = root.path().join("__gdl_runtime_writers");
        std::fs::create_dir_all(&writers).unwrap();
        let empty = writers.join("3f1c2b7e-7a51-4c55-9a0e-2d4b6f0c8e11.json");
        std::fs::write(&empty, b"").unwrap();
        let lifecycle = never_gone(root.path());
        lifecycle.reconcile_children().await.unwrap();
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle.refresh_adopted();
        assert_eq!(
            lifecycle.begin_relocation("move 1").unwrap_err().blockers,
            [unreadable(&empty)]
        );
        std::fs::remove_file(&empty).unwrap();
        // What /prepare-relocation runs: refresh, then the refuse-first lease.
        lifecycle.refresh_adopted();
        lifecycle.begin_relocation("move 2").unwrap();
        lifecycle.cancel_relocation("move 2").unwrap();
    }

    /// (e) A record removed while the check reads it is gone, whether it was
    /// removed after the folder was listed or between the two reads.
    #[tokio::test]
    async fn a_record_removed_during_the_check_is_gone() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = never_gone(root.path());
        let missing = root
            .path()
            .join("__gdl_runtime_writers")
            .join("listed-then-removed.json");
        lifecycle.reconcile_child(&missing).await.unwrap();

        /// Confirms the process gone and, like a second launcher cleaning
        /// up, removes the record before it is read again.
        #[derive(Debug)]
        struct RemovedOnCheck(PathBuf);
        impl ProcessAbsenceObserver for RemovedOnCheck {
            fn confirmed_absent(&self, _pid: u32, _start: u64) -> bool {
                std::fs::remove_file(&self.0).unwrap();
                true
            }
        }
        let writers = root.path().join("__gdl_runtime_writers");
        std::fs::create_dir_all(&writers).unwrap();
        let record = writers.join("removed-between-reads.json");
        std::fs::write(
            &record,
            encode_child_record(ProcessRecord::legacy(4242, 10)).unwrap(),
        )
        .unwrap();
        let racing = Arc::new(RuntimeLifecycle {
            process_observer: Arc::new(RemovedOnCheck(record.clone())),
            ..RuntimeLifecycle::with_runtime_path(root.path())
        });
        racing.reconcile_children().await.unwrap();
        for lifecycle in [&lifecycle, &racing] {
            let state = lifecycle.state.lock().unwrap();
            assert!(state.unresolved.is_empty(), "{:?}", state.unresolved);
            assert!(state.adopted.is_empty());
        }
    }

    /// (e) The startup reconcile runs beside the startup Java scan. A probe
    /// this core is spawning right then has an empty intent on disk; it is
    /// this session's own work, so it is neither evidence nor touched, and
    /// the probe still records its PID afterwards.
    #[tokio::test]
    async fn this_cores_own_spawn_intent_is_not_earlier_session_evidence() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        let mut probe = lifecycle.admit("java probe").unwrap();
        probe.external_child().unwrap();
        let intent = probe.record.clone().unwrap();
        assert!(std::fs::read(&intent).unwrap().is_empty());

        lifecycle.reconcile_children().await.unwrap();
        // Read before asserting: a failed assert must not poison the lock
        // the probe's own drop takes.
        let (unresolved, adopted) = {
            let state = lifecycle.state.lock().unwrap();
            (state.unresolved.clone(), state.adopted.clone())
        };
        assert!(unresolved.is_empty(), "{unresolved:?}");
        assert!(adopted.is_empty(), "{adopted:?}");
        assert!(std::fs::read(&intent).unwrap().is_empty());

        let pid = std::process::id();
        probe.record_child_pid(Some(pid)).unwrap();
        let recorded = decode_child_record(&std::fs::read(&intent).unwrap()).unwrap();
        assert_eq!(recorded.pid, pid);
        probe.finish();
        assert!(!intent.exists());
        assert!(lifecycle.state.lock().unwrap().own_records.is_empty());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle.begin_relocation("move").unwrap();
    }

    /// (f) One bad instance or server marker no longer fails the whole check:
    /// the folders after it are still reconciled, so running games are still
    /// adopted, and the bad markers are kept in place and named.
    #[tokio::test]
    async fn bad_markers_are_kept_and_named_while_valid_ones_are_adopted() {
        const MARKER: &str = ".gdl_instance.pid";
        let root = tempfile::tempdir().unwrap();
        let instances = root.path().join("instances");
        let folder = |name: &str| {
            let folder = instances.join(name);
            std::fs::create_dir_all(&folder).unwrap();
            folder
        };
        // Truncated by a power loss right after a launch.
        let empty = folder("Empty").join(MARKER);
        std::fs::write(&empty, b"").unwrap();
        let locked = folder("Locked").join(MARKER);
        std::fs::write(&locked, b"4243\n1700000000").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        // Where permissions do not stop this user (root, Windows), a folder
        // with the marker's name cannot be read as one either.
        if std::fs::read(&locked).is_ok() {
            std::fs::remove_file(&locked).unwrap();
            std::fs::create_dir(&locked).unwrap();
        }
        let running = folder("Running");
        std::fs::write(running.join(MARKER), b"4242\n1700000000").unwrap();
        // A stray file next to the instance folders is not an instance.
        std::fs::write(instances.join("notes.txt"), b"").unwrap();

        let lifecycle = never_gone(&root.path().join("runtime"));
        lifecycle.reconcile_root(&instances, MARKER).await.unwrap();
        lifecycle.reconciliation_finished(Ok(()));
        let record = ProcessRecord::legacy(4242, 1_700_000_000);
        {
            let state = lifecycle.state.lock().unwrap();
            assert_eq!(state.adopted.len(), 1);
            assert_eq!(state.adopted[&record], running.display().to_string());
            assert_eq!(
                state.unresolved.iter().cloned().collect::<Vec<_>>(),
                [empty.clone(), locked.clone()]
            );
        }
        assert_eq!(
            lifecycle.begin_relocation("move").unwrap_err().blockers,
            [
                RuntimeBlocker::new(Kind::PreviousProcess, running.display().to_string())
                    .with_pid(4242),
                unreadable(&empty),
                unreadable(&locked),
            ]
        );
        // Kept, not renamed: a restart sees them again.
        assert!(std::fs::read(&empty).unwrap().is_empty());
        assert!(locked.exists());
        // The running game's durable copy survives its marker being cleaned up.
        assert!(lifecycle.reconciled_record_path(record).unwrap().is_file());

        lifecycle
            .prepare_shutdown_and_commit("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .await
            .unwrap();
        #[cfg(unix)]
        if locked.is_file() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    /// Some network and FUSE file systems leave an entry's type out of the
    /// listing, and the stat behind it can fail. The move refusal says to
    /// delete what it lists, so it must name the marker inside an instance
    /// folder, never the folder with the user's worlds in it.
    #[tokio::test]
    async fn a_folder_whose_type_cannot_be_read_is_never_listed_only_its_marker() {
        const MARKER: &str = ".gdl_instance.pid";
        let root = tempfile::tempdir().unwrap();
        let instances = root.path().join("instances");
        let folder = |name: &str| {
            let folder = instances.join(name);
            std::fs::create_dir_all(&folder).unwrap();
            folder
        };
        let crashed = folder("Survival World");
        std::fs::write(crashed.join(MARKER), b"").unwrap();
        let idle = folder("Idle");
        let running = folder("Running");
        std::fs::write(running.join(MARKER), b"4242\n1700000000").unwrap();
        let stray = instances.join("notes.txt");
        std::fs::write(&stray, b"").unwrap();

        let lifecycle = never_gone(&root.path().join("runtime"));
        for entry in [&crashed, &idle, &running, &stray] {
            let unknown = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
            lifecycle
                .reconcile_folder(entry, Err(unknown), MARKER)
                .await;
        }
        let state = lifecycle.state.lock().unwrap();
        assert_eq!(
            state.unresolved.iter().cloned().collect::<Vec<_>>(),
            [crashed.join(MARKER)]
        );
        assert_eq!(
            state.adopted[&ProcessRecord::legacy(4242, 1_700_000_000)],
            running.display().to_string()
        );
        assert_eq!(state.adopted.len(), 1);
    }

    /// Try again checks each listed file again instead of only whether it
    /// still exists: one that reads again (an antivirus or sync lock lifted)
    /// and names a process that has ended clears, while one naming a
    /// process that may still run, or still unreadable, stays listed.
    #[tokio::test]
    async fn try_again_clears_a_listed_file_that_now_names_an_ended_process() {
        /// Only the given PID still runs.
        #[derive(Debug)]
        struct OnlyRunning(u32);
        impl ProcessAbsenceObserver for OnlyRunning {
            fn confirmed_absent(&self, pid: u32, _start: u64) -> bool {
                pid != self.0
            }
        }
        const MARKER: &str = ".gdl_instance.pid";
        let root = tempfile::tempdir().unwrap();
        let instances = root.path().join("instances");
        let writers = root.path().join("__gdl_runtime_writers");
        std::fs::create_dir_all(&writers).unwrap();
        let mut markers = vec![];
        for name in ["Ended", "Live", "Still empty"] {
            let folder = instances.join(name);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join(MARKER), b"").unwrap();
            markers.push(folder.join(MARKER));
        }
        let record = writers.join("3f1c2b7e-7a51-4c55-9a0e-2d4b6f0c8e11.json");
        std::fs::write(&record, b"").unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle {
            process_observer: Arc::new(OnlyRunning(4242)),
            ..RuntimeLifecycle::with_runtime_path(root.path())
        });
        lifecycle.reconcile_children().await.unwrap();
        lifecycle.reconcile_root(&instances, MARKER).await.unwrap();
        lifecycle.reconciliation_finished(Ok(()));
        assert_eq!(lifecycle.state.lock().unwrap().unresolved.len(), 4);

        std::fs::write(&markers[0], b"4241\n1700000000").unwrap();
        std::fs::write(&markers[1], b"4242\n1700000000").unwrap();
        std::fs::write(
            &record,
            encode_child_record(ProcessRecord::legacy(4243, 1_700_000_000)).unwrap(),
        )
        .unwrap();
        lifecycle.refresh_adopted();
        assert_eq!(
            lifecycle.begin_relocation("move 1").unwrap_err().blockers,
            [unreadable(&markers[1]), unreadable(&markers[2])]
        );
        // Checking changes nothing on disk.
        assert_eq!(std::fs::read(&markers[0]).unwrap(), b"4241\n1700000000");
        assert!(record.is_file());
    }
}

/// Server and game markers from an earlier session whose PID now belongs to
/// another process are retired, as 2.0.40 did, instead of being kept as a
/// live owner for as long as that unrelated process runs.
#[cfg(test)]
mod stale_marker_tests {
    use super::*;
    use IdentityStatus::{GoneOrReused, Same, Unknown};
    use RuntimeBlockerKind as Kind;
    use orphan_pid::{LiveProc, ProcessIdentity, ProcessObservation};

    const WINDOWS_OR_MACOS: bool = true;
    const LINUX: bool = false;
    const MARKER: &str = ".gdl_server.pid";

    fn present(
        is_java: bool,
        start_time: u64,
        identity: Option<ProcessIdentity>,
    ) -> ProcessObservation {
        ProcessObservation::Present {
            process: LiveProc {
                is_java,
                start_time,
            },
            identity,
        }
    }
    fn ticks(start_ticks: u64) -> ProcessIdentity {
        ProcessIdentity::LinuxBootTicks {
            boot_id: uuid::Uuid::parse_str("cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80").unwrap(),
            start_ticks,
        }
    }
    /// A two-line marker as 2.0.40 wrote it: PID and epoch start, no
    /// boot/ticks identity.
    fn legacy() -> ProcessRecord {
        ProcessRecord::legacy(4242, 1_700_000_000)
    }
    /// What the native probe says when sysinfo did not list the PID.
    fn classify(
        record: ProcessRecord,
        observed: ProcessObservation,
        require_java: bool,
        platform: bool,
        native_absent: bool,
    ) -> IdentityStatus {
        classify_observed(record, observed, require_java, platform, || native_absent)
    }

    #[test]
    fn windows_and_macos_retire_a_reused_pid_and_keep_the_same_process() {
        let record = legacy();
        // A PID Windows handed to svchost or chrome after a reboot.
        assert_eq!(
            classify(
                record,
                present(false, 1_700_000_000, None),
                true,
                WINDOWS_OR_MACOS,
                false
            ),
            GoneOrReused
        );
        // Another JVM (a game, another launcher's server) got the PID.
        assert_eq!(
            classify(
                record,
                present(true, 1_700_500_000, None),
                true,
                WINDOWS_OR_MACOS,
                false
            ),
            GoneOrReused
        );
        // The server JVM itself, orphaned when 2.0.40 was closed.
        assert_eq!(
            classify(
                record,
                present(true, 1_700_000_001, None),
                true,
                WINDOWS_OR_MACOS,
                false
            ),
            Same
        );
        // sysinfo did not list it but the native probe still sees it: a
        // partial listing is not exit proof.
        assert_eq!(
            classify(
                record,
                ProcessObservation::Absent,
                true,
                WINDOWS_OR_MACOS,
                false
            ),
            Unknown
        );
        assert_eq!(
            classify(
                record,
                ProcessObservation::Absent,
                true,
                WINDOWS_OR_MACOS,
                true
            ),
            GoneOrReused
        );
    }

    #[test]
    fn linux_retires_a_reused_pid_but_keeps_a_legacy_epoch_mismatch_on_java() {
        let record = legacy();
        assert_eq!(
            classify(
                record,
                present(false, 1_700_000_000, Some(ticks(7))),
                true,
                LINUX,
                false
            ),
            GoneOrReused
        );
        // A clock step moves the epoch Linux reports (M23): unconfirmed.
        assert_eq!(
            classify(
                record,
                present(true, 1_700_500_000, Some(ticks(7))),
                true,
                LINUX,
                false
            ),
            Unknown
        );
        let current = ProcessRecord {
            identity: Some(ticks(7)),
            ..record
        };
        assert_eq!(
            classify(
                current,
                present(true, 1_700_500_000, Some(ticks(8))),
                true,
                LINUX,
                false
            ),
            GoneOrReused
        );
        assert_eq!(
            classify(
                current,
                present(true, 1_700_500_000, Some(ticks(7))),
                true,
                LINUX,
                false
            ),
            Same
        );
        assert_eq!(
            classify(record, ProcessObservation::Absent, true, LINUX, false),
            Unknown
        );
    }

    #[test]
    fn a_live_hook_child_is_kept_although_it_is_not_java() {
        // Pre-launch and post-exit hooks run scripts, not Java.
        for platform in [WINDOWS_OR_MACOS, LINUX] {
            let identity = (!platform).then(|| ticks(7));
            let record = ProcessRecord {
                identity,
                ..legacy()
            };
            assert_eq!(
                classify(
                    record,
                    present(false, 1_700_000_000, identity),
                    false,
                    platform,
                    false
                ),
                Same
            );
            assert_eq!(
                classify(
                    record,
                    present(false, 1_700_000_000, identity),
                    true,
                    platform,
                    false
                ),
                GoneOrReused,
                "only a marker requires Java"
            );
        }
    }

    /// Injected observations for each PID; everything else is absent.
    #[derive(Debug, Default)]
    struct Processes {
        observed: Mutex<HashMap<u32, ProcessObservation>>,
        /// The program name the process table gives for a PID.
        programs: Mutex<HashMap<u32, &'static str>>,
    }
    impl Processes {
        fn set(&self, pid: u32, observed: ProcessObservation) {
            self.observed.lock().unwrap().insert(pid, observed);
        }
        fn name(&self, pid: u32, program: &'static str) {
            self.programs.lock().unwrap().insert(pid, program);
        }
    }
    impl ProcessAbsenceObserver for Processes {
        fn confirmed_absent(&self, _: u32, _: u64) -> bool {
            panic!("classification goes through status")
        }
        fn observation(&self, record: ProcessRecord) -> ProcessObservation {
            self.observed
                .lock()
                .unwrap()
                .get(&record.pid)
                .copied()
                .unwrap_or(ProcessObservation::Absent)
        }
        fn status(&self, record: ProcessRecord, require_java: bool) -> IdentityStatus {
            classify(
                record,
                self.observation(record),
                require_java,
                orphan_pid::EPOCH_START_TIMES_ARE_STABLE,
                true,
            )
        }
        /// Like the real observer, a name only comes with a listed process.
        fn inspect(
            &self,
            record: ProcessRecord,
            require_java: bool,
        ) -> (IdentityStatus, Option<String>) {
            let listed = matches!(self.observation(record), ProcessObservation::Present { .. });
            let program = self.programs.lock().unwrap().get(&record.pid).copied();
            (
                self.status(record, require_java),
                program.filter(|_| listed).map(str::to_owned),
            )
        }
    }
    fn lifecycle(root: &Path, processes: &Arc<Processes>) -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle {
            process_observer: processes.clone(),
            ..RuntimeLifecycle::with_runtime_path(root)
        });
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    /// The live process a marker for `record` describes on this platform.
    fn its_jvm(record: ProcessRecord) -> ProcessObservation {
        present(true, record.start_time.unwrap(), record.identity)
    }
    fn server_with_marker(servers: &Path, name: &str, record: ProcessRecord) -> PathBuf {
        let folder = servers.join(name);
        std::fs::create_dir_all(&folder).unwrap();
        let mut text = format!("{}\n{}", record.pid, record.start_time.unwrap());
        if let Some(ProcessIdentity::LinuxBootTicks {
            boot_id,
            start_ticks,
        }) = record.identity
        {
            text.push_str(&format!("\nlinux-boot-ticks-v1 {boot_id} {start_ticks}"));
        }
        std::fs::write(folder.join(MARKER), text).unwrap();
        folder
    }
    fn copies(root: &Path) -> Vec<String> {
        match std::fs::read_dir(root.join("__gdl_runtime_writers")) {
            Ok(entries) => entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => vec![],
        }
    }

    /// A marker whose PID a process other than Java now holds is never
    /// adopted: no blocker, no copy kept for the next start.
    #[tokio::test]
    async fn a_server_marker_on_a_reused_pid_is_not_adopted() {
        let root = tempfile::tempdir().unwrap();
        let servers = root.path().join("servers");
        let processes = Arc::new(Processes::default());
        let record = legacy();
        server_with_marker(&servers, "Survival", record);
        processes.set(record.pid, present(false, 1_700_000_000, record.identity));
        let lifecycle = lifecycle(root.path(), &processes);
        lifecycle.reconcile_root(&servers, MARKER).await.unwrap();
        assert!(lifecycle.state.lock().unwrap().adopted.is_empty());
        assert!(copies(root.path()).is_empty());
        lifecycle.begin_relocation("move").unwrap();
    }

    /// The copy kept of a marker names a JVM too, while other child records
    /// name hooks and probes, which are not Java.
    #[tokio::test]
    async fn a_marker_copy_requires_java_and_a_hook_record_does_not() {
        let root = tempfile::tempdir().unwrap();
        let writers = root.path().join("__gdl_runtime_writers");
        std::fs::create_dir_all(&writers).unwrap();
        let processes = Arc::new(Processes::default());
        let hook = ProcessRecord::legacy(5000, 1_700_000_000);
        let copied = ProcessRecord::legacy(5001, 1_700_000_000);
        let hook_file = writers.join("3f1c2b7e-7a51-4c55-9a0e-2d4b6f0c8e11.json");
        let copy_file = writers.join("reconciled-5001-epoch-1700000000.json");
        std::fs::write(&hook_file, encode_child_record(hook).unwrap()).unwrap();
        std::fs::write(&copy_file, encode_child_record(copied).unwrap()).unwrap();
        for pid in [hook.pid, copied.pid] {
            processes.set(pid, present(false, 1_700_000_000, None));
        }
        let lifecycle = lifecycle(root.path(), &processes);
        lifecycle.reconcile_children().await.unwrap();
        assert!(hook_file.exists());
        assert!(!copy_file.exists());
        let state = lifecycle.state.lock().unwrap();
        assert_eq!(state.adopted.keys().copied().collect::<Vec<_>>(), [hook]);
    }

    /// A server JVM left running by an earlier session is the server's to
    /// stop, not a reason to refuse Quit or Update. It still refuses a move
    /// of the runtime folder until it is stopped or its marker cleared, and
    /// releasing the marker forgets it together with its copy.
    #[tokio::test]
    async fn a_recovered_server_refuses_only_a_move_until_its_marker_is_released() {
        let root = tempfile::tempdir().unwrap();
        let servers = root.path().join("servers");
        let processes = Arc::new(Processes::default());
        let record = ProcessRecord {
            identity: (!orphan_pid::EPOCH_START_TIMES_ARE_STABLE).then(|| ticks(7)),
            ..legacy()
        };
        let folder = server_with_marker(&servers, "Survival", record);
        processes.set(record.pid, its_jvm(record));
        let lifecycle = lifecycle(root.path(), &processes);
        lifecycle.reconcile_root(&servers, MARKER).await.unwrap();
        assert_eq!(copies(root.path()).len(), 1);
        let blocker =
            RuntimeBlocker::new(Kind::PreviousProcess, folder.display().to_string()).with_pid(4242);

        for purpose in [ShutdownPurpose::Quit, ShutdownPurpose::Update] {
            let request = format!("{purpose:?}");
            lifecycle.begin_shutdown(&request, purpose).unwrap();
            let status = lifecycle.settle_shutdown(&request, Ok(())).unwrap();
            assert_eq!(status.phase, ShutdownPhase::Ready, "{purpose:?}");
            assert!(status.blockers.is_empty(), "{:?}", status.blockers);
            lifecycle.state.lock().unwrap().shutdown = None;
        }
        lifecycle.refresh_adopted();
        assert_eq!(
            lifecycle.begin_relocation("move 1").unwrap_err().blockers,
            [blocker]
        );

        // Stopped from the server page: the server code removes the
        // marker, then releases it here.
        std::fs::remove_file(folder.join(MARKER)).unwrap();
        lifecycle.release_marker(
            &folder.join(MARKER),
            Some(ProcessRecord::legacy(4242, 1_700_000_000)),
        );
        assert!(copies(root.path()).is_empty());
        assert!(lifecycle.state.lock().unwrap().adopted.is_empty());
        lifecycle.begin_relocation("move 2").unwrap();
    }

    /// An unreadable marker the user cleared from the server page no longer
    /// refuses a move.
    #[tokio::test]
    async fn a_cleared_unreadable_marker_no_longer_refuses_a_move() {
        let root = tempfile::tempdir().unwrap();
        let servers = root.path().join("servers");
        let folder = servers.join("Survival");
        std::fs::create_dir_all(&folder).unwrap();
        let marker = folder.join(MARKER);
        std::fs::write(&marker, b"\x004242\n17000").unwrap();
        let lifecycle = lifecycle(root.path(), &Arc::new(Processes::default()));
        lifecycle.reconcile_root(&servers, MARKER).await.unwrap();
        assert_eq!(
            lifecycle.begin_relocation("move 1").unwrap_err().blockers,
            [RuntimeBlocker::new(
                Kind::UnreadableRecord,
                marker.display().to_string()
            )]
        );
        std::fs::remove_file(&marker).unwrap();
        lifecycle.release_marker(&marker, None);
        lifecycle.begin_relocation("move 2").unwrap();
    }

    /// The record of a JVM on this platform: Linux records carry the
    /// boot/ticks identity this build writes.
    fn jvm_record(pid: u32) -> ProcessRecord {
        ProcessRecord {
            pid,
            identity: (!orphan_pid::EPOCH_START_TIMES_ARE_STABLE).then(|| ticks(u64::from(pid))),
            ..legacy()
        }
    }
    /// The same PID, start and identity, held by a program that is not Java.
    fn not_java(record: ProcessRecord) -> ProcessObservation {
        present(false, record.start_time.unwrap(), record.identity)
    }
    fn refused_pids(lifecycle: &RuntimeLifecycle) -> Vec<(Kind, Option<u32>)> {
        lifecycle.refresh_adopted();
        lifecycle
            .relocation_blockers("probe")
            .into_iter()
            .map(|blocker| (blocker.kind, blocker.pid))
            .collect()
    }

    /// Try again stops refusing a move once a program other than Java holds
    /// the PID of a recorded game or server, wherever the record came from:
    /// the marker, the instance check that published its game, or the copy
    /// a later start reads. 2.0.40 removed such a marker on the same verdict.
    #[tokio::test]
    async fn try_again_retires_a_record_whose_pid_a_program_other_than_java_holds() {
        let root = tempfile::tempdir().unwrap();
        let servers = root.path().join("servers");
        let processes = Arc::new(Processes::default());
        let server = jvm_record(4242);
        let game = jvm_record(4300);
        server_with_marker(&servers, "Survival", server);
        for record in [server, game] {
            processes.set(record.pid, its_jvm(record));
        }
        let lifecycle = lifecycle(root.path(), &processes);
        lifecycle.reconcile_root(&servers, MARKER).await.unwrap();
        lifecycle.adopted_record_published(game);
        assert_eq!(
            refused_pids(&lifecycle),
            [
                (Kind::PreviousGame, Some(4300)),
                (Kind::PreviousProcess, Some(4242))
            ]
        );
        assert_eq!(copies(root.path()).len(), 1);

        // Both JVMs ended and Windows gave their PIDs to Discord and
        // explorer.exe: only the Java check tells them apart.
        for record in [server, game] {
            processes.set(record.pid, not_java(record));
        }
        assert!(refused_pids(&lifecycle).is_empty());
        lifecycle.begin_relocation("move").unwrap();

        // The next start drops the copy kept of the server marker as well.
        let next = self::lifecycle(root.path(), &processes);
        next.reconcile_children().await.unwrap();
        assert!(copies(root.path()).is_empty());
        assert!(refused_pids(&next).is_empty());
    }

    /// A JVM that may still be the recorded game or server keeps refusing
    /// a move, also on Try again: one whose start matches, one whose start
    /// a one-line marker does not record, and one this look cannot read. So
    /// does a JVM whose start differs on Linux, where a clock step moves the
    /// start a legacy marker recorded; Windows and macOS keep creation times
    /// stable, so there it is another JVM that reused the PID.
    #[tokio::test]
    async fn a_java_process_with_a_matching_or_unknown_start_keeps_refusing_a_move() {
        let root = tempfile::tempdir().unwrap();
        let servers = root.path().join("servers");
        let processes = Arc::new(Processes::default());
        let matching = jvm_record(4242);
        server_with_marker(&servers, "Survival", matching);
        processes.set(matching.pid, its_jvm(matching));
        let one_line = servers.join("Creative");
        std::fs::create_dir_all(&one_line).unwrap();
        std::fs::write(one_line.join(MARKER), "4243").unwrap();
        processes.set(4243, present(true, 1_700_000_500, None));
        let unreadable = jvm_record(4244);
        server_with_marker(&servers, "Lobby", unreadable);
        processes.set(unreadable.pid, ProcessObservation::Unknown);
        let legacy_later = ProcessRecord {
            pid: 4245,
            ..legacy()
        };
        server_with_marker(&servers, "Hub", legacy_later);
        processes.set(
            legacy_later.pid,
            present(
                true,
                1_700_500_000,
                (!orphan_pid::EPOCH_START_TIMES_ARE_STABLE).then(|| ticks(5)),
            ),
        );

        let lifecycle = lifecycle(root.path(), &processes);
        lifecycle.reconcile_root(&servers, MARKER).await.unwrap();
        let mut expected = vec![4242, 4243, 4244];
        if !orphan_pid::EPOCH_START_TIMES_ARE_STABLE {
            expected.push(4245);
        }
        for _ in 0..2 {
            let mut pids: Vec<_> = refused_pids(&lifecycle)
                .into_iter()
                .map(|(kind, pid)| {
                    assert_eq!(kind, Kind::PreviousProcess);
                    pid.unwrap()
                })
                .collect();
            pids.sort();
            assert_eq!(pids, expected);
        }
    }

    /// A refused move names the program that holds each PID, as the look
    /// that kept the record read it, and never a name it no longer reads.
    #[tokio::test]
    async fn a_refused_move_names_the_program_that_holds_the_pid() {
        let root = tempfile::tempdir().unwrap();
        let servers = root.path().join("servers");
        let processes = Arc::new(Processes::default());
        let record = jvm_record(4242);
        let folder = server_with_marker(&servers, "Survival", record);
        processes.set(record.pid, its_jvm(record));
        processes.name(record.pid, "javaw.exe");
        let lifecycle = lifecycle(root.path(), &processes);
        lifecycle.reconcile_root(&servers, MARKER).await.unwrap();
        lifecycle.refresh_adopted();
        let blocker =
            RuntimeBlocker::new(Kind::PreviousProcess, folder.display().to_string()).with_pid(4242);
        let named = blocker.clone().with_program(Some("javaw.exe".into()));
        let refused = lifecycle.begin_relocation("move 1").unwrap_err();
        assert_eq!(refused.blockers, [named.clone()]);
        let json = serde_json::to_value(&named).unwrap();
        assert_eq!(json["pid"], 4242);
        assert_eq!(json["program"], "javaw.exe");
        assert!(refused.to_string().contains("(PID 4242, javaw.exe)"));

        // The process table cannot be read for it now: still refused, by
        // PID alone.
        processes.set(record.pid, ProcessObservation::Unknown);
        lifecycle.refresh_adopted();
        assert_eq!(
            lifecycle.begin_relocation("move 2").unwrap_err().blockers,
            [blocker.clone()]
        );
        assert!(
            serde_json::to_value(&blocker)
                .unwrap()
                .get("program")
                .is_none()
        );
    }
}

/// What a quit, restart or update does to this core's own work besides
/// games and servers: a hook the user set up runs to its end, lookups and
/// panicked workers never keep the launcher from closing.
#[cfg(test)]
mod owned_activities_during_quit_tests {
    use super::*;
    use std::time::Duration;

    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    /// Polls like Electron does until the request leaves Draining.
    async fn settled(lifecycle: &RuntimeLifecycle, request: &str) -> ShutdownStatus {
        tokio::time::timeout(Duration::from_secs(120), async {
            loop {
                let status = lifecycle
                    .shutdown_status(request, ShutdownPurpose::Quit)
                    .unwrap();
                if status.phase != ShutdownPhase::Draining {
                    return status;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("shutdown never settled")
    }
    fn hook_blocker() -> RuntimeBlocker {
        RuntimeBlocker::new(RuntimeBlockerKind::Hook, "temporary child")
    }

    #[test]
    fn a_quit_leaves_a_hook_running_and_names_it_as_one() {
        let lifecycle = ready();
        let hook = lifecycle
            .admit_owned("temporary child", ActivityPolicy::Hook)
            .unwrap();
        let installer = lifecycle
            .admit_owned("instance 3 preparation", ActivityPolicy::Installer)
            .unwrap();
        assert_eq!(
            lifecycle.relocation_blockers("move"),
            [
                hook_blocker(),
                RuntimeBlocker::new(RuntimeBlockerKind::Install, "instance 3 preparation")
            ]
        );
        assert!(
            lifecycle
                .begin_shutdown("quit", ShutdownPurpose::Quit)
                .unwrap()
        );
        assert!(
            !hook.cancellation().is_cancelled(),
            "the user's script is left to finish"
        );
        assert!(
            installer.cancellation().is_cancelled(),
            "other work is still cancelled as the quit begins"
        );
        assert_eq!(
            lifecycle
                .shutdown_status("quit", ShutdownPurpose::Quit)
                .unwrap()
                .blockers,
            [
                hook_blocker(),
                RuntimeBlocker::new(RuntimeBlockerKind::Install, "instance 3 preparation")
            ]
        );
        installer.finish();
        hook.finish();
        assert_eq!(
            lifecycle.settle_shutdown("quit", Ok(())).unwrap().phase,
            ShutdownPhase::Ready
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_hook_longer_than_the_budget_is_named_and_the_retry_finishes_once_it_ends() {
        let lifecycle = ready();
        let hook = lifecycle
            .admit_owned("temporary child", ActivityPolicy::Hook)
            .unwrap();
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let status = settled(&lifecycle, "quit").await;
        assert_eq!(status.phase, ShutdownPhase::Unresolved);
        assert_eq!(status.blockers, [hook_blocker()]);
        assert!(!hook.cancellation().is_cancelled());
        // Try again (or Wait) re-joins the same request.
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::time::sleep(Duration::from_secs(4)).await;
        hook.finish();
        assert_eq!(
            settled(&lifecycle, "quit").await.phase,
            ShutdownPhase::Ready
        );
        lifecycle
            .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
    }

    /// The real child path: a post-exit backup script that is still running
    /// when the user quits finishes its work and reports success, and the
    /// quit goes through right after it.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_running_hook_finishes_its_work_through_a_quit() {
        let lifecycle = ready();
        let folder = tempfile::tempdir().unwrap();
        let mut command = tokio::process::Command::new("sh");
        command
            .args(["-c", "sleep 1; echo backed up > done.txt; echo finished"])
            .current_dir(folder.path());
        let hook = tokio::spawn({
            let lifecycle = lifecycle.clone();
            async move {
                tracked_output_reserved(&lifecycle, &mut command, ActivityPolicy::Hook, None).await
            }
        });
        tokio::time::timeout(Duration::from_secs(10), async {
            while !lifecycle
                .state
                .lock()
                .unwrap()
                .activities
                .values()
                .any(|entry| {
                    entry.policy == ActivityPolicy::Hook && entry.phase == ActivityPhase::Running
                })
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the hook started");
        let status = lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert_eq!(status.blockers, [hook_blocker()]);
        let output = hook
            .await
            .unwrap()
            .expect("the quit does not stop the hook");
        assert!(output.status.success());
        assert_eq!(output.stdout, b"finished\n");
        assert_eq!(
            std::fs::read_to_string(folder.path().join("done.txt")).unwrap(),
            "backed up\n"
        );
        assert_eq!(
            settled(&lifecycle, "quit").await.phase,
            ShutdownPhase::Ready
        );
        lifecycle
            .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
    }

    /// A hook that pauses or asks for input (`pause`, `set /p`, `read`) gets
    /// an empty input at once instead of waiting out its 300 s timeout on
    /// the core's own stdin, which the launcher never writes to or closes.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_hook_that_reads_input_gets_none_at_once() {
        let lifecycle = ready();
        // Stands in for the inherited stdin: a pipe nobody writes to or
        // closes. (A piped stdin would not do: the wait closes its handle.)
        let (input, _held_open) = std::io::pipe().unwrap();
        let mut command = tokio::process::Command::new("sh");
        command
            .args(["-c", "read line; echo \"read: $?\""])
            .stdin(input);
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            tracked_output_reserved(&lifecycle, &mut command, ActivityPolicy::Hook, None),
        )
        .await
        .expect("the hook does not wait for input")
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"read: 1\n");
        assert!(lifecycle.state.lock().unwrap().activities.is_empty());
    }

    #[tokio::test]
    async fn a_lookup_is_dropped_once_cancelled_and_never_started_after() {
        let cancel = tokio_util::sync::CancellationToken::new();
        let lookup = tokio::spawn({
            let cancel = cancel.clone();
            async move { lookup_or_cancel(&cancel, std::future::pending::<Result<()>>()).await }
        });
        tokio::task::yield_now().await;
        assert!(!lookup.is_finished());
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(5), lookup)
            .await
            .expect("a cancelled lookup ends at once")
            .unwrap();
        assert!(result.is_err());
        let started = std::sync::atomic::AtomicBool::new(false);
        let result = lookup_or_cancel(&cancel, async {
            started.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert!(result.is_err());
        assert!(!started.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(
            lookup_or_cancel(&tokio_util::sync::CancellationToken::new(), async { Ok(7) })
                .await
                .unwrap(),
            7
        );
    }

    #[tokio::test]
    async fn a_worker_that_panics_without_starting_a_program_does_not_block_quit_or_move() {
        let lifecycle = ready();
        let activity = lifecycle
            .admit_owned("panicking installer", ActivityPolicy::Installer)
            .unwrap();
        let worker = spawn_retained(activity, move |_| {
            Box::pin(async move {
                if lifecycle_fixture_panics() {
                    panic!("fixture worker panic");
                }
            })
        });
        assert!(worker.await.unwrap_err().is_panic(), "the panic is kept");
        assert!(lifecycle.state.lock().unwrap().activities.is_empty());
        lifecycle.begin_relocation("move").unwrap();
        lifecycle.cancel_relocation("move").unwrap();

        let activity = lifecycle
            .admit_owned(
                "borrowed create/import instance setup",
                ActivityPolicy::Installer,
            )
            .unwrap();
        let requester = tokio::spawn(run_borrowed_owned(activity, async {
            if lifecycle_fixture_panics() {
                panic!("fixture setup panic");
            }
        }));
        assert!(requester.await.unwrap_err().is_panic());
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        assert_eq!(
            settled(&lifecycle, "quit").await.phase,
            ShutdownPhase::Ready
        );
    }

    #[tokio::test]
    async fn a_worker_that_panics_after_starting_a_program_stays_unconfirmed() {
        for already_unknown in [false, true] {
            let lifecycle = ready();
            let activity = lifecycle
                .admit_owned("server run", ActivityPolicy::CoreWork)
                .unwrap();
            let worker = spawn_retained(activity, move |slot| {
                Box::pin(async move {
                    let activity = slot.as_mut().unwrap();
                    if already_unknown {
                        activity.unknown("wait failed".into());
                    } else {
                        activity.external_child().unwrap();
                    }
                    if lifecycle_fixture_panics() {
                        panic!("fixture worker panic");
                    }
                })
            });
            assert!(worker.await.unwrap_err().is_panic());
            assert_eq!(
                lifecycle.relocation_blockers("move"),
                [RuntimeBlocker::new(
                    RuntimeBlockerKind::Unconfirmed,
                    "server run"
                )],
                "a program it started may still run"
            );
        }
    }

    /// Keeps the fixtures' panics reachable for the compiler.
    fn lifecycle_fixture_panics() -> bool {
        true
    }
}
