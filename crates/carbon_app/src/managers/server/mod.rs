use self::local::LocalServerProvider;
use self::provider::{ServerHandle, ServerProvider};
use super::ManagerRef;
use crate::api::keys::server::*;
use crate::domain::server::{
    self, ServerAddon, ServerDetails, ServerGroupId, ServerGroupMoveTarget, ServerId,
    ServerListEntry, ServerLogId, ServerModpackInfo, ServerMoveTarget, ServerSettingsUpdate,
    ServerState, ServerType,
};
use crate::domain::vtask::VisualTaskId;
use crate::managers::minecraft::modrinth::secure_path_join;
use crate::managers::orphan_pid;
use crate::managers::runtime_lifecycle::ShutdownDrain;
use anyhow::{Context, anyhow, bail};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::server::{self as server_repo, IndexShift, ServerPatch};
use carbon_rt_path::ServerPath;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tracing::{error, info, warn};
use unicode_segmentation::UnicodeSegmentation;

mod addons;
pub mod jars;
pub mod local;
pub mod modloader_install;
pub mod modloader_launch;
pub mod modpack;
pub mod properties;
pub mod provider;
pub(crate) mod world;

const MAX_PATH: usize = if cfg!(windows) { 260 } else { 4096 };
const ILLEGAL_CHARS: &[char] = &['/', ':', '\\', '<', '>', '*', '|', '"', '?', '^'];

/// Auto-restart tuning for a server that keeps crashing right after boot (bad
/// heap args, a corrupted world, etc). Both the per-attempt delay and the
/// total attempt count are bounded, so a server that crashes instantly can
/// never spin the JVM in a tight loop: the delay doubles per consecutive fast
/// crash up to a ceiling, and auto-restart gives up entirely past a cap.
const CRASH_RESTART_BASE_DELAY_SECS: u64 = 3;
const CRASH_RESTART_MAX_DELAY_SECS: u64 = 5 * 60;
const CRASH_RESTART_MAX_ATTEMPTS: u32 = 6;
/// A run lasting at least this long is treated as healthy: a later crash
/// starts a fresh attempt count instead of continuing the backoff.
const CRASH_RESTART_HEALTHY_UPTIME_SECS: i64 = 60;
/// How long `finalize_run` lets a confirmed exit's last console lines land.
const SERVER_LOG_DRAIN_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// Backoff delay before auto-restarting a crashed server, doubling per
/// consecutive fast crash and capped so it can never grow unbounded (the
/// exponent is clamped before the shift, so this never overflows).
fn crash_restart_delay(attempts: u32) -> std::time::Duration {
    let exponent = attempts.saturating_sub(1).min(10);
    let backoff_secs = CRASH_RESTART_BASE_DELAY_SECS.saturating_mul(1u64 << exponent);
    std::time::Duration::from_secs(backoff_secs.min(CRASH_RESTART_MAX_DELAY_SECS))
}

/// Strips control characters (notably newlines) from a single-line console
/// command, so an operator-supplied field embedded in one — a ban reason, an
/// IP — can't smuggle in extra newline-separated console commands.
fn sanitize_console_command(command: &str) -> String {
    command.chars().filter(|c| !c.is_control()).collect()
}

/// Picks the directory an installed addon belongs in.
///
fn ensure_server_files_available(state: &ServerState) -> anyhow::Result<()> {
    // A server holding a marker from an earlier session passes here: the
    // marker check that follows (`refuse_unowned_marker`) removes it if its
    // process is gone, and otherwise refuses with the way out.
    match addon_turn(state) {
        AddonTurn::Now => Ok(()),
        AddonTurn::Deleting => bail!("This server is being deleted."),
        AddonTurn::Wait => {
            let (doing, done) = busy(state);
            bail!("This server is still {doing}. Wait until it {done}, then try again.")
        }
    }
}

/// When an addon may change the files of a server in a given state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddonTurn {
    /// Now: the server runs or is stopped. A marker from an earlier session
    /// is checked next (`refuse_unowned_marker`).
    Now,
    /// Once the installation, start or stop under way has ended. A modpack
    /// install or Repair rewrites `mods/`, and a JVM that has not exited yet
    /// may still hold its files.
    Wait,
    /// Never: the server is going away.
    Deleting,
}

fn addon_turn(state: &ServerState) -> AddonTurn {
    match state {
        ServerState::Running { .. }
        | ServerState::Stopped { .. }
        | ServerState::RecoveredRunning { .. }
        | ServerState::RecoveredUnverified { .. } => AddonTurn::Now,
        ServerState::Installing(_) | ServerState::Starting(_) | ServerState::Stopping => {
            AddonTurn::Wait
        }
        ServerState::Deleting => AddonTurn::Deleting,
    }
    Ok(())
}

/// What a server in a state an addon waits for is doing, and what it is
/// once that has ended, for messages.
fn busy(state: &ServerState) -> (&'static str, &'static str) {
    match state {
        ServerState::Installing(_) => ("installing", "has finished installing"),
        ServerState::Starting(_) => ("starting", "is running"),
        _ => ("stopping", "has stopped"),
    }
}

/// How long an addon installation waits for an installation, start or stop
/// of its server to end before it gives up and says so (see
/// `ManagerRef<ServerManager>::addon_gate`). A modpack can take many minutes
/// to install; a stop that takes this long did not end.
const ADDON_WAIT_LIMIT: std::time::Duration = std::time::Duration::from_secs(30 * 60);
/// How often a waiting addon installation looks at the server's state again.
const ADDON_WAIT_POLL: std::time::Duration = std::time::Duration::from_millis(500);

/// A server reads `.jar` mods from `mods/` and `.zip` datapacks from
/// the selected world's `datapacks/`, and `cache_server_local` scans those two
/// extensions in those two directories — a `.jar` under `datapacks/` or a
/// `.zip` under `mods/` is both dead to the server and invisible to that scan,
/// so it can't be listed or removed from the UI.
///
/// That scan is what makes the extension the whole decision here. The
/// platform's project type can't improve on it: a datapack project's
/// loader-packaged `.jar` version is a mod and belongs in `mods/`, and routing
/// any `.zip` anywhere but `datapacks/` would hide it. Deciding by type
/// instead would only be safe if the scanner inspected file contents too, which
/// it doesn't.
fn server_addon_dir(
    server_path: &ServerPath,
    filename: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let is_zip = std::path::Path::new(filename)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
    if is_zip {
        world::datapacks(&server_path.get_data_path())
    } else {
        Ok(server_path.get_mods_path())
    }
}

/// Removes `path`, a folder (`dir`) or file Repair reinstalls, if it is
/// there. A link, or a junction on Windows, is removed itself and what it
/// points to is kept (a `mods/` shared between servers stays as it is for
/// the others), as 2.0.40 did; the pack is then installed into a new folder.
async fn wipe_entry(path: &std::path::Path, dir: bool) -> anyhow::Result<()> {
    let meta = match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if meta.file_type().is_symlink() {
        info!(
            "Repair removes the link {} and keeps what it points to",
            path.display()
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if meta.file_type().is_symlink_dir() {
                return Ok(tokio::fs::remove_dir(path).await?);
            }
        }
        return Ok(tokio::fs::remove_file(path).await?);
    }
    if dir {
        tokio::fs::remove_dir_all(path).await?;
    } else {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

/// Narrows a CurseForge file search by the server's modloader, but only when
/// the project is a mod.
///
/// A datapack is published without loader tags, so filtering its files by a
/// loader matches none of them and the search reports the addon as having no
/// version compatible with the server.
fn curseforge_modloader_filter(
    class_id: Option<&carbon_platforms::curseforge::ClassId>,
    server_modloader: Option<&str>,
) -> Option<carbon_platforms::curseforge::ModLoaderType> {
    use carbon_platforms::curseforge::{ClassId, ModLoaderType};

    if !matches!(class_id, Some(ClassId::Mods) | None) {
        return None;
    }

    match server_modloader? {
        "forge" => Some(ModLoaderType::Forge),
        "fabric" => Some(ModLoaderType::Fabric),
        "quilt" => Some(ModLoaderType::Quilt),
        "neoforge" => Some(ModLoaderType::NeoForge),
        _ => None,
    }
}

/// The Modrinth half of [`curseforge_modloader_filter`]. A datapack's versions
/// are tagged `datapack` rather than with a loader, so a loader-filtered
/// version query returns an empty list.
fn modrinth_loader_filter(
    project_type: &carbon_platforms::modrinth::project::ProjectType,
    server_modloader: Option<&str>,
) -> Option<Vec<String>> {
    use carbon_platforms::modrinth::project::ProjectType;

    if *project_type != ProjectType::Mod {
        return None;
    }

    server_modloader.map(|loader| vec![loader.to_string()])
}

/// Resolves the server port, defaulting when unset, and rejects a value outside
/// the valid TCP range so an out-of-range port fails here with a clear message
/// rather than as an opaque JVM bind error at launch.
fn resolve_server_port(port: Option<i32>) -> anyhow::Result<i32> {
    let port = port.unwrap_or(25565);
    if !(1..=65535).contains(&port) {
        bail!("Server port must be between 1 and 65535, got {port}");
    }
    Ok(port)
}

/// Validates a `server-port` value from a `server.properties` patch against
/// the same range `resolve_server_port` enforces, so a bad value is rejected
/// before any DB/file write instead of surfacing later as an opaque JVM bind
/// error.
fn validate_server_port_patch(value: &str) -> anyhow::Result<i32> {
    let parsed: i32 = value
        .parse()
        .map_err(|_| anyhow!("Server port must be a number, got `{value}`"))?;
    if !(1..=65535).contains(&parsed) {
        bail!("Server port must be between 1 and 65535, got {parsed}");
    }
    Ok(parsed)
}

#[derive(Debug, thiserror::Error)]
#[error("Minecraft server EULA has not been accepted for server {server_id}")]
pub struct EulaNotAcceptedError {
    pub server_id: i32,
}

impl crate::error::FeErrorCode for EulaNotAcceptedError {
    fn error_code(&self) -> &'static str {
        "EULA_NOT_ACCEPTED"
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "server_id": self.server_id,
        }))
    }
}

/// Known Minecraft server player list files.
#[derive(Debug, Clone, Copy)]
pub enum PlayerListFile {
    Whitelist,
    Ops,
    BannedPlayers,
    BannedIps,
}

impl PlayerListFile {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Whitelist => "whitelist.json",
            Self::Ops => "ops.json",
            Self::BannedPlayers => "banned-players.json",
            Self::BannedIps => "banned-ips.json",
        }
    }
}

#[derive(Debug)]
pub struct ServerData {
    pub shortpath: String,
    pub state: ServerState,
    pub handle: Option<ServerHandle>,
    /// Log ID from the previous (or current) session, used to clean up
    /// stale entries from `server_logs` when the server is started again.
    pub last_log_id: Option<ServerLogId>,
}

pub struct ServerManager {
    pub(crate) servers: RwLock<HashMap<ServerId, ServerData>>,
    server_op_locks: Arc<DashMap<ServerId, Arc<Mutex<()>>>>,
    /// One addon installation per server at a time, in the order they were
    /// asked for (tokio's mutex is fair), so each plans against what the one
    /// before it published. A second Install waits behind the first instead
    /// of being refused.
    addon_installations: Arc<DashMap<ServerId, Arc<Mutex<()>>>>,
    server_logs: RwLock<HashMap<ServerLogId, watch::Sender<Vec<String>>>>,
    log_counter: Mutex<i32>,
    index_lock: Mutex<()>,
    /// Consecutive fast-crash count per server, used to back off and cap
    /// automatic restarts. See `crash_restart_delay`.
    crash_restart_state: DashMap<ServerId, u32>,
    /// Servers whose auto-restart has hit `CRASH_RESTART_MAX_ATTEMPTS` and
    /// given up. Surfaced to the frontend as `auto_restart_abandoned`;
    /// cleared the moment the server is started manually.
    crash_restart_abandoned: DashMap<ServerId, ()>,
    /// Whether `watch_recovered_servers` runs.
    recovered_watcher: AtomicBool,
    /// How often it checks, in milliseconds: `RECOVERED_POLL_INTERVAL`,
    /// which tests shorten or stretch.
    recovered_poll_ms: AtomicU64,
}

impl std::fmt::Debug for ServerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerManager").finish()
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            server_op_locks: Arc::new(DashMap::new()),
            addon_installations: Arc::new(DashMap::new()),
            server_logs: RwLock::new(HashMap::new()),
            log_counter: Mutex::new(0),
            index_lock: Mutex::new(()),
            crash_restart_state: DashMap::new(),
            crash_restart_abandoned: DashMap::new(),
            recovered_watcher: AtomicBool::new(false),
            recovered_poll_ms: AtomicU64::new(RECOVERED_POLL_INTERVAL.as_millis() as u64),
        }
    }

    /// Whether `id`'s auto-restart has given up after repeated fast crashes
    /// without a healthy run. Cleared the moment the server is next started
    /// manually — see `ManagerRef<ServerManager>::start_server`.
    pub fn is_auto_restart_abandoned(&self, id: ServerId) -> bool {
        self.crash_restart_abandoned.contains_key(&id)
    }

    /// Records a crash for `id` and returns the updated consecutive
    /// fast-crash count plus whether auto-restart has now given up. A prior
    /// run lasting at least `CRASH_RESTART_HEALTHY_UPTIME_SECS` (`healthy`)
    /// resets the streak instead of continuing it. Hitting
    /// `CRASH_RESTART_MAX_ATTEMPTS` marks the server abandoned.
    fn record_crash_restart_attempt(&self, id: ServerId, healthy: bool) -> (u32, bool) {
        let attempts = {
            let mut entry = self.crash_restart_state.entry(id).or_insert(0);
            if healthy {
                *entry = 0;
            }
            *entry += 1;
            *entry
        };

        let abandoned = attempts > CRASH_RESTART_MAX_ATTEMPTS;
        if abandoned {
            self.crash_restart_abandoned.insert(id, ());
        }
        (attempts, abandoned)
    }

    async fn cancel_pending_restart(&self, id: ServerId) {
        if let Some(server) = self.servers.write().await.get_mut(&id) {
            if matches!(server.state, ServerState::Stopped { .. }) {
                server.last_log_id = None;
            }
        }
    }
    async fn restart_allowed(&self, id: ServerId, expected: ServerLogId, enabled: bool) -> bool {
        enabled
            && self.servers.read().await.get(&id).is_some_and(|server| {
                matches!(server.state, ServerState::Stopped { .. })
                    && server.last_log_id == Some(expected)
            })
    }
    async fn record_owned_restart(
        &self,
        id: ServerId,
        log_id: ServerLogId,
        healthy: bool,
    ) -> Option<(u32, bool)> {
        let lock = self
            .server_op_locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        if !self.restart_allowed(id, log_id, true).await {
            return None;
        }
        Some(self.record_crash_restart_attempt(id, healthy))
    }
    fn get_provider(&self) -> Box<dyn ServerProvider> {
        Box::new(LocalServerProvider)
    }
    /// The stable gate serializing lifecycle and file changes of one server.
    fn op_lock(&self, id: ServerId) -> Arc<Mutex<()>> {
        self.server_op_locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
    /// The turn of one addon installation on server `id` (see
    /// `addon_installations`).
    pub(crate) fn addon_queue(&self, id: ServerId) -> Arc<Mutex<()>> {
        self.addon_installations
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Drain captured runs concurrently without holding gates needed by finalizers.
    pub async fn shutdown_running(&self) -> anyhow::Result<()> {
        self.shutdown_running_for(&ShutdownDrain::default()).await
    }
    /// [`Self::shutdown_running`] for a quit, restart or update `request`.
    ///
    /// Servers that run now are asked to stop at once, before anything else
    /// is waited for, so one still saving when the user stops the launcher
    /// anyway already got 'stop' and finishes on its own. Starts that were
    /// already under way are joined through their gates and asked to stop
    /// next. Once the request is cancelled or replaced, servers that have not
    /// been asked yet are left running, and so is any started afterwards;
    /// those already asked still finish stopping.
    pub async fn shutdown_running_for(&self, request: &ShutdownDrain) -> anyhow::Result<()> {
        let running = self.ask_to_stop(request, &HashSet::new()).await;
        let asked: HashSet<_> = running
            .iter()
            .map(|(_, handle)| handle.generation)
            .collect();
        let started = async {
            // Freeze has already closed final spawn admission. Join
            // queued/preparing starts; no gate is held while waiting for exit.
            let gates: Vec<_> = self
                .server_op_locks
                .iter()
                .map(|entry| entry.value().clone())
                .collect();
            for gate in gates {
                let guard = gate.lock().await;
                drop(guard);
            }
            let started = self.ask_to_stop(request, &asked).await;
            self.drain_for_shutdown(&started).await
        };
        let (running, started) = tokio::join!(self.drain_for_shutdown(&running), started);
        running.and(started)
    }
    /// Marks every run with a handle, other than those in `skip`, Stopping and
    /// returns it for draining, unless `request` was cancelled. Checked under
    /// the lock that publishes handles: a server started after a cancel can
    /// only publish its handle once this has returned.
    async fn ask_to_stop(
        &self,
        request: &ShutdownDrain,
        skip: &HashSet<uuid::Uuid>,
    ) -> Vec<(ServerId, ServerHandle)> {
        let mut servers = self.servers.write().await;
        if request.cancelled() {
            return vec![];
        }
        let handles: Vec<_> = servers
            .iter_mut()
            .filter_map(|(id, data)| {
                let handle = data.handle.clone()?;
                if skip.contains(&handle.generation) {
                    return None;
                }
                if !matches!(data.state, ServerState::Deleting) {
                    data.state = ServerState::Stopping;
                }
                Some((*id, handle))
            })
            .collect();
        if !handles.is_empty() {
            request.servers_asked_to_stop();
        }
        handles
    }
    async fn drain_for_shutdown(&self, handles: &[(ServerId, ServerHandle)]) -> anyhow::Result<()> {
        let outcomes = futures::future::join_all(handles.iter().map(|(id, handle)| async move {
            provider::drain(handle, false, std::time::Duration::from_secs(30)).await?;
            self.finalize_run(*id, handle).await?;
            Ok::<_, anyhow::Error>(())
        }))
        .await;
        for outcome in outcomes {
            outcome?;
        }
        Ok(())
    }
    async fn finalize_run(&self, id: ServerId, handle: &ServerHandle) -> anyhow::Result<bool> {
        // Lets the last console lines land before the tile shows Stopped. The
        // process exit is already confirmed, so console output that is still
        // unsettled after the bound only costs log lines, never the cleanup.
        if let Some(mut signal) = handle.log_drained.clone() {
            match tokio::time::timeout(SERVER_LOG_DRAIN_GRACE, signal.wait_confirmed()).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    warn!("Server {} console publication ended early: {error:#}", id.0)
                }
                Err(_) => warn!(
                    "Server {} console output was still publishing {SERVER_LOG_DRAIN_GRACE:?} after exit; finalizing the confirmed exit anyway",
                    id.0
                ),
            }
        }
        let lock = self
            .server_op_locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        {
            let servers = self.servers.read().await;
            let Some(server) = servers.get(&id) else {
                return Ok(false);
            };
            if !server
                .handle
                .as_ref()
                .is_some_and(|current| current.generation == handle.generation)
            {
                return Ok(false);
            }
        }
        if let Some(root) = &handle.marker_root {
            orphan_pid::remove_pid_file_strict(root, provider::PID_FILE_NAME).await?;
        }
        let mut servers = self.servers.write().await;
        let server = servers
            .get_mut(&id)
            .context("Server disappeared during owned finalization")?;
        if !server
            .handle
            .as_ref()
            .is_some_and(|current| current.generation == handle.generation)
        {
            bail!("Server run changed during finalization");
        }
        let unexpected = matches!(server.state, ServerState::Running { .. });
        server.handle = None;
        if !matches!(server.state, ServerState::Deleting) {
            server.state = ServerState::Stopped { failed_task: None };
        }
        handle.finish_publication();
        Ok(unexpected)
    }
    async fn wait_for_process_exit(
        &self,
        id: ServerId,
        timeout: std::time::Duration,
    ) -> anyhow::Result<()> {
        let handle = self
            .servers
            .read()
            .await
            .get(&id)
            .and_then(|server| server.handle.clone());
        let Some(mut handle) = handle else {
            bail!("No captured server completion evidence");
        };
        tokio::time::timeout(timeout, handle.exited.wait_confirmed())
            .await
            .map_err(|_| anyhow!("Server exit not confirmed; files retained"))?
    }
}

/// How long a server left running by an earlier session gets to save and
/// exit after SIGTERM before it is killed: the graceful budget of an
/// ordinary Stop (`provider::drain`).
const RECOVERED_STOP_GRACE: std::time::Duration = std::time::Duration::from_secs(30);
/// How long a kill gets to be confirmed.
const RECOVERED_KILL_CONFIRM: std::time::Duration = std::time::Duration::from_secs(5);
/// How often servers from an earlier session are checked for having ended
/// on their own (`watch_recovered_servers`), and so how long their page can
/// lag a JVM that exited. Each check reads one small file and one process.
const RECOVERED_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// What a server's process marker from an earlier session says when it is
/// checked. Every check removes a marker whose process ended or whose PID
/// another process now holds, so only the two kinds that stay are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerCheck {
    /// No marker is left.
    Clear,
    /// The server's JVM from an earlier session still runs.
    Running(orphan_pid::ProcessRecord),
    /// The marker cannot be read (`None`), or it names a live process that
    /// may or may not be this server.
    Unverified(Option<orphan_pid::ProcessRecord>),
}

impl MarkerCheck {
    /// The state an idle server shows for this result.
    fn state(self) -> ServerState {
        match self {
            Self::Clear => ServerState::Stopped { failed_task: None },
            Self::Running(record) => ServerState::RecoveredRunning {
                process_id: record.pid,
            },
            Self::Unverified(record) => ServerState::RecoveredUnverified {
                process_id: record.map(|record| record.pid),
            },
        }
    }
    /// Refuses to start or change a server whose marker stays, saying what
    /// to do about it.
    fn refuse(self) -> anyhow::Result<()> {
        let (kind, pid) = match self {
            Self::Clear => return Ok(()),
            Self::Running(record) => (RecoveredMarkerKind::Running, Some(record.pid)),
            Self::Unverified(Some(record)) => (RecoveredMarkerKind::Unverified, Some(record.pid)),
            Self::Unverified(None) => (RecoveredMarkerKind::Unreadable, None),
        };
        Err(RecoveredServerError { kind, pid }.into())
    }
}

/// What a server's marker from an earlier session that refused a start,
/// delete or change says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredMarkerKind {
    /// The server's JVM from that session still runs.
    Running,
    /// It names a live process that may or may not be this server.
    Unverified,
    /// It cannot be read.
    Unreadable,
}

/// A server's marker from an earlier session refused a start, delete or
/// change. The launcher shows its own translated text for it, which points
/// to the server page; this English text is for logs and inline errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredServerError {
    pub kind: RecoveredMarkerKind,
    pub pid: Option<u32>,
}

impl std::fmt::Display for RecoveredServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pid = self
            .pid
            .map(|pid| format!(" (PID {pid})"))
            .unwrap_or_default();
        match self.kind {
            RecoveredMarkerKind::Running => write!(
                f,
                "This server is still running from a previous session{pid}, so GDLauncher can't start or change it. Stop it from the server page (Stop, or Force stop on Windows), then try again."
            ),
            RecoveredMarkerKind::Unverified => write!(
                f,
                "A record from a previous session says this server may still be running{pid}, but GDLauncher can't confirm it. If that process is this server, end it first; if you can't tell, restart your computer. Then clear the record on the server page and try again."
            ),
            RecoveredMarkerKind::Unreadable => write!(
                f,
                "A record from a previous session says this server may still be running, but GDLauncher can't read it. If no server from this folder is running (restarting your computer makes sure of that), clear the record on the server page, then try again."
            ),
        }
    }
}

impl std::error::Error for RecoveredServerError {}

impl crate::error::FeErrorCode for RecoveredServerError {
    fn error_code(&self) -> &'static str {
        "SERVER_RECOVERED_PROCESS"
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": match self.kind {
                RecoveredMarkerKind::Running => "running",
                RecoveredMarkerKind::Unverified => "unverified",
                RecoveredMarkerKind::Unreadable => "unreadable",
            },
            "pid": self.pid,
            // macOS and Linux let it save and stop; Windows has Force stop.
            "graceful_stop": cfg!(unix),
        }))
    }
}

/// The ways out a recovered server's stop names when the system refuses
/// it, on every desktop the launcher runs on.
const PROCESS_TOOLS: &str = "Task Manager, Activity Monitor or System Monitor";

/// Reads and classifies the marker under `root`. A marker whose process
/// ended, or whose PID is now held by another process (not Java, another
/// creation time on Windows and macOS, another boot/ticks identity on
/// Linux), is removed and the lifecycle forgets it. One that cannot be read
/// stays byte for byte: it is relocation evidence until the user clears it.
async fn classify_marker(
    lifecycle: &super::runtime_lifecycle::RuntimeLifecycle,
    root: &std::path::Path,
) -> anyhow::Result<MarkerCheck> {
    let marker = provider::pid_file_path(root);
    let record = match provider::read_process_file(root).await {
        Ok(None) => return Ok(MarkerCheck::Clear),
        Ok(Some(record)) => record,
        Err(error) => {
            warn!(
                "Cannot read the server process marker {}: {error:#}",
                marker.display()
            );
            return Ok(MarkerCheck::Unverified(None));
        }
    };
    match lifecycle.marker_status(record) {
        orphan_pid::IdentityStatus::Same => Ok(MarkerCheck::Running(record)),
        orphan_pid::IdentityStatus::Unknown => Ok(MarkerCheck::Unverified(Some(record))),
        orphan_pid::IdentityStatus::GoneOrReused => {
            provider::retire_recovered_marker(root, record)
                .await
                .with_context(|| {
                    format!(
                        "The server from the previous session has stopped, but its record {} could not be removed. Delete that file, then try again",
                        marker.display()
                    )
                })?;
            lifecycle.release_marker(&marker, Some(record));
            Ok(MarkerCheck::Clear)
        }
    }
}

/// [`classify_marker`] for server `id`, shown on it while it is idle so
/// the page and tile follow what the check found. Caller holds its gate.
async fn check_marker(
    app: &super::App,
    id: ServerId,
    root: &std::path::Path,
) -> anyhow::Result<MarkerCheck> {
    let check = classify_marker(&app.runtime_lifecycle, root).await?;
    let (changed, recovered) = {
        let mut servers = app.server_manager.servers.write().await;
        match servers.get_mut(&id) {
            Some(server) if server.handle.is_none() && server.state.is_stopped_or_recovered() => {
                let state = match (check, &server.state) {
                    // Keeps a failed install's error on the tile.
                    (MarkerCheck::Clear, ServerState::Stopped { .. }) => server.state.clone(),
                    _ => check.state(),
                };
                let changed = server.state != state;
                server.state = state;
                (changed, watched_while_recovered(&server.state))
            }
            _ => (false, false),
        }
    };
    if recovered {
        app.server_manager().watch_recovered_servers();
    }
    if changed {
        app.invalidate(GET_ALL_SERVERS, None);
        app.invalidate(GET_SERVER_DETAILS, None);
    }
    Ok(check)
}

/// A server from an earlier session whose record can go away without the
/// launcher, which `watch_recovered_servers` then notices: its JVM, or the
/// live process an unverified record names, ends; or the user deletes a
/// record that cannot be read, as the move dialog advises.
fn watched_while_recovered(state: &ServerState) -> bool {
    matches!(
        state,
        ServerState::RecoveredRunning { .. } | ServerState::RecoveredUnverified { .. }
    )
}

/// Whether the recovered process was signalled, or was already gone.
enum Signalled {
    Sent,
    Gone,
}

/// Sends `signal` to the JVM `record` names, after checking once more that
/// the PID still belongs to it.
fn signal_recovered(
    lifecycle: &super::runtime_lifecycle::RuntimeLifecycle,
    record: orphan_pid::ProcessRecord,
    signal: sysinfo::Signal,
) -> anyhow::Result<Signalled> {
    match lifecycle.marker_status(record) {
        orphan_pid::IdentityStatus::GoneOrReused => return Ok(Signalled::Gone),
        // The record no longer verifies: the page shows it as unverified,
        // with its way out.
        orphan_pid::IdentityStatus::Unknown => {
            return Err(RecoveredServerError {
                kind: RecoveredMarkerKind::Unverified,
                pid: Some(record.pid),
            }
            .into());
        }
        orphan_pid::IdentityStatus::Same => {}
    }
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(record.pid)]));
    let Some(process) = system.process(Pid::from_u32(record.pid)) else {
        return Ok(Signalled::Gone);
    };
    let sent = match signal {
        sysinfo::Signal::Kill => process.kill(),
        signal => process.kill_with(signal).unwrap_or(false),
    };
    if !sent {
        bail!(
            "The system did not let GDLauncher stop process {}. End it in {PROCESS_TOOLS}, then try again.",
            record.pid
        );
    }
    Ok(Signalled::Sent)
}

/// Whether the process `record` names is gone within `budget`.
async fn recovered_gone_within(
    lifecycle: &super::runtime_lifecycle::RuntimeLifecycle,
    record: orphan_pid::ProcessRecord,
    budget: std::time::Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        if lifecycle.marker_status(record) == orphan_pid::IdentityStatus::GoneOrReused {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// Stops a server JVM an earlier session left running. Its console is gone,
/// so on macOS and Linux it gets SIGTERM, which runs Minecraft's shutdown
/// hook (the worlds are saved, then it exits), and SIGKILL only when it is
/// still running after `grace`. Windows has no such request for a process
/// without a console of its own, so there it is ended right away; the
/// launcher asks the user to confirm that first. The PID's identity is
/// checked again right before each signal.
async fn stop_recovered_process(
    lifecycle: &super::runtime_lifecycle::RuntimeLifecycle,
    record: orphan_pid::ProcessRecord,
    grace: std::time::Duration,
) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        if let Signalled::Gone = signal_recovered(lifecycle, record, sysinfo::Signal::Term)? {
            return Ok(());
        }
        if recovered_gone_within(lifecycle, record, grace).await {
            return Ok(());
        }
        warn!(
            "Server process {} from a previous session was still running {grace:?} after SIGTERM; killing it",
            record.pid
        );
    }
    #[cfg(not(unix))]
    let _ = grace;
    if let Signalled::Gone = signal_recovered(lifecycle, record, sysinfo::Signal::Kill)? {
        return Ok(());
    }
    if recovered_gone_within(lifecycle, record, RECOVERED_KILL_CONFIRM).await {
        return Ok(());
    }
    bail!(
        "The server from the previous session (PID {}) did not stop. End it in {PROCESS_TOOLS}, then try again.",
        record.pid
    )
}

impl ManagerRef<'_, ServerManager> {
    pub async fn launch_background_tasks(self) {
        if let Err(e) = self.load_servers().await {
            error!("Failed to load servers: {}", e);
            self.app.runtime_lifecycle.reconciliation_finished(Err(e));
        }

        // Queue caching for all servers on startup
        let servers = self.servers.read().await;
        for (&server_id, _) in servers.iter() {
            self.app
                .meta_cache_manager()
                .queue_caching(
                    crate::managers::metadata::cache::CacheEntityId::Server(server_id.0),
                    false,
                )
                .await;
        }
    }

    async fn load_servers(self) -> anyhow::Result<()> {
        let db_servers = server_repo::get_all_servers(&self.app.db).await?;

        // Before any server is registered in memory, check its marker
        // against the live process table: a JVM from a session the core
        // didn't shut down cleanly (crash, force-quit, Windows
        // TerminateProcess) is otherwise invisible here and keeps holding
        // its port forever.
        match self
            .app
            .runtime_lifecycle
            .reconcile_root(
                &self
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_servers()
                    .to_path(),
                provider::PID_FILE_NAME,
            )
            .await
        {
            Ok(()) => self.clean_up_orphaned_servers(&db_servers).await,
            Err(error) => self
                .app
                .runtime_lifecycle
                .reconciliation_finished(Err(error)),
        }

        let any_recovered = recovered.values().any(watched_while_recovered);
        {
            let mut servers = self.servers.write().await;
            for db_server in db_servers {
                let id = ServerId(db_server.id);
                servers.insert(
                    id,
                    ServerData {
                        shortpath: db_server.shortpath,
                        state: recovered
                            .remove(&id)
                            .unwrap_or(ServerState::Stopped { failed_task: None }),
                        handle: None,
                        last_log_id: None,
                    },
                );
            }
        }
        if any_recovered {
            self.watch_recovered_servers();
        }

        Ok(())
    }

    /// Notices when a server from an earlier session ends without the
    /// launcher's help: it exited, crashed, was stopped in-game or ended in
    /// Task Manager, or (unverified) the process its record names ended.
    /// That server then reads Stopped and its marker and every record of it
    /// are removed, as Start would; without this its page would say it
    /// still hosts the world and offer only Stop or Force stop for a process
    /// that is gone. Like the watcher of adopted games, it runs only while
    /// such a server is left (`watched_while_recovered`) and ends on its own
    /// once none is.
    pub(crate) fn watch_recovered_servers(self) {
        if self.recovered_watcher.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = self.app.clone();
        tokio::spawn(async move {
            loop {
                let interval = std::time::Duration::from_millis(
                    app.server_manager.recovered_poll_ms.load(Ordering::Relaxed),
                );
                tokio::time::sleep(interval).await;
                let watched: Vec<ServerId> = {
                    let servers = app.server_manager.servers.read().await;
                    let watched: Vec<_> = servers
                        .iter()
                        .filter(|(_, server)| watched_while_recovered(&server.state))
                        .map(|(id, _)| *id)
                        .collect();
                    if watched.is_empty() {
                        // Under the lock: a check that makes a server
                        // recovered writes its state first and then starts
                        // the watcher again.
                        app.server_manager
                            .recovered_watcher
                            .store(false, Ordering::SeqCst);
                        return;
                    }
                    watched
                };
                for id in watched {
                    app.server_manager().recheck_recovered_server(id).await;
                }
            }
        });
    }

    /// One check of `watch_recovered_servers`. Skipped while a quit, restart
    /// or move of the runtime folder holds admission, since the check may
    /// remove the marker, and while an action on that server holds its gate:
    /// that action checks the marker itself.
    async fn recheck_recovered_server(self, id: ServerId) {
        let Ok(_runtime_activity) = self.app.runtime_lifecycle.admit("recovered server check")
        else {
            return;
        };
        let lock = self.get_op_lock(id);
        let Ok(_guard) = lock.try_lock() else {
            return;
        };
        let Some(state) = self
            .servers
            .read()
            .await
            .get(&id)
            .filter(|server| server.handle.is_none() && watched_while_recovered(&server.state))
            .map(|server| server.state.clone())
        else {
            return;
        };
        let Ok(root) = self.server_root(id).await else {
            return;
        };
        // A record that cannot be read only changes when it is deleted.
        // Reading it again every poll would only log the same error.
        if matches!(state, ServerState::RecoveredUnverified { process_id: None })
            && tokio::fs::try_exists(provider::pid_file_path(&root))
                .await
                .unwrap_or(true)
        {
            return;
        }
        if let Err(error) = check_marker(self.app, id, &root).await {
            tracing::debug!("Recovered server {} not rechecked: {error:#}", id.0);
        }
    }

    /// Removes every marker whose process ended or whose PID another
    /// process now holds, as 2.0.40 did, and returns the state of each
    /// server whose marker stays: running from a previous session, or
    /// unverified. Nothing is signalled here. 2.0.40 killed a recovered JVM
    /// without letting it save; stopping it is now the user's choice.
    async fn clean_up_orphaned_servers(
        self,
        db_servers: &[server_repo::ServerRow],
    ) -> HashMap<ServerId, ServerState> {
        let mut recovered = HashMap::new();
        for server in db_servers {
            let root = self
                .app
                .settings_manager()
                .runtime_path
                .get_servers()
                .get_server_path(&server.shortpath)
                .get_root();
            if let Ok(Some((pid, start))) = provider::read_pid_file(&root).await {
                if super::runtime_lifecycle::recorded_process_confirmed_absent(
                    pid,
                    start.unwrap_or(0),
                ) {
                    if let Err(error) =
                        orphan_pid::remove_pid_file_strict(&root, provider::PID_FILE_NAME).await
                    {
                        warn!("Cannot retire confirmed-absent server marker: {error:#}");
                    }
                    continue;
                }
            }
            if provider::pid_file_path(&root).exists() {
                warn!(
                    "Server {} has recovery ownership: verify it has stopped before retrying. No orphan is automatically terminated; save completion is unknown.",
                    server.id
                );
            }
        }
        recovered
    }

    /// Caller holds the stable server gate. A marker from an earlier session
    /// refuses destructive runtime writes unless its process turns out to be
    /// gone, in which case it is removed and the write goes ahead. A marker
    /// of a run this core started (it holds the handle) is its own.
    pub(crate) async fn refuse_unowned_marker(
        self,
        id: ServerId,
        root: &std::path::Path,
    ) -> anyhow::Result<()> {
        if self
            .servers
            .read()
            .await
            .get(&id)
            .is_some_and(|server| server.handle.is_some())
        {
            return Ok(());
        }
        check_marker(self.app, id, root).await?.refuse()
    }

    pub(crate) async fn ensure_addon_publication(
        self,
        id: ServerId,
        root: &std::path::Path,
    ) -> anyhow::Result<()> {
        {
            let servers = self.servers.read().await;
            let server = servers.get(&id).context("Server disappeared")?;
            ensure_server_files_available(&server.state)?;
        }
        self.refuse_unowned_marker(id, root).await
    }

    /// The server gate for an addon installation on server `id` (`name`, for
    /// messages), once its files may change: an installation, start or stop
    /// of the server under way is waited for, without holding the gate,
    /// instead of refused, so an addon picked meanwhile is installed once it
    /// has ended. 2.0.40 wrote the addon at once whatever the server did. A
    /// marker from an earlier session is checked as for every other addon
    /// change. Gives up after [`ADDON_WAIT_LIMIT`], or when `cancel` fires.
    pub(crate) async fn addon_gate(
        self,
        id: ServerId,
        name: &str,
        root: &std::path::Path,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<tokio::sync::OwnedMutexGuard<()>> {
        self.addon_gate_within(id, name, root, cancel, ADDON_WAIT_LIMIT)
            .await
    }

    async fn addon_gate_within(
        self,
        id: ServerId,
        name: &str,
        root: &std::path::Path,
        cancel: &tokio_util::sync::CancellationToken,
        limit: std::time::Duration,
    ) -> anyhow::Result<tokio::sync::OwnedMutexGuard<()>> {
        let deadline = tokio::time::Instant::now() + limit;
        loop {
            let guard = tokio::select! {
                biased;
                () = cancel.cancelled() => bail!("Server addon installation cancelled"),
                guard = self.get_op_lock(id).lock_owned() => guard,
            };
            let state = self
                .servers
                .read()
                .await
                .get(&id)
                .map(|server| server.state.clone())
                .context("Server not found")?;
            match addon_turn(&state) {
                AddonTurn::Now => {
                    self.refuse_unowned_marker(id, root).await?;
                    return Ok(guard);
                }
                AddonTurn::Deleting => {
                    bail!("{name} is being deleted, so the addon was not installed.")
                }
                AddonTurn::Wait => drop(guard),
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                let (doing, done) = busy(&state);
                bail!(
                    "{name} is still {doing}, so the addon was not installed. Install it again once the server {done}."
                );
            }
            tokio::select! {
                biased;
                () = cancel.cancelled() => bail!("Server addon installation cancelled"),
                () = tokio::time::sleep(ADDON_WAIT_POLL.min(deadline - now)) => {}
            }
        }
    }

    pub(crate) fn get_op_lock(self, id: ServerId) -> Arc<Mutex<()>> {
        self.op_lock(id)
    }

    pub async fn get_default_group(self) -> anyhow::Result<ServerGroupId> {
        let config = carbon_repos::repos::app_configuration::get_app_configuration(&self.app.db)
            .await?
            .ok_or_else(|| anyhow!("App configuration not found"))?;

        match config.default_server_group {
            Some(id) => Ok(ServerGroupId(id)),
            None => {
                // Find the first group
                let group = server_repo::first_server_group_ordered_by_group_index(&self.app.db)
                    .await?
                    .ok_or_else(|| anyhow!("No server group found"))?;

                Ok(ServerGroupId(group.id))
            }
        }
    }

    pub async fn list_groups(self) -> anyhow::Result<Vec<server::ServerGroup>> {
        let (groups, all_servers) = self
            .app
            .db
            .read(|conn| {
                // reads share one WAL snapshot
                let snap = conn.snapshot()?;
                let groups = server_repo::get_all_server_groups_ordered_by_group_index_conn(&snap)?;
                let servers = server_repo::get_all_servers_ordered_by_index_conn(&snap)?;
                Ok((groups, servers))
            })
            .await?;

        // Nest servers under their group, preserving the index-asc order the
        // query returned them in.
        let mut servers_by_group: HashMap<i32, Vec<server_repo::ServerRow>> = HashMap::new();
        for s in all_servers {
            servers_by_group.entry(s.group_id).or_default().push(s);
        }

        let active_servers = self.servers.read().await;

        Ok(groups
            .into_iter()
            .map(|group| server::ServerGroup {
                id: ServerGroupId(group.id),
                name: group.name,
                group_index: group.group_index,
                library_position: group.library_position,
                servers: servers_by_group
                    .remove(&group.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|s| active_servers.contains_key(&ServerId(s.id)))
                    .map(|s| ServerListEntry {
                        id: ServerId(s.id),
                        group_id: ServerGroupId(s.group_id),
                        index: s.index,
                        library_position: s.library_position,
                        name: s.name,
                        favorite: s.favorite,
                        server_type: ServerType::from_db_fields(
                            &s.server_type,
                            s.modloader_type.as_deref(),
                        )
                        .unwrap_or(ServerType::Vanilla),
                        game_version: s.game_version,
                        port: s.port,
                        date_created: s.date_created.into(),
                        last_started: s.last_started.map(|d| d.into()),
                        icon_revision: s.icon_revision.map(|v| v as u32),
                        modloader_type: s.modloader_type,
                        modloader_version: s.modloader_version,
                        modpack_info: s.modpack_platform.map(|platform| ServerModpackInfo {
                            platform,
                            project_id: s.modpack_project_id.unwrap_or_default(),
                            file_id: s.modpack_file_id.unwrap_or_default(),
                        }),
                    })
                    .collect(),
            })
            .collect())
    }

    pub async fn create_server(
        self,
        group_id: ServerGroupId,
        name: String,
        game_version: String,
        port: Option<i32>,
        modloader_type: Option<String>,
        modloader_version: Option<String>,
    ) -> anyhow::Result<ServerId> {
        let mut runtime_activity = self.app.runtime_lifecycle.admit("server create_server")?;
        use crate::api::translation::Translation;
        use crate::managers::vtask::VisualTask;

        if name.is_empty() {
            bail!("Server name cannot be empty");
        }

        let port = resolve_server_port(port)?;

        // Generate shortpath from name
        let shortpath = generate_shortpath(&name);
        let runtime_path = &self.app.settings_manager().runtime_path;
        let servers_path = runtime_path.get_servers();
        let server_path = servers_path.get_server_path(&shortpath);

        // Create directory structure
        tokio::fs::create_dir_all(server_path.get_data_path())
            .await
            .context("Failed to create server directory")?;

        // Hold index_lock to make lookup+create atomic
        let _index_guard = self.index_lock.lock().await;

        // Newly created servers appear at the TOP of their group — pick an
        // index strictly smaller than the current minimum so ascending
        // sort on `index` puts the new row first.
        let group_id_val = group_id.0;
        let min_index: Option<i32> =
            server_repo::min_index_server_in_group(&self.app.db, group_id_val)
                .await?
                .map(|s| s.index);
        let next_index = min_index.map(|n| n - 1).unwrap_or(0);

        // If the new server is in the default server group, also give it a
        // library_position that sorts above every existing server or
        // folder at the library's top level.
        let default_group_id = self.clone().get_default_group().await?;
        let library_position = if group_id == default_group_id {
            let default_id = default_group_id.0;
            let (min_server_pos, min_group_pos) = self
                .app
                .db
                .read(move |conn| {
                    // reads share one WAL snapshot
                    let snap = conn.snapshot()?;
                    let srv =
                        server_repo::min_library_position_server_in_group_conn(&snap, default_id)?
                            .and_then(|s| s.library_position);
                    let grp = server_repo::min_library_position_server_group_conn(&snap)?
                        .and_then(|g| g.library_position);
                    Ok((srv, grp))
                })
                .await?;

            let current_min = match (min_server_pos, min_group_pos) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            Some(current_min.map(|n| n - 1).unwrap_or(0))
        } else {
            None
        };

        // Create DB record
        let server_type = if modloader_type.is_some() {
            "modded".to_string()
        } else {
            "vanilla".to_string()
        };
        let (name_c, shortpath_c, game_version_c, modloader_type_c, modloader_version_c) = (
            name.clone(),
            shortpath.clone(),
            game_version.clone(),
            modloader_type.clone(),
            modloader_version.clone(),
        );
        let new_id = server_repo::insert_server(
            &self.app.db,
            name_c,
            shortpath_c,
            next_index,
            group_id_val,
            game_version_c,
            port,
            server_type,
            modloader_type_c,
            modloader_version_c,
            None,
            None,
            None,
            library_position,
            DbDateTime(Utc::now().into()),
        )
        .await?;

        drop(_index_guard);

        let server_id = ServerId(new_id as i32);

        // Create a visual task for install progress
        let task = VisualTask::new(Translation::ServerTaskInstall {
            server_name: name.clone(),
        });
        let task_id = self.app.task_manager().spawn_task(&task).await;

        // Register in memory as Installing
        self.servers.write().await.insert(
            server_id,
            ServerData {
                shortpath: shortpath.clone(),
                state: ServerState::Installing(task_id),
                handle: None,
                last_log_id: None,
            },
        );

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);

        // Do the heavy work (jar download, modloader install) in background
        let app = self.app.clone();
        runtime_activity.external();
        tokio::spawn(async move {
            let _runtime_activity = runtime_activity;
            let runtime_path = &app.settings_manager().runtime_path;
            let server_path = runtime_path.get_servers().get_server_path(&shortpath);

            // Create subtasks for progress reporting — only create ones that will run
            let t_download_jar = task.subtask(Translation::ServerTaskDownloadServerJar);
            t_download_jar.set_weight(10.0);
            let t_install_modloader = if modloader_type.is_some() && modloader_version.is_some() {
                let sub = task.subtask(Translation::ServerTaskInstallModloader);
                sub.set_weight(5.0);
                Some(sub)
            } else {
                None
            };
            task.edit(|data| data.state = crate::managers::vtask::TaskState::KnownProgress)
                .await;

            let install_result: anyhow::Result<()> = async {
                // Download server jar
                jars::download_vanilla_server_jar(
                    &app.reqwest_client,
                    &game_version,
                    &server_path,
                    Some(&t_download_jar),
                )
                .await
                .context("Failed to download server jar")?;

                // Write initial server.properties
                let props_content = properties::generate_properties(
                    port,
                    "A Minecraft Server",
                    20,
                    true,
                    &game_version,
                );
                properties::write_properties(
                    &server_path.get_server_properties_path(),
                    &props_content,
                )
                .await?;

                // Install modloader if specified
                if let (Some(ml_type), Some(ml_version), Some(t_install)) =
                    (&modloader_type, &modloader_version, &t_install_modloader)
                {
                    let java_path = app
                        .java_manager()
                        .find_java_for_server_version(&game_version, Some(ml_type.as_str()))
                        .await
                        .context("Cannot install modloader: no Java available")?;

                    info!(
                        "Installing modloader {} {} for server {}",
                        ml_type, ml_version, server_id.0
                    );

                    let launch_config = modloader_install::install_modloader(
                        &app.runtime_lifecycle,
                        &app.reqwest_client,
                        &server_path,
                        &game_version,
                        ml_type,
                        ml_version,
                        &java_path,
                        Some(t_install),
                    )
                    .await
                    .context(format!("Failed to install {} {}", ml_type, ml_version))?;

                    modloader_launch::save_launch_config(&server_path, &launch_config).await?;
                    info!(
                        "Modloader installed successfully for server {}",
                        server_id.0
                    );
                }

                Ok(())
            }
            .await;

            let failed_task = match install_result {
                Ok(()) => {
                    info!("Server {} created successfully", server_id.0);
                    drop(task);
                    None
                }
                Err(e) => {
                    error!("Failed to create server {}: {}", server_id.0, e);
                    task.fail(e).await;
                    Some(task_id)
                }
            };

            // Transition to Stopped
            if let Some(server_data) = app.server_manager.servers.write().await.get_mut(&server_id)
            {
                server_data.state = ServerState::Stopped { failed_task };
            }

            app.invalidate(GET_ALL_SERVERS, None);
            app.invalidate(GET_GROUPS, None);
            app.invalidate(GET_SERVER_DETAILS, None);
            _runtime_activity.finish();
        });

        Ok(server_id)
    }

    pub async fn create_server_from_modpack(
        self,
        group_id: ServerGroupId,
        name: String,
        modpack_source: modpack::ServerModpackSource,
        port: Option<i32>,
        icon_url: Option<String>,
    ) -> anyhow::Result<ServerId> {
        let runtime_activity = self
            .app
            .runtime_lifecycle
            .admit("server create_server_from_modpack")?;
        use crate::api::translation::Translation;
        use crate::managers::vtask::VisualTask;

        if name.is_empty() {
            bail!("Server name cannot be empty");
        }

        let requested_port = port;
        let port = resolve_server_port(port)?;
        let shortpath = generate_shortpath(&name);
        let runtime_path = &self.app.settings_manager().runtime_path;
        let servers_path = runtime_path.get_servers();
        let server_path = servers_path.get_server_path(&shortpath);

        // Create directory structure
        tokio::fs::create_dir_all(server_path.get_data_path())
            .await
            .context("Failed to create server directory")?;

        // Determine modpack metadata for DB
        let (modpack_platform, modpack_project_id, modpack_file_id) = match &modpack_source {
            modpack::ServerModpackSource::Curseforge {
                server_pack_file_id,
                project_id,
                ..
            } => (
                "curseforge".to_string(),
                project_id.to_string(),
                server_pack_file_id.to_string(),
            ),
            modpack::ServerModpackSource::Modrinth {
                project_id,
                version_id,
            } => (
                "modrinth".to_string(),
                project_id.clone(),
                version_id.clone(),
            ),
        };

        // Hold index_lock to make lookup+create atomic
        let _index_guard = self.index_lock.lock().await;

        // Top-of-group insertion: pick an index smaller than the current min
        let group_id_val = group_id.0;
        let min_index: Option<i32> =
            server_repo::min_index_server_in_group(&self.app.db, group_id_val)
                .await?
                .map(|s| s.index);
        let next_index = min_index.map(|n| n - 1).unwrap_or(0);

        // If placing in the default group, also bump library_position to
        // sort above all existing top-level items.
        let default_group_id = self.clone().get_default_group().await?;
        let library_position = if group_id == default_group_id {
            let default_id = default_group_id.0;
            let (min_server_pos, min_group_pos) = self
                .app
                .db
                .read(move |conn| {
                    // reads share one WAL snapshot
                    let snap = conn.snapshot()?;
                    let srv =
                        server_repo::min_library_position_server_in_group_conn(&snap, default_id)?
                            .and_then(|s| s.library_position);
                    let grp = server_repo::min_library_position_server_group_conn(&snap)?
                        .and_then(|g| g.library_position);
                    Ok((srv, grp))
                })
                .await?;

            let current_min = match (min_server_pos, min_group_pos) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            Some(current_min.map(|n| n - 1).unwrap_or(0))
        } else {
            None
        };

        // Create DB record with a placeholder game_version — will be updated after processing
        let (name_c, shortpath_c) = (name.clone(), shortpath.clone());
        let new_id = server_repo::insert_server(
            &self.app.db,
            name_c,
            shortpath_c,
            next_index,
            group_id_val,
            "unknown".to_string(),
            port,
            "modded".to_string(),
            None,
            None,
            Some(modpack_platform),
            Some(modpack_project_id),
            Some(modpack_file_id),
            library_position,
            DbDateTime(Utc::now().into()),
        )
        .await?;

        drop(_index_guard);

        let server_id = ServerId(new_id as i32);

        // Create a visual task for install progress
        let task = VisualTask::new(Translation::ServerTaskInstallFromModpack {
            server_name: name.clone(),
        });
        let task_id = self.app.task_manager().spawn_task(&task).await;

        // Register in memory as Installing
        self.servers.write().await.insert(
            server_id,
            ServerData {
                shortpath: shortpath.clone(),
                state: ServerState::Installing(task_id),
                handle: None,
                last_log_id: None,
            },
        );

        // Download and save the modpack icon before spawning (small thumbnail, won't block).
        // Reuse the instance downloader so the caller-supplied URL is scheme-checked
        // and the body is size-capped.
        if let Some(ref url) = icon_url {
            match self.app.instance_manager().download_icon(url.clone()).await {
                Ok((_, bytes)) => {
                    let icon_path = server_path.get_root().join("icon.png");
                    if let Err(e) = tokio::fs::write(&icon_path, &bytes).await {
                        warn!("Failed to write server icon: {}", e);
                    } else {
                        let sid = server_id.0;
                        let _ =
                            server_repo::set_server_icon_revision(&self.app.db, sid, Some(1)).await;
                    }
                }
                Err(e) => {
                    warn!("Failed to download server icon: {}", e);
                }
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);

        // Process the modpack in background
        Self::spawn_modpack_install(
            self.app.clone(),
            server_id,
            shortpath.clone(),
            port,
            requested_port,
            false,
            modpack_source,
            task,
            task_id,
            runtime_activity,
        );

        Ok(server_id)
    }

    /// Spawn the actual modpack install pipeline (download server pack, extract,
    /// download vanilla jar, install modloader, write properties, update DB).
    /// Shared between fresh-create and reinstall flows.
    fn spawn_modpack_install(
        app: Arc<crate::managers::AppInner>,
        server_id: ServerId,
        shortpath: String,
        port: i32,
        requested_port: Option<i32>,
        repair: bool,
        modpack_source: modpack::ServerModpackSource,
        task: crate::managers::vtask::VisualTask,
        task_id: VisualTaskId,
        mut runtime_activity: crate::managers::runtime_lifecycle::RuntimeActivity,
    ) {
        use crate::api::translation::Translation;

        runtime_activity.external();
        tokio::spawn(async move {
            let _runtime_activity = runtime_activity;
            let runtime_path = &app.settings_manager().runtime_path;
            let server_path = runtime_path.get_servers().get_server_path(&shortpath);

            // Create ALL subtasks upfront so the total weight is fixed from the start
            // and progress only moves forward. The process_* functions will create their
            // own internal subtasks for download/extract phases, but we pre-create the
            // jar download and modloader install subtasks here so their weight is already
            // accounted for. Skipped subtasks are marked complete with no work.
            let t_download_jar = task.subtask(Translation::ServerTaskDownloadServerJar);
            t_download_jar.set_weight(5.0);
            let t_install_modloader = task.subtask(Translation::ServerTaskInstallModloader);
            t_install_modloader.set_weight(5.0);

            task.edit(|data| data.state = crate::managers::vtask::TaskState::KnownProgress)
                .await;

            let result: anyhow::Result<()> = async {
                let pack_result = match modpack_source {
                    modpack::ServerModpackSource::Curseforge {
                        project_id,
                        server_pack_file_id,
                        ..
                    } => {
                        modpack::process_curseforge_server_pack(
                            &app,
                            &server_path,
                            project_id,
                            server_pack_file_id,
                            &task,
                        )
                        .await?
                    }
                    modpack::ServerModpackSource::Modrinth {
                        project_id,
                        version_id,
                    } => {
                        modpack::process_modrinth_server_pack(
                            &app,
                            &server_path,
                            &project_id,
                            &version_id,
                            &task,
                        )
                        .await?
                    }
                };

                // Download vanilla server jar only if not already present (CF server packs
                // usually bundle it, Modrinth mrpacks don't). Either way, complete the
                // pre-created subtask so its weight is accounted for.
                if !server_path.get_server_jar_path().exists() {
                    jars::download_vanilla_server_jar(
                        &app.reqwest_client,
                        &pack_result.game_version,
                        &server_path,
                        Some(&t_download_jar),
                    )
                    .await
                    .context("Failed to download vanilla server jar")?;
                } else {
                    t_download_jar.complete_opaque();
                }

                // Install the modloader if one was detected. Complete the
                // pre-created subtask either way to reach 100%.
                if let Some(ml_type) = &pack_result.modloader_type {
                    let ml_version = pack_result.modloader_version.as_deref();

                    // Most server packs arrive with the loader already unpacked.
                    // Reuse it rather than re-downloading and re-running the
                    // installer over the top of it.
                    let existing = modloader_install::existing_install_launch_config(
                        &server_path,
                        ml_type,
                        ml_version,
                    )
                    .await;

                    let launch_config = match existing {
                        Some(config) => {
                            info!(
                                "Server pack ships {} pre-installed, skipping installer",
                                ml_type
                            );
                            t_install_modloader.complete_opaque();
                            config
                        }
                        None => {
                            // Not pre-installed, so we have to run the installer —
                            // which needs an exact version. Failing here is much
                            // better than booting a vanilla server that modded
                            // clients cannot join.
                            let ml_version = ml_version.ok_or_else(|| {
                                anyhow!(
                                    "Server pack requires {} but ships neither an installed copy nor a version to install",
                                    ml_type
                                )
                            })?;

                            let java_path = app
                                .java_manager()
                                .find_java_for_server_version(
                                    &pack_result.game_version,
                                    Some(ml_type.as_str()),
                                )
                                .await
                                .context("Cannot install modloader: no Java available")?;

                            modloader_install::install_modloader(
                        &app.runtime_lifecycle,
                                &app.reqwest_client,
                                &server_path,
                                &pack_result.game_version,
                                ml_type,
                                ml_version,
                                &java_path,
                                Some(&t_install_modloader),
                            )
                            .await
                            .context(format!("Failed to install {} {}", ml_type, ml_version))?
                        }
                    };

                    modloader_launch::save_launch_config(&server_path, &launch_config).await?;
                } else {
                    t_install_modloader.complete_opaque();
                }

                let effective_port = properties::finalize_pack_properties(
                    &server_path.get_server_properties_path(), requested_port, port,
                    &pack_result.game_version, repair,
                ).await?;
                server_repo::set_server_game_version_and_modloader(
                    &app.db, server_id.0, &pack_result.game_version,
                    pack_result.modloader_type.as_deref(), pack_result.modloader_version.as_deref(),
                ).await?;
                app.db.write(move |conn| {
                    let mut patch = ServerPatch::default();
                    patch.port = Some(effective_port);
                    if let Some(query) = patch.build(server_id.0) { query.execute(&conn)?; }
                    Ok(())
                }).await?;

                Ok(())
            }
            .await;

            let failed_task = match result {
                Ok(()) => {
                    info!("Server modpack install completed: {}", server_id.0);
                    drop(task);
                    None
                }
                Err(e) => {
                    error!("Failed server modpack install: {}", e);
                    task.fail(e).await;
                    Some(task_id)
                }
            };

            // Transition to Stopped state
            if let Some(server_data) = app.server_manager.servers.write().await.get_mut(&server_id)
            {
                server_data.state = ServerState::Stopped { failed_task };
            }

            app.invalidate(GET_ALL_SERVERS, None);
            app.invalidate(GET_GROUPS, None);
            app.invalidate(GET_SERVER_DETAILS, None);
            _runtime_activity.finish();
        });
    }

    /// Reinstall a server from its original modpack. Wipes modpack-installed
    /// files (mods, libraries, server jars, modloader launch config) and
    /// re-runs the install pipeline. World saves, server.properties, eula.txt,
    /// whitelist/ops/banned-* files, and the configs/ directory are preserved.
    ///
    /// Refuses to run if the server is not in the Stopped state — wiping mods
    /// while a server is installing or running would corrupt the active task
    /// or the live process.
    pub async fn reinstall_server_from_modpack(self, id: ServerId) -> anyhow::Result<VisualTaskId> {
        let runtime_activity = self
            .app
            .runtime_lifecycle
            .admit("server reinstall_server_from_modpack")?;
        use crate::api::translation::Translation;
        use crate::managers::vtask::VisualTask;

        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        // State gate: only reinstall a server that's idle.
        {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            // A marker from an earlier session is checked again below.
            if !server.state.is_stopped_or_recovered() {
                bail!("Cannot reinstall while the server is running, installing, or being deleted");
            }
        }

        // Pull the modpack info we stored at create time.
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found in database"))?;

        let modpack_source = match (
            db_server.modpack_platform.as_deref(),
            db_server.modpack_project_id.as_deref(),
            db_server.modpack_file_id.as_deref(),
        ) {
            (Some("curseforge"), Some(project_id), Some(file_id)) => {
                modpack::ServerModpackSource::Curseforge {
                    project_id: project_id
                        .parse()
                        .context("invalid stored CurseForge project_id")?,
                    // file_id isn't used by the install pipeline (only
                    // server_pack_file_id is) so a placeholder is fine.
                    file_id: 0,
                    server_pack_file_id: file_id
                        .parse()
                        .context("invalid stored CurseForge server pack file_id")?,
                }
            }
            (Some("modrinth"), Some(project_id), Some(version_id)) => {
                modpack::ServerModpackSource::Modrinth {
                    project_id: project_id.to_string(),
                    version_id: version_id.to_string(),
                }
            }
            _ => bail!("Server does not have an associated modpack to reinstall"),
        };

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let data_path = server_path.get_data_path();

        self.refuse_unowned_marker(id, &server_path.get_root())
            .await?;

        // Wipe modpack-installed content. Anything not in this list (worlds,
        // server.properties, eula.txt, whitelist/ops/banned-*, logs) is left
        // alone so the operator's state survives a reinstall.
        let protection = {
            let data_path = data_path.clone();
            tokio::task::spawn_blocking(move || world::SaveProtection::read(&data_path)).await??
        };
        let dirs_to_wipe = ["mods", "libraries", "defaultconfigs"];
        let files_to_wipe = ["launch_config.json"];
        for name in dirs_to_wipe.into_iter().chain(files_to_wipe) {
            protection.refuse_wipe(name)?;
        }
        protection.refuse_wipe("launch_config.json")?;
        for name in dirs_to_wipe {
            let p = data_path.join(name);
            if p.exists() {
                tokio::fs::remove_dir_all(&p)
                    .await
                    .with_context(|| format!("Failed to wipe {}", p.display()))?;
            }
        }
        let files_to_wipe = ["launch_config.json"];
        for name in files_to_wipe {
            let p = data_path.join(name);
            if p.exists() {
                tokio::fs::remove_file(&p)
                    .await
                    .with_context(|| format!("Failed to wipe {}", p.display()))?;
            }
        }
        // Remove any .jar at the data root (server.jar, modloader installers,
        // forge/neoforge server jars). Keep recursing only at depth 0 — we
        // don't want to delete jars users dropped into mods/ etc, but mods/
        // was already wiped above.
        let mut entries = tokio::fs::read_dir(&data_path)
            .await
            .context("Failed to read server data dir")?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("jar"))
                    .unwrap_or(false)
            {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }

        // Spin up the install task and flip state to Installing.
        let task = VisualTask::new(Translation::ServerTaskInstallFromModpack {
            server_name: db_server.name.clone(),
        });
        let task_id = self.app.task_manager().spawn_task(&task).await;

        {
            let mut servers = self.servers.write().await;
            if let Some(s) = servers.get_mut(&id) {
                s.state = ServerState::Installing(task_id);
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Self::spawn_modpack_install(
            self.app.clone(),
            id,
            db_server.shortpath.clone(),
            db_server.port,
            None,
            true,
            modpack_source,
            task,
            task_id,
            runtime_activity,
        );

        Ok(task_id)
    }

    pub async fn delete_server(self, id: ServerId) -> anyhow::Result<()> {
        let _activity = self.app.runtime_lifecycle.admit_as(
            "server deletion",
            crate::managers::runtime_lifecycle::ActivityPolicy::Deletion,
        )?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        // Block on states where deleting now would race a background writer.
        // `Running` is handled below (stop it, then proceed) rather than
        // rejected here. `Deleting` is deliberately NOT rejected: it only
        // shows up on a server whose own earlier `delete_server` call already
        // deleted the DB row before failing (e.g. mid-`remove_dir_all`), and
        // rejecting it here would turn that failure into a permanently stuck
        // state instead of one the caller can retry by calling delete again.
        {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            match &server.state {
                ServerState::Stopped { .. }
                | ServerState::Running { .. }
                | ServerState::Deleting
                // `refuse_unowned_marker` below checks the marker again.
                | ServerState::RecoveredRunning { .. }
                | ServerState::RecoveredUnverified { .. } => {}
                ServerState::Installing(_) => {
                    bail!("Cannot delete a server while it is installing");
                }
                ServerState::Starting(_) => {
                    bail!("Cannot delete a server while it is starting");
                }
                ServerState::Stopping => {
                    bail!("Server is stopping — wait for it to fully stop before deleting");
                }
            }
        }

        let shortpath = self
            .servers
            .read()
            .await
            .get(&id)
            .context("Server disappeared")?
            .shortpath
            .clone();
        let root = self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(&shortpath)
            .get_root();
        self.refuse_unowned_marker(id, &root).await?;
        let handle = {
            let mut servers = self.servers.write().await;
            let server = servers.get_mut(&id).context("Server disappeared")?;
            server.state = ServerState::Deleting;
            server.handle.clone()
        };
        drop(_guard);
        if let Some(handle) = handle {
            provider::drain(&handle, false, std::time::Duration::from_secs(30)).await?;
            self.finalize_run(id, &handle).await?;
        }
        let _guard = lock.lock().await;
        if self
            .servers
            .read()
            .await
            .get(&id)
            .is_some_and(|s| s.handle.is_some() || !matches!(s.state, ServerState::Deleting))
        {
            bail!("Server ownership changed during deletion");
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        // Delete from DB, together with the server's cached mod rows: the
        // cascade only clears those while foreign keys are enforced.
        server_repo::delete_server_tx(&self.app.db, id.0).await?;

        // Delete files
        let shortpath = {
            let servers = self.servers.read().await;
            servers.get(&id).map(|s| s.shortpath.clone())
        };

        if let Some(shortpath) = shortpath {
            let runtime_path = &self.app.settings_manager().runtime_path;
            let server_path = runtime_path.get_servers().get_server_path(&shortpath);
            let root = server_path.get_root();
            if root.exists() {
                tokio::fs::remove_dir_all(&root)
                    .await
                    .context("Failed to delete server directory")?;
            }
        }

        // Remove from memory, including the server's log-broadcast channel
        // (keyed by its last log id) so it isn't leaked for the session.
        let removed = self.servers.write().await.remove(&id);
        if let Some(log_id) = removed.and_then(|server| server.last_log_id) {
            self.server_logs.write().await.remove(&log_id);
        }
        // Gates remain stable for all holders/waiters throughout the manager lifetime.
        self.crash_restart_state.remove(&id);
        self.crash_restart_abandoned.remove(&id);

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);

        Ok(())
    }

    /// Starts `id` manually. Always gets a fresh auto-restart cycle: past
    /// crash history — including a prior "gave up" state — no longer applies
    /// once the user has taken over, fulfilling the give-up log's "until it
    /// is started manually" promise. The auto-restart watcher's own retries
    /// call `start_server_impl` directly so a retry doesn't erase the streak
    /// it's the one accumulating.
    pub async fn start_server(self, id: ServerId) -> anyhow::Result<()> {
        self.start_server_impl(id, None).await
    }

    async fn start_server_impl(
        self,
        id: ServerId,
        restart: Option<ServerLogId>,
    ) -> anyhow::Result<()> {
        let runtime_activity = self
            .app
            .runtime_lifecycle
            .admit("server start_server_impl")?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        if restart.is_none() {
            self.crash_restart_state.remove(&id);
            self.crash_restart_abandoned.remove(&id);
        }
        // Check current state — block start while any non-stopped state is
        // active. In particular, `Installing` means a background task is
        // wiping/repopulating server files, so launching now would point at
        // an inconsistent data dir.
        {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            match &server.state {
                // A marker from an earlier session is checked again below.
                ServerState::Stopped { .. }
                | ServerState::RecoveredRunning { .. }
                | ServerState::RecoveredUnverified { .. } => {}
                ServerState::Running { .. } => {
                    bail!("Server is already running");
                }
                ServerState::Starting(_) => {
                    bail!("Server is already starting");
                }
                ServerState::Installing(_) => {
                    bail!("Server is currently installing — wait for install to finish");
                }
                ServerState::Stopping => {
                    bail!("Server is stopping — wait for it to fully stop before starting");
                }
                ServerState::Deleting => {
                    bail!("Server is being deleted");
                }
            }
        }

        // Get server info from DB
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found in database"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        if let Some(expected) = restart {
            if !self
                .restart_allowed(id, expected, db_server.auto_restart)
                .await
            {
                bail!("Automatic restart was cancelled or superseded");
            }
        }

        // Checked on every start, not only when the launcher starts: a
        // marker whose process has since ended, or whose PID another
        // process now holds, is removed here and the start goes ahead.
        check_marker(self.app, id, &server_path.get_root())
            .await?
            .refuse()?;

        // Check EULA acceptance
        let eula_path = server_path.get_eula_path();
        let eula_accepted = if eula_path.exists() {
            tokio::fs::read_to_string(&eula_path)
                .await
                .map(|content| content.contains("eula=true"))
                .unwrap_or(false)
        } else {
            false
        };

        if !eula_accepted {
            return Err(EulaNotAcceptedError { server_id: id.0 }.into());
        }

        // Find a Java matching the server's Minecraft version. Old versions
        // (Forge ≤1.16 in particular) hard-require Java 8 and crash on boot
        // with a newer one.
        let java_path = self
            .app
            .java_manager()
            .find_java_for_server_version(
                &db_server.game_version,
                db_server.modloader_type.as_deref(),
            )
            .await?;

        // Set up log streaming — clean up previous session's log entry first
        let log_id = {
            let mut counter = self.log_counter.lock().await;
            *counter += 1;
            ServerLogId(*counter)
        };

        {
            let servers = self.servers.read().await;
            if let Some(server) = servers.get(&id) {
                if let Some(old_log_id) = server.last_log_id {
                    self.server_logs.write().await.remove(&old_log_id);
                }
            }
        }

        let (log_watch_tx, _log_watch_rx) = watch::channel(Vec::new());
        self.server_logs
            .write()
            .await
            .insert(log_id, log_watch_tx.clone());

        let (log_tx, mut log_rx) = mpsc::unbounded_channel::<String>();

        let (log_drained_tx, log_drained) = provider::exit_signal();
        // Spawn log collector
        let log_watch = log_watch_tx.clone();
        tokio::spawn(async move {
            while let Some(line) = log_rx.recv().await {
                log_watch.send_modify(|logs| {
                    logs.push(line);
                    // Cap log buffer
                    if logs.len() > 10000 {
                        let drain = logs.len() - 5000;
                        logs.drain(..drain);
                    }
                });
            }
            let _ = log_drained_tx.send(true);
        });

        // Load modloader launch config, re-deriving a stale one that names
        // nothing to launch from what is actually installed.
        let launch_config = modloader_launch::resolve_launch_config(
            &server_path,
            db_server.modloader_type.as_deref(),
            db_server.modloader_version.as_deref(),
        )
        .await?;

        // Start server via provider
        let provider = self.get_provider();
        let mut handle = provider
            .start(
                &java_path,
                &server_path,
                db_server.xmx,
                db_server.xms,
                &db_server.extra_java_args,
                &launch_config,
                db_server.modloader_type.as_deref(),
                log_tx,
                runtime_activity,
            )
            .await?;

        handle.log_drained = Some(log_drained);
        let process_id = handle.process_id;

        let mut exited = handle.exited.clone();
        let watched_handle = handle.clone();

        // Update state
        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&id) {
                server.state = ServerState::Running {
                    start_time: Utc::now(),
                    log_id,
                    process_id,
                };
                server.handle = Some(handle);
                server.last_log_id = Some(log_id);
            }
        }

        // Update last_started in DB
        let _ = server_repo::set_server_last_started(
            &self.app.db,
            id.0,
            Some(DbDateTime(Utc::now().into())),
        )
        .await;

        // Pre-warm CPU metrics tracking so the first WebSocket poll gets non-zero values.
        // sysinfo needs two refresh calls to compute cpu_usage delta.
        self.app
            .system_info_manager()
            .get_process_metrics(process_id)
            .await;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        // Spawn a watcher for unexpected exits (crash/normal exit not triggered by stop/kill).
        // If auto_restart is enabled, restart the server automatically with a capped,
        // backing-off retry so a server that crashes instantly cannot tight-loop.
        let app = self.app.clone();
        tokio::spawn(async move {
            if exited.wait_confirmed().await.is_err() {
                return;
            }

            let uptime = {
                let servers = app.server_manager.servers.read().await;
                match servers.get(&id).map(|s| &s.state) {
                    Some(ServerState::Running { start_time, .. }) => Utc::now() - *start_time,
                    _ => chrono::Duration::zero(),
                }
            };
            let Ok(unexpected) = app.server_manager.finalize_run(id, &watched_handle).await else {
                return;
            };
            let should_restart = unexpected
                && server_repo::get_server(&app.db, id.0)
                    .await
                    .ok()
                    .flatten()
                    .is_some_and(|s| s.auto_restart);

            app.invalidate(GET_ALL_SERVERS, None);
            app.invalidate(GET_SERVER_DETAILS, None);

            if !should_restart {
                warn!("Server {} exited unexpectedly", id.0);
                return;
            }

            // A run lasting at least CRASH_RESTART_HEALTHY_UPTIME_SECS resets the
            // attempt count — this crash starts a fresh sequence rather than
            // continuing a tight loop.
            let healthy = uptime >= chrono::Duration::seconds(CRASH_RESTART_HEALTHY_UPTIME_SECS);
            let Some((attempts, abandoned)) = app
                .server_manager
                .record_owned_restart(id, log_id, healthy)
                .await
            else {
                return;
            };

            if abandoned {
                error!(
                    "Server {} crashed {} times in a row without staying up {}s; giving up on auto-restart until it is started manually",
                    id.0,
                    attempts - 1,
                    CRASH_RESTART_HEALTHY_UPTIME_SECS
                );
                // Surfaces `auto_restart_abandoned` to the frontend now,
                // rather than waiting for the next unrelated invalidation.
                app.invalidate(GET_ALL_SERVERS, None);
                app.invalidate(GET_SERVER_DETAILS, None);
                return;
            }

            let delay = crash_restart_delay(attempts);
            info!(
                "Server {} exited unexpectedly, auto-restarting in {:?} (attempt {}/{})",
                id.0, delay, attempts, CRASH_RESTART_MAX_ATTEMPTS
            );
            tokio::time::sleep(delay).await;
            // ManagerRef's future is not Send, so we use a oneshot to
            // bridge into a context where we can call start_server_impl.
            let (tx, rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
            let app2 = app.clone();
            // Captured here, on a runtime thread: the handle lives in a
            // thread-local that a freshly spawned OS thread does not inherit,
            // so resolving it inside the closure below would panic instead.
            let rt = tokio::runtime::Handle::current();
            // This inner task owns the Arc and can create a ManagerRef locally.
            // Calls `start_server_impl` directly (bypassing the public
            // `start_server`'s crash-state reset) — this retry is the one
            // accumulating the streak `record_crash_restart_attempt` tracks,
            // so it must not erase it on every attempt.
            std::thread::spawn(move || {
                let result = rt.block_on(app2.server_manager().start_server_impl(id, Some(log_id)));
                let _ = tx.send(result);
            });
            match rx.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!("Failed to auto-restart server {}: {}", id.0, e),
                Err(_) => error!("Auto-restart channel dropped for server {}", id.0),
            }
        });

        info!("Server {} started with PID {}", id.0, process_id);
        Ok(())
    }

    pub async fn stop_server(self, id: ServerId) -> anyhow::Result<()> {
        self.drain_server(id, false).await
    }
    pub async fn kill_server(self, id: ServerId) -> anyhow::Result<()> {
        self.drain_server(id, true).await
    }
    async fn drain_server(self, id: ServerId, force: bool) -> anyhow::Result<()> {
        let handle = {
            let lock = self.get_op_lock(id);
            let _guard = lock.lock().await;
            let mut servers = self.servers.write().await;
            let server = servers.get_mut(&id).context("Server not found")?;
            let handle = server.handle.clone().context("Server is not running")?;
            if !matches!(server.state, ServerState::Deleting) {
                server.state = ServerState::Stopping;
            }
            handle
        };
        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);
        provider::drain(&handle, force, std::time::Duration::from_secs(30)).await?;
        self.finalize_run(id, &handle).await?;
        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);
        Ok(())
    }

    /// Where server `id` keeps its files and marker.
    async fn server_root(self, id: ServerId) -> anyhow::Result<std::path::PathBuf> {
        let shortpath = self
            .servers
            .read()
            .await
            .get(&id)
            .context("Server not found")?
            .shortpath
            .clone();
        Ok(self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(&shortpath)
            .get_root())
    }

    /// Stops the JVM an earlier session left running for server `id`, after
    /// checking again that the marker still names it. On macOS and Linux
    /// the server saves and exits on SIGTERM and is killed only if it is
    /// still running after the same 30 s an ordinary Stop allows; on
    /// Windows it is ended right away, which the launcher has the user
    /// confirm. Once it is gone its marker and every record of it are
    /// removed, so it can be started, changed or deleted again.
    pub async fn stop_recovered_server(self, id: ServerId) -> anyhow::Result<()> {
        let root = self.server_root(id).await?;
        let lock = self.get_op_lock(id);
        let record = {
            let _guard = lock.lock().await;
            match self.servers.read().await.get(&id).map(|s| &s.state) {
                // The check below decides: the page may show an older state.
                Some(
                    ServerState::RecoveredRunning { .. } | ServerState::RecoveredUnverified { .. },
                ) => {}
                // Already stopped, from another window or on its own.
                Some(ServerState::Stopped { .. }) => return Ok(()),
                Some(_) => bail!("This server is not running from a previous session"),
                None => bail!("Server not found"),
            }
            let record = match check_marker(self.app, id, &root).await? {
                MarkerCheck::Clear => return Ok(()),
                MarkerCheck::Running(record) => record,
                check @ MarkerCheck::Unverified(_) => return check.refuse(),
            };
            if let Some(server) = self.servers.write().await.get_mut(&id) {
                server.state = ServerState::Stopping;
            }
            record
        };
        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);
        // Detached, so a dropped request never leaves the server Stopping:
        // the stop and the clean-up after it always run to the end.
        let app = self.app.clone();
        let stop = tokio::spawn(async move {
            let stopped =
                stop_recovered_process(&app.runtime_lifecycle, record, RECOVERED_STOP_GRACE).await;
            let lock = app.server_manager.op_lock(id);
            let _guard = lock.lock().await;
            if let Some(server) = app.server_manager.servers.write().await.get_mut(&id) {
                if server.handle.is_none() && matches!(server.state, ServerState::Stopping) {
                    server.state = ServerState::RecoveredRunning {
                        process_id: record.pid,
                    };
                }
            }
            // Watched again if the check below cannot settle it.
            app.server_manager().watch_recovered_servers();
            // Removes the marker once the process is gone, and shows
            // whatever the check finds either way.
            let checked = check_marker(&app, id, &root).await;
            app.invalidate(GET_ALL_SERVERS, None);
            app.invalidate(GET_SERVER_DETAILS, None);
            (stopped, checked)
        });
        let (stopped, checked) = stop
            .await
            .context("The stop of the recovered server failed")?;
        stopped?;
        checked?.refuse()
    }

    /// Removes a marker from an earlier session that cannot be verified,
    /// once the user has confirmed that no server from this folder is still
    /// running. Nothing is signalled. A marker that turns out to name this
    /// server's running JVM is not removed: that one has Stop instead.
    pub async fn clear_server_marker(self, id: ServerId) -> anyhow::Result<()> {
        let root = self.server_root(id).await?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;
        match self.servers.read().await.get(&id).map(|s| &s.state) {
            // The check below decides: the page may show an older state.
            Some(
                ServerState::RecoveredUnverified { .. } | ServerState::RecoveredRunning { .. },
            ) => {}
            Some(ServerState::Stopped { .. }) => return Ok(()),
            Some(_) => bail!("This server has no unverified record from a previous session"),
            None => bail!("Server not found"),
        }
        let record = match check_marker(self.app, id, &root).await? {
            MarkerCheck::Clear => return Ok(()),
            // A verified JVM is never cleared away: it has Stop instead.
            check @ MarkerCheck::Running(_) => return check.refuse(),
            MarkerCheck::Unverified(record) => record,
        };
        let marker = provider::pid_file_path(&root);
        let removed = match record {
            // Only the marker that was checked: never a newer one.
            Some(record) => provider::retire_recovered_marker(&root, record).await,
            None => orphan_pid::remove_pid_file_strict(&root, provider::PID_FILE_NAME)
                .await
                .map_err(anyhow::Error::from),
        };
        removed.with_context(|| {
            format!(
                "GDLauncher could not remove {}. Delete that file, then try again",
                marker.display()
            )
        })?;
        info!(
            "Cleared the unverified process marker {} of server {} at the user's request",
            marker.display(),
            id.0
        );
        self.app.runtime_lifecycle.release_marker(&marker, record);
        check_marker(self.app, id, &root).await?.refuse()
    }

    pub async fn accept_eula(self, id: ServerId) -> anyhow::Result<()> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        let eula_path = server_path.get_eula_path();
        tokio::fs::write(&eula_path, "eula=true\n")
            .await
            .context("Failed to write eula.txt")?;

        Ok(())
    }

    pub async fn send_console_command(self, id: ServerId, command: String) -> anyhow::Result<()> {
        let provider = self.get_provider();

        let servers = self.servers.read().await;
        let server = servers
            .get(&id)
            .ok_or_else(|| anyhow!("Server not found"))?;
        let handle = server
            .handle
            .as_ref()
            .ok_or_else(|| anyhow!("Server is not running"))?;

        provider.send_command(handle, &command).await
    }

    pub async fn server_details(self, id: ServerId) -> anyhow::Result<ServerDetails> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let state = {
            let servers = self.servers.read().await;
            servers
                .get(&id)
                .map(|s| s.state.clone())
                .unwrap_or(ServerState::Stopped { failed_task: None })
        };

        Ok(ServerDetails {
            id,
            name: db_server.name,
            favorite: db_server.favorite,
            server_type: ServerType::from_db_fields(
                &db_server.server_type,
                db_server.modloader_type.as_deref(),
            )
            .unwrap_or(ServerType::Vanilla),
            game_version: db_server.game_version,
            port: db_server.port,
            motd: db_server.motd,
            max_players: db_server.max_players,
            online_mode: db_server.online_mode,
            xmx: db_server.xmx,
            xms: db_server.xms,
            extra_java_args: db_server.extra_java_args,
            auto_restart: db_server.auto_restart,
            auto_restart_abandoned: self.is_auto_restart_abandoned(id),
            date_created: db_server.date_created.into(),
            last_started: db_server.last_started.map(|d| d.into()),
            state,
            icon_revision: db_server.icon_revision.map(|v| v as u32),
            modloader_type: db_server.modloader_type,
            modloader_version: db_server.modloader_version,
            modpack_info: db_server
                .modpack_platform
                .map(|platform| ServerModpackInfo {
                    platform,
                    project_id: db_server.modpack_project_id.unwrap_or_default(),
                    file_id: db_server.modpack_file_id.unwrap_or_default(),
                }),
        })
    }

    pub async fn update_server(self, update: ServerSettingsUpdate) -> anyhow::Result<()> {
        let lock = self.get_op_lock(update.server_id);
        let _guard = lock.lock().await;
        let mut patch = ServerPatch::default();

        if let Some(name) = update.name {
            patch.name = Some(name);
        }
        if let Some(xmx) = update.xmx {
            patch.xmx = Some(xmx);
        }
        if let Some(xms) = update.xms {
            patch.xms = Some(xms);
        }
        if let Some(extra_args) = update.extra_java_args {
            patch.extra_java_args = Some(extra_args.unwrap_or_default());
        }
        if let Some(auto_restart) = update.auto_restart {
            patch.auto_restart = Some(auto_restart);
        }

        let server_id = update.server_id.0;
        self.app
            .db
            .write(move |conn| {
                if let Some(q) = patch.build(server_id) {
                    q.execute(&conn)?;
                }
                Ok(())
            })
            .await?;

        if update.auto_restart == Some(false) {
            self.cancel_pending_restart(update.server_id).await;
        }
        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    /// Get all server.properties as a key-value map
    pub async fn get_server_properties(
        self,
        id: ServerId,
    ) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let props_path = server_path.get_server_properties_path();

        if props_path.exists() {
            properties::read_properties(&props_path).await
        } else {
            Ok(std::collections::BTreeMap::new())
        }
    }

    /// Update server.properties with the given key-value pairs
    pub async fn update_server_properties(
        self,
        id: ServerId,
        updates: std::collections::HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("server properties mutation")?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let props_path = server_path.get_server_properties_path();

        self.ensure_addon_publication(id, &server_path.get_root())
            .await?;

        let btree_updates: std::collections::BTreeMap<String, String> =
            updates.into_iter().collect();

        // Reject an out-of-range `server-port` before touching the DB or the
        // file — the same range `resolve_server_port` enforces at launch, so
        // a bad value is rejected here rather than surfacing later as an
        // opaque JVM bind error.
        if let Some(port) = btree_updates.get("server-port") {
            validate_server_port_patch(port)?;
        }
        if let Some(world) = btree_updates.get("level-name") {
            // As the server will read it; the world may be a link to a
            // folder elsewhere.
            if !properties::written_value(world)
                .is_some_and(|world| world::validate_local(&world).is_ok())
            {
                bail!(
                    "`{world}` cannot be the World Name: use a plain folder name inside the server folder."
                );
            }
        }

        if props_path.exists() {
            let existing = tokio::fs::read_to_string(&props_path).await?;
            let updated = properties::update_properties(&existing, &btree_updates);
            tokio::fs::write(&props_path, &updated).await?;
        } else {
            // Create new file, routed through the same sanitizer as the
            // update-existing-file path so a raw newline in a value can't
            // inject an extra property line.
            let content = properties::update_properties("", &btree_updates);
            tokio::fs::write(&props_path, &content).await?;
        }

        // Sync core fields back to DB for list display
        let mut patch = ServerPatch::default();
        if let Some(port) = btree_updates.get("server-port") {
            if let Ok(port) = port.parse::<i32>() {
                patch.port = Some(port);
            }
        }
        if let Some(motd) = btree_updates.get("motd") {
            patch.motd = Some(motd.clone());
        }
        if let Some(max_players) = btree_updates.get("max-players") {
            if let Ok(max_players) = max_players.parse::<i32>() {
                patch.max_players = Some(max_players);
            }
        }
        if let Some(online_mode) = btree_updates.get("online-mode") {
            patch.online_mode = Some(online_mode == "true");
        }

        self.app
            .db
            .write(move |conn| {
                if let Some(q) = patch.build(id.0) {
                    q.execute(&conn)?;
                }
                Ok(())
            })
            .await?;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    /// Read a player list JSON file (whitelist, ops, banned-players, banned-ips)
    pub async fn get_player_list<T: serde::de::DeserializeOwned>(
        self,
        id: ServerId,
        list: PlayerListFile,
    ) -> anyhow::Result<Vec<T>> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let file_path = server_path.get_data_path().join(list.filename());

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let entries: Vec<T> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", list.filename()))?;
        Ok(entries)
    }

    /// Resolve a Minecraft username to UUID via Mojang API
    pub async fn resolve_player_uuid(self, username: &str) -> anyhow::Result<(String, String)> {
        let url = format!(
            "https://api.mojang.com/users/profiles/minecraft/{}",
            username
        );
        let resp = self
            .app
            .reqwest_client
            .get(&url)
            .send()
            .await
            .context("Failed to query Mojang API")?;

        if !resp.status().is_success() {
            bail!("Player '{}' not found", username);
        }

        #[derive(serde::Deserialize)]
        struct MojangProfile {
            id: String,
            name: String,
        }

        let profile: MojangProfile = resp
            .json()
            .await
            .context("Failed to parse Mojang response")?;

        // Mojang returns UUID without dashes, convert to standard format
        let uuid = if profile.id.len() == 32 && !profile.id.contains('-') {
            format!(
                "{}-{}-{}-{}-{}",
                &profile.id[0..8],
                &profile.id[8..12],
                &profile.id[12..16],
                &profile.id[16..20],
                &profile.id[20..32]
            )
        } else {
            profile.id
        };

        Ok((uuid, profile.name))
    }

    /// Check if a server is currently running
    pub async fn is_server_running(self, id: ServerId) -> bool {
        let servers = self.servers.read().await;
        matches!(
            servers.get(&id).map(|s| &s.state),
            Some(ServerState::Running { .. })
        )
    }

    /// Send a console command if the server is running (best-effort)
    pub async fn send_console_if_running(self, id: ServerId, command: String) {
        if self.is_server_running(id).await {
            // Structured console commands are single-line; an operator-supplied
            // field (ban reason, IP) with an embedded newline would otherwise
            // inject additional console commands.
            let command = sanitize_console_command(&command);
            let _ = self.send_console_command(id, command).await;
        }
    }

    pub async fn legacy_datapacks(
        self,
        id: ServerId,
    ) -> anyhow::Result<Vec<world::LegacyDatapack>> {
        let gate = self.get_op_lock(id);
        let _guard = gate.lock().await;
        let server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .context("Server not found")?;
        let data = self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(&server.shortpath)
            .get_data_path();
        // Only an offer to move files: what cannot be read is left out, as
        // the addon list leaves it out, instead of failing every visit to
        // the Addons tab.
        let selected_world = match world::selected(&data) {
            Ok(world) => world,
            Err(error) => {
                tracing::warn!(?error, "Server {}: no datapack recovery offer", id.0);
                return Ok(vec![]);
            }
        };
        let mods = world::folder(&data, "mods")?;
        let mut entries = match tokio::fs::read_dir(&mods).await {
            Ok(entries) => entries,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(?error, "Server {}: cannot list {}", id.0, mods.display());
                }
                return Ok(vec![]);
            }
        };
        let mut candidates = Vec::new();
        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        "Server {}: stopped listing {}",
                        id.0,
                        mods.display()
                    );
                    break;
                }
            };
            let filename = entry.file_name().to_string_lossy().to_string();
            let base = filename.strip_suffix(".disabled").unwrap_or(&filename);
            if !base.to_ascii_lowercase().ends_with(".zip") {
                continue;
            }
            let path = entry.path();
            let sha512 = match world::legacy_datapack_evidence(&path) {
                Ok(true) => addons::digest(&path).await,
                Ok(false) => continue,
                Err(error) => Err(error),
            };
            match sha512 {
                Ok(sha512) => candidates.push(world::LegacyDatapack {
                    filename,
                    sha512: hex::encode(sha512),
                    selected_world: selected_world.clone(),
                }),
                Err(error) => {
                    tracing::warn!(?error, "Server {}: cannot read {}", id.0, path.display());
                }
            }
        }
        Ok(candidates)
    }
    pub async fn recover_legacy_datapack(
        self,
        id: ServerId,
        candidate: world::LegacyDatapack,
    ) -> anyhow::Result<()> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("legacy server datapack recovery")?;
        let gate = self.get_op_lock(id);
        let _guard = gate.lock().await;
        let server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .context("Server not found")?;
        let data = self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(&server.shortpath)
            .get_data_path();
        let root = self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(&server.shortpath)
            .get_root();
        self.ensure_addon_publication(id, &root).await?;
        world::validate_relative(&candidate.filename)?;
        if candidate.filename.contains('/') || world::selected(&data)? != candidate.selected_world {
            bail!("Recovery source or selected world changed; refresh candidates");
        }
        let source = world::folder(&data, &format!("mods/{}", candidate.filename))?;
        if !world::legacy_datapack_evidence(&source)?
            || hex::encode(addons::digest(&source).await?) != candidate.sha512
        {
            bail!("Recovery datapack changed; refresh candidates");
        }
        let folder = world::datapacks(&data)?;
        let base = candidate
            .filename
            .strip_suffix(".disabled")
            .unwrap_or(&candidate.filename);
        if folder.join(base).try_exists()?
            || folder.join(format!("{base}.disabled")).try_exists()?
        {
            bail!("Datapack destination is occupied");
        }
        tokio::fs::create_dir_all(&folder).await?;
        crate::util::publish::move_noclobber_async(&source, &folder.join(&candidate.filename))
            .await?;
        self.app
            .meta_cache_manager()
            .refresh_server_addon(
                id.0,
                &folder,
                base.into(),
                format!("{}/datapacks/{base}", candidate.selected_world),
                !candidate.filename.ends_with(".disabled"),
                "datapacks".into(),
            )
            .await?;
        self.app.invalidate(GET_SERVER_ADDONS, None);
        Ok(())
    }

    /// List server addons from database cache. Triggers caching if needed.
    pub async fn list_server_addons(self, id: ServerId) -> anyhow::Result<Vec<ServerAddon>> {
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Query from cache with metadata joins
        // Caching is handled by the queue system (triggered on install, startup, and tab navigation)
        let cached_mods = mfcdb::get_server_mods_full(&self.app.db, id.0).await?;

        let data = self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath)
            .get_data_path();
        // Like the addon scan, a level-name that cannot be used still lists the
        // mods; only the world's datapacks are left out until it is fixed.
        let active_world = match world::selected(&data) {
            Ok(world) => Some(world),
            Err(error) => {
                tracing::warn!(
                    ?error,
                    "Server {}: cannot use its level-name; listing mods only",
                    id.0
                );
                None
            }
        };
        let mut addons: Vec<ServerAddon> = cached_mods
            .into_iter()
            .filter(|entry| {
                (entry.addon_type == "mods" && entry.location == format!("mods/{}", entry.filename))
                    || active_world.as_ref().is_some_and(|world| {
                        entry.addon_type == "datapacks"
                            && entry.location == format!("{world}/datapacks/{}", entry.filename)
                    })
            })
            .map(|entry| {
                let display_name = entry.meta_name.clone().unwrap_or_else(|| {
                    entry
                        .filename
                        .trim_end_matches(".jar")
                        .trim_end_matches(".zip")
                        .to_string()
                });

                ServerAddon {
                    id: entry.id,
                    filename: entry.filename,
                    display_name,
                    enabled: entry.enabled,
                    addon_type: entry.addon_type,
                    file_size: entry.filesize,
                    has_image: entry.has_local_image || entry.has_cf_image || entry.has_mr_image,
                    curseforge_project_id: entry.cf_project_id.map(|c| c as u32),
                    modrinth_project_id: entry.mr_project_id,
                }
            })
            .collect();

        // Sort by filename for consistent ordering
        addons.sort_by(|a, b| a.filename.cmp(&b.filename));

        Ok(addons)
    }

    /// Enable or disable a server addon by renaming the file
    pub async fn enable_server_addon(
        self,
        id: ServerId,
        addon_id: String,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let _activity = self.app.runtime_lifecycle.admit("server addon mutation")?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Look up the cache entry to get the filename
        let lookup_id = addon_id.clone();
        let cache_entry = mfcdb::get_server_mod_file_cache_by_id(&self.app.db, &lookup_id)
            .await?
            .ok_or_else(|| anyhow!("Addon cache entry not found: {}", addon_id))?;

        if cache_entry.server_id != id.0 {
            bail!("Addon {} does not belong to server {}", addon_id, id.0);
        }

        let base_filename = &cache_entry.filename;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        self.ensure_addon_publication(id, &server_path.get_root())
            .await?;
        let file_path = world::owned_addon_entry(
            &server_path.get_data_path(),
            &cache_entry.addon_type,
            &cache_entry.location,
            base_filename,
        )?;

        let new_path = if enabled {
            let name = file_path
                .to_string_lossy()
                .trim_end_matches(".disabled")
                .to_string();
            std::path::PathBuf::from(name)
        } else {
            let mut name = file_path.to_string_lossy().to_string();
            if !name.ends_with(".disabled") {
                name.push_str(".disabled");
            }
            std::path::PathBuf::from(name)
        };

        if file_path != new_path {
            if new_path.try_exists()? {
                bail!("Addon destination already exists");
            }
            // A link is renamed itself; what it points to stays as it is.
            tokio::fs::rename(&file_path, &new_path).await?;
        }

        // Update cache entry's enabled state directly (no re-hash needed)
        mfcdb::update_server_mod_file_enabled(
            &self.app.db,
            &addon_id,
            enabled,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        self.app.invalidate(GET_SERVER_ADDONS, None);

        Ok(())
    }

    /// Delete a server addon file
    pub async fn delete_server_addon(self, id: ServerId, addon_id: String) -> anyhow::Result<()> {
        let _activity = self.app.runtime_lifecycle.admit("server addon mutation")?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Look up the cache entry to get the filename
        let lookup_id = addon_id.clone();
        let cache_entry = mfcdb::get_server_mod_file_cache_by_id(&self.app.db, &lookup_id)
            .await?
            .ok_or_else(|| anyhow!("Addon cache entry not found: {}", addon_id))?;

        if cache_entry.server_id != id.0 {
            bail!("Addon {} does not belong to server {}", addon_id, id.0);
        }

        let base_filename = &cache_entry.filename;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        self.ensure_addon_publication(id, &server_path.get_root())
            .await?;
        let file_path = world::owned_addon_entry(
            &server_path.get_data_path(),
            &cache_entry.addon_type,
            &cache_entry.location,
            base_filename,
        )?;

        // A link is removed itself, never the file it points to.
        tokio::fs::remove_file(&file_path).await?;

        // Remove cache entry
        mfcdb::delete_server_mod_file_cache_by_id(&self.app.db, &addon_id).await?;

        // GC orphaned metadata
        self.app.meta_cache_manager().gc_mod_metadata().await;

        self.app.invalidate(GET_SERVER_ADDONS, None);

        Ok(())
    }

    /// Install a CurseForge mod on a server by project_id + file_id
    pub async fn install_curseforge_mod(
        self,
        id: ServerId,
        project_id: u32,
        file_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        addons::install(
            &self.app,
            id,
            addons::Selection::Curseforge(project_id, Some(file_id)),
        )
        .await
    }

    /// Install the latest compatible CurseForge mod on a server
    pub async fn install_latest_curseforge_mod(
        self,
        id: ServerId,
        project_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        use carbon_platforms::curseforge::filters::{
            ModFilesParameters, ModFilesParametersQuery, ModParameters,
        };

        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let project = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod(ModParameters {
                mod_id: project_id as i32,
            })
            .await?;

        let game_version = db_server.game_version.clone();
        let modloader_type = curseforge_modloader_filter(
            project.data.class_id.as_ref(),
            db_server.modloader_type.as_deref(),
        );

        let files = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: project_id as i32,
                query: ModFilesParametersQuery {
                    game_version: Some(game_version.clone()),
                    game_version_type_id: None,
                    mod_loader_type: modloader_type,
                    index: None,
                    page_size: Some(200),
                },
            })
            .await?;

        let file = files
            .data
            .iter()
            .find(|f| f.game_versions.contains(&game_version))
            .ok_or_else(|| anyhow!("Can't find a compatible version for this server"))?;

        let file_id: u32 = file.id.try_into()?;
        self.install_curseforge_mod(id, project_id, file_id).await
    }

    /// Install a Modrinth mod on a server by project_id + version_id
    pub async fn install_modrinth_mod(
        self,
        id: ServerId,
        project_id: String,
        version_id: String,
    ) -> anyhow::Result<VisualTaskId> {
        addons::install(
            &self.app,
            id,
            addons::Selection::Modrinth(project_id, Some(version_id)),
        )
        .await
    }

    /// Install the latest compatible Modrinth mod on a server
    pub async fn install_latest_modrinth_mod(
        self,
        id: ServerId,
        project_id: String,
    ) -> anyhow::Result<VisualTaskId> {
        use carbon_platforms::modrinth::project::ProjectVersionsFilters;
        use carbon_platforms::modrinth::search::ProjectID;

        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let project = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project(ProjectID(project_id.clone()))
            .await?;

        let game_version = db_server.game_version.clone();
        let loaders =
            modrinth_loader_filter(&project.project_type, db_server.modloader_type.as_deref());

        let versions = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id.clone()),
                game_versions: Some(vec![game_version]),
                loaders,
                limit: None,
                offset: None,
            })
            .await?;

        let version = versions
            .first()
            .ok_or_else(|| anyhow!("Can't find a compatible version for this server"))?;

        let version_id = version.id.clone();
        self.install_modrinth_mod(id, project_id, version_id).await
    }

    /// Write a player list JSON file. A server still running from an
    /// earlier session refuses it, as it refuses a change of its
    /// properties: that JVM never hears of the edit (its console is gone)
    /// and later writes the list back from memory.
    pub async fn write_player_list<T: serde::Serialize>(
        self,
        id: ServerId,
        list: PlayerListFile,
        entries: &[T],
    ) -> anyhow::Result<()> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("server player list mutation")?;
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        self.refuse_unowned_marker(id, &server_path.get_root())
            .await?;
        let file_path = server_path.get_data_path().join(list.filename());

        let content = serde_json::to_string_pretty(entries)?;
        tokio::fs::write(&file_path, content).await?;
        Ok(())
    }

    pub async fn set_favorite(self, id: ServerId, favorite: bool) -> anyhow::Result<()> {
        server_repo::set_server_favorite(&self.app.db, id.0, favorite).await?;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    pub async fn get_server_metrics(
        self,
        id: ServerId,
    ) -> anyhow::Result<Option<server::ProcessMetrics>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(&id)
            .ok_or_else(|| anyhow!("Server not found"))?;

        match &server.state {
            ServerState::Running { process_id, .. } => Ok(self
                .app
                .system_info_manager()
                .get_process_metrics(*process_id)
                .await),
            _ => Ok(None),
        }
    }

    pub async fn get_server_log(
        self,
        id: ServerId,
    ) -> anyhow::Result<Option<watch::Receiver<Vec<String>>>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(&id)
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Prefer the live log_id while running, but fall back to the most
        // recent session's log_id when the server is stopped/crashed so the
        // UI keeps the post-mortem logs (and so a reconnecting WebSocket
        // doesn't get "Server not running" back-to-back with the crash).
        // The buffer is dropped on next start_server when last_log_id is
        // recycled, so this only persists logs from the latest run.
        let log_id = match &server.state {
            ServerState::Running { log_id, .. } => Some(*log_id),
            _ => server.last_log_id,
        };

        match log_id {
            Some(log_id) => {
                let logs = self.server_logs.read().await;
                Ok(logs.get(&log_id).map(|tx| tx.subscribe()))
            }
            None => Ok(None),
        }
    }

    pub async fn get_server_state(self, id: ServerId) -> anyhow::Result<ServerState> {
        let servers = self.servers.read().await;
        let server = servers
            .get(&id)
            .ok_or_else(|| anyhow!("Server not found"))?;
        Ok(server.state.clone())
    }

    pub async fn open_folder(self, id: ServerId) -> anyhow::Result<()> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found in database"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        let path = server_path.get_data_path();

        if !path.is_dir() {
            tokio::fs::create_dir_all(&path).await.with_context(|| {
                format!("Creating server folder at `{}`", path.to_string_lossy())
            })?;
        }

        opener::open(path)?;

        Ok(())
    }

    pub async fn set_server_icon(self, id: ServerId, base64_data: String) -> anyhow::Result<()> {
        use base64::Engine;

        const MAX_ICON_SIZE: usize = 8 * 1024 * 1024; // 8 MB
        const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

        let image_data = base64::engine::general_purpose::STANDARD
            .decode(&base64_data)
            .context("Invalid base64 image data")?;

        if image_data.len() > MAX_ICON_SIZE {
            bail!(
                "Icon is too large ({:.1} MB, max {} MB)",
                image_data.len() as f64 / (1024.0 * 1024.0),
                MAX_ICON_SIZE / (1024 * 1024)
            );
        }

        if !image_data.starts_with(PNG_SIGNATURE) {
            bail!("Icon must be a valid PNG image");
        }

        // Get the server's shortpath
        let shortpath = {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            server.shortpath.clone()
        };

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path.get_servers().get_server_path(&shortpath);
        let icon_path = server_path.get_root().join("icon.png");

        tokio::fs::write(&icon_path, &image_data)
            .await
            .context("Failed to write server icon")?;

        // Bump iconRevision
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found in database"))?;

        let new_revision = db_server.icon_revision.unwrap_or(0) + 1;

        server_repo::set_server_icon_revision(&self.app.db, id.0, Some(new_revision)).await?;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    pub async fn server_icon(self, id: ServerId) -> anyhow::Result<Option<(String, Vec<u8>)>> {
        let shortpath = {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            server.shortpath.clone()
        };

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path.get_servers().get_server_path(&shortpath);
        let icon_path = server_path.get_root().join("icon.png");

        if icon_path.exists() {
            let data = tokio::fs::read(&icon_path)
                .await
                .context("Failed to read server icon")?;
            Ok(Some(("icon.png".to_string(), data)))
        } else {
            Ok(None)
        }
    }

    pub async fn move_server(
        self,
        server_id: ServerId,
        target: ServerMoveTarget,
    ) -> anyhow::Result<()> {
        let _index_lock = self.index_lock.lock().await;

        let default_group_id = self.get_default_group().await?;

        let (start_group, start_idx, start_library_pos) = {
            let server = server_repo::get_server(&self.app.db, server_id.0)
                .await?
                .ok_or_else(|| anyhow!("Server not found in database"))?;

            (
                ServerGroupId(server.group_id),
                server.index,
                server.library_position,
            )
        };

        let (target_group, target_idx, target_library_pos) = match target {
            ServerMoveTarget::BeforeServer(target_id) => {
                let srv = server_repo::get_server(&self.app.db, target_id.0)
                    .await?
                    .ok_or_else(|| anyhow!("Target server not found in database"))?;

                (ServerGroupId(srv.group_id), srv.index, srv.library_position)
            }
            ServerMoveTarget::EndOfGroup(group) => {
                let group_val = group.0;
                let target_idx =
                    server_repo::count_servers_in_group(&self.app.db, group_val).await? as i32;

                let lib_pos = if group == default_group_id {
                    let (max_server_pos, max_group_pos) = self
                        .app
                        .db
                        .read(move |conn| {
                            // reads share one WAL snapshot
                            let snap = conn.snapshot()?;
                            let srv = server_repo::max_library_position_server_in_group_conn(
                                &snap, group_val,
                            )?
                            .and_then(|s| s.library_position);
                            let grp = server_repo::max_library_position_server_group_conn(&snap)?
                                .and_then(|g| g.library_position);
                            Ok((srv, grp))
                        })
                        .await?;

                    let max_pos = max_server_pos.unwrap_or(0).max(max_group_pos.unwrap_or(0));
                    Some(max_pos + 1)
                } else {
                    None
                };

                (group, target_idx, lib_pos)
            }
            ServerMoveTarget::BeforeGroup(group_id) => {
                let folder_id = group_id.0;
                let target_folder = server_repo::get_server_group(&self.app.db, folder_id)
                    .await?
                    .ok_or_else(|| anyhow!("Server group not found in database"))?;

                let lib_pos = target_folder
                    .library_position
                    .ok_or_else(|| anyhow!("Target folder has no libraryPosition"))?;

                let default_id = default_group_id.0;
                let target_idx =
                    server_repo::count_servers_in_group(&self.app.db, default_id).await? as i32;

                (default_group_id, target_idx, Some(lib_pos))
            }
        };

        let index_shifts: Vec<IndexShift> = if start_group == target_group {
            vec![match (start_idx, target_idx) {
                (start, target) if start < target => IndexShift::DownExclusive {
                    group_id: target_group.0,
                    gt: start,
                    lt: target,
                },
                (start, target) if start > target => IndexShift::UpRange {
                    group_id: target_group.0,
                    gte: target,
                    lt: start,
                },
                _ => return Ok(()),
            }]
        } else {
            vec![
                IndexShift::DownAfter {
                    group_id: start_group.0,
                    gt: start_idx,
                },
                IndexShift::UpFrom {
                    group_id: target_group.0,
                    gte: target_idx,
                },
            ]
        };

        let final_idx = if start_group == target_group && start_idx < target_idx {
            target_idx - 1
        } else {
            target_idx
        };

        let new_library_pos = if target_group == default_group_id {
            target_library_pos
        } else {
            None
        };

        let default_id = default_group_id.0;
        // If moving TO default group and inserting before an item, shift library positions
        if target_group == default_group_id {
            if let Some(target_lib_pos) = target_library_pos {
                if start_library_pos != Some(target_lib_pos) {
                    let sid = server_id.0;
                    // Two shifts run in one writer dispatch so no other write
                    // interleaves; they run in ONE transaction —
                    // all-or-nothing: a failure rolls the whole group back and readers
                    // never observe an intermediate state. `_conn` forms on the tx guard.
                    self.app
                        .db
                        .write(move |mut conn| {
                            let tx = conn.transaction()?;
                            server_repo::shift_server_library_positions_up_in_group_except_conn(
                                &tx,
                                default_id,
                                target_lib_pos,
                                sid,
                            )?;
                            server_repo::shift_all_server_group_library_positions_up_from_conn(
                                &tx,
                                target_lib_pos,
                            )?;
                            tx.commit()?;
                            Ok(())
                        })
                        .await?;
                }
            }
        }

        // If moving FROM default group, shift library positions to fill the gap
        if start_group == default_group_id && target_group != default_group_id {
            if let Some(start_lib_pos) = start_library_pos {
                // Two shifts run in one writer dispatch so no other write
                // interleaves; they run in ONE transaction —
                // all-or-nothing: a failure rolls the whole group back and readers
                // never observe an intermediate state. `_conn` forms on the tx guard.
                self.app
                    .db
                    .write(move |mut conn| {
                        let tx = conn.transaction()?;
                        server_repo::shift_server_library_positions_down_in_group_conn(
                            &tx,
                            default_id,
                            start_lib_pos,
                        )?;
                        server_repo::shift_all_server_group_library_positions_down_after_conn(
                            &tx,
                            start_lib_pos,
                        )?;
                        tx.commit()?;
                        Ok(())
                    })
                    .await?;
            }
        }

        let final_group = target_group.0;
        server_repo::move_server_tx(
            &self.app.db,
            index_shifts,
            server_id.0,
            final_group,
            final_idx,
            new_library_pos,
        )
        .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        // Auto-dissolve a non-default group left empty or with a single
        // server after moving one out: a one-server folder is pointless,
        // so its last server also returns to the default group.
        if start_group != default_group_id && start_group != target_group {
            let start_group_id = start_group.0;
            let remaining_count =
                server_repo::count_servers_in_group(&self.app.db, start_group_id).await?;

            if remaining_count == 0 {
                server_repo::delete_server_group(&self.app.db, start_group_id).await?;
                self.app.invalidate(GET_GROUPS, None);
            } else if remaining_count == 1 {
                if let Some(last) =
                    server_repo::first_server_in_group(&self.app.db, start_group_id).await?
                {
                    // Moving the last server out empties the group, which the
                    // recursive call's branch above then deletes. Release the
                    // index lock first since move_server re-acquires it.
                    drop(_index_lock);
                    Box::pin(self.move_server(
                        ServerId(last.id),
                        ServerMoveTarget::EndOfGroup(default_group_id),
                    ))
                    .await?;
                }
            }
        }

        Ok(())
    }

    pub async fn move_server_group(
        self,
        group: ServerGroupId,
        target: ServerGroupMoveTarget,
    ) -> anyhow::Result<()> {
        let _index_lock = self.index_lock.lock().await;

        let default_group_id = self.get_default_group().await?;

        let group_val = group.0;
        let moving_group = server_repo::get_server_group(&self.app.db, group_val)
            .await?
            .ok_or_else(|| anyhow!("Server group not found in database"))?;

        let start_pos = moving_group.library_position;

        let target_pos = match target {
            ServerGroupMoveTarget::BeforeGroup(target_group_id) => {
                let target_id = target_group_id.0;
                let target_group = server_repo::get_server_group(&self.app.db, target_id)
                    .await?
                    .ok_or_else(|| anyhow!("Target server group not found in database"))?;

                target_group.library_position.ok_or_else(|| {
                    anyhow!("Target group has no libraryPosition (is it the default group?)")
                })?
            }
            ServerGroupMoveTarget::BeforeServer(server_id) => {
                let server = server_repo::get_server(&self.app.db, server_id.0)
                    .await?
                    .ok_or_else(|| anyhow!("Server not found in database"))?;

                if server.group_id != default_group_id.0 {
                    bail!(
                        "Can only position a group before ungrouped servers (servers in default group)"
                    );
                }

                server
                    .library_position
                    .ok_or_else(|| anyhow!("Server has no libraryPosition"))?
            }
            ServerGroupMoveTarget::EndOfLibrary => {
                let default_id = default_group_id.0;
                let (max_server_pos, max_group_pos) = self
                    .app
                    .db
                    .read(move |conn| {
                        // reads share one WAL snapshot
                        let snap = conn.snapshot()?;
                        let srv = server_repo::max_library_position_server_in_group_conn(
                            &snap, default_id,
                        )?
                        .and_then(|s| s.library_position);
                        let grp = server_repo::max_library_position_server_group_conn(&snap)?
                            .and_then(|g| g.library_position);
                        Ok((srv, grp))
                    })
                    .await?;

                let max_pos = max_server_pos.unwrap_or(0).max(max_group_pos.unwrap_or(0));
                max_pos + 1
            }
        };

        let Some(start_pos) = start_pos else {
            bail!("Group has no libraryPosition - cannot move");
        };

        if start_pos == target_pos {
            return Ok(());
        }

        let default_id = default_group_id.0;
        if start_pos < target_pos {
            // Moving forward: shift items in (start, target] down by 1
            let target_upper = target_pos - 1;
            // Three writes run in one writer dispatch so no other write
            // interleaves; they run in ONE transaction —
            // all-or-nothing: a failure rolls the whole group back and readers
            // never observe an intermediate state. `_conn` forms on the tx guard.
            self.app
                .db
                .write(move |mut conn| {
                    let tx = conn.transaction()?;
                    server_repo::shift_server_group_library_positions_down_conn(
                        &tx,
                        start_pos,
                        target_upper,
                    )?;
                    server_repo::shift_server_library_positions_down_scoped_conn(
                        &tx,
                        default_id,
                        start_pos,
                        target_upper,
                    )?;
                    server_repo::set_server_group_library_position_conn(
                        &tx,
                        group_val,
                        Some(target_upper),
                    )?;
                    tx.commit()?;
                    Ok(())
                })
                .await?;
        } else {
            // Moving backward: shift items in [target, start) up by 1
            // Three writes run in one writer dispatch so no other write
            // interleaves; they run in ONE transaction —
            // all-or-nothing: a failure rolls the whole group back and readers
            // never observe an intermediate state. `_conn` forms on the tx guard.
            self.app
                .db
                .write(move |mut conn| {
                    let tx = conn.transaction()?;
                    server_repo::shift_server_group_library_positions_up_conn(
                        &tx, target_pos, start_pos,
                    )?;
                    server_repo::shift_server_library_positions_up_scoped_conn(
                        &tx, default_id, target_pos, start_pos,
                    )?;
                    server_repo::set_server_group_library_position_conn(
                        &tx,
                        group_val,
                        Some(target_pos),
                    )?;
                    tx.commit()?;
                    Ok(())
                })
                .await?;
        }

        // Keep groupIndex in sync
        let all_groups =
            server_repo::get_server_groups_with_library_position_ordered(&self.app.db).await?;

        // Interleaved app logic: restamp every group's index from the ordered
        // in-memory list. Runs in one writer dispatch, so no other write
        // interleaves; they run in ONE transaction —
        // all-or-nothing: a failure rolls the whole group back and readers
        // never observe an intermediate state. `_conn` forms on the tx guard.
        self.app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                for (idx, g) in all_groups.iter().enumerate() {
                    server_repo::set_server_group_index_conn(&tx, g.id, idx as i32)?;
                }
                tx.commit()?;
                Ok(())
            })
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);
        Ok(())
    }

    /// Generate a unique folder name by appending (1), (2), etc. if needed.
    async fn generate_unique_folder_name(&self, base_name: &str) -> anyhow::Result<String> {
        let base = base_name.to_string();
        let existing = server_repo::find_server_group_by_name(&self.app.db, &base).await?;

        if existing.is_none() {
            return Ok(base_name.to_string());
        }

        let mut counter = 1;
        loop {
            let candidate = format!("{} ({})", base_name, counter);
            let candidate_q = candidate.clone();
            let exists = server_repo::find_server_group_by_name(&self.app.db, &candidate_q).await?;

            if exists.is_none() {
                return Ok(candidate);
            }
            counter += 1;
        }
    }

    pub async fn create_server_group(self, name: String) -> anyhow::Result<ServerGroupId> {
        let group_count = server_repo::count_server_groups(&self.app.db).await? as i32;

        let default_group_id = self.get_default_group().await?;

        // Calculate next libraryPosition.
        let default_id = default_group_id.0;
        let (max_server_pos, max_group_pos) = self
            .app
            .db
            .read(move |conn| {
                // reads share one WAL snapshot
                let snap = conn.snapshot()?;
                let srv =
                    server_repo::max_library_position_server_in_group_conn(&snap, default_id)?
                        .and_then(|s| s.library_position);
                let grp = server_repo::max_library_position_server_group_conn(&snap)?
                    .and_then(|g| g.library_position);
                Ok((srv, grp))
            })
            .await?;

        let next_library_pos = max_server_pos.unwrap_or(0).max(max_group_pos.unwrap_or(0)) + 1;

        let group_id = server_repo::insert_server_group(
            &self.app.db,
            name,
            group_count,
            Some(next_library_pos),
        )
        .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(ServerGroupId(group_id as i32))
    }

    pub async fn create_server_group_at_position(
        self,
        name: String,
        target_position: i32,
    ) -> anyhow::Result<ServerGroupId> {
        let group_count = server_repo::count_server_groups(&self.app.db).await? as i32;

        let default_group_id = self.get_default_group().await?;

        // Shift all items with library_position >= target_position up by 1
        // (the server shift is scoped to the default group), then create the
        // group at the target position.
        let default_id = default_group_id.0;
        // Interleaved app logic: shift existing items up then insert the new
        // group at the freed position. Runs in one writer dispatch, so no other
        // write interleaves; they run in ONE transaction —
        // all-or-nothing: a failure rolls the whole group back and readers
        // never observe an intermediate state. `_conn` forms on the tx guard.
        let group_id = self
            .app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                server_repo::shift_server_library_positions_up_in_group_conn(
                    &tx,
                    default_id,
                    target_position,
                )?;
                server_repo::shift_all_server_group_library_positions_up_from_conn(
                    &tx,
                    target_position,
                )?;
                let id = server_repo::insert_server_group_conn(
                    &tx,
                    &name,
                    group_count,
                    Some(target_position),
                )?;
                tx.commit()?;
                Ok(id)
            })
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(ServerGroupId(group_id as i32))
    }

    pub async fn create_folder_from_servers(
        self,
        server_ids: Vec<ServerId>,
        target_server_id: Option<ServerId>,
    ) -> anyhow::Result<ServerGroupId> {
        if server_ids.is_empty() {
            bail!("Cannot create folder from empty list of servers");
        }

        let folder_name = self.generate_unique_folder_name("New Folder").await?;

        let target_library_pos = if let Some(target_id) = target_server_id {
            let default_group_id = self.get_default_group().await?;
            let target_server = server_repo::get_server(&self.app.db, target_id.0).await?;

            target_server
                .filter(|s| s.group_id == default_group_id.0)
                .and_then(|s| s.library_position)
        } else {
            None
        };

        let group_id = match target_library_pos {
            Some(pos) => {
                self.create_server_group_at_position(folder_name, pos)
                    .await?
            }
            None => self.create_server_group(folder_name).await?,
        };

        for sid in server_ids {
            self.move_server(sid, ServerMoveTarget::EndOfGroup(group_id))
                .await?;
        }

        Ok(group_id)
    }

    pub async fn arrange_server_library(self) -> anyhow::Result<()> {
        let default_group_id = self.get_default_group().await?;

        let _index_lock = self.index_lock.lock().await;

        // Get all servers in default group and sort by name
        let default_id = default_group_id.0;
        let servers = server_repo::get_servers_by_group(&self.app.db, default_id).await?;

        let mut sortable_servers: Vec<(i32, String)> =
            servers.iter().map(|s| (s.id, s.name.clone())).collect();
        sortable_servers.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        // Non-default server groups, sorted by name — rendered after
        // ungrouped servers in the library.
        let groups = server_repo::get_all_server_groups(&self.app.db).await?;

        let mut sortable_groups: Vec<(i32, String)> = groups
            .iter()
            .filter(|g| g.id != default_group_id.0)
            .map(|g| (g.id, g.name.clone()))
            .collect();
        sortable_groups.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        // Folders always come first, followed by ungrouped servers.
        // Frontend sorts top-level items by `libraryPosition ?? index`
        // (or `libraryPosition ?? 10000` for folders). Writing only
        // `index` leaves drag-reordered rows frozen. Stamp both fields so
        // the new order is visible whether library_position is set or not.
        let mut group_updates = Vec::new();
        group_updates.push(server_repo::ServerGroupArrange {
            id: default_group_id.0,
            group_index: 0,
            library_position: None,
            set_library_position: false,
        });
        for (i, (group_id, _)) in sortable_groups.iter().enumerate() {
            let p = i as i32;
            group_updates.push(server_repo::ServerGroupArrange {
                id: *group_id,
                group_index: (i + 1) as i32,
                library_position: Some(p),
                set_library_position: true,
            });
        }

        let server_base = sortable_groups.len() as i32;
        let mut server_updates = Vec::new();
        for (i, (server_id, _)) in sortable_servers.iter().enumerate() {
            let p = server_base + i as i32;
            server_updates.push(server_repo::ServerArrange {
                id: *server_id,
                index: p,
                library_position: Some(p),
            });
        }

        server_repo::arrange_server_library_tx(&self.app.db, group_updates, server_updates).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(())
    }

    pub async fn rename_server_group(
        self,
        group: ServerGroupId,
        name: String,
    ) -> anyhow::Result<()> {
        server_repo::set_server_group_name(&self.app.db, group.0, &name).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(())
    }

    pub async fn delete_server_group(self, group: ServerGroupId) -> anyhow::Result<()> {
        let _index_lock = self.index_lock.lock().await;

        let group_id = group.0;
        let any_servers = server_repo::count_servers_in_group(&self.app.db, group_id).await? != 0;

        if any_servers {
            let default_group = self.get_default_group().await?;

            // Server-side oddity (preserved verbatim): base_index counts the
            // DEFAULT group, not the group being deleted.
            let default_id = default_group.0;
            let base_index =
                server_repo::count_servers_in_group(&self.app.db, default_id).await? as i32;

            server_repo::delete_server_group_tx(&self.app.db, group_id, default_id, base_index)
                .await?;
        } else {
            server_repo::delete_server_group(&self.app.db, group_id).await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);
        Ok(())
    }
}

fn generate_shortpath(name: &str) -> String {
    let sanitized: String = name
        .graphemes(true)
        .filter(|g| {
            let c = g.chars().next().unwrap_or('_');
            !ILLEGAL_CHARS.contains(&c) && !c.is_control()
        })
        .collect::<String>()
        .trim()
        .to_string();

    let base = if sanitized.is_empty() {
        "server".to_string()
    } else if sanitized.graphemes(true).count() > 64 {
        sanitized.graphemes(true).take(64).collect::<String>()
    } else {
        sanitized
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("{}_{}", base, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The auto-restart watcher bridges into `start_server` by moving a runtime
    /// handle onto a dedicated OS thread, because the `ManagerRef` future is not
    /// `Send`. Resolving the handle on that thread instead would panic — tokio
    /// keeps it in a thread-local that `std::thread::spawn` does not inherit —
    /// and the panic surfaces only as a dropped oneshot, so the restart silently
    /// never happens.
    #[tokio::test]
    async fn runtime_handle_bridges_onto_a_plain_os_thread() {
        let (tx, rx) = tokio::sync::oneshot::channel::<u32>();
        let rt = tokio::runtime::Handle::current();

        std::thread::spawn(move || {
            // Drives a future that needs the runtime's timer driver, the way
            // `start_server` needs its IO driver.
            let result = rt.block_on(async {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                7
            });
            let _ = tx.send(result);
        });

        assert_eq!(
            rx.await.expect("bridge thread dropped the channel"),
            7,
            "the bridge must run the future rather than panicking"
        );
    }

    #[test]
    fn resolve_server_port_defaults_and_bounds() {
        assert_eq!(resolve_server_port(None).unwrap(), 25565);
        assert_eq!(resolve_server_port(Some(25577)).unwrap(), 25577);
        assert_eq!(resolve_server_port(Some(1)).unwrap(), 1);
        assert_eq!(resolve_server_port(Some(65535)).unwrap(), 65535);
        assert!(resolve_server_port(Some(0)).is_err());
        assert!(resolve_server_port(Some(-1)).is_err());
        assert!(resolve_server_port(Some(65536)).is_err());
    }

    #[test]
    fn validate_server_port_patch_rejects_out_of_range_and_non_numeric() {
        // A `server-port` patch outside the TCP range must be rejected before
        // `update_server_properties` ever reaches the DB/file write — not
        // surfaced later as an opaque JVM bind error.
        assert!(validate_server_port_patch("70000").is_err());
        assert!(validate_server_port_patch("0").is_err());
        assert!(validate_server_port_patch("-1").is_err());
        assert!(validate_server_port_patch("not-a-number").is_err());

        assert_eq!(validate_server_port_patch("25565").unwrap(), 25565);
        assert_eq!(validate_server_port_patch("1").unwrap(), 1);
        assert_eq!(validate_server_port_patch("65535").unwrap(), 65535);
    }

    #[test]
    fn sanitize_console_command_strips_control_chars() {
        assert_eq!(
            sanitize_console_command("ban Steve griefing"),
            "ban Steve griefing"
        );
        // A newline in the ban reason must not become a second console command.
        assert_eq!(
            sanitize_console_command("ban Steve reason\nop attacker"),
            "ban Steve reasonop attacker"
        );
        assert_eq!(
            sanitize_console_command("ban-ip 1.2.3.4 reason\r\nop attacker"),
            "ban-ip 1.2.3.4 reasonop attacker"
        );
    }

    #[test]
    fn server_addon_dir_matches_the_scan_by_extension() {
        let root = tempfile::tempdir().unwrap();
        let server_path = ServerPath::new(root.path().to_path_buf());

        // `cache_server_local` scans `mods/` for `*.jar` and `datapacks/` for
        // `*.zip`, so the destination has to match the extension or the
        // installed file is invisible to that scan — unlistable, unremovable.
        assert_eq!(
            server_addon_dir(&server_path, "cool-datapack.zip").unwrap(),
            server_path.get_datapacks_path()
        );
        assert_eq!(
            server_addon_dir(&server_path, "cool-mod.jar").unwrap(),
            server_path.get_mods_path()
        );

        // A datapack project's loader-packaged version is a `.jar`, so it lands
        // in `mods/` where a loader can find it. The project's type doesn't
        // enter into it — only what the scan will see does.
        assert_eq!(
            server_addon_dir(&server_path, "cool-datapack-fabric.jar").unwrap(),
            server_path.get_mods_path()
        );

        // A CurseForge/Modrinth-supplied filename's extension case is not
        // guaranteed — an uppercase `.ZIP` must still route to `datapacks/`,
        // matching the scan's now case-insensitive extension check.
        assert_eq!(
            server_addon_dir(&server_path, "Pack.ZIP").unwrap(),
            server_path.get_datapacks_path()
        );
        assert_eq!(
            server_addon_dir(&server_path, "Cool-Mod.JAR").unwrap(),
            server_path.get_mods_path()
        );
    }

    #[test]
    fn modloader_filter_narrows_mod_searches_only() {
        use carbon_platforms::curseforge::{ClassId, ModLoaderType};
        use carbon_platforms::modrinth::project::ProjectType;

        // A modded server narrows a mod search to its own loader.
        assert!(matches!(
            curseforge_modloader_filter(Some(&ClassId::Mods), Some("fabric")),
            Some(ModLoaderType::Fabric)
        ));
        assert_eq!(
            modrinth_loader_filter(&ProjectType::Mod, Some("fabric")),
            Some(vec!["fabric".to_string()])
        );

        // A datapack carries no loader tag on either platform, so keeping the
        // filter would match nothing and report the pack as having no version
        // compatible with the server.
        assert!(
            curseforge_modloader_filter(Some(&ClassId::Datapacks), Some("fabric")).is_none(),
            "the server's modloader must not narrow a datapack search"
        );
        assert_eq!(
            modrinth_loader_filter(&ProjectType::DataPack, Some("fabric")),
            None,
            "the server's modloader must not narrow a datapack search"
        );

        // A vanilla server has no loader to narrow by in the first place.
        assert!(curseforge_modloader_filter(Some(&ClassId::Mods), None).is_none());
        assert_eq!(modrinth_loader_filter(&ProjectType::Mod, None), None);
    }

    #[test]
    fn crash_restart_delay_grows_and_caps() {
        assert_eq!(crash_restart_delay(1), std::time::Duration::from_secs(3));
        assert_eq!(crash_restart_delay(2), std::time::Duration::from_secs(6));
        assert_eq!(crash_restart_delay(3), std::time::Duration::from_secs(12));
        assert_eq!(crash_restart_delay(4), std::time::Duration::from_secs(24));

        // Eventually reaches the configured ceiling (well past MAX_ATTEMPTS,
        // which is what actually stops the retries in practice — see
        // `crash_restart_delay_never_zero` for the overflow-safety guarantee).
        assert_eq!(
            crash_restart_delay(20),
            std::time::Duration::from_secs(CRASH_RESTART_MAX_DELAY_SECS)
        );

        // A pathologically large attempt count must not overflow the shift or
        // wrap the delay back down — it just stays at the ceiling.
        assert_eq!(
            crash_restart_delay(u32::MAX),
            std::time::Duration::from_secs(CRASH_RESTART_MAX_DELAY_SECS)
        );
    }

    #[test]
    fn crash_restart_delay_never_zero() {
        // attempts is always >= 1 in practice (incremented before use), but
        // guard the boundary explicitly: no delay would defeat the backoff.
        assert!(crash_restart_delay(1) > std::time::Duration::ZERO);
    }

    #[test]
    fn crash_restart_abandons_after_max_attempts_and_clears_on_manual_start() {
        let manager = ServerManager::new();
        let id = ServerId(1);

        // CRASH_RESTART_MAX_ATTEMPTS consecutive fast (unhealthy) crashes are
        // still within budget — auto-restart keeps retrying.
        for _ in 0..CRASH_RESTART_MAX_ATTEMPTS {
            let (_, abandoned) = manager.record_crash_restart_attempt(id, false);
            assert!(!abandoned);
        }
        assert!(!manager.is_auto_restart_abandoned(id));

        // One more fast crash past the cap — the seventh in a row — gives up
        // and marks the server abandoned.
        let (attempts, abandoned) = manager.record_crash_restart_attempt(id, false);
        assert_eq!(attempts, CRASH_RESTART_MAX_ATTEMPTS + 1);
        assert!(abandoned);
        assert!(manager.is_auto_restart_abandoned(id));

        // A manual start (the public `start_server`'s reset step) clears both
        // the streak and the abandoned flag, handing the server one fresh
        // retry cycle instead of staying permanently given up.
        manager.crash_restart_state.remove(&id);
        manager.crash_restart_abandoned.remove(&id);

        assert!(!manager.is_auto_restart_abandoned(id));
        let (attempts, abandoned) = manager.record_crash_restart_attempt(id, false);
        assert_eq!(attempts, 1);
        assert!(!abandoned);
    }

    // Pure decision logic (`reconcile_pid`, `is_live_java_process`,
    // `PidReconcileAction`) and pidfile read/write/remove lifecycle tests
    // now live in `managers::orphan_pid`, which both this manager and
    // `InstanceManager` share.

    #[test]
    fn task07_pack_install_ownership_blocks_addon_and_properties_publication() {
        // Refused with what the server is doing and the way out.
        assert_eq!(
            ensure_server_files_available(&ServerState::Installing(VisualTaskId(1)))
                .unwrap_err()
                .to_string(),
            "This server is still installing. Wait until it has finished installing, then try again."
        );
        assert_eq!(
            ensure_server_files_available(&ServerState::Deleting)
                .unwrap_err()
                .to_string(),
            "This server is being deleted."
        );
        assert_eq!(
            ensure_server_files_available(&ServerState::Stopping)
                .unwrap_err()
                .to_string(),
            "This server is still stopping. Wait until it has stopped, then try again."
        );
        ensure_server_files_available(&ServerState::Stopped { failed_task: None }).unwrap();
        ensure_server_files_available(&running_server_with_handle().state).unwrap();
    }

    #[tokio::test]
    async fn task07_old_finalizer_cannot_clear_replacement_or_deleting_state() {
        let manager = ServerManager::new();
        let old = running_server_with_handle().handle.unwrap();
        let replacement = running_server_with_handle();
        let replacement_generation = replacement.handle.as_ref().unwrap().generation;
        manager
            .servers
            .write()
            .await
            .insert(ServerId(1), replacement);
        assert!(!manager.finalize_run(ServerId(1), &old).await.unwrap());
        assert_eq!(
            manager.servers.read().await[&ServerId(1)]
                .handle
                .as_ref()
                .unwrap()
                .generation,
            replacement_generation
        );
        let current = manager.servers.read().await[&ServerId(1)]
            .handle
            .clone()
            .unwrap();
        manager
            .servers
            .write()
            .await
            .get_mut(&ServerId(1))
            .unwrap()
            .state = ServerState::Deleting;
        manager.finalize_run(ServerId(1), &current).await.unwrap();
        assert!(matches!(
            manager.servers.read().await[&ServerId(1)].state,
            ServerState::Deleting
        ));
    }
    #[tokio::test]
    async fn task07_pending_restart_toggle_off_on_remains_cancelled() {
        let manager = ServerManager::new();
        let mut data = running_server_with_handle();
        data.state = ServerState::Stopped { failed_task: None };
        data.handle = None;
        data.last_log_id = Some(ServerLogId(7));
        manager.servers.write().await.insert(ServerId(1), data);
        assert!(
            manager
                .restart_allowed(ServerId(1), ServerLogId(7), true)
                .await
        );
        assert!(
            !manager
                .restart_allowed(ServerId(1), ServerLogId(7), false)
                .await
        );
        manager.cancel_pending_restart(ServerId(1)).await;
        assert!(
            !manager
                .restart_allowed(ServerId(1), ServerLogId(7), true)
                .await
        );
        assert!(
            !manager
                .restart_allowed(ServerId(1), ServerLogId(8), true)
                .await
        );
    }

    // --- shutdown_running ---------------------------------------------

    #[tokio::test]
    async fn task07_shutdown_requests_stop_and_waits_for_confirmation() {
        let manager = ServerManager::new();
        let (kill_tx, mut kill_rx) = mpsc::channel(1);
        let (stdin_tx, mut stdin_rx) = mpsc::channel(1);
        let (exited_tx, exited) = provider::exit_signal();
        manager.servers.write().await.insert(
            ServerId(1),
            ServerData {
                shortpath: "fixture".into(),
                state: ServerState::Running {
                    start_time: Utc::now(),
                    log_id: ServerLogId(1),
                    process_id: 4242,
                },
                handle: Some(ServerHandle {
                    generation: uuid::Uuid::new_v4(),
                    log_drained: None,
                    marker_root: None,
                    runtime_activity: Default::default(),
                    process_id: 4242,
                    kill_tx,
                    stdin_tx,
                    exited,
                }),
                last_log_id: None,
            },
        );
        let mut drain = Box::pin(manager.shutdown_running());
        tokio::select! {
            _ = &mut drain => panic!("shutdown returned before the child confirmed exit"),
            command = stdin_rx.recv() => assert_eq!(command.as_deref(), Some("stop")),
        }
        assert!(kill_rx.try_recv().is_err());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut drain)
                .await
                .is_err()
        );
        exited_tx.send(true).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), drain)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn shutdown_running_ignores_stopped_servers_and_returns_promptly() {
        let manager = ServerManager::new();

        manager.servers.write().await.insert(
            ServerId(1),
            ServerData {
                shortpath: "test-server".to_string(),
                state: ServerState::Stopped { failed_task: None },
                handle: None,
                last_log_id: None,
            },
        );

        // Must return promptly (well within the 3s bound) when nothing is
        // running, and must not panic on a `None` handle.
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.shutdown_running(),
        )
        .await
        .expect("shutdown_running must not hang when no server is running");
    }

    #[tokio::test]
    async fn shutdown_running_with_no_servers_at_all_returns_promptly() {
        let manager = ServerManager::new();

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.shutdown_running(),
        )
        .await
        .expect("shutdown_running must not hang with an empty server map");
    }

    // --- wait_for_process_exit ----------------------------------------

    fn running_server_with_handle() -> ServerData {
        let (kill_tx, _kill_rx) = mpsc::channel::<()>(1);
        let (stdin_tx, _stdin_rx) = mpsc::channel::<String>(1);
        let (exited_tx, exited) = provider::exit_signal();
        // Leak the far ends so the channels stay open — and the process stays
        // un-exited — for the test's lifetime.
        std::mem::forget(_kill_rx);
        std::mem::forget(_stdin_rx);
        std::mem::forget(exited_tx);
        ServerData {
            shortpath: "test-server".to_string(),
            state: ServerState::Running {
                start_time: Utc::now(),
                log_id: ServerLogId(1),
                process_id: 4242,
            },
            handle: Some(ServerHandle {
                generation: uuid::Uuid::new_v4(),
                log_drained: None,
                marker_root: None,
                runtime_activity: Default::default(),
                process_id: 4242,
                kill_tx,
                stdin_tx,
                exited,
            }),
            last_log_id: None,
        }
    }

    #[tokio::test]
    async fn wait_for_process_exit_blocks_while_the_process_is_running() {
        let manager = ServerManager::new();
        manager
            .servers
            .write()
            .await
            .insert(ServerId(1), running_server_with_handle());

        // The handle is still present (the JVM hasn't exited). The whole point
        // of the fix is that the caller must NOT proceed to delete files yet, so
        // the wait must still be pending — the outer timeout must elapse.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            manager.wait_for_process_exit(ServerId(1), std::time::Duration::from_secs(30)),
        )
        .await;

        assert!(
            result.is_err(),
            "wait_for_process_exit returned while the process was still running"
        );
    }

    #[tokio::test]
    async fn wait_for_process_exit_missing_handle_is_unknown() {
        let manager = ServerManager::new();
        let mut stopped = running_server_with_handle();
        stopped.handle = None;
        stopped.state = ServerState::Stopped { failed_task: None };
        manager.servers.write().await.insert(ServerId(1), stopped);

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.wait_for_process_exit(ServerId(1), std::time::Duration::from_secs(30)),
        )
        .await
        .expect("missing evidence should return promptly");
        assert!(outcome.is_err(), "a vanished handle cannot prove exit");
    }

    #[tokio::test]
    async fn wait_for_process_exit_is_bounded_when_the_process_never_exits() {
        let manager = ServerManager::new();
        manager
            .servers
            .write()
            .await
            .insert(ServerId(1), running_server_with_handle());

        // The handle never clears; the call must still return, bounded by its
        // own timeout rather than blocking the delete forever.
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            manager.wait_for_process_exit(ServerId(1), std::time::Duration::from_millis(300)),
        )
        .await
        .expect("must be bounded by its own timeout");
    }

    /// A .gdl_server.pid left empty by a crash used to fail the startup
    /// ownership check, which refused every quit and update install for the
    /// session. Every server still loads, the one with the marker as
    /// unverified, the marker is kept for a move to refuse on, and quitting
    /// goes through.
    #[tokio::test]
    async fn an_empty_server_marker_still_loads_every_server_and_quit_commits() {
        use crate::managers::runtime_lifecycle::{RuntimeBlockerKind, ShutdownPurpose};
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let group = app.server_manager().get_default_group().await.unwrap();
        let servers_root = app.settings_manager().runtime_path.get_servers();
        let mut ids = vec![];
        for (index, shortpath) in ["crashed", "fine"].into_iter().enumerate() {
            let id = server_repo::insert_server(
                &app.db,
                shortpath.to_owned(),
                shortpath.to_owned(),
                index as i32,
                group.0,
                "1.20.1".into(),
                25565 + index as i32,
                "vanilla".into(),
                None,
                None,
                None,
                None,
                None,
                None,
                DbDateTime(Utc::now().into()),
            )
            .await
            .unwrap();
            ids.push(ServerId(id as i32));
            tokio::fs::create_dir_all(servers_root.get_server_path(shortpath).get_root())
                .await
                .unwrap();
        }
        let marker = provider::pid_file_path(&servers_root.get_server_path("crashed").get_root());
        tokio::fs::write(&marker, b"").await.unwrap();

        app.server_manager().load_servers().await.unwrap();

        {
            let servers = app.server_manager.servers.read().await;
            assert_eq!(
                servers.get(&ids[0]).map(|server| &server.state),
                Some(&ServerState::RecoveredUnverified { process_id: None }),
            );
            assert_eq!(
                servers.get(&ids[1]).map(|server| &server.state),
                Some(&ServerState::Stopped { failed_task: None }),
            );
        }
        assert_eq!(tokio::fs::read(&marker).await.unwrap(), b"");
        let refused = app.runtime_lifecycle.begin_relocation("move").unwrap_err();
        assert!(
            refused.blockers.iter().any(|blocker| {
                blocker.kind == RuntimeBlockerKind::UnreadableRecord
                    && blocker.name == marker.display().to_string()
            }),
            "{refused}"
        );

        let drain = app.clone();
        app.runtime_lifecycle
            .prepare_shutdown_and_commit("quit", ShutdownPurpose::Quit, async move {
                drain.server_manager().shutdown_running().await
            })
            .await
            .unwrap();
        assert!(app.runtime_lifecycle.shutdown_committed());
        assert_eq!(tokio::fs::read(&marker).await.unwrap(), b"");
    }

    /// A quit, restart or update that began during startup, before the
    /// server check got its turn, used to leave the server list empty for
    /// the rest of the session. The check is skipped, its markers stay, a
    /// move is refused, and every server is still listed.
    #[tokio::test]
    async fn servers_still_load_when_a_quit_began_before_their_check() {
        use crate::managers::runtime_lifecycle::{RuntimeBlockerKind, ShutdownPurpose};
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let group = app.server_manager().get_default_group().await.unwrap();
        let servers_root = app.settings_manager().runtime_path.get_servers();
        let id = ServerId(
            server_repo::insert_server(
                &app.db,
                "late".to_owned(),
                "late".to_owned(),
                0,
                group.0,
                "1.20.1".into(),
                25565,
                "vanilla".into(),
                None,
                None,
                None,
                None,
                None,
                None,
                DbDateTime(Utc::now().into()),
            )
            .await
            .unwrap() as i32,
        );
        let root = servers_root.get_server_path("late").get_root();
        tokio::fs::create_dir_all(&root).await.unwrap();
        let marker = provider::pid_file_path(&root);
        tokio::fs::write(&marker, b"4242\n1700000000")
            .await
            .unwrap();

        assert!(
            app.runtime_lifecycle
                .begin_shutdown("quit", ShutdownPurpose::Quit)
                .unwrap()
        );
        app.server_manager().load_servers().await.unwrap();

        assert!(matches!(
            app.server_manager
                .servers
                .read()
                .await
                .get(&id)
                .map(|server| &server.state),
            Some(ServerState::Stopped { .. })
        ));
        assert_eq!(tokio::fs::read(&marker).await.unwrap(), b"4242\n1700000000");
        assert!(
            app.runtime_lifecycle
                .relocation_blockers("move")
                .iter()
                .any(
                    |blocker| blocker.kind == RuntimeBlockerKind::StartupCheckFailed
                        && blocker.name.contains("server check was skipped")
                ),
            "a skipped check must keep a move refused"
        );
    }
}

/// Markers left by an earlier session: a stale one is removed as 2.0.40 did,
/// a server still running from that session can be stopped from the
/// launcher, and one that cannot be verified can be cleared, all without
/// restarting the launcher.
#[cfg(test)]
mod stale_marker_tests {
    use super::*;
    use crate::managers::runtime_lifecycle::{RuntimeBlocker, RuntimeBlockerKind};

    async fn server_fixture(
        app: &crate::managers::App,
        shortpath: &str,
    ) -> (ServerId, std::path::PathBuf) {
        let group = app.server_manager().get_default_group().await.unwrap();
        let id = server_repo::insert_server(
            &app.db,
            shortpath.to_owned(),
            shortpath.to_owned(),
            0,
            group.0,
            "1.20.1".into(),
            25565,
            "vanilla".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            DbDateTime(Utc::now().into()),
        )
        .await
        .unwrap();
        let root = app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(shortpath)
            .get_root();
        tokio::fs::create_dir_all(&root).await.unwrap();
        (ServerId(id as i32), root)
    }
    async fn state(app: &crate::managers::App, id: ServerId) -> ServerState {
        app.server_manager.servers.read().await[&id].state.clone()
    }
    /// Start got past every marker check and stopped at the EULA, the first
    /// thing a fixture server without eula.txt cannot pass.
    fn reached_eula_check(result: anyhow::Result<()>) -> bool {
        match result {
            Ok(()) => false,
            Err(error) => error.downcast_ref::<EulaNotAcceptedError>().is_some(),
        }
    }
    fn blockers(app: &crate::managers::App) -> Vec<RuntimeBlocker> {
        app.runtime_lifecycle.relocation_blockers("probe")
    }
    fn reconciled_copies(app: &crate::managers::App, pid: u32) -> usize {
        let writers = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join("__gdl_runtime_writers");
        std::fs::read_dir(writers)
            .map(|entries| {
                entries
                    .filter(|entry| {
                        entry
                            .as_ref()
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .starts_with(&format!("reconciled-{pid}-"))
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    /// A process the launcher takes for a server JVM: a copy of `sleep`
    /// named `java` (on Windows `PING.EXE` named `java.exe`, pinging the
    /// loopback once a second). A thread reaps it, as init reaps a real
    /// orphan.
    struct FakeJvm {
        pid: u32,
        reaper: Option<std::thread::JoinHandle<()>>,
        _dir: tempfile::TempDir,
    }
    impl FakeJvm {
        fn spawn() -> Self {
            let dir = tempfile::tempdir().unwrap();
            #[cfg(unix)]
            let command = {
                let java = dir.path().join("java");
                std::fs::copy(unix_sleep(), &java).unwrap();
                let mut command = std::process::Command::new(java);
                command.arg("600");
                command
            };
            #[cfg(windows)]
            let command = {
                let system_root =
                    std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
                let ping = std::path::Path::new(&system_root)
                    .join("System32")
                    .join("PING.EXE");
                let java = dir.path().join("java.exe");
                std::fs::copy(&ping, &java).unwrap();
                let mut command = std::process::Command::new(java);
                command.args(["-n", "600", "127.0.0.1"]);
                command
            };
            Self::start(dir, command)
        }
        /// Like [`FakeJvm::spawn`], but SIGTERM is ignored, as by a server
        /// that hangs while saving: only SIGKILL ends it. An ignored signal
        /// stays ignored across `exec`, so the shell hands that on to the
        /// copy named `java`.
        #[cfg(unix)]
        fn spawn_ignoring_sigterm() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let java = dir.path().join("java");
            std::fs::copy(unix_sleep(), &java).unwrap();
            for _ in 0..20 {
                let mut command = std::process::Command::new("/bin/sh");
                command
                    .arg("-c")
                    .arg("trap '' TERM; exec \"$0\" 600")
                    .arg(&java);
                let (pid, reaper) = Self::launch(command);
                // Until the shell has run `exec`, the process is `sh`.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while !reaper.is_finished() && std::time::Instant::now() < deadline {
                    if matches!(
                        orphan_pid::observe_process(pid),
                        orphan_pid::ProcessObservation::Present { process, .. } if process.is_java
                    ) {
                        return Self {
                            pid,
                            reaper: Some(reaper),
                            _dir: dir,
                        };
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                // `exec` hit ETXTBSY (see `launch`) and the shell gave up.
                drop(Self {
                    pid,
                    reaper: Some(reaper),
                    _dir: tempfile::tempdir().unwrap(),
                });
            }
            panic!("cannot start a fake JVM that ignores SIGTERM");
        }
        fn start(dir: tempfile::TempDir, command: std::process::Command) -> Self {
            let (pid, reaper) = Self::launch(command);
            // Until then it is not yet a Java process to recover. Windows
            // names a process when it creates it.
            #[cfg(unix)]
            orphan_pid::wait_until_named_java(pid);
            Self {
                pid,
                reaper: Some(reaper),
                _dir: dir,
            }
        }
        fn launch(mut command: std::process::Command) -> (u32, std::thread::JoinHandle<()>) {
            command
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            // Another test forking while the copy was open for writing can
            // briefly hold it: ETXTBSY.
            let mut attempts = 0;
            let mut child = loop {
                match command.spawn() {
                    Ok(child) => break child,
                    Err(error)
                        if cfg!(unix) && error.raw_os_error() == Some(26) && attempts < 50 =>
                    {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(error) => panic!("cannot start the fake JVM: {error}"),
                }
            };
            let pid = child.id();
            let reaper = std::thread::spawn(move || {
                let _ = child.wait();
            });
            (pid, reaper)
        }
        fn record(&self) -> orphan_pid::ProcessRecord {
            assert!(!self.exited(), "the fake JVM exited right away");
            orphan_pid::capture_process(self.pid).expect("the fake JVM is running")
        }
        /// Ends it from outside the launcher, as Task Manager would.
        fn end(&mut self) {
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(self.pid)]));
            if let Some(process) = system.process(Pid::from_u32(self.pid)) {
                process.kill();
            }
            if let Some(reaper) = self.reaper.take() {
                reaper.join().unwrap();
            }
        }
        fn exited(&self) -> bool {
            self.reaper
                .as_ref()
                .is_none_or(|reaper| reaper.is_finished())
        }
        /// Whether it exits within 10 s.
        async fn exits(&self) -> bool {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            while !self.exited() {
                if tokio::time::Instant::now() >= deadline {
                    return false;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            true
        }
    }
    impl Drop for FakeJvm {
        fn drop(&mut self) {
            self.end();
        }
    }
    #[cfg(unix)]
    fn unix_sleep() -> std::path::PathBuf {
        ["/bin/sleep", "/usr/bin/sleep"]
            .into_iter()
            .map(std::path::PathBuf::from)
            .find(|path| path.exists())
            .expect("a sleep binary")
    }
    /// The test binary's own record: a live process that is not Java, as
    /// one that took over a server JVM's PID would be. Signalling it would
    /// end this test run.
    fn own_record() -> orphan_pid::ProcessRecord {
        orphan_pid::capture_process(std::process::id()).expect("the test process is running")
    }
    fn refusal(error: &anyhow::Error) -> RecoveredServerError {
        *error
            .downcast_ref::<RecoveredServerError>()
            .unwrap_or_else(|| panic!("not a recovered server refusal: {error:#}"))
    }
    /// Keeps `watch_recovered_servers` from getting to a server before the
    /// action a test makes, so the test shows what that action does.
    fn hold_watcher(app: &crate::managers::App) {
        app.server_manager
            .recovered_poll_ms
            .store(60 * 60 * 1000, Ordering::Relaxed);
    }

    /// A 2.0.40 two-line marker whose PID a process other than Java holds
    /// now (here the test itself) was removed at startup by 2.0.40. It is
    /// again, and the server starts and deletes without a restart.
    #[tokio::test]
    async fn a_two_line_marker_on_a_reused_pid_is_removed_and_the_server_starts_and_deletes() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let (id, root) = server_fixture(&app, "reused").await;
        let marker = provider::pid_file_path(&root);
        let pid = std::process::id();
        tokio::fs::write(&marker, format!("{pid}\n1600000000"))
            .await
            .unwrap();

        app.server_manager().load_servers().await.unwrap();

        assert!(!marker.exists());
        assert_eq!(
            state(&app, id).await,
            ServerState::Stopped { failed_task: None }
        );
        assert!(
            !blockers(&app)
                .iter()
                .any(|blocker| blocker.pid == Some(pid)),
            "{:?}",
            blockers(&app)
        );
        assert!(reached_eula_check(
            app.server_manager().start_server(id).await
        ));
        app.server_manager().delete_server(id).await.unwrap();
        assert!(!root.exists());
    }

    /// The server JVM an earlier session left running is shown as running
    /// from that session instead of Stopped. Start and Delete say to stop it
    /// first; Stop checks it is still that JVM, stops it (SIGTERM on macOS
    /// and Linux, the Force stop kill on Windows), and removes its marker,
    /// the copy kept of it and the move blocker, so Start works without a
    /// restart.
    #[tokio::test]
    async fn a_server_left_running_is_recovered_and_stop_lets_it_start_again() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "orphaned").await;
        let jvm = FakeJvm::spawn();
        provider::write_process_file(&root, jvm.record()).await;
        let marker = provider::pid_file_path(&root);

        app.server_manager().load_servers().await.unwrap();

        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredRunning {
                process_id: jvm.pid
            }
        );
        let running = RecoveredServerError {
            kind: RecoveredMarkerKind::Running,
            pid: Some(jvm.pid),
        };
        let refused = app.server_manager().start_server(id).await.unwrap_err();
        assert_eq!(refusal(&refused), running);
        assert!(
            refused
                .to_string()
                .contains("still running from a previous session"),
            "{refused:#}"
        );
        assert!(!refused.to_string().contains("restart the launcher"));
        let refused = app.server_manager().delete_server(id).await.unwrap_err();
        assert_eq!(refusal(&refused), running);
        // A verified JVM's marker is never cleared away, only stopped.
        let refused = app
            .server_manager()
            .clear_server_marker(id)
            .await
            .unwrap_err();
        assert_eq!(refusal(&refused), running);
        assert!(root.exists() && marker.exists());
        assert!(!jvm.exited(), "nothing is signalled before the user asks");
        assert!(
            blockers(&app).iter().any(|blocker| {
                blocker.kind == RuntimeBlockerKind::PreviousProcess && blocker.pid == Some(jvm.pid)
            }),
            "a move still waits for it: {:?}",
            blockers(&app)
        );
        assert_eq!(reconciled_copies(&app, jvm.pid), 1);

        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            app.server_manager().stop_recovered_server(id),
        )
        .await
        .expect("the fake JVM ends at once on SIGTERM, and on Windows on the kill")
        .unwrap();

        assert!(jvm.exits().await);
        assert!(!marker.exists());
        assert_eq!(reconciled_copies(&app, jvm.pid), 0);
        assert_eq!(
            state(&app, id).await,
            ServerState::Stopped { failed_task: None }
        );
        assert!(
            !blockers(&app)
                .iter()
                .any(|blocker| blocker.pid == Some(jvm.pid)),
            "{:?}",
            blockers(&app)
        );
        assert!(reached_eula_check(
            app.server_manager().start_server(id).await
        ));
    }

    /// The JVM an earlier session left running exited after the launcher
    /// loaded it as running from that session, and before the user clicked
    /// Stop another process took its PID (here the test itself, with its
    /// real identity). Stop must leave that process alone: it finds the
    /// marker no longer names this server, removes it, and the server reads
    /// Stopped. Had Stop signalled the PID, this test run would have ended.
    #[tokio::test]
    async fn stop_leaves_alone_a_process_that_took_over_the_pid_after_load() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "taken-over").await;
        let mut jvm = FakeJvm::spawn();
        provider::write_process_file(&root, jvm.record()).await;
        let marker = provider::pid_file_path(&root);
        app.server_manager().load_servers().await.unwrap();
        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredRunning {
                process_id: jvm.pid
            }
        );

        jvm.end();
        let taken_over = own_record();
        provider::write_process_file(&root, taken_over).await;

        app.server_manager()
            .stop_recovered_server(id)
            .await
            .unwrap();

        assert!(!marker.exists());
        assert_eq!(
            state(&app, id).await,
            ServerState::Stopped { failed_task: None }
        );
        // The marker named the process that took over, so that one is what
        // Stop released. The ended JVM, which the startup check adopted, is
        // dropped by the refresh a move runs before it asks.
        app.runtime_lifecycle.refresh_adopted();
        assert!(
            !blockers(&app).iter().any(|blocker| {
                blocker.pid == Some(jvm.pid) || blocker.pid == Some(taken_over.pid)
            }),
            "{:?}",
            blockers(&app)
        );
    }

    /// The re-check right before each signal: a PID that no longer names
    /// the recovered JVM is reported gone and never signalled, even when
    /// the caller still holds the JVM's PID.
    #[tokio::test]
    async fn the_stop_of_a_recovered_process_never_signals_a_pid_another_process_holds() {
        let env = crate::setup_managers_for_test().await;
        stop_recovered_process(
            &env.app.runtime_lifecycle,
            own_record(),
            std::time::Duration::from_millis(100),
        )
        .await
        .unwrap();
        assert!(orphan_pid::capture_process(std::process::id()).is_some());
    }

    /// A recovered JVM that does not exit on SIGTERM (a save that hangs) is
    /// killed once the grace runs out: the same kill and kill confirmation
    /// Windows runs right away for Force stop.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_recovered_jvm_that_ignores_sigterm_is_killed_after_the_grace() {
        let env = crate::setup_managers_for_test().await;
        let jvm = FakeJvm::spawn_ignoring_sigterm();
        let grace = std::time::Duration::from_millis(500);
        let started = tokio::time::Instant::now();

        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            stop_recovered_process(&env.app.runtime_lifecycle, jvm.record(), grace),
        )
        .await
        .expect("the kill ends it")
        .unwrap();

        assert!(
            started.elapsed() >= grace,
            "it outlived SIGTERM, so only the kill after the grace ended it"
        );
        assert!(jvm.exits().await);
    }

    /// A server from an earlier session whose JVM exits on its own (it
    /// crashed, was stopped in-game or ended in Task Manager) reads Stopped
    /// again within a few seconds, with no action from the user: its marker,
    /// the copy kept of it and the move blocker go, and the watcher ends
    /// once no such server is left.
    #[tokio::test]
    async fn a_recovered_server_whose_jvm_exits_on_its_own_reads_stopped_by_itself() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        app.server_manager
            .recovered_poll_ms
            .store(50, Ordering::Relaxed);
        let (id, root) = server_fixture(&app, "exits").await;
        let mut jvm = FakeJvm::spawn();
        provider::write_process_file(&root, jvm.record()).await;
        let marker = provider::pid_file_path(&root);
        app.server_manager().load_servers().await.unwrap();
        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredRunning {
                process_id: jvm.pid
            }
        );
        assert!(app.server_manager.recovered_watcher.load(Ordering::SeqCst));
        // A live JVM keeps its state across checks.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredRunning {
                process_id: jvm.pid
            }
        );
        assert!(marker.exists());

        jvm.end();

        let settled = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while state(&app, id).await != (ServerState::Stopped { failed_task: None })
                || app.server_manager.recovered_watcher.load(Ordering::SeqCst)
            {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(
            settled.is_ok(),
            "still {:?}, watcher running: {}",
            state(&app, id).await,
            app.server_manager.recovered_watcher.load(Ordering::SeqCst)
        );
        assert!(!marker.exists());
        assert_eq!(reconciled_copies(&app, jvm.pid), 0);
        assert!(
            !blockers(&app)
                .iter()
                .any(|blocker| blocker.pid == Some(jvm.pid)),
            "{:?}",
            blockers(&app)
        );
    }

    /// A marker truncated by a power loss is kept byte for byte and refuses
    /// a move. The server shows it as unverified, and the user can clear it
    /// from the server page, after which the server starts and a move no
    /// longer lists it.
    #[tokio::test]
    async fn an_unreadable_marker_is_kept_until_the_user_clears_it() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let (id, root) = server_fixture(&app, "truncated").await;
        let marker = provider::pid_file_path(&root);
        let bytes = b"\0\0\0\0\0\0".to_vec();
        tokio::fs::write(&marker, &bytes).await.unwrap();
        let listed = |app: &crate::managers::App| {
            blockers(app).iter().any(|blocker| {
                blocker.kind == RuntimeBlockerKind::UnreadableRecord
                    && blocker.name == marker.display().to_string()
            })
        };

        app.server_manager().load_servers().await.unwrap();

        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredUnverified { process_id: None }
        );
        // Polled for being deleted, but left as it is meanwhile.
        assert!(app.server_manager.recovered_watcher.load(Ordering::SeqCst));
        assert_eq!(tokio::fs::read(&marker).await.unwrap(), bytes);
        assert!(listed(&app), "{:?}", blockers(&app));
        let unreadable = RecoveredServerError {
            kind: RecoveredMarkerKind::Unreadable,
            pid: None,
        };
        let refused = app.server_manager().start_server(id).await.unwrap_err();
        assert_eq!(refusal(&refused), unreadable);
        assert!(
            refused.to_string().contains("clear the record"),
            "{refused:#}"
        );
        // Stop has no verified process to signal and says what to do instead.
        let refused = app
            .server_manager()
            .stop_recovered_server(id)
            .await
            .unwrap_err();
        assert_eq!(refusal(&refused), unreadable);
        let refused = app.server_manager().delete_server(id).await.unwrap_err();
        assert_eq!(refusal(&refused), unreadable);
        assert!(root.exists());
        assert_eq!(tokio::fs::read(&marker).await.unwrap(), bytes);

        app.server_manager().clear_server_marker(id).await.unwrap();

        assert!(!marker.exists());
        assert_eq!(
            state(&app, id).await,
            ServerState::Stopped { failed_task: None }
        );
        assert!(!listed(&app), "{:?}", blockers(&app));
        assert!(reached_eula_check(
            app.server_manager().start_server(id).await
        ));
    }

    /// The move dialog tells the user to delete a record that cannot be
    /// read once nothing from that session runs. The server page follows
    /// within one check, without a Clear record warning about a process
    /// that is no longer recorded. While the file stays, its state and bytes
    /// stay too.
    #[tokio::test]
    async fn deleting_an_unreadable_marker_by_hand_reads_stopped_within_one_check() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        app.server_manager
            .recovered_poll_ms
            .store(50, Ordering::Relaxed);
        let (id, root) = server_fixture(&app, "deleted-by-hand").await;
        let marker = provider::pid_file_path(&root);
        let bytes = b"\0\0\0\0".to_vec();
        tokio::fs::write(&marker, &bytes).await.unwrap();

        app.server_manager().load_servers().await.unwrap();

        let unreadable = ServerState::RecoveredUnverified { process_id: None };
        assert_eq!(state(&app, id).await, unreadable);
        assert!(app.server_manager.recovered_watcher.load(Ordering::SeqCst));
        // Several checks later nothing has changed.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert_eq!(state(&app, id).await, unreadable);
        assert_eq!(tokio::fs::read(&marker).await.unwrap(), bytes);

        tokio::fs::remove_file(&marker).await.unwrap();

        let settled = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while state(&app, id).await != (ServerState::Stopped { failed_task: None })
                || app.server_manager.recovered_watcher.load(Ordering::SeqCst)
            {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(
            settled.is_ok(),
            "still {:?}, watcher running: {}",
            state(&app, id).await,
            app.server_manager.recovered_watcher.load(Ordering::SeqCst)
        );
        assert!(reached_eula_check(
            app.server_manager().start_server(id).await
        ));
    }

    /// Whitelist, op and ban edits on a server still running from an
    /// earlier session never reach that JVM, which later writes the lists
    /// back from memory. They are refused like a change of its properties,
    /// with the file left as it was, and go through once it has stopped.
    #[tokio::test]
    async fn player_list_edits_wait_until_a_server_from_an_earlier_session_has_stopped() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "players").await;
        let mut jvm = FakeJvm::spawn();
        provider::write_process_file(&root, jvm.record()).await;
        app.server_manager().load_servers().await.unwrap();
        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredRunning {
                process_id: jvm.pid
            }
        );
        let data = root.join("server");
        tokio::fs::create_dir_all(&data).await.unwrap();
        let banned = data.join(PlayerListFile::BannedPlayers.filename());
        let before = b"[]".to_vec();
        tokio::fs::write(&banned, &before).await.unwrap();
        let entry = [serde_json::json!({
            "uuid": "00000000-0000-0000-0000-000000000001",
            "name": "Griefer"
        })];

        let refused = app
            .server_manager()
            .write_player_list(id, PlayerListFile::BannedPlayers, &entry)
            .await
            .unwrap_err();

        assert_eq!(
            refusal(&refused),
            RecoveredServerError {
                kind: RecoveredMarkerKind::Running,
                pid: Some(jvm.pid),
            }
        );
        assert_eq!(tokio::fs::read(&banned).await.unwrap(), before);
        assert!(!jvm.exited(), "a refused edit signals nothing");

        jvm.end();

        app.server_manager()
            .write_player_list(id, PlayerListFile::BannedPlayers, &entry)
            .await
            .unwrap();
        let written: Vec<serde_json::Value> =
            serde_json::from_slice(&tokio::fs::read(&banned).await.unwrap()).unwrap();
        assert_eq!(written, entry);
        assert_eq!(
            state(&app, id).await,
            ServerState::Stopped { failed_task: None }
        );
    }

    /// The JVM from an earlier session ended on its own (or was ended in
    /// Task Manager) after the launcher started: the next Start finds that
    /// out and goes ahead, without a restart.
    #[tokio::test]
    async fn a_marker_whose_process_exits_mid_session_is_retired_on_the_next_start() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        hold_watcher(&app);
        let (id, root) = server_fixture(&app, "ended").await;
        let mut jvm = FakeJvm::spawn();
        provider::write_process_file(&root, jvm.record()).await;
        let marker = provider::pid_file_path(&root);
        app.server_manager().load_servers().await.unwrap();
        assert_eq!(
            state(&app, id).await,
            ServerState::RecoveredRunning {
                process_id: jvm.pid
            }
        );

        jvm.end();

        assert!(reached_eula_check(
            app.server_manager().start_server(id).await
        ));
        assert!(!marker.exists());
        assert_eq!(
            state(&app, id).await,
            ServerState::Stopped { failed_task: None }
        );
        assert_eq!(reconciled_copies(&app, jvm.pid), 0);
        assert!(
            !blockers(&app)
                .iter()
                .any(|blocker| blocker.pid == Some(jvm.pid)),
            "{:?}",
            blockers(&app)
        );
    }
}

/// Links are symlinks, and junctions on Windows.
#[cfg(test)]
mod link_repair_tests {
    use super::world::link_folder;
    use super::*;

    async fn modpack_server(
        app: &crate::TestEnv,
        shortpath: &str,
    ) -> (ServerId, std::path::PathBuf) {
        let group = app.server_manager().get_default_group().await.unwrap();
        let id = server_repo::insert_server(
            &app.db,
            shortpath.into(),
            shortpath.into(),
            0,
            group.0,
            "1.20.1".into(),
            25565,
            "modded".into(),
            Some("forge".into()),
            Some("47.2.0".into()),
            Some("modrinth".into()),
            Some("project".into()),
            Some("version".into()),
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap() as i32;
        let id = ServerId(id);
        app.server_manager.servers.write().await.insert(
            id,
            ServerData {
                shortpath: shortpath.into(),
                state: ServerState::Stopped { failed_task: None },
                handle: None,
                last_log_id: None,
            },
        );
        let data = app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(shortpath)
            .get_data_path();
        std::fs::create_dir_all(&data).unwrap();
        (id, data)
    }

    /// The World Name may be one the server saves escaped (`World \#2`) or a
    /// world linked from elsewhere; a `\` the server would drop, as in
    /// `worlds\survival`, is refused with what to use instead.
    #[tokio::test]
    async fn the_world_name_accepts_escapes_and_links_and_explains_a_refusal() {
        let app = crate::setup_managers_for_test().await;
        let (id, data) = modpack_server(&app, "world-name").await;
        let elsewhere = app
            .settings_manager()
            .runtime_path
            .to_path_buf()
            .join("worlds");
        std::fs::create_dir_all(elsewhere.join("main")).unwrap();
        link_folder(&elsewhere.join("main"), &data.join("linked"));
        let set = |value: &str| {
            let app = app.app.clone();
            let updates = HashMap::from([("level-name".to_owned(), value.to_owned())]);
            async move {
                app.server_manager()
                    .update_server_properties(id, updates)
                    .await
            }
        };
        // Outside Windows `:` is an ordinary character in a folder name.
        let colon = "SMP\\: Season 2";
        if cfg!(windows) {
            assert!(set(colon).await.is_err());
        }
        let others = (!cfg!(windows)).then_some(colon);
        for accepted in ["World \\#2"].into_iter().chain(others).chain(["linked"]) {
            set(accepted).await.unwrap();
            assert!(
                std::fs::read_to_string(data.join("server.properties"))
                    .unwrap()
                    .contains(&format!("level-name={accepted}")),
                "{accepted}"
            );
        }
        assert_eq!(world::selected(&data).unwrap(), "linked");
        let refused = set("worlds\\survival").await.unwrap_err().to_string();
        assert!(refused.contains("plain folder name"), "{refused}");
        assert_eq!(world::selected(&data).unwrap(), "linked");
    }

    /// Repair of a modpack server whose world is on another drive, whose
    /// `mods/` and one library are links to files shared with other servers,
    /// and with a stray link under backups goes ahead, as on 2.0.40: the
    /// linked `mods/` is removed as a link (the pack is installed into a new
    /// folder), and nothing any link points to is touched.
    #[tokio::test]
    async fn repair_goes_ahead_with_links_and_keeps_what_they_point_to() {
        let app = crate::setup_managers_for_test().await;
        let (id, data) = modpack_server(&app, "linked-repair").await;
        let elsewhere = app
            .settings_manager()
            .runtime_path
            .to_path_buf()
            .join("elsewhere");
        let write = |path: std::path::PathBuf, bytes: &[u8]| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        };
        write(elsewhere.join("world/level.dat"), b"save");
        write(elsewhere.join("shared-mods/shared.jar"), b"shared");
        write(elsewhere.join("shared-lib.jar"), b"lib");
        write(elsewhere.join("backup/level.dat"), b"backup");
        write(data.join("libraries/own.jar"), b"own");
        write(data.join("server.properties"), b"level-name=world\n");
        std::fs::create_dir_all(data.join("backups")).unwrap();
        link_folder(&elsewhere.join("world"), &data.join("world"));
        link_folder(&elsewhere.join("shared-mods"), &data.join("mods"));
        // A linked file needs rights on Windows.
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            elsewhere.join("shared-lib.jar"),
            data.join("libraries/shared-lib.jar"),
        )
        .unwrap();
        link_folder(
            &elsewhere.join("backup"),
            &data.join("backups").join("latest"),
        );

        let task = app.server_manager().reinstall_server_from_modpack(id).await;
        assert!(task.is_ok(), "{task:?}");

        assert!(
            std::fs::symlink_metadata(data.join("mods")).is_err(),
            "the linked mods folder is removed as a link"
        );
        assert!(std::fs::symlink_metadata(data.join("libraries")).is_err());
        for (kept, bytes) in [
            ("shared-mods/shared.jar", &b"shared"[..]),
            ("shared-lib.jar", b"lib"),
            ("world/level.dat", b"save"),
            ("backup/level.dat", b"backup"),
        ] {
            assert_eq!(
                std::fs::read(elsewhere.join(kept)).unwrap(),
                bytes,
                "{kept}"
            );
        }
        for link in ["world", "backups/latest"] {
            assert!(
                std::fs::symlink_metadata(data.join(link))
                    .unwrap()
                    .file_type()
                    .is_symlink(),
                "{link}"
            );
        }
    }
}
