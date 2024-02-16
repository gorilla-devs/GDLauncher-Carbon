use tracing::error;

use crate::{
    db,
    domain::{
        instance::{
            info::{CurseforgeModpack, ModrinthModpack},
            InstanceModpackInfo,
        },
        modplatforms::{
            curseforge::filters::{ModFileParameters, ModParameters},
            modrinth::search::{ProjectID, VersionID},
        },
    },
    managers::{metadata::cache, App},
};

pub async fn get_modpack_icon(app: &App, modrinth: ModrinthModpack) -> anyhow::Result<Vec<u8>> {
    app.prisma_client
        .modrinth_modpack_image_cache()
        .find_unique(db::modrinth_modpack_image_cache::project_id_version_id(
            modrinth.project_id,
            modrinth.version_id,
        ))
        .exec()
        .await?
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))?
        .data
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))
}

pub async fn get_modpack_metadata(
    app: &App,
    modrinth: ModrinthModpack,
) -> anyhow::Result<InstanceModpackInfo> {
    let cache_entry = app
        .prisma_client
        .modrinth_modpack_cache()
        .find_unique(db::modrinth_modpack_cache::project_id_version_id(
            modrinth.project_id.clone(),
            modrinth.version_id.clone(),
        ))
        .with(db::modrinth_modpack_cache::logo_image::fetch())
        .exec()
        .await?;

    let logo = cache_entry
        .as_ref()
        .and_then(|cache_entry| cache_entry.logo_image.as_ref())
        .and_then(|logo_image| logo_image.as_ref().map(|logo_image| logo_image));

    let is_entry_up_to_date = cache_entry
        .as_ref()
        .map(|entry| {
            entry.updated_at.timestamp() + 60 * 60 * 24 * 7 > chrono::Utc::now().timestamp()
        })
        .unwrap_or(false);

    let has_cached_entry = cache_entry.is_some();
    let has_cached_logo = logo.is_some();

    if has_cached_entry && is_entry_up_to_date {
        let Some(cache_entry) = cache_entry else {
            unreachable!("We just checked that cache_entry.is_some()");
        };

        return Ok(InstanceModpackInfo {
            name: cache_entry.modpack_name,
            version_name: cache_entry.version_name,
            url_slug: cache_entry.url_slug,
            has_image: cache_entry
                .logo_image
                .flatten()
                .map(|logo| logo.data.is_some())
                .unwrap_or(false),
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

            app.prisma_client
                ._transaction()
                .run(|prisma_client| async move {
                    if has_cached_entry {
                        prisma_client
                            .modrinth_modpack_cache()
                            .update(
                                db::modrinth_modpack_cache::project_id_version_id(
                                    modrinth.project_id.clone(),
                                    modrinth.version_id.clone(),
                                ),
                                vec![
                                    db::modrinth_modpack_cache::modpack_name::set(name),
                                    db::modrinth_modpack_cache::version_name::set(file_name),
                                    db::modrinth_modpack_cache::url_slug::set(slug),
                                ],
                            )
                            .exec()
                            .await?;
                    } else {
                        prisma_client
                            .modrinth_modpack_cache()
                            .create(
                                modrinth.project_id.clone(),
                                modrinth.version_id.clone(),
                                name,
                                file_name,
                                slug,
                                vec![],
                            )
                            .exec()
                            .await?;
                    }

                    if has_cached_logo {
                        prisma_client
                            .modrinth_modpack_image_cache()
                            .update(
                                db::modrinth_modpack_image_cache::project_id_version_id(
                                    modrinth.project_id.clone(),
                                    modrinth.version_id.clone(),
                                ),
                                vec![
                                    db::modrinth_modpack_image_cache::url::set(
                                        url.unwrap_or_default(),
                                    ),
                                    db::modrinth_modpack_image_cache::data::set(
                                        icon_bytes.map(|icon_bytes| icon_bytes.to_vec()),
                                    ),
                                ],
                            )
                            .exec()
                            .await?;
                    } else {
                        prisma_client
                            .modrinth_modpack_image_cache()
                            .create(
                                url.unwrap_or_default(),
                                db::modrinth_modpack_cache::project_id_version_id(
                                    modrinth.project_id.clone(),
                                    modrinth.version_id.clone(),
                                ),
                                vec![db::modrinth_modpack_image_cache::data::set(
                                    icon_bytes.map(|icon_bytes| icon_bytes.to_vec()),
                                )],
                            )
                            .exec()
                            .await?;
                    }

                    Ok::<(), anyhow::Error>(())
                })
                .await?;

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
                        has_image: cache_entry
                            .logo_image
                            .flatten()
                            .map(|logo| logo.data.is_some())
                            .unwrap_or(false),
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
