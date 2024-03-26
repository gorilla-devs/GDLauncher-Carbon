use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::Cursor;
use std::io::Read;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::atomic;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use std::usize;

use anyhow::anyhow;
use futures::join;
use futures::Future;
use image::ImageOutputFormat;
use itertools::Itertools;
use md5::Digest;
use murmurhash32::Murmur2Digest;
use sha1::Sha1;
use sha2::Sha512;
use tokio::io::AsyncSeekExt;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

use crate::api::keys::instance::INSTANCE_MODS;
use crate::db::read_filters::BytesFilter;
use crate::db::read_filters::IntFilter;
use crate::db::read_filters::StringFilter;
use crate::db::{mod_file_cache as fcdb, mod_metadata as metadb};
use crate::domain::instance::InstanceId;

use crate::domain::runtime_path::InstancesPath;
use crate::managers::App;
use crate::managers::ManagerRef;

pub mod curseforge;
pub mod modrinth;

use curseforge::CurseforgeModCacher;
use modrinth::ModrinthModCacher;

pub struct MetaCacheManager {
    //waiting_instances: RwLock<HashSet<InstanceId>>,
    //scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_cf_hashes: RwLock<HashSet<u32>>,
    ignored_remote_mr_hashes: RwLock<HashSet<String>>,
    failed_cf_instances: RwLock<HashMap<InstanceId, (std::time::Instant, u32)>>,
    failed_mr_instances: RwLock<HashMap<InstanceId, (std::time::Instant, u32)>>,
    failed_cf_thumbs: RwLock<HashMap<i32, (std::time::Instant, u32)>>,
    failed_mr_thumbs: RwLock<HashMap<String, (std::time::Instant, u32)>>,
    local_targets: LockNotify<CacheTargets>,
    curseforge_targets: LockNotify<CacheTargets>,
    modrinth_targets: LockNotify<CacheTargets>,
    image_scale_semaphore: Semaphore,
    image_download_semaphore: Semaphore,
    watched_instance: watch::Sender<Option<InstanceId>>,
    pause_caching: watch::Sender<bool>,
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
            watched_instance: watch::channel(None).0,
            pause_caching: watch::channel(false).0,
        }
    }
}

#[derive(Clone)]
struct UpdateNotifier {
    target: Arc<AtomicI32>,
    sender: Arc<watch::Sender<()>>,
}

impl UpdateNotifier {
    fn send(&self, instance_id: InstanceId) {
        let target = self.target.load(atomic::Ordering::SeqCst);

        if target == *instance_id {
            let _ = self.sender.send(());
        }
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
    waiting: VecDeque<InstanceId>,
}

struct CacheTarget {
    instance_id: InstanceId,
    callback: Option<Box<dyn CompletionSender>>,
}

struct CacheTargetInfo {
    instance_id: InstanceId,
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
                backend_override: Some(CacheTarget { instance_id, .. }),
                priority,
                waiting: _,
            } => Some(CacheTargetInfo {
                instance_id: *instance_id,
                is_override: true,
                is_priority: priority
                    .as_ref()
                    .is_some_and(|v| *instance_id == v.instance_id),
            }),
            Self {
                backend_override: None,
                priority: Some(CacheTarget { instance_id, .. }),
                waiting: _,
            } => Some(CacheTargetInfo {
                instance_id: *instance_id,
                is_override: false,
                is_priority: true,
            }),
            Self {
                backend_override: None,
                priority: None,
                waiting,
            } => waiting.front().map(|instance_id| CacheTargetInfo {
                instance_id: *instance_id,
                is_override: false,
                is_priority: false,
            }),
        }
    }

    fn release_target(&mut self, instance_id: InstanceId, r: anyhow::Result<()>) -> bool {
        let mut changed = false;

        let check_target_callback = |target: &mut CacheTarget| {
            if target.instance_id == instance_id {
                if let Some(callback) = target.callback.take() {
                    callback.complete(
                        r.as_ref()
                            .map(|_| ())
                            .map_err(|_| anyhow!("error caching mods for instance")),
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
            if self.waiting[i] == instance_id {
                self.waiting.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }

        changed
    }

    // TODO: ensure this immediately cancels the target if running
    fn revoke_target(&mut self, instance_id: InstanceId) -> bool {
        let mut changed = false;

        let mut revoke_option = |target_option: &mut Option<CacheTarget>| {
            if let Some(target) = target_option {
                if target.instance_id == instance_id {
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
            if self.waiting[i] == instance_id {
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
        self.cancel_override();
        self.backend_override = Some(target);
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
    type Token = Option<InstanceId>;
    type Value = CacheTargetInfo;

    fn token(&self) -> Self::Token {
        self.target().map(|target| target.instance_id)
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)> {
        self.target().map(|target| {
            let instance_id = target.instance_id;
            (target, token == Some(instance_id))
        })
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
        instance_id: InstanceId,
        sender: &mut BundleSender<Self::SaveBundle>,
    ) -> anyhow::Result<()>;

    async fn save_batch(app: &App, instance_id: InstanceId, batch: Self::SaveBundle);

    async fn cache_icons(app: &App, instance_id: InstanceId, update_notifier: &UpdateNotifier);
}

type ModplatformCacheBundle<T> = (InstanceId, bool, Option<T>, Option<oneshot::Sender<()>>);

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
            .send((self.instance_id, self.update_images, Some(bundle), tx));
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
                        .send((self.instance_id, self.update_images, None, None));
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
        let image_rx = LockNotify::<Option<InstanceId>>::new(None);
        let image_tx = image_rx.clone();

        let batch_tx = &batch_tx;
        let mut query_loop_watcher = LoopWatcher::new(rx).await;
        let query_loop = query_loop_watcher.loop_interrupt(
            |CacheTargetInfo {
                instance_id,
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
                        debug!({ is_priority, is_override }, "Beginning {} mod caching for instance {instance_id}", C::NAME);

                        // true could be optimized to "if there is a callback" if this is a bottleneck
                        let mut sender = BundleSender::new(instance_id, true, is_priority, batch_tx);
                        let r = C::query_platform(&app, instance_id, &mut sender).await;

                        if let Err(e) = &r {
                            tracing::error!({ error = ?e }, "Could not query {} mod metadata for instance {instance_id}", C::NAME);
                        }

                        sender.wait().await;

                        r
                    };

                    tokio::select! {
                        _ = wait_for_pause => {
                            tracing::info!("Remote mod caching paused");

                            // wait for unpause
                            loop {
                                if !*pause.borrow() {
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

                move |targets: &mut CacheTargets| targets.release_target(instance_id, r)
            },
        );

        let save_loop = async {
            while let Some((instance_id, update_images, bundle, notify)) = batch_rx.recv().await {
                if let Some(bundle) = bundle {
                    debug!(
                        "Saving {} mod cache update bundle for instance {instance_id}",
                        C::NAME
                    );
                    C::save_batch(&app, instance_id, bundle).await;

                    if let Some(notify) = notify {
                        let _ = notify.send(());
                    }

                    let _ = update_notifier.send(instance_id);
                }

                if update_images {
                    image_tx.send(Some(instance_id)).await;
                }
            }
        };

        let mut image_loop_watcher = LoopWatcher::new(image_rx).await;
        let image_loop = image_loop_watcher.loop_interrupt(|instance_id| async move {
            debug!("Caching {} mod icons for instance {instance_id}", C::NAME);

            C::cache_icons(&app, instance_id, &update_notifier).await;

            |_: &mut Option<InstanceId>| false
        });

        // None of the futures should ever exit.
        // This join polls both while allowing them to share variables in this scope.
        futures::join!(query_loop, save_loop, image_loop);
    });
}

impl ManagerRef<'_, MetaCacheManager> {
    pub async fn instance_removed(self, instance_id: InstanceId) {
        join!(
            self.local_targets
                .send_modify(|targets| targets.revoke_target(instance_id)),
            self.curseforge_targets
                .send_modify(|targets| targets.revoke_target(instance_id)),
            self.modrinth_targets
                .send_modify(|targets| targets.revoke_target(instance_id)),
        );

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

    // this will need further refactoring. left for later.
    pub async fn cache_with_priority(self, instance_id: InstanceId) {
        let app = self.app.clone();

        // todo: trace scanned instances, but not here as we also need to account for waiting instances.
        self.local_targets
            .send_modify_always(move |targets| {
                targets.set_priority(CacheTarget {
                    instance_id,
                    callback: Some(Box::new(move |r: anyhow::Result<()>| {
                        if r.is_ok() {
                            tokio::spawn(async move {
                                let mcm = app.meta_cache_manager();

                                join!(
                                    mcm.curseforge_targets.send_modify_always(move |targets| {
                                        targets.set_priority(CacheTarget {
                                            instance_id,
                                            callback: None,
                                        })
                                    }),
                                    mcm.modrinth_targets.send_modify_always(move |targets| {
                                        targets.set_priority(CacheTarget {
                                            instance_id,
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
        instance_id: InstanceId,
        curseforge: bool,
        modrinth: bool,
    ) -> anyhow::Result<()> {
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
                    instance_id,
                    callback: Some(Box::new(move |r: anyhow::Result<()>| match r {
                        Ok(()) => {
                            let _ = local_tx.send(Ok(()));

                            tokio::spawn(async move {
                                let mcm = app.meta_cache_manager();

                                let cf = cf_tx.map(|tx| {
                                    mcm.curseforge_targets.send_modify_always(move |targets| {
                                        targets.set_override(CacheTarget {
                                            instance_id,
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
                                            instance_id,
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

        Ok(())
    }

    pub async fn watch_and_prioritize(self, instance_id: Option<InstanceId>) {
        let _ = self.watched_instance.send(instance_id);

        if let Some(instance_id) = instance_id {
            self.cache_with_priority(instance_id).await;
        }
    }

    pub async fn queue_caching(self, instance_id: InstanceId, _force: bool) {
        // TODO: make track scanned instances for _force
        self.local_targets
            .send_modify_always(|targets| {
                targets.waiting.push_back(instance_id);
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
        let mut debounce_watch_rx = self.watched_instance.subscribe();
        tokio::spawn(async move {
            // wait until watched is some, then wait until we get a list debounce that matches.
            // Then wait 2 seconds, interrupted if the watch changes.
            // note: the various `return`s will only be hit if the cache manager is dropped somehow. they prevent a spinloop.
            loop {
                let watched = *debounce_watch_rx.borrow();

                debounce_target.store(
                    match watched {
                        Some(id) => *id,
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
                        app_debounce.invalidate(INSTANCE_MODS, Some(watched.0.into()));
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
                    // TODO: use the same zip in parse_metadata
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

                let logo_insert = match image_data {
                    Some(image_data) => {
                        let permit = self
                            .image_scale_semaphore
                            .acquire()
                            .await
                            .expect("the image scale semaphore is never closed");

                        let logo = carbon_scheduler::cpu_block(|| {
                            let scaled = scale_mod_image(&image_data[..])?;
                            Ok::<_, anyhow::Error>(Some(scaled))
                        })
                        .await;

                        drop(permit);

                        match logo {
                            Ok(Some(data)) => {
                                Some(self.app.prisma_client.local_mod_image_cache().create(
                                    data,
                                    metadb::UniqueWhereParam::IdEquals(meta_id.clone()),
                                    Vec::new(),
                                ))
                            }
                            Ok(None) => None,
                            Err(e) => {
                                error!({ error = ?e }, "could not scale mod icon for {}", mod_filename);
                                None
                            }
                        }
                    }
                    None => None,
                };

                let meta_insert = self.app.prisma_client.mod_metadata().create(
                    meta_id.clone(),
                    murmur2 as i32,
                    Vec::from(sha512),
                    Vec::from(sha1),
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

fn cache_local(app: App, rx: LockNotify<CacheTargets>, update_notifier: UpdateNotifier) {
    tokio::spawn(async move {
        let app = &app;
        let update_notifier = &update_notifier;

        let cache_instance = |instance_id: InstanceId| async move {
            let app2 = app.clone();
            let cached_entries = tokio::spawn(async move {
                app2.prisma_client
                    .mod_file_cache()
                    .find_many(vec![fcdb::WhereParam::InstanceId(IntFilter::Equals(
                        *instance_id,
                    ))])
                    .exec()
                    .await
            });

            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let Some(instance) = instances.get(&instance_id) else {
                error!("invalid instance id {instance_id} for mod scanning");
                return Ok(());
            };

            let subpath = InstancesPath::subpath()
                .get_instance_path(&instance.shortpath)
                .get_mods_path();

            drop(instances);

            let mut pathbuf = PathBuf::new();
            pathbuf.push(app.settings_manager().runtime_path.get_root().to_path());
            pathbuf.push(&subpath);

            if !pathbuf.is_dir() {
                info!("skipping instance {instance_id} for local caching because it does not have a mods folder");
                return Ok(());
            }

            trace!({ dir = ?pathbuf }, "scanning mods dir for instance {instance_id}");
            let mut modpaths = HashMap::<String, (bool, u64)>::new();
            let mut entries = match tokio::fs::read_dir(&pathbuf).await {
                Ok(entries) => entries,
                Err(e) => {
                    error!({ dir = ?pathbuf, error = ?e }, "could not read instance {instance_id} for mod scanning");
                    return Ok(());
                }
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                // trace!("scanning mods folder entry `{:?}`", entry.file_name());
                let file_name = entry.file_name();
                let Some(mut utf8_name) = file_name.to_str() else {
                    continue;
                };

                let allowed_base_ext = [".jar", ".zip"].iter().any(|&ext| utf8_name.ends_with(ext));
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

            let mut has_outdated_entries = false;

            if let Ok(Ok(cached_entries)) = cached_entries.await {
                has_outdated_entries = cached_entries.len() != modpaths.len();

                for entry in cached_entries {
                    if let Some((enabled, real_size)) = modpaths.get(&entry.filename) {
                        // enabled probably shouldn't be here
                        if *real_size == entry.filesize as u64 && *enabled == entry.enabled {
                            modpaths.remove(&entry.filename);
                            // trace!(
                            //     "up to data metadata entry for mod `{}`, skipping",
                            //     &entry.filename
                            // );
                            continue;
                        }

                        trace!(
                            "outdated metadata entry for mod `{}`, adding to update list",
                            &entry.filename
                        );
                    } else {
                        trace!(
                            "removed metadata entry for mod `{}`, removing",
                            &entry.filename
                        );

                        app.prisma_client
                            .mod_file_cache()
                            .delete(fcdb::UniqueWhereParam::InstanceIdFilenameEquals(
                                *instance_id,
                                entry.filename,
                            ))
                            .exec()
                            .await?;
                    }

                    has_outdated_entries = true;
                }
            }

            let entry_futures = modpaths.into_iter().map(|(subpath, (enabled, _))| {
                let pathbuf = &pathbuf;
                let update_notifier = &update_notifier;

                async move {
                    app.meta_cache_manager()
                        .cache_mod_file_unchecked(instance_id, pathbuf, subpath, enabled)
                        .await?;

                    update_notifier.send(instance_id);

                    Ok(())
                }
            });

            let r = futures::future::join_all(entry_futures)
                .await
                .into_iter()
                .collect::<anyhow::Result<()>>();

            if let Err(e) = r {
                error!({ error = ?e }, "could not store mod scan results for instance {instance_id} in db");
            }

            if has_outdated_entries {
                let _ = update_notifier.send(instance_id);
            }

            Ok(())
        };

        let cache_instance = &cache_instance;

        LoopWatcher::new(rx).await.loop_interrupt(
            |CacheTargetInfo {
                instance_id,
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
                        debug!("Beginning local mod caching for instance {instance_id}");

                        let r = cache_instance(instance_id).await;

                        if let Err(e) = &r {
                            tracing::error!({ error = ?e }, "Could not query local mod metadata for instance {instance_id}");
                        }

                        // waiting list targets cascade into curseforge and modrinth caching.
                        if !is_override && !is_priority {
                            let mcm = app.meta_cache_manager();

                            join!(
                                mcm.curseforge_targets.send_modify_always(|targets| targets.waiting.push_back(instance_id)),
                                mcm.modrinth_targets.send_modify_always(|targets| targets.waiting.push_back(instance_id)),
                            );
                        }

                        r
                    };

                    tokio::select! {
                        _ = wait_for_pause => {
                            tracing::info!("Local mod caching paused");

                            // wait for unpause
                            loop {
                                if !*pause.borrow() {
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

                move |targets: &mut CacheTargets| targets.release_target(instance_id, r)
            }
        ).await;
    });
}
