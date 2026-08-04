use crate::{
    domain::instance::{
        InstanceModpackInfo,
        info::{CurseforgeModpack, ModrinthModpack},
    },
    managers::{App, metadata::cache},
};
use carbon_platforms::{
    curseforge::filters::{ModFileParameters, ModParameters},
    modrinth::search::{ProjectID, VersionID},
};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::modpack_cache as modpackdb;
use tracing::error;

pub async fn get_modpack_icon(app: &App, modrinth: ModrinthModpack) -> anyhow::Result<Vec<u8>> {
    modpackdb::get_mr_modpack_logo(&app.db, &modrinth.project_id, &modrinth.version_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))?
        .data
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))
}

pub async fn get_modpack_metadata(
    app: &App,
    modrinth: ModrinthModpack,
) -> anyhow::Result<InstanceModpackInfo> {
    let project_id_read = modrinth.project_id.clone();
    let version_id_read = modrinth.version_id.clone();
    let cache_entry =
        modpackdb::get_mr_modpack(&app.db, &project_id_read, &version_id_read).await?;

    let is_entry_up_to_date = cache_entry
        .as_ref()
        .map(|entry| {
            entry.updated_at.timestamp() + 60 * 60 * 24 * 7 > chrono::Utc::now().timestamp()
        })
        .unwrap_or(false);

    let has_cached_entry = cache_entry.is_some();
    let has_cached_logo = cache_entry.as_ref().map(|e| e.has_logo).unwrap_or(false);

    if has_cached_entry && is_entry_up_to_date {
        let Some(cache_entry) = cache_entry else {
            unreachable!("We just checked that cache_entry.is_some()");
        };

        return Ok(InstanceModpackInfo {
            name: cache_entry.modpack_name,
            version_name: cache_entry.version_name,
            url_slug: cache_entry.url_slug,
            has_image: cache_entry.logo_data.is_some(),
        });
    } else {
        let app = app.clone();
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

            modpackdb::upsert_mr_modpack(
                &app.db,
                &modrinth.project_id,
                &modrinth.version_id,
                &name,
                &file_name,
                &slug,
                DbDateTime(chrono::Utc::now().fixed_offset()),
            )
            .await?;

            if icon_bytes_is_some || has_cached_logo {
                let image_url = url.clone().unwrap_or_default();
                let image_data = icon_bytes.clone().map(|icon_bytes| icon_bytes.to_vec());
                modpackdb::upsert_mr_modpack_image(
                    &app.db,
                    &modrinth.project_id,
                    &modrinth.version_id,
                    &image_url,
                    image_data.as_deref(),
                )
                .await?;
            }

            Ok::<_, anyhow::Error>((modpack, version, icon_bytes_is_some))
        })
        .await?;

        let (addon, addon_file, has_icon) = match runner {
            Ok(a) => a,
            Err(e) => {
                error!("Failed to get modpack metadata: {:?}", e);

                if let Some(cache_entry) = cache_entry {
                    return Ok(InstanceModpackInfo {
                        name: cache_entry.modpack_name,
                        version_name: cache_entry.version_name,
                        url_slug: cache_entry.url_slug,
                        has_image: cache_entry.logo_data.is_some(),
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
