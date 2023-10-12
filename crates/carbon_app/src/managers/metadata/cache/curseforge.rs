use std::collections::HashMap;
use std::collections::VecDeque;

use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::trace;

use crate::db::read_filters::DateTimeFilter;
use crate::db::read_filters::IntFilter;
use crate::domain::instance::InstanceId;
use crate::domain::modplatforms::curseforge::filters::ModsParameters;
use crate::domain::modplatforms::curseforge::filters::ModsParametersBody;
use crate::domain::modplatforms::curseforge::FingerprintsMatchesResult;
use crate::domain::modplatforms::curseforge::Mod;
use crate::managers::App;

use super::BundleSender;
use super::ModplatformCacher;
use crate::db::{curse_forge_mod_cache as cfdb, curse_forge_mod_image_cache as cfimgdb, mod_file_cache as fcdb, mod_metadata as metadb};

struct CurseforgeModCacher;

#[async_trait::async_trait]
impl ModplatformCacher for CurseforgeModCacher {
    type SaveBundle = (
        Vec<u32>,
        Vec<(String, u32)>,
        FingerprintsMatchesResult,
        Vec<Mod>,
    );

    async fn query_platform(
        app: &App,
        instance_id: InstanceId,
        sender: &BundleSender<Self::SaveBundle>,
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

            sender
                .send((fingerprints, metadata, fp_response, mods_response))
                .expect("batch processor should not drop until the transmitter is dropped");
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
            .exact_fingerprints
            .into_iter()
            .zip(fp_response.exact_matches.into_iter())
            .zip(mods_response.into_iter())
            .map(|((fingerprint, fileinfo), modinfo)| (fingerprint, (fileinfo, modinfo)))
            .collect::<HashMap<_, _>>();

        let mcm = app.meta_cache_manager();
        let mut ignored_hashes = mcm.ignored_remote_cf_hashes.write().await;
        ignored_hashes.extend(fingerprints.iter().filter(|fp| !matches.contains_key(fp)));
        drop(ignored_hashes);

        let futures = batch.into_iter().filter_map(|(metadata_id, murmur2)| {
            let fpmatch = matches.remove(&murmur2);
            fpmatch.map(|(fileinfo, modinfo)| async move {
                let r = app
                    .meta_cache_manager()
                    .cache_curseforge_meta_unchecked(
                        metadata_id,
                        fileinfo.file.id,
                        murmur2,
                        modinfo,
                    )
                    .await;

                if let Err(e) = r {
                    error!({ error = ?e }, "Could not store curseforge mod metadata");
                }
            })
        });

        futures::future::join_all(futures).await;
    }

    async fn cache_icons(app: &App, instance_id: InstanceId, update_notifier: &mpsc::UnboundedSender<InstanceId>) {
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

            (
                file.filename,
                cf.project_id,
                cf.file_id,
                row,
            )
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

    fn print_error(instance_id: InstanceId, error: &anyhow::Error) {
        error!({ ?error }, "Could not query curseforge mod metadata for instance {instance_id}");
    }
}
