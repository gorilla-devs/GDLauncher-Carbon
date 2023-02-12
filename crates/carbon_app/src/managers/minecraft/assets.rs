use std::path::Path;

use carbon_domain::minecraft::assets::AssetIndex;
use carbon_net::IntoVecDownloadable;

async fn download(
    asset_index: AssetIndex,
    base_path: &Path,
    progress: tokio::sync::watch::Sender<u32>,
) {
    let downloadable_assets = asset_index.into_vec_downloadable(base_path);
}
