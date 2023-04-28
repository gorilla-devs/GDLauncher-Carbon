use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub minecraft: Minecraft,
    pub manifest_type: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub overrides: String,
    pub files: Vec<ManifestFileReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Minecraft {
    pub version: String,
    pub mod_loaders: Vec<ModLoaders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModLoaders {
    pub id: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFileReference {
    pub project_id: i32,
    pub file_id: i32,
    pub required: bool,
}
