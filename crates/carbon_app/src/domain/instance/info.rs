//! Schema for instance jsons

use std::{collections::HashSet, str::FromStr};

use anyhow::bail;
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
    Modrinth(ModrinthModpack),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ModpackPlatform {
    Curseforge,
    Modrinth,
}

#[derive(Debug, Clone)]
pub enum CurseforgeModpack {
    RemoteManaged {
        project_id: i32,
        file_id: i32,
    },
    LocalManaged {
        project_id: i32,
        file_id: i32,
        archive_path: String,
    },
    Unmanaged {
        archive_path: String,
    },
}

#[derive(Debug, Clone)]
pub enum ModrinthModpack {
    RemoteManaged {
        project_id: String,
        version_id: String,
    },
    LocalManaged {
        project_id: String,
        version_id: String,
        mrpack_path: String,
    },
    Unmanaged {
        mrpack_path: String,
    },
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub version: Option<GameVersion>,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub memory: Option<(u16, u16)>,
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
    Quilt,
}

impl ToString for ModLoaderType {
    fn to_string(&self) -> String {
        match self {
            Self::Forge => "forge",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
        }
        .to_string()
    }
}

impl TryFrom<&str> for ModLoaderType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "forge" => Ok(Self::Forge),
            "fabric" => Ok(Self::Fabric),
            "quilt" => Ok(Self::Quilt),
            _ => bail!("unknown modloader type {s}"),
        }
    }
}

impl FromStr for ModLoaderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl Modpack {
    pub fn as_platform(&self) -> ModpackPlatform {
        match self {
            Self::Curseforge(_) => ModpackPlatform::Curseforge,
            Self::Modrinth(_) => ModpackPlatform::Modrinth,
        }
    }
}
