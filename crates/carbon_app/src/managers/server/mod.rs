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
use anyhow::{Context, anyhow, bail};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::server::{self as server_repo, IndexShift, ServerPatch};
use carbon_rt_path::ServerPath;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tracing::{error, info, warn};
use unicode_segmentation::UnicodeSegmentation;

pub mod jars;
pub mod local;
pub mod modloader_install;
pub mod modloader_launch;
pub mod modpack;
pub mod properties;
pub mod provider;

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
/// A server reads `.jar` mods from `mods/` and `.zip` datapacks from
/// `world/datapacks/`, and `cache_server_local` scans for exactly those two
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
fn server_addon_dir(server_path: &ServerPath, filename: &str) -> std::path::PathBuf {
    if filename.ends_with(".zip") {
        server_path.get_datapacks_path()
    } else {
        server_path.get_mods_path()
    }
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
    server_logs: RwLock<HashMap<ServerLogId, watch::Sender<Vec<String>>>>,
    log_counter: Mutex<i32>,
    index_lock: Mutex<()>,
    /// Consecutive fast-crash count per server, used to back off and cap
    /// automatic restarts. See `crash_restart_delay`.
    crash_restart_state: DashMap<ServerId, u32>,
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
            server_logs: RwLock::new(HashMap::new()),
            log_counter: Mutex::new(0),
            index_lock: Mutex::new(()),
            crash_restart_state: DashMap::new(),
        }
    }

    fn get_provider(&self) -> Box<dyn ServerProvider> {
        Box::new(LocalServerProvider)
    }

    /// Best-effort graceful shutdown of every currently running (or
    /// starting) server, meant for the core process itself being terminated
    /// (SIGTERM/SIGINT/Ctrl+C) so servers get a `kill` signal instead of
    /// being silently orphaned. Bounded to `SHUTDOWN_TIMEOUT` for the whole
    /// operation — including a stalled kill — so a caller awaiting this can
    /// never hang past that; a timeout here is only logged; it does not
    /// change what the caller does next (main.rs exits the process either
    /// way, relying on `.kill_on_drop(true)` and the pidfile-based cleanup
    /// on next launch as the fallback for whatever didn't get killed).
    pub async fn shutdown_running(&self) {
        const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

        let outcome = tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
            let provider = self.get_provider();
            let servers = self.servers.read().await;

            let kills = servers.iter().filter_map(|(id, data)| {
                let is_live = matches!(
                    data.state,
                    ServerState::Running { .. } | ServerState::Starting(_)
                );
                match (is_live, &data.handle) {
                    (true, Some(handle)) => Some((*id, handle)),
                    _ => None,
                }
            });

            futures::future::join_all(kills.map(|(id, handle)| {
                let provider = &provider;
                async move {
                    if let Err(e) = provider.kill(handle).await {
                        warn!(
                            "Failed to signal shutdown to server {} (pid {}): {}",
                            id.0, handle.process_id, e
                        );
                    }
                }
            }))
            .await;
        })
        .await;

        if outcome.is_err() {
            warn!(
                "shutdown_running did not finish within {:?}; proceeding with core exit anyway",
                SHUTDOWN_TIMEOUT
            );
        }
    }

    /// Waits until the server's process handle has been cleared or `timeout`
    /// elapses. `stop_server_locked` clears the handle from its background task
    /// only once the JVM has actually exited (force-killing it after its own
    /// graceful budget), so a caller about to touch the server's files on disk
    /// uses this to avoid `remove_dir_all`ing a directory the JVM still holds
    /// open. Returns early the moment the handle is gone; the timeout is only a
    /// backstop so a stuck process can't wedge the caller forever.
    async fn wait_for_process_exit(&self, id: ServerId, timeout: std::time::Duration) {
        let start = std::time::Instant::now();

        loop {
            {
                let servers = self.servers.read().await;
                match servers.get(&id) {
                    Some(server) if server.handle.is_some() => {}
                    // Handle cleared (process exited) or the server is gone.
                    _ => return,
                }
            }

            if start.elapsed() >= timeout {
                warn!(
                    "Server {} did not exit within {}s; proceeding without waiting",
                    id.0,
                    timeout.as_secs()
                );
                return;
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }
}

impl ManagerRef<'_, ServerManager> {
    pub async fn launch_background_tasks(self) {
        if let Err(e) = self.load_servers().await {
            error!("Failed to load servers: {}", e);
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

        // Before any server is registered in memory as Stopped, reconcile
        // its pidfile against the live process table: a JVM from a session
        // the core didn't shut down cleanly (crash, force-quit, Windows
        // TerminateProcess) is otherwise invisible here and keeps holding
        // its port forever.
        self.clean_up_orphaned_servers(&db_servers).await;

        let mut servers = self.servers.write().await;
        for db_server in db_servers {
            servers.insert(
                ServerId(db_server.id),
                ServerData {
                    shortpath: db_server.shortpath,
                    state: ServerState::Stopped { failed_task: None },
                    handle: None,
                    last_log_id: None,
                },
            );
        }

        Ok(())
    }

    /// Reconcile every server's on-disk pidfile against the live process
    /// table, killing any pid that is still alive AND still looks like a
    /// java process (an orphaned JVM this core recorded but never cleaned
    /// up), and otherwise just discarding a stale/reused pid. Entirely
    /// best-effort: every failure is logged and swallowed so this can never
    /// fail or delay startup.
    async fn clean_up_orphaned_servers(self, db_servers: &[server_repo::ServerRow]) {
        let runtime_path = &self.app.settings_manager().runtime_path;
        let servers_path = runtime_path.get_servers();

        // Pass 1: read every server's pidfile (best-effort, one small file
        // read each) and collect the recorded pids up front, so the process
        // table only needs a single targeted refresh for this whole pass
        // instead of a full system scan per server.
        let mut recorded: Vec<(i32, std::path::PathBuf, Option<u32>)> =
            Vec::with_capacity(db_servers.len());
        for db_server in db_servers {
            let root = servers_path
                .get_server_path(&db_server.shortpath)
                .get_root();
            let pid = match provider::read_pid_file(&root).await {
                Ok(pid) => pid,
                Err(e) => {
                    warn!(
                        "Failed to read pidfile for server {} at {}: {}",
                        db_server.id,
                        root.display(),
                        e
                    );
                    None
                }
            };
            recorded.push((db_server.id, root, pid));
        }

        let pids: Vec<Pid> = recorded
            .iter()
            .filter_map(|(_, _, pid)| pid.map(Pid::from_u32))
            .collect();

        let mut system = System::new();
        if !pids.is_empty() {
            system.refresh_processes(ProcessesToUpdate::Some(&pids));
        }

        // Pass 2: reconcile. Every server with a recorded pid gets its
        // pidfile removed one way or another; only a pid sysinfo confirms
        // is still alive AND still java is killed first.
        for (server_id, root, pid) in recorded {
            let is_live_java = pid
                .map(|p| orphan_pid::is_live_java_process(&system, p))
                .unwrap_or(false);

            match orphan_pid::reconcile_pid(pid, is_live_java) {
                orphan_pid::PidReconcileAction::NoPidFile => {}
                orphan_pid::PidReconcileAction::RemoveStale => {
                    provider::remove_pid_file(&root).await;
                }
                orphan_pid::PidReconcileAction::StillRunning => {
                    // A server is launcher-hosted infrastructure and does not
                    // outlive the launcher, so a live one here is an orphan to
                    // clean up — the opposite of `InstanceManager`, which
                    // adopts the game it finds.
                    //
                    // Safe: StillRunning is only ever produced from `Some(pid)`.
                    let pid = pid.expect("StillRunning implies a recorded pid");
                    warn!(
                        "Server {} has an orphaned java process (pid {}) still running from a previous session — killing it",
                        server_id, pid
                    );
                    if let Some(process) = system.process(Pid::from_u32(pid)) {
                        if !process.kill() {
                            warn!("Failed to signal orphaned server process (pid {})", pid);
                        }
                    }
                    provider::remove_pid_file(&root).await;
                }
            }
        }
    }

    fn get_op_lock(self, id: ServerId) -> Arc<Mutex<()>> {
        self.server_op_locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
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
        tokio::spawn(async move {
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
        use crate::api::translation::Translation;
        use crate::managers::vtask::VisualTask;

        if name.is_empty() {
            bail!("Server name cannot be empty");
        }

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
            modpack_source,
            task,
            task_id,
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
        modpack_source: modpack::ServerModpackSource,
        task: crate::managers::vtask::VisualTask,
        task_id: VisualTaskId,
    ) {
        use crate::api::translation::Translation;

        tokio::spawn(async move {
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

                // Write server.properties if not present
                let props_path = server_path.get_server_properties_path();
                if !props_path.exists() {
                    let props = properties::generate_properties(
                        port,
                        "A Minecraft Server",
                        20,
                        true,
                        &pack_result.game_version,
                    );
                    properties::write_properties(&props_path, &props).await?;
                }

                // Update DB with detected versions
                let sid = server_id.0;
                let _ = server_repo::set_server_game_version_and_modloader(
                    &app.db,
                    sid,
                    &pack_result.game_version,
                    pack_result.modloader_type.as_deref(),
                    pack_result.modloader_version.as_deref(),
                )
                .await;

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
            if !matches!(server.state, ServerState::Stopped { .. }) {
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

        // Wipe modpack-installed content. Anything not in this list (worlds,
        // server.properties, eula.txt, whitelist/ops/banned-*, logs) is left
        // alone so the operator's state survives a reinstall.
        let dirs_to_wipe = ["mods", "libraries", "defaultconfigs"];
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
            modpack_source,
            task,
            task_id,
        );

        Ok(task_id)
    }

    pub async fn delete_server(self, id: ServerId) -> anyhow::Result<()> {
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
                | ServerState::Deleting => {}
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

        // Stop if running
        {
            let servers = self.servers.read().await;
            if let Some(server) = servers.get(&id) {
                if matches!(server.state, ServerState::Running { .. }) {
                    drop(servers);
                    // Already holding this server's op-lock, so the unlocked
                    // body must be called directly — going through
                    // `stop_server` would re-lock the same non-reentrant
                    // per-id `Mutex` and deadlock.
                    self.stop_server_locked(id).await?;
                    // The JVM holds the server directory's files open. Wait for
                    // it to actually exit before deleting on disk, rather than
                    // guessing with a fixed sleep — `stop_server_locked`
                    // force-kills after ~35s, so wait a little past that.
                    self.wait_for_process_exit(id, std::time::Duration::from_secs(45))
                        .await;
                }
            }
        }

        // Mark as deleting
        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&id) {
                server.state = ServerState::Deleting;
            }
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
        self.server_op_locks.remove(&id);
        self.crash_restart_state.remove(&id);

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);

        Ok(())
    }

    pub async fn start_server(self, id: ServerId) -> anyhow::Result<()> {
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

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
                ServerState::Stopped { .. } => {}
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
        let handle = provider
            .start(
                &java_path,
                &server_path,
                db_server.xmx,
                db_server.xms,
                &db_server.extra_java_args,
                &launch_config,
                db_server.modloader_type.as_deref(),
                log_tx,
            )
            .await?;

        let process_id = handle.process_id;

        let mut exited = handle.exited.clone();

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
            exited.wait().await;

            // Check if the exit was unexpected (state is still Running).
            // If stop_server/kill_server initiated the shutdown, they will have
            // already transitioned the state away from Running.
            let (should_restart, uptime) = {
                let mut servers = app.server_manager.servers.write().await;
                let Some(server) = servers.get_mut(&id) else {
                    return;
                };
                let start_time = match &server.state {
                    ServerState::Running { start_time, .. } => *start_time,
                    // stop_server or kill_server already handling cleanup
                    _ => return,
                };
                // Unexpected exit — clean up the handle
                server.handle = None;
                server.state = ServerState::Stopped { failed_task: None };

                // Check auto_restart setting from DB
                let auto_restart = server_repo::get_server(&app.db, id.0)
                    .await
                    .ok()
                    .flatten()
                    .map(|s| s.auto_restart)
                    .unwrap_or(false);

                (auto_restart, Utc::now() - start_time)
            };

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
            let attempts = {
                let mut entry = app
                    .server_manager
                    .crash_restart_state
                    .entry(id)
                    .or_insert(0);
                if healthy {
                    *entry = 0;
                }
                *entry += 1;
                *entry
            };

            if attempts > CRASH_RESTART_MAX_ATTEMPTS {
                error!(
                    "Server {} crashed {} times in a row without staying up {}s; giving up on auto-restart until it is started manually",
                    id.0,
                    attempts - 1,
                    CRASH_RESTART_HEALTHY_UPTIME_SECS
                );
                return;
            }

            let delay = crash_restart_delay(attempts);
            info!(
                "Server {} exited unexpectedly, auto-restarting in {:?} (attempt {}/{})",
                id.0, delay, attempts, CRASH_RESTART_MAX_ATTEMPTS
            );
            tokio::time::sleep(delay).await;
            // ManagerRef's future is not Send, so we use a oneshot to
            // bridge into a context where we can call start_server.
            let (tx, rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
            let app2 = app.clone();
            // Captured here, on a runtime thread: the handle lives in a
            // thread-local that a freshly spawned OS thread does not inherit,
            // so resolving it inside the closure below would panic instead.
            let rt = tokio::runtime::Handle::current();
            // This inner task owns the Arc and can create a ManagerRef locally
            std::thread::spawn(move || {
                let result = rt.block_on(app2.server_manager().start_server(id));
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
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        self.stop_server_locked(id).await
    }

    /// Body of `stop_server`, assuming the caller already holds `id`'s
    /// op-lock. `delete_server` holds that same lock across its own "stop if
    /// running" step and calls this directly — going through `stop_server`
    /// there would re-lock the non-reentrant per-id `Mutex` the caller is
    /// still holding and deadlock every time the server is `Running`.
    async fn stop_server_locked(self, id: ServerId) -> anyhow::Result<()> {
        let provider = self.get_provider();

        let mut exited = {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            let handle = server
                .handle
                .as_ref()
                .ok_or_else(|| anyhow!("Server is not running"))?;
            handle.exited.clone()
        };

        // Send stop command
        {
            let servers = self.servers.read().await;
            if let Some(server) = servers.get(&id) {
                if let Some(handle) = &server.handle {
                    provider.stop(handle).await?;
                }
            }
        }

        // Update state to Stopping
        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&id) {
                server.state = ServerState::Stopping;
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        // Wait for actual process exit in the background
        let app = self.app.clone();
        tokio::spawn(async move {
            const GRACEFUL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

            match tokio::time::timeout(GRACEFUL_TIMEOUT, exited.wait()).await {
                Ok(()) => {
                    info!("Server {} process exited gracefully", id.0);
                }
                Err(_) => {
                    warn!(
                        "Server {} did not stop within {}s, force killing",
                        id.0,
                        GRACEFUL_TIMEOUT.as_secs()
                    );
                    let servers = app.server_manager.servers.read().await;
                    if let Some(server) = servers.get(&id) {
                        if let Some(handle) = &server.handle {
                            let _ = handle.kill_tx.send(()).await;
                        }
                    }
                    drop(servers);
                    // Brief wait for the kill to complete
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), exited.wait())
                        .await;
                }
            }

            // Clean up state
            {
                let mut servers = app.server_manager.servers.write().await;
                if let Some(server) = servers.get_mut(&id) {
                    server.state = ServerState::Stopped { failed_task: None };
                    server.handle = None;
                }
            }

            app.invalidate(GET_ALL_SERVERS, None);
            app.invalidate(GET_SERVER_DETAILS, None);
            info!("Server {} stopped", id.0);
        });

        Ok(())
    }

    pub async fn kill_server(self, id: ServerId) -> anyhow::Result<()> {
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        let provider = self.get_provider();

        {
            let servers = self.servers.read().await;
            if let Some(server) = servers.get(&id) {
                if let Some(handle) = &server.handle {
                    provider.kill(handle).await?;
                }
            }
        }

        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&id) {
                server.state = ServerState::Stopped { failed_task: None };
                server.handle = None;
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
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
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let props_path = server_path.get_server_properties_path();

        let btree_updates: std::collections::BTreeMap<String, String> =
            updates.into_iter().collect();

        if props_path.exists() {
            let existing = tokio::fs::read_to_string(&props_path).await?;
            let updated = properties::update_properties(&existing, &btree_updates);
            tokio::fs::write(&props_path, &updated).await?;
        } else {
            // Create new file
            let mut content = String::from("#Minecraft server properties\n");
            for (key, value) in &btree_updates {
                content.push_str(&format!("{}={}\n", key, value));
            }
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

    /// List server addons from database cache. Triggers caching if needed.
    pub async fn list_server_addons(self, id: ServerId) -> anyhow::Result<Vec<ServerAddon>> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Query from cache with metadata joins
        // Caching is handled by the queue system (triggered on install, startup, and tab navigation)
        let cached_mods = mfcdb::get_server_mods_full(&self.app.db, id.0).await?;

        let mut addons: Vec<ServerAddon> = cached_mods
            .into_iter()
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

        // Find the actual file on disk (enabled or disabled variant)
        let disabled_name = format!("{}.disabled", base_filename);
        let dirs = [
            server_path.get_mods_path(),
            server_path.get_datapacks_path(),
        ];
        let mut file_path = None;
        for dir in &dirs {
            let enabled_path = dir.join(base_filename);
            let disabled_path = dir.join(&disabled_name);
            if enabled_path.exists() {
                file_path = Some(enabled_path);
                break;
            }
            if disabled_path.exists() {
                file_path = Some(disabled_path);
                break;
            }
        }
        let file_path =
            file_path.ok_or_else(|| anyhow!("Addon file not found: {}", base_filename))?;

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
            tokio::fs::rename(&file_path, &new_path).await?;
        }

        // Update cache entry's enabled state directly (no re-hash needed)
        let _ = mfcdb::update_server_mod_file_enabled(
            &self.app.db,
            &addon_id,
            enabled,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await;

        self.app.invalidate(GET_SERVER_ADDONS, None);

        Ok(())
    }

    /// Delete a server addon file
    pub async fn delete_server_addon(self, id: ServerId, addon_id: String) -> anyhow::Result<()> {
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

        // Find the actual file on disk
        let disabled_name = format!("{}.disabled", base_filename);
        let dirs = [
            server_path.get_mods_path(),
            server_path.get_datapacks_path(),
        ];
        let mut file_path = None;
        for dir in &dirs {
            let enabled_path = dir.join(base_filename);
            let disabled_path = dir.join(&disabled_name);
            if enabled_path.exists() {
                file_path = Some(enabled_path);
                break;
            }
            if disabled_path.exists() {
                file_path = Some(disabled_path);
                break;
            }
        }
        let file_path =
            file_path.ok_or_else(|| anyhow!("Addon file not found: {}", base_filename))?;

        tokio::fs::remove_file(&file_path).await?;

        // Remove cache entry
        let _ = mfcdb::delete_server_mod_file_cache_by_id(&self.app.db, &addon_id).await;

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
        use crate::api::translation::Translation;
        use crate::managers::vtask::VisualTask;
        use carbon_net::{Checksum, DownloadOptions, Downloadable};
        use carbon_platforms::curseforge::filters::ModFileParameters;

        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        // Fetch file info from CurseForge
        let file = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_file(ModFileParameters {
                mod_id: project_id as i32,
                file_id: file_id as i32,
            })
            .await?
            .data;

        let download_url = file
            .download_url
            .clone()
            .ok_or_else(|| anyhow!("Mod cannot be downloaded without privileged API key"))?;

        let install_dir = server_addon_dir(&server_path, &file.file_name);
        tokio::fs::create_dir_all(&install_dir).await?;
        // The filename comes from the platform response; confine it under the
        // addon directory so a `..`/absolute name can't write elsewhere.
        let install_path = secure_path_join(&install_dir, &file.file_name)?;

        let checksums = file
            .hashes
            .iter()
            .map(|hash| match hash.algo {
                carbon_platforms::curseforge::HashAlgo::Sha1 => Checksum::Sha1(hash.value.clone()),
                carbon_platforms::curseforge::HashAlgo::Md5 => Checksum::Md5(hash.value.clone()),
            })
            .collect::<Vec<_>>();

        let downloadable = Downloadable::new(&download_url, &install_path)
            .with_checksum(checksums.first().cloned())
            .with_size(file.file_length as u64);

        // Create visual task for progress
        let task = VisualTask::new(Translation::ServerTaskInstallMod {
            mod_name: file.display_name.clone(),
            server_name: db_server.name.clone(),
        });
        let task_id = self.app.task_manager().spawn_task(&task).await;

        let server_id_val = id.0;
        let app = self.app.clone();
        tokio::spawn(async move {
            let result = carbon_net::download_multiple(
                &[downloadable],
                DownloadOptions::builder().concurrency(1).build(),
            )
            .await;

            if let Err(e) = result {
                error!("Failed to download server mod: {e}");
            }

            // Queue caching for server addons after download
            app.meta_cache_manager()
                .queue_caching(
                    crate::managers::metadata::cache::CacheEntityId::Server(server_id_val),
                    true,
                )
                .await;

            app.invalidate(GET_SERVER_ADDONS, None);
            drop(task);
        });

        Ok(task_id)
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
        use crate::api::translation::Translation;
        use crate::managers::vtask::VisualTask;
        use carbon_net::{Checksum, DownloadOptions, Downloadable};
        use carbon_platforms::modrinth::search::VersionID;

        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        // Fetch version info from Modrinth
        let version = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_version(VersionID(version_id.clone()))
            .await?;

        let file = version
            .files
            .iter()
            .reduce(|a, b| if b.primary { b } else { a })
            .ok_or_else(|| anyhow!("Modrinth version has no files"))?;

        let install_dir = server_addon_dir(&server_path, &file.filename);
        tokio::fs::create_dir_all(&install_dir).await?;
        // The filename comes from the platform response; confine it under the
        // addon directory so a `..`/absolute name can't write elsewhere.
        let install_path = secure_path_join(&install_dir, &file.filename)?;

        let checksum = Checksum::Sha1(file.hashes.sha1.clone());

        let downloadable = Downloadable::new(&file.url, &install_path)
            .with_checksum(Some(checksum))
            .with_size(file.size as u64);

        // Create visual task for progress
        let task = VisualTask::new(Translation::ServerTaskInstallMod {
            mod_name: file.filename.clone(),
            server_name: db_server.name.clone(),
        });
        let task_id = self.app.task_manager().spawn_task(&task).await;

        let server_id_val = id.0;
        let app = self.app.clone();
        tokio::spawn(async move {
            let result = carbon_net::download_multiple(
                &[downloadable],
                DownloadOptions::builder().concurrency(1).build(),
            )
            .await;

            if let Err(e) = result {
                error!("Failed to download server mod: {e}");
            }

            // Queue caching for server addons after download
            app.meta_cache_manager()
                .queue_caching(
                    crate::managers::metadata::cache::CacheEntityId::Server(server_id_val),
                    true,
                )
                .await;

            app.invalidate(GET_SERVER_ADDONS, None);
            drop(task);
        });

        Ok(task_id)
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

    /// Write a player list JSON file
    pub async fn write_player_list<T: serde::Serialize>(
        self,
        id: ServerId,
        list: PlayerListFile,
        entries: &[T],
    ) -> anyhow::Result<()> {
        let db_server = server_repo::get_server(&self.app.db, id.0)
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
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
        let server_path = ServerPath::new(std::path::PathBuf::from("servers/test"));

        // `cache_server_local` scans `mods/` for `*.jar` and `datapacks/` for
        // `*.zip`, so the destination has to match the extension or the
        // installed file is invisible to that scan — unlistable, unremovable.
        assert_eq!(
            server_addon_dir(&server_path, "cool-datapack.zip"),
            server_path.get_datapacks_path()
        );
        assert_eq!(
            server_addon_dir(&server_path, "cool-mod.jar"),
            server_path.get_mods_path()
        );

        // A datapack project's loader-packaged version is a `.jar`, so it lands
        // in `mods/` where a loader can find it. The project's type doesn't
        // enter into it — only what the scan will see does.
        assert_eq!(
            server_addon_dir(&server_path, "cool-datapack-fabric.jar"),
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

    // Pure decision logic (`reconcile_pid`, `is_live_java_process`,
    // `PidReconcileAction`) and pidfile read/write/remove lifecycle tests
    // now live in `managers::orphan_pid`, which both this manager and
    // `InstanceManager` share.

    // --- shutdown_running ---------------------------------------------

    #[tokio::test]
    async fn shutdown_running_kills_a_running_servers_handle() {
        let manager = ServerManager::new();

        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
        let (stdin_tx, _stdin_rx) = mpsc::channel::<String>(1);
        let (exited_tx, exited) = provider::exit_signal();
        // Keep the process alive from the signal's point of view.
        std::mem::forget(exited_tx);
        let handle = ServerHandle {
            process_id: 4242,
            kill_tx,
            stdin_tx,
            exited,
        };

        manager.servers.write().await.insert(
            ServerId(1),
            ServerData {
                shortpath: "test-server".to_string(),
                state: ServerState::Running {
                    start_time: Utc::now(),
                    log_id: ServerLogId(1),
                    process_id: 4242,
                },
                handle: Some(handle),
                last_log_id: None,
            },
        );

        manager.shutdown_running().await;

        // `LocalServerProvider::kill` just forwards onto `kill_tx` —
        // receiving on it confirms `shutdown_running` found the running
        // server's handle and drove it through the same kill path
        // `kill_server` uses.
        assert!(
            kill_rx.try_recv().is_ok(),
            "expected shutdown_running to send a kill signal to the running server"
        );
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
    async fn wait_for_process_exit_returns_once_the_handle_is_cleared() {
        let manager = ServerManager::new();
        let mut stopped = running_server_with_handle();
        stopped.handle = None;
        stopped.state = ServerState::Stopped { failed_task: None };
        manager.servers.write().await.insert(ServerId(1), stopped);

        // Handle already cleared (process exited): must return promptly, not
        // block for the full timeout.
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.wait_for_process_exit(ServerId(1), std::time::Duration::from_secs(30)),
        )
        .await
        .expect("must return promptly once the handle is cleared");
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
}
