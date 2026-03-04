use self::local::LocalServerProvider;
use self::provider::{ServerHandle, ServerProvider};
use super::ManagerRef;
use crate::api::keys::server::*;
use crate::domain::server::{
    self, ServerConfig, ServerDetails, ServerGroupId, ServerId, ServerListEntry, ServerLogId,
    ServerSettingsUpdate, ServerState, ServerType,
};
use crate::domain::vtask::VisualTaskId;
use anyhow::{Context, anyhow, bail};
use carbon_repos::db::{self, read_filters::IntFilter};
use carbon_repos::pcr::Direction;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tracing::{error, info};
use unicode_segmentation::UnicodeSegmentation;

pub mod jars;
pub mod local;
pub mod properties;
pub mod provider;

const MAX_PATH: usize = if cfg!(windows) { 260 } else { 4096 };
const ILLEGAL_CHARS: &[char] = &['/', ':', '\\', '<', '>', '*', '|', '"', '?', '^'];

#[derive(Debug)]
pub struct ServerData {
    pub shortpath: String,
    pub state: ServerState,
    pub handle: Option<ServerHandle>,
}

pub struct ServerManager {
    pub(crate) servers: RwLock<HashMap<ServerId, ServerData>>,
    server_op_locks: Arc<DashMap<ServerId, Arc<Mutex<()>>>>,
    server_logs: RwLock<HashMap<ServerLogId, watch::Sender<Vec<String>>>>,
    log_counter: Mutex<i32>,
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
        }
    }

    fn get_provider(&self) -> Box<dyn ServerProvider> {
        Box::new(LocalServerProvider)
    }
}

impl ManagerRef<'_, ServerManager> {
    pub async fn launch_background_tasks(self) {
        if let Err(e) = self.load_servers().await {
            error!("Failed to load servers: {}", e);
        }
    }

    async fn load_servers(self) -> anyhow::Result<()> {
        let db_servers = self
            .app
            .prisma_client
            .server()
            .find_many(vec![])
            .exec()
            .await?;

        let mut servers = self.servers.write().await;
        for db_server in db_servers {
            servers.insert(
                ServerId(db_server.id),
                ServerData {
                    shortpath: db_server.shortpath,
                    state: ServerState::Stopped { failed_task: None },
                    handle: None,
                },
            );
        }

        Ok(())
    }

    fn get_op_lock(self, id: ServerId) -> Arc<Mutex<()>> {
        self.server_op_locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub async fn get_default_group(self) -> anyhow::Result<ServerGroupId> {
        let config = self
            .app
            .prisma_client
            .app_configuration()
            .find_unique(db::app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("App configuration not found"))?;

        match config.default_server_group {
            Some(id) => Ok(ServerGroupId(id)),
            None => {
                // Find the first group
                let group = self
                    .app
                    .prisma_client
                    .server_group()
                    .find_first(vec![])
                    .order_by(db::server_group::OrderByParam::GroupIndex(Direction::Asc))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("No server group found"))?;

                Ok(ServerGroupId(group.id))
            }
        }
    }

    pub async fn list_groups(self) -> anyhow::Result<Vec<server::ServerGroup>> {
        let groups = self
            .app
            .prisma_client
            .server_group()
            .find_many(vec![])
            .order_by(db::server_group::OrderByParam::GroupIndex(Direction::Asc))
            .with(
                db::server_group::servers::fetch(vec![])
                    .order_by(db::server::OrderByParam::Index(Direction::Asc)),
            )
            .exec()
            .await?;

        let active_servers = self.servers.read().await;

        Ok(groups
            .into_iter()
            .map(|group| server::ServerGroup {
                id: ServerGroupId(group.id),
                name: group.name,
                group_index: group.group_index,
                library_position: group.library_position,
                servers: group
                    .servers
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
                        server_type: ServerType::from_db_string(&s.server_type)
                            .unwrap_or(ServerType::Vanilla),
                        game_version: s.game_version,
                        port: s.port,
                        date_created: s.date_created.into(),
                        last_started: s.last_started.map(|d| d.into()),
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
    ) -> anyhow::Result<ServerId> {
        if name.is_empty() {
            bail!("Server name cannot be empty");
        }

        let port = port.unwrap_or(25565);

        // Generate shortpath from name
        let shortpath = generate_shortpath(&name);
        let runtime_path = &self.app.settings_manager().runtime_path;
        let servers_path = runtime_path.get_servers();
        let server_path = servers_path.get_server_path(&shortpath);

        // Create directory structure
        tokio::fs::create_dir_all(server_path.get_data_path())
            .await
            .context("Failed to create server directory")?;

        // Download server jar
        jars::download_vanilla_server_jar(
            &self.app.reqwest_client,
            &game_version,
            &server_path,
        )
        .await
        .context("Failed to download server jar")?;

        // Write initial server.properties
        let props_content = properties::generate_properties(port, "A Minecraft Server", 20, true);
        properties::write_properties(&server_path.get_server_properties_path(), &props_content)
            .await?;

        // Get next index
        let count = self
            .app
            .prisma_client
            .server()
            .count(vec![db::server::group_id::equals(group_id.0)])
            .exec()
            .await? as i32;

        // Create DB record
        let db_server = self
            .app
            .prisma_client
            .server()
            .create(
                name.clone(),
                shortpath.clone(),
                count,
                db::server_group::id::equals(group_id.0),
                game_version,
                vec![db::server::port::set(port)],
            )
            .exec()
            .await?;

        let server_id = ServerId(db_server.id);

        // Register in memory
        self.servers.write().await.insert(
            server_id,
            ServerData {
                shortpath,
                state: ServerState::Stopped { failed_task: None },
                handle: None,
            },
        );

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);

        Ok(server_id)
    }

    pub async fn delete_server(self, id: ServerId) -> anyhow::Result<()> {
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        // Stop if running
        {
            let servers = self.servers.read().await;
            if let Some(server) = servers.get(&id) {
                if matches!(server.state, ServerState::Running { .. }) {
                    drop(servers);
                    self.stop_server(id).await?;
                    // Wait a moment for graceful shutdown
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

        // Delete from DB
        self.app
            .prisma_client
            .server()
            .delete(db::server::id::equals(id.0))
            .exec()
            .await?;

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

        // Remove from memory
        self.servers.write().await.remove(&id);
        self.server_op_locks.remove(&id);

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_GROUPS, None);

        Ok(())
    }

    pub async fn start_server(self, id: ServerId) -> anyhow::Result<()> {
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        // Check current state
        {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            if matches!(server.state, ServerState::Running { .. } | ServerState::Starting(_)) {
                bail!("Server is already running or starting");
            }
        }

        // Get server info from DB
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found in database"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        // Find Java
        let java_path = self
            .app
            .java_manager()
            .find_best_java_for_server()
            .await?;

        // Set up log streaming
        let log_id = {
            let mut counter = self.log_counter.lock().await;
            *counter += 1;
            ServerLogId(*counter)
        };

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

        // Start server via provider
        let provider = self.get_provider();
        let handle = provider
            .start(
                &java_path,
                &server_path,
                db_server.xmx,
                db_server.xms,
                &db_server.extra_java_args,
                log_tx,
            )
            .await?;

        let process_id = handle.process_id;

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
            }
        }

        // Update last_started in DB
        let _ = self
            .app
            .prisma_client
            .server()
            .update(
                db::server::id::equals(id.0),
                vec![db::server::last_started::set(Some(Utc::now().into()))],
            )
            .exec()
            .await;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

        info!("Server {} started with PID {}", id.0, process_id);
        Ok(())
    }

    pub async fn stop_server(self, id: ServerId) -> anyhow::Result<()> {
        let provider = self.get_provider();

        let handle_exists = {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            server.handle.is_some()
        };

        if !handle_exists {
            bail!("Server is not running");
        }

        // Send stop command
        {
            let servers = self.servers.read().await;
            if let Some(server) = servers.get(&id) {
                if let Some(handle) = &server.handle {
                    provider.stop(handle).await?;
                }
            }
        }

        // Update state
        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&id) {
                server.state = ServerState::Stopping;
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

        // Wait for process to exit, then clean up
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&id) {
                server.state = ServerState::Stopped { failed_task: None };
                server.handle = None;
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

        info!("Server {} stopped", id.0);
        Ok(())
    }

    pub async fn kill_server(self, id: ServerId) -> anyhow::Result<()> {
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
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

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
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
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
            server_type: ServerType::from_db_string(&db_server.server_type)
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
        })
    }

    pub async fn update_server(self, update: ServerSettingsUpdate) -> anyhow::Result<()> {
        // Clone values needed for both DB update and properties file update
        let props_port = update.port;
        let props_motd = update.motd.clone();
        let props_max_players = update.max_players;
        let props_online_mode = update.online_mode;

        let mut params = Vec::new();

        if let Some(name) = update.name {
            params.push(db::server::name::set(name));
        }
        if let Some(port) = update.port {
            params.push(db::server::port::set(port));
        }
        if let Some(motd) = update.motd {
            params.push(db::server::motd::set(motd));
        }
        if let Some(max_players) = update.max_players {
            params.push(db::server::max_players::set(max_players));
        }
        if let Some(online_mode) = update.online_mode {
            params.push(db::server::online_mode::set(online_mode));
        }
        if let Some(xmx) = update.xmx {
            params.push(db::server::xmx::set(xmx));
        }
        if let Some(xms) = update.xms {
            params.push(db::server::xms::set(xms));
        }
        if let Some(extra_args) = update.extra_java_args {
            params.push(db::server::extra_java_args::set(
                extra_args.unwrap_or_default(),
            ));
        }
        if let Some(auto_restart) = update.auto_restart {
            params.push(db::server::auto_restart::set(auto_restart));
        }

        if !params.is_empty() {
            self.app
                .prisma_client
                .server()
                .update(db::server::id::equals(update.server_id.0), params)
                .exec()
                .await?;
        }

        // Also update server.properties if relevant fields changed
        if props_port.is_some()
            || props_motd.is_some()
            || props_max_players.is_some()
            || props_online_mode.is_some()
        {
            let db_server = self
                .app
                .prisma_client
                .server()
                .find_unique(db::server::id::equals(update.server_id.0))
                .exec()
                .await?;

            if let Some(server) = db_server {
                let runtime_path = &self.app.settings_manager().runtime_path;
                let server_path = runtime_path
                    .get_servers()
                    .get_server_path(&server.shortpath);
                let props_path = server_path.get_server_properties_path();

                if props_path.exists() {
                    let mut updates = std::collections::BTreeMap::new();
                    if let Some(port) = props_port {
                        updates.insert("server-port".to_string(), port.to_string());
                    }
                    if let Some(motd) = props_motd {
                        updates.insert("motd".to_string(), motd);
                    }
                    if let Some(max_players) = props_max_players {
                        updates.insert("max-players".to_string(), max_players.to_string());
                    }
                    if let Some(online_mode) = props_online_mode {
                        updates.insert("online-mode".to_string(), online_mode.to_string());
                    }

                    let existing = tokio::fs::read_to_string(&props_path).await?;
                    let updated = properties::update_properties(&existing, &updates);
                    tokio::fs::write(&props_path, &updated).await?;
                }
            }
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    pub async fn set_favorite(self, id: ServerId, favorite: bool) -> anyhow::Result<()> {
        self.app
            .prisma_client
            .server()
            .update(
                db::server::id::equals(id.0),
                vec![db::server::favorite::set(favorite)],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_ALL_SERVERS, None);

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
            ServerState::Running { process_id, .. } => {
                Ok(self
                    .app
                    .system_info_manager()
                    .get_process_metrics(*process_id)
                    .await)
            }
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

        match &server.state {
            ServerState::Running { log_id, .. } => {
                let logs = self.server_logs.read().await;
                Ok(logs.get(log_id).map(|tx| tx.subscribe()))
            }
            _ => Ok(None),
        }
    }

    pub async fn get_server_state(self, id: ServerId) -> anyhow::Result<ServerState> {
        let servers = self.servers.read().await;
        let server = servers
            .get(&id)
            .ok_or_else(|| anyhow!("Server not found"))?;
        Ok(server.state.clone())
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
    } else if sanitized.len() > 64 {
        sanitized[..64].to_string()
    } else {
        sanitized
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("{}_{}", base, timestamp)
}
