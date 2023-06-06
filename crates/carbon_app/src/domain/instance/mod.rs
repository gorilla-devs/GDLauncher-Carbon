use crate::domain::vtask::VisualTaskId;
use chrono::{DateTime, Utc};

pub mod info;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GroupId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct InstanceId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct GameLogId(pub i32);

#[derive(Clone, Debug)]
pub struct GameLogEntry {
    pub id: GameLogId,
    pub instance_id: InstanceId,
    pub active: bool,
}

pub struct InstanceDetails {
    pub favorite: bool,
    pub name: String,
    pub version: Option<String>,
    pub modpack: Option<info::Modpack>,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub memory: Option<(u16, u16)>,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u32,
    pub modloaders: Vec<info::ModLoader>,
    pub state: LaunchState,
    pub notes: String,
    pub mods: Vec<Mod>,
}

pub struct InstanceSettingsUpdate {
    pub instance_id: InstanceId,
    pub name: Option<String>,
    pub use_loaded_icon: Option<bool>,
    pub notes: Option<String>,
    pub version: Option<String>,
    pub modloader: Option<Option<info::ModLoader>>,
    pub global_java_args: Option<bool>,
    pub extra_java_args: Option<Option<String>>,
    pub memory: Option<Option<(u16, u16)>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchState {
    Inactive {
        failed_task: Option<VisualTaskId>,
    },
    Preparing(VisualTaskId),
    Running {
        start_time: DateTime<Utc>,
        log_id: GameLogId,
    },
}

pub struct Mod {
    pub id: String,
    pub filename: String,
    pub enabled: bool,
    pub modloader: info::ModLoaderType,
    pub metadata: ModFileMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFileMetadata {
    pub modid: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum InstanceFolder {
    Root,
    Data,
    Mods,
    Configs,
    Screenshots,
    Saves,
    Logs,
    CrashReports,
    ResourcePacks,
    TexturePacks,
    ShaderPacks,
}
