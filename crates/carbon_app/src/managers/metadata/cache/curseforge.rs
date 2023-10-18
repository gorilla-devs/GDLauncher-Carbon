use itertools::Itertools;
use std::collections::HashMap;
use std::collections::VecDeque;

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
use super::UpdateNotifier;
use crate::db::{
    curse_forge_mod_cache as cfdb, curse_forge_mod_image_cache as cfimgdb, mod_file_cache as fcdb,
    mod_metadata as metadb,
};

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
                let r = cache_curseforge_meta_unchecked(
                    app,
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
    file_id: i32,
    murmur2: u32,
    modinfo: Mod,
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

    let mut o_delete_cfmeta = None;
    let mut o_insert_logo = None;
    let mut o_update_logo = None;
    let mut o_delete_logo = None;

    let o_insert_cfmeta = app.prisma_client.curse_forge_mod_cache().create(
        murmur2 as i32,
        modinfo.id,
        file_id,
        modinfo.name,
        modinfo.slug,
        modinfo.summary,
        modinfo.authors.into_iter().map(|a| a.name).join(", "),
        chrono::Utc::now().into(),
        metadb::UniqueWhereParam::IdEquals(metadata_id.clone()),
        Vec::new(),
    );

    if let Some(prev) = prev {
        o_delete_cfmeta = Some(app.prisma_client.curse_forge_mod_cache().delete(
            cfdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
        ));

        if let Some(prev) = prev
            .logo_image
            .expect("logo_image was requesred but not returned by prisma")
        {
            match modinfo.logo.as_ref().map(|it| &it.url) {
                Some(url) => {
                    if *url != prev.url {
                        o_update_logo =
                            Some(app.prisma_client.curse_forge_mod_image_cache().update(
                                cfimgdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                                vec![
                                    cfimgdb::SetParam::SetUrl(url.clone()),
                                    cfimgdb::SetParam::SetUpToDate(0),
                                ],
                            ));
                    }
                }
                None => {
                    o_delete_logo = Some(app.prisma_client.curse_forge_mod_image_cache().delete(
                        cfimgdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                    ));
                }
            }
        }
    }

    if o_update_logo.is_none() && o_delete_logo.is_none() {
        if let Some(url) = modinfo.logo.map(|it| it.url) {
            o_insert_logo = Some(app.prisma_client.curse_forge_mod_image_cache().create(
                url,
                cfdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                Vec::new(),
            ));
        }
    }

    debug!("updating curseforge metadata entry for {metadata_id}");

    app.prisma_client
        ._batch((
            o_delete_cfmeta.into_iter().collect::<Vec<_>>(),
            o_insert_cfmeta,
            o_delete_logo.into_iter().collect::<Vec<_>>(),
            o_insert_logo.into_iter().collect::<Vec<_>>(),
            o_update_logo.into_iter().collect::<Vec<_>>(),
        ))
        .await?;

    Ok(())
}
