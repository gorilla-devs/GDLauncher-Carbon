use anyhow::Context;
use chrono::Utc;
use daedalus::modded::{LoaderVersion, Manifest, PartialVersionInfo};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::trace;
use url::Url;

use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::version_meta;

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
    db: &Db,
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    quilt_version: &str,
    meta_base_url: &Url,
) -> anyhow::Result<PartialVersionInfo> {
    let db_entry_name = format!("quilt-{}", quilt_version);

    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let update_cache = || async {
        let version_url = meta_base_url.join(&format!(
            "quilt/{}/versions/{}.json",
            META_VERSION, quilt_version
        ))?;

        let resp = reqwest_client.get(version_url.clone()).send().await?;

        let status = resp.status();

        if !status.is_success() {
            anyhow::bail!(
                "Failed to fetch quilt version from `{}`: {}",
                version_url.clone(),
                status
            );
        }

        let version_bytes = resp.bytes().await.with_context(|| {
            format!(
                "Failed to fetch quilt version from `{}`: {}",
                version_url.clone(),
                status
            )
        })?;

        // Validate the freshly fetched body before caching it: a 200 response with an
        // unparseable body must not overwrite a previously-good cached version.
        let parsed =
            serde_json::from_slice::<PartialVersionInfo>(&version_bytes).with_context(|| {
                format!(
                    "Failed to parse quilt version from `{}`",
                    version_url.clone()
                )
            })?;

        let db_entry_name_owned = db_entry_name.clone();
        let version_bytes_owned = version_bytes.to_vec();
        db.write(move |conn| {
            Ok(version_meta::upsert_partial_version_info(
                conn,
                &db_entry_name_owned,
                &version_bytes_owned,
                DbDateTime(Utc::now().fixed_offset()),
            )?)
        })
        .await?;

        Ok(parsed)
    };

    match update_cache().await {
        Ok(parsed) => Ok(parsed),
        Err(err) => {
            let db_entry_name_owned = db_entry_name.clone();
            let db_cache = db
                .read(move |conn| {
                    Ok(version_meta::get_partial_version_info(
                        conn,
                        &db_entry_name_owned,
                    )?)
                })
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

            anyhow::bail!("Failed to fetch quilt version: {}", err);
        }
    }
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
