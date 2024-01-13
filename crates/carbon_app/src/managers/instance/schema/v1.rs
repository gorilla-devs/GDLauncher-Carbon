use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn get_current_datetime() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instance {
    pub name: String,
    #[serde(default)]
    pub icon: InstanceIcon,
    #[serde(default = "get_current_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "get_current_datetime")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    pub seconds_played: u64,
    #[serde(default)]
    pub modpack: Option<Modpack>,
    pub game_configuration: GameConfig,
    #[serde(default)]
    pub mod_sources: Option<ModSources>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum InstanceIcon {
    Default,
    RelativePath(String),
}

impl Default for InstanceIcon {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "platform")]
pub enum Modpack {
    Curseforge(CurseforgeModpack),
    Modrinth(ModrinthModpack),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurseforgeModpack {
    pub project_id: u32,
    pub file_id: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthModpack {
    pub project_id: String,
    pub version_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModPlatform {
    Curseforge,
    Modrinth,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModChannel {
    Alpha,
    Beta,
    Stable,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModChannelWithUsage {
    pub channel: ModChannel,
    pub allow_updates: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub version: Option<GameVersion>,
    #[serde(default = "default_global_java_args")]
    pub global_java_args: bool,
    #[serde(default)]
    pub extra_java_args: Option<String>,
    #[serde(default)]
    pub memory: Option<MemoryRange>,
}

fn default_global_java_args() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GameVersion {
    Standard(StandardVersion),
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardVersion {
    pub release: String,
    #[serde(default)]
    pub modloaders: HashSet<ModLoader>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct ModLoader {
    #[serde(rename = "type")]
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ModLoaderType {
    Neoforge,
    Forge,
    Fabric,
    Quilt,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryRange {
    pub min_mb: u16,
    pub max_mb: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModSources {
    pub channels: Vec<ModChannelWithUsage>,
    pub platform_blacklist: Vec<ModPlatform>,
}

use crate::domain::{instance::info, modplatforms};

impl From<Instance> for info::Instance {
    fn from(value: Instance) -> Self {
        Self {
            name: value.name,
            icon: value.icon.into(),
            date_created: value.created_at,
            date_updated: value.updated_at,
            last_played: value.last_played,
            seconds_played: value.seconds_played,
            modpack: value.modpack.map(Into::into),
            game_configuration: value.game_configuration.into(),
            mod_sources: value.mod_sources.map(Into::into),
            notes: value.notes,
        }
    }
}

impl From<info::Instance> for Instance {
    fn from(value: info::Instance) -> Self {
        Self {
            name: value.name,
            icon: value.icon.into(),
            created_at: value.date_created,
            updated_at: value.date_updated,
            last_played: value.last_played,
            seconds_played: value.seconds_played,
            modpack: value.modpack.map(Into::into),
            game_configuration: value.game_configuration.into(),
            mod_sources: value.mod_sources.map(Into::into),
            notes: value.notes,
        }
    }
}

impl From<InstanceIcon> for info::InstanceIcon {
    fn from(value: InstanceIcon) -> Self {
        use InstanceIcon as Schema;

        match value {
            Schema::Default => Self::Default,
            Schema::RelativePath(path) => Self::RelativePath(path),
        }
    }
}

impl From<info::InstanceIcon> for InstanceIcon {
    fn from(value: info::InstanceIcon) -> Self {
        use info::InstanceIcon as Info;

        match value {
            Info::Default => Self::Default,
            Info::RelativePath(path) => Self::RelativePath(path),
        }
    }
}

impl From<Modpack> for info::Modpack {
    fn from(value: Modpack) -> Self {
        match value {
            Modpack::Curseforge(cf) => Self::Curseforge(cf.into()),
            Modpack::Modrinth(mdr) => Self::Modrinth(mdr.into()),
        }
    }
}

impl From<info::Modpack> for Modpack {
    fn from(value: info::Modpack) -> Self {
        match value {
            info::Modpack::Curseforge(cf) => Self::Curseforge(cf.into()),
            info::Modpack::Modrinth(mdr) => Self::Modrinth(mdr.into()),
        }
    }
}

impl From<CurseforgeModpack> for info::CurseforgeModpack {
    fn from(value: CurseforgeModpack) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
        }
    }
}

impl From<info::CurseforgeModpack> for CurseforgeModpack {
    fn from(value: info::CurseforgeModpack) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
        }
    }
}

impl From<ModrinthModpack> for info::ModrinthModpack {
    fn from(value: ModrinthModpack) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
        }
    }
}

impl From<info::ModrinthModpack> for ModrinthModpack {
    fn from(value: info::ModrinthModpack) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
        }
    }
}

impl From<GameConfig> for info::GameConfig {
    fn from(value: GameConfig) -> Self {
        Self {
            version: value.version.map(Into::into),
            global_java_args: value.global_java_args,
            extra_java_args: value.extra_java_args,
            memory: value.memory.map(Into::into),
        }
    }
}

impl From<info::GameConfig> for GameConfig {
    fn from(value: info::GameConfig) -> Self {
        Self {
            version: value.version.map(Into::into),
            global_java_args: value.global_java_args,
            extra_java_args: value.extra_java_args,
            memory: value.memory.map(Into::into),
        }
    }
}

impl From<GameVersion> for info::GameVersion {
    fn from(value: GameVersion) -> Self {
        use GameVersion as Schema;

        match value {
            Schema::Standard(v) => Self::Standard(v.into()),
            Schema::Custom(v) => Self::Custom(v),
        }
    }
}

impl From<info::GameVersion> for GameVersion {
    fn from(value: info::GameVersion) -> Self {
        use info::GameVersion as Info;

        match value {
            Info::Standard(v) => Self::Standard(v.into()),
            Info::Custom(v) => Self::Custom(v),
        }
    }
}

impl From<StandardVersion> for info::StandardVersion {
    fn from(value: StandardVersion) -> Self {
        Self {
            release: value.release,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<info::StandardVersion> for StandardVersion {
    fn from(value: info::StandardVersion) -> Self {
        Self {
            release: value.release,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ModLoader> for info::ModLoader {
    fn from(value: ModLoader) -> Self {
        Self {
            type_: value.type_.into(),
            version: value.version,
        }
    }
}

impl From<info::ModLoader> for ModLoader {
    fn from(value: info::ModLoader) -> Self {
        Self {
            type_: value.type_.into(),
            version: value.version,
        }
    }
}

impl From<ModLoaderType> for info::ModLoaderType {
    fn from(value: ModLoaderType) -> Self {
        use ModLoaderType as Schema;

        match value {
            Schema::Neoforge => Self::Neoforge,
            Schema::Forge => Self::Forge,
            Schema::Fabric => Self::Fabric,
            Schema::Quilt => Self::Quilt,
        }
    }
}

impl From<info::ModLoaderType> for ModLoaderType {
    fn from(value: info::ModLoaderType) -> Self {
        use info::ModLoaderType as Info;

        match value {
            Info::Neoforge => Self::Neoforge,
            Info::Forge => Self::Forge,
            Info::Fabric => Self::Fabric,
            Info::Quilt => Self::Quilt,
        }
    }
}

impl From<MemoryRange> for (u16, u16) {
    fn from(value: MemoryRange) -> Self {
        (value.min_mb, value.max_mb)
    }
}

impl From<(u16, u16)> for MemoryRange {
    fn from(value: (u16, u16)) -> Self {
        Self {
            min_mb: value.0,
            max_mb: value.1,
        }
    }
}

use crate::mirror_into;

mirror_into!(
    ModPlatform,
    modplatforms::ModPlatform,
    |value| match value {
        Other::Curseforge => Self::Curseforge,
        Other::Modrinth => Self::Modrinth,
    }
);

mirror_into!(ModChannel, modplatforms::ModChannel, |value| match value {
    Other::Alpha => Self::Alpha,
    Other::Beta => Self::Beta,
    Other::Stable => Self::Stable,
});

mirror_into!(
    ModChannelWithUsage,
    modplatforms::ModChannelWithUsage,
    |value| Self {
        channel: value.channel.into(),
        allow_updates: value.allow_updates,
    }
);

mirror_into!(ModSources, modplatforms::ModSources, |value| Self {
    channels: value.channels.into_iter().map(Into::into).collect(),
    platform_blacklist: value
        .platform_blacklist
        .into_iter()
        .map(Into::into)
        .collect(),
});
