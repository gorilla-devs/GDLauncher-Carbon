use tracing::error;

use crate::{
    db,
    domain::{
        instance::{info::CurseforgeModpack, InstanceModpackInfo},
        modplatforms::curseforge::filters::{ModFileParameters, ModParameters},
    },
    managers::{metadata::cache, App},
};

pub async fn get_modpack_icon(app: &App, curseforge: CurseforgeModpack) -> anyhow::Result<Vec<u8>> {
    app.prisma_client
        .curse_forge_modpack_image_cache()
        .find_unique(db::curse_forge_modpack_image_cache::project_id_file_id(
            curseforge.project_id as i32,
            curseforge.file_id as i32,
        ))
        .exec()
        .await?
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))?
        .data
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))
}

pub async fn get_modpack_metadata(
    app: &App,
    curseforge: CurseforgeModpack,
) -> anyhow::Result<InstanceModpackInfo> {
    let cache_entry = app
        .prisma_client
        .curse_forge_modpack_cache()
        .find_unique(db::curse_forge_modpack_cache::project_id_file_id(
            curseforge.project_id as i32,
            curseforge.file_id as i32,
        ))
        .with(db::curse_forge_modpack_cache::logo_image::fetch())
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

    let has_cache_entry = cache_entry.is_some();
    let has_cache_logo = logo.is_some();

    if has_cache_entry && is_entry_up_to_date {
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
                let permit = mcm.image_scale_semaphore.acquire().await
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
                    if has_cache_entry {
                        prisma_client
                            .curse_forge_modpack_cache()
                            .update(
                                db::curse_forge_modpack_cache::project_id_file_id(
                                    curseforge.project_id as i32,
                                    curseforge.file_id as i32,
                                ),
                                vec![
                                    db::curse_forge_modpack_cache::modpack_name::set(name),
                                    db::curse_forge_modpack_cache::version_name::set(file_name),
                                    db::curse_forge_modpack_cache::url_slug::set(slug),
                                ],
                            )
                            .exec()
                            .await?;
                    } else {
                        prisma_client
                            .curse_forge_modpack_cache()
                            .create(
                                curseforge.project_id as i32,
                                curseforge.file_id as i32,
                                name,
                                file_name,
                                slug,
                                vec![],
                            )
                            .exec()
                            .await?;
                    }

                    if has_cache_logo {
                        prisma_client
                            .curse_forge_modpack_image_cache()
                            .update(
                                db::curse_forge_modpack_image_cache::project_id_file_id(
                                    curseforge.project_id as i32,
                                    curseforge.file_id as i32,
                                ),
                                vec![
                                    db::curse_forge_modpack_image_cache::url::set(
                                        url.unwrap_or_default(),
                                    ),
                                    db::curse_forge_modpack_image_cache::data::set(
                                        icon_bytes.map(|icon_bytes| icon_bytes.to_vec()),
                                    ),
                                ],
                            )
                            .exec()
                            .await?;
                    } else {
                        prisma_client
                            .curse_forge_modpack_image_cache()
                            .create(
                                url.unwrap_or_default(),
                                db::curse_forge_modpack_cache::project_id_file_id(
                                    curseforge.project_id as i32,
                                    curseforge.file_id as i32,
                                ),
                                vec![db::curse_forge_modpack_image_cache::data::set(
                                    icon_bytes.map(|icon_bytes| icon_bytes.to_vec()),
                                )],
                            )
                            .exec()
                            .await?;
                    }

                    Ok::<(), anyhow::Error>(())
                })
                .await?;

            Ok::<_, anyhow::Error>((addon, addon_file, icon_bytes_is_some))
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
            name: addon.data.name,
            version_name: addon_file.data.display_name,
            url_slug: addon.data.slug,
            has_image: has_icon,
        });
    }
}
