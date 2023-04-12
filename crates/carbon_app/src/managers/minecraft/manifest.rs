use crate::domain::minecraft::manifest::{ManifestVersion, MinecraftManifest};
use prisma_client_rust::QueryError;
use thiserror::Error;

use super::MC_MANIFEST_META_URL;

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Could not fetch manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] QueryError),
}

// get should abstract the complexity of fetching it from either the network or the db
pub async fn get_meta(
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
) -> anyhow::Result<Vec<ManifestVersion>> {
    let server_url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
    let new_manifest = reqwest_client
        .get(MC_MANIFEST_META_URL)
        .send()
        .await?
        .json::<MinecraftManifest>()
        .await?;

    Ok(new_manifest.versions)
}
