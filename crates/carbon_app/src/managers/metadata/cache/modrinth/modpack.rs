use crate::{
    domain::instance::{InstanceModpackInfo, info::ModrinthModpack},
    managers::{App, metadata::cache},
};
use carbon_platforms::modrinth::search::{ProjectID, VersionID};
use carbon_repos::queries;
use tracing::error;

pub async fn get_modpack_icon(app: &App, modrinth: ModrinthModpack) -> anyhow::Result<Vec<u8>> {
    let pool = app.db_pool.clone();
    let project_id = modrinth.project_id.clone();
    let version_id = modrinth.version_id.clone();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let result = conn.query_row(
            queries::modpack::FindModrinthModpackImageCache::SQL,
            rusqlite::params![project_id, version_id],
            |row| {
                let data: Option<Vec<u8>> = row.get("data")?;
                Ok(data)
            },
        );

        match result {
            Ok(Some(data)) => Ok(data),
            Ok(None) => Err(anyhow::anyhow!("No icon found for modpack")),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(anyhow::anyhow!("No icon found for modpack"))
            }
            Err(e) => Err(anyhow::Error::from(e)),
        }
    })
    .await?
}

/// Cache entry with optional image data
struct ModpackCacheEntry {
    modpack_name: String,
    version_name: String,
    url_slug: String,
    updated_at: String,
    has_image_data: bool,
}

pub async fn get_modpack_metadata(
    app: &App,
    modrinth: ModrinthModpack,
) -> anyhow::Result<InstanceModpackInfo> {
    let pool = app.db_pool.clone();
    let project_id = modrinth.project_id.clone();
    let version_id = modrinth.version_id.clone();

    // Query cache entry with image
    let cache_entry = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let result = conn.query_row(
            queries::modpack::FindModrinthModpackCacheWithImage::SQL,
            rusqlite::params![project_id, version_id],
            |row| {
                let modpack_name: String = row.get("modpackName")?;
                let version_name: String = row.get("versionName")?;
                let url_slug: String = row.get("urlSlug")?;
                let updated_at = cache::read_datetime_column(row, "updatedAt")?;
                let img_data: Option<Vec<u8>> = row.get("img_data")?;
                Ok(ModpackCacheEntry {
                    modpack_name,
                    version_name,
                    url_slug,
                    updated_at,
                    has_image_data: img_data.is_some(),
                })
            },
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    })
    .await??;

    let is_entry_up_to_date = cache_entry
        .as_ref()
        .map(|entry| {
            // Parse updated_at timestamp and check if it's within 7 days
            chrono::DateTime::parse_from_rfc3339(&entry.updated_at)
                .map(|dt| dt.timestamp() + 60 * 60 * 24 * 7 > chrono::Utc::now().timestamp())
                .unwrap_or(false)
        })
        .unwrap_or(false);

    let has_cached_entry = cache_entry.is_some();
    let has_cached_logo = cache_entry
        .as_ref()
        .map(|e| e.has_image_data)
        .unwrap_or(false);

    if has_cached_entry && is_entry_up_to_date {
        let Some(cache_entry) = cache_entry else {
            unreachable!("We just checked that cache_entry.is_some()");
        };

        return Ok(InstanceModpackInfo {
            name: cache_entry.modpack_name,
            version_name: cache_entry.version_name,
            url_slug: cache_entry.url_slug,
            has_image: cache_entry.has_image_data,
        });
    } else {
        let app = app.clone();
        let cache_entry_for_fallback = cache_entry;
        let runner = tokio::spawn(async move {
            let modplatform_manager = app.modplatforms_manager();
            let modpack = modplatform_manager
                .modrinth
                .get_project(ProjectID(modrinth.project_id.clone()));
            let version = modplatform_manager
                .modrinth
                .get_version(VersionID(modrinth.version_id.clone()));

            let (version, modpack) = tokio::try_join!(version, modpack)?;

            let name = modpack.title.clone();
            let file_name = version.version_number.clone();
            let slug = modpack.slug.clone();
            let url = modpack.icon_url.clone();

            let mut icon_bytes = None;

            if let Some(url) = url.as_ref() {
                let original_image = app
                    .reqwest_client
                    .get(url)
                    .header("avoid-caching", "")
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;

                let mcm = app.meta_cache_manager();
                let permit = mcm
                    .image_scale_semaphore
                    .acquire()
                    .await
                    .expect("the image scale semaphore is never closed");

                let scaled_image = carbon_scheduler::cpu_block(|| {
                    let scaled = cache::scale_mod_image(&original_image[..])?;
                    Ok::<_, anyhow::Error>(scaled)
                })
                .await?;

                drop(permit);

                icon_bytes = Some(scaled_image);
            }

            let icon_bytes_is_some = icon_bytes.is_some();

            // Upsert modpack cache
            let pool = app.db_pool.clone();
            let project_id = modrinth.project_id.clone();
            let version_id = modrinth.version_id.clone();
            let name_clone = name.clone();
            let file_name_clone = file_name.clone();
            let slug_clone = slug.clone();

            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;
                conn.execute(
                    queries::modpack::UpsertModrinthModpackCache::SQL,
                    rusqlite::params![
                        project_id,
                        version_id,
                        name_clone,
                        file_name_clone,
                        slug_clone
                    ],
                )?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;

            // Upsert image cache if needed
            if icon_bytes_is_some || has_cached_logo {
                let pool = app.db_pool.clone();
                let project_id = modrinth.project_id.clone();
                let version_id = modrinth.version_id.clone();
                let url_clone = url.clone().unwrap_or_default();
                let icon_data = icon_bytes.clone();

                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    conn.execute(
                        queries::modpack::UpsertModrinthModpackImageCache::SQL,
                        rusqlite::params![project_id, version_id, url_clone, icon_data],
                    )?;
                    Ok::<_, anyhow::Error>(())
                })
                .await??;
            }

            Ok::<_, anyhow::Error>((modpack, version, icon_bytes_is_some))
        })
        .await?;

        let (addon, addon_file, has_icon) = match runner {
            Ok(a) => a,
            Err(e) => {
                error!("Failed to get modpack metadata: {:?}", e);

                if let Some(cache_entry) = cache_entry_for_fallback {
                    return Ok(InstanceModpackInfo {
                        name: cache_entry.modpack_name,
                        version_name: cache_entry.version_name,
                        url_slug: cache_entry.url_slug,
                        has_image: cache_entry.has_image_data,
                    });
                }

                return Err(e);
            }
        };

        return Ok(InstanceModpackInfo {
            name: addon.title,
            version_name: addon_file.version_number,
            url_slug: addon.slug,
            has_image: has_icon,
        });
    }
}
