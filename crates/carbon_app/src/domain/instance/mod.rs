use chrono::{DateTime, Utc};

use crate::managers::vtask::VisualTaskId;

pub mod info;

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
    Running { start_time: DateTime<Utc> },
}

pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

pub enum ModLoaderType {
    Forge,
    Fabirc,
}
