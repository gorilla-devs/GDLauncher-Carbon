use crate::db::PrismaClient;
use crate::managers::ManagersInner;
use carbon_domain::minecraft::MinecraftManifest;
use std::sync::{Arc, Weak};
use thiserror::Error;

use super::Managers;

#[derive(Error, Debug)]
pub enum MinecraftManagerError {
    #[error("Cannot fetch manifest from HTTP: {0}")]
    ManifestFetchError(reqwest::Error),
    #[error("Manifest does not meet expected JSON structure: {0}")]
    ManifestParseError(reqwest::Error),
}

pub(crate) struct MinecraftManager {
    managers: Weak<ManagersInner>,
}

impl MinecraftManager {
    pub async fn make_for_app(app: &Managers) -> Self {
        let managers = Arc::downgrade(app);

        Self { managers }
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
                version.type_.into(),
                version.url,
                version.time,
                version.release_time,
                version.sha1,
                vec![],
            )
            .exec()
            .await
            .unwrap();
    }

    Ok(())
}
