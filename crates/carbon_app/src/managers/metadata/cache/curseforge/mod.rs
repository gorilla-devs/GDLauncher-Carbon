use super::BundleSender;
use super::ModplatformCacher;
use super::UpdateNotifier;
use crate::domain::instance::InstanceId;
use crate::domain::instance::info::ModLoaderType;
use crate::managers::App;
use carbon_platforms::ModChannel;
use carbon_platforms::curseforge::File;
use carbon_platforms::curseforge::FingerprintsMatchesResult;
use carbon_platforms::curseforge::Mod;
use carbon_platforms::curseforge::filters::ModFilesParameters;
use carbon_platforms::curseforge::filters::ModFilesParametersQuery;
use carbon_platforms::curseforge::filters::ModParameters;
use carbon_repos::queries;
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
        instance_id: InstanceId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()> {
        // Query mod files needing CurseForge update
        let pool = app.db_pool.clone();
        let id = *instance_id;
        let modlist_result = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt =
                conn.prepare(queries::metadata::ListModFilesNeedingCurseForgeUpdate::SQL)?;
            let results = stmt
                .query_map(rusqlite::params![id], |row| {
                    let metadata_id: String = row.get(0)?;
                    let murmur2: i32 = row.get(1)?;
                    Ok((metadata_id, murmur2 as u32))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_cf_hashes.read().await;

        let mut modlist = modlist_result
            .into_iter()
            .filter(|(_, murmur2)| !ignored_hashes.contains(murmur2))
            .map(|(metadata_id, murmur2)| (murmur2, (metadata_id, murmur2)))
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
        let delay = failed_instances.get(&instance_id);

        if let Some((end_time, _)) = delay {
            if Instant::now() < *end_time {
                warn!(
                    "Not attempting to cache curseforge mods for {instance_id} as too many attempts have failed recently"
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

                trace!("querying curseforge mod batch for instance {instance_id}");

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
            error!({ error = ?e }, "Error occured while caching curseforge mods for instance {instance_id}");

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
        (fingerprints, batch, fp_response, mod_responses): Self::SaveBundle,
    ) {
        trace!("processing curseforge mod batch for instance {instance_id}");

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

    async fn cache_icons(app: &App, instance_id: InstanceId, update_notifier: &UpdateNotifier) {
        // Query mod files with outdated CurseForge icons
        let pool = app.db_pool.clone();
        let id = *instance_id;
        let modlist = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt =
                conn.prepare(queries::metadata::ListModFilesWithOutdatedCurseForgeIcons::SQL)?;
            let results = stmt
                .query_map(rusqlite::params![id], |row| {
                    let filename: String = row.get(0)?;
                    let project_id: i32 = row.get(1)?;
                    let file_id: i32 = row.get(2)?;
                    let metadata_id: String = row.get(3)?;
                    let url: String = row.get(4)?;
                    Ok((filename, project_id, file_id, metadata_id, url))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, anyhow::Error>(results)
        })
        .await;

        let modlist = match modlist {
            Ok(Ok(modlist)) => modlist,
            Ok(Err(e)) => {
                error!({ error = ?e }, "error querying database for updated curseforge mod icons list");
                return;
            }
            Err(e) => {
                error!({ error = ?e }, "error spawning blocking task for curseforge mod icons list query");
                return;
            }
        };

        let app = &app;
        let futures = modlist
            .into_iter()
            .map(|(filename, project_id, file_id, metadata_id, url)| async move {
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
                        .get(&url)
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

                    // Update image cache
                    let pool = app.db_pool.clone();
                    let metadata_id_clone = metadata_id.clone();
                    tokio::task::spawn_blocking(move || {
                        let conn = pool.get()?;
                        queries::metadata::UpdateCurseForgeModImageCacheData::execute(
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
                    error!({ error = ?e }, "error downloading mod icon for {instance_id}/{filename} (project: {project_id}, file: {file_id}, image url: {})", url);

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

    // Check if entry exists and is recent
    let pool = app.db_pool.clone();
    let metadata_id_clone = metadata_id.clone();
    let existing_entry = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let result =
            queries::metadata::FindCurseForgeModCache::fetch_optional(&conn, &metadata_id_clone)?;
        Ok::<_, anyhow::Error>(result)
    })
    .await??;

    if let Some(cache) = existing_entry {
        // Check if the cached entry is recent (within 1 day)
        if cache.cached_at > (chrono::Utc::now() - chrono::Duration::days(1)) {
            return Ok(());
        }
    }

    // Upsert CurseForge mod cache
    let pool = app.db_pool.clone();
    let metadata_id_clone = metadata_id.clone();
    let murmur2_i32 = murmur2 as i32;
    let project_id = modinfo.id as i32;
    let file_id = fileinfo.id as i32;
    let name = modinfo.name.clone();
    let version = fileinfo.display_name.clone();
    let urlslug = modinfo.slug.clone();
    let summary = modinfo.summary.clone();
    let authors = modinfo.authors.iter().map(|a| &a.name).join(", ");
    let release_type = ModChannel::from(fileinfo.release_type) as i32;
    let update_paths_clone = update_paths.clone();
    let cached_at = chrono::Utc::now().to_rfc3339();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::metadata::UpsertCurseForgeModCache::execute(
            &conn,
            &metadata_id_clone,
            murmur2_i32,
            project_id,
            file_id,
            &name,
            &version,
            &urlslug,
            &summary,
            &authors,
            release_type,
            &update_paths_clone,
            &cached_at,
        )?;
        Ok::<_, anyhow::Error>(())
    })
    .await??;

    // Handle logo image cache
    if let Some(logo) = &modinfo.logo {
        let pool = app.db_pool.clone();
        let metadata_id_clone = metadata_id.clone();
        let logo_url = logo.url.clone();

        if let Err(e) = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::metadata::UpsertCurseForgeModImageCache::execute(
                &conn,
                &metadata_id_clone,
                &logo_url,
                None, // upToDate = 0, mark as needing download
                0,
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .await?
        {
            warn!(
                "Failed to upsert curseforge image for metadata_id {}: {:?}",
                metadata_id, e
            );
        }
    }

    Ok(())
}
