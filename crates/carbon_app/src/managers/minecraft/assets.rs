use std::path::PathBuf;

use daedalus::minecraft::{AssetIndex, AssetsIndex};
use prisma_client_rust::QueryError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Can't fetch assets index manifest: {0}")]
    FetchAssetsIndexManifest(#[from] reqwest::Error),
    #[error("Can't execute db query: {0}")]
    QueryError(#[from] QueryError),
}

pub async fn get_meta(
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_asset_index: AssetIndex,
    asset_indexes_path: PathBuf,
) -> anyhow::Result<AssetsIndex> {
    let asset_index_bytes = reqwest_client
        .get(version_asset_index.url)
        .send()
        .await?
        .bytes()
        .await?;

    tokio::fs::create_dir_all(&asset_indexes_path).await?;
    tokio::fs::write(
        asset_indexes_path.join(format!("{}.json", version_asset_index.id)),
        asset_index_bytes.clone(),
    )
    .await?;

    Ok(serde_json::from_slice(&asset_index_bytes)?)
}
