use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::domain::instance::info::{ModLoader, ModLoaderType, StandardVersion};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub minecraft: Minecraft,
    pub manifest_type: String,
    pub manifest_version: i32,
    pub name: String,
    pub version: Option<String>,
    pub author: String,
    pub overrides: String,
    pub files: Vec<ManifestFileReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Minecraft {
    pub version: String,
    pub mod_loaders: Vec<ModLoaders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLoaders {
    pub id: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFileReference {
    #[serde(rename = "projectID")]
    pub project_id: i32,
    #[serde(rename = "fileID")]
    pub file_id: i32,
    pub required: bool,
}
