use crate::domain::vtask::VisualTaskId;
use chrono::{DateTime, Utc};

pub mod info;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct GameLogId(pub i32);

pub struct InstanceDetails {
    pub favorite: bool,
    pub name: String,
    pub version: Option<String>,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u32,
    pub modloaders: Vec<ModLoader>,
    pub state: LaunchState,
    pub notes: String,
    pub mods: Vec<Mod>,
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

pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Debug, Copy, Clone)]
pub enum ModLoaderType {
    Forge,
    Fabirc,
}

pub struct Mod {
    pub id: String,
    pub filename: String,
    pub enabled: bool,
    pub modloader: ModLoaderType,
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
