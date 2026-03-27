use self::local::LocalServerProvider;
use self::provider::{ServerHandle, ServerProvider};
use super::ManagerRef;
use crate::api::keys::server::*;
use crate::domain::server::{
    self, ServerAddon, ServerDetails, ServerGroupId, ServerGroupMoveTarget, ServerId,
    ServerListEntry, ServerLogId, ServerMoveTarget, ServerSettingsUpdate, ServerState, ServerType,
};
use crate::domain::vtask::VisualTaskId;
use anyhow::{Context, anyhow, bail};
use carbon_repos::db::read_filters::StringFilter;
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
pub mod modloader_install;
pub mod modloader_launch;
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
    index_lock: Mutex<()>,
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
                game_version.clone(),
                vec![
                    db::server::port::set(port),
                    db::server::modloader_type::set(modloader_type.clone()),
                    db::server::modloader_version::set(modloader_version.clone()),
                    db::server::server_type::set(
                        if modloader_type.is_some() { "modded".to_string() } else { "vanilla".to_string() }
                    ),
                ],
            )
            .exec()
            .await?;

        let server_id = ServerId(db_server.id);

        // Register in memory
        self.servers.write().await.insert(
            server_id,
            ServerData {
                shortpath: shortpath.clone(),
                state: ServerState::Stopped { failed_task: None },
                handle: None,
            },
        );

        // Install modloader if specified
        if let (Some(ml_type), Some(ml_version)) = (&modloader_type, &modloader_version) {
            let runtime_path = &self.app.settings_manager().runtime_path;
            let server_path = runtime_path
                .get_servers()
                .get_server_path(&shortpath);

            info!("Installing modloader {} {} for server {}", ml_type, ml_version, server_id.0);

            match modloader_install::install_modloader(
                &self.app.reqwest_client,
                &server_path,
                &game_version,
                ml_type,
                ml_version,
            )
            .await
            {
                Ok(launch_config) => {
                    modloader_launch::save_launch_config(&server_path, &launch_config).await?;
                    info!("Modloader installed successfully for server {}", server_id.0);
                }
                Err(e) => {
                    error!("Failed to install modloader for server {}: {}", server_id.0, e);
                    // Don't fail server creation - it can still run vanilla
                    // But we should let the user know
                }
            }
        }

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

        // Load modloader launch config
        let launch_config = modloader_launch::get_launch_config(&server_path).await?;

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
        })
    }

    pub async fn update_server(self, update: ServerSettingsUpdate) -> anyhow::Result<()> {
        let mut params = Vec::new();

        if let Some(name) = update.name {
            params.push(db::server::name::set(name));
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

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app
            .invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    /// Get all server.properties as a key-value map
    pub async fn get_server_properties(
        self,
        id: ServerId,
    ) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
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
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
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
        let mut db_params = Vec::new();
        if let Some(port) = btree_updates.get("server-port") {
            if let Ok(port) = port.parse::<i32>() {
                db_params.push(db::server::port::set(port));
            }
        }
        if let Some(motd) = btree_updates.get("motd") {
            db_params.push(db::server::motd::set(motd.clone()));
        }
        if let Some(max_players) = btree_updates.get("max-players") {
            if let Ok(max_players) = max_players.parse::<i32>() {
                db_params.push(db::server::max_players::set(max_players));
            }
        }
        if let Some(online_mode) = btree_updates.get("online-mode") {
            db_params.push(db::server::online_mode::set(online_mode == "true"));
        }

        if !db_params.is_empty() {
            self.app
                .prisma_client
                .server()
                .update(db::server::id::equals(id.0), db_params)
                .exec()
                .await?;
        }

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    /// Read a player list JSON file (whitelist, ops, banned-players)
    pub async fn get_player_list<T: serde::de::DeserializeOwned>(
        self,
        id: ServerId,
        filename: &str,
    ) -> anyhow::Result<Vec<T>> {
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let file_path = server_path.get_data_path().join(filename);

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let entries: Vec<T> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", filename))?;
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

        let profile: MojangProfile = resp.json().await.context("Failed to parse Mojang response")?;

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
            let _ = self.send_console_command(id, command).await;
        }
    }

    /// List server addons (mods and datapacks) by scanning filesystem
    pub async fn list_server_addons(
        self,
        id: ServerId,
    ) -> anyhow::Result<Vec<ServerAddon>> {
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        let mut addons = Vec::new();

        // Scan mods directory
        let mods_path = server_path.get_mods_path();
        if mods_path.exists() {
            let mut entries = tokio::fs::read_dir(&mods_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let filename = entry.file_name().to_string_lossy().to_string();
                if filename.ends_with(".jar") || filename.ends_with(".jar.disabled") {
                    let metadata = entry.metadata().await?;
                    let enabled = !filename.ends_with(".disabled");
                    let display_name = filename
                        .trim_end_matches(".disabled")
                        .trim_end_matches(".jar")
                        .to_string();
                    addons.push(ServerAddon {
                        id: filename.clone(),
                        filename,
                        display_name,
                        enabled,
                        addon_type: "mods".to_string(),
                        file_size: metadata.len() as i32,
                    });
                }
            }
        }

        // Scan datapacks directory
        let datapacks_path = server_path.get_datapacks_path();
        if datapacks_path.exists() {
            let mut entries = tokio::fs::read_dir(&datapacks_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let filename = entry.file_name().to_string_lossy().to_string();
                if filename.ends_with(".zip") || filename.ends_with(".zip.disabled") {
                    let metadata = entry.metadata().await?;
                    let enabled = !filename.ends_with(".disabled");
                    let display_name = filename
                        .trim_end_matches(".disabled")
                        .trim_end_matches(".zip")
                        .to_string();
                    addons.push(ServerAddon {
                        id: filename.clone(),
                        filename,
                        display_name,
                        enabled,
                        addon_type: "datapacks".to_string(),
                        file_size: metadata.len() as i32,
                    });
                }
            }
        }

        Ok(addons)
    }

    /// Enable or disable a server addon by renaming the file
    pub async fn enable_server_addon(
        self,
        id: ServerId,
        addon_id: String,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        // Try mods/ first, then datapacks/
        let file_path = {
            let mods_path = server_path.get_mods_path().join(&addon_id);
            if mods_path.exists() {
                mods_path
            } else {
                let dp_path = server_path.get_datapacks_path().join(&addon_id);
                if dp_path.exists() {
                    dp_path
                } else {
                    bail!("Addon file not found: {}", addon_id);
                }
            }
        };

        let new_path = if enabled {
            // Remove .disabled suffix
            let name = file_path.to_string_lossy().trim_end_matches(".disabled").to_string();
            std::path::PathBuf::from(name)
        } else {
            // Add .disabled suffix
            let mut name = file_path.to_string_lossy().to_string();
            if !name.ends_with(".disabled") {
                name.push_str(".disabled");
            }
            std::path::PathBuf::from(name)
        };

        if file_path != new_path {
            tokio::fs::rename(&file_path, &new_path).await?;
        }

        self.app.invalidate(GET_SERVER_ADDONS, None);

        Ok(())
    }

    /// Delete a server addon file
    pub async fn delete_server_addon(
        self,
        id: ServerId,
        addon_id: String,
    ) -> anyhow::Result<()> {
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);

        // Try mods/ first, then datapacks/
        let file_path = {
            let mods_path = server_path.get_mods_path().join(&addon_id);
            if mods_path.exists() {
                mods_path
            } else {
                let dp_path = server_path.get_datapacks_path().join(&addon_id);
                if dp_path.exists() {
                    dp_path
                } else {
                    bail!("Addon file not found: {}", addon_id);
                }
            }
        };

        tokio::fs::remove_file(&file_path).await?;
        self.app.invalidate(GET_SERVER_ADDONS, None);

        Ok(())
    }

    /// Write a player list JSON file
    pub async fn write_player_list<T: serde::Serialize>(
        self,
        id: ServerId,
        filename: &str,
        entries: &[T],
    ) -> anyhow::Result<()> {
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path
            .get_servers()
            .get_server_path(&db_server.shortpath);
        let file_path = server_path.get_data_path().join(filename);

        let content = serde_json::to_string_pretty(entries)?;
        tokio::fs::write(&file_path, content).await?;
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

    pub async fn set_server_icon(
        self,
        id: ServerId,
        base64_data: String,
    ) -> anyhow::Result<()> {
        use base64::Engine;

        let image_data = base64::engine::general_purpose::STANDARD
            .decode(&base64_data)
            .context("Invalid base64 image data")?;

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
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found in database"))?;

        let new_revision = db_server.icon_revision.unwrap_or(0) + 1;

        self.app
            .prisma_client
            .server()
            .update(
                db::server::id::equals(id.0),
                vec![db::server::icon_revision::set(Some(new_revision))],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        Ok(())
    }

    pub async fn server_icon(
        self,
        id: ServerId,
    ) -> anyhow::Result<Option<(String, Vec<u8>)>> {
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
        use db::server::{SetParam, UniqueWhereParam, WhereParam};

        let _index_lock = self.index_lock.lock().await;

        let default_group_id = self.get_default_group().await?;

        let (start_group, start_idx, start_library_pos) = {
            let server = self
                .app
                .prisma_client
                .server()
                .find_unique(UniqueWhereParam::IdEquals(server_id.0))
                .exec()
                .await?
                .ok_or_else(|| anyhow!("Server not found in database"))?;

            (ServerGroupId(server.group_id), server.index, server.library_position)
        };

        let (target_group, target_idx, target_library_pos) = match target {
            ServerMoveTarget::BeforeServer(target_id) => {
                let srv = self
                    .app
                    .prisma_client
                    .server()
                    .find_unique(UniqueWhereParam::IdEquals(target_id.0))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("Target server not found in database"))?;

                (ServerGroupId(srv.group_id), srv.index, srv.library_position)
            }
            ServerMoveTarget::EndOfGroup(group) => {
                let target_idx = self
                    .app
                    .prisma_client
                    .server()
                    .count(vec![WhereParam::GroupId(IntFilter::Equals(group.0))])
                    .exec()
                    .await? as i32;

                let lib_pos = if group == default_group_id {
                    let max_server_pos = self
                        .app
                        .prisma_client
                        .server()
                        .find_first(vec![
                            WhereParam::GroupId(IntFilter::Equals(group.0)),
                            WhereParam::LibraryPosition(
                                db::read_filters::IntNullableFilter::Not(None),
                            ),
                        ])
                        .order_by(db::server::OrderByParam::LibraryPosition(Direction::Desc))
                        .exec()
                        .await?
                        .and_then(|s| s.library_position);

                    let max_group_pos = self
                        .app
                        .prisma_client
                        .server_group()
                        .find_first(vec![db::server_group::WhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Not(None),
                        )])
                        .order_by(db::server_group::OrderByParam::LibraryPosition(
                            Direction::Desc,
                        ))
                        .exec()
                        .await?
                        .and_then(|g| g.library_position);

                    let max_pos = max_server_pos.unwrap_or(0).max(max_group_pos.unwrap_or(0));
                    Some(max_pos + 1)
                } else {
                    None
                };

                (group, target_idx, lib_pos)
            }
            ServerMoveTarget::BeforeGroup(group_id) => {
                let target_folder = self
                    .app
                    .prisma_client
                    .server_group()
                    .find_unique(db::server_group::UniqueWhereParam::IdEquals(group_id.0))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("Server group not found in database"))?;

                let lib_pos = target_folder.library_position.ok_or_else(|| {
                    anyhow!("Target folder has no libraryPosition")
                })?;

                let target_idx = self
                    .app
                    .prisma_client
                    .server()
                    .count(vec![WhereParam::GroupId(IntFilter::Equals(default_group_id.0))])
                    .exec()
                    .await? as i32;

                (default_group_id, target_idx, Some(lib_pos))
            }
        };

        let index_shifts = if start_group == target_group {
            vec![match (start_idx, target_idx) {
                (start, target) if start < target => self.app.prisma_client.server().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(target_group.0)),
                        WhereParam::Index(IntFilter::Gt(start)),
                        WhereParam::Index(IntFilter::Lt(target)),
                    ],
                    vec![SetParam::DecrementIndex(1)],
                ),
                (start, target) if start > target => self.app.prisma_client.server().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(target_group.0)),
                        WhereParam::Index(IntFilter::Gte(target)),
                        WhereParam::Index(IntFilter::Lt(start)),
                    ],
                    vec![SetParam::IncrementIndex(1)],
                ),
                _ => return Ok(()),
            }]
        } else {
            vec![
                self.app.prisma_client.server().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(start_group.0)),
                        WhereParam::Index(IntFilter::Gt(start_idx)),
                    ],
                    vec![SetParam::DecrementIndex(1)],
                ),
                self.app.prisma_client.server().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(target_group.0)),
                        WhereParam::Index(IntFilter::Gte(target_idx)),
                    ],
                    vec![SetParam::IncrementIndex(1)],
                ),
            ]
        };

        let final_idx = if start_group == target_group && start_idx < target_idx {
            target_idx - 1
        } else {
            target_idx
        };

        let mut update_params = vec![
            SetParam::SetGroupId(target_group.0),
            SetParam::SetIndex(final_idx),
        ];

        let new_library_pos = if target_group == default_group_id {
            target_library_pos
        } else {
            None
        };

        update_params.push(SetParam::SetLibraryPosition(new_library_pos));

        // If moving TO default group and inserting before an item, shift library positions
        if target_group == default_group_id {
            if let Some(target_lib_pos) = target_library_pos {
                if start_library_pos != Some(target_lib_pos) {
                    self.app
                        .prisma_client
                        .server()
                        .update_many(
                            vec![
                                WhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                                WhereParam::LibraryPosition(
                                    db::read_filters::IntNullableFilter::Gte(target_lib_pos),
                                ),
                                WhereParam::Id(db::read_filters::IntFilter::Not(server_id.0)),
                            ],
                            vec![SetParam::IncrementLibraryPosition(1)],
                        )
                        .exec()
                        .await?;

                    self.app
                        .prisma_client
                        .server_group()
                        .update_many(
                            vec![db::server_group::WhereParam::LibraryPosition(
                                db::read_filters::IntNullableFilter::Gte(target_lib_pos),
                            )],
                            vec![db::server_group::SetParam::IncrementLibraryPosition(1)],
                        )
                        .exec()
                        .await?;
                }
            }
        }

        // If moving FROM default group, shift library positions to fill the gap
        if start_group == default_group_id && target_group != default_group_id {
            if let Some(start_lib_pos) = start_library_pos {
                self.app
                    .prisma_client
                    .server()
                    .update_many(
                        vec![
                            WhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                            WhereParam::LibraryPosition(
                                db::read_filters::IntNullableFilter::Gt(start_lib_pos),
                            ),
                        ],
                        vec![SetParam::DecrementLibraryPosition(1)],
                    )
                    .exec()
                    .await?;

                self.app
                    .prisma_client
                    .server_group()
                    .update_many(
                        vec![db::server_group::WhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Gt(start_lib_pos),
                        )],
                        vec![db::server_group::SetParam::DecrementLibraryPosition(1)],
                    )
                    .exec()
                    .await?;
            }
        }

        self.app
            .prisma_client
            ._batch((
                index_shifts,
                self.app.prisma_client.server().update(
                    UniqueWhereParam::IdEquals(server_id.0),
                    update_params,
                ),
            ))
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        // Auto-delete empty non-default groups after moving server out
        if start_group != default_group_id && start_group != target_group {
            let remaining_count = self
                .app
                .prisma_client
                .server()
                .count(vec![WhereParam::GroupId(IntFilter::Equals(start_group.0))])
                .exec()
                .await?;

            if remaining_count == 0 {
                self.app
                    .prisma_client
                    .server_group()
                    .delete(db::server_group::UniqueWhereParam::IdEquals(start_group.0))
                    .exec()
                    .await?;
                self.app.invalidate(GET_GROUPS, None);
            }
        }

        Ok(())
    }

    pub async fn move_server_group(
        self,
        group: ServerGroupId,
        target: ServerGroupMoveTarget,
    ) -> anyhow::Result<()> {
        use db::server::{
            SetParam as ServerSetParam, WhereParam as ServerWhereParam,
        };
        use db::server_group::{SetParam, UniqueWhereParam, WhereParam};

        let _index_lock = self.index_lock.lock().await;

        let default_group_id = self.get_default_group().await?;

        let moving_group = self
            .app
            .prisma_client
            .server_group()
            .find_unique(UniqueWhereParam::IdEquals(group.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server group not found in database"))?;

        let start_pos = moving_group.library_position;

        let target_pos = match target {
            ServerGroupMoveTarget::BeforeGroup(target_group_id) => {
                let target_group = self
                    .app
                    .prisma_client
                    .server_group()
                    .find_unique(UniqueWhereParam::IdEquals(target_group_id.0))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("Target server group not found in database"))?;

                target_group.library_position.ok_or_else(|| {
                    anyhow!("Target group has no libraryPosition (is it the default group?)")
                })?
            }
            ServerGroupMoveTarget::BeforeServer(server_id) => {
                let server = self
                    .app
                    .prisma_client
                    .server()
                    .find_unique(db::server::UniqueWhereParam::IdEquals(server_id.0))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("Server not found in database"))?;

                if server.group_id != default_group_id.0 {
                    bail!("Can only position a group before ungrouped servers (servers in default group)");
                }

                server.library_position.ok_or_else(|| {
                    anyhow!("Server has no libraryPosition")
                })?
            }
            ServerGroupMoveTarget::EndOfLibrary => {
                let max_server_pos: Option<i32> = self
                    .app
                    .prisma_client
                    .server()
                    .find_first(vec![
                        ServerWhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                        ServerWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Not(None),
                        ),
                    ])
                    .order_by(db::server::OrderByParam::LibraryPosition(Direction::Desc))
                    .exec()
                    .await?
                    .and_then(|s| s.library_position);

                let max_group_pos: Option<i32> = self
                    .app
                    .prisma_client
                    .server_group()
                    .find_first(vec![WhereParam::LibraryPosition(
                        db::read_filters::IntNullableFilter::Not(None),
                    )])
                    .order_by(db::server_group::OrderByParam::LibraryPosition(
                        Direction::Desc,
                    ))
                    .exec()
                    .await?
                    .and_then(|g| g.library_position);

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

        if start_pos < target_pos {
            // Moving forward: shift items in (start, target] down by 1
            self.app
                .prisma_client
                .server_group()
                .update_many(
                    vec![
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gt(
                            start_pos,
                        )),
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Lte(
                            target_pos - 1,
                        )),
                    ],
                    vec![SetParam::DecrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            self.app
                .prisma_client
                .server()
                .update_many(
                    vec![
                        ServerWhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                        ServerWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Gt(start_pos),
                        ),
                        ServerWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Lte(target_pos - 1),
                        ),
                    ],
                    vec![ServerSetParam::DecrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            self.app
                .prisma_client
                .server_group()
                .update(
                    UniqueWhereParam::IdEquals(group.0),
                    vec![SetParam::SetLibraryPosition(Some(target_pos - 1))],
                )
                .exec()
                .await?;
        } else {
            // Moving backward: shift items in [target, start) up by 1
            self.app
                .prisma_client
                .server_group()
                .update_many(
                    vec![
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gte(
                            target_pos,
                        )),
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Lt(
                            start_pos,
                        )),
                    ],
                    vec![SetParam::IncrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            self.app
                .prisma_client
                .server()
                .update_many(
                    vec![
                        ServerWhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                        ServerWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Gte(target_pos),
                        ),
                        ServerWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Lt(start_pos),
                        ),
                    ],
                    vec![ServerSetParam::IncrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            self.app
                .prisma_client
                .server_group()
                .update(
                    UniqueWhereParam::IdEquals(group.0),
                    vec![SetParam::SetLibraryPosition(Some(target_pos))],
                )
                .exec()
                .await?;
        }

        // Keep groupIndex in sync
        let all_groups = self
            .app
            .prisma_client
            .server_group()
            .find_many(vec![WhereParam::LibraryPosition(
                db::read_filters::IntNullableFilter::Not(None),
            )])
            .order_by(db::server_group::OrderByParam::LibraryPosition(
                Direction::Asc,
            ))
            .exec()
            .await?;

        for (idx, g) in all_groups.iter().enumerate() {
            self.app
                .prisma_client
                .server_group()
                .update(
                    UniqueWhereParam::IdEquals(g.id),
                    vec![SetParam::SetGroupIndex(idx as i32)],
                )
                .exec()
                .await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);
        Ok(())
    }

    /// Generate a unique folder name by appending (1), (2), etc. if needed.
    async fn generate_unique_folder_name(&self, base_name: &str) -> anyhow::Result<String> {
        use db::server_group::WhereParam;

        let existing = self
            .app
            .prisma_client
            .server_group()
            .find_first(vec![WhereParam::Name(StringFilter::Equals(
                base_name.to_string(),
            ))])
            .exec()
            .await?;

        if existing.is_none() {
            return Ok(base_name.to_string());
        }

        let mut counter = 1;
        loop {
            let candidate = format!("{} ({})", base_name, counter);
            let exists = self
                .app
                .prisma_client
                .server_group()
                .find_first(vec![WhereParam::Name(StringFilter::Equals(
                    candidate.clone(),
                ))])
                .exec()
                .await?;

            if exists.is_none() {
                return Ok(candidate);
            }
            counter += 1;
        }
    }

    pub async fn create_server_group(self, name: String) -> anyhow::Result<ServerGroupId> {
        use db::server_group::WhereParam;

        let group_count = self
            .app
            .prisma_client
            .server_group()
            .count(vec![])
            .exec()
            .await? as i32;

        let default_group_id = self.get_default_group().await?;

        // Calculate next libraryPosition
        let max_server_pos: Option<i32> = self
            .app
            .prisma_client
            .server()
            .find_first(vec![
                db::server::WhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                db::server::WhereParam::LibraryPosition(
                    db::read_filters::IntNullableFilter::Not(None),
                ),
            ])
            .order_by(db::server::OrderByParam::LibraryPosition(Direction::Desc))
            .exec()
            .await?
            .and_then(|s| s.library_position);

        let max_group_pos: Option<i32> = self
            .app
            .prisma_client
            .server_group()
            .find_first(vec![WhereParam::LibraryPosition(
                db::read_filters::IntNullableFilter::Not(None),
            )])
            .order_by(db::server_group::OrderByParam::LibraryPosition(
                Direction::Desc,
            ))
            .exec()
            .await?
            .and_then(|g| g.library_position);

        let next_library_pos = max_server_pos.unwrap_or(0).max(max_group_pos.unwrap_or(0)) + 1;

        let group = self
            .app
            .prisma_client
            .server_group()
            .create(
                name,
                group_count,
                vec![db::server_group::SetParam::SetLibraryPosition(Some(
                    next_library_pos,
                ))],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(ServerGroupId(group.id))
    }

    pub async fn create_server_group_at_position(
        self,
        name: String,
        target_position: i32,
    ) -> anyhow::Result<ServerGroupId> {
        let group_count = self
            .app
            .prisma_client
            .server_group()
            .count(vec![])
            .exec()
            .await? as i32;

        let default_group_id = self.get_default_group().await?;

        // Shift all items with library_position >= target_position up by 1
        self.app
            .prisma_client
            .server()
            .update_many(
                vec![
                    db::server::WhereParam::GroupId(IntFilter::Equals(default_group_id.0)),
                    db::server::WhereParam::LibraryPosition(
                        db::read_filters::IntNullableFilter::Gte(target_position),
                    ),
                ],
                vec![db::server::SetParam::IncrementLibraryPosition(1)],
            )
            .exec()
            .await?;

        self.app
            .prisma_client
            .server_group()
            .update_many(
                vec![db::server_group::WhereParam::LibraryPosition(
                    db::read_filters::IntNullableFilter::Gte(target_position),
                )],
                vec![db::server_group::SetParam::IncrementLibraryPosition(1)],
            )
            .exec()
            .await?;

        let group = self
            .app
            .prisma_client
            .server_group()
            .create(
                name,
                group_count,
                vec![db::server_group::SetParam::SetLibraryPosition(Some(
                    target_position,
                ))],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(ServerGroupId(group.id))
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
            let target_server = self
                .app
                .prisma_client
                .server()
                .find_unique(db::server::UniqueWhereParam::IdEquals(target_id.0))
                .exec()
                .await?;

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
        use db::server::{SetParam, UniqueWhereParam, WhereParam};
        use db::server_group::{
            SetParam as GroupSetParam, UniqueWhereParam as GroupUniqueWhereParam,
        };

        let default_group_id = self.get_default_group().await?;

        let _index_lock = self.index_lock.lock().await;

        // Get all servers in default group and sort by name
        let servers = self
            .app
            .prisma_client
            .server()
            .find_many(vec![WhereParam::GroupId(IntFilter::Equals(
                default_group_id.0,
            ))])
            .exec()
            .await?;

        let mut sortable_servers: Vec<(i32, String)> = servers
            .iter()
            .map(|s| (s.id, s.name.clone()))
            .collect();

        sortable_servers.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        let mut updates = Vec::new();
        for (new_index, (server_id, _)) in sortable_servers.iter().enumerate() {
            updates.push(self.app.prisma_client.server().update(
                UniqueWhereParam::IdEquals(*server_id),
                vec![SetParam::SetIndex(new_index as i32)],
            ));
        }

        if !updates.is_empty() {
            self.app.prisma_client._batch(updates).await?;
        }

        // Sort groups by name
        let groups = self
            .app
            .prisma_client
            .server_group()
            .find_many(vec![])
            .exec()
            .await?;

        let mut sortable_groups: Vec<(i32, String)> = groups
            .iter()
            .filter(|g| g.id != default_group_id.0)
            .map(|g| (g.id, g.name.clone()))
            .collect();

        sortable_groups.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        let mut group_updates = Vec::new();

        group_updates.push(self.app.prisma_client.server_group().update(
            GroupUniqueWhereParam::IdEquals(default_group_id.0),
            vec![GroupSetParam::SetGroupIndex(0)],
        ));

        for (new_index, (group_id, _)) in sortable_groups.iter().enumerate() {
            group_updates.push(self.app.prisma_client.server_group().update(
                GroupUniqueWhereParam::IdEquals(*group_id),
                vec![GroupSetParam::SetGroupIndex((new_index + 1) as i32)],
            ));
        }

        if !group_updates.is_empty() {
            self.app.prisma_client._batch(group_updates).await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(())
    }

    pub async fn rename_server_group(
        self,
        group: ServerGroupId,
        name: String,
    ) -> anyhow::Result<()> {
        use db::server_group::{SetParam, UniqueWhereParam};

        self.app
            .prisma_client
            .server_group()
            .update(
                UniqueWhereParam::IdEquals(group.0),
                vec![SetParam::SetName(name)],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_SERVERS, None);

        Ok(())
    }

    pub async fn delete_server_group(self, group: ServerGroupId) -> anyhow::Result<()> {
        use db::{server, server_group};

        let _index_lock = self.index_lock.lock().await;

        let any_servers = self
            .app
            .prisma_client
            .server()
            .count(vec![server::WhereParam::GroupId(IntFilter::Equals(
                group.0,
            ))])
            .exec()
            .await?
            != 0;

        if any_servers {
            let default_group = self.get_default_group().await?;

            let base_index = self
                .app
                .prisma_client
                .server()
                .count(vec![server::WhereParam::GroupId(IntFilter::Equals(
                    default_group.0,
                ))])
                .exec()
                .await?;

            self.app
                .prisma_client
                ._batch((
                    self.app.prisma_client.server().update_many(
                        vec![server::WhereParam::GroupId(IntFilter::Equals(group.0))],
                        vec![
                            server::SetParam::SetGroupId(default_group.0),
                            server::SetParam::IncrementIndex(base_index as i32),
                        ],
                    ),
                    self.app
                        .prisma_client
                        .server_group()
                        .delete(server_group::UniqueWhereParam::IdEquals(group.0)),
                ))
                .await?;
        } else {
            self.app
                .prisma_client
                .server_group()
                .delete(server_group::UniqueWhereParam::IdEquals(group.0))
                .exec()
                .await?;
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
