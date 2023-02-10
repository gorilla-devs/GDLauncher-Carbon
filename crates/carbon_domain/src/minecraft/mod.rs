use rspc::Type;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct MinecraftManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: Type,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub enum Type {
    #[serde(rename = "old_alpha")]
    OldAlpha,
    #[serde(rename = "old_beta")]
    OldBeta,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "snapshot")]
    Snapshot,
}

impl From<Type> for String {
    fn from(type_: Type) -> Self {
        match type_ {
            Type::OldAlpha => "old_alpha".to_string(),
            Type::OldBeta => "old_beta".to_string(),
            Type::Release => "release".to_string(),
            Type::Snapshot => "snapshot".to_string(),
        }
    }
}
