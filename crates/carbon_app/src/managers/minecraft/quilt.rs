use std::sync::Arc;

use daedalus::modded::{LoaderVersion, Manifest, PartialVersionInfo};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::trace;
use url::Url;

use crate::db::PrismaClient;

use super::META_VERSION;

#[derive(Error, Debug)]
pub enum QuiltManifestError {
    #[error("Could not fetch quilt manifest from launchermeta: {0}")]
    NetworkError(#[from] reqwest::Error),
}

pub async fn get_manifest(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    meta_base_url: &reqwest::Url,
) -> anyhow::Result<Manifest> {
    let server_url = meta_base_url.join(&format!("quilt/{}/manifest.json", META_VERSION))?;
    let new_manifest = reqwest_client
        .get(server_url)
        .send()
        .await?
        .json::<Manifest>()
        .await
        .map_err(QuiltManifestError::from)?;

    Ok(new_manifest)
}

pub async fn get_version(
    db_client: Arc<PrismaClient>,
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    quilt_version: &str,
    meta_base_url: &Url,
) -> anyhow::Result<PartialVersionInfo> {
    let db_entry_name = format!("quilt-{}", quilt_version);

    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let db_cache = db_client
        .partial_version_info_cache()
        .find_unique(crate::db::partial_version_info_cache::id::equals(
            db_entry_name.clone(),
        ))
        .exec()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to query db: {}", err))?;

    if let Some(db_cache) = db_cache {
        let db_cache = serde_json::from_slice(&db_cache.partial_version_info);

        if let Ok(db_cache) = db_cache {
            trace!("Quilt version {} found in cache", quilt_version);
            return Ok(db_cache);
        } else {
            tracing::warn!(
                "Failed to deserialize quilt version for {} from cache, re-fetching",
                quilt_version
            );
        }
    }

    let version_url = meta_base_url.join(&format!(
        "quilt/{}/versions/{}.json",
        META_VERSION, quilt_version
    ))?;
    let version_bytes = reqwest_client
        .get(version_url)
        .send()
        .await?
        .bytes()
        .await?;

    db_client
        .partial_version_info_cache()
        .upsert(
            crate::db::partial_version_info_cache::id::equals(db_entry_name.clone()),
            crate::db::partial_version_info_cache::create(
                db_entry_name.clone(),
                version_bytes.to_vec(),
                vec![
                    crate::db::partial_version_info_cache::partial_version_info::set(
                        version_bytes.to_vec(),
                    ),
                ],
            ),
            vec![],
        )
        .exec()
        .await?;

    Ok(serde_json::from_slice(&version_bytes)?)
}

pub fn replace_template(
    template_info: &PartialVersionInfo,
    game_version: &str,
    template: &str,
) -> PartialVersionInfo {
    let mut version_info = template_info.clone();
    version_info.id = version_info.id.replace(template, game_version);
    version_info.inherits_from = version_info.inherits_from.replace(template, game_version);
    for library in version_info.libraries.iter_mut() {
        library.name.version = library.name.version.replace(template, game_version);
    }

    version_info
}
