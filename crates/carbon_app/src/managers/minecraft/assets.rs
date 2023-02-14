use std::{path::Path, sync::Arc};

use carbon_domain::minecraft::{assets::AssetIndex, version::VersionAssetIndex};
use carbon_net::IntoVecDownloadable;
use prisma_client_rust::QueryError;
use thiserror::Error;

use crate::db::PrismaClient;

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Can't fetch assets index manifest: {0}")]
    FetchAssetsIndexManifest(#[from] reqwest::Error),
    #[error("Can't execute db query: {0}")]
    QueryError(#[from] QueryError),
}

pub async fn get_meta(
    db: Arc<PrismaClient>,
    version_asset_index: VersionAssetIndex,
) -> Result<AssetIndex, AssetsError> {
    let asset_index = reqwest::get(version_asset_index.url)
        .await?
        .json::<AssetIndex>()
        .await?;

    let bytes = serde_json::to_vec(&asset_index).unwrap();

    db.minecraft_assets()
        .upsert(
            crate::db::minecraft_assets::id_sha_1::equals(version_asset_index.sha1.clone()),
            crate::db::minecraft_assets::create(
                version_asset_index.sha1.clone(),
                bytes.clone(),
                vec![],
            ),
            vec![crate::db::minecraft_assets::json::set(bytes)],
        )
        .exec()
        .await?;

    Ok(asset_index)
}

async fn download(
    asset_index: AssetIndex,
    base_path: &Path,
    progress: tokio::sync::watch::Sender<u32>,
) {
    let downloadable_assets = asset_index.into_vec_downloadable(base_path);
}
