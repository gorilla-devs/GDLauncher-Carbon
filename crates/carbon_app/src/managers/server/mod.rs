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
use anyhow::{Context, anyhow, bail};
use carbon_repos::db::read_filters::StringFilter;
use carbon_repos::db::{self, read_filters::IntFilter};
use carbon_repos::pcr::Direction;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
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
                    last_log_id: None,
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

        // Hold index_lock to make lookup+create atomic
        let _index_guard = self.index_lock.lock().await;

        // Newly created servers appear at the TOP of their group — pick an
        // index strictly smaller than the current minimum so ascending
        // sort on `index` puts the new row first.
        let min_index: Option<i32> = self
            .app
            .prisma_client
            .server()
            .find_first(vec![db::server::group_id::equals(group_id.0)])
            .order_by(db::server::OrderByParam::Index(Direction::Asc))
            .exec()
            .await?
            .map(|s| s.index);
        let next_index = min_index.map(|n| n - 1).unwrap_or(0);

        // If the new server is in the default group, also give it a
        // library_position that sorts above every existing server or
        // folder at the library's top level.
        let default_group_id = self.clone().get_default_group().await?;
        let library_position = if group_id == default_group_id {
            let min_server_pos: Option<i32> = self
                .app
                .prisma_client
                .server()
                .find_first(vec![
                    db::server::group_id::equals(default_group_id.0),
                    db::server::library_position::not(None),
                ])
                .order_by(db::server::OrderByParam::LibraryPosition(Direction::Asc))
                .exec()
                .await?
                .and_then(|s| s.library_position);

            let min_group_pos: Option<i32> = self
                .app
                .prisma_client
                .server_group()
                .find_first(vec![db::server_group::library_position::not(None)])
                .order_by(db::server_group::OrderByParam::LibraryPosition(
                    Direction::Asc,
                ))
                .exec()
                .await?
                .and_then(|g| g.library_position);

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
        let mut extra_params = vec![
            db::server::port::set(port),
            db::server::modloader_type::set(modloader_type.clone()),
            db::server::modloader_version::set(modloader_version.clone()),
            db::server::server_type::set(if modloader_type.is_some() {
                "modded".to_string()
            } else {
                "vanilla".to_string()
            }),
        ];
        if let Some(pos) = library_position {
            extra_params.push(db::server::library_position::set(Some(pos)));
        }

        let db_server = self
            .app
            .prisma_client
            .server()
            .create(
                name.clone(),
                shortpath.clone(),
                next_index,
                db::server_group::id::equals(group_id.0),
                game_version.clone(),
                extra_params,
            )
            .exec()
            .await?;

        drop(_index_guard);

        let server_id = ServerId(db_server.id);

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
                let props_content =
                    properties::generate_properties(port, "A Minecraft Server", 20, true);
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
                        .find_best_java_for_server()
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

        let port = port.unwrap_or(25565);
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
        let min_index: Option<i32> = self
            .app
            .prisma_client
            .server()
            .find_first(vec![db::server::group_id::equals(group_id.0)])
            .order_by(db::server::OrderByParam::Index(Direction::Asc))
            .exec()
            .await?
            .map(|s| s.index);
        let next_index = min_index.map(|n| n - 1).unwrap_or(0);

        // If placing in the default group, also bump library_position to
        // sort above all existing top-level items.
        let default_group_id = self.clone().get_default_group().await?;
        let library_position = if group_id == default_group_id {
            let min_server_pos: Option<i32> = self
                .app
                .prisma_client
                .server()
                .find_first(vec![
                    db::server::group_id::equals(default_group_id.0),
                    db::server::library_position::not(None),
                ])
                .order_by(db::server::OrderByParam::LibraryPosition(Direction::Asc))
                .exec()
                .await?
                .and_then(|s| s.library_position);

            let min_group_pos: Option<i32> = self
                .app
                .prisma_client
                .server_group()
                .find_first(vec![db::server_group::library_position::not(None)])
                .order_by(db::server_group::OrderByParam::LibraryPosition(
                    Direction::Asc,
                ))
                .exec()
                .await?
                .and_then(|g| g.library_position);

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

        let mut extra_params = vec![
            db::server::port::set(port),
            db::server::server_type::set("modded".to_string()),
            db::server::modpack_platform::set(Some(modpack_platform)),
            db::server::modpack_project_id::set(Some(modpack_project_id)),
            db::server::modpack_file_id::set(Some(modpack_file_id)),
        ];
        if let Some(pos) = library_position {
            extra_params.push(db::server::library_position::set(Some(pos)));
        }

        // Create DB record with a placeholder game_version — will be updated after processing
        let db_server = self
            .app
            .prisma_client
            .server()
            .create(
                name.clone(),
                shortpath.clone(),
                next_index,
                db::server_group::id::equals(group_id.0),
                "unknown".to_string(),
                extra_params,
            )
            .exec()
            .await?;

        drop(_index_guard);

        let server_id = ServerId(db_server.id);

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

        // Download and save the modpack icon before spawning (small thumbnail, won't block)
        if let Some(ref url) = icon_url {
            match self.app.reqwest_client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(bytes) = response.bytes().await {
                            let icon_path = server_path.get_root().join("icon.png");
                            if let Err(e) = tokio::fs::write(&icon_path, &bytes).await {
                                warn!("Failed to write server icon: {}", e);
                            } else {
                                let _ = self
                                    .app
                                    .prisma_client
                                    .server()
                                    .update(
                                        db::server::id::equals(server_id.0),
                                        vec![db::server::icon_revision::set(Some(1))],
                                    )
                                    .exec()
                                    .await;
                            }
                        }
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

                // Install modloader only if detected. Complete the pre-created subtask
                // either way to reach 100%.
                if let (Some(ml_type), Some(ml_version)) =
                    (&pack_result.modloader_type, &pack_result.modloader_version)
                {
                    let java_path = app
                        .java_manager()
                        .find_best_java_for_server()
                        .await
                        .context("Cannot install modloader: no Java available")?;

                    let launch_config = modloader_install::install_modloader(
                        &app.reqwest_client,
                        &server_path,
                        &pack_result.game_version,
                        ml_type,
                        ml_version,
                        &java_path,
                        Some(&t_install_modloader),
                    )
                    .await
                    .context(format!("Failed to install {} {}", ml_type, ml_version))?;

                    modloader_launch::save_launch_config(&server_path, &launch_config).await?;
                } else {
                    t_install_modloader.complete_opaque();
                }

                // Write server.properties if not present
                let props_path = server_path.get_server_properties_path();
                if !props_path.exists() {
                    let props =
                        properties::generate_properties(port, "A Minecraft Server", 20, true);
                    properties::write_properties(&props_path, &props).await?;
                }

                // Update DB with detected versions
                let _ = app
                    .prisma_client
                    .server()
                    .update(
                        db::server::id::equals(server_id.0),
                        vec![
                            db::server::game_version::set(pack_result.game_version),
                            db::server::modloader_type::set(pack_result.modloader_type),
                            db::server::modloader_version::set(pack_result.modloader_version),
                        ],
                    )
                    .exec()
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
        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
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
        self.app.invalidate(GET_SERVER_DETAILS, None);

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

        // Find Java
        let java_path = self.app.java_manager().find_best_java_for_server().await?;

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

        let exit_notify = handle.exit_notify.clone();

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

        // Pre-warm CPU metrics tracking so the first WebSocket poll gets non-zero values.
        // sysinfo needs two refresh calls to compute cpu_usage delta.
        self.app
            .system_info_manager()
            .get_process_metrics(process_id)
            .await;

        self.app.invalidate(GET_ALL_SERVERS, None);
        self.app.invalidate(GET_SERVER_DETAILS, None);

        // Spawn a watcher for unexpected exits (crash/normal exit not triggered by stop/kill).
        // If auto_restart is enabled, restart the server automatically.
        let app = self.app.clone();
        tokio::spawn(async move {
            exit_notify.notified().await;

            // Check if the exit was unexpected (state is still Running).
            // If stop_server/kill_server initiated the shutdown, they will have
            // already transitioned the state away from Running.
            let should_restart = {
                let mut servers = app.server_manager.servers.write().await;
                let Some(server) = servers.get_mut(&id) else {
                    return;
                };
                if !matches!(server.state, ServerState::Running { .. }) {
                    // stop_server or kill_server already handling cleanup
                    return;
                }
                // Unexpected exit — clean up the handle
                server.handle = None;
                server.state = ServerState::Stopped { failed_task: None };

                // Check auto_restart setting from DB
                app.prisma_client
                    .server()
                    .find_unique(db::server::id::equals(id.0))
                    .exec()
                    .await
                    .ok()
                    .flatten()
                    .map(|s| s.auto_restart)
                    .unwrap_or(false)
            };

            app.invalidate(GET_ALL_SERVERS, None);
            app.invalidate(GET_SERVER_DETAILS, None);

            if should_restart {
                info!("Server {} exited unexpectedly, auto-restarting", id.0);
                // Brief delay to avoid tight crash loops
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                // ManagerRef's future is not Send, so we use a oneshot to
                // bridge into a context where we can call start_server.
                let (tx, rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
                let app2 = app.clone();
                // This inner task owns the Arc and can create a ManagerRef locally
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Handle::current();
                    let result = rt.block_on(app2.server_manager().start_server(id));
                    let _ = tx.send(result);
                });
                match rx.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => error!("Failed to auto-restart server {}: {}", id.0, e),
                    Err(_) => error!("Auto-restart channel dropped for server {}", id.0),
                }
            } else {
                warn!("Server {} exited unexpectedly", id.0);
            }
        });

        info!("Server {} started with PID {}", id.0, process_id);
        Ok(())
    }

    pub async fn stop_server(self, id: ServerId) -> anyhow::Result<()> {
        let lock = self.get_op_lock(id);
        let _guard = lock.lock().await;

        let provider = self.get_provider();

        let exit_notify = {
            let servers = self.servers.read().await;
            let server = servers
                .get(&id)
                .ok_or_else(|| anyhow!("Server not found"))?;
            let handle = server
                .handle
                .as_ref()
                .ok_or_else(|| anyhow!("Server is not running"))?;
            handle.exit_notify.clone()
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

            match tokio::time::timeout(GRACEFUL_TIMEOUT, exit_notify.notified()).await {
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
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        exit_notify.notified(),
                    )
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
        self.app.invalidate(GET_SERVER_DETAILS, None);

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

    /// Read a player list JSON file (whitelist, ops, banned-players, banned-ips)
    pub async fn get_player_list<T: serde::de::DeserializeOwned>(
        self,
        id: ServerId,
        list: PlayerListFile,
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
            let _ = self.send_console_command(id, command).await;
        }
    }

    /// List server addons from database cache. Triggers caching if needed.
    pub async fn list_server_addons(self, id: ServerId) -> anyhow::Result<Vec<ServerAddon>> {
        use carbon_repos::db::{
            curse_forge_mod_cache as cfdb, mod_metadata as metadb, modrinth_mod_cache as mrdb,
            server_mod_file_cache as sfcdb,
        };

        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Query from cache with metadata joins
        // Caching is handled by the queue system (triggered on install, startup, and tab navigation)
        let cached_mods = self
            .app
            .prisma_client
            .server_mod_file_cache()
            .find_many(vec![sfcdb::server_id::equals(id.0)])
            .with(
                sfcdb::metadata::fetch()
                    .with(metadb::logo_image::fetch())
                    .with(metadb::curseforge::fetch().with(cfdb::logo_image::fetch()))
                    .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
            )
            .exec()
            .await?;

        let mut addons: Vec<ServerAddon> = cached_mods
            .into_iter()
            .filter_map(|entry| {
                let metadata = match entry.metadata.as_ref() {
                    Some(m) => m,
                    None => {
                        warn!(
                            "ServerModFileCache entry {} has no metadata, skipping",
                            entry.id
                        );
                        return None;
                    }
                };

                let display_name = metadata.name.clone().unwrap_or_else(|| {
                    entry
                        .filename
                        .trim_end_matches(".jar")
                        .trim_end_matches(".zip")
                        .to_string()
                });

                let has_local_image = metadata
                    .logo_image
                    .as_ref()
                    .and_then(|opt| opt.as_ref())
                    .is_some();

                let cf = metadata.curseforge.as_ref().and_then(|opt| opt.as_ref());
                let mr = metadata.modrinth.as_ref().and_then(|opt| opt.as_ref());

                let has_cf_image = cf
                    .and_then(|c| c.logo_image.as_ref())
                    .and_then(|opt| opt.as_ref())
                    .is_some();

                let has_mr_image = mr
                    .and_then(|m| m.logo_image.as_ref())
                    .and_then(|opt| opt.as_ref())
                    .is_some();

                Some(ServerAddon {
                    id: entry.id,
                    filename: entry.filename,
                    display_name,
                    enabled: entry.enabled,
                    addon_type: entry.addon_type,
                    file_size: entry.filesize,
                    has_image: has_local_image || has_cf_image || has_mr_image,
                    curseforge_project_id: cf.map(|c| c.project_id as u32),
                    modrinth_project_id: mr.map(|m| m.project_id.clone()),
                })
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
        use carbon_repos::db::server_mod_file_cache as sfcdb;

        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Look up the cache entry to get the filename
        let cache_entry = self
            .app
            .prisma_client
            .server_mod_file_cache()
            .find_unique(sfcdb::UniqueWhereParam::IdEquals(addon_id.clone()))
            .exec()
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
        let _ = self
            .app
            .prisma_client
            .server_mod_file_cache()
            .update(
                sfcdb::UniqueWhereParam::IdEquals(addon_id),
                vec![sfcdb::enabled::set(enabled)],
            )
            .exec()
            .await;

        self.app.invalidate(GET_SERVER_ADDONS, None);

        Ok(())
    }

    /// Delete a server addon file
    pub async fn delete_server_addon(self, id: ServerId, addon_id: String) -> anyhow::Result<()> {
        use carbon_repos::db::server_mod_file_cache as sfcdb;

        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        // Look up the cache entry to get the filename
        let cache_entry = self
            .app
            .prisma_client
            .server_mod_file_cache()
            .find_unique(sfcdb::UniqueWhereParam::IdEquals(addon_id.clone()))
            .exec()
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
        let _ = self
            .app
            .prisma_client
            .server_mod_file_cache()
            .delete(sfcdb::UniqueWhereParam::IdEquals(addon_id))
            .exec()
            .await;

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

        let mods_path = server_path.get_mods_path();
        tokio::fs::create_dir_all(&mods_path).await?;
        let install_path = mods_path.join(&file.file_name);

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
        use carbon_platforms::curseforge::filters::{ModFilesParameters, ModFilesParametersQuery};

        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let game_version = db_server.game_version.clone();
        let modloader_type = db_server.modloader_type.as_deref().and_then(|ml| match ml {
            "forge" => Some(carbon_platforms::curseforge::ModLoaderType::Forge),
            "fabric" => Some(carbon_platforms::curseforge::ModLoaderType::Fabric),
            "quilt" => Some(carbon_platforms::curseforge::ModLoaderType::Quilt),
            "neoforge" => Some(carbon_platforms::curseforge::ModLoaderType::NeoForge),
            _ => None,
        });

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

        let mods_path = server_path.get_mods_path();
        tokio::fs::create_dir_all(&mods_path).await?;
        let install_path = mods_path.join(&file.filename);

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

        let db_server = self
            .app
            .prisma_client
            .server()
            .find_unique(db::server::id::equals(id.0))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("Server not found"))?;

        let game_version = db_server.game_version.clone();
        let loaders = db_server.modloader_type.as_ref().map(|ml| vec![ml.clone()]);

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
        let file_path = server_path.get_data_path().join(list.filename());

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

            (
                ServerGroupId(server.group_id),
                server.index,
                server.library_position,
            )
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
                            WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Not(
                                None,
                            )),
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

                let lib_pos = target_folder
                    .library_position
                    .ok_or_else(|| anyhow!("Target folder has no libraryPosition"))?;

                let target_idx = self
                    .app
                    .prisma_client
                    .server()
                    .count(vec![WhereParam::GroupId(IntFilter::Equals(
                        default_group_id.0,
                    ))])
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
                            WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gt(
                                start_lib_pos,
                            )),
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
                self.app
                    .prisma_client
                    .server()
                    .update(UniqueWhereParam::IdEquals(server_id.0), update_params),
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
        use db::server::{SetParam as ServerSetParam, WhereParam as ServerWhereParam};
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
                    bail!(
                        "Can only position a group before ungrouped servers (servers in default group)"
                    );
                }

                server
                    .library_position
                    .ok_or_else(|| anyhow!("Server has no libraryPosition"))?
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
                        ServerWhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gt(
                            start_pos,
                        )),
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
                        ServerWhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Lt(
                            start_pos,
                        )),
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
                db::server::WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Not(
                    None,
                )),
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
        let default_group_id = self.get_default_group().await?;

        let _index_lock = self.index_lock.lock().await;

        // Get all servers in default group and sort by name
        let servers = self
            .app
            .prisma_client
            .server()
            .find_many(vec![db::server::group_id::equals(default_group_id.0)])
            .exec()
            .await?;

        let mut sortable_servers: Vec<(i32, String)> =
            servers.iter().map(|s| (s.id, s.name.clone())).collect();
        sortable_servers.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        // Non-default server groups, sorted by name — rendered after
        // ungrouped servers in the library.
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

        // Folders always come first, followed by ungrouped servers.
        // Frontend sorts top-level items by `libraryPosition ?? index`
        // (or `libraryPosition ?? 10000` for folders). Writing only
        // `index` leaves drag-reordered rows frozen. Stamp both fields so
        // the new order is visible whether library_position is set or not.
        let mut group_updates = Vec::new();
        group_updates.push(self.app.prisma_client.server_group().update(
            db::server_group::UniqueWhereParam::IdEquals(default_group_id.0),
            vec![db::server_group::group_index::set(0)],
        ));
        for (i, (group_id, _)) in sortable_groups.iter().enumerate() {
            let p = i as i32;
            group_updates.push(self.app.prisma_client.server_group().update(
                db::server_group::UniqueWhereParam::IdEquals(*group_id),
                vec![
                    db::server_group::group_index::set((i + 1) as i32),
                    db::server_group::library_position::set(Some(p)),
                ],
            ));
        }
        if !group_updates.is_empty() {
            self.app.prisma_client._batch(group_updates).await?;
        }

        let server_base = sortable_groups.len() as i32;
        let mut updates = Vec::new();
        for (i, (server_id, _)) in sortable_servers.iter().enumerate() {
            let p = server_base + i as i32;
            updates.push(self.app.prisma_client.server().update(
                db::server::UniqueWhereParam::IdEquals(*server_id),
                vec![
                    db::server::index::set(p),
                    db::server::library_position::set(Some(p)),
                ],
            ));
        }
        if !updates.is_empty() {
            self.app.prisma_client._batch(updates).await?;
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
