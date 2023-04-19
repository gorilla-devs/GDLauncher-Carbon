//! Schema for instance jsons

use std::collections::HashSet;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub icon: InstanceIcon,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u64,
    pub modpack: Option<Modpack>,
    pub game_configuration: GameConfig,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub enum InstanceIcon {
    Default,
    RelativePath(String),
}

#[derive(Debug, Clone)]
pub enum Modpack {
    Curseforge(CurseforgeModpack),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ModpackPlatform {
    Curseforge,
}

#[derive(Debug, Clone)]
pub struct CurseforgeModpack {
    pub project_id: String,
    pub file_id: String,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub version: GameVersion,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub memory: Option<(u64, u64)>,
}

#[derive(Debug, Clone)]
pub enum GameVersion {
    Standard(StandardVersion),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct StandardVersion {
    pub release: String,
    pub modloaders: HashSet<ModLoader>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum ModLoaderType {
    Forge,
    Fabric,
}

impl Modpack {
    pub fn as_platform(&self) -> ModpackPlatform {
        match self {
            Self::Curseforge(_) => ModpackPlatform::Curseforge,
        }
    }
}
