use super::{BundleSender, ModplatformCacher, UpdateNotifier};
use crate::domain::instance::InstanceId;
use crate::domain::instance::info::ModLoaderType;
use crate::managers::App;
use anyhow::anyhow;
use carbon_platforms::ModChannel;
use carbon_platforms::modrinth::search::VersionIDs;
use carbon_platforms::modrinth::version::Version;
use carbon_platforms::modrinth::{
    project::Project,
    responses::{ProjectsResponse, TeamResponse, VersionHashesResponse},
    search::{ProjectIDs, TeamIDs, VersionHashesQuery},
    version::HashAlgorithm,
};
use carbon_repos::queries;
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, error, trace, warn};

pub mod modpack;

pub struct ModrinthModCacher;

#[async_trait::async_trait]
impl ModplatformCacher for ModrinthModCacher {
    const NAME: &'static str = "modrinth";
    type SaveBundle = (
        Vec<String>,
        Vec<(String, String)>,
        VersionHashesResponse,
        ProjectsResponse,
        Vec<TeamResponse>,
        Vec<Version>,
    );

    async fn query_platform(
        app: &App,
        instance_id: InstanceId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()> {
        // Query mod files needing Modrinth update
        let pool = app.db_pool.clone();
        let id = *instance_id;
        let modlist_result = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt =
                conn.prepare(queries::metadata::ListModFilesNeedingModrinthUpdate::SQL)?;
            let results = stmt
                .query_map(rusqlite::params![id], |row| {
                    let metadata_id: String = row.get(0)?;
                    let sha512: Vec<u8> = row.get(1)?;
                    let sha512_hex = hex::encode(&sha512);
                    Ok((metadata_id, sha512_hex))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_mr_hashes.read().await;

        let mut modlist = modlist_result
            .into_iter()
            .filter(|(_, sha512)| !ignored_hashes.contains(sha512))
            .map(|(metadata_id, sha512)| (sha512.clone(), (metadata_id, sha512)))
            .collect::<VecDeque<_>>();

        drop(ignored_hashes);

        if modlist.is_empty() {
            return Ok(());
        }

        let total_mod_count = modlist.len();
        debug!(
            "Found {} mods to process for Modrinth caching",
            total_mod_count
        );

        let failed_instances = mcm.failed_mr_instances.read().await;
        let delay = failed_instances.get(&instance_id);

        if let Some((end_time, _)) = delay {
            if Instant::now() < *end_time {
                warn!(
                    "Not attempting to cache modrinth mods for {instance_id} as too many attempts have failed recently"
                );
                return Ok(());
            }
        }

        drop(failed_instances);

        let fut = async {
            while !modlist.is_empty() {
                let (sha512_hashes, metadata) = modlist
                    .drain(0..usize::min(1000, modlist.len()))
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                trace!("querying modrinth mod batch for instance {instance_id}");

                let versions_response = app
                    .modplatforms_manager()
                    .modrinth
                    .get_versions_from_hash(&VersionHashesQuery {
                        hashes: sha512_hashes.clone(),
                        algorithm: HashAlgorithm::SHA512,
                    })
                    .await?;

                let projects_response = app
                    .modplatforms_manager()
                    .modrinth
                    .get_projects(ProjectIDs {
                        ids: versions_response
                            .iter()
                            .map(|(_, ver)| ver.project_id.clone())
                            .collect(),
                    })
                    .await?;

                let teams_response = app
                    .modplatforms_manager()
                    .modrinth
                    .get_teams(TeamIDs {
                        ids: projects_response
                            .iter()
                            .map(|proj| proj.team.clone())
                            .collect(),
                    })
                    .await?;

                let mpm = app.modplatforms_manager();

                let combined_versions_list = projects_response
                    .iter()
                    .map(|project| &project.versions)
                    .flatten()
                    .map(|v| v.clone())
                    .collect::<Vec<_>>();

                let mpm = app.modplatforms_manager();
                let combined_version_futures = combined_versions_list
                    .chunks(350) // ~13 chars per version, 500 worked fine at time of testing
                    .map(|chunk| async {
                        let resp = mpm
                            .modrinth
                            .get_versions(VersionIDs {
                                ids: chunk.to_vec(),
                            })
                            .await;

                        resp
                    });

                let combined_versions_response =
                    futures::future::join_all(combined_version_futures)
                        .await
                        .into_iter()
                        .fold(Ok::<_, anyhow::Error>(Vec::new()), |a, c| match (a, c) {
                            (Ok(mut a), Ok(c)) => {
                                a.extend(c.0);
                                Ok(a)
                            }
                            (Err(e), _) => Err(anyhow!(e)),
                            (_, Err(e)) => Err(anyhow!(e)),
                        })?;

                sender.send((
                    sha512_hashes,
                    metadata,
                    versions_response,
                    projects_response,
                    teams_response,
                    combined_versions_response,
                ));
            }

            Ok::<_, anyhow::Error>(())
        };

        if let Err(e) = fut.await {
            error!({ error = ?e }, "Error occured while caching modrinth mods for instance {instance_id}");

            let mut failed_instances = mcm.failed_mr_instances.write().await;
            let entry = failed_instances
                .entry(instance_id)
                .or_insert((Instant::now(), 0));
            entry.0 = Instant::now() + Duration::from_secs(u64::pow(2, entry.1));
            entry.1 += 1;
        } else {
            let mut failed_instances = mcm.failed_mr_instances.write().await;
            failed_instances.remove(&instance_id);
        }

        Ok::<_, anyhow::Error>(())
    }

    async fn save_batch(
        app: &App,
        instance_id: InstanceId,
        (sha512_hashes, batch, versions, projects, teams, combined_versions): Self::SaveBundle,
    ) {
        trace!("processing modrinth mod batch for instance {instance_id}");

        let mut matches = sha512_hashes
            .iter()
            .map(|hash| versions.get_key_value(hash))
            .filter_map(|version_match| match version_match {
                Some((hash, version)) => projects
                    .iter()
                    .zip(teams.iter())
                    .find(|(proj, _team)| proj.id == version.project_id)
                    .map(|(proj, team)| (hash, (proj, team, version))),
                None => None,
            })
            .collect::<HashMap<_, _>>();
        let mcm = app.meta_cache_manager();
        let mut ignored_hashes = mcm.ignored_remote_mr_hashes.write().await;
        ignored_hashes.extend(
            sha512_hashes
                .iter()
                .filter(|hash| !matches.contains_key(hash))
                .cloned(),
        );
        drop(ignored_hashes);

        let combined_versions = &combined_versions;
        let futures = batch.into_iter().filter_map(|(metadata_id, sha512)| {
            let sha512_match = matches.remove(&sha512);
            sha512_match.map(|(project, team, version)| async move {
                let file = version
                    .files
                    .iter()
                    .find(|file| file.hashes.sha512 == sha512)
                    .expect("file to be present in it's response");

                let authors = team
                    .iter()
                    .map(|member| {
                        member
                            .user
                            .name
                            .clone()
                            .unwrap_or_else(|| member.user.username.clone())
                    })
                    .join(", ");

                let r = cache_modrinth_meta_unchecked(
                    app,
                    metadata_id,
                    &version,
                    file.hashes.sha512.clone(),
                    file.filename.clone(),
                    file.url.clone(),
                    project.clone(),
                    authors,
                    &combined_versions[..],
                )
                .await;

                if let Err(e) = r {
                    error!({ error = ?e }, "Could not store modrinth mod metadata");
                }
            })
        });

        futures::future::join_all(futures).await;
    }

    async fn cache_icons(app: &App, instance_id: InstanceId, update_notifier: &UpdateNotifier) {
        // Query mod files with outdated Modrinth icons
        let pool = app.db_pool.clone();
        let id = *instance_id;
        let modlist = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt =
                conn.prepare(queries::metadata::ListModFilesWithOutdatedModrinthIcons::SQL)?;
            let results = stmt
                .query_map(rusqlite::params![id], |row| {
                    let filename: String = row.get(0)?;
                    let project_id: String = row.get(1)?;
                    let version_id: String = row.get(2)?;
                    let metadata_id: String = row.get(3)?;
                    let url: String = row.get(4)?;
                    Ok((filename, project_id, version_id, metadata_id, url))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, anyhow::Error>(results)
        })
        .await;

        let modlist = match modlist {
            Ok(Ok(modlist)) => modlist,
            Ok(Err(e)) => {
                error!({ error = ?e }, "error querying database for updated modrinth mod icons list");
                return;
            }
            Err(e) => {
                error!({ error = ?e }, "error spawning blocking task for modrinth mod icons list query");
                return;
            }
        };

        let app = &app;
        let futures = modlist
            .into_iter()
            .map(|(filename, project_id, version_id, metadata_id, url)| async move {
                let mcm = app.meta_cache_manager();

                {
                    let fails = mcm.failed_mr_thumbs.read().await;
                    if let Some((time, _)) = fails.get(&project_id) {
                        if *time > std::time::Instant::now() {
                            return
                        } else {
                            mcm.failed_mr_thumbs.write().await.remove(&project_id);
                        }
                    }
                }

                let r = async {
                    let dl_guard = mcm
                        .image_download_semaphore
                        .acquire()
                        .await
                        .expect("the image download semaphore is never closed");


                    let icon = app.reqwest_client
                        .get(&url)
                        .header("avoid-caching", "")
                        .send()
                        .await?
                        .error_for_status()?
                        .bytes()
                        .await?;

                    drop(dl_guard);
                    let scale_guard = mcm
                        .image_scale_semaphore
                        .acquire()
                        .await
                        .expect("the image scale semaphore is never closed");

                    let image = icon.to_vec();

                    let image = carbon_scheduler::cpu_block(|| {
                        let scaled = super::scale_mod_image(&image[..])?;
                        Ok::<_, anyhow::Error>(scaled)
                    }).await?;

                    drop(scale_guard);

                    // Update image cache
                    let pool = app.db_pool.clone();
                    let metadata_id_clone = metadata_id.clone();
                    tokio::task::spawn_blocking(move || {
                        let conn = pool.get()?;
                        queries::metadata::UpdateModrinthModImageCacheData::execute(
                            &conn,
                            &metadata_id_clone,
                            Some(&image[..]),
                            1,
                        )?;
                        Ok::<_, anyhow::Error>(())
                    }).await??;

                    let _ = update_notifier.send(instance_id);
                    Ok::<_, anyhow::Error>(())
                }.await;

                if let Err(e) = r {
                    error!({ error = ?e }, "error downloading mod icon for {instance_id}/{filename} (project: {project_id}, version: {version_id}, image url: {})", url);

                    let mut fails = mcm.failed_mr_thumbs.write().await;
                    fails.entry(project_id)
                        .and_modify(|v| *v = (
                            std::time::Instant::now() + std::time::Duration::from_secs(u64::pow(2, v.1 + 1)),
                            v.1 + 1,
                        ))
                        .or_insert_with(|| (
                            std::time::Instant::now() + std::time::Duration::from_secs(2),
                            1
                        ));
                }
            });

        futures::future::join_all(futures).await.into_iter();
    }
}

// Cache modrinth metadata for a mod without downloading the icon
async fn cache_modrinth_meta_unchecked(
    app: &App,
    metadata_id: String,
    version: &Version,
    sha512: String,
    filename: String,
    file_url: String,
    project: Project,
    authors: String,
    versions: &[Version],
) -> anyhow::Result<()> {
    let mut file_update_paths = HashSet::<(&str, ModLoaderType, ModChannel)>::new();

    let mut versions_sorted = versions.iter().collect::<Vec<_>>();
    versions_sorted.sort_by(|f1, f2| Ord::cmp(&f2.date_published, &f1.date_published));

    for other_version in versions_sorted {
        if other_version.project_id != project.id
            || other_version.id == version.id
            || !version
                .game_versions
                .iter()
                .any(|v| other_version.game_versions.contains(v))
            || !version
                .loaders
                .iter()
                .any(|l| other_version.loaders.contains(l))
        {
            break;
        }

        for game_version in &other_version.game_versions {
            for loader in &other_version.loaders {
                let Ok(loader) = ModLoaderType::try_from(loader as &str) else {
                    continue;
                };

                file_update_paths.insert((game_version, loader, other_version.version_type.into()));
            }
        }
    }

    let update_paths = file_update_paths
        .into_iter()
        .map(|(gamever, loader, channel)| {
            format!(
                "{gamever},{},{}",
                loader.to_string().to_lowercase(),
                channel.as_str(),
            )
        })
        .join(";");

    // Check if entry exists and is recent
    let pool = app.db_pool.clone();
    let metadata_id_clone = metadata_id.clone();
    let existing_entry = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let result =
            queries::metadata::FindModrinthModCache::fetch_optional(&conn, &metadata_id_clone)?;
        Ok::<_, anyhow::Error>(result)
    })
    .await??;

    if let Some(cache) = existing_entry {
        // Check if the cached entry is recent (within 1 day)
        if cache.cached_at > (chrono::Utc::now() - chrono::Duration::days(1)) {
            return Ok(());
        }
    }

    // Upsert Modrinth mod cache
    let pool = app.db_pool.clone();
    let metadata_id_clone = metadata_id.clone();
    let sha512_clone = sha512.clone();
    let project_id = project.id.clone();
    let version_id = version.id.clone();
    let title = project.title.clone();
    let version_name = version.name.clone();
    let urlslug = project.slug.clone();
    let description = project.description.clone();
    let authors_clone = authors.clone();
    let release_type = ModChannel::from(version.version_type) as i32;
    let update_paths_clone = update_paths.clone();
    let filename_clone = filename.clone();
    let file_url_clone = file_url.clone();
    let cached_at = chrono::Utc::now().to_rfc3339();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::metadata::UpsertModrinthModCache::execute(
            &conn,
            &metadata_id_clone,
            &sha512_clone,
            &project_id,
            &version_id,
            &title,
            &version_name,
            &urlslug,
            &description,
            &authors_clone,
            release_type,
            &update_paths_clone,
            &filename_clone,
            &file_url_clone,
            &cached_at,
        )?;
        Ok::<_, anyhow::Error>(())
    })
    .await??;

    // Handle icon image cache
    if let Some(icon_url) = &project.icon_url {
        let pool = app.db_pool.clone();
        let metadata_id_clone = metadata_id.clone();
        let icon_url_clone = icon_url.clone();

        if let Err(e) = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::metadata::UpsertModrinthModImageCache::execute(
                &conn,
                &metadata_id_clone,
                &icon_url_clone,
                None, // upToDate = 0, mark as needing download
                0,
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .await?
        {
            warn!(
                "Failed to upsert modrinth image for metadata_id {}: {:?}",
                metadata_id, e
            );
        }
    }

    Ok(())
}
