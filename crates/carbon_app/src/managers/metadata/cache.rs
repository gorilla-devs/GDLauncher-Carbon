use std::collections::HashMap;
use std::collections::HashSet;

use std::collections::VecDeque;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::path::PathBuf;
use std::usize;

use image::ImageOutputFormat;
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

use crate::domain::modplatforms::modrinth::responses::ProjectsResponse;
use crate::domain::modplatforms::modrinth::responses::TeamResponse;
use crate::domain::modplatforms::modrinth::responses::VersionHashesResponse;
use crate::domain::modplatforms::modrinth::search::ProjectIDs;
use crate::domain::modplatforms::modrinth::search::TeamIDs;
use crate::domain::modplatforms::modrinth::search::VersionHashesQuery;
use crate::domain::modplatforms::modrinth::version::HashAlgorithm;
use crate::domain::runtime_path::InstancesPath;

use crate::managers::ManagerRef;
use crate::once_send::OnceSend;

use crate::db::{
    curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb,
    modrinth_mod_cache as mrdb,
};
use itertools::Itertools;

pub struct MetaCacheManager {
    waiting_instances: RwLock<HashSet<InstanceId>>,
    scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_cf_hashes: RwLock<HashSet<u32>>,
    ignored_remote_mr_hashes: RwLock<HashSet<String>>,
    priority_instance: Mutex<Option<InstanceId>>,
    remote_instance: watch::Sender<Option<InstanceId>>,
    waiting_notify: watch::Sender<()>,
    // local cache notify, remote cache notify
    background_watches: OnceSend<(watch::Receiver<()>, watch::Receiver<Option<InstanceId>>)>,
}

impl Default for MetaCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaCacheManager {
    pub fn new() -> Self {
        let (local_tx, local_rx) = watch::channel(());
        let (remote_tx, remote_rx) = watch::channel(None);

        Self {
            waiting_instances: RwLock::new(HashSet::new()),
            scanned_instances: Mutex::new(HashSet::new()),
            ignored_remote_cf_hashes: RwLock::new(HashSet::new()),
            ignored_remote_mr_hashes: RwLock::new(HashSet::new()),
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
        let (mut local_notify, remote_watch) = self
            .background_watches
            .take()
            .expect("launch_background_tasks may only be called once");

        let app_local = self.app.clone();
        let app_cf = self.app.clone();
        let app_mr = self.app.clone();
        let cf_remote_watch = remote_watch.clone();
        let mr_remote_watch = remote_watch;

        tokio::spawn(async move {
            let app = app_local;
            let instance_manager = app.instance_manager();
            let basepath = app.settings_manager().runtime_path.get_root().to_path();
            let mut pathbuf = PathBuf::new();

            while local_notify.changed().await.is_ok() {
                trace!("mod caching task woken");
                loop {
                    let (priority, instance_id) = 'pi: {
                        let priority_instance =
                            *app.meta_cache_manager().priority_instance.lock().await;

                        let mcm = app.meta_cache_manager();
                        let mut waiting = mcm.waiting_instances.write().await;

                        if let Some(pi) =
                            priority_instance.and_then(|instance| waiting.take(&instance))
                        {
                            break 'pi (true, Some(pi));
                        }

                        let next = waiting.iter().next().cloned();
                        (false, next.and_then(|n| waiting.take(&n)))
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
                                *instance_id,
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

                                let content = tokio::fs::read(path).await?;
                                let (content, sha512, meta, murmur2) =
                                    tokio::task::spawn_blocking(move || {
                                        let sha512: [u8; 64] = Sha512::new_with_prefix(&content).finalize().into();
                                        let meta = super::mods::parse_metadata(Cursor::new(&content));

                                        // curseforge's api removes whitespace in murmur2 hashes
                                        let mut murmur_content = content.clone();
                                        murmur_content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
                                        let murmur2 = murmurhash32::murmurhash2(&murmur_content);

                                        (
                                            content,
                                            sha512,
                                            meta,
                                            murmur2,
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


                                let (meta_id, meta, logo_data) = match dbmeta {
                                    Some(dbmeta) => (dbmeta.id, None, None),
                                    None => {
                                        let logo_data = 'logo: {
                                            let Some(meta) = &meta else { break 'logo None };

                                            let logo_data = (|| {
                                                let Some(logo_file) = &meta.logo_file else { return Ok(None) };
                                                let mut zip = zip::ZipArchive::new(Cursor::new(&content)).unwrap();
                                                let Ok(mut file) = zip.by_name(logo_file) else { return Ok(None) };
                                                let mut image = Vec::with_capacity(file.size() as usize);
                                                file.read_to_end(&mut image)?;
                                                let scaled = scale_mod_image(&image[..])?;
                                                Ok::<_, anyhow::Error>(Some(scaled))
                                            })();

                                            match logo_data {
                                                Ok(data) => data,
                                                Err(e) => {
                                                    error!({ error = ?e }, "could not scale mod icon for {}", meta.modid);
                                                    None
                                                },
                                            }
                                        };


                                        (Uuid::new_v4().to_string(), Some(meta), logo_data)
                                    },
                                };

                                Ok::<_, anyhow::Error>(Some((
                                    subpath, filesize, enabled, sha512, murmur2, meta_id, meta, logo_data,
                                )))
                            }
                        });

                    let (new_fc_entries, new_meta_entries): (_, Vec<_>) =
                        futures::future::join_all(entry_futures)
                            .await
                            .into_iter()
                            .filter_map(|m| m.unwrap_or(None))
                            .map(
                                |(
                                    subpath,
                                    filesize,
                                    enabled,
                                    sha512,
                                    murmur2,
                                    meta_id,
                                    meta,
                                    logo_data,
                                )| {
                                    (
                                        (
                                            *instance_id,
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
                                                        vec.iter().map(|m| m.to_string()).join(",")
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
                                                        metadb::SetParam::SetLogoImage(logo_data),
                                                    ],

                                                    // Prisma sucks and is generating invalid sql.
                                                    // As a workaround, all the defaults are explicitly set manually.
                                                    None => vec![
                                                        metadb::SetParam::SetName(None),
                                                        metadb::SetParam::SetModid(None),
                                                        metadb::SetParam::SetVersion(None),
                                                        metadb::SetParam::SetDescription(None),
                                                        metadb::SetParam::SetAuthors(None),
                                                        metadb::SetParam::SetLogoImage(None),
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
                            app.prisma_client
                                .mod_metadata()
                                .create_many(new_meta_entries.into_iter().flatten().collect()),
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
            let mut remote_watch = cf_remote_watch;

            while remote_watch.changed().await.is_ok() {
                loop {
                    debug!("remote watch target updated (curseforge)");
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
                        let ignored_hashes = mcm.ignored_remote_cf_hashes.read().await;

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
                                trace!(
                                    "processing curseforge mod batch for instance {instance_id}"
                                );

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
                                let mut ignored_hashes = mcm.ignored_remote_cf_hashes.write().await;
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

                                trace!(
                                    "saving curseforge mod metadata batch of size {}",
                                    deletes.len()
                                );
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
                                    tracing::error!({ error = ?e }, "Could not store curseforge mod metadata");
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

        tokio::spawn(async move {
            let app = app_mr;
            let mut remote_watch = mr_remote_watch;

            while remote_watch.changed().await.is_ok() {
                loop {
                    debug!("remote watch updated (modrinth)");
                    let Some(instance_id) = *remote_watch.borrow() else { break };
                    info!("updating modrinth metadata cache for instance {instance_id}");

                    let fut = async {
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

                        drop(ignored_hashes);

                        let (batch_tx, mut batch_rx) = mpsc::unbounded_channel::<(
                            Vec<String>,
                            Vec<(String, String)>,
                            VersionHashesResponse,
                            ProjectsResponse,
                            Vec<TeamResponse>,
                        )>();

                        let app_db = app.clone();
                        tokio::spawn(async move {
                            while let Some((sha512_hashes, batch, versions, projects, teams)) =
                                batch_rx.recv().await
                            {
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
                                let mcm = app_db.meta_cache_manager();
                                let mut ignored_hashes = mcm.ignored_remote_mr_hashes.write().await;
                                ignored_hashes.extend(
                                    sha512_hashes
                                        .iter()
                                        .filter(|hash| !matches.contains_key(hash))
                                        .cloned(),
                                );
                                drop(ignored_hashes);

                                let (deletes, insert_data) = batch
                                    .into_iter()
                                    .filter_map(|(metadata_id, sha512)| {
                                        let sha512_match = matches.remove(&sha512);
                                        sha512_match.map(|(project, team, version)| {
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
                                            (
                                                app_db
                                                    .prisma_client
                                                    .modrinth_mod_cache()
                                                    .delete_many(vec![
                                                        mrdb::WhereParam::MetadataId(
                                                            StringFilter::Equals(
                                                                metadata_id.clone(),
                                                            ),
                                                        ),
                                                    ]),
                                                (
                                                    file.hashes.sha512.clone(),
                                                    file.hashes.sha1.clone(),
                                                    file.filename.clone(),
                                                    project.id.clone(),
                                                    version.id.clone(),
                                                    project.title.clone(),
                                                    project.slug.clone(),
                                                    project.description.clone(),
                                                    authors,
                                                    chrono::Utc::now().into(),
                                                    metadata_id.clone(),
                                                    Vec::new(),
                                                ),
                                            )
                                        })
                                    })
                                    .unzip::<_, _, Vec<_>, Vec<_>>();

                                trace!(
                                    "saving modrinth mod metadata batch of size {}",
                                    deletes.len()
                                );

                                // may fail if the user removed a mod
                                let r = app_db
                                    .prisma_client
                                    ._batch((
                                        deletes,
                                        app_db
                                            .prisma_client
                                            .modrinth_mod_cache()
                                            .create_many(insert_data),
                                    ))
                                    .await;

                                if let Err(e) = r {
                                    tracing::error!({error = ?e}, "could not store modrinth mod metadata");
                                }
                            }
                        });

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

                            batch_tx
                                .send((sha512_hashes, metadata, versions_response, projects_response, teams_response))
                                .expect("batch processor should not drop until the transmitter is dropped");
                        }

                        Ok::<_, anyhow::Error>(())
                    };

                    // TODO: if we changed to the same instance, just add new mods instead of
                    // forcing the whole thing to rerun
                    tokio::select! {
                        _ = remote_watch.changed() => continue,
                        r = fut => {
                            if let Err(e) = r {
                                error!({ error = ?e }, "failed to query modrinth for instance {instance_id} mods");
                            }
                            break
                        },
                    };
                }
            }
        });
    }
}

fn scale_mod_image(image: &[u8]) -> anyhow::Result<Vec<u8>> {
    use image::imageops::*;

    const TARGET_SIZE: f32 = 45.0;

    let reader = image::io::Reader::new(Cursor::new(image))
        .with_guessed_format()
        .expect("cursor io cannot fail");

    let image = reader.decode()?;

    let mut target = image::RgbaImage::new(45, 45);

    let width = image.width() as f32;
    let height = image.height() as f32;

    if width != 0.0 && height != 0.0 {
        let scale = f32::min(TARGET_SIZE / width, TARGET_SIZE / height);
        let scaled_width = width * scale;
        let scaled_height = height * scale;
        let x_offset = (TARGET_SIZE - scaled_width) * 0.5;
        let y_offset = (TARGET_SIZE - scaled_height) * 0.5;

        overlay(
            &mut target,
            &resize(
                &image,
                scaled_width as u32,
                scaled_height as u32,
                FilterType::Nearest,
            ),
            x_offset as i64,
            y_offset as i64,
        );
    }

    let mut output = Vec::<u8>::new();
    target.write_to(&mut Cursor::new(&mut output), ImageOutputFormat::Png)?;
    Ok(output)
}
