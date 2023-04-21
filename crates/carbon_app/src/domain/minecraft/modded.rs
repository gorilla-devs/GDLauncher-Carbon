use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::minecraft::{Library, VersionArguments, VersionType};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModdedManifest {
    pub game_versions: Vec<ModdedManifestVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModdedManifestVersion {
    pub id: String,
    pub stable: bool,
    pub loaders: Vec<ModdedManifestLoaderVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModdedManifestLoaderVersion {
    pub id: String,
    pub url: String,
    pub stable: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// A partial version returned by fabric meta
pub struct PartialVersionInfo {
    pub id: String,
    pub inherits_from: String,
    pub release_time: DateTime<Utc>,
    pub time: DateTime<Utc>,
    pub main_class: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<VersionArguments>,
    pub libraries: Vec<Library>,
    #[serde(rename = "type")]
    pub type_: VersionType,
    /// (Forge-only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, SidedDataEntry>>,
    /// (Forge-only) The list of processors to run after downloading the files
    pub processors: Option<Vec<Processor>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Processor {
    /// Maven coordinates for the JAR library of this processor.
    pub jar: String,
    /// Maven coordinates for all the libraries that must be included in classpath when running this processor.
    pub classpath: Vec<String>,
    /// Arguments for this processor.
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Represents a map of outputs. Keys and values can be data values
    pub outputs: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Which sides this processor shall be ran on.
    /// Valid values: client, server, extract
    pub sides: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SidedDataEntry {
    /// The value on the client
    pub client: String,
    /// The value on the server
    pub server: String,
}
