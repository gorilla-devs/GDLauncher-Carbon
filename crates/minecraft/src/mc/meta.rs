use super::version::Version as VersionManifest;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::trace;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McMeta {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: Type,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
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

impl Version {
    #[tracing::instrument]
    pub async fn get_version_meta(&self) -> Result<VersionManifest> {
        trace!("Downloading version manifest for {}", self.id);
        let resp = reqwest::get(&self.url)
            .await?
            .json::<VersionManifest>()
            .await?;

        Ok(resp)
    }
}

impl Latest {
    #[tracing::instrument]
    pub fn version_for_release<'a>(&self, launcher_meta: &'a McMeta) -> &'a Version {
        launcher_meta
            .versions
            .iter()
            .find(|version| version.id == self.release)
            .unwrap()
    }

    #[tracing::instrument]
    pub fn version_for_snapshot<'a>(&self, launcher_meta: &'a McMeta) -> &'a Version {
        launcher_meta
            .versions
            .iter()
            .find(|version| version.id == self.snapshot)
            .unwrap()
    }
}

impl McMeta {
    #[tracing::instrument]
    pub async fn download_meta() -> Result<Self, reqwest::Error> {
        let server_url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
        trace!("Downloading launcher meta from {}", server_url);
        reqwest::get(server_url).await?.json::<McMeta>().await
    }
}
