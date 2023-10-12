use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::Cursor;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::usize;

use futures::Future;
use image::ImageOutputFormat;
use itertools::Itertools;
use md5::Digest;
use sha2::Sha512;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;
use tokio::sync::Semaphore;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::api::keys::instance::INSTANCE_MODS;
use crate::db::read_filters::BytesFilter;
use crate::db::read_filters::DateTimeFilter;
use crate::db::read_filters::IntFilter;
use crate::db::read_filters::StringFilter;
use crate::db::{
    curse_forge_mod_cache as cfdb, curse_forge_mod_image_cache as cfimgdb, mod_file_cache as fcdb,
    mod_metadata as metadb, modrinth_mod_cache as mrdb, modrinth_mod_image_cache as mrimgdb,
};
use crate::domain::instance::InstanceId;

use crate::managers::App;
use crate::managers::ManagerRef;

mod curseforge;
mod modrinth;

pub struct MetaCacheManager {
    waiting_instances: RwLock<HashSet<InstanceId>>,
    scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_cf_hashes: RwLock<HashSet<u32>>,
    ignored_remote_mr_hashes: RwLock<HashSet<String>>,
    failed_cf_thumbs: RwLock<HashMap<i32, (std::time::Instant, u32)>>,
    failed_mr_thumbs: RwLock<HashMap<String, (std::time::Instant, u32)>>,
    local_instance: watch::Sender<RemoteCacheTargets>,
    image_scale_semaphore: Semaphore,
    image_download_semaphore: Semaphore,
}

/// Variant of watch where both sides are simultaneously senders and receivers.
#[derive(Clone)]
struct LockNotify<T: Send + Sync> {
    lock: Arc<RwLock<T>>,
    notify: Arc<Notify>,
}

impl<T: Send + Sync> LockNotify<T> {
    fn new(value: T) -> Self {
        Self {
            lock: Arc::new(RwLock::new(value)),
            notify: Arc::new(Notify::new()),
        }
    }

    async fn send_modify(&self, f: impl FnOnce(&mut T) -> bool) {
        let mut lock = self.lock.write().await;

        if f(&mut *lock) {
            self.notify.notify_waiters();
        }
    }

    async fn send(&self, value: T) {
        self.send_modify(|v| {
            *v = value;
            true
        });
    }

    async fn send_silent(&self, value: T) {
        self.send_modify(|v| {
            *v = value;
            false
        });
    }

    async fn borrow(&self) -> RwLockReadGuard<T> {
        self.lock.read().await
    }

    /// Note: will hang forever if all senders drop
    async fn await_change(&mut self) {
        self.notify.notified();
    }
}

trait CompletionSender: Send + Sync {
    fn complete(self, result: anyhow::Result<()>);
}

struct RemoteCacheTargets {
    backend_override: Option<(InstanceId, Box<dyn CompletionSender>)>,
    priority: Option<InstanceId>,
}

struct CacheTarget {
    instance_id: InstanceId,
    is_override: bool,
    is_priority: bool,
}

impl RemoteCacheTargets {
    fn new() -> Self {
        Self {
            backend_override: None,
            priority: None,
        }
    }

    fn target(&self) -> Option<CacheTarget> {
        match self {
            Self {
                backend_override: Some((target, _)),
                priority,
            } => Some(CacheTarget {
                instance_id: *target,
                is_override: true,
                is_priority: priority.is_some_and(|v| target == v),
            }),
            Self {
                backend_override: None,
                priority: Some(target),
            } => Some(CacheTarget {
                instance_id: *target,
                is_override: false,
                is_priority: true,
            }),
            Self {
                backend_override: None,
                priority: None,
            } => None,
        }
    }
}

struct LoopWatcher<T: LoopValue> {
    watcher: LockNotify<T>,
    token: T::Token,
}

trait LoopValue: Send + Sync {
    type Token: Clone + Copy;
    type Value;

    fn token(&self) -> Self::Token;

    // Option<(value, value matches last)>
    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)>;
}

impl LoopValue for RemoteCacheTargets {
    type Token = Option<InstanceId>;
    type Value = (InstanceId, bool);

    fn token(&self) -> Self::Token {
        self.target().map(|(instance, _)| instance)
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)> {
        self.target()
            .map(|v @ (instance, _)| (v, token == Some(instance)))
    }
}

impl LoopValue for Option<InstanceId> {
    type Token = Self;
    type Value = InstanceId;

    fn token(&self) -> Self::Token {
        *self
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)> {
        match self {
            Some(v) => Some((*v, token == Some(*v))),
            None => None,
        }
    }
}

impl<T: LoopValue> LoopWatcher<T> {
    async fn new(watch: LockNotify<T>) -> Self {
        let token = watch.borrow().await.token();
        Self {
            token,
            watcher: watch,
        }
    }

    // Option<(value, value matches last)>
    async fn next(&mut self) -> Option<(T::Value, bool)> {
        loop {
            if let Some(v) = self.watcher.borrow().await.loop_cmp(self.token) {
                return Some(v);
            }

            self.watcher.await_change().await;
        }
    }

    /// Calls F whenever a new value is recieved, interrupting the current call if the value is different.
    /// Calls the function F returns when F completes. The returned function cannot be interrupted.
    async fn loop_interrupt<F>(&mut self, mut f: impl FnMut(T::Value) -> F)
    where
        F: Future,
        F::Output: for<'a> FnOnce(&'a mut T) -> bool,
    {
        let val = &|(v, _): (T::Value, bool)| v;
        let mut next = self.next().await.map(val);

        while let Some(n) = next.take() {
            tokio::select! {
                v = async {
                    loop {
                        match self.next().await {
                            Some((v, false)) => break v,
                            Some((v, true)) => {
                                next = Some(v);
                                continue
                            },
                            None => futures::future::pending().await,
                        }
                    }
                } => next = Some(v),
                f2 = f(n) => {
                    self.watcher.send_modify(f2).await;

                    if next.is_none() {
                        next = self.next().await.map(val);
                    }
                }
            }
        }
    }
}

#[async_trait::async_trait]
trait ModplatformCacher {
    type SaveBundle;

    async fn query_platform(
        app: &App,
        instance_id: InstanceId,
        sender: &BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()>;

    async fn save_batch(app: &App, instance_id: InstanceId, batch: Self::SaveBundle);

    async fn cache_icons(
        app: &App,
        instance_id: InstanceId,
        update_notifier: &mpsc::UnboundedSender<InstanceId>,
    );

    fn print_error(instance_id: InstanceId, error: &anyhow::Error);
}

type ModplatformCacheBundle<T> = (InstanceId, bool, T, Option<oneshot::Sender<()>>);

struct BundleSender<'a, T> {
    should_wait: bool,
    instance_id: InstanceId,
    update_images: bool,
    active_wait: Option<oneshot::Receiver<()>>,
    sender: &'a mpsc::UnboundedSender<ModplatformCacheBundle<T>>,
}

impl<'a, T> BundleSender<'a, T> {
    fn new(
        instance_id: InstanceId,
        wait: bool,
        update_images: bool,
        sender: &'a mpsc::UnboundedSender<ModplatformCacheBundle<T>>,
    ) -> Self {
        Self {
            instance_id,
            should_wait: wait,
            update_images,
            active_wait: None,
            sender,
        }
    }

    fn send(&self, bundle: T) {
        let (tx, rx) = match self.should_wait {
            true => {
                let (tx, rx) = oneshot::channel();
                (Some(tx), Some(rx))
            }
            false => (None, None),
        };

        self.active_wait = rx;
        let _ = self
            .sender
            .send((self.instance_id, self.update_images, bundle, tx));
    }

    async fn wait(self) {
        if let Some(wait) = self.active_wait {
            let _ = wait.await;
        }
    }
}

fn cache_modplatform<C: ModplatformCacher>(
    app: App,
    rx: LockNotify<RemoteCacheTargets>,
    update_notifier: mpsc::UnboundedSender<InstanceId>,
) {
    tokio::spawn(async move {
        let (batch_tx, batch_rx) =
            mpsc::unbounded_channel::<ModplatformCacheBundle<C::SaveBundle>>();
        let image_rx = LockNotify::<Option<InstanceId>>::new(None);
        let image_tx = image_rx.clone();

        let query_loop = LoopWatcher::new(rx).await.loop_interrupt(
            |CacheTarget {
                 instance_id,
                 is_override,
                 is_priority,
             }| async {
                let sender = BundleSender::new(instance_id, is_override, is_priority, &batch_tx);
                let r = C::query_platform(&app, instance_id, &sender).await;

                if let Err(e) = &r {
                    C::print_error(instance_id, e);
                }

                sender.wait();

                |targets: &mut RemoteCacheTargets| {
                    if is_override {
                        match targets.backend_override.take() {
                            Some((instance, callback)) if instance == instance_id => {
                                callback.complete(r);
                                true
                            }
                            Some(v) => {
                                targets.backend_override = Some(v);
                                false
                            }
                            None => false,
                        }
                    } else {
                        false
                    }
                }
            },
        );

        let save_loop = async {
            while let Some((instance_id, update_images, bundle, notify)) = batch_rx.recv().await {
                C::save_batch(&app, instance_id, bundle).await;

                if update_images {
                    image_tx.send(Some(instance_id)).await;
                }

                if let Some(notify) = notify {
                    let _ = notify.send(());
                }

                update_notifier.send(instance_id);
            }
        };

        let image_loop = LoopWatcher::new(image_rx)
            .await
            .loop_interrupt(|instance_id| async {
                C::cache_icons(&app, instance_id, &update_notifier).await;
                |_| false
            });

        // None of the futures should ever exit.
        // This join polls both while allowing them to share variables in this scope.
        futures::join!(query_loop, save_loop, image_loop);
    });
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
            local_instance: watch::channel(RemoteCacheTargets::new()).0,
            curseforge_instance: watch::channel(RemoteCacheTargets::new()).0,
            modrinth_instance: watch::channel(RemoteCacheTargets::new()).0,
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
        let (cf_cache_tx, cf_watch) = watch::channel(RemoteCacheTargets::new());
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

                if let CacheRequest {
                    target: Some(target_instance),
                    ..
                } = target_instance
                {
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

                        let allowed_base_ext =
                            [".jar", ".zip"].iter().any(|&ext| utf8_name.ends_with(ext));
                        let allowed_disabled_ext = [".jar.disabled", ".zip.disabled"]
                            .iter()
                            .any(|&ext| utf8_name.ends_with(ext));

                        if !allowed_base_ext && !allowed_disabled_ext {
                            continue;
                        }

                        utf8_name = utf8_name.strip_suffix(".disabled").unwrap_or(utf8_name);

                        let Ok(metadata) = entry.metadata().await else {
                            continue;
                        };
                        // file || symlink
                        if metadata.is_dir() {
                            continue;
                        }

                        trace!("tracking mod `{utf8_name}` for instance {instance_id}");
                        modpaths.insert(
                            utf8_name.to_string(),
                            (!allowed_disabled_ext, metadata.len()),
                        );
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

        let prev_ext = path
            .extension()
            .and_then(OsStr::to_str)
            .ok_or(anyhow::anyhow!(
                "mod file `{}` has no extension",
                mod_filename
            ))?;

        if !enabled {
            path.set_extension(format!("{prev_ext}.disabled"));
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

        let meta = match meta {
            Ok(meta) => meta,
            Err(e) => {
                warn!({ error = ?e }, "could not parse mod metadata for {}", mod_filename);
                None
            }
        };

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
                            error!({ error = ?e }, "could not scale mod icon for {}", mod_filename);
                            None
                        }
                        Err(e) => {
                            error!({ error = ?e }, "could not scale mod icon for {}", mod_filename);
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
                            metadb::SetParam::SetModid(meta.modid),
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
