use crate::api::keys::instance::{INSTANCE_DETAILS, INSTANCE_MODS};
use crate::api::keys::server::GET_SERVER_ADDONS;
use crate::api::translation::Translation;
use crate::domain::instance::InstanceId;
use crate::managers::App;
use crate::managers::ManagerRef;
use crate::managers::vtask::VisualTask;
use anyhow::anyhow;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::mod_metadata as metarepo;
use chrono::Utc;
use carbon_rt_path::InstancesPath;
use curseforge::CurseforgeModCacher;
use futures::Future;
use futures::join;
use image::ImageFormat;
use itertools::Itertools;
use md5::Digest;
use modrinth::ModrinthModCacher;
use murmurhash32::Murmur2Digest;
use sha1::Sha1;
use sha2::Sha512;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::Cursor;
use std::io::Read;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic;
use std::sync::atomic::AtomicI32;
use std::thread::available_parallelism;
use std::usize;
use tokio::io::AsyncSeekExt;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time::Instant as TokioInstant;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

pub mod curseforge;
pub mod modrinth;

/// Throttle Modrinth requests originating from the metadata-cache loop.
/// Modrinth's public limit is 300 req/min (sliding window, reset via
/// `X-RateLimit-Reset`). We cap ourselves at 210/min — 70% of the budget —
/// so user-driven requests (which bypass this throttle) always have headroom
/// even if the cache is running flat-out.
///
/// Scoped to the metadata cache only — direct user actions (search, install,
/// browser) hit `app.modplatforms_manager().modrinth` without going through
/// this throttle.
pub(crate) struct ModrinthCacheThrottle {
    history: Mutex<std::collections::VecDeque<TokioInstant>>,
    max_per_window: usize,
    window: std::time::Duration,
}

impl ModrinthCacheThrottle {
    fn new(max_per_window: usize, window: std::time::Duration) -> Self {
        Self {
            history: Mutex::new(std::collections::VecDeque::with_capacity(max_per_window)),
            max_per_window,
            window,
        }
    }

    /// Block until firing another request would stay within the configured
    /// window, then record the request and return. Bursts up to
    /// `max_per_window` are allowed.
    pub async fn acquire(&self) {
        loop {
            let wait_until = {
                let mut history = self.history.lock().await;
                let now = TokioInstant::now();

                while let Some(&front) = history.front() {
                    if now.duration_since(front) >= self.window {
                        history.pop_front();
                    } else {
                        break;
                    }
                }

                if history.len() < self.max_per_window {
                    history.push_back(now);
                    return;
                }

                // Wait for the oldest entry to age out of the window.
                *history.front().expect("history is full so front exists") + self.window
            };

            tokio::time::sleep_until(wait_until).await;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheEntityId {
    Instance(InstanceId),
    Server(i32),
}

impl std::fmt::Display for CacheEntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Instance(id) => write!(f, "instance {}", *id),
            Self::Server(id) => write!(f, "server {}", id),
        }
    }
}

/// Intermediate result from hashing and parsing a mod file.
/// Entity-independent — used by both instance and server caching.
struct ModFileParseResult {
    sha512: [u8; 64],
    sha1: [u8; 20],
    murmur2: u32,
    content_len: usize,
    meta: Option<super::mods::ModFileMetadata>,
    image_data: Option<Vec<u8>>,
}

pub struct MetaCacheManager {
    //waiting_instances: RwLock<HashSet<InstanceId>>,
    //scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_cf_hashes: RwLock<HashSet<u32>>,
    ignored_remote_mr_hashes: RwLock<HashSet<String>>,
    failed_cf_instances: RwLock<HashMap<CacheEntityId, (std::time::Instant, u32)>>,
    failed_mr_instances: RwLock<HashMap<CacheEntityId, (std::time::Instant, u32)>>,
    failed_cf_thumbs: RwLock<HashMap<i32, (std::time::Instant, u32)>>,
    failed_mr_thumbs: RwLock<HashMap<String, (std::time::Instant, u32)>>,
    local_targets: LockNotify<CacheTargets>,
    curseforge_targets: LockNotify<CacheTargets>,
    modrinth_targets: LockNotify<CacheTargets>,
    image_scale_semaphore: Semaphore,
    image_download_semaphore: Semaphore,
    watched_entity: watch::Sender<Option<CacheEntityId>>,
    pause_caching: watch::Sender<bool>,
    pub(crate) modrinth_throttle: ModrinthCacheThrottle,
}

impl MetaCacheManager {
    pub fn new() -> Self {
        Self {
            //waiting_instances: RwLock::new(HashSet::new()),
            //scanned_instances: Mutex::new(HashSet::new()),
            ignored_remote_cf_hashes: RwLock::new(HashSet::new()),
            ignored_remote_mr_hashes: RwLock::new(HashSet::new()),
            failed_cf_instances: RwLock::new(HashMap::new()),
            failed_mr_instances: RwLock::new(HashMap::new()),
            failed_cf_thumbs: RwLock::new(HashMap::new()),
            failed_mr_thumbs: RwLock::new(HashMap::new()),
            local_targets: LockNotify::new(CacheTargets::new()),
            curseforge_targets: LockNotify::new(CacheTargets::new()),
            modrinth_targets: LockNotify::new(CacheTargets::new()),
            image_scale_semaphore: Semaphore::new(1),
            image_download_semaphore: Semaphore::new(10),
            watched_entity: watch::channel(None).0,
            pause_caching: watch::channel(false).0,
            modrinth_throttle: ModrinthCacheThrottle::new(210, std::time::Duration::from_secs(60)),
        }
    }

    /// Get the entity IDs that are currently being cached
    pub async fn get_currently_caching_entities(&self) -> Vec<CacheEntityId> {
        let mut result = Vec::new();

        let local = self.local_targets.borrow().await.target();
        let cf = self.curseforge_targets.borrow().await.target();
        let mr = self.modrinth_targets.borrow().await.target();

        if let Some(target) = local {
            result.push(target.entity_id);
        }

        if let Some(target) = cf {
            result.push(target.entity_id);
        }

        if let Some(target) = mr {
            result.push(target.entity_id);
        }

        result
    }
}

#[derive(Clone)]
struct UpdateNotifier {
    target: Arc<AtomicI32>,
    sender: Arc<watch::Sender<()>>,
}

impl UpdateNotifier {
    fn send(&self, _entity_id: CacheEntityId) {
        // let target = self.target.load(atomic::Ordering::SeqCst);

        // if target == entity_id {
        let _ = self.sender.send(());
        // }
    }
}

/// Variant of watch where both sides are simultaneously senders and receivers.
struct LockNotify<T: Send + Sync> {
    lock: Arc<RwLock<T>>,
    notify: Arc<watch::Sender<()>>,
    notify_rx: watch::Receiver<()>,
}

impl<T: Send + Sync> Clone for LockNotify<T> {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock.clone(),
            notify: self.notify.clone(),
            notify_rx: self.notify_rx.clone(),
        }
    }
}

impl<T: Send + Sync> LockNotify<T> {
    fn new(value: T) -> Self {
        let (notify, notify_rx) = watch::channel::<()>(());

        Self {
            lock: Arc::new(RwLock::new(value)),
            notify: Arc::new(notify),
            notify_rx,
        }
    }

    async fn send_modify(&self, f: impl FnOnce(&mut T) -> bool) {
        let mut lock = self.lock.write().await;

        if f(&mut *lock) {
            let _ = self.notify.send(());
        }
    }

    async fn send_modify_always(&self, f: impl FnOnce(&mut T)) {
        self.send_modify(|v| {
            f(v);
            true
        })
        .await;
    }

    async fn send(&self, value: T) {
        self.send_modify(|v| {
            *v = value;
            true
        })
        .await;
    }

    async fn send_silent(&self, value: T) {
        self.send_modify(|v| {
            *v = value;
            false
        })
        .await;
    }

    async fn borrow(&self) -> RwLockReadGuard<T> {
        self.lock.read().await
    }

    /// Note: will hang forever if all senders drop
    async fn await_change(&mut self) {
        if self.notify_rx.changed().await.is_err() {
            warn!("LockNotify sender was dropped, halting forever");
            futures::future::pending::<()>().await;
        }
    }
}

trait CompletionSender: Send + Sync {
    fn complete(self: Box<Self>, result: anyhow::Result<()>);
}

impl<F: FnOnce(anyhow::Result<()>) + Send + Sync> CompletionSender for F {
    fn complete(self: Box<Self>, result: anyhow::Result<()>) {
        self(result);
    }
}

struct CacheTargets {
    backend_override: Option<CacheTarget>,
    priority: Option<CacheTarget>,
    waiting: VecDeque<CacheEntityId>,
}

struct CacheTarget {
    entity_id: CacheEntityId,
    callback: Option<Box<dyn CompletionSender>>,
}

struct CacheTargetInfo {
    entity_id: CacheEntityId,
    is_override: bool,
    is_priority: bool,
}

impl CacheTargets {
    fn new() -> Self {
        Self {
            backend_override: None,
            priority: None,
            waiting: VecDeque::new(),
        }
    }

    fn target(&self) -> Option<CacheTargetInfo> {
        match self {
            Self {
                backend_override: Some(CacheTarget { entity_id, .. }),
                priority,
                waiting: _,
            } => Some(CacheTargetInfo {
                entity_id: *entity_id,
                is_override: true,
                is_priority: priority.as_ref().is_some_and(|v| *entity_id == v.entity_id),
            }),
            Self {
                backend_override: None,
                priority: Some(CacheTarget { entity_id, .. }),
                waiting: _,
            } => Some(CacheTargetInfo {
                entity_id: *entity_id,
                is_override: false,
                is_priority: true,
            }),
            Self {
                backend_override: None,
                priority: None,
                waiting,
            } => waiting.front().map(|entity_id| CacheTargetInfo {
                entity_id: *entity_id,
                is_override: false,
                is_priority: false,
            }),
        }
    }

    fn release_target(&mut self, entity_id: CacheEntityId, r: anyhow::Result<()>) -> bool {
        let mut changed = false;

        let check_target_callback = |target: &mut CacheTarget| {
            if target.entity_id == entity_id {
                if let Some(callback) = target.callback.take() {
                    callback.complete(
                        r.as_ref()
                            .map(|_| ())
                            .map_err(|_| anyhow!("error caching mods for entity")),
                    );
                }

                true
            } else {
                false
            }
        };

        let release_target = |target_option: &mut Option<CacheTarget>| {
            if let Some(target) = target_option {
                if check_target_callback(target) {
                    *target_option = None;

                    return true;
                }
            }

            false
        };

        changed |= release_target(&mut self.backend_override);
        changed |= release_target(&mut self.priority);

        let mut i = 0;
        while i < self.waiting.len() {
            if self.waiting[i] == entity_id {
                self.waiting.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }

        changed
    }

    // TODO: ensure this immediately cancels the target if running
    fn revoke_target(&mut self, entity_id: CacheEntityId) -> bool {
        let mut changed = false;

        let mut revoke_option = |target_option: &mut Option<CacheTarget>| {
            if let Some(target) = target_option {
                if target.entity_id == entity_id {
                    if let Some(callback) = target.callback.take() {
                        callback.complete(Err(anyhow!("This cache target was revoked")));
                    }

                    *target_option = None;
                    changed = true;
                }
            }
        };

        revoke_option(&mut self.backend_override);
        revoke_option(&mut self.priority);

        let mut i = 0;
        while i < self.waiting.len() {
            if self.waiting[i] == entity_id {
                self.waiting.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }

        changed
    }

    fn cancel_priority(&mut self) {
        if let Some(old) = self.priority.take() {
            if let Some(callback) = old.callback {
                callback.complete(Err(anyhow!("Caching priority was lost")));
            }
        }
    }

    fn cancel_override(&mut self) {
        if let Some(old) = self.backend_override.take() {
            if let Some(callback) = old.callback {
                callback.complete(Err(anyhow!("Backend override was canceled")));
            }
        }
    }

    fn set_priority(&mut self, target: CacheTarget) {
        self.cancel_priority();
        self.priority = Some(target);
    }

    fn set_override(&mut self, target: CacheTarget) {
        // Coalesce overrides for the same entity. Otherwise a second waiter
        // (e.g. dependency mod + parent mod racing to wait on the same
        // instance's cache pass) would cancel the first one's callback,
        // failing the in-flight install with "Backend override was canceled".
        if let Some(existing) = self.backend_override.as_mut() {
            if existing.entity_id == target.entity_id {
                let prior = existing.callback.take();
                let new = target.callback;
                existing.callback = Some(Box::new(move |r: anyhow::Result<()>| {
                    let r2 = match &r {
                        Ok(()) => Ok(()),
                        Err(e) => Err(anyhow!("{e:#}")),
                    };
                    if let Some(cb) = prior {
                        cb.complete(r);
                    }
                    if let Some(cb) = new {
                        cb.complete(r2);
                    }
                }));
                return;
            }
        }
        self.cancel_override();
        self.backend_override = Some(target);
    }

    fn get_queue_position(&self, entity_id: CacheEntityId) -> Option<usize> {
        self.waiting.iter().position(|&id| id == entity_id)
    }

    fn get_queue_length(&self) -> usize {
        self.waiting.len()
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

impl LoopValue for CacheTargets {
    type Token = Option<CacheEntityId>;
    type Value = CacheTargetInfo;

    fn token(&self) -> Self::Token {
        self.target().map(|target| target.entity_id)
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)> {
        self.target().map(|target| {
            let entity_id = target.entity_id;
            (target, token == Some(entity_id))
        })
    }
}

impl LoopValue for Option<CacheEntityId> {
    type Token = Self;
    type Value = CacheEntityId;

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
            self.watcher.await_change().await;

            let watcher = self.watcher.borrow().await;
            if let Some(v) = watcher.loop_cmp(self.token) {
                self.token = watcher.token();
                return Some(v);
            }
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
    const NAME: &'static str;
    type SaveBundle: Send + Sync;

    async fn query_platform(
        app: &App,
        entity_id: CacheEntityId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()>;

    async fn save_batch(app: &App, entity_id: CacheEntityId, batch: Self::SaveBundle);

    async fn cache_icons(app: &App, entity_id: CacheEntityId, update_notifier: &UpdateNotifier);
}

type ModplatformCacheBundle<T> = (CacheEntityId, bool, Option<T>, Option<oneshot::Sender<()>>);

struct BundleSender<'a, T> {
    should_wait: bool,
    entity_id: CacheEntityId,
    update_images: bool,
    active_wait: Option<oneshot::Receiver<()>>,
    sender: &'a mpsc::UnboundedSender<ModplatformCacheBundle<T>>,
}

impl<'a, T> BundleSender<'a, T> {
    fn new(
        entity_id: CacheEntityId,
        wait: bool,
        update_images: bool,
        sender: &'a mpsc::UnboundedSender<ModplatformCacheBundle<T>>,
    ) -> Self {
        Self {
            entity_id,
            should_wait: wait,
            update_images,
            active_wait: None,
            sender,
        }
    }

    fn send(&mut self, bundle: T) {
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
            .send((self.entity_id, self.update_images, Some(bundle), tx));
    }

    async fn wait(self) {
        match self.active_wait {
            Some(wait) => {
                let _ = wait.await;
            }
            None => {
                if self.update_images {
                    let _ = self
                        .sender
                        .send((self.entity_id, self.update_images, None, None));
                }
            }
        }
    }
}

fn cache_modplatform<C: ModplatformCacher>(
    app: App,
    rx: LockNotify<CacheTargets>,
    update_notifier: UpdateNotifier,
) {
    tokio::spawn(async move {
        let app = &app;
        let update_notifier = &update_notifier;

        let (batch_tx, mut batch_rx) =
            mpsc::unbounded_channel::<ModplatformCacheBundle<C::SaveBundle>>();
        let image_rx = LockNotify::<Option<CacheEntityId>>::new(None);
        let image_tx = image_rx.clone();

        let batch_tx = &batch_tx;
        let mut query_loop_watcher = LoopWatcher::new(rx).await;
        let query_loop = query_loop_watcher.loop_interrupt(
            |CacheTargetInfo {
                entity_id,
                is_priority,
                is_override,
            }| async move {
                let mut pause = app.meta_cache_manager().pause_caching.subscribe();
                let r = loop {
                    let wait_for_pause = async {
                        loop {
                            if *pause.borrow() {
                                break;
                            }

                            if pause.changed().await.is_err() {
                                futures::future::pending().await
                            }
                        }
                    };

                    let do_caching = async {
                        let entity_name = match entity_id {
                            CacheEntityId::Instance(instance_id) => {
                                let instance_manager = app.instance_manager();
                                let instances = instance_manager.instances.read().await;
                                instances.get(&instance_id)
                                    .map(|instance| instance.shortpath.clone())
                                    .unwrap_or_else(|| format!("Instance {}", instance_id.0))
                            }
                            CacheEntityId::Server(server_id) => format!("Server {}", server_id),
                        };

                        let task = VisualTask::new(match C::NAME {
                            "curseforge" => Translation::CacheTaskCurseForge { instance_name: entity_name.clone() },
                            "modrinth" => Translation::CacheTaskModrinth { instance_name: entity_name.clone() },
                            _ => Translation::CacheTaskLocal { instance_name: entity_name.clone() }, // fallback
                        });
                        let _task_id = app.task_manager().spawn_task(&task).await;

                        info!({ is_priority, is_override }, "Starting {} mod caching for {}", C::NAME, entity_id);

                        let platform_subtask = task.subtask(Translation::CacheSubtaskQueryingPlatform {
                            platform: C::NAME.to_string(),
                        });
                        platform_subtask.start_opaque();

                        // true could be optimized to "if there is a callback" if this is a bottleneck
                        let mut sender = BundleSender::new(entity_id, true, is_priority, batch_tx);
                        let r = C::query_platform(&app, entity_id, &mut sender).await;

                        if r.is_ok() {
                            platform_subtask.complete_opaque();
                        }

                        if let Err(e) = &r {
                            tracing::error!({ error = ?e }, "Could not query {} mod metadata for {}", C::NAME, entity_id);
                        } else {
                            info!("Completed {} mod caching for {}", C::NAME, entity_id);
                        }

                        sender.wait().await;

                        r
                    };

                    tokio::select! {
                        _ = wait_for_pause => {
                            info!("Remote {} mod caching paused for {entity_id} - waiting for unpause", C::NAME);

                            // wait for unpause
                            loop {
                                if !*pause.borrow() {
                                    info!("Remote {} mod caching unpaused for {entity_id} - resuming", C::NAME);
                                    break;
                                }

                                if pause.changed().await.is_err() {
                                    futures::future::pending().await
                                }
                            }
                        },
                        r = do_caching => break r,
                    };
                };

                move |targets: &mut CacheTargets| targets.release_target(entity_id, r)
            },
        );

        let save_loop = async {
            while let Some((entity_id, update_images, bundle, notify)) = batch_rx.recv().await {
                if let Some(bundle) = bundle {
                    debug!(
                        "Saving {} mod cache update bundle for {}",
                        C::NAME,
                        entity_id
                    );
                    C::save_batch(&app, entity_id, bundle).await;

                    if let Some(notify) = notify {
                        let _ = notify.send(());
                    }

                    let _ = update_notifier.send(entity_id);
                }

                if update_images {
                    image_tx.send(Some(entity_id)).await;
                }
            }
        };

        let mut image_loop_watcher = LoopWatcher::new(image_rx).await;
        let image_loop = image_loop_watcher.loop_interrupt(|entity_id| async move {
            info!("Starting {} mod icon caching for {}", C::NAME, entity_id);

            C::cache_icons(&app, entity_id, &update_notifier).await;
            info!("Completed {} mod icon caching for {}", C::NAME, entity_id);

            |_: &mut Option<CacheEntityId>| false
        });

        // None of the futures should ever exit.
        // This join polls both while allowing them to share variables in this scope.
        futures::join!(query_loop, save_loop, image_loop);
    });
}

impl ManagerRef<'_, MetaCacheManager> {
    pub async fn instance_removed(self, instance_id: InstanceId) {
        let entity_id = CacheEntityId::Instance(instance_id);
        join!(
            self.local_targets
                .send_modify(|targets| targets.revoke_target(entity_id)),
            self.curseforge_targets
                .send_modify(|targets| targets.revoke_target(entity_id)),
            self.modrinth_targets
                .send_modify(|targets| targets.revoke_target(entity_id)),
        );

        let instance_id_val = *instance_id;
        let _ = mfcdb::delete_mod_file_cache_by_instance(&self.app.db, instance_id_val).await;

        self.gc_mod_metadata().await;
    }

    pub async fn gc_mod_metadata(self) {
        let _ = metarepo::gc_orphan_metadata(&self.app.db)
            .await;
    }

    // this will need further refactoring. left for later.
    pub async fn cache_with_priority(self, entity_id: CacheEntityId) {
        let app = self.app.clone();

        // todo: trace scanned instances, but not here as we also need to account for waiting instances.
        self.local_targets
            .send_modify_always(move |targets| {
                targets.set_priority(CacheTarget {
                    entity_id,
                    callback: Some(Box::new(move |r: anyhow::Result<()>| {
                        if r.is_ok() {
                            tokio::spawn(async move {
                                let mcm = app.meta_cache_manager();

                                join!(
                                    mcm.curseforge_targets.send_modify_always(move |targets| {
                                        targets.set_priority(CacheTarget {
                                            entity_id,
                                            callback: None,
                                        })
                                    }),
                                    mcm.modrinth_targets.send_modify_always(move |targets| {
                                        targets.set_priority(CacheTarget {
                                            entity_id,
                                            callback: None,
                                        })
                                    })
                                );
                            });
                        }
                    })),
                });
            })
            .await;
    }

    pub async fn override_caching_and_wait(
        self,
        entity_id: CacheEntityId,
        curseforge: bool,
        modrinth: bool,
    ) -> anyhow::Result<()> {
        tracing::info!("Overriding caching and waiting for {entity_id}");

        let app = self.app.clone();

        let split = |c| match c {
            Some((tx, rx)) => (Some(tx), Some(rx)),
            None => (None, None),
        };

        let (local_tx, local_rx) = oneshot::channel::<anyhow::Result<()>>();
        let (cf_tx, cf_rx) = split(curseforge.then(|| oneshot::channel::<anyhow::Result<()>>()));
        let (mr_tx, mr_rx) = split(modrinth.then(|| oneshot::channel::<anyhow::Result<()>>()));

        self.local_targets
            .send_modify_always(move |targets| {
                targets.set_override(CacheTarget {
                    entity_id,
                    callback: Some(Box::new(move |r: anyhow::Result<()>| match r {
                        Ok(()) => {
                            let _ = local_tx.send(Ok(()));

                            tokio::spawn(async move {
                                let mcm = app.meta_cache_manager();

                                let cf = cf_tx.map(|tx| {
                                    mcm.curseforge_targets.send_modify_always(move |targets| {
                                        targets.set_override(CacheTarget {
                                            entity_id,
                                            callback: Some(Box::new(
                                                move |r: anyhow::Result<()>| {
                                                    let _ = tx.send(r);
                                                },
                                            )),
                                        })
                                    })
                                });

                                let mr = mr_tx.map(|tx| {
                                    mcm.modrinth_targets.send_modify_always(move |targets| {
                                        targets.set_override(CacheTarget {
                                            entity_id,
                                            callback: Some(Box::new(
                                                move |r: anyhow::Result<()>| {
                                                    let _ = tx.send(r);
                                                },
                                            )),
                                        })
                                    })
                                });

                                join!(
                                    async {
                                        if let Some(cf) = cf {
                                            cf.await;
                                        }
                                    },
                                    async {
                                        if let Some(mr) = mr {
                                            mr.await;
                                        }
                                    }
                                );
                            });
                        }
                        e @ Err(_) => {
                            let _ = local_tx.send(e);
                        }
                    })),
                });
            })
            .await;

        local_rx.await??;

        if let Some(rx) = cf_rx {
            rx.await??;
        }

        if let Some(rx) = mr_rx {
            rx.await??;
        }

        tracing::info!("Overriding caching and waiting for {entity_id} done");

        Ok(())
    }

    pub async fn watch_and_prioritize(self, entity_id: Option<CacheEntityId>) {
        match entity_id {
            Some(id) => {
                info!("Switching cache priority to {id}");
                let _ = self.watched_entity.send(entity_id);
                self.cache_with_priority(id).await;
            }
            None => {
                info!("Clearing cache priority - no entity being watched");
                let _ = self.watched_entity.send(entity_id);
            }
        }
    }

    pub async fn queue_caching(self, entity_id: CacheEntityId, _force: bool) {
        // TODO: make track scanned instances for _force
        info!("Queuing mod caching for {}", entity_id);

        self.local_targets
            .send_modify_always(|targets| {
                targets.waiting.push_back(entity_id);
            })
            .await;
    }

    pub async fn launch_background_tasks(self) {
        let app_pause = self.app.clone();
        tokio::spawn(async move {
            let mut any_instance_changed_watcher = app_pause
                .instance_manager()
                .any_instance_running
                .subscribe();

            loop {
                let any_instance_running = *any_instance_changed_watcher.borrow();

                if any_instance_running {
                    info!("Pausing mod caching - instance is running");
                } else {
                    info!("Resuming mod caching - no instances running");
                }

                app_pause
                    .meta_cache_manager()
                    .pause_caching
                    .send_replace(any_instance_running);

                if any_instance_changed_watcher.changed().await.is_err() {
                    break;
                }
            }
        });

        let (list_debounce_tx, mut list_debounce_rx) = watch::channel(());

        let debounce_target = Arc::new(AtomicI32::new(-1));
        let debounce_notifier = UpdateNotifier {
            target: debounce_target.clone(),
            sender: Arc::new(list_debounce_tx),
        };

        let app_debounce = self.app.clone();
        let mut debounce_watch_rx = self.watched_entity.subscribe();
        tokio::spawn(async move {
            // wait until watched is some, then wait until we get a list debounce that matches.
            // Then wait 2 seconds, interrupted if the watch changes.
            // note: the various `return`s will only be hit if the cache manager is dropped somehow. they prevent a spinloop.
            loop {
                let watched = *debounce_watch_rx.borrow();

                debounce_target.store(
                    match watched {
                        Some(CacheEntityId::Instance(id)) => *id,
                        Some(CacheEntityId::Server(id)) => id,
                        None => -1,
                    },
                    atomic::Ordering::SeqCst,
                );

                let Some(watched) = watched else {
                    if debounce_watch_rx.changed().await.is_err() {
                        return;
                    }

                    continue;
                };

                tokio::select! {
                    _ = list_debounce_rx.changed() => {
                        match watched {
                            CacheEntityId::Instance(id) => app_debounce.invalidate(INSTANCE_MODS, Some(id.0.into())),
                            CacheEntityId::Server(id) => app_debounce.invalidate(GET_SERVER_ADDONS, Some(id.into())),
                        }

                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    },
                    r = debounce_watch_rx.changed() => {
                        if r.is_err() {
                            return;
                        }
                    },
                };
            }
        });

        cache_local(
            self.app.clone(),
            self.local_targets.clone(),
            debounce_notifier.clone(),
        );
        cache_modplatform::<CurseforgeModCacher>(
            self.app.clone(),
            self.curseforge_targets.clone(),
            debounce_notifier.clone(),
        );
        cache_modplatform::<ModrinthModCacher>(
            self.app.clone(),
            self.modrinth_targets.clone(),
            debounce_notifier,
        );
    }

    /// Cache a mod file without first checking the validity of the instance
    /// Hash a mod file and parse its JAR metadata. Entity-independent.
    async fn hash_and_parse_mod_file(
        &self,
        mods_dir_path: &PathBuf,
        mod_filename: &str,
        enabled: bool,
    ) -> anyhow::Result<ModFileParseResult> {
        let mut path = mods_dir_path.join(mod_filename);

        let prev_ext = path
            .extension()
            .and_then(OsStr::to_str)
            .ok_or(anyhow!("mod file `{}` has no extension", mod_filename))?;

        if !enabled {
            path.set_extension(format!("{prev_ext}.disabled"));
        }

        let mut file = tokio::fs::File::open(path).await?;

        let mut sha512 = Sha512::new();
        let mut sha1 = Sha1::new();
        let mut murmur_len = 0;
        let mut content_len = 0;

        carbon_scheduler::buffered_digest(&mut file, |chunk| {
            sha512.update(&chunk);
            sha1.update(&chunk);
            murmur_len += chunk
                .iter()
                .filter(|&&x| x != 9 && x != 10 && x != 13 && x != 32)
                .count();
            content_len += chunk.len();
        })
        .await?;

        let sha512: [u8; 64] = sha512.finalize().into();
        let sha1: [u8; 20] = sha1.finalize().into();

        file.seek(SeekFrom::Start(0)).await?;
        let mut file = file.into_std().await;

        let (file, meta, image_data) = tokio::task::spawn_blocking(|| {
            let meta = super::mods::parse_metadata(&mut file);

            let image_data = match &meta
                .as_ref()
                .map(|m| m.as_ref().map(|m| m.logo_file.as_ref()))
            {
                Ok(Some(Some(logo_file))) => {
                    let mut zip = zip::ZipArchive::new(&mut file).unwrap();
                    let r = match zip.by_name(&logo_file) {
                        Ok(mut file) => {
                            let mut image = Vec::with_capacity(file.size() as usize);
                            file.read_to_end(&mut image)?;
                            Some(image)
                        }
                        _ => None,
                    };

                    r
                }
                _ => None,
            };

            Ok::<_, anyhow::Error>((file, meta, image_data))
        })
        .await??;

        let mut file = tokio::fs::File::from_std(file);
        file.seek(SeekFrom::Start(0)).await?;
        let mut murmur2 = Murmur2Digest::new(murmur_len as u32);

        let mut workbuf = Vec::<u8>::with_capacity(carbon_scheduler::BUFSIZE);

        carbon_scheduler::buffered_digest(&mut file, |chunk| {
            workbuf.splice(.., chunk.iter().map(|&b| b));
            workbuf.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
            murmur2.update(&workbuf[..]);
        })
        .await?;

        let murmur2 = murmur2.finalize();

        drop(file);

        let meta = match meta {
            Ok(meta) => meta,
            Err(e) => {
                debug!({ error = ?e }, "could not parse mod metadata for {}", mod_filename);
                None
            }
        };

        Ok(ModFileParseResult {
            sha512,
            sha1,
            murmur2,
            content_len,
            meta,
            image_data,
        })
    }

    /// Build a parse result for a world directory. Worlds are directories, not
    /// archives, so they get stable name-derived hashes instead of content
    /// hashes, letting rescans dedupe to the same metadata row.
    async fn world_dir_parse_result(
        &self,
        worlds_dir_path: &Path,
        dir_name: &str,
    ) -> anyhow::Result<ModFileParseResult> {
        let dir_meta = tokio::fs::metadata(worlds_dir_path.join(dir_name)).await?;

        let mut sha512 = Sha512::new();
        sha512.update(b"gdl-world-dir:");
        sha512.update(dir_name.as_bytes());

        let mut sha1 = Sha1::new();
        sha1.update(b"gdl-world-dir:");
        sha1.update(dir_name.as_bytes());

        Ok(ModFileParseResult {
            sha512: sha512.finalize().into(),
            sha1: sha1.finalize().into(),
            murmur2: 0,
            content_len: dir_meta.len() as usize,
            meta: Some(super::mods::ModFileMetadata {
                modid: None,
                name: Some(dir_name.to_string()),
                version: None,
                description: None,
                authors: None,
                modloaders: Vec::new(),
                logo_file: None,
            }),
            image_data: None,
        })
    }

    /// Find or create a ModMetadata row for the given parse result. Entity-independent.
    async fn ensure_mod_metadata(
        &self,
        result: &ModFileParseResult,
        mod_filename: &str,
    ) -> anyhow::Result<String> {
        let sha512 = Vec::from(result.sha512);
        let murmur2 = result.murmur2 as i32;

        let dbmeta = metarepo::find_metadata_by_hashes(&self.app.db, &sha512, murmur2).await?;

        let meta_id = match dbmeta {
            Some(meta) => meta.id,
            None => {
                let meta_id = Uuid::new_v4().to_string();

                // Scale the icon (if any) before touching the database — the
                // insert runs on the writer thread and cannot await.
                let logo_data: Option<Vec<u8>> = match &result.image_data {
                    Some(image_data) => {
                        let permit = self
                            .image_scale_semaphore
                            .acquire()
                            .await
                            .expect("the image scale semaphore is never closed");

                        let image_data = image_data.clone();
                        let logo = carbon_scheduler::cpu_block(move || {
                            let scaled = scale_mod_image(&image_data[..])?;
                            Ok::<_, anyhow::Error>(Some(scaled))
                        })
                        .await;

                        drop(permit);

                        match logo {
                            Ok(scaled) => scaled,
                            Err(e) => {
                                error!({ error = ?e }, "could not scale mod icon for {}", mod_filename);
                                None
                            }
                        }
                    }
                    None => None,
                };

                let sha1 = Vec::from(result.sha1);
                let modloaders = result
                    .meta
                    .as_ref()
                    .map(|meta| &meta.modloaders)
                    .map(|modloaders| modloaders.iter().map(ToString::to_string).join(","))
                    .unwrap_or(String::new());
                let (name, modid, version, description, authors) = match &result.meta {
                    Some(meta) => (
                        meta.name.clone(),
                        meta.modid.clone(),
                        meta.version.clone(),
                        meta.description.clone(),
                        meta.authors.clone(),
                    ),
                    None => (None, None, None, None, None),
                };

                let meta_id_owned = meta_id.clone();
                // Interleaved app logic: insert the metadata row and, only when
                // a logo was scaled, its local image. Runs in one writer
                // dispatch, so no other write interleaves; not one transaction
                // (each autocommits). Uses `_conn` forms inside the closure.
                self.app
                    .db
                    .write(move |conn| {
                        metarepo::insert_metadata_conn(
                            &conn,
                            &meta_id_owned,
                            murmur2,
                            &sha512,
                            &sha1,
                            &modloaders,
                            name.as_deref(),
                            modid.as_deref(),
                            version.as_deref(),
                            description.as_deref(),
                            authors.as_deref(),
                            DbDateTime(Utc::now().fixed_offset()),
                        )?;
                        if let Some(data) = logo_data {
                            metarepo::insert_local_image_conn(&conn, &meta_id_owned, &data)?;
                        }
                        Ok(())
                    })
                    .await?;

                meta_id
            }
        };

        Ok(meta_id)
    }

    /// Cache a mod file for an instance. Hashes, parses metadata, and upserts into mod_file_cache.
    async fn cache_mod_file_unchecked(
        self,
        instance_id: InstanceId,
        mods_dir_path: &PathBuf,
        mod_filename: String,
        enabled: bool,
        addon_type: String,
    ) -> anyhow::Result<String> {
        let is_world = crate::domain::instance::AddonType::from_db_string(&addon_type)
            == Some(crate::domain::instance::AddonType::Worlds);

        let result = if is_world {
            self.world_dir_parse_result(mods_dir_path, &mod_filename)
                .await?
        } else {
            self.hash_and_parse_mod_file(mods_dir_path, &mod_filename, enabled)
                .await?
        };
        let meta_id = self.ensure_mod_metadata(&result, &mod_filename).await?;

        let instance_id_val = *instance_id;
        let filename_owned = mod_filename.to_string();
        let filesize = result.content_len as i32;
        let addon_type_owned = addon_type.clone();
        let meta_id_owned = meta_id.clone();
        mfcdb::upsert_mod_file_cache(
            &self.app.db,
            instance_id_val,
            filename_owned,
            filesize,
            enabled,
            addon_type_owned,
            meta_id_owned,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        Ok(meta_id)
    }

    /// Cache a mod file for a server. Hashes, parses metadata, and upserts into server_mod_file_cache.
    async fn cache_server_mod_file_unchecked(
        self,
        server_id: i32,
        mods_dir_path: &PathBuf,
        mod_filename: String,
        enabled: bool,
        addon_type: String,
    ) -> anyhow::Result<String> {
        let result = self
            .hash_and_parse_mod_file(mods_dir_path, &mod_filename, enabled)
            .await?;
        let meta_id = self.ensure_mod_metadata(&result, &mod_filename).await?;

        let filename_owned = mod_filename.to_string();
        let filesize = result.content_len as i32;
        let addon_type_owned = addon_type.clone();
        let meta_id_owned = meta_id.clone();
        mfcdb::upsert_server_mod_file_cache(
            &self.app.db,
            server_id,
            filename_owned,
            filesize,
            enabled,
            addon_type_owned,
            meta_id_owned,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;

        Ok(meta_id)
    }

    /// Cache all mod files for a server. Scans filesystem, hashes new/changed files,
    /// and stores results in server_mod_file_cache.
    pub async fn cache_server_local(
        self,
        server_id: i32,
        server_shortpath: &str,
    ) -> anyhow::Result<()> {
        let runtime_path = &self.app.settings_manager().runtime_path;
        let server_path = runtime_path.get_servers().get_server_path(server_shortpath);

        // Get existing cache entries
        let cached = mfcdb::get_server_mod_files_by_server(&self.app.db, server_id)
            .await?;

        let cached_map: HashMap<String, (i32, bool)> = cached
            .iter()
            .map(|c| (c.filename.clone(), (c.filesize, c.enabled)))
            .collect();

        // Scan filesystem
        let mut disk_files: Vec<(String, String, bool)> = Vec::new(); // (base_filename, addon_type, enabled)

        let mods_path = server_path.get_mods_path();
        if mods_path.exists() {
            let mut entries = tokio::fs::read_dir(&mods_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let filename = entry.file_name().to_string_lossy().to_string();
                if filename.ends_with(".jar") || filename.ends_with(".jar.disabled") {
                    let enabled = !filename.ends_with(".disabled");
                    let base_filename = filename.trim_end_matches(".disabled").to_string();
                    disk_files.push((base_filename, "mods".to_string(), enabled));
                }
            }
        }

        let datapacks_path = server_path.get_datapacks_path();
        if datapacks_path.exists() {
            let mut entries = tokio::fs::read_dir(&datapacks_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let filename = entry.file_name().to_string_lossy().to_string();
                if filename.ends_with(".zip") || filename.ends_with(".zip.disabled") {
                    let enabled = !filename.ends_with(".disabled");
                    let base_filename = filename.trim_end_matches(".disabled").to_string();
                    disk_files.push((base_filename, "datapacks".to_string(), enabled));
                }
            }
        }

        let disk_filenames: HashSet<String> =
            disk_files.iter().map(|(f, _, _)| f.clone()).collect();

        // Delete stale cache entries (files no longer on disk)
        let stale: Vec<_> = cached
            .iter()
            .filter(|c| !disk_filenames.contains(&c.filename))
            .collect();

        for entry in &stale {
            let entry_id = entry.id.clone();
            let _ = mfcdb::delete_server_mod_file_cache_by_id(&self.app.db, &entry_id)
                .await;
        }

        // Cache new/changed files
        for (base_filename, addon_type, enabled) in &disk_files {
            // Check if already cached with same size
            if let Some((cached_size, cached_enabled)) = cached_map.get(base_filename) {
                // Get current file size
                let dir = if addon_type == "mods" {
                    &mods_path
                } else {
                    &datapacks_path
                };
                let actual_filename = if *enabled {
                    base_filename.clone()
                } else {
                    format!("{}.disabled", base_filename)
                };
                let file_size = tokio::fs::metadata(dir.join(&actual_filename))
                    .await
                    .map(|m| m.len() as i32)
                    .unwrap_or(0);

                if *cached_size == file_size && *cached_enabled == *enabled {
                    continue; // Skip unchanged file
                }
            }

            let dir = if addon_type == "mods" {
                mods_path.clone()
            } else {
                datapacks_path.clone()
            };

            if let Err(e) = self
                .cache_server_mod_file_unchecked(
                    server_id,
                    &dir,
                    base_filename.clone(),
                    *enabled,
                    addon_type.clone(),
                )
                .await
            {
                warn!("Failed to cache server addon '{}': {}", base_filename, e);
            }
        }

        info!("Completed local caching for server {}", server_id);
        Ok(())
    }

    /// Check if a specific entity is currently being cached
    pub async fn is_entity_being_cached(&self, entity_id: CacheEntityId) -> bool {
        let currently_caching = self.get_currently_caching_entities().await;
        currently_caching.contains(&entity_id)
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
    target.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
    Ok(output)
}

fn cache_local(app: App, rx: LockNotify<CacheTargets>, update_notifier: UpdateNotifier) {
    tokio::spawn(async move {
        let app = &app;
        let update_notifier = &update_notifier;

        let cache_instance = |instance_id: InstanceId| async move {
            let app2 = app.clone();
            let cached_entries = tokio::spawn(async move {
                let instance_id_val = *instance_id;
                mfcdb::get_mod_files_by_instance(&app2.db, instance_id_val).await
            });

            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let Some(instance) = instances.get(&instance_id) else {
                error!("invalid instance id {instance_id} for mod scanning");
                return Ok(());
            };

            let instance_path = InstancesPath::subpath().get_instance_path(&instance.shortpath);
            let instance_name = instance.shortpath.clone();

            drop(instances);

            let task = VisualTask::new(Translation::CacheTaskLocal {
                instance_name: instance_name.clone(),
            });
            let task_id = app.task_manager().spawn_task(&task).await;

            info!("Starting local mod caching for instance {}", instance_id);

            let root_path = app.settings_manager().runtime_path.get_root().to_path();

            let mut modpaths = HashMap::<String, (bool, u64, String)>::new();
            let mut total_files_scanned = 0;

            let addon_types = crate::domain::instance::AddonType::all();
            let scanning_subtask = task.subtask(Translation::CacheSubtaskScanningFiles);
            scanning_subtask.update_items(0, addon_types.len() as u32);

            let mut addon_count = 0;
            for addon_type in crate::domain::instance::AddonType::all() {
                let addon_subpath = addon_type.get_folder_path(&instance_path);
                let mut pathbuf = PathBuf::new();
                pathbuf.push(&root_path);
                pathbuf.push(&addon_subpath);

                if !pathbuf.is_dir() {
                    debug!(
                        "Skipping {:?} directory for instance {} - does not exist: {}",
                        addon_type,
                        instance_id,
                        pathbuf.display()
                    );
                    continue;
                }

                debug!(
                    "Scanning {:?} directory for instance {}: {}",
                    addon_type,
                    instance_id,
                    pathbuf.display()
                );

                let mut entries = match tokio::fs::read_dir(&pathbuf).await {
                    Ok(entries) => entries,
                    Err(e) => {
                        error!({ dir = ?pathbuf, error = ?e }, "could not read {addon_type:?} folder for instance {instance_id}");
                        continue;
                    }
                };

                while let Ok(Some(entry)) = entries.next_entry().await {
                    let file_name = entry.file_name();
                    let Some(mut utf8_name) = file_name.to_str() else {
                        continue;
                    };

                    let allowed_extensions = match addon_type {
                        crate::domain::instance::AddonType::Mods => vec![".jar", ".zip"],
                        crate::domain::instance::AddonType::ResourcePacks => vec![".zip"],
                        crate::domain::instance::AddonType::Shaders => vec![".zip"],
                        crate::domain::instance::AddonType::DataPacks => vec![".zip"],
                        crate::domain::instance::AddonType::Worlds => vec![], // Worlds are directories, handled differently
                    };

                    if addon_type == crate::domain::instance::AddonType::Worlds {
                        let Ok(metadata) = entry.metadata().await else {
                            continue;
                        };
                        if !metadata.is_dir() {
                            continue;
                        }

                        modpaths.insert(
                            utf8_name.to_string(),
                            (true, metadata.len(), addon_type.to_db_string().to_string()),
                        );
                        total_files_scanned += 1;
                    } else {
                        let allowed_base_ext = allowed_extensions
                            .iter()
                            .any(|&ext| utf8_name.ends_with(ext));
                        let disabled_extensions: Vec<String> = allowed_extensions
                            .iter()
                            .map(|ext| format!("{}.disabled", ext))
                            .collect();
                        let allowed_disabled_ext = disabled_extensions
                            .iter()
                            .any(|ext| utf8_name.ends_with(ext));

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

                        modpaths.insert(
                            utf8_name.to_string(),
                            (
                                !allowed_disabled_ext,
                                metadata.len(),
                                addon_type.to_db_string().to_string(),
                            ),
                        );
                        total_files_scanned += 1;
                    }
                }

                addon_count += 1;
                scanning_subtask.update_items(addon_count, addon_types.len() as u32);
            }

            debug!(
                "File scanning complete for instance {}: found {} files total",
                instance_id, total_files_scanned
            );

            trace!({ modpaths = ?modpaths }, "modpaths found for instance {instance_id}");

            scanning_subtask.complete_items();

            let mut has_outdated_entries = false;

            if let Ok(Ok(cached_entries)) = cached_entries.await {
                has_outdated_entries = cached_entries.len() != modpaths.len();

                for entry in cached_entries {
                    if let Some((enabled, real_size, addon_type)) = modpaths.get(&entry.filename) {
                        if *real_size == entry.filesize as u64
                            && *enabled == entry.enabled
                            && *addon_type == entry.addon_type
                        {
                            modpaths.remove(&entry.filename);
                            continue;
                        }
                    } else {
                        let instance_id_val = *instance_id;
                        mfcdb::delete_mod_file_cache_by_instance_filename(
                            &app.db,
                            instance_id_val,
                            &entry.filename,
                        )
                        .await?;
                    }

                    has_outdated_entries = true;
                }
            }

            // Throttle concurrent file hashing to about half the available cores, with a
            // floor of 1 so single-core or cgroup-limited hosts still make progress; a value
            // of 0 would make the Semaphore::acquire below block forever.
            let default_parallelism_approx =
                (available_parallelism().map_or(1, |n| n.get()) / 2).max(1);

            let rate_limiter = Arc::new(tokio::sync::Semaphore::new(default_parallelism_approx));

            let total_files_to_process = modpaths.len() as u32;
            let finalization_subtask = if !modpaths.is_empty() {
                let subtask = task.subtask(Translation::CacheSubtaskFinalizingCache);
                subtask.update_items(0, total_files_to_process);
                Some(subtask)
            } else {
                None
            };

            let processed_count = Arc::new(AtomicI32::new(0));

            let entry_futures =
                modpaths
                    .into_iter()
                    .map(|(filename, (enabled, _, addon_type_str))| {
                        let instance_path = instance_path.clone();
                        let root_path = root_path.clone();
                        let update_notifier = &update_notifier;
                        let finalization_subtask = finalization_subtask.as_ref();
                        let processed_count_clone = Arc::clone(&processed_count);
                        let total_files = total_files_to_process;

                        let rate_limiter = Arc::clone(&rate_limiter);

                        async move {
                            let _permit = rate_limiter.acquire().await.expect("rate limiter");

                            let addon_type =
                                crate::domain::instance::AddonType::from_db_string(&addon_type_str)
                                    .unwrap_or(crate::domain::instance::AddonType::Mods);

                            let addon_subpath = addon_type.get_folder_path(&instance_path);
                            let mut pathbuf = PathBuf::new();
                            pathbuf.push(&root_path);
                            pathbuf.push(&addon_subpath);

                            let stored = match app
                                .meta_cache_manager()
                                .cache_mod_file_unchecked(
                                    instance_id,
                                    &pathbuf,
                                    filename.clone(),
                                    enabled,
                                    addon_type_str,
                                )
                                .await
                            {
                                Ok(_) => true,
                                Err(e) => {
                                    // One bad entry must not poison the rest of the scan.
                                    error!(
                                        { error = ?e },
                                        "could not store scan result for `{filename}` of instance {instance_id} in db"
                                    );
                                    false
                                }
                            };

                            if let Some(finalization_subtask) = finalization_subtask {
                                let current_count = processed_count_clone
                                    .fetch_add(1, atomic::Ordering::Relaxed)
                                    + 1;
                                finalization_subtask
                                    .update_items(current_count as u32, total_files);
                            }

                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                            if stored {
                                update_notifier.send(CacheEntityId::Instance(instance_id));
                            }

                            stored
                        }
                    });

            let results = futures::future::join_all(entry_futures).await;

            if let Some(finalization_subtask) = finalization_subtask {
                finalization_subtask.complete_items();
            }

            let success_count = results.into_iter().filter(|&stored| stored).count();

            if has_outdated_entries {
                let _ = update_notifier.send(CacheEntityId::Instance(instance_id));
            }

            info!(
                "Completed local mod caching for instance {}: scanned {} files, updated {} entries",
                instance_id, total_files_scanned, success_count
            );

            Ok(())
        };

        let cache_instance = &cache_instance;

        LoopWatcher::new(rx).await.loop_interrupt(
            |CacheTargetInfo {
                entity_id,
                is_override,
                is_priority,
            }| async move {
                let mut pause = app.meta_cache_manager().pause_caching.subscribe();
                let r = loop {
                    let wait_for_pause = async {
                        loop {
                            if *pause.borrow() {
                                break;
                            }

                            if pause.changed().await.is_err() {
                                futures::future::pending().await
                            }
                        }
                    };

                    let do_caching = async {
                        info!("Beginning local mod caching for {entity_id}");

                        let r = match entity_id {
                            CacheEntityId::Instance(instance_id) => {
                                cache_instance(instance_id).await
                            }
                            CacheEntityId::Server(server_id) => {
                                let db_server = carbon_repos::repos::server::get_server(&app.db, server_id)
                                    .await?
                                    .ok_or_else(|| anyhow!("Server {} not found", server_id))?;
                                app.meta_cache_manager()
                                    .cache_server_local(server_id, &db_server.shortpath)
                                    .await
                            }
                        };

                        if let Err(e) = &r {
                            tracing::error!({ error = ?e }, "Could not query local mod metadata for {entity_id}");
                        }

                        // waiting list targets cascade into curseforge and modrinth caching.
                        if !is_override && !is_priority {
                            info!("Cascading to platform caching for {}", entity_id);
                            let mcm = app.meta_cache_manager();

                            join!(
                                mcm.curseforge_targets.send_modify_always(|targets| targets.waiting.push_back(entity_id)),
                                mcm.modrinth_targets.send_modify_always(|targets| targets.waiting.push_back(entity_id)),
                            );
                        }

                        r
                    };

                    tokio::select! {
                        _ = wait_for_pause => {
                            info!("Local mod caching paused for {entity_id} - waiting for unpause");

                            // wait for unpause
                            loop {
                                if !*pause.borrow() {
                                    info!("Local mod caching unpaused for {entity_id} - resuming");
                                    break;
                                }

                                if pause.changed().await.is_err() {
                                    futures::future::pending().await
                                }
                            }
                        },
                        r = do_caching => break r,
                    };
                };

                move |targets: &mut CacheTargets| targets.release_target(entity_id, r)
            }
        ).await;
    });
}
