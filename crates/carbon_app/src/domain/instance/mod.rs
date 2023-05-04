use carbon_domain::vtask::VisualTaskId;
use chrono::{DateTime, Utc};

pub mod info;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct GameLogId(pub i32);

pub struct InstanceDetails {
    pub favorite: bool,
    pub name: String,
    pub version: String,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u32,
    pub modloaders: Vec<ModLoader>,
    pub state: LaunchState,
    pub notes: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchState {
    Inactive,
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

pub enum ModLoaderType {
    Forge,
    Fabirc,
}
