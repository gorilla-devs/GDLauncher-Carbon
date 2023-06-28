use crate::domain::vtask::VisualTaskId;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod info;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GroupId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct InstanceId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct GameLogId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct InstanceModId(pub Uuid);

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mod {
    pub id: String,
    pub filename: String,
    pub enabled: bool,
    pub metadata: Option<ModFileMetadata>,
    pub curseforge: Option<CurseForgeModMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFileMetadata {
    pub modid: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: Vec<info::ModLoaderType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeModMetadata {
    pub project_id: u32,
    pub file_id: u32,
    pub name: String,
    pub urlslug: String,
    pub summary: String,
    pub authors: String,
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
