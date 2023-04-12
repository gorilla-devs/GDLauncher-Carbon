//! Schema for instance jsons

use std::collections::HashSet;

use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Instance {
    pub name: String,
    pub icon: InstanceIcon,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u64,
    pub modpack: Option<Modpack>,
    pub game_configuration: GameConfig,
    pub notes: String,
}

#[derive(Debug)]
pub enum InstanceIcon {
    Default,
    RelativePath(String),
}

#[derive(Debug)]
pub enum Modpack {
    Curseforge(CurseforgeModpack),
}

#[derive(Debug)]
pub struct CurseforgeModpack {
    pub project_id: String,
    pub file_id: String,
}

#[derive(Debug)]
pub struct GameConfig {
    pub version: GameVersion,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub memory: Option<(u64, u64)>,
}

#[derive(Debug)]
pub enum GameVersion {
    Standard(StandardVersion),
    Custom(String),
}

#[derive(Debug)]
pub struct StandardVersion {
    pub release: String,
    pub modloaders: HashSet<ModLoader>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ModLoaderType {
    Forge,
    Fabric,
}
