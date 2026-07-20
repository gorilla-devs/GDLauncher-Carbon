use super::{BundleSender, CacheEntityId, ModplatformCacher, UpdateNotifier};
use crate::domain::instance::InstanceId;
use crate::domain::instance::info::{GameVersion, ModLoaderType};
use crate::managers::App;
use crate::managers::instance::InstanceType;
use anyhow::anyhow;
use carbon_platforms::ModChannel;
use carbon_platforms::modrinth::version::Version;
use carbon_platforms::modrinth::{
    project::Project,
    responses::{ProjectsResponse, TeamResponse, VersionHashesResponse},
    search::{ProjectIDs, TeamIDs, VersionHashesQuery},
    version::{HashAlgorithm, LatestVersionsBody, VersionType},
};
use carbon_repos::db::read_filters::{DateTimeFilter, IntFilter, StringFilter};
use carbon_repos::db::{
    mod_file_cache as fcdb, mod_metadata as metadb, modrinth_mod_cache as mrdb,
    modrinth_mod_image_cache as mrimgdb, server as serverdb, server_mod_file_cache as sfcdb,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, error, trace, warn};

pub mod modpack;

/// The game version and loaders an entity runs, as Modrinth spells them.
///
/// Update paths are only ever read filtered by this pair, so it is also the only
/// filter worth asking the API about. `None` when the entity has no usable
/// version yet, in which case update paths are left alone.
async fn target_compatibility(
    app: &App,
    entity_id: CacheEntityId,
) -> Option<(Vec<String>, Vec<String>)> {
    match entity_id {
        CacheEntityId::Instance(instance_id) => {
            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let instance = instances.get(&instance_id)?;

            let InstanceType::Valid(data) = &instance.type_ else {
                return None;
            };

            let Some(GameVersion::Standard(version)) = data.game_version() else {
                return None;
            };

            let loaders = version
                .modloaders
                .iter()
                .map(|loader| loader.type_.to_string().to_lowercase())
                .collect::<Vec<_>>();

            (!loaders.is_empty()).then(|| (vec![version.release.clone()], loaders))
        }
        CacheEntityId::Server(server_id) => {
            let server = app
                .prisma_client
                .server()
                .find_unique(serverdb::UniqueWhereParam::IdEquals(server_id))
                .exec()
                .await
                .ok()??;

            let loader = server.modloader_type?;

            Some((vec![server.game_version], vec![loader.to_lowercase()]))
        }
    }
}

/// The `gamever,loader,channel` triples an installed file can be updated along,
/// in the `;`-separated form the cache stores and the mod list parses back.
///
/// `candidates` are the versions the platform reported as compatible with what
/// the entity runs. A candidate only describes an update when it belongs to the
/// same project and was published after the installed file: the newest version
/// for this entity's game version can predate a file installed for another one.
fn build_update_paths(installed: &Version, project_id: &str, candidates: &[Version]) -> String {
    let mut paths = HashSet::<(&str, ModLoaderType, ModChannel)>::new();

    let updates = candidates.iter().filter(|candidate| {
        candidate.project_id == project_id
            && candidate.id != installed.id
            && candidate.date_published > installed.date_published
    });

    for update in updates {
        for game_version in &update.game_versions {
            for loader in &update.loaders {
                let Ok(loader) = ModLoaderType::try_from(loader as &str) else {
                    continue;
                };

                paths.insert((game_version, loader, update.version_type.into()));
            }
        }
    }

    paths
        .into_iter()
        .map(|(gamever, loader, channel)| {
            format!(
                "{gamever},{},{}",
                loader.to_string().to_lowercase(),
                channel.as_str(),
            )
        })
        .join(";")
}

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
        entity_id: CacheEntityId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()> {
        let modlist = match entity_id {
            CacheEntityId::Instance(instance_id) => app
                .prisma_client
                .mod_file_cache()
                .find_many(vec![
                    fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                    // Worlds are directories with name-derived pseudo-hashes,
                    // so there is nothing to match on the platform.
                    fcdb::WhereParam::AddonType(StringFilter::Not(
                        crate::domain::instance::AddonType::Worlds
                            .to_db_string()
                            .to_string(),
                    )),
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
                })
                .collect::<Vec<_>>(),
            CacheEntityId::Server(server_id) => app
                .prisma_client
                .server_mod_file_cache()
                .find_many(vec![
                    sfcdb::WhereParam::ServerId(IntFilter::Equals(server_id)),
                    sfcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::ModrinthIsNot(vec![
                        mrdb::WhereParam::CachedAt(DateTimeFilter::Gt(
                            (chrono::Utc::now() - chrono::Duration::days(1)).into(),
                        )),
                    ])]),
                ])
                .with(sfcdb::metadata::fetch())
                .exec()
                .await?
                .into_iter()
                .map(|m| {
                    let metadata = m
                        .metadata
                        .expect("metadata was queried with server mod cache yet is not present");
                    let sha512 = hex::encode(&metadata.sha_512);
                    (sha512.clone(), (metadata.id, sha512))
                })
                .collect::<Vec<_>>(),
        };

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_mr_hashes.read().await;

        let mut modlist = modlist
            .into_iter()
            .filter(|(_, (_, sha512))| !ignored_hashes.contains(sha512))
            .collect::<VecDeque<_>>();

        if modlist.is_empty() {
            return Ok(());
        }

        let total_mod_count = modlist.len();
        debug!(
            "Found {} mods to process for Modrinth caching",
            total_mod_count
        );

        let failed_instances = mcm.failed_mr_instances.read().await;
        let delay = failed_instances.get(&entity_id);

        if let Some((end_time, _)) = delay {
            if Instant::now() < *end_time {
                warn!(
                    "Not attempting to cache modrinth mods for {entity_id} as too many attempts have failed recently"
                );
                return Ok(());
            }
        }

        drop(failed_instances);

        let target_compat = target_compatibility(app, entity_id).await;

        let fut = async {
            while !modlist.is_empty() {
                let (sha512_hashes, metadata) = modlist
                    .drain(0..usize::min(1000, modlist.len()))
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                trace!("querying modrinth mod batch for {entity_id}");

                mcm.modrinth_throttle.acquire().await;
                let versions_response = app
                    .modplatforms_manager()
                    .modrinth
                    .get_versions_from_hash(&VersionHashesQuery {
                        hashes: sha512_hashes.clone(),
                        algorithm: HashAlgorithm::SHA512,
                    })
                    .await?;

                mcm.modrinth_throttle.acquire().await;
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

                mcm.modrinth_throttle.acquire().await;
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

                // Update paths are only ever read filtered by what the entity runs,
                // so let the server pick the newest version for that game version
                // and loader. Fetching every version every project ever published
                // costs hundreds of requests and answers the same question.
                //
                // Each channel is asked separately: a mod whose newest build is a
                // beta may still have a newer stable one than the installed file,
                // and which of them counts as an update is the user's choice.
                let combined_versions_response = match &target_compat {
                    Some((game_versions, loaders)) => {
                        let mut newest_per_channel = Vec::new();

                        for version_type in
                            [VersionType::Release, VersionType::Beta, VersionType::Alpha]
                        {
                            mcm.modrinth_throttle.acquire().await;
                            let latest = mpm
                                .modrinth
                                .get_latest_versions_from_hashes(&LatestVersionsBody {
                                    hashes: sha512_hashes.clone(),
                                    algorithm: HashAlgorithm::SHA512,
                                    loaders: loaders.clone(),
                                    game_versions: game_versions.clone(),
                                    version_types: Some(vec![version_type]),
                                })
                                .await?;

                            newest_per_channel.extend(latest.0.into_values());
                        }

                        newest_per_channel
                    }
                    // Without a known game version and loader there is nothing to
                    // ask for; mods are still cached, only update paths are skipped.
                    None => Vec::new(),
                };

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
            error!({ error = ?e }, "Error occured while caching modrinth mods for {entity_id}");

            let mut failed_instances = mcm.failed_mr_instances.write().await;
            let entry = failed_instances
                .entry(entity_id)
                .or_insert((Instant::now(), 0));
            entry.0 = Instant::now() + Duration::from_secs(u64::pow(2, entry.1));
            entry.1 += 1;
        } else {
            let mut failed_instances = mcm.failed_mr_instances.write().await;
            failed_instances.remove(&entity_id);
        }

        Ok::<_, anyhow::Error>(())
    }

    async fn save_batch(
        app: &App,
        entity_id: CacheEntityId,
        (sha512_hashes, batch, versions, projects, teams, combined_versions): Self::SaveBundle,
    ) {
        trace!("processing modrinth mod batch for {entity_id}");

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

    async fn cache_icons(app: &App, entity_id: CacheEntityId, update_notifier: &UpdateNotifier) {
        // Collect (filename, project_id, version_id, image_row) for mods needing icon updates.
        let modlist: Vec<(String, String, String, _)> = match entity_id {
            CacheEntityId::Instance(instance_id) => {
                let result = app
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

                match result {
                    Ok(list) => list
                        .into_iter()
                        .map(|file| {
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
                        })
                        .collect(),
                    Err(e) => {
                        error!({ error = ?e }, "error querying database for updated modrinth mod icons list");
                        return;
                    }
                }
            }
            CacheEntityId::Server(server_id) => {
                let result = app
                    .prisma_client
                    .server_mod_file_cache()
                    .find_many(vec![
                        sfcdb::WhereParam::ServerId(IntFilter::Equals(server_id)),
                        sfcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::ModrinthIs(vec![
                            mrdb::WhereParam::LogoImageIs(vec![mrimgdb::WhereParam::UpToDate(
                                IntFilter::Equals(0),
                            )]),
                        ])]),
                    ])
                    .with(
                        sfcdb::metadata::fetch()
                            .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
                    )
                    .exec()
                    .await;

                match result {
                    Ok(list) => list
                        .into_iter()
                        .map(|file| {
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
                        })
                        .collect(),
                    Err(e) => {
                        error!({ error = ?e }, "error querying database for updated modrinth mod icons list");
                        return;
                    }
                }
            }
        };

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

                    let image = carbon_scheduler::cpu_block(|| {
                        let scaled = super::scale_mod_image(&image[..])?;
                        Ok::<_, anyhow::Error>(scaled)
                    }).await?;

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


                    let _ = update_notifier.send(entity_id);
                    Ok::<_, anyhow::Error>(())
                }.await;

                if let Err(e) = r {
                    error!({ error = ?e }, "error downloading mod icon for {entity_id}/{filename} (project: {project_id}, version: {version_id}, image url: {})", row.url);

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
    let update_paths = build_update_paths(version, &project.id, versions);

    if let Ok(Some(existing_entry)) = app
        .prisma_client
        .modrinth_mod_cache()
        .find_unique(mrdb::UniqueWhereParam::MetadataIdEquals(
            metadata_id.clone(),
        ))
        .exec()
        .await
    {
        if existing_entry.cached_at > (chrono::Utc::now() - chrono::Duration::days(1)) {
            return Ok(());
        }
    }

    let cache_result = app
        .prisma_client
        .modrinth_mod_cache()
        .upsert(
            mrdb::UniqueWhereParam::ProjectIdVersionIdEquals(
                project.id.clone(),
                version.id.clone(),
            ),
            mrdb::create(
                sha512.clone(),
                project.id.clone(),
                version.id.clone(),
                project.title.clone(),
                version.name.clone(),
                project.slug.clone(),
                project.description.clone(),
                authors.clone(),
                ModChannel::from(version.version_type) as i32,
                update_paths.clone(),
                filename.clone(),
                file_url.clone(),
                chrono::Utc::now().into(),
                metadb::UniqueWhereParam::IdEquals(metadata_id.clone()),
                Vec::new(),
            ),
            vec![
                mrdb::SetParam::SetSha512(sha512.clone()),
                mrdb::SetParam::SetProjectId(project.id.clone()),
                mrdb::SetParam::SetVersionId(version.id.clone()),
                mrdb::SetParam::SetTitle(project.title.clone()),
                mrdb::SetParam::SetVersion(version.name.clone()),
                mrdb::SetParam::SetUrlslug(project.slug.clone()),
                mrdb::SetParam::SetDescription(project.description.clone()),
                mrdb::SetParam::SetAuthors(authors.clone()),
                mrdb::SetParam::SetReleaseType(ModChannel::from(version.version_type) as i32),
                mrdb::SetParam::SetUpdatePaths(update_paths.clone()),
                mrdb::SetParam::SetFilename(filename.clone()),
                mrdb::SetParam::SetFileUrl(file_url.clone()),
                mrdb::SetParam::SetCachedAt(chrono::Utc::now().into()),
            ],
        )
        .exec()
        .await?;

    if let Some(icon_url) = &project.icon_url {
        if let Err(e) = app
            .prisma_client
            .modrinth_mod_image_cache()
            .upsert(
                mrimgdb::UniqueWhereParam::MetadataIdEquals(cache_result.metadata_id.clone()),
                mrimgdb::create(
                    icon_url.clone(),
                    mrdb::UniqueWhereParam::MetadataIdEquals(cache_result.metadata_id.clone()),
                    vec![
                        mrimgdb::SetParam::SetUpToDate(0), // Mark as needing download
                        mrimgdb::SetParam::SetData(None),
                    ],
                ),
                vec![
                    mrimgdb::SetParam::SetUrl(icon_url.clone()),
                    mrimgdb::SetParam::SetUpToDate(0), // Mark as needing download on update
                ],
            )
            .exec()
            .await
        {
            warn!(
                "Failed to upsert modrinth image for metadata_id {}: {:?}",
                cache_result.metadata_id, e
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use carbon_platforms::modrinth::UtcDateTime;
    use carbon_platforms::modrinth::version::VersionType;

    fn version(id: &str, project: &str, day: u32, version_type: VersionType) -> Version {
        Version {
            name: id.to_string(),
            version_number: id.to_string(),
            changelog: None,
            dependencies: Vec::new(),
            game_versions: vec!["1.20.1".to_string()],
            version_type,
            loaders: vec!["forge".to_string()],
            featured: false,
            status: None,
            requested_status: None,
            id: id.to_string(),
            project_id: project.to_string(),
            author_id: "author".to_string(),
            date_published: format!("2026-01-{day:02}T00:00:00Z")
                .parse::<UtcDateTime>()
                .unwrap(),
            downloads: 0,
            files: Vec::new(),
        }
    }

    #[test]
    fn no_update_paths_without_candidates() {
        let installed = version("installed", "project", 10, VersionType::Release);

        assert_eq!(build_update_paths(&installed, "project", &[]), "");
    }

    #[test]
    fn the_installed_version_is_not_an_update_of_itself() {
        let installed = version("installed", "project", 10, VersionType::Release);

        assert_eq!(
            build_update_paths(&installed, "project", &[installed.clone()]),
            ""
        );
    }

    #[test]
    fn only_versions_published_after_the_installed_one_are_updates() {
        let installed = version("installed", "project", 10, VersionType::Release);
        let older = version("older", "project", 5, VersionType::Release);
        let newer = version("newer", "project", 20, VersionType::Release);

        assert_eq!(build_update_paths(&installed, "project", &[older]), "");
        assert_eq!(
            build_update_paths(&installed, "project", &[newer]),
            "1.20.1,forge,stable"
        );
    }

    /// Every project's versions used to be walked as one list, which collected
    /// nothing as soon as another project's version came first.
    #[test]
    fn versions_of_other_projects_are_ignored_without_hiding_later_ones() {
        let installed = version("installed", "project", 10, VersionType::Release);
        // Published later than the real update and distinguishable, so including
        // it would both hide the update and show up in the result.
        let mut foreign = version("foreign", "other-project", 30, VersionType::Release);
        foreign.game_versions = vec!["1.19.2".to_string()];
        let newer = version("newer", "project", 20, VersionType::Release);

        assert_eq!(
            build_update_paths(&installed, "project", &[foreign, newer]),
            "1.20.1,forge,stable"
        );
    }

    #[test]
    fn each_channel_is_reported_so_the_allowed_one_can_be_picked() {
        let installed = version("installed", "project", 10, VersionType::Release);
        let stable = version("stable", "project", 20, VersionType::Release);
        let beta = version("beta", "project", 30, VersionType::Beta);

        let paths = build_update_paths(&installed, "project", &[stable, beta]);
        let mut paths = paths.split(';').collect::<Vec<_>>();
        paths.sort();

        assert_eq!(paths, vec!["1.20.1,forge,beta", "1.20.1,forge,stable"]);
    }

    #[test]
    fn unknown_loaders_are_skipped() {
        let installed = version("installed", "project", 10, VersionType::Release);
        let mut newer = version("newer", "project", 20, VersionType::Release);
        newer.loaders = vec!["not-a-loader".to_string()];

        assert_eq!(build_update_paths(&installed, "project", &[newer]), "");
    }
}
