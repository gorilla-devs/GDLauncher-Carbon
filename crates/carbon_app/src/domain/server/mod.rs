use crate::domain::vtask::VisualTaskId;
use chrono::{DateTime, Utc};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ServerGroupId(pub i32);

impl std::ops::Deref for ServerGroupId {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ServerId(pub i32);

impl std::ops::Deref for ServerId {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct ServerLogId(pub i32);

impl From<i32> for ServerLogId {
    fn from(id: i32) -> Self {
        ServerLogId(id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ServerState {
    Stopped {
        failed_task: Option<VisualTaskId>,
    },
    Installing(VisualTaskId),
    Starting(VisualTaskId),
    Running {
        start_time: DateTime<Utc>,
        log_id: ServerLogId,
        process_id: u32,
    },
    Stopping,
    Deleting,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Vanilla,
    Forge,
    NeoForge,
    Fabric,
    Quilt,
}

impl ServerType {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
        }
    }

    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "vanilla" => Some(Self::Vanilla),
            "forge" => Some(Self::Forge),
            "neoforge" => Some(Self::NeoForge),
            "fabric" => Some(Self::Fabric),
            "quilt" => Some(Self::Quilt),
            _ => None,
        }
    }

    /// Construct from database fields (serverType column + modloaderType column)
    pub fn from_db_fields(server_type: &str, modloader_type: Option<&str>) -> Option<Self> {
        match modloader_type {
            Some(ml) => Self::from_db_string(ml),
            None => Self::from_db_string(server_type),
        }
    }

    pub fn is_modded(&self) -> bool {
        !matches!(self, Self::Vanilla)
    }
}

#[derive(Debug, Clone)]
pub struct ServerDetails {
    pub id: ServerId,
    pub name: String,
    pub favorite: bool,
    pub server_type: ServerType,
    pub game_version: String,
    pub port: i32,
    pub motd: String,
    pub max_players: i32,
    pub online_mode: bool,
    pub xmx: i32,
    pub xms: i32,
    pub extra_java_args: String,
    pub auto_restart: bool,
    pub date_created: DateTime<Utc>,
    pub last_started: Option<DateTime<Utc>>,
    pub state: ServerState,
    pub icon_revision: Option<u32>,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    pub modpack_info: Option<ServerModpackInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerModpackInfo {
    pub platform: String,
    pub project_id: String,
    pub file_id: String,
}

#[derive(Debug)]
pub struct ServerSettingsUpdate {
    pub server_id: ServerId,
    pub name: Option<String>,
    pub xmx: Option<i32>,
    pub xms: Option<i32>,
    pub extra_java_args: Option<Option<String>>,
    pub auto_restart: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ServerGroup {
    pub id: ServerGroupId,
    pub name: String,
    pub group_index: i32,
    pub library_position: Option<i32>,
    pub servers: Vec<ServerListEntry>,
}

#[derive(Debug, Clone)]
pub struct ServerListEntry {
    pub id: ServerId,
    pub group_id: ServerGroupId,
    pub index: i32,
    pub library_position: Option<i32>,
    pub name: String,
    pub favorite: bool,
    pub server_type: ServerType,
    pub game_version: String,
    pub port: i32,
    pub date_created: DateTime<Utc>,
    pub last_started: Option<DateTime<Utc>>,
    pub icon_revision: Option<u32>,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    pub modpack_info: Option<ServerModpackInfo>,
}

#[derive(Debug)]
pub enum ServerMoveTarget {
    BeforeServer(ServerId),
    EndOfGroup(ServerGroupId),
    BeforeGroup(ServerGroupId),
}

#[derive(Debug)]
pub enum ServerGroupMoveTarget {
    BeforeGroup(ServerGroupId),
    BeforeServer(ServerId),
    EndOfLibrary,
}

#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

// Player list types for whitelist.json, ops.json, banned-players.json

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpsEntry {
    pub uuid: String,
    pub name: String,
    pub level: i32,
    #[serde(default)]
    pub bypasses_player_limit: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BannedPlayerEntry {
    pub uuid: String,
    pub name: String,
    pub created: String,
    pub source: String,
    pub expires: String,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BannedIpEntry {
    pub ip: String,
    pub created: String,
    pub source: String,
    pub expires: String,
    pub reason: String,
}

/// A server addon (mod or datapack) from the database cache
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerAddon {
    pub id: String,
    pub filename: String,
    pub display_name: String,
    pub enabled: bool,
    pub addon_type: String,
    pub file_size: i32,
    pub has_image: bool,
    pub curseforge_project_id: Option<u32>,
    pub modrinth_project_id: Option<String>,
}

/// Launch configuration for modded servers, persisted to modloader_config.json
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaunchConfig {
    /// Java argument file to expand at launch (e.g.
    /// "libraries/net/neoforged/neoforge/21.1.77/unix_args.txt"), relative to
    /// the data dir. Passed to the JVM as `@<path>`, which is how Forge's and
    /// NeoForge's own run.sh/run.bat launch a server. The JVM tokenizes the
    /// file itself, so the module path, main class and game args all come from
    /// it and none of the other fields are needed.
    ///
    /// Takes precedence over `main_class` and `jar_path` when set.
    #[serde(default)]
    pub args_file: Option<String>,
    /// Override for server.jar path (e.g. "fabric-server-launch.jar"), relative to data dir
    pub jar_path: Option<String>,
    /// Override main class (Forge/NeoForge use this)
    pub main_class: Option<String>,
    /// Additional classpath entries (library paths)
    pub classpath: Vec<String>,
    /// Additional JVM arguments
    pub extra_jvm_args: Vec<String>,
    /// Additional game arguments (after main class)
    pub extra_game_args: Vec<String>,
}

impl LaunchConfig {
    pub fn vanilla() -> Self {
        Self {
            args_file: None,
            jar_path: None,
            main_class: None,
            classpath: Vec::new(),
            extra_jvm_args: Vec::new(),
            extra_game_args: Vec::new(),
        }
    }

    /// Launch via a Forge/NeoForge argument file.
    pub fn from_args_file(relative_path: String) -> Self {
        Self {
            args_file: Some(relative_path),
            ..Self::vanilla()
        }
    }
}
