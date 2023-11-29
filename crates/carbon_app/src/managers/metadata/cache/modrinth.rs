use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use itertools::Itertools;
use tracing::{debug, error, trace, warn};

use crate::db::{
    mod_file_cache as fcdb, mod_metadata as metadb, modrinth_mod_cache as mrdb,
    modrinth_mod_image_cache as mrimgdb,
};
use crate::{
    db::read_filters::{DateTimeFilter, IntFilter},
    domain::{
        instance::InstanceId,
        modplatforms::modrinth::{
            project::Project,
            responses::{ProjectsResponse, TeamResponse, VersionHashesResponse},
            search::{ProjectIDs, TeamIDs, VersionHashesQuery},
            version::HashAlgorithm,
        },
    },
    managers::App,
};

use super::{BundleSender, ModplatformCacher, UpdateNotifier};

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
    );

    async fn query_platform(
        app: &App,
        instance_id: InstanceId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()> {
        let modlist = app
            .prisma_client
            .mod_file_cache()
            .find_many(vec![
                fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                fcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::ModrinthIsNot(vec![
                    mrdb::WhereParam::CachedAt(DateTimeFilter::Gt(
                        (chrono::Utc::now() - chrono::Duration::days(1)).into(),
                    )),
                ])]),
            ])
            .with(fcdb::metadata::fetch())
            .exec()
            .await?
            .into_iter()
            .map(|m| {
                let metadata = m
                    .metadata
                    .expect("metadata was queried with mod cache yet is not present");
                let sha512 = hex::encode(&metadata.sha_512);

                (sha512.clone(), (metadata.id, sha512))
            });

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_mr_hashes.read().await;

        let mut modlist = modlist
            .filter(|(_, (_, sha512))| !ignored_hashes.contains(sha512))
            .collect::<VecDeque<_>>();

        if modlist.is_empty() {
            return Ok(());
        }

        let failed_instances = mcm.failed_mr_instances.read().await;
        let delay = failed_instances.get(&instance_id);

        if let Some((end_time, _)) = delay {
            if Instant::now() < *end_time {
                warn!("Not attempting to cache modrinth mods for {instance_id} as too many attempts have failed recently");
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

                sender.send((
                    sha512_hashes,
                    metadata,
                    versions_response,
                    projects_response,
                    teams_response,
                ));
            }

            Ok::<_, anyhow::Error>(())
        };

        if let Err(e) = fut.await {
            error!({ error = ?e }, "Error occured while caching modrinth mods for instance {instance_id}");

            let mut failed_instances = mcm.failed_cf_instances.write().await;
            let entry = failed_instances
                .entry(instance_id)
                .or_insert((Instant::now(), 0));
            entry.0 = Instant::now() + Duration::from_secs(u64::pow(2, entry.1));
            entry.1 += 1;
        } else {
            let mut failed_instances = mcm.failed_cf_instances.write().await;
            failed_instances.remove(&instance_id);
        }

        Ok::<_, anyhow::Error>(())
    }

    async fn save_batch(
        app: &App,
        instance_id: InstanceId,
        (sha512_hashes, batch, versions, projects, teams): Self::SaveBundle,
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
                    version.id.clone(),
                    file.hashes.sha512.clone(),
                    file.filename.clone(),
                    file.url.clone(),
                    project.clone(),
                    authors,
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
        let modlist = app
            .prisma_client
            .mod_file_cache()
            .find_many(vec![
                fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                fcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::ModrinthIs(vec![
                    mrdb::WhereParam::LogoImageIs(vec![mrimgdb::WhereParam::UpToDate(
                        IntFilter::Equals(0),
                    )]),
                ])]),
            ])
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
            )
            .exec()
            .await;

        let modlist = match modlist {
            Ok(modlist) => modlist,
            Err(e) => {
                error!({ error = ?e }, "error querying database for updated curseforge mod icons list");
                return;
            }
        };

        let modlist = modlist.into_iter().map(|file| {
            let meta = file
                .metadata
                .expect("metadata was ensured present but not returned");
            let mr = meta
                .modrinth
                .flatten()
                .expect("modrinth was ensured present but not returned");
            let row = mr
                .logo_image
                .flatten()
                .expect("mod image was ensured present but not returned");

            (file.filename, mr.project_id, mr.version_id, row)
        });

        let app = &app;
        let futures = modlist
            .into_iter()
            .map(|(filename, project_id, version_id, row)| async move {
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

                    debug!("thumbnailing modrinth mod icon for {instance_id}/{filename} (project: {project_id}, version: {version_id})");

                    let icon = app.reqwest_client
                        .get(&row.url)
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

                    let image = tokio::task::spawn_blocking(move || {
                        let scaled = super::scale_mod_image(&image[..])?;
                        Ok::<_, anyhow::Error>(scaled)
                    })
                        .await??;

                    drop(scale_guard);

                    app.prisma_client.modrinth_mod_image_cache()
                        .update(
                            mrimgdb::UniqueWhereParam::MetadataIdEquals(row.metadata_id.clone()),
                            vec![
                                mrimgdb::SetParam::SetUpToDate(1),
                                mrimgdb::SetParam::SetData(Some(image))
                            ]
                        )
                        .exec()
                        .await?;

                    debug!("saved modrinth mod thumbnail for {instance_id}/{filename} (project: {project_id}, version: {version_id})");

                    let _ = update_notifier.send(instance_id);
                    Ok::<_, anyhow::Error>(())
                }.await;

                if let Err(e) = r {
                    error!({ error = ?e }, "error downloading mod icon for {instance_id}/{filename} (project: {project_id}, version: {version_id}, image url: {})", row.url);

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
    version_id: String,
    sha512: String,
    filename: String,
    file_url: String,
    project: Project,
    authors: String,
) -> anyhow::Result<()> {
    let prev = app
        .prisma_client
        .modrinth_mod_cache()
        .find_unique(mrdb::UniqueWhereParam::MetadataIdEquals(
            metadata_id.clone(),
        ))
        .with(mrdb::logo_image::fetch())
        .exec()
        .await?;

    let o_insert_mrmeta = app.prisma_client.modrinth_mod_cache().create(
        sha512.clone(),
        project.id,
        version_id,
        project.title,
        project.slug,
        project.description,
        authors,
        filename,
        file_url,
        chrono::Utc::now().into(),
        metadb::UniqueWhereParam::IdEquals(metadata_id.clone()),
        Vec::new(),
    );

    let o_delete_mrmeta = prev.as_ref().map(|_| {
        app.prisma_client
            .modrinth_mod_cache()
            .delete(mrdb::UniqueWhereParam::MetadataIdEquals(
                metadata_id.clone(),
            ))
    });

    let old_image = prev
        .map(|p| {
            p.logo_image
                .expect("logo_image was requested but not returned by prisma")
        })
        .flatten();
    let new_image = project.icon_url;

    let image = match (new_image, old_image) {
        (Some(new), Some(old)) => Some((old.up_to_date == 1 && new == old.url, new, old.data)),
        (Some(new), None) => Some((false, new, None)),
        (None, Some(old)) => Some((old.up_to_date == 1, old.url, old.data)),
        (None, None) => None,
    };

    let o_insert_logo = image.map(|(up_to_date, url, data)| {
        app.prisma_client.modrinth_mod_image_cache().create(
            url,
            mrdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
            vec![
                mrimgdb::SetParam::SetUpToDate(if up_to_date { 1 } else { 0 }),
                mrimgdb::SetParam::SetData(data),
            ],
        )
    });

    debug!("updating modrinth metadata entry for {metadata_id}");

    app.prisma_client
        ._batch((
            o_delete_mrmeta.into_iter().collect::<Vec<_>>(),
            o_insert_mrmeta,
            o_insert_logo.into_iter().collect::<Vec<_>>(),
        ))
        .await?;

    Ok(())
}
