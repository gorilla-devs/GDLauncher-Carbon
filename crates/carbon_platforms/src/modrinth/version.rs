//! Models related to versions
//!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/version_model)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ModChannel;

use super::UtcDateTime;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minecraft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neoforge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fabric_loader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quilt_loader: Option<String>,
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
    #[serde(deserialize_with = "deserialize_lowercase_hex")]
    pub sha512: String,
    #[serde(deserialize_with = "deserialize_lowercase_hex")]
    pub sha1: String,
    /// A map of other hashes that may have been provided
    #[serde(flatten)]
    pub others: HashMap<String, String>,
}

/// Folds a hex digest to lowercase as it is read.
///
/// Modrinth's API is not guaranteed to return these in any particular case,
/// but callers both compare them against locally-computed digests (always
/// lowercase — `hex::encode`) and key maps by them, so a digest that arrives
/// differently-cased than a local one would fail an `==` comparison, or miss
/// a map entry keyed by the other casing, despite being the same hash.
/// Folding once here leaves every downstream use working on one casing —
/// the same fold `gdlpack`'s `FileHashes` applies for the same reason.
///
/// ASCII-only on purpose: a hex digest is ASCII, and full Unicode folding
/// would let a locale-specific rule alter a character it should leave alone.
fn deserialize_lowercase_hex<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    Ok(hex.to_ascii_lowercase())
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
    /// Restricts the answer to these release channels. Absent from Modrinth's
    /// published schema for this route but honoured by it, and omitted from the
    /// request entirely when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_types: Option<Vec<VersionType>>,
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
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RequestedVersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AdditionalFileType {
    RequiredResourcePack,
    OptionalResourcePack,
    SourcesJar,
    DevJar,
    JavadocJar,
    Signature,
    #[serde(other)]
    Unknown,
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

impl From<ModChannel> for VersionType {
    fn from(value: ModChannel) -> Self {
        match value {
            ModChannel::Alpha => VersionType::Alpha,
            ModChannel::Beta => VersionType::Beta,
            ModChannel::Stable => VersionType::Release,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Modrinth's own API is not guaranteed to echo hex digests in any
    /// particular case; callers compare `sha512`/`sha1` against
    /// locally-computed (always-lowercase, `hex::encode`) digests, so both
    /// must land lowercase regardless of how the API spelled them.
    #[test]
    fn mixed_case_hashes_are_read_as_lowercase() {
        let json = r#"{
            "hashes": { "sha512": "AB12cd34", "sha1": "EF34ab", "crc32": "DEADBEEF" },
            "url": "https://cdn.modrinth.com/data/AAAA/versions/1/mod.jar",
            "filename": "mod.jar",
            "primary": true,
            "size": 1024,
            "file_type": null
        }"#;

        let file: VersionFile = serde_json::from_str(json).unwrap();

        assert_eq!(file.hashes.sha512, "ab12cd34");
        assert_eq!(file.hashes.sha1, "ef34ab");
        // The flattened catch-all is untouched: only the two named,
        // compared-against fields are folded.
        assert_eq!(
            file.hashes.others.get("crc32").map(String::as_str),
            Some("DEADBEEF")
        );
    }
}
