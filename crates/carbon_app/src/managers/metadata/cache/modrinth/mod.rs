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
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::mod_metadata as metarepo;
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
            let server = carbon_repos::repos::server::get_server(&app.db, server_id)
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
        // Worlds are directories with name-derived pseudo-hashes, so there is
        // nothing to match on the platform (instance variant excludes them). A
        // file needs a refresh when its metadata has no Modrinth cache row, or
        // one cached more than a day ago.
        let cutoff = DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset());
        let modlist = match entity_id {
            CacheEntityId::Instance(instance_id) => {
                let instance_id_val = *instance_id;
                mfcdb::instance_mods_needing_mr_refresh(&app.db, instance_id_val, cutoff)
                    .await?
                    .into_iter()
                    .map(|m| {
                        let sha512 = hex::encode(&m.sha512);
                        (sha512.clone(), (m.metadata_id, sha512))
                    })
                    .collect::<Vec<_>>()
            }
            CacheEntityId::Server(server_id) => {
                mfcdb::server_mods_needing_mr_refresh(&app.db, server_id, cutoff)
                    .await?
                    .into_iter()
                    .map(|m| {
                        let sha512 = hex::encode(&m.sha512);
                        (sha512.clone(), (m.metadata_id, sha512))
                    })
                    .collect::<Vec<_>>()
            }
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
                // The version matched on the queried hash, but Modrinth's response
                // is external data: an odd or hash-mismatched result can still list
                // no file whose own hash matches. Skip just this mod rather than
                // panicking the shared query/save/icon task group, which would
                // silently stop all Modrinth caching for the rest of the session.
                let Some(file) = version
                    .files
                    .iter()
                    .find(|file| file.hashes.sha512.eq_ignore_ascii_case(&sha512))
                else {
                    let project_id = &project.id;
                    let version_id = &version.id;
                    warn!(
                        "Modrinth version {version_id} of project {project_id} has no file matching queried hash {sha512}; skipping this mod"
                    );
                    return;
                };

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
        // Collect the files whose Modrinth logo is stale (upToDate = 0) for mods
        // needing icon updates, from the appropriate file cache table.
        let result = match entity_id {
            CacheEntityId::Instance(instance_id) => {
                let instance_id_val = *instance_id;
                mfcdb::instance_mods_stale_mr_logo(&app.db, instance_id_val).await
            }
            CacheEntityId::Server(server_id) => {
                mfcdb::server_mods_stale_mr_logo(&app.db, server_id).await
            }
        };

        let modlist: Vec<mfcdb::MrLogoRefreshRow> = match result {
            Ok(list) => list,
            Err(e) => {
                error!({ error = ?e }, "error querying database for updated modrinth mod icons list");
                return;
            }
        };

        let app = &app;
        let futures = modlist
            .into_iter()
            .map(|row| async move {
                let filename = row.filename;
                let project_id = row.project_id;
                let version_id = row.version_id;
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

                    metarepo::mark_mr_image_downloaded(&app.db, &row.metadata_id, &image).await?;


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

    {
        let metadata_id = metadata_id.clone();
        if let Ok(Some(existing_entry)) =
            metarepo::get_mr_cache_by_metadata(&app.db, &metadata_id).await
        {
            if existing_entry.cached_at > (chrono::Utc::now() - chrono::Duration::days(1)) {
                return Ok(());
            }
        }
    }

    let release_type = ModChannel::from(version.version_type) as i32;
    let project_id = project.id.clone();
    let version_id = version.id.clone();
    let title = project.title.clone();
    let version_name = version.name.clone();
    let urlslug = project.slug.clone();
    let description = project.description.clone();
    let sha512_owned = sha512.clone();
    let authors_owned = authors.clone();
    let update_paths_owned = update_paths.clone();
    let filename_owned = filename.clone();
    let file_url_owned = file_url.clone();
    let metadata_id_owned = metadata_id.clone();

    // The composite `(projectId, versionId)` conflict may land on a row that
    // owns a different `metadataId`; the upsert returns the surviving one so the
    // image row attaches to the correct metadata.
    let result_metadata_id = metarepo::upsert_mr_mod_cache(
        &app.db,
        sha512_owned,
        project_id,
        version_id,
        title,
        version_name,
        urlslug,
        description,
        authors_owned,
        release_type,
        update_paths_owned,
        filename_owned,
        file_url_owned,
        DbDateTime(chrono::Utc::now().fixed_offset()),
        metadata_id_owned,
    )
    .await?;

    if let Some(icon_url) = &project.icon_url {
        let url = icon_url.clone();
        let image_metadata_id = result_metadata_id.clone();
        if let Err(e) = metarepo::upsert_mr_image(&app.db, &image_metadata_id, &url).await {
            warn!(
                "Failed to upsert modrinth image for metadata_id {}: {:?}",
                result_metadata_id, e
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use carbon_platforms::modrinth::UtcDateTime;
    use carbon_platforms::modrinth::project::{
        License, ProjectStatus, ProjectSupportRange, ProjectType,
    };
    use carbon_platforms::modrinth::user::{TeamMember, User};
    use carbon_platforms::modrinth::version::{Hashes, VersionFile};

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

    /// Walking every project's versions as one list would collect nothing as
    /// soon as another project's version came first.
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

    /// Regression test: a version can match on the queried hash while an odd or
    /// hash-mismatched Modrinth response still lists no file whose own hash
    /// matches it. `save_batch` must skip just that entry rather than panic --
    /// a panic here would take down the whole shared query/save/icon task group
    /// (`cache_modplatform`'s unsupervised `join!`), silently stopping ALL
    /// Modrinth caching for the rest of the session. The rest of the batch --
    /// including a perfectly normal entry right alongside the bad one -- must
    /// still cache.
    #[tokio::test]
    async fn odd_response_for_one_mod_does_not_abort_the_rest_of_the_batch() {
        let app = crate::setup_managers_for_test().await;

        // The version matched on this hash, but (an odd/hash-mismatched
        // response) none of its files actually carry that hash.
        let bad_sha512 = "b".repeat(128);
        let mut bad_version = version("bad-version", "bad-project", 1, VersionType::Release);
        bad_version.files = vec![file("some-other-hash")];

        // A normal response where the file really does carry the queried hash.
        let good_sha512 = "g".repeat(128);
        let mut good_version = version("good-version", "good-project", 2, VersionType::Release);
        good_version.files = vec![file(&good_sha512)];

        let mut versions = HashMap::new();
        versions.insert(bad_sha512.clone(), bad_version);
        versions.insert(good_sha512.clone(), good_version);

        insert_test_metadata(&app, "bad-metadata", 1, b"bad-sha512-raw", b"bad-sha1-raw").await;
        insert_test_metadata(
            &app,
            "good-metadata",
            2,
            b"good-sha512-raw",
            b"good-sha1-raw",
        )
        .await;

        let bundle = (
            vec![bad_sha512.clone(), good_sha512.clone()],
            vec![
                ("bad-metadata".to_string(), bad_sha512.clone()),
                ("good-metadata".to_string(), good_sha512.clone()),
            ],
            VersionHashesResponse(versions),
            ProjectsResponse(vec![project("bad-project"), project("good-project")]),
            vec![team("bad-project"), team("good-project")],
            Vec::new(),
        );

        // Must not panic: a panic here would take down the shared
        // query/save/icon task group and stop Modrinth caching for the rest
        // of the session.
        ModrinthModCacher::save_batch(&app, CacheEntityId::Server(1), bundle).await;

        let good_cached = metarepo::get_mr_cache_by_metadata(&app.db, "good-metadata")
            .await
            .unwrap();
        assert!(
            good_cached.is_some(),
            "the good entry alongside the bad one should still be cached"
        );

        let bad_cached = metarepo::get_mr_cache_by_metadata(&app.db, "bad-metadata")
            .await
            .unwrap();
        assert!(
            bad_cached.is_none(),
            "the bad entry has no matching file and must be skipped, not cached"
        );
    }

    /// Modrinth's stored file hash is external data, while the queried hash is
    /// this app's own lowercase `hex::encode` output. A response serving the hash
    /// in a different case still describes the same file, so it must cache rather
    /// than be skipped as unmatched -- being skipped would also park the hash in
    /// `ignored_remote_mr_hashes` and stop it being retried for the session.
    #[tokio::test]
    async fn a_differently_cased_response_hash_still_matches_the_queried_hash() {
        let app = crate::setup_managers_for_test().await;

        let queried_sha512 = "a".repeat(128);
        let mut version = version("version", "project", 1, VersionType::Release);
        version.files = vec![file(&queried_sha512.to_uppercase())];

        let mut versions = HashMap::new();
        versions.insert(queried_sha512.clone(), version);

        insert_test_metadata(&app, "metadata", 1, b"sha512-raw", b"sha1-raw").await;

        let bundle = (
            vec![queried_sha512.clone()],
            vec![("metadata".to_string(), queried_sha512.clone())],
            VersionHashesResponse(versions),
            ProjectsResponse(vec![project("project")]),
            vec![team("project")],
            Vec::new(),
        );

        ModrinthModCacher::save_batch(&app, CacheEntityId::Server(1), bundle).await;

        let cached = metarepo::get_mr_cache_by_metadata(&app.db, "metadata")
            .await
            .unwrap();
        assert!(
            cached.is_some(),
            "an uppercase response hash describes the same file and must still cache"
        );
    }

    async fn insert_test_metadata(
        app: &App,
        metadata_id: &str,
        murmur: i32,
        sha512: &[u8],
        sha1: &[u8],
    ) {
        metarepo::insert_metadata(
            &app.db,
            metadata_id,
            murmur,
            sha512,
            sha1,
            "forge",
            None,
            None,
            None,
            None,
            None,
            DbDateTime(chrono::Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
    }

    fn project(id: &str) -> Project {
        Project {
            slug: id.to_string(),
            title: id.to_string(),
            description: String::new(),
            categories: Vec::new(),
            client_side: ProjectSupportRange::Required,
            server_side: ProjectSupportRange::Required,
            body: String::new(),
            additional_categories: Vec::new(),
            issues_url: None,
            source_url: None,
            wiki_url: None,
            discord_url: None,
            donation_urls: Vec::new(),
            project_type: ProjectType::Mod,
            downloads: 0,
            icon_url: None,
            color: None,
            id: id.to_string(),
            team: format!("{id}-team"),
            moderator_message: None,
            published: "2026-01-01T00:00:00Z".parse::<UtcDateTime>().unwrap(),
            updated: "2026-01-01T00:00:00Z".parse::<UtcDateTime>().unwrap(),
            approved: None,
            followers: 0,
            status: ProjectStatus::Approved,
            license: License {
                id: "MIT".to_string(),
                name: "MIT".to_string(),
                url: None,
            },
            versions: Vec::new(),
            game_versions: vec!["1.20.1".to_string()],
            loaders: vec!["forge".to_string()],
            gallery: Vec::new(),
        }
    }

    fn team(project_id: &str) -> TeamResponse {
        TeamResponse(vec![TeamMember {
            team_id: format!("{project_id}-team"),
            user: User {
                username: "author".to_string(),
                name: None,
                id: "author-id".to_string(),
                avatar_url: None,
            },
            role: "Owner".to_string(),
            accepted: true,
            ordering: None,
        }])
    }

    fn file(sha512: &str) -> VersionFile {
        VersionFile {
            hashes: Hashes {
                sha512: sha512.to_string(),
                sha1: "irrelevant".to_string(),
                others: HashMap::new(),
            },
            url: format!("https://example.invalid/{sha512}.jar"),
            filename: format!("{sha512}.jar"),
            primary: true,
            size: 1,
            file_type: None,
        }
    }
}
