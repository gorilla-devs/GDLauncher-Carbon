// Modpack metadata caching functions
// These are used by instance manager for modpack metadata

use crate::{
    domain::instance::{
        InstanceModpackInfo,
        info::{CurseforgeModpack, ModrinthModpack},
    },
    managers::App,
};
use carbon_platforms::curseforge::filters::{ModFileParameters, ModParameters};
use carbon_platforms::modrinth::search::{ProjectID, VersionID};
use carbon_repos::db;
use tracing::error;

// CurseForge modpack functions

pub async fn get_curseforge_modpack_icon(
    app: &App,
    curseforge: CurseforgeModpack,
) -> anyhow::Result<Vec<u8>> {
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

pub async fn get_curseforge_modpack_metadata(
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

                let result = super::utils::scale_mod_image(&original_image[..]);

                match result {
                    Ok(scaled_icon) => {
                        icon_bytes = Some(scaled_icon);
                    }
                    Err(e) => {
                        error!("Failed to scale icon: {}", e);
                    }
                }
            }

            let delete_cache_entry_query = app.prisma_client.curse_forge_modpack_cache().delete(
                db::curse_forge_modpack_cache::project_id_file_id(
                    curseforge.project_id as i32,
                    curseforge.file_id as i32,
                ),
            );

            let create_cache_entry_query = app.prisma_client.curse_forge_modpack_cache().create(
                curseforge.project_id as i32,
                curseforge.file_id as i32,
                name.clone(),
                file_name.clone(),
                slug.clone(),
                vec![],
            );

            let delete_icon_entry_query = app
                .prisma_client
                .curse_forge_modpack_image_cache()
                .delete(db::curse_forge_modpack_image_cache::project_id_file_id(
                    curseforge.project_id as i32,
                    curseforge.file_id as i32,
                ));

            let create_icon_entry_query =
                app.prisma_client.curse_forge_modpack_image_cache().create(
                    url.unwrap_or_default(),
                    db::curse_forge_modpack_cache::project_id_file_id(
                        curseforge.project_id as i32,
                        curseforge.file_id as i32,
                    ),
                    vec![db::curse_forge_modpack_image_cache::data::set(
                        icon_bytes.clone(),
                    )],
                );

            // Execute queries separately due to different table types
            let _ = delete_cache_entry_query.exec().await;
            let _ = delete_icon_entry_query.exec().await;
            create_cache_entry_query.exec().await?;
            create_icon_entry_query.exec().await?;

            anyhow::Ok(InstanceModpackInfo {
                name: name.clone(),
                version_name: file_name.clone(),
                url_slug: slug.clone(),
                has_image: has_cache_logo || icon_bytes.is_some(),
            })
        });

        runner.await?
    }
}

// Modrinth modpack functions

pub async fn get_modrinth_modpack_icon(
    app: &App,
    modrinth: ModrinthModpack,
) -> anyhow::Result<Vec<u8>> {
    app.prisma_client
        .modrinth_modpack_image_cache()
        .find_unique(db::modrinth_modpack_image_cache::project_id_version_id(
            modrinth.project_id.clone(),
            modrinth.version_id.clone(),
        ))
        .exec()
        .await?
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))?
        .data
        .ok_or_else(|| anyhow::anyhow!("No icon found for modpack"))
}

pub async fn get_modrinth_modpack_metadata(
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
            let version = modplatform_manager
                .modrinth
                .get_version(VersionID(modrinth.version_id.clone()));
            let project = modplatform_manager
                .modrinth
                .get_project(ProjectID(modrinth.project_id.clone()));

            let (version, project) = tokio::try_join!(version, project)?;

            let name = project.title.clone();
            let version_name = version.name.clone();
            let slug = project.slug.clone();
            let url = project.icon_url.clone();

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

                let result = super::utils::scale_mod_image(&original_image[..]);

                match result {
                    Ok(scaled_icon) => {
                        icon_bytes = Some(scaled_icon);
                    }
                    Err(e) => {
                        error!("Failed to scale icon: {}", e);
                    }
                }
            }

            let delete_cache_entry_query = app.prisma_client.modrinth_modpack_cache().delete(
                db::modrinth_modpack_cache::project_id_version_id(
                    modrinth.project_id.clone(),
                    modrinth.version_id.clone(),
                ),
            );

            let create_cache_entry_query = app.prisma_client.modrinth_modpack_cache().create(
                modrinth.project_id.clone(),
                modrinth.version_id.clone(),
                name.clone(),
                version_name.clone(),
                slug.clone(),
                vec![],
            );

            let delete_icon_entry_query = app.prisma_client.modrinth_modpack_image_cache().delete(
                db::modrinth_modpack_image_cache::project_id_version_id(
                    modrinth.project_id.clone(),
                    modrinth.version_id.clone(),
                ),
            );

            let create_icon_entry_query = app.prisma_client.modrinth_modpack_image_cache().create(
                url.unwrap_or_default(),
                db::modrinth_modpack_cache::project_id_version_id(
                    modrinth.project_id.clone(),
                    modrinth.version_id.clone(),
                ),
                vec![db::modrinth_modpack_image_cache::data::set(
                    icon_bytes.clone(),
                )],
            );

            // Execute queries separately due to different table types
            let _ = delete_cache_entry_query.exec().await;
            let _ = delete_icon_entry_query.exec().await;
            create_cache_entry_query.exec().await?;
            create_icon_entry_query.exec().await?;

            anyhow::Ok(InstanceModpackInfo {
                name: name.clone(),
                version_name: version_name.clone(),
                url_slug: slug.clone(),
                has_image: has_cache_logo || icon_bytes.is_some(),
            })
        });

        runner.await?
    }
}
