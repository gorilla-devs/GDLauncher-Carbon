use std::sync::Arc;

use carbon_domain::minecraft::manifest::{ManifestVersion, MinecraftManifest};
use prisma_client_rust::QueryError;
use rspc::ErrorCode;
use thiserror::Error;

use crate::db::{minecraft_manifest::SetParam, PrismaClient};

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Could not fetch manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Manifest database query error: {0}")]
    DBQueryError(#[from] QueryError),
}

impl From<ManifestError> for rspc::Error {
    fn from(value: ManifestError) -> Self {
        rspc::Error::new(
            ErrorCode::InternalServerError,
            format!("Manifest Error: {value}"),
        )
    }
}

// get should abstract the complexity of fetching it from either the network or the db
pub async fn get_meta(db: Arc<PrismaClient>) -> Result<Vec<ManifestVersion>, ManifestError> {
    let server_url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
    let new_manifest = reqwest::get(server_url)
        .await?
        .json::<MinecraftManifest>()
        .await?;
    // TODO ^^ should not throw but try to fetch from DB first

    for version in &new_manifest.versions {
        db.minecraft_manifest()
            .upsert(
                crate::db::minecraft_manifest::id::equals(version.id.clone()),
                crate::db::minecraft_manifest::create(
                    version.id.clone(),
                    version.type_.clone().into(),
                    version.url.clone(),
                    version.time.clone(),
                    version.release_time.clone(),
                    version.sha1.clone(),
                    vec![],
                ),
                vec![
                    crate::db::minecraft_manifest::id::set(version.id.clone()),
                    crate::db::minecraft_manifest::r#type::set(version.type_.clone().into()),
                    crate::db::minecraft_manifest::url::set(version.url.clone()),
                    crate::db::minecraft_manifest::time::set(version.time.clone()),
                    crate::db::minecraft_manifest::release_time::set(version.release_time.clone()),
                    crate::db::minecraft_manifest::sha_1::set(version.sha1.clone()),
                ],
            )
            .exec()
            .await?;
    }

    Ok(new_manifest.versions)
}
