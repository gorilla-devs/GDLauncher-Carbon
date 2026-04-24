use chrono::{DateTime, Utc};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;

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
    pub seconds_played: u32,
    #[serde(default)]
    pub modpack: Option<ModpackInfo>,
    pub game_configuration: GameConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_launch_hook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_exit_hook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_command: Option<String>,
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
pub struct ModpackInfo {
    #[serde(flatten)]
    pub modpack: Modpack,
    #[serde(default)]
    pub locked: bool,
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
#[serde(tag = "type", content = "value")]
pub enum GameResolution {
    Standard(u16, u16),
    Custom(u16, u16),
}

fn serialize_resolution<S>(
    game_resolution: &Option<GameResolution>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match game_resolution {
        Some(GameResolution::Standard(width, height)) => {
            serializer.serialize_str(&format!("standard:{}x{}", width, height))
        }
        Some(GameResolution::Custom(width, height)) => {
            serializer.serialize_str(&format!("custom:{}x{}", width, height))
        }
        None => serializer.serialize_none(),
    }
}

// Custom deserialization function for GameResolution
fn deserialize_resolution<'de, D>(deserializer: D) -> Result<Option<GameResolution>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ResolutionVisitor;

    impl<'de> Visitor<'de> for ResolutionVisitor {
        type Value = Option<GameResolution>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a game_resolution string in the format `type:widthxheight`")
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<GameResolution>, E>
        where
            E: de::Error,
        {
            let parts: Vec<&str> = value.split(':').collect();
            if parts.len() != 2 {
                return Err(E::custom("invalid format"));
            }

            let size_parts: Vec<&str> = parts[1].split('x').collect();
            if size_parts.len() != 2 {
                return Err(E::custom("invalid size format"));
            }

            let width: u16 = size_parts[0].parse().map_err(de::Error::custom)?;
            let height: u16 = size_parts[1].parse().map_err(de::Error::custom)?;

            match parts[0] {
                "standard" => Ok(Some(GameResolution::Standard(width, height))),
                "custom" => Ok(Some(GameResolution::Custom(width, height))),
                _ => Err(E::custom("unknown game_resolution type")),
            }
        }
    }

    deserializer.deserialize_str(ResolutionVisitor)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum JavaOverride {
    Profile(Option<String>),
    Path(Option<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<GameVersion>,
    #[serde(default = "default_global_java_args")]
    pub global_java_args: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_java_args: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryRange>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_resolution")]
    #[serde(serialize_with = "serialize_resolution")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_resolution: Option<GameResolution>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_override: Option<JavaOverride>,
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
