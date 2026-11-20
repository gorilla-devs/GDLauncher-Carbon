//! Schema for instance jsons

use anyhow::bail;
use carbon_platforms::{ModPlatform, ModSources};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub icon: InstanceIcon,
    pub date_created: DateTime<Utc>,
    pub date_updated: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub seconds_played: u32,
    pub modpack: Option<ModpackInfo>,
    pub game_configuration: GameConfig,
    pub pre_launch_hook: Option<String>,
    pub post_exit_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub mod_sources: Option<ModSources>,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub enum InstanceIcon {
    Default,
    RelativePath(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ModpackInfo {
    pub modpack: Modpack,
    pub locked: bool,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Modpack {
    Curseforge(CurseforgeModpack),
    Modrinth(ModrinthModpack),
}

impl ToString for Modpack {
    fn to_string(&self) -> String {
        match self {
            Self::Curseforge(_) => "curseforge".to_string(),
            Self::Modrinth(_) => "modrinth".to_string(),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CurseforgeModpack {
    pub project_id: u32,
    pub file_id: u32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ModrinthModpack {
    pub project_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum GameResolution {
    Standard(u16, u16),
    Custom(u16, u16),
}

#[derive(Debug, Clone)]
pub enum JavaOverride {
    Profile(Option<String>),
    Path(Option<String>),
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub version: Option<GameVersion>,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub java_override: Option<JavaOverride>,
    /// The oldest Java major this instance runs on, as a wrong-Java crash
    /// showed: what fits it and what Automatic picks start there. Cleared
    /// when the Minecraft version or the modloader changes.
    pub java_floor: Option<u8>,
    pub memory: Option<(u16, u16)>,
    pub game_resolution: Option<GameResolution>,
}

impl GameConfig {
    /// Sets the Minecraft version and modloaders. The Java floor goes when
    /// the release or the modloaders change, since a crash of the old ones
    /// taught it; a newer build of the same modloader keeps it.
    pub fn set_version(&mut self, version: Option<GameVersion>) {
        if java_floor_basis(self.version.as_ref()) != java_floor_basis(version.as_ref()) {
            self.java_floor = None;
        }
        self.version = version;
    }
}

/// What a Java floor holds for: the release, and which modloaders.
fn java_floor_basis(version: Option<&GameVersion>) -> Option<(&str, HashSet<ModLoaderType>)> {
    match version? {
        GameVersion::Standard(version) => Some((
            version.release.as_str(),
            version
                .modloaders
                .iter()
                .map(|loader| loader.type_)
                .collect(),
        )),
        GameVersion::Custom(custom) => Some((custom.as_str(), HashSet::new())),
    }
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
    Neoforge,
    Forge,
    Fabric,
    Quilt,
}

impl ToString for ModLoaderType {
    fn to_string(&self) -> String {
        match self {
            Self::Neoforge => "neoforge",
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
            "neoforge" => Ok(Self::Neoforge),
            "forge" => Ok(Self::Forge),
            "fabric" => Ok(Self::Fabric),
            "quilt" => Ok(Self::Quilt),
            _ => bail!("unknown modloader type {s}"),
        }
    }
}

impl Modpack {
    pub fn as_platform(&self) -> ModPlatform {
        match self {
            Self::Curseforge(_) => ModPlatform::Curseforge,
            Self::Modrinth(_) => ModPlatform::Modrinth,
        }
    }
}

impl From<ModLoaderType> for carbon_platforms::curseforge::ModLoaderType {
    fn from(value: ModLoaderType) -> Self {
        match value {
            ModLoaderType::Neoforge => Self::NeoForge,
            ModLoaderType::Forge => Self::Forge,
            ModLoaderType::Fabric => Self::Fabric,
            ModLoaderType::Quilt => Self::Quilt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(release: &str, loader: Option<(ModLoaderType, &str)>) -> Option<GameVersion> {
        Some(GameVersion::Standard(StandardVersion {
            release: release.into(),
            modloaders: loader
                .map(|(type_, version)| ModLoader {
                    type_,
                    version: version.into(),
                })
                .into_iter()
                .collect(),
        }))
    }

    fn with_floor(version: Option<GameVersion>) -> GameConfig {
        GameConfig {
            version,
            global_java_args: true,
            extra_java_args: None,
            java_override: None,
            java_floor: Some(21),
            memory: None,
            game_resolution: None,
        }
    }

    #[test]
    fn a_new_minecraft_version_or_modloader_clears_the_java_floor() {
        let fabric = |v| Some((ModLoaderType::Fabric, v));
        let floor_after = |from, to| {
            let mut config = with_floor(from);
            config.set_version(to);
            config.java_floor
        };

        // The same version again, or a newer build of its modloader.
        assert_eq!(
            floor_after(
                version("1.20.1", fabric("0.15.0")),
                version("1.20.1", fabric("0.15.0"))
            ),
            Some(21)
        );
        assert_eq!(
            floor_after(
                version("1.20.1", fabric("0.15.0")),
                version("1.20.1", fabric("0.16.9"))
            ),
            Some(21)
        );

        // Another Minecraft version, another modloader, or none.
        assert_eq!(
            floor_after(
                version("1.20.1", fabric("0.15.0")),
                version("1.21.1", fabric("0.15.0"))
            ),
            None
        );
        assert_eq!(
            floor_after(
                version("1.20.1", fabric("0.15.0")),
                version("1.20.1", Some((ModLoaderType::Forge, "47.3.0")))
            ),
            None
        );
        assert_eq!(
            floor_after(version("1.20.1", fabric("0.15.0")), version("1.20.1", None)),
            None
        );
    }
}
