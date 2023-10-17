use std::collections::{HashMap, VecDeque};

use tokio::sync::mpsc;
use tracing::{trace, debug, error};
use itertools::Itertools;

use crate::{managers::App, domain::{instance::InstanceId, modplatforms::modrinth::{responses::{VersionHashesResponse, ProjectsResponse, TeamResponse}, version::HashAlgorithm, search::{TeamIDs, ProjectIDs, VersionHashesQuery}, project::Project}}, db::read_filters::{IntFilter, DateTimeFilter}};
use crate::db::{
    mod_file_cache as fcdb,
    mod_metadata as metadb, modrinth_mod_cache as mrdb, modrinth_mod_image_cache as mrimgdb,
};

use super::{BundleSender, ModplatformCacher};

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
                fcdb::WhereParam::MetadataIs(vec![
                    metadb::WhereParam::ModrinthIsNot(vec![
                        mrdb::WhereParam::CachedAt(DateTimeFilter::Gt(
                            (chrono::Utc::now() - chrono::Duration::days(1)).into(),
                        )),
                    ]),
                ]),
            ])
            .with(fcdb::metadata::fetch())
            .exec()
            .await?
            .into_iter()
            .map(|m| {
                let metadata = m.metadata.expect(
                    "metadata was queried with mod cache yet is not present",
                );
                let sha512 = hex::encode(&metadata.sha_512);

                (sha512.clone(), (metadata.id, sha512))
            });

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_mr_hashes.read().await;

        let mut modlist = modlist
            .filter(|(_, (_, sha512))| !ignored_hashes.contains(sha512))
            .collect::<VecDeque<_>>();

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

            sender.send((sha512_hashes, metadata, versions_response, projects_response, teams_response));
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

        let futures = batch
            .into_iter()
            .filter_map(|(metadata_id, sha512)| {
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
                            member.user.name.clone().unwrap_or_else(|| {
                                member.user.username.clone()
                            })
                        })
                        .join(", ");

                    let r = cache_modrinth_meta_unchecked(
                        app,
                        metadata_id,
                        version.id.clone(),
                        file.hashes.sha512.clone(),
                        project.clone(),
                        authors,
                    ).await;

                    if let Err(e) = r {
                        error!({ error = ?e }, "Could not store modrinth mod metadata");
                    }
                })
            });

        futures::future::join_all(futures).await;
    }

    async fn cache_icons(
        app: &App,
        instance_id: InstanceId,
        update_notifier: &mpsc::UnboundedSender<InstanceId>,
    ) {
        let modlist = app
            .prisma_client
            .mod_file_cache()
            .find_many(vec![
                fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                fcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::ModrinthIs(
                    vec![mrdb::WhereParam::LogoImageIs(vec![
                        mrimgdb::WhereParam::UpToDate(IntFilter::Equals(0)),
                    ])],
                )]),
            ])
            .with(
                fcdb::metadata::fetch().with(
                    metadb::modrinth::fetch().with(mrdb::logo_image::fetch()),
                ),
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

            (
                file.filename,
                mr.project_id,
                mr.version_id,
                row,
            )
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

        let mut o_delete_mrmeta = None;
        let mut o_insert_logo = None;
        let mut o_update_logo = None;
        let mut o_delete_logo = None;

        let o_insert_mrmeta = app.prisma_client.modrinth_mod_cache().create(
            sha512.clone(),
            project.id,
            version_id,
            project.title,
            project.slug,
            project.description,
            authors,
            chrono::Utc::now().into(),
            metadb::UniqueWhereParam::IdEquals(metadata_id.clone()),
            Vec::new(),
        );

        if let Some(prev) = prev {
            o_delete_mrmeta = Some(app.prisma_client.modrinth_mod_cache().delete(
                mrdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
            ));

            if let Some(prev) = prev
                .logo_image
                .expect("logo_image was requesred but not returned by prisma")
            {
                match project.icon_url.as_ref() {
                    Some(url) => {
                        if *url != prev.url {
                            o_update_logo =
                                Some(app.prisma_client.modrinth_mod_image_cache().update(
                                    mrimgdb::UniqueWhereParam::MetadataIdEquals(
                                        metadata_id.clone(),
                                    ),
                                    vec![
                                        mrimgdb::SetParam::SetUrl(url.clone()),
                                        mrimgdb::SetParam::SetUpToDate(0),
                                    ],
                                ));
                        }
                    }
                    None => {
                        o_delete_logo =
                            Some(app.prisma_client.modrinth_mod_image_cache().delete(
                                mrimgdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                            ));
                    }
                }
            }
        }

        if o_update_logo.is_none() && o_delete_logo.is_none() {
            if let Some(url) = project.icon_url {
                o_insert_logo = Some(app.prisma_client.modrinth_mod_image_cache().create(
                    url,
                    mrdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                    Vec::new(),
                ));
            }
        }

        debug!("updating modrinth metadata entry for {metadata_id}");

        app
            .prisma_client
            ._batch((
                o_delete_mrmeta.into_iter().collect::<Vec<_>>(),
                o_insert_mrmeta,
                o_delete_logo.into_iter().collect::<Vec<_>>(),
                o_insert_logo.into_iter().collect::<Vec<_>>(),
                o_update_logo.into_iter().collect::<Vec<_>>(),
            ))
            .await?;

        Ok(())
    }
