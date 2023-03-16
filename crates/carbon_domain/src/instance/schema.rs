//! Schema for instance jsons

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub icon: InstanceIcon,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u64,
    pub modpack: Option<Modpack>,
    pub game_configuration: GameConfig,
    pub notes: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InstanceIcon {
    Default,
    RelativePath(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "platform")]
pub enum Modpack {
    Curseforge(CurseforgeModpack),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurseforgeModpack {
    pub project_id: String,
    pub file_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameConfig {
    pub version: GameVersion,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub memory: Option<MemoryRange>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GameVersion {
    Standard(StandardVersion),
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StandardVersion {
    pub release: String,
    pub modloaders: HashSet<ModLoader>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModLoader {
    #[serde(rename = "type")]
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModLoaderType {
    Forge,
    Fabric,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryRange {
    pub min_mb: u64,
    pub max_mb: u64,
}
