use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

impl MinecraftManifest {
    pub async fn fetch() -> Result<Self, reqwest::Error> {
        reqwest::get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
            .await?
            .json::<MinecraftManifest>()
            .await
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl ManifestVersion {
    pub async fn fetch(&self) -> Result<super::version::Version, reqwest::Error> {
        reqwest::get(&self.url)
            .await?
            .json::<super::version::Version>()
            .await
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
