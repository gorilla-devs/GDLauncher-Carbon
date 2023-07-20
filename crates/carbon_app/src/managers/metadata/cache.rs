use std::collections::HashMap;
use std::collections::HashSet;

use std::collections::VecDeque;
use std::io::Cursor;
use std::path::PathBuf;
use std::usize;

use md5::Digest;

use sha2::Sha512;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use uuid::Uuid;

use crate::api::keys::instance::INSTANCE_MODS;
use crate::db::read_filters::BytesFilter;
use crate::db::read_filters::DateTimeFilter;
use crate::db::read_filters::IntFilter;

use crate::db::read_filters::StringFilter;
use crate::domain::instance::InstanceId;
use crate::domain::modplatforms::curseforge::filters::ModsParameters;
use crate::domain::modplatforms::curseforge::filters::ModsParametersBody;
use crate::domain::modplatforms::curseforge::FingerprintsMatchesResult;
use crate::domain::modplatforms::curseforge::Mod;
use crate::domain::runtime_path::InstancesPath;

use crate::managers::ManagerRef;
use crate::once_send::OnceSend;

use crate::db::{curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb};
use itertools::Itertools;

pub struct MetaCacheManager {
    waiting_instances: RwLock<HashSet<InstanceId>>,
    scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_hashes: RwLock<HashSet<u32>>,
    priority_instance: Mutex<Option<InstanceId>>,
    remote_instance: watch::Sender<Option<InstanceId>>,
    waiting_notify: watch::Sender<()>,
    // local cache notify, remote cache notify
    background_watches: OnceSend<(watch::Receiver<()>, watch::Receiver<Option<InstanceId>>)>,
}

impl MetaCacheManager {
    pub fn new() -> Self {
        let (local_tx, local_rx) = watch::channel(());
        let (remote_tx, remote_rx) = watch::channel(None);

        Self {
            waiting_instances: RwLock::new(HashSet::new()),
            scanned_instances: Mutex::new(HashSet::new()),
            ignored_remote_hashes: RwLock::new(HashSet::new()),
            priority_instance: Mutex::new(None),
            remote_instance: remote_tx,
            waiting_notify: local_tx,
            background_watches: OnceSend::new((local_rx, remote_rx)),
        }
    }
}

impl ManagerRef<'_, MetaCacheManager> {
    pub async fn instance_removed(self, instance_id: InstanceId) {
        let mut priority = self.priority_instance.lock().await;
        if let Some(priority_v) = &*priority {
            if *priority_v == instance_id {
                *priority = None;
            }
        }
        drop(priority);

        let _ = self
            .app
            .prisma_client
            .mod_file_cache()
            .delete_many(vec![fcdb::WhereParam::InstanceId(IntFilter::Equals(
                *instance_id,
            ))])
            .exec()
            .await;

        self.gc_mod_metadata().await;
    }

    pub async fn gc_mod_metadata(self) {
        let _ = self
            .app
            .prisma_client
            .mod_metadata()
            .delete_many(vec![metadb::WhereParam::CachedFilesNone(Vec::new())])
            .exec()
            .await;
    }

    pub async fn queue_local_caching(self, instance_id: InstanceId, force_recache: bool) {
        trace!("possibly queueing instance {instance_id} for recaching");

        let mut lock = self.scanned_instances.lock().await;

        if force_recache || !lock.contains(&instance_id) {
            self.waiting_instances.write().await.insert(instance_id);
            let _ = self.waiting_notify.send(());

            // prevent future calls
            lock.insert(instance_id);

            info!("queued instance {instance_id} for recaching");
        }
    }

    pub async fn prioritize_instance(self, instance_id: InstanceId) {
        *self.priority_instance.lock().await = Some(instance_id);
    }

    pub async fn focus_instance(self, instance_id: InstanceId) {
        let mut lock = self.scanned_instances.lock().await;
        if lock.contains(&instance_id) {
            info!("queueing remote metadata download for instance {instance_id}");

            let _ = self
                .app
                .meta_cache_manager()
                .remote_instance
                .send(Some(instance_id));
        } else {
            self.prioritize_instance(instance_id).await;

            self.waiting_instances.write().await.insert(instance_id);
            let _ = self.waiting_notify.send(());
            lock.insert(instance_id);
            info!("queued instance {instance_id} for recaching");
        }
    }

    /// Panics if called more than once
    pub async fn launch_background_tasks(self) {
        let (mut local_notify, mut remote_watch) = self
            .background_watches
            .take()
            .expect("launch_background_tasks may only be called once");

        let app_local = self.app.clone();
        let app_cf = self.app.clone();

        tokio::spawn(async move {
            let app = app_local;
            let instance_manager = app.instance_manager();
            let basepath = app.settings_manager().runtime_path.get_root().to_path();
            let mut pathbuf = PathBuf::new();

            while local_notify.changed().await.is_ok() {
                trace!("mod caching task woken");
                loop {
                    let (priority, instance_id) = 'pi: {
                        let priority_instance = app
                            .meta_cache_manager()
                            .priority_instance
                            .lock()
                            .await
                            .clone();

                        let mcm = app.meta_cache_manager();
                        let mut waiting = mcm.waiting_instances.write().await;

                        if let Some(pi) = priority_instance
                            .map(|instance| waiting.take(&instance))
                            .flatten()
                        {
                            break 'pi (true, Some(pi));
                        }

                        let next = waiting.iter().next().cloned();
                        (false, next.map(|n| waiting.take(&n)).flatten())
                    };

                    let Some(instance_id) = instance_id else { break };
                    info!(
                        { priority },
                        "recaching instance mod metadata for {instance_id}"
                    );

                    let instances = instance_manager.instances.read().await;
                    let Some(instance) = instances.get(&instance_id) else { continue };

                    let cache_app = app.clone();
                    let cached_entries = tokio::spawn(async move {
                        cache_app
                            .prisma_client
                            .mod_file_cache()
                            .find_many(vec![fcdb::WhereParam::InstanceId(IntFilter::Equals(
                                *instance_id as i32,
                            ))])
                            .exec()
                            .await
                    });

                    let subpath = InstancesPath::subpath()
                        .get_instance_path(&instance.shortpath)
                        .get_mods_path();

                    pathbuf.clear();
                    pathbuf.push(&basepath);
                    pathbuf.push(&subpath);

                    trace!({ dir = ?pathbuf }, "scanning mods dir for instance {instance_id}");
                    let mut modpaths = HashMap::<String, (bool, u64)>::new();
                    let mut entries = match tokio::fs::read_dir(&pathbuf).await {
                        Ok(entries) => entries,
                        Err(e) => {
                            error!({ dir = ?pathbuf, error = ?e }, "could not read instance {instance_id}  for mod scanning");
                            continue;
                        }
                    };

                    while let Ok(Some(entry)) = entries.next_entry().await {
                        trace!("scanning mods folder entry `{:?}`", entry.file_name());
                        let file_name = entry.file_name();
                        let Some(mut utf8_name) = file_name.to_str() else { continue };

                        let is_jar = utf8_name.ends_with(".jar");
                        let is_jar_disabled = utf8_name.ends_with(".jar.disabled");

                        if !is_jar && !is_jar_disabled {
                            continue;
                        }

                        if is_jar_disabled {
                            utf8_name = utf8_name.strip_suffix(".disabled").unwrap();
                        }

                        let Ok(metadata) = entry.metadata().await else { continue };
                        // file || symlink
                        if metadata.is_dir() {
                            continue;
                        }

                        trace!("tracking mod `{utf8_name}` for instance {instance_id}");
                        modpaths.insert(utf8_name.to_string(), (!is_jar_disabled, metadata.len()));
                    }

                    let mut dirty_cache = Vec::<fcdb::UniqueWhereParam>::new();

                    if let Ok(Ok(cached_entries)) = cached_entries.await {
                        for entry in cached_entries {
                            if let Some((enabled, real_size)) = modpaths.get(&entry.filename) {
                                // enabled probably shouldn't be here
                                if *real_size == entry.filesize as u64 && *enabled == entry.enabled
                                {
                                    modpaths.remove(&entry.filename);
                                    trace!(
                                        "up to data metadata entry for mod `{}`, skipping",
                                        &entry.filename
                                    );
                                    continue;
                                }
                            }

                            trace!(
                                "outdated metadata entry for mod `{}`, adding to update list",
                                &entry.filename
                            );

                            dirty_cache.push(fcdb::UniqueWhereParam::InstanceIdFilenameEquals(
                                *instance_id,
                                entry.filename,
                            ));
                        }
                    }

                    let entry_futures =
                        modpaths.into_iter().map(|(subpath, (enabled, filesize))| {
                            let pathbuf = &pathbuf;
                            let db = &app.prisma_client;
                            async move {
                                let mut path = pathbuf.join(&subpath);
                                if !enabled {
                                    path.set_extension("jar.disabled");
                                }

                                let mut content = tokio::fs::read(path).await?;
                                let (sha512, meta, murmur2) =
                                    tokio::task::spawn_blocking(move || {
                                        (
                                            <[u8; 64] as From<_>>::from(
                                                Sha512::new_with_prefix(&content).finalize(),
                                            ),
                                            super::mods::parse_metadata(Cursor::new(&content)),
                                            murmurhash32::murmurhash2({
                                                // curseforge's weird api
                                                content.retain(|&x| {
                                                    x != 9 && x != 10 && x != 13 && x != 32
                                                });
                                                &content
                                            }),
                                        )
                                    })
                                    .await?;

                                let meta = meta?;

                                let dbmeta = db
                                    .mod_metadata()
                                    // just check both hashes for now
                                    .find_first(vec![
                                        metadb::WhereParam::Sha512(BytesFilter::Equals(Vec::from(
                                            sha512,
                                        ))),
                                        metadb::WhereParam::Murmur2(IntFilter::Equals(
                                            murmur2 as i32,
                                        )),
                                    ])
                                    .exec()
                                    .await?;

                                let (meta_id, meta) = match dbmeta {
                                    Some(dbmeta) => (dbmeta.id, None),
                                    None => (Uuid::new_v4().to_string(), Some(meta)),
                                };

                                Ok::<_, anyhow::Error>(Some((
                                    subpath, filesize, enabled, sha512, murmur2, meta_id, meta,
                                )))
                            }
                        });

                    let (new_fc_entries, new_meta_entries): (_, Vec<_>) =
                        futures::future::join_all(entry_futures)
                            .await
                            .into_iter()
                            .map(|m| m.unwrap_or(None))
                            .filter_map(|m| m)
                            .map(
                                |(subpath, filesize, enabled, sha512, murmur2, meta_id, meta)| {
                                    (
                                        (
                                            *instance_id as i32,
                                            subpath,
                                            filesize as i32,
                                            enabled,
                                            meta_id.clone(),
                                            Vec::new(),
                                        ),
                                        meta.map(|meta| {
                                            (
                                                meta_id,
                                                murmur2 as i32,
                                                Vec::from(sha512),
                                                meta.as_ref()
                                                    .map(|meta| &meta.modloaders)
                                                    .map(|vec| {
                                                        vec.into_iter()
                                                            .map(|m| m.to_string())
                                                            .join(",")
                                                    })
                                                    .unwrap_or(String::new()),
                                                match meta {
                                                    Some(meta) => vec![
                                                        metadb::SetParam::SetName(meta.name),
                                                        metadb::SetParam::SetModid(Some(
                                                            meta.modid,
                                                        )),
                                                        metadb::SetParam::SetVersion(meta.version),
                                                        metadb::SetParam::SetDescription(
                                                            meta.description,
                                                        ),
                                                        metadb::SetParam::SetAuthors(meta.authors),
                                                    ],

                                                    // Prisma sucks and is generating invalid sql.
                                                    // As a workaround, all the defaults are explicitly set manually.
                                                    None => vec![
                                                        metadb::SetParam::SetName(None),
                                                        metadb::SetParam::SetModid(None),
                                                        metadb::SetParam::SetVersion(None),
                                                        metadb::SetParam::SetDescription(None),
                                                        metadb::SetParam::SetAuthors(None),
                                                    ],
                                                },
                                            )
                                        }),
                                    )
                                },
                            )
                            .unzip();

                    let r = app
                        .prisma_client
                        ._batch((
                            dirty_cache
                                .into_iter()
                                .map(|id| app.prisma_client.mod_file_cache().delete(id))
                                .collect::<Vec<_>>(),
                            app.prisma_client.mod_metadata().create_many(
                                new_meta_entries.into_iter().filter_map(|e| e).collect(),
                            ),
                            app.prisma_client
                                .mod_file_cache()
                                .create_many(new_fc_entries),
                        ))
                        .await;

                    if let Err(e) = r {
                        error!({ error = ?e }, "could not store mod scan results for instance {instance_id} in db");
                    }

                    app.invalidate(INSTANCE_MODS, Some(instance_id.0.into()));

                    if priority {
                        info!("queueing remote metadata download for instance {instance_id}");

                        let _ = app
                            .meta_cache_manager()
                            .remote_instance
                            .send(Some(instance_id));
                    } else {
                        debug!(
                            "not queueing remote metadata for non priority instance {instance_id}"
                        );
                    }
                }
            }
        });

        tokio::spawn(async move {
            let app = app_cf;

            while remote_watch.changed().await.is_ok() {
                loop {
                    debug!("remote watch target updated");
                    let Some(instance_id) = *remote_watch.borrow() else { break };
                    info!("updating curseforge metadata cache for instance {instance_id}");

                    let fut = async {
                        let modlist = app
                            .prisma_client
                            .mod_file_cache()
                            .find_many(vec![
                                fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                                fcdb::WhereParam::MetadataIs(vec![
                                    metadb::WhereParam::CurseforgeIsNot(vec![
                                        cfdb::WhereParam::CachedAt(DateTimeFilter::Gt(
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

                                (
                                    metadata.murmur_2 as u32,
                                    (metadata.id, metadata.murmur_2 as u32),
                                )
                            });

                        let mcm = app.meta_cache_manager();
                        let ignored_hashes = mcm.ignored_remote_hashes.read().await;

                        let mut modlist = modlist
                            .filter(|(_, (_, murmur2))| !ignored_hashes.contains(murmur2))
                            .collect::<VecDeque<_>>();

                        drop(ignored_hashes);

                        let (batch_tx, mut batch_rx) = mpsc::unbounded_channel::<(
                            Vec<u32>,
                            Vec<(String, u32)>,
                            FingerprintsMatchesResult,
                            Vec<Mod>,
                        )>();

                        let app_db = app.clone();
                        tokio::spawn(async move {
                            while let Some((fingerprints, batch, fp_response, mods_response)) =
                                batch_rx.recv().await
                            {
                                trace!("processing mod batch for instance {instance_id}");

                                let mut matches = fp_response
                                    .exact_fingerprints
                                    .into_iter()
                                    .zip(fp_response.exact_matches.into_iter())
                                    .zip(mods_response.into_iter())
                                    .map(|((fingerprint, fileinfo), modinfo)| {
                                        (fingerprint, (fileinfo, modinfo))
                                    })
                                    .collect::<HashMap<_, _>>();

                                let mcm = app_db.meta_cache_manager();
                                let mut ignored_hashes = mcm.ignored_remote_hashes.write().await;
                                ignored_hashes.extend(
                                    fingerprints.iter().filter(|fp| !matches.contains_key(fp)),
                                );
                                drop(ignored_hashes);

                                let (deletes, insert_data) = batch
                                    .into_iter()
                                    .filter_map(|(metadata_id, murmur2)| {
                                        let fpmatch = matches.remove(&murmur2);
                                        fpmatch.map(|(fileinfo, modinfo)| {
                                            (
                                                app_db
                                                    .prisma_client
                                                    .curse_forge_mod_cache()
                                                    .delete_many(vec![
                                                        cfdb::WhereParam::MetadataId(
                                                            StringFilter::Equals(
                                                                metadata_id.clone(),
                                                            ),
                                                        ),
                                                    ]),
                                                (
                                                    murmur2 as i32,
                                                    modinfo.id,
                                                    fileinfo.file.id,
                                                    modinfo.name,
                                                    modinfo.slug,
                                                    modinfo.summary,
                                                    modinfo
                                                        .authors
                                                        .into_iter()
                                                        .map(|a| a.name)
                                                        .join(", "),
                                                    chrono::Utc::now().into(),
                                                    metadata_id.clone(),
                                                    Vec::new(),
                                                ),
                                            )
                                        })
                                    })
                                    .unzip::<_, _, Vec<_>, Vec<_>>();

                                trace!("saving mod metadata batch of size {}", deletes.len());
                                // may fail if the user removed a mod
                                let r = app_db
                                    .prisma_client
                                    ._batch((
                                        deletes,
                                        app_db
                                            .prisma_client
                                            .curse_forge_mod_cache()
                                            .create_many(insert_data),
                                    ))
                                    .await;

                                if let Err(e) = r {
                                    tracing::error!({ error = ?e }, "Could not store mod metadata");
                                }

                                app_db.invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
                            }
                        });

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

                            batch_tx
                                .send((fingerprints, metadata, fp_response, mods_response))
                                .expect(
                                "batch processor should not drop until the transmitter is dropped",
                            );
                        }

                        Ok::<_, anyhow::Error>(())
                    };

                    // TODO: if we changed to the same instance, just add new mods instead of
                    // forcing the whole thing to rerun
                    tokio::select! {
                        _ = remote_watch.changed() => continue,
                        r = fut => {
                            if let Err(e) = r {
                                error!({ error = ?e }, "failed to query curseforge for instance {instance_id} mods");
                            }
                            break
                        },
                    };
                }
            }
        });
    }
}
