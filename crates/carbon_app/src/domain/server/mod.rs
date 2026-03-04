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
}

impl ServerType {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
        }
    }

    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "vanilla" => Some(Self::Vanilla),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub name: String,
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
}

#[derive(Debug)]
pub struct ServerSettingsUpdate {
    pub server_id: ServerId,
    pub name: Option<String>,
    pub port: Option<i32>,
    pub motd: Option<String>,
    pub max_players: Option<i32>,
    pub online_mode: Option<bool>,
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
}

#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}
