use crate::{
    domain::instance::{InstanceModpackInfo, info::CurseforgeModpack},
    managers::{App, metadata::cache},
};
use carbon_platforms::curseforge::filters::{ModFileParameters, ModParameters};
use carbon_repos::queries;
use tracing::error;

pub async fn get_modpack_icon(app: &App, curseforge: CurseforgeModpack) -> anyhow::Result<Vec<u8>> {
    let pool = app.db_pool.clone();
    let project_id = curseforge.project_id as i32;
    let file_id = curseforge.file_id as i32;

    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let result = queries::modpack::FindCurseForgeModpackImageCache::fetch_optional(
            &conn, project_id, file_id,
        )?;

        match result.and_then(|r| r.data) {
            Some(data) => Ok(data),
            None => Err(anyhow::anyhow!("No icon found for modpack")),
        }
    })
    .await?
}

pub async fn get_modpack_metadata(
    app: &App,
    curseforge: CurseforgeModpack,
) -> anyhow::Result<InstanceModpackInfo> {
    let pool = app.db_pool.clone();
    let project_id = curseforge.project_id as i32;
    let file_id = curseforge.file_id as i32;

    // Query cache entry with image availability
    let cache_entry = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, anyhow::Error>(
            queries::modpack::FindCurseForgeModpackCacheWithImage::fetch_optional(
                &conn, project_id, file_id,
            )?,
        )
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

    let has_cache_entry = cache_entry.is_some();
    let has_cache_logo = cache_entry.as_ref().map(|e| e.has_image).unwrap_or(false);

    if has_cache_entry && is_entry_up_to_date {
        let Some(cache_entry) = cache_entry else {
            unreachable!("We just checked that cache_entry.is_some()");
        };

        return Ok(InstanceModpackInfo {
            name: cache_entry.modpack_name,
            version_name: cache_entry.version_name,
            url_slug: cache_entry.url_slug,
            has_image: cache_entry.has_image,
        });
    } else {
        let app = app.clone();
        let cache_entry_for_fallback = cache_entry;
        let runner = tokio::spawn(async move {
            let modplatform_manager = app.modplatforms_manager();
            let addon_file = modplatform_manager
                .curseforge
                .get_mod_file(ModFileParameters {
                    mod_id: curseforge.project_id as i32,
                    file_id: curseforge.file_id as i32,
                });
            let addon = modplatform_manager.curseforge.get_mod(ModParameters {
                mod_id: curseforge.project_id as i32,
            });

            let (addon_file, addon) = tokio::try_join!(addon_file, addon)?;

            let name = addon.data.name.clone();
            let file_name = addon_file.data.file_name.clone();
            let slug = addon.data.slug.clone();
            let url = addon.data.logo.as_ref().map(|logo| logo.url.clone());

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
            let project_id = curseforge.project_id as i32;
            let file_id = curseforge.file_id as i32;
            let name_clone = name.clone();
            let file_name_clone = file_name.clone();
            let slug_clone = slug.clone();

            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;
                queries::modpack::UpsertCurseForgeModpackCache::execute(
                    &conn,
                    project_id,
                    file_id,
                    &name_clone,
                    &file_name_clone,
                    &slug_clone,
                )?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;

            // Upsert image cache if needed
            if icon_bytes_is_some || has_cache_logo {
                let pool = app.db_pool.clone();
                let project_id = curseforge.project_id as i32;
                let file_id = curseforge.file_id as i32;
                let url_clone = url.clone().unwrap_or_default();
                let icon_data = icon_bytes.clone();

                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    queries::modpack::UpsertCurseForgeModpackImageCache::execute(
                        &conn,
                        project_id,
                        file_id,
                        &url_clone,
                        icon_data.as_deref(),
                    )?;
                    Ok::<_, anyhow::Error>(())
                })
                .await??;
            }

            Ok::<_, anyhow::Error>((addon, addon_file, icon_bytes_is_some))
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
                        has_image: cache_entry.has_image,
                    });
                }

                return Err(e);
            }
        };

        return Ok(InstanceModpackInfo {
            name: addon.data.name,
            version_name: addon_file.data.display_name,
            url_slug: addon.data.slug,
            has_image: has_icon,
        });
    }
}
