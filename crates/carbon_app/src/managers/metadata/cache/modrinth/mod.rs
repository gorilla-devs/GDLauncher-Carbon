use super::eligibility::UpdateContext;
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
pub(crate) async fn target_compatibility(
    app: &App,
    entity_id: CacheEntityId,
) -> Option<UpdateContext> {
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

            let game = version.release.clone();
            drop(instances);
            use crate::managers::instance::shader_loader::{
                ShaderLoaderKind, ShaderRecommendation,
            };
            let shader_engines = match app
                .instance_manager()
                .check_shader_requirements(instance_id)
                .await
                .ok()
            {
                Some(ShaderRecommendation::LoaderPresent { loader }) => vec![match loader {
                    ShaderLoaderKind::Iris | ShaderLoaderKind::Oculus => "iris".into(),
                    ShaderLoaderKind::OptiFine => "optifine".into(),
                }],
                _ => Vec::new(),
            };
            Some(UpdateContext {
                game,
                loaders,
                shader_engines,
            })
        }
        CacheEntityId::Server(server_id) => {
            let server = carbon_repos::repos::server::get_server(&app.db, server_id)
                .await
                .ok()??;

            Some(UpdateContext {
                game: server.game_version,
                loaders: server
                    .modloader_type
                    .into_iter()
                    .map(|l| l.to_lowercase())
                    .collect(),
                shader_engines: vec![],
            })
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
    let mut paths = HashSet::<(&str, String, ModChannel)>::new();

    let updates = candidates.iter().filter(|candidate| {
        super::eligibility::strictly_newer(
            candidate.project_id == project_id,
            candidate.id == installed.id,
            &candidate.date_published,
            &installed.date_published,
        )
    });

    for update in updates {
        for game_version in &update.game_versions {
            for loader in &update.loaders {
                let loader = loader.to_lowercase();
                if ModLoaderType::try_from(loader.as_str()).is_err()
                    && !matches!(
                        loader.as_str(),
                        "minecraft" | "datapack" | "iris" | "optifine"
                    )
                {
                    continue;
                }
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

/// The files of `entity_id` a Modrinth lookup is due for, keyed by hash, each
/// with its metadata id, hash and addon kind. A file is due when its metadata
/// has no Modrinth cache row, one cached more than a day ago, or no fresh
/// update context for what the entity runs (`context`), unless a lookup found
/// no match for it within [`super::IGNORED_HASH_TTL`]. Worlds are directories
/// with name-derived pseudo-hashes, so there is nothing to match on the
/// platform (the instance query excludes them).
async fn due_for_lookup(
    app: &App,
    entity_id: CacheEntityId,
    context: &str,
) -> anyhow::Result<VecDeque<(String, (String, String, String))>> {
    let cutoff = DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset());
    let modlist = match entity_id {
        CacheEntityId::Instance(instance_id) => {
            let instance_id_val = *instance_id;
            mfcdb::instance_mods_needing_mr_context(&app.db, instance_id_val, context, cutoff)
                .await?
                .into_iter()
                .map(|m| {
                    let sha512 = hex::encode(&m.sha512);
                    (sha512.clone(), (m.metadata_id, sha512, m.addon_type))
                })
                .collect::<Vec<_>>()
        }
        CacheEntityId::Server(server_id) => {
            mfcdb::server_mods_needing_mr_context(&app.db, server_id, context, cutoff)
                .await?
                .into_iter()
                .map(|m| {
                    let sha512 = hex::encode(&m.sha512);
                    (sha512.clone(), (m.metadata_id, sha512, m.addon_type))
                })
                .collect::<Vec<_>>()
        }
    };

    let mcm = app.meta_cache_manager();
    let now = mcm.now();
    let ignored_hashes = mcm.ignored_remote_mr_hashes.read().await;

    Ok(modlist
        .into_iter()
        .filter(|(_, (_, sha512, _))| !ignored_hashes.contains(sha512, now))
        .collect())
}

pub struct ModrinthModCacher;

#[async_trait::async_trait]
impl ModplatformCacher for ModrinthModCacher {
    const NAME: &'static str = "modrinth";
    type SaveBundle = (
        Vec<String>,
        Vec<(String, String, String)>,
        VersionHashesResponse,
        ProjectsResponse,
        Vec<TeamResponse>,
        Vec<Version>,
        Option<UpdateContext>,
    );

    async fn query_platform(
        app: &App,
        entity_id: CacheEntityId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()> {
        let target_compat = target_compatibility(app, entity_id).await;
        let context = target_compat
            .as_ref()
            .map(UpdateContext::base_key)
            .unwrap_or_default();
        let mut modlist = due_for_lookup(app, entity_id, &context).await?;

        if modlist.is_empty() {
            return Ok(());
        }

        let total_mod_count = modlist.len();
        debug!(
            "Found {} mods to process for Modrinth caching",
            total_mod_count
        );

        let mcm = app.meta_cache_manager();
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
                let mut combined_versions_response = Vec::new();
                if let Some(context) = &target_compat {
                    let kinds = metadata
                        .iter()
                        .map(|(_, _, kind)| kind.as_str())
                        .collect::<HashSet<_>>();
                    for kind in kinds {
                        let loaders = context.request_loaders(kind);
                        if loaders.is_empty() {
                            continue;
                        }
                        let hashes = metadata
                            .iter()
                            .filter(|(_, _, k)| k == kind)
                            .map(|(_, hash, _)| hash.clone())
                            .collect::<Vec<_>>();
                        for version_type in
                            [VersionType::Release, VersionType::Beta, VersionType::Alpha]
                        {
                            mcm.modrinth_throttle.acquire().await;
                            let latest = mpm
                                .modrinth
                                .get_latest_versions_from_hashes(&LatestVersionsBody {
                                    hashes: hashes.clone(),
                                    algorithm: HashAlgorithm::SHA512,
                                    loaders: loaders.clone(),
                                    game_versions: vec![context.game.clone()],
                                    version_types: Some(vec![version_type]),
                                })
                                .await?;
                            combined_versions_response.extend(latest.0.into_values());
                        }
                    }
                }

                sender.send((
                    sha512_hashes,
                    metadata,
                    versions_response,
                    projects_response,
                    teams_response,
                    combined_versions_response,
                    target_compat.clone(),
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
        (sha512_hashes, batch, versions, projects, teams, combined_versions, context): Self::SaveBundle,
    ) {
        trace!("processing modrinth mod batch for {entity_id}");

        let matches = sha512_hashes
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
        let now = mcm.now();
        mcm.ignored_remote_mr_hashes.write().await.ignore(
            sha512_hashes
                .iter()
                .filter(|hash| !matches.contains_key(hash))
                .cloned(),
            now,
        );
        let unmatched = batch
            .iter()
            .filter(|(_, sha512, _)| !matches.contains_key(sha512))
            .map(|(metadata_id, _, _)| metadata_id.clone())
            .collect::<HashSet<_>>();
        super::log_unmatched(app, "Modrinth", entity_id, sha512_hashes.len(), &unmatched).await;

        let combined_versions = &combined_versions;
        let context = &context;
        let futures = batch.into_iter().filter_map(|(metadata_id, sha512, kind)| {
            let sha512_match = matches.get(&sha512).copied();
            sha512_match.map(|(project, team, version)| async move {
                // The version matched on the queried hash, but Modrinth's response
                // is external data: an odd or hash-mismatched result can still list
                // no file whose own hash matches. Skip just this mod rather than
                // panicking the shared query/save/icon task group, which would
                // silently stop all Modrinth caching for the rest of the session.
                //
                // `sha512` is `hex::encode`'s (always lowercase) output; `file.hashes.sha512`
                // is folded lowercase the same way at deserialize
                // (`carbon_platforms::modrinth::version::Hashes`) — both sides are
                // guaranteed lowercase, so a plain comparison suffices.
                let Some(file) = version
                    .files
                    .iter()
                    .find(|file| file.hashes.sha512 == sha512)
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

                if let Some(context) = context {
                    let candidates = combined_versions.iter().filter(|v| context.compatible(&kind, &v.game_versions, &v.loaders)).cloned().collect::<Vec<_>>();
                    let paths = build_update_paths(version, &project.id, &candidates);
                    if let Err(error) = metarepo::upsert_mr_update_context(&app.db, &metadata_id, &context.key(&kind), &paths,
                        DbDateTime(chrono::Utc::now().fixed_offset())).await {
                        error!(?error, "Could not store Modrinth update context");
                    }
                }

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

                if !super::thumbnail_retry_allowed(&mcm.failed_mr_thumbs, &project_id).await {
                    return;
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
                            std::time::Instant::now() + std::time::Duration::from_secs(u64::pow(2, v.1.saturating_add(1).min(10))),
                            v.1.saturating_add(1).min(10),
                        ))
                        .or_insert_with(|| (
                            std::time::Instant::now() + std::time::Duration::from_secs(2),
                            1
                        ));
                } else {
                    mcm.failed_mr_thumbs.write().await.remove(&project_id);
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

    // The repository validates the exact physical hash before publishing this
    // artifact URL, filename and owner. Other files of this version are distinct.
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

/// Records the Modrinth version an install just published as the identity of
/// its file's metadata row, from what the installer downloaded. The hash
/// lookup that follows can come back without the file, and the addon would
/// then lack its Modrinth project (the button that installed it reading
/// Download again) until a later lookup matched.
///
/// A row a lookup or an earlier install wrote is kept, and nothing is recorded
/// unless the row's own hash is `file`'s. The record is due for a lookup from
/// the start, which fills what the installer does not know (authors, update
/// paths) and replaces it, even with another version holding the same jar.
pub(crate) async fn record_installed_version(
    app: &App,
    metadata_id: &str,
    version: &Version,
    file: &carbon_platforms::modrinth::version::VersionFile,
    project: &Project,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        version.project_id == project.id,
        "Modrinth version {} is not of project {}",
        version.id,
        project.id
    );
    let recorded = metarepo::record_installed_mr_version(
        &app.db,
        metadata_id,
        &file.hashes.sha512,
        &project.id,
        &version.id,
        &project.title,
        &version.name,
        &project.slug,
        &project.description,
        ModChannel::from(version.version_type) as i32,
        &file.filename,
        &file.url,
    )
    .await?;
    if recorded == 0 {
        debug!(
            "Kept the Modrinth identity of {metadata_id}: it already has one, or its hash is not that of {}",
            file.filename
        );
        return Ok(());
    }

    if let Some(icon_url) = &project.icon_url {
        metarepo::upsert_mr_image(&app.db, metadata_id, icon_url).await?;
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
    fn task11_non_mod_loader_tags_produce_update_paths() {
        for loader in ["minecraft", "datapack", "iris", "optifine"] {
            let mut installed = version("installed", "project", 10, VersionType::Release);
            installed.loaders = vec![loader.into()];
            let mut newer = version("newer", "project", 20, VersionType::Release);
            newer.loaders = vec![loader.into()];
            assert!(
                !build_update_paths(&installed, "project", &[newer]).is_empty(),
                "{loader}"
            );
        }
    }

    #[tokio::test]
    async fn task11_shared_artifact_refresh_keeps_both_game_contexts() {
        use crate::domain::instance::{
            AddonType,
            info::{ModLoader, StandardVersion},
        };
        let app = crate::setup_managers_for_test().await;
        let mut instances = Vec::new();
        for (name, game) in [("context-a", "1.20.1"), ("context-b", "1.20.2")] {
            let (id, _) = super::super::tests::addon_fixture_version(
                &app,
                name,
                AddonType::Mods,
                "mod.jar",
                b"shared artifact",
                StandardVersion {
                    release: game.into(),
                    modloaders: [ModLoader {
                        type_: ModLoaderType::Forge,
                        version: "fixture".into(),
                    }]
                    .into_iter()
                    .collect(),
                },
            )
            .await;
            app.meta_cache_manager()
                .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
                .await
                .unwrap();
            let row = mfcdb::get_mod_files_by_instance(&app.db, *id)
                .await
                .unwrap()
                .remove(0);
            instances.push((id, row.metadata_id, game));
        }
        assert_eq!(
            instances[0].1, instances[1].1,
            "physical artifact stays shared"
        );
        let meta = metarepo::get_metadata_by_id(&app.db, &instances[0].1)
            .await
            .unwrap()
            .unwrap();
        let hash = hex::encode(meta.sha512);
        for (id, metadata, game) in &instances {
            let context = target_compatibility(&app, CacheEntityId::Instance(*id))
                .await
                .unwrap();
            let missing = mfcdb::instance_mods_needing_mr_context(
                &app.db,
                **id,
                &context.base_key(),
                DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset()),
            )
            .await
            .unwrap();
            assert_eq!(
                missing.len(),
                1,
                "a fresh shared artifact must not suppress missing target eligibility"
            );
            let mut installed = version("installed", "project", 10, VersionType::Release);
            installed.game_versions = vec!["1.20.1".into(), "1.20.2".into()];
            installed.files = vec![file(&hash)];
            let mut newer = version(&format!("new-{game}"), "project", 20, VersionType::Release);
            newer.game_versions = vec![(*game).into()];
            ModrinthModCacher::save_batch(
                &app,
                CacheEntityId::Instance(*id),
                (
                    vec![hash.clone()],
                    vec![(metadata.clone(), hash.clone(), "mods".into())],
                    VersionHashesResponse([(hash.clone(), installed)].into_iter().collect()),
                    ProjectsResponse(vec![project("project")]),
                    vec![team("project")],
                    vec![newer],
                    target_compatibility(&app, CacheEntityId::Instance(*id)).await,
                ),
            )
            .await;
        }
        for (id, _, game) in &instances {
            let listed = app.instance_manager().list_mods(*id, None).await.unwrap();
            assert!(
                listed[0].has_update,
                "{game} must retain its own newer compatible candidate"
            );
            let context = target_compatibility(&app, CacheEntityId::Instance(*id))
                .await
                .unwrap();
            assert!(
                mfcdb::instance_mods_needing_mr_context(
                    &app.db,
                    **id,
                    &context.base_key(),
                    DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset())
                )
                .await
                .unwrap()
                .is_empty()
            );
        }
        let (id, metadata, _) = &instances[1];
        let context = target_compatibility(&app, CacheEntityId::Instance(*id))
            .await
            .unwrap();
        // Negative eligibility has its own freshness and may not erase A's positive path.
        metarepo::upsert_mr_update_context(
            &app.db,
            metadata,
            &context.key("mods"),
            "",
            DbDateTime(chrono::Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        assert!(!app.instance_manager().list_mods(*id, None).await.unwrap()[0].has_update);
        assert!(
            app.instance_manager()
                .list_mods(instances[0].0, None)
                .await
                .unwrap()[0]
                .has_update
        );
        assert!(
            mfcdb::instance_mods_needing_mr_context(
                &app.db,
                **id,
                &context.base_key(),
                DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset())
            )
            .await
            .unwrap()
            .is_empty()
        );
        metarepo::upsert_mr_update_context(
            &app.db,
            metadata,
            &context.key("mods"),
            "",
            DbDateTime((chrono::Utc::now() - chrono::Duration::days(2)).fixed_offset()),
        )
        .await
        .unwrap();
        assert_eq!(
            mfcdb::instance_mods_needing_mr_context(
                &app.db,
                **id,
                &context.base_key(),
                DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset())
            )
            .await
            .unwrap()
            .len(),
            1
        );
    }

    /// A miss leaves the file out of lookups for `IGNORED_HASH_TTL`, not for
    /// the rest of the session: a lookup can answer without a file Modrinth
    /// serves, and the file then had no identity and no update checks until
    /// the app restarted.
    #[tokio::test]
    async fn a_missed_file_is_looked_up_again_once_its_ignore_entry_expires() {
        use crate::domain::instance::AddonType;
        let app = crate::setup_managers_for_test().await;
        let (id, _) = super::super::tests::addon_fixture(
            &app,
            "mr-miss",
            AddonType::Mods,
            "unknown.jar",
            b"not on modrinth",
        )
        .await;
        let entity = CacheEntityId::Instance(id);
        app.meta_cache_manager()
            .override_caching_and_wait(entity, false, false)
            .await
            .unwrap();
        let context = target_compatibility(&app, entity)
            .await
            .map(|context| context.base_key())
            .unwrap_or_default();
        async fn due(app: &App, entity: CacheEntityId, context: &str) -> Vec<String> {
            due_for_lookup(app, entity, context)
                .await
                .unwrap()
                .into_iter()
                .map(|(hash, _)| hash)
                .collect()
        }
        let (hash, metadata) = due_for_lookup(&app, entity, &context)
            .await
            .unwrap()
            .pop_front()
            .expect("an unidentified file is due for a lookup");

        ModrinthModCacher::save_batch(
            &app,
            entity,
            (
                vec![hash.clone()],
                vec![metadata],
                VersionHashesResponse(HashMap::new()),
                ProjectsResponse(Vec::new()),
                Vec::new(),
                Vec::new(),
                None,
            ),
        )
        .await;
        assert!(
            due(&app, entity, &context).await.is_empty(),
            "a miss is not asked about again at once"
        );

        let mcm = app.meta_cache_manager();
        mcm.advance_clock(super::super::IGNORED_HASH_TTL - Duration::from_secs(1));
        assert!(
            due(&app, entity, &context).await.is_empty(),
            "nor before its entry expires"
        );
        mcm.advance_clock(Duration::from_secs(2));
        assert_eq!(
            due(&app, entity, &context).await,
            vec![hash],
            "but again once it has"
        );
    }

    /// What an install records stands until a lookup answers, which fills in
    /// what the installer did not know and may name another version of the
    /// same jar; a lookup's answer is never overwritten by a later install.
    #[tokio::test]
    async fn an_install_record_holds_until_a_lookup_replaces_it() {
        use crate::domain::instance::{
            AddonType,
            info::{ModLoader, StandardVersion},
        };
        let app = crate::setup_managers_for_test().await;
        let (id, _) = super::super::tests::addon_fixture_version(
            &app,
            "install-record",
            AddonType::Mods,
            "mod.jar",
            b"installed jar",
            StandardVersion {
                release: "1.20.1".into(),
                modloaders: [ModLoader {
                    type_: ModLoaderType::Forge,
                    version: "fixture".into(),
                }]
                .into_iter()
                .collect(),
            },
        )
        .await;
        let entity = CacheEntityId::Instance(id);
        app.meta_cache_manager()
            .override_caching_and_wait(entity, false, false)
            .await
            .unwrap();
        let metadata_id = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0)
            .metadata_id;
        let hash = hex::encode(
            metarepo::get_metadata_by_id(&app.db, &metadata_id)
                .await
                .unwrap()
                .unwrap()
                .sha512,
        );
        /// The listed file's Modrinth version and authors, and its update badge.
        async fn listed(app: &App, id: InstanceId) -> (String, String, bool) {
            let listed = app
                .instance_manager()
                .list_mods(id, None)
                .await
                .unwrap()
                .remove(0);
            let modrinth = listed.modrinth.expect("the file has a Modrinth identity");
            (modrinth.version_id, modrinth.authors, listed.has_update)
        }

        // Another file's hash is refused.
        let mut installed = version("installed", "project", 10, VersionType::Release);
        installed.files = vec![file(&"d".repeat(128))];
        record_installed_version(
            &app,
            &metadata_id,
            &installed,
            &installed.files[0],
            &project("project"),
        )
        .await
        .unwrap();
        assert!(
            metarepo::get_mr_cache_by_metadata(&app.db, &metadata_id)
                .await
                .unwrap()
                .is_none()
        );

        installed.files = vec![file(&hash)];
        record_installed_version(
            &app,
            &metadata_id,
            &installed,
            &installed.files[0],
            &project("project"),
        )
        .await
        .unwrap();
        assert_eq!(
            listed(&app, id).await,
            ("installed".into(), String::new(), false)
        );
        let context = target_compatibility(&app, entity).await.unwrap();
        assert!(
            due_for_lookup(&app, entity, &context.base_key())
                .await
                .unwrap()
                .iter()
                .any(|(due, _)| *due == hash),
            "the record is due for a lookup"
        );

        // A lookup that misses leaves it as it is.
        let batch = || vec![(metadata_id.clone(), hash.clone(), "mods".to_string())];
        ModrinthModCacher::save_batch(
            &app,
            entity,
            (
                vec![hash.clone()],
                batch(),
                VersionHashesResponse(HashMap::new()),
                ProjectsResponse(Vec::new()),
                Vec::new(),
                Vec::new(),
                None,
            ),
        )
        .await;
        assert_eq!(
            listed(&app, id).await,
            ("installed".into(), String::new(), false)
        );

        // One that matches, here another version holding the same jar, takes
        // over with its authors and update context.
        let mut matched = version("matched", "project", 10, VersionType::Release);
        matched.files = vec![file(&hash)];
        let newer = version("newer", "project", 20, VersionType::Release);
        ModrinthModCacher::save_batch(
            &app,
            entity,
            (
                vec![hash.clone()],
                batch(),
                VersionHashesResponse([(hash.clone(), matched)].into_iter().collect()),
                ProjectsResponse(vec![project("project")]),
                vec![team("project")],
                vec![newer],
                Some(context),
            ),
        )
        .await;
        assert_eq!(
            listed(&app, id).await,
            ("matched".into(), "author".into(), true)
        );

        record_installed_version(
            &app,
            &metadata_id,
            &installed,
            &installed.files[0],
            &project("project"),
        )
        .await
        .unwrap();
        assert_eq!(
            listed(&app, id).await,
            ("matched".into(), "author".into(), true)
        );
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
        let good_sha512 = "c".repeat(128);
        let mut good_version = version("good-version", "good-project", 2, VersionType::Release);
        good_version.files = vec![file(&good_sha512)];

        let mut versions = HashMap::new();
        versions.insert(bad_sha512.clone(), bad_version);
        versions.insert(good_sha512.clone(), good_version);

        // As in `query_platform`, each queried hash is the hex of its
        // metadata row's own SHA-512, which the cache write checks again.
        insert_test_metadata(&app, "bad-metadata", 1, &bad_sha512, b"bad-sha1-raw").await;
        insert_test_metadata(&app, "good-metadata", 2, &good_sha512, b"good-sha1-raw").await;

        let bundle = (
            vec![bad_sha512.clone(), good_sha512.clone()],
            vec![
                (
                    "bad-metadata".to_string(),
                    bad_sha512.clone(),
                    "mods".into(),
                ),
                (
                    "good-metadata".to_string(),
                    good_sha512.clone(),
                    "mods".into(),
                ),
            ],
            VersionHashesResponse(versions),
            ProjectsResponse(vec![project("bad-project"), project("good-project")]),
            vec![team("bad-project"), team("good-project")],
            Vec::new(),
            None,
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
    /// in a different case still describes the same file, so it must cache
    /// rather than be skipped as unmatched -- being skipped would also park the
    /// hash in `ignored_remote_mr_hashes` and stop it being retried for the
    /// session. The file below is built through real JSON deserialization
    /// (rather than the `file()` literal-construction helper) so the fixture
    /// actually exercises `Hashes`' `deserialize_lowercase_hex` fold, the
    /// mechanism now responsible for this -- not an in-memory struct the fold
    /// never runs on.
    #[tokio::test]
    async fn a_differently_cased_response_hash_still_matches_the_queried_hash() {
        let app = crate::setup_managers_for_test().await;

        let queried_sha512 = "a".repeat(128);
        let mut version = version("version", "project", 1, VersionType::Release);
        let uppercase_sha512 = queried_sha512.to_uppercase();
        version.files = vec![
            serde_json::from_str(&format!(
                r#"{{
                    "hashes": {{ "sha512": "{uppercase_sha512}", "sha1": "irrelevant" }},
                    "url": "https://example.invalid/{uppercase_sha512}.jar",
                    "filename": "{uppercase_sha512}.jar",
                    "primary": true,
                    "size": 1,
                    "file_type": null
                }}"#
            ))
            .unwrap(),
        ];

        let mut versions = HashMap::new();
        versions.insert(queried_sha512.clone(), version);

        insert_test_metadata(&app, "metadata", 1, &queried_sha512, b"sha1-raw").await;

        let bundle = (
            vec![queried_sha512.clone()],
            vec![(
                "metadata".to_string(),
                queried_sha512.clone(),
                "mods".into(),
            )],
            VersionHashesResponse(versions),
            ProjectsResponse(vec![project("project")]),
            vec![team("project")],
            Vec::new(),
            None,
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

    /// A metadata row for a file whose SHA-512 is `sha512_hex`, the way a
    /// scan stores it (raw bytes, queried as their lowercase hex).
    async fn insert_test_metadata(
        app: &App,
        metadata_id: &str,
        murmur: i32,
        sha512_hex: &str,
        sha1: &[u8],
    ) {
        metarepo::insert_metadata(
            &app.db,
            metadata_id,
            murmur,
            &hex::decode(sha512_hex).unwrap(),
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
