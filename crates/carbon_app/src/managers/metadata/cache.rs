use std::collections::HashMap;
use std::collections::HashSet;

use std::collections::VecDeque;
use std::io::Cursor;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::usize;

use futures::Future;
use image::ImageOutputFormat;
use md5::Digest;

use sha2::Sha512;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
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

use crate::domain::modplatforms::modrinth::project::Project;
use crate::domain::modplatforms::modrinth::responses::ProjectsResponse;
use crate::domain::modplatforms::modrinth::responses::TeamResponse;
use crate::domain::modplatforms::modrinth::responses::VersionHashesResponse;
use crate::domain::modplatforms::modrinth::search::ProjectIDs;
use crate::domain::modplatforms::modrinth::search::TeamIDs;
use crate::domain::modplatforms::modrinth::search::VersionHashesQuery;
use crate::domain::modplatforms::modrinth::version::HashAlgorithm;
use crate::domain::runtime_path::InstancesPath;

use crate::managers::ManagerRef;

use crate::db::{
    curse_forge_mod_cache as cfdb, curse_forge_mod_image_cache as cfimgdb, mod_file_cache as fcdb,
    mod_metadata as metadb, modrinth_mod_cache as mrdb, modrinth_mod_image_cache as mrimgdb,
};
use itertools::Itertools;

pub struct MetaCacheManager {
    waiting_instances: RwLock<HashSet<InstanceId>>,
    scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_cf_hashes: RwLock<HashSet<u32>>,
    ignored_remote_mr_hashes: RwLock<HashSet<String>>,
    failed_cf_thumbs: RwLock<HashMap<i32, (std::time::Instant, u32)>>,
    failed_mr_thumbs: RwLock<HashMap<String, (std::time::Instant, u32)>>,
    local_instance: watch::Sender<CacheTargets>,
    image_scale_semaphore: Semaphore,
    image_download_semaphore: Semaphore,
}

trait CompletionSender {
    fn complete(self);
}

struct CacheTargets<S: CompletionSender> {
    backend_override: Arc<RwLock<Option<(InstanceId, S)>>>,
    priority: Option<InstanceId>,
}

#[derive(Clone)]
enum ActiveCacheTask<S: CompletionSender> {
    BackendOverride(Arc<RwLock<Option<(InstanceId, S)>>>),
    Standard(InstanceId),
}

impl<S: CompletionSender> CacheTargets<S> {
    fn new() -> Self {
        Self {
            backend_override: None,
            priority: None,
        }
    }

    // TODO: need a locking form of activecachetask maybe
    async fn target_instance(&self) -> Option<InstanceId> {
        let backend_override = self.backend_override.read().await.map(|(instance, _)| instance);

        match (backend_override, self.priority) {
            (Some(instance), _) => Some(instance),
            (None, Some(instance)) => Some(instance),
            (None, None) => None,
        }
    }


    fn target(&self) -> Option<&CacheRequest<S>> {
        match self {
            Self { backend_override: Some(target), priority: _ } => Some(target),
            Self { backend_override: None, priority: Some(target) } => Some(target),
            Self { backend_override: None, priority: None } => None,
        }
    }
}

impl<S: CompletionSender> CacheRequest<S> {
    fn new(instance: InstanceId) -> Self {
        Self {
            instance,
            progress_sender: None,
        }
    }

    fn take(&mut self) -> ActiveCacheTask<S> {
        ActiveCacheTask {
            instance: self.instance,
            sender: self.progress_sender.take(),
        }
    }
}

impl<S: CompletionSender> ActiveCacheTask<S> {
    fn complete(self) {
        if let Some(sender) = self.sender {
            sender.complete();
        }
    }
}

struct LoopWatcher<T: LoopValue> {
    watcher: watch::Receiver<T>,
    token: T::Token,
}

trait LoopValue {
    type Token: Clone + Copy;
    type Value;

    fn token(&self) -> Self::Token;
    // Option<(value, value matches last)>
    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)>;
}

impl<S: CompletionSender> LoopValue for CacheTargets<S> {
    type Token = Option<InstanceId>;
    type Value = ActiveCacheTask<S>;

    fn token(&self) -> Self::Token {
        self.target().map(|target| target.instance)
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)> {
        match self.target() {
            Some(req @ CacheRequest {
                instance, ..
            }) => Some((req.take(), token == Some(*instance))),
            None => None,
        }
    }
}

impl LoopValue for Option<InstanceId> {
    type Token = Self;
    type Value = InstanceId;

    fn token(&self) -> Self::Token {
        *self
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<Self::Value> {
        match *self == token {
            false => *self,
            true => None,
        }
    }
}

impl<T: LoopValue> LoopWatcher<T> {
    fn new(watch: watch::Receiver<T>) -> Self {
        Self {
            token: watch.borrow().token(),
            watcher: watch,
        }
    }

    // Option<(value, value matches last)>
    async fn next(&mut self) -> Option<(T::Value, bool)> {
        loop {
            if let Some(v) = self.watcher.borrow().loop_cmp(self.token) {
                return Some(v)
            }

            if !self.watcher.changed().await.is_ok() {
                return None
            }
        }
    }

    async fn loop_interrupt<F: Future<Output = ()>>(&mut self, mut f: impl FnMut(T::Value) -> F) {
        let mut next = self.next().await;

        while let Some(n) = next.take() {
            tokio::select! {
                v = async {
                    loop {
                        match self.next().await {
                            Some((v, false)) => break v,
                            Some((v, true)) => {
                                next = v;
                                continue
                            },
                            None => futures::future::pending().await,
                        }
                    }
                } => next = Some(v),
                _ = f(n) => {
                    if next.is_none() {
                        next = self.next().await;
                    }
                }
            }
        }
    }
}

impl MetaCacheManager {
    pub fn new() -> Self {
        Self {
            waiting_instances: RwLock::new(HashSet::new()),
            scanned_instances: Mutex::new(HashSet::new()),
            ignored_remote_cf_hashes: RwLock::new(HashSet::new()),
            ignored_remote_mr_hashes: RwLock::new(HashSet::new()),
            failed_cf_thumbs: RwLock::new(HashMap::new()),
            failed_mr_thumbs: RwLock::new(HashMap::new()),
            local_instance: watch::channel(CacheTargets::new()).0,
            curseforge_instance: watch::channel(CacheTargets::new()).0,
            modrinth_instance: watch::channel(CacheTargets::new()).0,
            image_cache_instance: watch::channel(None).0,
            image_scale_semaphore: Semaphore::new(num_cpus::get()),
            image_download_semaphore: Semaphore::new(10),
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
        let local_watch = self.local_instance.subscribe();
        let remote_watch = self.remote_instance.subscribe();
        let cf_remote_watch = self.curseforge_instance.subscribe();
        let mr_remote_watch = self.modrinth_instance.subscribe();
        let (cf_cache_tx, cf_watch) = watch::channel(CacheTargets::new());
        let (cf_icons_tx, cf_icons_watch) = watch::channel(Option::<InstanceId>::None);

        let app_local = self.app.clone();
        let app_cf = self.app.clone();
        let app_cficons = self.app.clone();
        let app_mr = self.app.clone();
        let app_debounce = self.app.clone();
        let debounce_remote_watch = remote_watch;

        let (list_debounce_tx, mut list_debounce_rx) = mpsc::unbounded_channel::<()>();

        let cf_list_debounce_tx = list_debounce_tx.clone();
        let mr_list_debounce_tx = list_debounce_tx;

        tokio::spawn(async move {
            while list_debounce_rx.recv().await.is_some() {
                let target_instance = (*debounce_remote_watch.borrow()).clone();

                // clear notification queue
                while list_debounce_rx.try_recv().is_ok() {}

                if let CacheRequest { target: Some(target_instance), .. } = target_instance {
                    app_debounce.invalidate(INSTANCE_MODS, Some(target_instance.0.into()));
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });

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

                    let Some(instance_id) = instance_id else {
                        break;
                    };
                    info!(
                        { priority },
                        "recaching instance mod metadata for {instance_id}"
                    );

                    let instances = instance_manager.instances.read().await;
                    let Some(instance) = instances.get(&instance_id) else {
                        continue;
                    };

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
                        let Some(mut utf8_name) = file_name.to_str() else {
                            continue;
                        };

                        let is_jar = utf8_name.ends_with(".jar");
                        let is_jar_disabled = utf8_name.ends_with(".jar.disabled");

                        if !is_jar && !is_jar_disabled {
                            continue;
                        }

                        if is_jar_disabled {
                            utf8_name = utf8_name.strip_suffix(".disabled").unwrap();
                        }

                        let Ok(metadata) = entry.metadata().await else {
                            continue;
                        };
                        // file || symlink
                        if metadata.is_dir() {
                            continue;
                        }

                        trace!("tracking mod `{utf8_name}` for instance {instance_id}");
                        modpaths.insert(utf8_name.to_string(), (!is_jar_disabled, metadata.len()));
                    }

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
                        }
                    }

                    let entry_futures = modpaths.into_iter().map(|(subpath, (enabled, _))| {
                        let app = &app;
                        let pathbuf = &pathbuf;

                        async move {
                            app.meta_cache_manager()
                                .cache_mod_file_unchecked(instance_id, pathbuf, subpath, enabled)
                                .await
                                .map(|_| ())
                        }
                    });

                    let r = futures::future::join_all(entry_futures)
                        .await
                        .into_iter()
                        .collect::<anyhow::Result<()>>();

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
            let app = &app_cf;
            let list_debounce = &cf_list_debounce_tx;
            let icons_tx = &cf_icons_tx;

            let f = |task| async move {
                let instance_id = task.instance;

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
                            Option<ActiveCacheTask>,
                )>();

                let app_db = app.clone();
                let list_debounce = list_debounce.clone();
                tokio::spawn(async move {
                    while let Some((fingerprints, batch, fp_response, mods_response, task)) =
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

                        let app_db = &app_db;
                        let futures =
                            batch.into_iter().filter_map(|(metadata_id, murmur2)| {
                                let fpmatch = matches.remove(&murmur2);
                                fpmatch.map(|(fileinfo, modinfo)| async move {
                                    let r = app_db
                                        .meta_cache_manager()
                                        .cache_curseforge_meta_unchecked(
                                            metadata_id,
                                            fileinfo.file.id,
                                            murmur2,
                                            modinfo,
                                        ).await;

                                    if let Err(e) = r {
                                        tracing::error!({ error = ?e }, "Could not store curseforge mod metadata");
                                    }
                                })
                            });

                        futures::future::join_all(futures).await;
                        let _ = list_debounce.send(());
                        icons_tx.send(Some(task.instance));

                        if let Some(task) = task {
                            task.complete();
                        }
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

            LoopWatcher::new(cf_watch).loop_interrupt(|task| async move {
                let instance_id = task.instance;

                if let Err(e) = f(task).await {
                    error!({ error = ?e }, "failed to query curseforge for instance {instance_id} mods");
                }
            }).await;
        });

        tokio::spawn(async move {
            let app = &app_cficons;

            LoopWatcher::new(cf_icons_watch).loop_interrupt(|instance_id| async move {
                let modlist =
                    app.prisma_client
                        .mod_file_cache()
                        .find_many(vec![
                            fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                            fcdb::WhereParam::MetadataIs(vec![
                                metadb::WhereParam::CurseforgeIs(vec![
                                    cfdb::WhereParam::LogoImageIs(vec![
                                        cfimgdb::WhereParam::UpToDate(IntFilter::Equals(0)),
                                    ]),
                                ]),
                            ]),
                        ])
                        .with(fcdb::metadata::fetch().with(
                            metadb::curseforge::fetch().with(cfdb::logo_image::fetch()),
                        ))
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
                                file.instance_id,
                                file.filename,
                                cf.project_id,
                                cf.file_id,
                                row,
                    )
                });

                let app = &app;
                let futures = modlist
                    .map(|(instance_id, filename, project_id, file_id, row)| async move {
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
                                let scaled = scale_mod_image(&image[..])?;
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

                            let _ = list_debounce.send(());
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
            });
        });

        tokio::spawn(async move {
            let app = app_mr;
            let list_debounce = &mr_list_debounce_tx;
            let mut remote_watch = mr_remote_watch;

            while remote_watch.changed().await.is_ok() {
                loop {
                    debug!("remote watch updated (modrinth)");
                    let Some(instance_id) = *remote_watch.borrow() else {
                        break;
                    };
                    info!("updating modrinth metadata cache for instance {instance_id}");

                    let icons_fut = async {
                        let modlist = app
                            .prisma_client
                            .mod_file_cache()
                            .find_many(vec![
                                fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
                                fcdb::WhereParam::MetadataIs(vec![metadb::WhereParam::ModrinthIs(
                                    vec![mrdb::WhereParam::LogoImageIs(vec![
                                        mrimgdb::WhereParam::UpToDate(IntFilter::Equals(0)),
                                    ])],
                                )]),
                            ])
                            .with(
                                fcdb::metadata::fetch().with(
                                    metadb::modrinth::fetch().with(mrdb::logo_image::fetch()),
                                ),
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
                            let mr = meta
                                .modrinth
                                .flatten()
                                .expect("modrinth was ensured present but not returned");
                            let row = mr
                                .logo_image
                                .flatten()
                                .expect("mod image was ensured present but not returned");

                            (
                                file.instance_id,
                                file.filename,
                                mr.project_id,
                                mr.version_id,
                                row,
                            )
                        });

                        let app = &app;
                        let futures = modlist
                            .into_iter()
                            .map(|(instance_id, filename, project_id, version_id, row)| async move {
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

                                    debug!("thumbnailing modrinth mod icon for {instance_id}/{filename} (project: {project_id}, version: {version_id})");

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
                                        let scaled = scale_mod_image(&image[..])?;
                                        Ok::<_, anyhow::Error>(scaled)
                                    })
                                        .await??;

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

                                    debug!("saved modrinth mod thumbnail for {instance_id}/{filename} (project: {project_id}, version: {version_id})");

                                    let _ = list_debounce.send(());
                                    Ok::<_, anyhow::Error>(())
                                }.await;

                                if let Err(e) = r {
                                    error!({ error = ?e }, "error downloading mod icon for {instance_id}/{filename} (project: {project_id}, version: {version_id}, image url: {})", row.url);

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
                    };

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
                        let list_debounce = list_debounce.clone();
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

                                let app_db = &app_db;
                                let futures = batch
                                    .into_iter()
                                    .filter_map(|(metadata_id, sha512)| {
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
                                                    member.user.name.clone().unwrap_or_else(|| {
                                                        member.user.username.clone()
                                                    })
                                                })
                                                .join(", ");

                                            let r = app_db.meta_cache_manager()
                                                .cache_modrinth_meta_unchecked(
                                                    metadata_id,
                                                    version.id.clone(),
                                                    file.hashes.sha512.clone(),
                                                    project.clone(),
                                                    authors,
                                                ).await;

                                            if let Err(e) = r {
                                                tracing::error!({ error = ?e }, "Could not store modrinth mod metadata");
                                            }
                                        })
                                    });

                                futures::future::join_all(futures).await;
                                let _ = list_debounce.send(());
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

                    // not done inline to avoid indenting it even further
                    let fut = async {
                        if let Err(e) = fut.await {
                            error!({ error = ?e }, "failed to query modrinth for instance {instance_id} mods");
                        }
                    };

                    let wait_changed = async {
                        while remote_watch.changed().await.is_ok() {
                            if *remote_watch.borrow() != Some(instance_id) {
                                break;
                            }
                        }
                    };

                    tokio::select! {
                        _ = wait_changed => continue,
                        _ = futures::future::join(fut, icons_fut) => break,
                    };
                }
            }
        });
    }

    /// Cache a mod file without first checking the validity of the instance
    async fn cache_mod_file_unchecked(
        self,
        instance_id: InstanceId,
        mods_dir_path: &PathBuf,
        mod_filename: String,
        enabled: bool,
    ) -> anyhow::Result<String> {
        let mut path = mods_dir_path.join(&mod_filename);

        if !enabled {
            path.set_extension("jar.disabled");
        }

        let content = tokio::fs::read(path).await?;
        let content_len = content.len();
        let (content, sha512, meta, murmur2) = tokio::task::spawn_blocking(move || {
            let sha512: [u8; 64] = Sha512::new_with_prefix(&content).finalize().into();
            let meta = super::mods::parse_metadata(Cursor::new(&content));

            // curseforge's api removes whitespace in murmur2 hashes
            let mut murmur_content = content.clone();
            murmur_content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
            let murmur2 = murmurhash32::murmurhash2(&murmur_content);

            (content, sha512, meta, murmur2)
        })
        .await?;

        let meta = meta?;

        let dbmeta = self
            .app
            .prisma_client
            .mod_metadata()
            // just check both hashes for now
            .find_first(vec![
                metadb::WhereParam::Sha512(BytesFilter::Equals(Vec::from(sha512))),
                metadb::WhereParam::Murmur2(IntFilter::Equals(murmur2 as i32)),
            ])
            .exec()
            .await?;

        let (meta_id, meta_insert, logo_insert) = match dbmeta {
            Some(meta) => (meta.id, None, None),
            None => {
                let meta_id = Uuid::new_v4().to_string();

                let logo_insert = 'logo: {
                    let Some(meta) = &meta else { break 'logo None };
                    let Some(logo_file) = &meta.logo_file else {
                        break 'logo None;
                    };
                    let logo_file = logo_file.to_string();

                    let mcm = self.app.meta_cache_manager();
                    let guard = mcm
                        .image_scale_semaphore
                        .acquire()
                        .await
                        .expect("the image scale semaphore is never closed");

                    let logo = tokio::task::spawn_blocking(move || {
                        let mut zip = zip::ZipArchive::new(Cursor::new(&content)).unwrap();
                        let Ok(mut file) = zip.by_name(&logo_file) else {
                            return Ok(None);
                        };
                        let mut image = Vec::with_capacity(file.size() as usize);
                        file.read_to_end(&mut image)?;
                        let scaled = scale_mod_image(&image[..])?;
                        Ok::<_, anyhow::Error>(Some(scaled))
                    })
                    .await;

                    drop(guard);

                    match logo {
                        Ok(Ok(Some(data))) => {
                            Some(self.app.prisma_client.local_mod_image_cache().create(
                                data,
                                metadb::UniqueWhereParam::IdEquals(meta_id.clone()),
                                Vec::new(),
                            ))
                        }
                        Ok(Ok(None)) => None,
                        Ok(Err(e)) => {
                            error!({ error = ?e }, "could not scale mod icon for {}", meta.modid);
                            None
                        }
                        Err(e) => {
                            error!({ error = ?e }, "could not scale mod icon for {}", meta.modid);
                            None
                        }
                    }
                };

                let meta_insert = self.app.prisma_client.mod_metadata().create(
                    meta_id.clone(),
                    murmur2 as i32,
                    Vec::from(sha512),
                    meta.as_ref()
                        .map(|meta| &meta.modloaders)
                        .map(|modloaders| modloaders.iter().map(ToString::to_string).join(","))
                        .unwrap_or(String::new()),
                    match meta {
                        Some(meta) => vec![
                            metadb::SetParam::SetName(meta.name),
                            metadb::SetParam::SetModid(Some(meta.modid)),
                            metadb::SetParam::SetVersion(meta.version),
                            metadb::SetParam::SetDescription(meta.description),
                            metadb::SetParam::SetAuthors(meta.authors),
                        ],

                        // Prisma sucks and is generating invalid sql.
                        // As a workaround, all the defaults are manually set to None.
                        None => vec![
                            metadb::SetParam::SetName(None),
                            metadb::SetParam::SetModid(None),
                            metadb::SetParam::SetVersion(None),
                            metadb::SetParam::SetDescription(None),
                            metadb::SetParam::SetAuthors(None),
                        ],
                    },
                );

                (meta_id, Some(meta_insert), logo_insert)
            }
        };

        let filecache_delete = self.app.prisma_client.mod_file_cache().delete_many(vec![
            fcdb::WhereParam::InstanceId(IntFilter::Equals(*instance_id)),
            fcdb::WhereParam::Filename(StringFilter::Equals(mod_filename.to_string())),
        ]);

        let filecache_insert = self.app.prisma_client.mod_file_cache().create(
            crate::db::instance::UniqueWhereParam::IdEquals(*instance_id),
            mod_filename.to_string(),
            content_len as i32,
            enabled,
            metadb::UniqueWhereParam::IdEquals(meta_id.clone()),
            Vec::new(),
        );

        debug!(
            "updating metadata entries for {}/{mod_filename}",
            *instance_id
        );

        self.app
            .prisma_client
            ._batch((
                meta_insert.into_iter().collect::<Vec<_>>(),
                logo_insert.into_iter().collect::<Vec<_>>(),
                filecache_delete,
                filecache_insert,
            ))
            .await?;

        Ok(meta_id)
    }

    // Cache curseforge metadata for a mod without downloading the icon
    async fn cache_curseforge_meta_unchecked(
        self,
        metadata_id: String,
        file_id: i32,
        murmur2: u32,
        modinfo: Mod,
    ) -> anyhow::Result<()> {
        let prev = self
            .app
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

        let o_insert_cfmeta = self.app.prisma_client.curse_forge_mod_cache().create(
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
            o_delete_cfmeta = Some(self.app.prisma_client.curse_forge_mod_cache().delete(
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
                                Some(self.app.prisma_client.curse_forge_mod_image_cache().update(
                                    cfimgdb::UniqueWhereParam::MetadataIdEquals(
                                        metadata_id.clone(),
                                    ),
                                    vec![
                                        cfimgdb::SetParam::SetUrl(url.clone()),
                                        cfimgdb::SetParam::SetUpToDate(0),
                                    ],
                                ));
                        }
                    }
                    None => {
                        o_delete_logo =
                            Some(self.app.prisma_client.curse_forge_mod_image_cache().delete(
                                cfimgdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                            ));
                    }
                }
            }
        }

        if o_update_logo.is_none() && o_delete_logo.is_none() {
            if let Some(url) = modinfo.logo.map(|it| it.url) {
                o_insert_logo = Some(self.app.prisma_client.curse_forge_mod_image_cache().create(
                    url,
                    cfdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                    Vec::new(),
                ));
            }
        }

        debug!("updating curseforge metadata entry for {metadata_id}");

        self.app
            .prisma_client
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

    // Cache modrinth metadata for a mod without downloading the icon
    async fn cache_modrinth_meta_unchecked(
        self,
        metadata_id: String,
        version_id: String,
        sha512: String,
        project: Project,
        authors: String,
    ) -> anyhow::Result<()> {
        let prev = self
            .app
            .prisma_client
            .modrinth_mod_cache()
            .find_unique(mrdb::UniqueWhereParam::MetadataIdEquals(
                metadata_id.clone(),
            ))
            .with(mrdb::logo_image::fetch())
            .exec()
            .await?;

        let mut o_delete_mrmeta = None;
        let mut o_insert_logo = None;
        let mut o_update_logo = None;
        let mut o_delete_logo = None;

        let o_insert_mrmeta = self.app.prisma_client.modrinth_mod_cache().create(
            sha512.clone(),
            project.id,
            version_id,
            project.title,
            project.slug,
            project.description,
            authors,
            chrono::Utc::now().into(),
            metadb::UniqueWhereParam::IdEquals(metadata_id.clone()),
            Vec::new(),
        );

        if let Some(prev) = prev {
            o_delete_mrmeta = Some(self.app.prisma_client.modrinth_mod_cache().delete(
                mrdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
            ));

            if let Some(prev) = prev
                .logo_image
                .expect("logo_image was requesred but not returned by prisma")
            {
                match project.icon_url.as_ref() {
                    Some(url) => {
                        if *url != prev.url {
                            o_update_logo =
                                Some(self.app.prisma_client.modrinth_mod_image_cache().update(
                                    mrimgdb::UniqueWhereParam::MetadataIdEquals(
                                        metadata_id.clone(),
                                    ),
                                    vec![
                                        mrimgdb::SetParam::SetUrl(url.clone()),
                                        mrimgdb::SetParam::SetUpToDate(0),
                                    ],
                                ));
                        }
                    }
                    None => {
                        o_delete_logo =
                            Some(self.app.prisma_client.modrinth_mod_image_cache().delete(
                                mrimgdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                            ));
                    }
                }
            }
        }

        if o_update_logo.is_none() && o_delete_logo.is_none() {
            if let Some(url) = project.icon_url {
                o_insert_logo = Some(self.app.prisma_client.modrinth_mod_image_cache().create(
                    url,
                    mrdb::UniqueWhereParam::MetadataIdEquals(metadata_id.clone()),
                    Vec::new(),
                ));
            }
        }

        debug!("updating modrinth metadata entry for {metadata_id}");

        self.app
            .prisma_client
            ._batch((
                o_delete_mrmeta.into_iter().collect::<Vec<_>>(),
                o_insert_mrmeta,
                o_delete_logo.into_iter().collect::<Vec<_>>(),
                o_insert_logo.into_iter().collect::<Vec<_>>(),
                o_update_logo.into_iter().collect::<Vec<_>>(),
            ))
            .await?;

        Ok(())
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
