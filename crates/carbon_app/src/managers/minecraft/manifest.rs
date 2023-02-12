use std::sync::Arc;

use carbon_domain::minecraft::manifest::MinecraftManifest;
use prisma_client_rust::QueryError;
use thiserror::Error;

use crate::db::{minecraft_manifest::SetParam, PrismaClient};

#[derive(Error, Debug)]
enum ManifestError {
    #[error("Could not fetch manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] QueryError),
}

// get should abstract the complexity of fetching it from either the network or the db
async fn get(db: Arc<PrismaClient>) -> Result<MinecraftManifest, ManifestError> {
    let server_url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
    let new_manifest = reqwest::get(server_url)
        .await?
        .json::<MinecraftManifest>()
        .await?;
    // TODO ^^ should not throw but try to fetch from DB first

    for version in &new_manifest.versions {
        db.minecraft_manifest()
            .create(
                version.id.clone(),
                version.type_.clone().into(),
                version.url.clone(),
                version.time.clone(),
                version.release_time.clone(),
                vec![SetParam::SetSha1(version.sha1.clone())],
            )
            .exec()
            .await?;
    }

    Ok(new_manifest)
}
