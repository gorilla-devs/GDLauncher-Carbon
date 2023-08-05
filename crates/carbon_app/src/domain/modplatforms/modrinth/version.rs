//! Models related to versions
//!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/version_model)

use super::*;
use std::collections::{HashMap, HashSet};

use crate::domain::{
    instance::info::{ModLoader, ModLoaderType, StandardVersion},
    modplatforms::ModChannel,
};

use anyhow::anyhow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackIndex {
    pub format_version: u32,
    pub game: ModrinthGame,
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<ModrinthFile>,
    pub dependencies: ModrinthPackDependencies,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModrinthGame {
    Minecraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthFile {
    /// path relative to the Minecraft instance directory
    pub path: String,
    pub hashes: Hashes,
    pub env: Option<ModrinthFileEnvironment>,
    /// list of valid https URLs to the file. Each url is a full path. Functions as a mirror list.
    pub downloads: Vec<String>,
    pub file_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthFileEnvironment {
    pub client: ModrinthEnvironmentSupport,
    pub server: ModrinthEnvironmentSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModrinthEnvironmentSupport {
    Required,
    Unsupported,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModrinthPackDependencies {
    pub minecraft: Option<String>,
    #[serde(flatten)]
    pub loaders: HashMap<String, String>,
}

impl TryFrom<ModrinthPackDependencies> for StandardVersion {
    type Error = anyhow::Error;

    fn try_from(value: ModrinthPackDependencies) -> Result<Self, Self::Error> {
        let minecraft_version = value
            .minecraft
            .ok_or_else(|| anyhow!("Modpack does not have a Minecraft version listed"))?;
        let mut modloaders = HashSet::new();
        for (key, version) in value.loaders.into_iter() {
            let modloader_name = key.strip_suffix("_loader").unwrap_or(&key);
            modloaders.insert(ModLoader {
                type_: modloader_name.parse()?,
                version: format!("{}-{}", &minecraft_version, version),
            });
        }

        Ok(StandardVersion {
            release: minecraft_version,
            modloaders,
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Version {
    pub name: String,
    /// The version number.
    /// Ideally, this will follow semantic versioning.
    pub version_number: String,
    pub changelog: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub game_versions: Vec<String>,
    /// The release channel for this version
    pub version_type: VersionType,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub status: Option<Status>,
    pub requested_status: Option<RequestedVersionStatus>,
    pub id: String,
    /// The ID of the project this version is for
    pub project_id: String,
    /// The ID of the author who published this version
    pub author_id: String,
    pub date_published: UtcDateTime,
    pub downloads: u32,
    /// A link to the version's changelog (only present for old versions)
    #[deprecated = "Read from `changelog` instead"]
    pub changelog_url: Option<String>,
    /// A list of files available for download
    pub files: Vec<VersionFile>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionFile {
    pub hashes: Hashes,
    pub url: String,
    pub filename: String,
    /// Whether the file is the primary file of its version.
    ///
    /// There can only be a maximum of one primary file per version.
    /// If there are no primary files specified, the first file can be taken as the primary file.
    pub primary: bool,
    /// The size of the file in bytes
    pub size: u32,
    /// The type of the additional file, used mainly for adding resource packs to datapacks
    pub file_type: Option<AdditionalFileType>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Hashes {
    pub sha512: String,
    pub sha1: String,
    /// A map of other hashes that may have been provided
    #[serde(flatten)]
    pub others: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LatestVersionBody {
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LatestVersionsBody {
    pub hashes: Vec<String>,
    pub algorithm: HashAlgorithm,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: DependencyType,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    SHA512,
    SHA1,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VersionType {
    Alpha,
    Beta,
    Release,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RequestedVersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AdditionalFileType {
    RequiredResourcePack,
    OptionalResourcePack,
}

impl From<VersionType> for ModChannel {
    fn from(value: VersionType) -> Self {
        match value {
            VersionType::Alpha => ModChannel::Alpha,
            VersionType::Beta => ModChannel::Beta,
            VersionType::Release => ModChannel::Stable,
        }
    }
}
