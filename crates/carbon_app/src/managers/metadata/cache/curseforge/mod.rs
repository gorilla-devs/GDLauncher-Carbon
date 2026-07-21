use super::BundleSender;
use super::CacheEntityId;
use super::ModplatformCacher;
use super::UpdateNotifier;
use crate::domain::instance::InstanceId;
use crate::domain::instance::info::ModLoaderType;
use crate::managers::App;
use carbon_platforms::ModChannel;
use carbon_platforms::curseforge::File;
use carbon_platforms::curseforge::FileReleaseType;
use carbon_platforms::curseforge::FingerprintsMatchesResult;
use carbon_platforms::curseforge::Mod;
use carbon_platforms::curseforge::filters::ModFilesParameters;
use carbon_platforms::curseforge::filters::ModFilesParametersQuery;
use carbon_platforms::curseforge::filters::ModParameters;
use carbon_platforms::curseforge::filters::ModsParameters;
use carbon_platforms::curseforge::filters::ModsParametersBody;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::mod_metadata as metarepo;
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;

pub mod modpack;

pub struct CurseforgeModCacher;

#[async_trait::async_trait]
impl ModplatformCacher for CurseforgeModCacher {
    const NAME: &'static str = "curseforge";

    type SaveBundle = (
        Vec<u32>,
        Vec<(String, u32)>,
        FingerprintsMatchesResult,
        Vec<(Mod, Vec<File>)>,
    );

    async fn query_platform(
        app: &App,
        entity_id: CacheEntityId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()> {
        // Worlds are directories with name-derived pseudo-hashes, so there is
        // nothing to match on the platform (instance variant excludes them; the
        // server table has no world addons). A file needs a refresh when its
        // metadata has no CurseForge cache row, or one cached more than a day ago.
        let cutoff = DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset());
        let modlist = match entity_id {
            CacheEntityId::Instance(instance_id) => {
                let instance_id_val = *instance_id;
                mfcdb::instance_mods_needing_cf_refresh(&app.db, instance_id_val, cutoff)
                    .await?
                    .into_iter()
                    .map(|m| (m.murmur2 as u32, (m.metadata_id, m.murmur2 as u32)))
                    .collect::<Vec<_>>()
            }
            CacheEntityId::Server(server_id) => mfcdb::server_mods_needing_cf_refresh(&app.db, server_id, cutoff)
                .await?
                .into_iter()
                .map(|m| (m.murmur2 as u32, (m.metadata_id, m.murmur2 as u32)))
                .collect::<Vec<_>>(),
        };

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_cf_hashes.read().await;

        let mut modlist = modlist
            .into_iter()
            .filter(|(_, (_, murmur2))| !ignored_hashes.contains(murmur2))
            .collect::<VecDeque<_>>();

        drop(ignored_hashes);

        if modlist.is_empty() {
            return Ok(());
        }

        let total_mod_count = modlist.len();
        debug!(
            "Found {} mods to process for CurseForge caching",
            total_mod_count
        );

        let failed_instances = mcm.failed_cf_instances.read().await;
        let delay = failed_instances.get(&entity_id);

        if let Some((end_time, _)) = delay {
            if Instant::now() < *end_time {
                warn!(
                    "Not attempting to cache curseforge mods for {entity_id} as too many attempts have failed recently"
                );
                return Ok(());
            }
        }

        drop(failed_instances);

        let fut = async {
            while !modlist.is_empty() {
                let (fingerprints, metadata) = modlist
                    .drain(0..usize::min(1000, modlist.len()))
                    .unzip::<_, _, Vec<_>, Vec<_>>();

                trace!("querying curseforge mod batch for {entity_id}");

                let fp_response = app
                    .modplatforms_manager()
                    .curseforge
                    .get_fingerprints(&fingerprints[..])
                    .await?
                    .data;

                let mpm = app.modplatforms_manager();
                let mod_responses = fp_response.exact_matches.iter().map(|m| async {
                    let cfmod = mpm
                        .curseforge
                        .get_mod(ModParameters {
                            mod_id: m.file.mod_id,
                        })
                        .await?;

                    let files = mpm
                        .curseforge
                        .get_mod_files(ModFilesParameters {
                            mod_id: m.file.mod_id,
                            query: ModFilesParametersQuery {
                                game_version: None,
                                mod_loader_type: None,
                                game_version_type_id: None,
                                index: None,
                                page_size: None,
                            },
                        })
                        .await?;

                    Ok::<_, anyhow::Error>((cfmod.data, files.data))
                });

                let mod_responses = futures::future::join_all(mod_responses)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?;

                sender.send((fingerprints, metadata, fp_response, mod_responses));
            }

            Ok::<_, anyhow::Error>(())
        };

        if let Err(e) = fut.await {
            error!({ error = ?e }, "Error occured while caching curseforge mods for {entity_id}");

            let mut failed_instances = mcm.failed_cf_instances.write().await;
            let entry = failed_instances
                .entry(entity_id)
                .or_insert((Instant::now(), 0));
            entry.0 = Instant::now() + Duration::from_secs(u64::pow(2, entry.1));
            entry.1 += 1;
        } else {
            let mut failed_instances = mcm.failed_cf_instances.write().await;
            failed_instances.remove(&entity_id);
        }

        Ok::<_, anyhow::Error>(())
    }

    async fn save_batch(
        app: &App,
        entity_id: CacheEntityId,
        (fingerprints, batch, fp_response, mod_responses): Self::SaveBundle,
    ) {
        trace!("processing curseforge mod batch for {entity_id}");

        let mut matches = fp_response
            .exact_matches
            .into_iter()
            .map(|fp_match| {
                mod_responses
                    .iter()
                    .find(|m| m.0.id == fp_match.file.mod_id)
                    .map(|m| (fp_match.file.file_fingerprint, (fp_match, m)))
            })
            .flatten()
            .collect::<HashMap<_, _>>();

        let mcm = app.meta_cache_manager();
        let mut ignored_hashes = mcm.ignored_remote_cf_hashes.write().await;
        ignored_hashes.extend(fingerprints.iter().filter(|fp| !matches.contains_key(fp)));
        drop(ignored_hashes);

        let futures = batch.into_iter().filter_map(|(metadata_id, murmur2)| {
            let fp_match = matches.get(&murmur2);
            fp_match.map(|(fp_match, modinfo)| {
                async move {
                let r = cache_curseforge_meta_unchecked(
                    app,
                    metadata_id.clone(),
                    &fp_match.file,
                    murmur2,
                    &modinfo.0,
                    &modinfo.1[..],
                )
                .await;

                if let Err(e) = r {
                    error!({ error = ?e, metadata_id, file_id = ?fp_match.file.id }, "Could not store curseforge mod metadata. Will not attempt to download again for this session.");

                    mcm.ignored_remote_cf_hashes.write().await.insert(murmur2);
                }
                }
            })
        });

        futures::future::join_all(futures).await;
    }

    async fn cache_icons(app: &App, entity_id: CacheEntityId, update_notifier: &UpdateNotifier) {
        // Collect the files whose CurseForge logo is stale (upToDate = 0) for all
        // mods needing icon updates, from the appropriate file cache table.
        let result = match entity_id {
            CacheEntityId::Instance(instance_id) => {
                let instance_id_val = *instance_id;
                mfcdb::instance_mods_stale_cf_logo(&app.db, instance_id_val)
                    .await
            }
            CacheEntityId::Server(server_id) => {
                mfcdb::server_mods_stale_cf_logo(&app.db, server_id)
                    .await
            }
        };

        let modlist: Vec<mfcdb::CfLogoRefreshRow> = match result {
            Ok(list) => list,
            Err(e) => {
                error!({ error = ?e }, "error querying database for updated curseforge mod icons list");
                return;
            }
        };

        let app = &app;
        let futures = modlist
            .into_iter()
            .map(|row| async move {
                let filename = row.filename;
                let project_id = row.project_id;
                let file_id = row.file_id;
                let mcm = app.meta_cache_manager();

                {
                    let fails = mcm.failed_cf_thumbs.read().await;
                    if let Some((time, _)) = fails.get(&project_id) {
                        if *time > std::time::Instant::now() {
                            return
                        } else {
                            mcm.failed_cf_thumbs.write().await.remove(&project_id);
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

                    let image = icon.to_vec();

                    let scale_guard = mcm
                        .image_scale_semaphore
                        .acquire()
                        .await
                        .expect("the image scale semaphore is never closed");

                    let image = carbon_scheduler::cpu_block(|| {
                        let scaled = super::scale_mod_image(&image[..])?;
                        Ok::<_, anyhow::Error>(scaled)
                    }).await?;

                    drop(scale_guard);

                    metarepo::mark_cf_image_downloaded(&app.db, &row.metadata_id, &image).await?;


                    let _ = update_notifier.send(entity_id);
                    Ok::<_, anyhow::Error>(())
                }.await;

                if let Err(e) = r {
                    error!({ error = ?e }, "error downloading mod icon for {entity_id}/{filename} (project: {project_id}, file: {file_id}, image url: {})", row.url);

                    let mut fails = mcm.failed_cf_thumbs.write().await;
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

// Cache curseforge metadata for a mod without downloading the icon
async fn cache_curseforge_meta_unchecked(
    app: &App,
    metadata_id: String,
    fileinfo: &File,
    murmur2: u32,
    modinfo: &Mod,
    mod_files: &[File],
) -> anyhow::Result<()> {
    // This is undocumented, we're guessing what the valid values here are.
    // It seems to contain both game versions and modloaders
    fn parse_update_paths(file_info: &File) -> Vec<(String, ModLoaderType, ModChannel)> {
        let mut game_versions = Vec::new();
        let mut loaders = Vec::new();

        for entry in &file_info.game_versions {
            let entry = entry.to_lowercase();
            match ModLoaderType::try_from(&entry as &str) {
                Ok(loader) => loaders.push(loader),
                Err(_) => game_versions.push(entry),
            }
        }

        let mut pairs = Vec::new();

        for game_version in game_versions {
            for loader in &loaders {
                pairs.push((
                    game_version.to_lowercase(),
                    *loader,
                    file_info.release_type.into(),
                ));
            }
        }

        pairs
    }

    let file_update_paths = parse_update_paths(fileinfo);
    let mut update_paths = Vec::<(String, ModLoaderType, ModChannel)>::new();

    let mut latest_files_sorted = mod_files.iter().collect::<Vec<_>>();
    latest_files_sorted.sort_by(|f1, f2| Ord::cmp(&f2.file_date, &f1.file_date));

    for file in latest_files_sorted {
        if file.id == fileinfo.id {
            break; // skip all older files than the one we currently have
        }

        let nf_update_paths = parse_update_paths(&file);

        for path in nf_update_paths {
            let (pv, pl, pc) = &path;

            let can_use = file_update_paths
                .iter()
                .any(|(pv2, pl2, pc2)| pv == pv2 && pl == pl2 && pc >= pc2);

            if can_use {
                if !update_paths.contains(&path) {
                    update_paths.push(path);
                }
            }
        }
    }

    let update_paths = update_paths
        .iter()
        .map(|(gamever, loader, channel)| {
            format!(
                "{gamever},{},{}",
                loader.to_string().to_lowercase(),
                channel.as_str(),
            )
        })
        .join(";");

    {
        let metadata_id = metadata_id.clone();
        if let Ok(Some(existing_entry)) = metarepo::get_cf_cache_by_metadata(&app.db, &metadata_id)
            .await
        {
            if existing_entry.cached_at > (chrono::Utc::now() - chrono::Duration::days(1)) {
                return Ok(());
            }
        }
    }

    let name = modinfo.name.clone();
    let version = fileinfo.display_name.clone();
    let urlslug = modinfo.slug.clone();
    let summary = modinfo.summary.clone();
    let authors = modinfo.authors.iter().map(|a| &a.name).join(", ");
    let release_type = ModChannel::from(fileinfo.release_type) as i32;
    let project_id = modinfo.id as i32;
    let file_id = fileinfo.id as i32;
    let murmur2 = murmur2 as i32;
    let update_paths = update_paths.clone();
    let metadata_id_owned = metadata_id.clone();

    // The composite `(projectId, fileId)` conflict may land on a row that owns
    // a different `metadataId`; the upsert returns the surviving one so the
    // image row attaches to the correct metadata.
    let result_metadata_id = metarepo::upsert_cf_mod_cache(
        &app.db,
        murmur2,
        project_id,
        file_id,
        name,
        version,
        urlslug,
        summary,
        authors,
        release_type,
        update_paths,
        DbDateTime(chrono::Utc::now().fixed_offset()),
        metadata_id_owned,
    )
    .await?;

    if let Some(logo) = &modinfo.logo {
        let url = logo.url.clone();
        let image_metadata_id = result_metadata_id.clone();
        if let Err(e) = metarepo::upsert_cf_image(&app.db, &image_metadata_id, &url)
            .await
        {
            warn!(
                "Failed to upsert curseforge image for metadata_id {}: {:?}",
                result_metadata_id, e
            );
        }
    }

    Ok(())
}
