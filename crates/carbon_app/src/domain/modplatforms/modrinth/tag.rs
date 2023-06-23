//! Models related to tags

use super::*;

/// A category that projects of `project_type` specify
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Category {
    /// An SVG icon for the category
    pub icon: String,
    pub name: String,
    /// The project type this category is applicable to
    pub project_type: project::ProjectType,
    /// The header under which the category should go
    pub header: String,
}

/// A loader that can load projects of `project_type`
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Loader {
    /// An SVG icon for the loader
    pub icon: String,
    pub name: String,
    /// The project types that this loader can load
    pub supported_project_types: Vec<project::ProjectType>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GameVersion {
    pub version: String,
    /// The type of the game version
    pub version_type: GameVersionType,
    /// When the game version released
    pub date: UtcDateTime,
    /// Whether this game version was considered a major version
    ///
    /// This is set to true if this version introduced many breaking changes to internal APIs
    /// that causes most mods made for previous versions of the game to break on this version.
    pub major: bool,
}

/// The licenses that projects can be searched with
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct License {
    /// The SPDX license ID of a project
    pub short: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DonationPlatform {
    /// A short identifier for the donation platform
    pub short: String,
    pub name: String,
}

/// The type of a game version
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GameVersionType {
    Snapshot,
    Release,
    Beta,
    Alpha,
}
