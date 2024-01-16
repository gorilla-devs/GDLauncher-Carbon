use itertools::Itertools;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;
use tracing::warn;

use tracing::debug;
use tracing::error;
use tracing::trace;

use crate::db::read_filters::DateTimeFilter;
use crate::db::read_filters::IntFilter;
use crate::domain::instance::info::ModLoaderType;
use crate::domain::instance::InstanceId;
use crate::domain::modplatforms::curseforge::filters::ModsParameters;
use crate::domain::modplatforms::curseforge::filters::ModsParametersBody;
use crate::domain::modplatforms::curseforge::File;
use crate::domain::modplatforms::curseforge::FileReleaseType;
use crate::domain::modplatforms::curseforge::FingerprintsMatchesResult;
use crate::domain::modplatforms::curseforge::Mod;
use crate::domain::modplatforms::ModChannel;
use crate::managers::App;

use super::BundleSender;
use super::ModplatformCacher;
use super::UpdateNotifier;
use crate::db::{
    curse_forge_mod_cache as cfdb, curse_forge_mod_image_cache as cfimgdb, mod_file_cache as fcdb,
    mod_metadata as metadb,
};

pub mod modpack;

pub struct CurseforgeModCacher;

#[async_trait::async_trait]
impl ModplatformCacher for CurseforgeModCacher {
    const NAME: &'static str = "curseforge";

    type SaveBundle = (
        Vec<u32>,
        Vec<(String, u32)>,
        FingerprintsMatchesResult,
        Vec<Mod>,
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
                fcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::CurseforgeIsNot(vec![
                    cfdb::WhereParam::CachedAt(DateTimeFilter::Gt(
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

                (
                    metadata.murmur_2 as u32,
                    (metadata.id, metadata.murmur_2 as u32),
                )
            });

        let mcm = app.meta_cache_manager();
        let ignored_hashes = mcm.ignored_remote_cf_hashes.read().await;

        let mut modlist = modlist
            .filter(|(_, (_, murmur2))| !ignored_hashes.contains(murmur2))
            .collect::<VecDeque<_>>();

        drop(ignored_hashes);

        if modlist.is_empty() {
            return Ok(());
        }

        let failed_instances = mcm.failed_cf_instances.read().await;
        let delay = failed_instances.get(&instance_id);

        if let Some((end_time, _)) = delay {
            if Instant::now() < *end_time {
                warn!("Not attempting to cache curseforge mods for {instance_id} as too many attempts have failed recently");
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

                let mods_response = app
                    .modplatforms_manager()
                    .curseforge
                    .get_mods(ModsParameters {
                        body: ModsParametersBody {
                            mod_ids: fp_response
                                .exact_matches
                                .iter()
                                .map(|m| m.file.mod_id)
                                .collect::<Vec<_>>(),
                        },
                    })
                    .await?
                    .data;

                sender.send((fingerprints, metadata, fp_response, mods_response));
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
        (fingerprints, batch, fp_response, mods_response): Self::SaveBundle,
    ) {
        trace!("processing curseforge mod batch for instance {instance_id}");

        let mut matches = fp_response
            .exact_matches
            .into_iter()
            .map(|fp_match| {
                mods_response
                    .iter()
                    .find(|m| m.id == fp_match.file.mod_id)
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
            fp_match.map(|(fp_match, modinfo)| async move {
                let r = cache_curseforge_meta_unchecked(
                    app,
                    metadata_id.clone(),
                    &fp_match.file,
                    murmur2,
                    &modinfo,
                )
                .await;

                if let Err(e) = r {
                    error!({ error = ?e, metadata_id, file_id = ?fp_match.file.id }, "Could not store curseforge mod metadata. Will not attempt to download again for this session.");

                    mcm.ignored_remote_cf_hashes.write().await.insert(murmur2);
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
                fcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::CurseforgeIs(vec![
                    cfdb::WhereParam::LogoImageIs(vec![cfimgdb::WhereParam::UpToDate(
                        IntFilter::Equals(0),
                    )]),
                ])]),
            ])
            .with(
                fcdb::metadata::fetch()
                    .with(metadb::curseforge::fetch().with(cfdb::logo_image::fetch())),
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
            let cf = meta
                .curseforge
                .flatten()
                .expect("curseforge was ensured present but not returned");
            let row = cf
                .logo_image
                .flatten()
                .expect("mod image was ensured present but not returned");

            (file.filename, cf.project_id, cf.file_id, row)
        });

        let app = &app;
        let futures = modlist
            .map(|(filename, project_id, file_id, row)| async move {
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

                    debug!("thumbnailing curseforge mod icon for {instance_id}/{filename} (project: {project_id}, file: {file_id})");

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

                    app.prisma_client.curse_forge_mod_image_cache()
                        .update(
                            cfimgdb::UniqueWhereParam::MetadataIdEquals(row.metadata_id.clone()),
                            vec![
                                cfimgdb::SetParam::SetUpToDate(1),
                                cfimgdb::SetParam::SetData(Some(image))
                            ]
                        )
                        .exec()
                        .await?;

                    debug!("saved curseforge mod thumbnail for {instance_id}/{filename} (project: {project_id}, file: {file_id})");

                    let _ = update_notifier.send(instance_id);
                    Ok::<_, anyhow::Error>(())
                }.await;

                if let Err(e) = r {
                    error!({ error = ?e }, "error downloading mod icon for {instance_id}/{filename} (project: {project_id}, file: {file_id}, image url: {})", row.url);

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
) -> anyhow::Result<()> {
    let prev = app
        .prisma_client
        .curse_forge_mod_cache()
        .find_unique(cfdb::UniqueWhereParam::MetadataIdEquals(
            metadata_id.clone(),
        ))
        .with(cfdb::logo_image::fetch())
        .exec()
        .await?;

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

    let file_update_paths = parse_update_paths(&fileinfo);
    let mut update_paths = Vec::<(String, ModLoaderType, ModChannel)>::new();

    let mut latest_files_sorted = modinfo.latest_files.iter().collect::<Vec<_>>();
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

    let o_insert_cfmeta = app.prisma_client.curse_forge_mod_cache().create(
        murmur2 as i32,
        modinfo.id,
        fileinfo.id,
        modinfo.name.clone(),
        modinfo.slug.clone(),
        modinfo.summary.clone(),
        modinfo.authors.iter().map(|a| &a.name).join(", "),
        ModChannel::from(fileinfo.release_type) as i32,
        update_paths,
        chrono::Utc::now().into(),
        metadb::UniqueWhereParam::IdEquals(metadata_id.clone()),
        Vec::new(),
    );

    let o_delete_cfmeta =
        prev.as_ref().map(|_| {
            app.prisma_client.curse_forge_mod_cache().delete(
                cfdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
            )
        });

    let old_image = prev
        .map(|p| {
            p.logo_image
                .expect("logo_image was requested but not returned by prisma")
        })
        .flatten();
    let new_image = modinfo.logo.as_ref().map(|it| &it.url).cloned();

    let image = match (new_image, old_image) {
        (Some(new), Some(old)) => Some((old.up_to_date == 1 && new == old.url, new, old.data)),
        (Some(new), None) => Some((false, new, None)),
        (None, Some(old)) => Some((old.up_to_date == 1, old.url, old.data)),
        (None, None) => None,
    };

    let o_insert_logo = image.map(|(up_to_date, url, data)| {
        app.prisma_client.curse_forge_mod_image_cache().create(
            url,
            cfdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
            vec![
                cfimgdb::SetParam::SetUpToDate(if up_to_date { 1 } else { 0 }),
                cfimgdb::SetParam::SetData(data),
            ],
        )
    });

    debug!("updating curseforge metadata entry for {metadata_id}");

    app.prisma_client
        ._batch((
            o_delete_cfmeta.into_iter().collect::<Vec<_>>(),
            o_insert_cfmeta,
            o_insert_logo.into_iter().collect::<Vec<_>>(),
        ))
        .await?;

    Ok(())
}
