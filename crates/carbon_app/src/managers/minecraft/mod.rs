use crate::db::{minecraft_manifest::SetParam, PrismaClient};
use carbon_domain::minecraft::MinecraftManifest;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MinecraftManagerError {
    #[error("Cannot fetch manifest from HTTP: {0}")]
    ManifestFetchError(reqwest::Error),
    #[error("Manifest does not meet expected JSON structure: {0}")]
    ManifestParseError(reqwest::Error),
}

pub(crate) struct MinecraftManager {}

impl MinecraftManager {
    pub fn new() -> Self {
        Self {}
    }
}

pub async fn init_manifest_v2(db_client: PrismaClient) -> Result<(), MinecraftManagerError> {
    let manifestv2 =
        reqwest::get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
            .await
            .map_err(MinecraftManagerError::ManifestFetchError)?
            .json::<MinecraftManifest>()
            .await
            .map_err(MinecraftManagerError::ManifestParseError)?;

    for version in manifestv2.versions {
        db_client
            .minecraft_manifest()
            .create(
                version.id,
                version.type_.into(),
                version.url,
                version.time,
                version.release_time,
                vec![SetParam::SetSha1(version.sha1)],
            )
            .exec()
            .await
            .unwrap();
    }

    Ok(())
}
