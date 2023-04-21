use prisma_client_rust::QueryError;
use thiserror::Error;

use crate::domain::minecraft::modded::ModdedManifest;

#[derive(Error, Debug)]
pub enum ForgeManifestError {
    #[error("Could not fetch forge manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] QueryError),
}

pub async fn get_manifest(
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    meta_base_url: &reqwest::Url,
) -> anyhow::Result<ModdedManifest> {
    let server_url = meta_base_url.join("forge/v0/manifest.json")?;
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<ModdedManifest>()
        .await?;

    Ok(new_manifest)
}
