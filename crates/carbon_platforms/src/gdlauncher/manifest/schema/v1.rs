use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub manifest_version: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modpack_reference: Option<String>,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<String>,
    pub files: Vec<ModFile>,
    pub dependencies: PackDependencies,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDependencies {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "source", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ModFile {
    Curseforge(CurseforgeFile),
    Modrinth(ModrinthFile),
    Direct(DirectFile),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurseforgeFile {
    pub project_id: u32,
    pub file_id: u32,
    pub required: bool,
    pub blake3_hash: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub project_id: String,
    pub version_id: String,
    pub required: bool,
    pub blake3_hash: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectFile {
    pub url: String,
    pub filename: String,
    pub required: bool,
    pub blake3_hash: String,
    pub size: u64,
}
