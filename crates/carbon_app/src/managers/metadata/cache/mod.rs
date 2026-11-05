use crate::api::keys::instance::{INSTANCE_DETAILS, INSTANCE_MODS};
use crate::api::keys::server::GET_SERVER_ADDONS;
use crate::api::translation::Translation;
use crate::domain::instance::InstanceId;
use crate::managers::App;
use crate::managers::ManagerRef;
use crate::managers::vtask::VisualTask;
use anyhow::{Context, anyhow};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use carbon_repos::repos::mod_metadata as metarepo;
use carbon_rt_path::InstancesPath;
use chrono::Utc;
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
pub(crate) mod eligibility;
pub mod modrinth;
#[cfg(test)]
mod scan_tests;

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

/// How long a file a platform's hash lookup found no match for is left out of
/// later lookups. Most misses are files the platform does not host, but a
/// lookup has also answered without a file it serves, and the file then lost
/// its identity and update checks until the app restarted. Lookups run only
/// when an instance or server is opened, changed or scanned, and one request
/// covers a whole batch of files, so asking again about an entity's unknown
/// files at most every half hour costs a few requests per pass.
pub(crate) const IGNORED_HASH_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Hashes a platform lookup found no match for, each with when it missed.
pub(crate) struct IgnoredHashes<K> {
    missed_at: HashMap<K, std::time::Instant>,
}

impl<K: Eq + std::hash::Hash> IgnoredHashes<K> {
    fn new() -> Self {
        Self {
            missed_at: HashMap::new(),
        }
    }

    /// Leaves `keys` out of lookups for [`IGNORED_HASH_TTL`] from `now`.
    pub(crate) fn ignore(&mut self, keys: impl IntoIterator<Item = K>, now: std::time::Instant) {
        // Expired entries go here, so the map only holds recent misses.
        self.missed_at
            .retain(|_, missed| now.saturating_duration_since(*missed) < IGNORED_HASH_TTL);
        self.missed_at
            .extend(keys.into_iter().map(|key| (key, now)));
    }

    pub(crate) fn contains(&self, key: &K, now: std::time::Instant) -> bool {
        self.missed_at
            .get(key)
            .is_some_and(|missed| now.saturating_duration_since(*missed) < IGNORED_HASH_TTL)
    }
}

/// Logs, in one line, how many of the files a platform lookup asked about got
/// no match, naming a few (`unmatched` holds their metadata ids).
async fn log_unmatched(
    app: &App,
    platform: &str,
    entity_id: CacheEntityId,
    queried: usize,
    unmatched: &HashSet<String>,
) {
    if unmatched.is_empty() {
        return;
    }
    // A failed read only costs the names.
    let files: Vec<(String, String)> = match entity_id {
        CacheEntityId::Instance(id) => mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .into_iter()
            .flatten()
            .map(|row| (row.metadata_id, row.filename))
            .collect(),
        CacheEntityId::Server(id) => mfcdb::get_server_mod_files_by_server(&app.db, id)
            .await
            .into_iter()
            .flatten()
            .map(|row| (row.metadata_id, row.filename))
            .collect(),
    };
    let names = files
        .into_iter()
        .filter(|(metadata_id, _)| unmatched.contains(metadata_id))
        .map(|(_, filename)| filename)
        .collect::<Vec<_>>();
    let more = match names.len().saturating_sub(5) {
        0 => String::new(),
        more => format!(" and {more} more"),
    };
    info!(
        "{platform} found no match for {} of {queried} files of {entity_id}, not asking again for {} minutes: {}{more}",
        unmatched.len(),
        IGNORED_HASH_TTL.as_secs() / 60,
        names.iter().take(5).join(", "),
    );
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
    file_stamp: Option<String>,
}

#[cfg(test)]
#[derive(Default)]
struct ScanTestState {
    fail_after: HashMap<(PathBuf, &'static str), usize>,
    observations: HashMap<(PathBuf, &'static str), usize>,
    mutate_before_recheck: HashMap<PathBuf, Vec<u8>>,
    pauses: HashMap<(PathBuf, &'static str), Arc<ScanPause>>,
}

/// A one-shot stop in a scan, armed by a test for one path and stage.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct ScanPause {
    pub(crate) entered: tokio::sync::Notify,
    pub(crate) release: tokio::sync::Notify,
}

pub struct MetaCacheManager {
    #[cfg(test)]
    scan_test_state: std::sync::Mutex<ScanTestState>,
    /// How far tests have moved [`Self::now`] ahead of the real clock.
    #[cfg(test)]
    clock_offset: std::sync::Mutex<std::time::Duration>,
    //waiting_instances: RwLock<HashSet<InstanceId>>,
    //scanned_instances: Mutex<HashSet<InstanceId>>,
    ignored_remote_cf_hashes: RwLock<IgnoredHashes<u32>>,
    ignored_remote_mr_hashes: RwLock<IgnoredHashes<String>>,
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
    /// Serializes `ensure_mod_metadata` per content hash. Without it two
    /// concurrent scans of the same jar both miss the lookup and each insert a
    /// `ModMetadata` row, since there is no unique constraint on the hash pair.
    /// `ensure_mod_metadata` evicts its own entry once settled (see its own
    /// doc, right before it returns), so this only ever holds one entry per
    /// (sha512, murmur2) pair with an in-flight or racing-to-start lookup,
    /// never one per pair ever scanned over the process's lifetime.
    metadata_hash_locks: dashmap::DashMap<(Vec<u8>, i32), std::sync::Arc<tokio::sync::Mutex<()>>>,
}

impl MetaCacheManager {
    pub fn new() -> Self {
        Self {
            #[cfg(test)]
            scan_test_state: std::sync::Mutex::new(ScanTestState::default()),
            #[cfg(test)]
            clock_offset: std::sync::Mutex::new(std::time::Duration::ZERO),
            //waiting_instances: RwLock::new(HashSet::new()),
            //scanned_instances: Mutex::new(HashSet::new()),
            ignored_remote_cf_hashes: RwLock::new(IgnoredHashes::new()),
            ignored_remote_mr_hashes: RwLock::new(IgnoredHashes::new()),
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
            metadata_hash_locks: dashmap::DashMap::new(),
        }
    }

    /// The time ignore-list entries are judged by. Tests move it forward
    /// instead of sleeping.
    pub(crate) fn now(&self) -> std::time::Instant {
        let now = std::time::Instant::now();
        #[cfg(test)]
        let now = now + *self.clock_offset.lock().unwrap();
        now
    }

    #[cfg(test)]
    pub(crate) fn advance_clock(&self, by: std::time::Duration) {
        *self.clock_offset.lock().unwrap() += by;
    }

    #[cfg(test)]
    fn scan_test_boundary(&self, path: &Path, stage: &'static str) -> std::io::Result<()> {
        let mut state = self.scan_test_state.lock().unwrap();
        let key = (path.to_owned(), stage);
        let count = state.observations.entry(key.clone()).or_default();
        *count += 1;
        let count = *count;
        if state
            .fail_after
            .get(&key)
            .is_some_and(|after| count >= *after)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("injected {stage} observation failure"),
            ));
        }
        Ok(())
    }

    /// Stop the scan at `stage` for `path` once, if a test armed it.
    #[cfg(test)]
    async fn scan_test_pause(&self, path: &Path, stage: &'static str) {
        let pause = self
            .scan_test_state
            .lock()
            .unwrap()
            .pauses
            .remove(&(path.to_owned(), stage));
        if let Some(pause) = pause {
            pause.entered.notify_one();
            pause.release.notified().await;
        }
    }

    #[cfg(test)]
    pub(crate) fn arm_scan_pause(&self, path: &Path, stage: &'static str) -> Arc<ScanPause> {
        let pause = Arc::new(ScanPause::default());
        self.scan_test_state
            .lock()
            .unwrap()
            .pauses
            .insert((path.to_owned(), stage), pause.clone());
        pause
    }

    #[cfg(test)]
    pub(crate) fn scan_observations(&self, path: &Path, stage: &'static str) -> usize {
        self.scan_test_state
            .lock()
            .unwrap()
            .observations
            .get(&(path.to_owned(), stage))
            .copied()
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn clear_scan_observations(&self) {
        self.scan_test_state.lock().unwrap().observations.clear();
    }

    #[cfg(test)]
    pub(crate) fn mutate_before_recheck(&self, path: &Path, bytes: Vec<u8>) {
        self.scan_test_state
            .lock()
            .unwrap()
            .mutate_before_recheck
            .insert(path.to_owned(), bytes);
    }

    async fn scan_folder_metadata(&self, path: &Path) -> std::io::Result<std::fs::Metadata> {
        #[cfg(test)]
        self.scan_test_boundary(path, "folder-stat")?;
        tokio::fs::metadata(path).await
    }

    async fn scan_read_dir(&self, path: &Path) -> std::io::Result<tokio::fs::ReadDir> {
        #[cfg(test)]
        self.scan_test_boundary(path, "read-dir")?;
        tokio::fs::read_dir(path).await
    }

    async fn scan_next_entry(
        &self,
        entries: &mut tokio::fs::ReadDir,
        path: &Path,
    ) -> std::io::Result<Option<tokio::fs::DirEntry>> {
        #[cfg(test)]
        self.scan_test_boundary(path, "iterate")?;
        entries.next_entry().await
    }

    async fn scan_entry_metadata(
        &self,
        entry: &tokio::fs::DirEntry,
    ) -> std::io::Result<std::fs::Metadata> {
        #[cfg(test)]
        self.scan_test_boundary(&entry.path(), "entry-stat")?;
        entry.metadata().await
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

/// Addon folders whose bytes a local pass re-reads even when the stored stamp
/// still matches. Every other file is trusted on its parser-qualified stamp, so
/// a pass costs one `stat` per unchanged addon instead of a full read.
///
/// Exports ask for [`VerifyScope::MODS`], the only folder they link to a
/// platform file, so a same-size replacement with a preserved timestamp can
/// never ship as the old platform file. Installs ask for nothing: publication
/// already hashed the file it wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerifyScope(u8);

impl VerifyScope {
    pub const NONE: Self = Self(0);
    pub const MODS: Self = Self(1);
    pub const ALL: Self = Self(0b1111);

    fn bit(kind: crate::domain::instance::AddonType) -> u8 {
        use crate::domain::instance::AddonType;
        match kind {
            AddonType::Mods => 1,
            AddonType::ResourcePacks => 1 << 1,
            AddonType::Shaders => 1 << 2,
            AddonType::DataPacks => 1 << 3,
            // Worlds are directories with name-derived identities.
            AddonType::Worlds => 0,
        }
    }

    pub fn contains(self, kind: crate::domain::instance::AddonType) -> bool {
        self.0 & Self::bit(kind) != 0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

struct CacheTargets {
    backend_override: VecDeque<CacheTarget>,
    priority: Option<CacheTarget>,
    waiting: VecDeque<CacheEntityId>,
    /// Requested byte verification per entity, OR-ed across coalesced requests
    /// and cleared when a pass that observed them completes.
    verify: HashMap<CacheEntityId, VerifyScope>,
    generations: HashMap<CacheEntityId, u64>,
}

struct CacheTarget {
    entity_id: CacheEntityId,
    callback: Option<Box<dyn CompletionSender>>,
}

struct CacheTargetInfo {
    entity_id: CacheEntityId,
    is_override: bool,
    is_priority: bool,
    verify: VerifyScope,
    generation: u64,
}

impl CacheTargets {
    fn new() -> Self {
        Self {
            backend_override: VecDeque::new(),
            priority: None,
            waiting: VecDeque::new(),
            verify: HashMap::new(),
            generations: HashMap::new(),
        }
    }

    fn verify_scope(&self, entity_id: CacheEntityId) -> VerifyScope {
        self.verify.get(&entity_id).copied().unwrap_or_default()
    }

    fn target(&self) -> Option<CacheTargetInfo> {
        if let Some(target) = self.backend_override.front() {
            Some(CacheTargetInfo {
                entity_id: target.entity_id,
                is_override: true,
                verify: self.verify_scope(target.entity_id),
                generation: self
                    .generations
                    .get(&target.entity_id)
                    .copied()
                    .unwrap_or(0),
                is_priority: self
                    .priority
                    .as_ref()
                    .is_some_and(|p| p.entity_id == target.entity_id),
            })
        } else if let Some(target) = &self.priority {
            Some(CacheTargetInfo {
                entity_id: target.entity_id,
                is_override: false,
                is_priority: true,
                verify: self.verify_scope(target.entity_id),
                generation: self
                    .generations
                    .get(&target.entity_id)
                    .copied()
                    .unwrap_or(0),
            })
        } else {
            self.waiting.front().map(|&entity_id| CacheTargetInfo {
                entity_id,
                is_override: false,
                is_priority: false,
                verify: self.verify_scope(entity_id),
                generation: self.generations.get(&entity_id).copied().unwrap_or(0),
            })
        }
    }

    fn release_pass(
        &mut self,
        entity_id: CacheEntityId,
        generation: u64,
        r: anyhow::Result<()>,
    ) -> bool {
        if self.generations.get(&entity_id).copied().unwrap_or(0) != generation {
            return true; // Re-run the live request; this pass did not observe it.
        }
        self.release_target(entity_id, r)
    }

    fn release_target(&mut self, entity_id: CacheEntityId, r: anyhow::Result<()>) -> bool {
        self.verify.remove(&entity_id);
        let mut changed = false;

        let check_target_callback =
            |target: &mut CacheTarget| {
                if target.entity_id == entity_id {
                    if let Some(callback) = target.callback.take() {
                        callback.complete(r.as_ref().map(|_| ()).map_err(|error| {
                            anyhow!("error caching mods for {entity_id}: {error:#}")
                        }));
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

        if let Some(index) = self
            .backend_override
            .iter()
            .position(|t| t.entity_id == entity_id)
        {
            let mut target = self.backend_override.remove(index).unwrap();
            check_target_callback(&mut target);
            changed = true;
        }
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
        self.verify.remove(&entity_id);
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

        revoke_option(&mut self.priority);
        if let Some(index) = self
            .backend_override
            .iter()
            .position(|t| t.entity_id == entity_id)
        {
            if let Some(callback) = self.backend_override.remove(index).unwrap().callback {
                callback.complete(Err(anyhow!("This cache target was revoked")));
            }
            changed = true;
        }

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

    fn set_priority(&mut self, target: CacheTarget) {
        self.cancel_priority();
        self.priority = Some(target);
    }

    /// Queue an explicit pass that also re-reads the bytes of `verify`'s
    /// folders. Scopes of coalesced requests are OR-ed, so every waiter's pass
    /// covers what it asked for.
    fn set_override_verified(&mut self, target: CacheTarget, verify: VerifyScope) {
        if verify != VerifyScope::NONE {
            let scope = self.verify.entry(target.entity_id).or_default();
            *scope = scope.union(verify);
        }
        self.set_override(target);
    }

    fn set_override(&mut self, target: CacheTarget) {
        *self.generations.entry(target.entity_id).or_default() += 1;
        // Coalesce overrides for the same entity. Otherwise a second waiter
        // (e.g. dependency mod + parent mod racing to wait on the same
        // instance's cache pass) would cancel the first one's callback,
        // failing the in-flight install with "Backend override was canceled".
        if let Some(existing) = self
            .backend_override
            .iter_mut()
            .find(|t| t.entity_id == target.entity_id)
        {
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
        self.backend_override.push_back(target);
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
    /// Queue completion dequeues and notifies; image completion retains pending work.
    const RESELECT_AFTER_COMPLETION: bool;

    fn token(&self) -> Self::Token;

    // Option<(value, value matches last)>
    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)>;
}

impl LoopValue for CacheTargets {
    type Token = Option<(CacheEntityId, bool, VerifyScope, u64)>;
    type Value = CacheTargetInfo;
    const RESELECT_AFTER_COMPLETION: bool = true;

    fn token(&self) -> Self::Token {
        self.target().map(|target| {
            (
                target.entity_id,
                target.is_override,
                target.verify,
                target.generation,
            )
        })
    }

    fn loop_cmp(&self, token: Self::Token) -> Option<(Self::Value, bool)> {
        self.target().map(|target| {
            let identity = (
                target.entity_id,
                target.is_override,
                target.verify,
                target.generation,
            );
            (target, token == Some(identity))
        })
    }
}

impl LoopValue for Option<CacheEntityId> {
    type Token = Self;
    type Value = CacheEntityId;
    const RESELECT_AFTER_COMPLETION: bool = false;

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

                    // Queue consumers must reselect live ownership/generation. Image
                    // consumers must retain same-entity work observed during this pass.
                    if T::RESELECT_AFTER_COMPLETION || next.is_none() {
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
                verify: _,
                generation,
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
                        biased;
                        _ = wait_for_pause, if !is_override => {
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

                move |targets: &mut CacheTargets| targets.release_pass(entity_id, generation, r)
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
    pub async fn gc_mod_metadata(self) {
        if let Err(error) = metarepo::gc_orphan_metadata(&self.app.db).await {
            warn!(
                ?error,
                "Metadata garbage collection failed; database changes were rolled back"
            );
        }
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

    /// Run a local pass for `entity_id` ahead of background work and wait for
    /// it (and the requested platform passes). Unchanged addons are trusted on
    /// their stamp; see [`Self::override_caching_verified_and_wait`] for a pass
    /// that re-reads bytes.
    pub async fn override_caching_and_wait(
        self,
        entity_id: CacheEntityId,
        curseforge: bool,
        modrinth: bool,
    ) -> anyhow::Result<()> {
        self.override_caching_verified_and_wait(entity_id, VerifyScope::NONE, curseforge, modrinth)
            .await
    }

    /// Like [`Self::override_caching_and_wait`], but the local pass also
    /// re-reads every addon in `verify`'s folders and re-hashes any whose bytes
    /// no longer match their row, even when size and timestamps are unchanged.
    pub async fn override_caching_verified_and_wait(
        self,
        entity_id: CacheEntityId,
        verify: VerifyScope,
        curseforge: bool,
        modrinth: bool,
    ) -> anyhow::Result<()> {
        tracing::info!("Overriding caching and waiting for {entity_id} (verify {verify:?})");

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
                targets.set_override_verified(
                    CacheTarget {
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
                    },
                    verify,
                );
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

    pub async fn queue_caching(self, entity_id: CacheEntityId, force: bool) {
        info!("Queuing mod caching for {}", entity_id);

        self.local_targets
            .send_modify_always(|targets| {
                if force {
                    // Re-read every addon's bytes on this pass.
                    let scope = targets.verify.entry(entity_id).or_default();
                    *scope = scope.union(VerifyScope::ALL);
                }
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

        let mut file = tokio::fs::File::open(&path).await?;
        let initial_stamp = observed_file_stamp(&file.metadata().await?);

        #[cfg(test)]
        {
            self.scan_test_boundary(&path, "hash")?;
            self.scan_test_pause(&path, "hash").await;
        }
        let mut sha512 = Sha512::new();
        let mut sha1 = Sha1::new();
        let mut murmur_len = 0;
        let mut content_len = 0;

        #[cfg(test)]
        note_whole_file_read(&path);
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

        #[cfg(test)]
        {
            self.scan_test_boundary(&path, "parse")?;
            self.scan_test_pause(&path, "parse").await;
        }
        let (file, meta, image_data) = tokio::task::spawn_blocking(|| {
            let meta = super::mods::parse_metadata(&mut file);

            let image_data = match &meta
                .as_ref()
                .map(|m| m.as_ref().map(|m| m.logo_file.as_ref()))
            {
                Ok(Some(Some(logo_file))) => {
                    // A damaged icon entry only costs the icon, not the addon.
                    match zip::ZipArchive::new(&mut file) {
                        Ok(mut zip) => match zip.by_name(&logo_file) {
                            Ok(mut file) => {
                                let mut image = Vec::with_capacity(file.size() as usize);
                                match file.read_to_end(&mut image) {
                                    Ok(_) => Some(image),
                                    Err(error) => {
                                        debug!(?error, "could not read the icon of {}", logo_file);
                                        None
                                    }
                                }
                            }
                            _ => None,
                        },
                        Err(_) => None,
                    }
                }
                _ => None,
            };

            Ok::<_, anyhow::Error>((file, meta, image_data))
        })
        .await??;

        // The murmur2 seed needs the first read's length, so the file is read
        // twice. The second read opens the path again and also re-hashes it:
        // equal SHA-512s show the path still holds the bytes the first read
        // hashed and the metadata was parsed from, even where a stamp cannot
        // tell (Windows has no change time). No third read is needed.
        drop(file);
        let mut file = tokio::fs::File::open(&path).await?;
        let mut murmur2 = Murmur2Digest::new(murmur_len as u32);
        let mut recheck = Sha512::new();

        let mut workbuf = Vec::<u8>::with_capacity(carbon_scheduler::BUFSIZE);

        #[cfg(test)]
        note_whole_file_read(&path);
        carbon_scheduler::buffered_digest(&mut file, |chunk| {
            recheck.update(&chunk);
            workbuf.splice(.., chunk.iter().map(|&b| b));
            workbuf.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
            murmur2.update(&workbuf[..]);
        })
        .await?;

        let murmur2 = murmur2.finalize();
        let recheck: [u8; 64] = recheck.finalize().into();

        drop(file);
        if recheck != sha512
            || observed_file_stamp(&tokio::fs::metadata(&path).await?) != initial_stamp
        {
            anyhow::bail!("Addon changed while being scanned; retry the operation");
        }

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
            file_stamp: Some(initial_stamp),
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
            file_stamp: None,
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

        // Serialize the lookup-then-insert for this hash: concurrent scans of the
        // same jar (e.g. the same mod across instances) would otherwise both miss
        // and each insert a duplicate `ModMetadata` row. Clone the Arc and drop
        // the map guard before awaiting the lock so the DashMap shard isn't held
        // across the await.
        let lock_key = (sha512.clone(), murmur2);
        let hash_lock = self
            .metadata_hash_locks
            .entry(lock_key.clone())
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        let _hash_guard = hash_lock.lock().await;

        let dbmeta = metarepo::find_metadata_by_hashes(&self.app.db, &sha512, murmur2).await?;

        let meta_id = match dbmeta {
            Some(meta) => {
                // Heal only the known parser attribution bug, based on actual
                // current bytes. Keep hashes, IDs, images and remote references.
                if result.meta.as_ref().is_some_and(|m| {
                    m.modloaders
                        .contains(&crate::domain::instance::info::ModLoaderType::Neoforge)
                }) {
                    let loaders = result
                        .meta
                        .as_ref()
                        .unwrap()
                        .modloaders
                        .iter()
                        .map(ToString::to_string)
                        .join(",");
                    if meta.modloaders != loaders {
                        metarepo::update_local_modloaders(
                            &self.app.db,
                            &meta.id,
                            &loaders,
                            DbDateTime(Utc::now().fixed_offset()),
                        )
                        .await?;
                    }
                }
                meta.id
            }
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
                        #[cfg(test)]
                        self.scan_test_pause(Path::new(mod_filename), "icon-scale")
                            .await;

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
                // dispatch, so no other write interleaves; they run in ONE transaction —
                // all-or-nothing: a failure rolls the whole group back and readers
                // never observe an intermediate state. `_conn` forms on the tx guard.
                self.app
                    .db
                    .write(move |mut conn| {
                        let tx = conn.transaction()?;
                        metarepo::insert_metadata_conn(
                            &tx,
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
                            metarepo::insert_local_image_conn(&tx, &meta_id_owned, &data)?;
                        }
                        tx.commit()?;
                        Ok(())
                    })
                    .await?;

                meta_id
            }
        };

        // Evicts this hash's lock entry once nothing else needs it, so
        // `metadata_hash_locks` doesn't grow forever across a long-running
        // process's full mod library — an entry that's never removed would
        // otherwise accumulate one permanent `Arc<Mutex<()>>` per distinct
        // (sha512, murmur2) ever scanned. Both the guard and this function's
        // own `hash_lock` clone are dropped first: `remove_if`'s closure
        // only observes the copy stored IN the map, so as long as either is
        // still alive here the true strong count is higher than what the
        // closure sees, and `Arc::strong_count(v) == 1` would never fire —
        // silently defeating the eviction, not merely delaying it. Dropping
        // both before calling `remove_if` also keeps this clear of DashMap's
        // own single-shard self-deadlock rule: neither drop leaves an
        // outstanding `Ref`/`RefMut` borrowed from `metadata_hash_locks`
        // itself (the `.entry(..).or_insert_with(..).clone()` above already
        // released its shard guard the moment that statement finished), so
        // `remove_if`'s own shard lock is never re-entered while already
        // held.
        //
        // Safe against a race with a concurrent caller: another in-flight
        // `ensure_mod_metadata` call for the SAME hash holds its own `Arc`
        // clone (from the same `.entry()` lookup, before this one's
        // `remove_if` runs), which keeps `strong_count` above 1 and the
        // entry survives untouched. A caller that acquires the lock for the
        // first time strictly *between* this drop and `remove_if` actually
        // running would have its own fresh `Arc` clone racing this removal —
        // either it wins (its clone keeps `strong_count` > 1, `remove_if` is
        // a no-op) or `remove_if` wins and removes the entry it no longer
        // needs, since `DashMap::entry` on its next lookup just re-inserts a
        // fresh lock for the same key. Either outcome is correct: no
        // duplicate `ModMetadata` row can result, only — in the second,
        // narrower case — one wasted removal-then-reinsert.
        drop(_hash_guard);
        drop(hash_lock);
        self.metadata_hash_locks
            .remove_if(&lock_key, |_, v| std::sync::Arc::strong_count(v) == 1);

        Ok(meta_id)
    }

    /// Cache a mod file for an instance. Hashes, parses metadata, and upserts into mod_file_cache.
    /// Refresh only an already-published artifact under the caller's instance
    /// ownership. The scanner takes the same gate for its snapshot and commit.
    pub(crate) async fn refresh_published_addon(
        self,
        ownership: &crate::managers::instance::InstanceOperationGuard,
        folder: &PathBuf,
        filename: String,
        enabled: bool,
        addon_type: String,
    ) -> anyhow::Result<String> {
        self.cache_mod_file_unchecked(
            ownership.instance_id(),
            folder,
            filename,
            enabled,
            addon_type,
        )
        .await
    }

    /// Re-read a cached addon once and compare it with its row's metadata.
    ///
    /// Only rows stamped by this parser revision for their current metadata
    /// qualify. The opened handle's stamp before the read must equal the path's
    /// stamp after it, as in [`Self::hash_and_parse_mod_file`]. A replacement
    /// that also keeps every stamp field between those two checks (possible on
    /// Windows, which has no change time) is caught by the exporters' final
    /// re-read of every linked file.
    async fn verify_cached_bytes(
        self,
        stored: Option<&str>,
        metadata_id: &str,
        path: &Path,
    ) -> anyhow::Result<ByteCheck> {
        let Some(stored) = stored else {
            return Ok(ByteCheck::Different);
        };
        let Some(metadata) = metarepo::get_metadata_by_id(&self.app.db, metadata_id).await? else {
            return Ok(ByteCheck::Different);
        };
        if metadata.sha512.len() != 64
            || metadata.sha1.len() != 20
            || stored.split_once('|').map(|v| v.0)
                != qualified_file_stamp(&metadata, "")
                    .split_once('|')
                    .map(|v| v.0)
        {
            return Ok(ByteCheck::Different);
        }
        let mut file = tokio::fs::File::open(path)
            .await
            .with_context(|| format!("Could not open addon {}", path.display()))?;
        let before = observed_file_stamp(&file.metadata().await?);
        #[cfg(test)]
        {
            self.scan_test_boundary(path, "hash")?;
            self.scan_test_pause(path, "hash").await;
            note_whole_file_read(path);
        }
        let mut digest = Sha512::new();
        carbon_scheduler::buffered_digest(&mut file, |bytes| digest.update(bytes))
            .await
            .with_context(|| format!("Could not read addon {}", path.display()))?;
        drop(file);
        let current_hash = digest.finalize().to_vec();
        #[cfg(test)]
        {
            let replacement = self
                .scan_test_state
                .lock()
                .unwrap()
                .mutate_before_recheck
                .remove(path);
            if let Some(bytes) = replacement {
                let modified = std::fs::metadata(path)?.modified()?;
                tokio::fs::write(path, bytes).await?;
                std::fs::File::options()
                    .write(true)
                    .open(path)?
                    .set_times(std::fs::FileTimes::new().set_modified(modified))?;
            }
            self.scan_test_boundary(path, "hash-recheck")?;
        }
        let after = observed_file_stamp(&tokio::fs::metadata(path).await?);
        if before != after || current_hash != metadata.sha512 {
            return Ok(ByteCheck::Different);
        }
        Ok(ByteCheck::Same(after))
    }

    /// Settle a row written before stamps existed (2.0.40 and older) with as
    /// little reading as possible. Returns the stamp to record for it, or
    /// `None` when the file must be hashed and parsed again.
    ///
    /// The caller has already matched size and enabled state. The row is kept
    /// when its metadata has full-length hashes and nothing about the file
    /// changed since it was hashed:
    /// - from `stat` alone, without reading the file, when the file is regular
    ///   and both its modification time and its change (Unix) or creation
    ///   (Windows) time are no later than the row's last write, unless `read`
    ///   (the pass verifies this folder's bytes, as exports do);
    /// - otherwise (a data folder that was copied, migrated or restored gives
    ///   every file newer times) by one read of its bytes, equal to the row's
    ///   SHA-512, instead of a full hash and parse.
    ///
    /// Forge-labelled rows then re-read the jar's mod list (zip directory and
    /// manifest entries only), because older parsers labelled NeoForge jars as
    /// Forge; such rows are healed to NeoForge here.
    ///
    /// Accepted residual risk: a same-size replacement made before the upgrade
    /// that also left both times older than the row keeps its old hashes until
    /// the file next changes. Exports re-read every mod they link, so such a
    /// file is never shipped as the old platform file.
    async fn settle_legacy_row(
        self,
        metadata_id: &str,
        last_written: chrono::DateTime<chrono::FixedOffset>,
        stamp: &str,
        times: Option<(std::time::SystemTime, std::time::SystemTime)>,
        path: &Path,
        read: bool,
    ) -> anyhow::Result<Option<String>> {
        let Some(metadata) = metarepo::get_metadata_by_id(&self.app.db, metadata_id).await? else {
            return Ok(None);
        };
        if metadata.sha512.len() != 64 || metadata.sha1.len() != 20 {
            return Ok(None);
        }
        let last_written = std::time::SystemTime::from(last_written);
        let untouched = !read
            && times.is_some_and(|(modified, changed)| {
                modified <= last_written && changed <= last_written
            });
        let stamp = if untouched {
            stamp.to_owned()
        } else {
            #[cfg(test)]
            {
                self.scan_test_boundary(path, "hash")?;
                self.scan_test_pause(path, "hash").await;
            }
            let (hash, stable) = read_stable_file_hash(path)
                .await
                .with_context(|| format!("Could not read addon {}", path.display()))?;
            match stable {
                Some(observed) if hash == metadata.sha512 => observed,
                _ => return Ok(None),
            }
        };
        if metadata
            .modloaders
            .split(',')
            .any(|loader| loader.trim() == "forge")
        {
            #[cfg(test)]
            self.scan_test_boundary(path, "loader-check")?;
            let owned = path.to_owned();
            let (observed, parsed) = tokio::task::spawn_blocking(move || {
                let mut file = std::fs::File::open(&owned)?;
                let observed = observed_file_stamp(&file.metadata()?);
                Ok::<_, std::io::Error>((observed, super::mods::parse_metadata(&mut file)))
            })
            .await??;
            if observed != stamp {
                return Ok(None);
            }
            if let Ok(Some(parsed)) = parsed {
                if parsed
                    .modloaders
                    .contains(&crate::domain::instance::info::ModLoaderType::Neoforge)
                {
                    let loaders = parsed.modloaders.iter().map(ToString::to_string).join(",");
                    if loaders != metadata.modloaders {
                        metarepo::update_local_modloaders(
                            &self.app.db,
                            &metadata.id,
                            &loaders,
                            DbDateTime(Utc::now().fixed_offset()),
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(Some(stamp))
    }

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
        // GC may collect an unreferenced ensured owner before this dispatch.
        // Re-ensure once on that exact stale-owner outcome; publication and the
        // qualified marker share a writer transaction, including with FKs off.
        for attempt in 0..2 {
            let meta_id = self.ensure_mod_metadata(&result, &mod_filename).await?;
            let published = publish_instance_row(
                &self.app.db,
                instance_id,
                &mod_filename,
                &addon_type,
                enabled,
                &meta_id,
                &result,
                None,
            )
            .await;
            match published {
                Ok(_) => return Ok(meta_id),
                Err(error) if attempt == 0 && error.is_foreign_key_violation() => continue,
                Err(error) => return Err(error.into()),
            }
        }
        unreachable!("bounded publication loop returns on its last attempt")
    }

    /// Prepare hash-deduplicated metadata while bytes are still privately staged.
    /// No server owner row or shared provider identity is changed here.
    pub(crate) async fn prepare_server_addon_metadata(
        self,
        staged: &Path,
        selected_filename: &str,
    ) -> anyhow::Result<String> {
        let folder = staged
            .parent()
            .context("Staged addon has no parent")?
            .to_path_buf();
        let filename = staged
            .file_name()
            .and_then(|name| name.to_str())
            .context("Invalid staged addon filename")?;
        let result = self
            .hash_and_parse_mod_file(&folder, filename, true)
            .await?;
        self.ensure_mod_metadata(&result, selected_filename).await
    }

    /// Cache a mod file for a server. Hashes, parses metadata, and upserts into server_mod_file_cache.
    pub(crate) async fn refresh_server_addon(
        self,
        server_id: i32,
        folder: &PathBuf,
        filename: String,
        location: String,
        enabled: bool,
        addon_type: String,
    ) -> anyhow::Result<String> {
        // Caller holds the stable server operation gate through local publication.
        let result = self
            .hash_and_parse_mod_file(folder, &filename, enabled)
            .await?;
        for attempt in 0..2 {
            let meta_id = self.ensure_mod_metadata(&result, &filename).await?;
            match publish_server_row(
                &self.app.db,
                server_id,
                &filename,
                &location,
                enabled,
                &addon_type,
                &meta_id,
                &result,
                None,
            )
            .await
            {
                Ok(_) => return Ok(meta_id),
                Err(error) if attempt == 0 && error.is_foreign_key_violation() => continue,
                Err(error) => return Err(error.into()),
            }
        }
        unreachable!("bounded publication loop returns on its last attempt")
    }

    /// Scan a server's `mods/` and selected world's `datapacks/` and bring
    /// `ServerModFileCache` in line with them.
    ///
    /// The server operation gate (Start, Stop, toggles, the addon list) is held
    /// only to snapshot the folders and to commit results, never while files
    /// are hashed. Unchanged files are trusted on their stamp. One unreadable
    /// file, an enabled/disabled duplicate or an unusable `level-name` never
    /// fails the scan: those locations keep their rows and everything else is
    /// still refreshed and pruned.
    pub async fn cache_server_local(
        self,
        server_id: i32,
        server_shortpath: &str,
    ) -> anyhow::Result<()> {
        let activity = self.app.runtime_lifecycle.admit("server addon scan")?;
        let cancel = activity.cancellation();
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(anyhow!(
                "Addon scan of server {server_id} stopped because the launcher is closing"
            )),
            result = self.scan_server(server_id, server_shortpath) => result,
        };
        drop(activity);
        result
    }

    async fn scan_server(self, server_id: i32, server_shortpath: &str) -> anyhow::Result<()> {
        use crate::managers::server::world;
        use futures::StreamExt;
        let lock = self
            .app
            .server_manager()
            .get_op_lock(crate::domain::server::ServerId(server_id));

        // Phase 1: snapshot rows and folder listings under the gate.
        let guard = lock.lock().await;
        let current = carbon_repos::repos::server::get_server(&self.app.db, server_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server disappeared"))?;
        if current.shortpath != server_shortpath {
            anyhow::bail!("Server root changed during scan");
        }
        let data = self
            .app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(server_shortpath)
            .get_data_path();
        let world = match world::selected(&data) {
            Ok(world) => Some(world),
            Err(error) => {
                warn!(
                    ?error,
                    "Server {server_id}: cannot use its level-name; scanning mods only and keeping datapack entries"
                );
                None
            }
        };
        let rows = mfcdb::get_server_mod_files_by_server(&self.app.db, server_id).await?;
        let mut folders = vec![("mods".to_owned(), "mods", "jar")];
        if let Some(world) = &world {
            folders.push((format!("{world}/datapacks"), "datapacks", "zip"));
        }
        // Locations that exist on disk in some form; their rows are never pruned.
        let mut seen = HashSet::new();
        // Folders listed completely, with the extension they hold; only these
        // may have stale rows pruned.
        let mut read_folders = HashMap::new();
        let mut by_location = HashMap::<String, Vec<ServerListedAddon>>::new();
        for (relative, kind, extension) in folders {
            // A linked folder (a `mods/` shared between servers, a world on
            // another drive) is listed through, as installs write through it.
            let folder = match world::folder(&data, &relative) {
                Ok(folder) => folder,
                Err(error) => {
                    warn!(
                        ?error,
                        "Server {server_id}: skipping addon folder {relative}"
                    );
                    continue;
                }
            };
            let mut entries = match tokio::fs::read_dir(&folder).await {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // Behind a link to a drive that is not plugged in, the
                    // folder is not known to be empty: its entries are kept.
                    match world::unavailable_link(&folder) {
                        Some(link) => warn!(
                            "Server {server_id}: keeping the entries of {relative}, as {} is a link to a folder that cannot be found",
                            link.display()
                        ),
                        None => {
                            read_folders.insert(relative, extension);
                        }
                    }
                    continue;
                }
                Err(error) => {
                    warn!(
                        ?error,
                        "Server {server_id}: cannot list {}",
                        folder.display()
                    );
                    continue;
                }
            };
            let mut complete = true;
            loop {
                let entry = match entries.next_entry().await {
                    Ok(Some(entry)) => entry,
                    Ok(None) => break,
                    Err(error) => {
                        warn!(
                            ?error,
                            "Server {server_id}: stopped listing {}",
                            folder.display()
                        );
                        complete = false;
                        break;
                    }
                };
                let filename = entry.file_name().to_string_lossy().to_string();
                if !addon_has_extension(&filename, extension) {
                    continue;
                }
                let enabled = !filename.ends_with(".disabled");
                let base = filename
                    .strip_suffix(".disabled")
                    .unwrap_or(&filename)
                    .to_owned();
                let location = format!("{relative}/{base}");
                let metadata = match entry.metadata().await {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        warn!(
                            ?error,
                            "Server {server_id}: cannot read {}",
                            entry.path().display()
                        );
                        seen.insert(location);
                        continue;
                    }
                };
                if !metadata.file_type().is_file() {
                    continue;
                }
                seen.insert(location.clone());
                by_location
                    .entry(location.clone())
                    .or_default()
                    .push(ServerListedAddon {
                        kind,
                        folder: folder.clone(),
                        filename: base,
                        location,
                        enabled,
                        size: metadata.len(),
                        stamp: observed_file_stamp(&metadata),
                        times: change_times(&metadata),
                    });
            }
            if complete {
                read_folders.insert(relative, extension);
            }
        }
        // Rows whose files are gone go now, before anything is read, so the
        // addon list never shows a removed jar next to its replacement while
        // the rest is read, nor keeps it when the scan is stopped.
        let pruned = self
            .prune_server_rows(server_id, &data, &rows, &seen, &read_folders)
            .await;
        drop(guard);
        if pruned != 0 {
            self.app
                .invalidate(GET_SERVER_ADDONS, Some(server_id.into()));
        }

        let mut listed = Vec::new();
        for (location, mut entries) in by_location {
            if entries.len() > 1 {
                warn!(
                    "Server {server_id}: both {location} and {location}.disabled exist; keeping its entry unchanged until one of them is removed"
                );
                continue;
            }
            listed.extend(entries.pop());
        }
        listed.sort_by(|a, b| a.location.cmp(&b.location));

        // Phase 2: hash new or changed files with the gate released, on a
        // worker that keeps going while a batch waits for the gate.
        let parallelism = (available_parallelism().map_or(1, |n| n.get()) / 2).max(1);
        let app = self.app.clone();
        let rows = Arc::new(rows);
        let mut results = ScanWorker::spawn(
            futures::stream::iter(listed)
                .map(move |addon| {
                    let (app, rows) = (app.clone(), rows.clone());
                    async move {
                        app.meta_cache_manager()
                            .server_scan_work(server_id, &rows, addon)
                            .await
                    }
                })
                .buffer_unordered(parallelism)
                .filter_map(futures::future::ready),
        );

        // Phase 3, once per batch: re-take the gate only to commit what is
        // still current.
        loop {
            let (batch, finished) = next_commit_batch(&mut results, || {}).await;
            let Some(orphaned) = self
                .commit_server_batch(server_id, server_shortpath, &data, world.as_deref(), batch)
                .await?
            else {
                return Ok(());
            };
            if !orphaned.is_empty() {
                let again = self.recreate_server_metadata(server_id, orphaned).await;
                let Some(lost) = self
                    .commit_server_batch(
                        server_id,
                        server_shortpath,
                        &data,
                        world.as_deref(),
                        again,
                    )
                    .await?
                else {
                    return Ok(());
                };
                if !lost.is_empty() {
                    warn!(
                        "Server {server_id}: {} scan result(s) lost their metadata twice; the next scan reads them again",
                        lost.len()
                    );
                }
            }
            if finished {
                return results.settled().await;
            }
        }
    }

    /// Create again, with the server gate released, the metadata of results
    /// whose new metadata was garbage-collected before they were committed.
    async fn recreate_server_metadata(
        self,
        server_id: i32,
        orphaned: Vec<ServerScanOutcome>,
    ) -> Vec<ServerScanOutcome> {
        let mut again = Vec::new();
        for outcome in orphaned {
            let ServerScanOutcome::Parsed {
                row, addon, result, ..
            } = outcome
            else {
                continue;
            };
            match self.ensure_mod_metadata(&result, &addon.filename).await {
                Ok(metadata_id) => again.push(ServerScanOutcome::Parsed {
                    row,
                    addon,
                    result,
                    metadata_id,
                }),
                Err(error) => warn!(?error, "Server {server_id}: could not store a scan result"),
            }
        }
        again
    }

    /// Remove the rows of server addons that are gone. The caller holds the
    /// server gate and has just listed the folders: `seen` holds every
    /// location that exists in some form, `read_folders` every folder listed
    /// completely, with the extension it holds. Returns how many were removed.
    async fn prune_server_rows(
        self,
        server_id: i32,
        data: &Path,
        rows: &[mfcdb::ServerModFileCacheRow],
        seen: &HashSet<String>,
        read_folders: &HashMap<String, &'static str>,
    ) -> usize {
        use crate::managers::server::world;
        let mut listings = FolderListings::default();
        let mut removed = 0;
        for row in rows {
            if seen.contains(&row.location) {
                continue;
            }
            let Some((relative, extension, name)) =
                read_folders.iter().find_map(|(relative, extension)| {
                    row.location
                        .strip_prefix(&format!("{relative}/"))
                        .filter(|name| !name.contains('/'))
                        .map(|name| (relative, *extension, name))
                })
            else {
                continue;
            };
            let Ok(folder) = world::folder(data, relative) else {
                continue;
            };
            // A name the scan never lists is stale even if such a file exists.
            if addon_has_extension(name, extension)
                && !listings.confirmed_absent(&folder, name).await
            {
                continue;
            }
            let expected = row.clone();
            let deleted = self
                .app
                .db
                .write(move |mut conn| {
                    let tx = conn.transaction()?;
                    let current = mfcdb::get_server_mod_file_cache_by_id_conn(&tx, &expected.id)?;
                    if current.as_ref() != Some(&expected) {
                        return Ok(false);
                    }
                    mfcdb::delete_server_mod_file_cache_by_id_conn(&tx, &expected.id)?;
                    tx.commit()?;
                    Ok(true)
                })
                .await;
            match deleted {
                Ok(true) => removed += 1,
                Ok(false) => {}
                Err(error) => warn!(?error, "Server {server_id}: could not remove a stale entry"),
            }
        }
        removed
    }

    /// Decide what one listed server addon needs and do it, without the gate.
    /// `None`: nothing to store (unchanged, or unreadable and logged).
    async fn server_scan_work(
        self,
        server_id: i32,
        rows: &[mfcdb::ServerModFileCacheRow],
        addon: ServerListedAddon,
    ) -> Option<ServerScanOutcome> {
        let row = rows
            .iter()
            .find(|row| row.addon_type == addon.kind && row.location == addon.location)
            .cloned();
        let path = addon.folder.join(addon.disk_name());
        if let Some(row) = &row {
            if row.enabled == addon.enabled && row.filesize as u64 == addon.size {
                let observed = row.file_stamp.as_deref().and_then(qualified_observation);
                if observed == Some(addon.stamp.as_str()) {
                    return None;
                }
                if observed.is_some() {
                    match self
                        .verify_cached_bytes(row.file_stamp.as_deref(), &row.metadata_id, &path)
                        .await
                    {
                        Ok(ByteCheck::Same(observed)) => {
                            return Some(ServerScanOutcome::Stamp {
                                row: row.clone(),
                                addon,
                                observed,
                            });
                        }
                        Ok(ByteCheck::Different) => {}
                        Err(error) => {
                            warn!(
                                ?error,
                                "Server {server_id}: cannot check {}",
                                path.display()
                            );
                            return None;
                        }
                    }
                }
                if row.file_stamp.is_none() {
                    match self
                        .settle_legacy_row(
                            &row.metadata_id,
                            row.last_updated_at,
                            &addon.stamp,
                            addon.times,
                            &path,
                            false,
                        )
                        .await
                    {
                        Ok(Some(observed)) => {
                            return Some(ServerScanOutcome::Stamp {
                                row: row.clone(),
                                addon,
                                observed,
                            });
                        }
                        Ok(None) => {}
                        Err(error) => {
                            warn!(
                                ?error,
                                "Server {server_id}: cannot check {}",
                                path.display()
                            );
                            return None;
                        }
                    }
                }
            }
        }
        let parsed = async {
            let result = self
                .hash_and_parse_mod_file(&addon.folder, &addon.filename, addon.enabled)
                .await?;
            let metadata_id = self.ensure_mod_metadata(&result, &addon.filename).await?;
            Ok::<_, anyhow::Error>((result, metadata_id))
        }
        .await;
        match parsed {
            Ok((result, metadata_id)) => Some(ServerScanOutcome::Parsed {
                row,
                addon,
                result,
                metadata_id,
            }),
            Err(error) => {
                warn!(
                    ?error,
                    "Server {server_id}: could not scan {}",
                    path.display()
                );
                None
            }
        }
    }

    /// Commit one batch of a server scan under the server gate. `None`: the
    /// server moved or was removed since the snapshot, so the scan stops.
    /// Otherwise the results whose new metadata was garbage-collected before
    /// this commit; the gate is never held while metadata is created.
    async fn commit_server_batch(
        self,
        server_id: i32,
        server_shortpath: &str,
        data: &Path,
        world: Option<&str>,
        outcomes: Vec<ServerScanOutcome>,
    ) -> anyhow::Result<Option<Vec<ServerScanOutcome>>> {
        use crate::managers::server::world;
        #[cfg(test)]
        self.scan_test_pause(Path::new(&format!("server-{server_id}")), "commit")
            .await;
        let mut orphaned = Vec::new();
        if outcomes.is_empty() {
            return Ok(Some(orphaned));
        }
        let lock = self
            .app
            .server_manager()
            .get_op_lock(crate::domain::server::ServerId(server_id));
        let _guard = lock.lock().await;
        match carbon_repos::repos::server::get_server(&self.app.db, server_id).await? {
            Some(current) if current.shortpath == server_shortpath => {}
            _ => {
                info!("Server {server_id} moved or was removed during its addon scan");
                return Ok(None);
            }
        }
        let world_now = world::selected(data).ok();
        let datapacks_current = world.is_some() && world_now.as_deref() == world;
        let mut changed = 0;
        for outcome in outcomes {
            let kind = match &outcome {
                ServerScanOutcome::Stamp { addon, .. }
                | ServerScanOutcome::Parsed { addon, .. } => addon.kind,
            };
            if kind == "datapacks" && !datapacks_current {
                continue;
            }
            let committed = match &outcome {
                ServerScanOutcome::Stamp {
                    row,
                    addon,
                    observed,
                } => self.commit_server_stamp(row, addon, observed).await,
                ServerScanOutcome::Parsed {
                    row,
                    addon,
                    result,
                    metadata_id,
                } => {
                    self.commit_server_parsed(server_id, row, addon, result, metadata_id)
                        .await
                }
            };
            match committed {
                Ok(CommitOutcome::Written) => changed += 1,
                Ok(CommitOutcome::Unchanged | CommitOutcome::Discarded) => {}
                Ok(CommitOutcome::Orphaned) => orphaned.push(outcome),
                Err(error) => warn!(?error, "Server {server_id}: could not store a scan result"),
            }
        }
        if changed != 0 {
            self.app
                .invalidate(GET_SERVER_ADDONS, Some(server_id.into()));
        }
        Ok(Some(orphaned))
    }

    async fn commit_server_stamp(
        self,
        row: &mfcdb::ServerModFileCacheRow,
        addon: &ServerListedAddon,
        observed: &str,
    ) -> anyhow::Result<CommitOutcome> {
        let path = addon.folder.join(addon.disk_name());
        if current_file_stamp(&path).await.as_deref() != Some(observed) {
            return Ok(CommitOutcome::Discarded);
        }
        let expected = row.clone();
        let observed = observed.to_owned();
        Ok(self
            .app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                let current = mfcdb::get_server_mod_file_cache_by_id_conn(&tx, &expected.id)?;
                if current.as_ref() != Some(&expected) {
                    return Ok(CommitOutcome::Discarded);
                }
                let Some(metadata) = metarepo::get_metadata_by_id_conn(&tx, &expected.metadata_id)?
                else {
                    return Ok(CommitOutcome::Discarded);
                };
                let stamp = qualified_file_stamp(&metadata, &observed);
                if expected.file_stamp.as_deref() == Some(stamp.as_str()) {
                    return Ok(CommitOutcome::Unchanged);
                }
                mfcdb::update_server_mod_file_stamp_conn(
                    &tx,
                    expected.server_id,
                    &expected.addon_type,
                    &expected.location,
                    &expected.metadata_id,
                    Some(&stamp),
                    DbDateTime(Utc::now().fixed_offset()),
                )?;
                tx.commit()?;
                Ok(CommitOutcome::Written)
            })
            .await?)
    }

    /// Publish a freshly parsed server addon. Never creates metadata: the
    /// caller holds the server gate, and an install holding that gate may be
    /// waiting for the icon permit or hash lock that creating it takes.
    async fn commit_server_parsed(
        self,
        server_id: i32,
        row: &Option<mfcdb::ServerModFileCacheRow>,
        addon: &ServerListedAddon,
        result: &ModFileParseResult,
        metadata_id: &str,
    ) -> anyhow::Result<CommitOutcome> {
        let path = addon.folder.join(addon.disk_name());
        let current = current_file_stamp(&path).await;
        if current.is_none() || current != result.file_stamp {
            return Ok(CommitOutcome::Discarded);
        }
        match publish_server_row(
            &self.app.db,
            server_id,
            &addon.filename,
            &addon.location,
            addon.enabled,
            addon.kind,
            metadata_id,
            result,
            Some(row.clone()),
        )
        .await
        {
            Ok(true) => Ok(CommitOutcome::Written),
            Ok(false) => Ok(CommitOutcome::Discarded),
            Err(error) if error.is_foreign_key_violation() => Ok(CommitOutcome::Orphaned),
            Err(error) => Err(error.into()),
        }
    }

    /// Check if a specific entity is currently being cached
    pub async fn is_entity_being_cached(&self, entity_id: CacheEntityId) -> bool {
        let currently_caching = self.get_currently_caching_entities().await;
        currently_caching.contains(&entity_id)
    }

    /// Leaves the addon at `path` (named with its extension) out of both
    /// platforms' lookups, as a lookup that found no match for it does, and
    /// returns its CurseForge fingerprint.
    #[cfg(test)]
    pub(crate) async fn miss_lookups(&self, path: &Path) -> u32 {
        let parsed = self
            .hash_and_parse_mod_file(
                &path.parent().unwrap().to_path_buf(),
                path.file_name().unwrap().to_str().unwrap(),
                true,
            )
            .await
            .unwrap();
        let now = self.now();
        self.ignored_remote_mr_hashes
            .write()
            .await
            .ignore([hex::encode(parsed.sha512)], now);
        self.ignored_remote_cf_hashes
            .write()
            .await
            .ignore([parsed.murmur2], now);
        parsed.murmur2
    }
}

/// Whole-file reads per path, for tests that count how often a file is read.
#[cfg(test)]
static WHOLE_FILE_READS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn note_whole_file_read(path: &Path) {
    WHOLE_FILE_READS.lock().unwrap().push(path.to_owned());
}

/// How many times the whole of `path` has been read so far.
#[cfg(test)]
pub(crate) fn whole_file_reads(path: &Path) -> usize {
    WHOLE_FILE_READS
        .lock()
        .unwrap()
        .iter()
        .filter(|read| *read == path)
        .count()
}

/// Hash `path` once. The stamp is returned only when nothing about the file
/// changed during the read: the opened handle's stamp before it equals the
/// path's stamp after it, as in [`MetaCacheManager::hash_and_parse_mod_file`].
pub(crate) async fn read_stable_file_hash(
    path: &Path,
) -> std::io::Result<(Vec<u8>, Option<String>)> {
    let mut file = tokio::fs::File::open(path).await?;
    let before = observed_file_stamp(&file.metadata().await?);
    let mut digest = Sha512::new();
    #[cfg(test)]
    note_whole_file_read(path);
    carbon_scheduler::buffered_digest(&mut file, |bytes| digest.update(bytes)).await?;
    drop(file);
    let after = current_file_stamp(path).await;
    let stable = (after.as_deref() == Some(before.as_str())).then_some(before);
    Ok((digest.finalize().to_vec(), stable))
}

fn qualified_file_stamp(metadata: &metarepo::ModMetadataRow, observed: &str) -> String {
    // Bind parser qualification to local metadata, including the loader field
    // repaired by the NeoForge correction. Legacy raw/NULL stamps do not qualify.
    let fields = serde_json::to_vec(&(
        &metadata.id,
        &metadata.modloaders,
        &metadata.modid,
        &metadata.name,
        &metadata.version,
        &metadata.description,
        &metadata.authors,
        metadata.murmur2,
        &metadata.sha512,
        &metadata.sha1,
    ))
    .expect("local metadata fields serialize");
    format!(
        "parsed-v1:{}|{observed}",
        hex::encode(Sha512::digest(fields))
    )
}

fn qualified_observation(stamp: &str) -> Option<&str> {
    stamp
        .strip_prefix("parsed-v1:")?
        .split_once('|')
        .map(|v| v.1)
}

fn observed_file_stamp(metadata: &std::fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok());
    // No device number: it changes when an external or network drive is
    // mounted again under another number, and the path already names the
    // filesystem. No inode number either: Linux numbers FAT and exFAT files
    // (and some FUSE and network mounts) again whenever it loads them, so
    // every addon would be read again after each reboot or remount. A
    // replaced file still changes its change time, which no program can set.
    // Moving, copying or restoring the data folder, or a recursive chmod, also
    // moves every change time: the next scan then reads each addon once.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{:?}:{}:{}:{}",
            modified,
            metadata.len(),
            metadata.ctime(),
            metadata.ctime_nsec()
        )
    }
    #[cfg(not(unix))]
    {
        format!(
            "{:?}:{:?}:{}",
            modified,
            metadata.created().ok(),
            metadata.len()
        )
    }
}

/// Make an instance row's stored stamp describe `path` as it is now, as if
/// the file had been replaced without changing anything a stamp observes
/// (possible on Windows, which has no change time).
#[cfg(test)]
pub(crate) async fn restamp_for_test(
    app: &App,
    instance_id: InstanceId,
    kind: &str,
    filename: &str,
    path: &Path,
) {
    let row =
        mfcdb::get_mod_file_cache_by_instance_kind_filename(&app.db, *instance_id, kind, filename)
            .await
            .unwrap()
            .unwrap();
    let metadata = metarepo::get_metadata_by_id(&app.db, &row.metadata_id)
        .await
        .unwrap()
        .unwrap();
    let stamp = qualified_file_stamp(
        &metadata,
        &observed_file_stamp(&std::fs::metadata(path).unwrap()),
    );
    mfcdb::update_mod_file_stamp(
        &app.db,
        *instance_id,
        kind,
        filename,
        &row.metadata_id,
        Some(&stamp),
        DbDateTime(Utc::now().fixed_offset()),
    )
    .await
    .unwrap();
}

/// Whether an instance scan lists an entry of this name in `kind`'s folder.
/// Worlds are directories of any name.
fn listable_addon_name(kind: crate::domain::instance::AddonType, name: &str) -> bool {
    use crate::domain::instance::AddonType;
    let extensions: &[&str] = match kind {
        AddonType::Worlds => return true,
        AddonType::Mods => &["jar", "zip"],
        AddonType::ResourcePacks | AddonType::Shaders | AddonType::DataPacks => &["zip"],
    };
    extensions.iter().any(|ext| addon_has_extension(name, ext))
}

/// Whether `filename`'s extension matches `ext`, ignoring case (a
/// CurseForge/Modrinth-supplied file can arrive as `Pack.ZIP`), and
/// tolerating a trailing `.disabled` suffix — appended by this app to mark an
/// installed addon disabled without deleting it, so it must not itself count
/// as the extension.
fn addon_has_extension(filename: &str, ext: &str) -> bool {
    let stripped = filename.strip_suffix(".disabled").unwrap_or(filename);
    Path::new(stripped)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

async fn thumbnail_retry_allowed<K: Eq + std::hash::Hash>(
    failures: &RwLock<HashMap<K, (std::time::Instant, u32)>>,
    project: &K,
) -> bool {
    // Retain the attempt count across eligible retries. The read guard is
    // dropped before any download, semaphore, or writer acquisition.
    failures
        .read()
        .await
        .get(project)
        .is_none_or(|(deadline, _)| *deadline <= std::time::Instant::now())
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

/// One addon as listed under the instance gate.
#[derive(Debug, Clone)]
struct ListedAddon {
    kind: crate::domain::instance::AddonType,
    /// Name without the `.disabled` suffix.
    filename: String,
    enabled: bool,
    size: u64,
    /// Observed stamp of a regular file. Links take the full parse path, whose
    /// stamps describe the link target; worlds have no content stamp.
    stamp: Option<String>,
    times: Option<(std::time::SystemTime, std::time::SystemTime)>,
}

impl ListedAddon {
    fn disk_name(&self) -> String {
        if self.enabled || self.kind == crate::domain::instance::AddonType::Worlds {
            self.filename.clone()
        } else {
            format!("{}.disabled", self.filename)
        }
    }
}

/// Rows and folder listings of one instance, taken under its gate.
struct InstanceSnapshot {
    shortpath: String,
    folders: Arc<HashMap<&'static str, PathBuf>>,
    rows: Vec<mfcdb::ModFileCacheRow>,
    listed: HashMap<(String, String), ListedAddon>,
    /// Folders that could not be listed completely, with the reason. Their
    /// rows are kept: a missing name there proves nothing.
    unobserved: HashMap<String, String>,
}

enum InstanceScanWork {
    /// Re-read the bytes of a file in the requested verify scope.
    Verify(mfcdb::ModFileCacheRow, ListedAddon),
    /// A row from before stamps existed; may be stamped from `stat` alone,
    /// or with `true` (inside the verify scope) after one read of its bytes.
    Legacy(mfcdb::ModFileCacheRow, ListedAddon, bool),
    /// New, changed, linked or unqualified: hash and parse it.
    Parse(Option<mfcdb::ModFileCacheRow>, ListedAddon),
}

enum InstanceScanOutcome {
    /// The row still describes the file; record this stamp for it.
    Stamp(mfcdb::ModFileCacheRow, ListedAddon, String),
    Parsed {
        row: Option<mfcdb::ModFileCacheRow>,
        addon: ListedAddon,
        result: ModFileParseResult,
        metadata_id: String,
    },
    /// Could not be read; logged, and its row (if any) is kept.
    Failed,
}

enum ByteCheck {
    /// The bytes equal the row's metadata; carries the file's stamp.
    Same(String),
    Different,
}

/// Modification time and change (Unix) or creation (elsewhere) time.
fn change_times(
    metadata: &std::fs::Metadata,
) -> Option<(std::time::SystemTime, std::time::SystemTime)> {
    let modified = metadata.modified().ok()?;
    #[cfg(unix)]
    let changed = {
        use std::os::unix::fs::MetadataExt;
        std::time::UNIX_EPOCH.checked_add(std::time::Duration::new(
            u64::try_from(metadata.ctime()).ok()?,
            u32::try_from(metadata.ctime_nsec()).ok()?,
        ))?
    };
    #[cfg(not(unix))]
    let changed = metadata.created().ok()?;
    Some((modified, changed))
}

/// Stamp of whatever `path` names now, following links like the parser does.
pub(crate) async fn current_file_stamp(path: &Path) -> Option<String> {
    tokio::fs::metadata(path)
        .await
        .ok()
        .map(|metadata| observed_file_stamp(&metadata))
}

/// Whether `path` still has the stamp an addon row stored (`stored`, the row's
/// `fileStamp`), i.e. holds the bytes the row's metadata names as far as a
/// stamp can tell, without reading it. The file is opened, so a missing or
/// unreadable file is an error here rather than a mismatch. A row without a
/// stamp of this parser revision never matches.
pub(crate) async fn opened_stamp_matches(
    path: &Path,
    stored: Option<&str>,
) -> std::io::Result<bool> {
    let file = tokio::fs::File::open(path).await?;
    let observed = observed_file_stamp(&file.metadata().await?);
    Ok(stored.and_then(qualified_observation) == Some(observed.as_str()))
}

/// Entry names of addon folders exactly as the folder lists them, read once
/// per folder under the gate before stale rows are removed.
///
/// Membership is by exact name, never by probing a path: on case-insensitive
/// filesystems (Windows, macOS) `JEI.jar` resolves to a file renamed to
/// `jei.jar`, which would keep the old row next to the new one forever.
#[derive(Default)]
struct FolderListings(HashMap<PathBuf, Option<HashSet<String>>>);

impl FolderListings {
    /// Whether neither spelling of an addon is listed in `folder`, links
    /// included. A folder that cannot be listed completely keeps its rows; a
    /// missing folder lists nothing, unless it is behind a link to a folder
    /// that cannot be found (a drive that is not plugged in), which keeps
    /// them.
    async fn confirmed_absent(&mut self, folder: &Path, name: &str) -> bool {
        if !self.0.contains_key(folder) {
            let listing = Self::list(folder).await;
            self.0.insert(folder.to_owned(), listing);
        }
        match &self.0[folder] {
            Some(names) => !names.contains(name) && !names.contains(&format!("{name}.disabled")),
            None => false,
        }
    }

    async fn list(folder: &Path) -> Option<HashSet<String>> {
        let mut entries = match tokio::fs::read_dir(folder).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return crate::managers::server::world::unavailable_link(folder)
                    .is_none()
                    .then(HashSet::new);
            }
            Err(_) => return None,
        };
        let mut names = HashSet::new();
        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => {
                    names.insert(entry.file_name().to_string_lossy().into_owned());
                }
                Ok(None) => return Some(names),
                Err(_) => return None,
            }
        }
    }
}

/// Scan results are committed in batches of at most this many, so a pass
/// that is paused (Play), pre-empted (another instance's Addons tab, an
/// install) or stopped (Quit) keeps what it already read, and the addon list
/// fills in while a large folder is read, as when each file was stored on its
/// own.
const SCAN_COMMIT_BATCH: usize = 32;
/// A result waits at most this long for its batch to fill.
const SCAN_COMMIT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// Collect scan outcomes until a commit is due: [`SCAN_COMMIT_BATCH`] of
/// them, [`SCAN_COMMIT_INTERVAL`] after the first, or the end of the work.
/// Returns the batch and whether the work is finished.
async fn next_commit_batch<T>(
    results: &mut (impl futures::Stream<Item = T> + Unpin),
    mut progress: impl FnMut(),
) -> (Vec<T>, bool) {
    use futures::StreamExt;
    let mut batch = Vec::new();
    let Some(first) = results.next().await else {
        return (batch, true);
    };
    progress();
    batch.push(first);
    let due = TokioInstant::now() + SCAN_COMMIT_INTERVAL;
    while batch.len() < SCAN_COMMIT_BATCH {
        match tokio::time::timeout_at(due, results.next()).await {
            Ok(Some(outcome)) => {
                progress();
                batch.push(outcome);
            }
            Ok(None) => return (batch, true),
            Err(_) => break,
        }
    }
    (batch, false)
}

/// The hashing and parsing of one scan pass, run on its own task.
///
/// It keeps running while the pass waits for the instance or server gate to
/// commit a batch. Work it has in flight may hold the launcher-wide icon
/// permit or a per-hash metadata lock, and an install holding the gate waits
/// for exactly those: if the work stopped whenever the committer waited, the
/// two would wait for each other forever. Dropping the worker (the pass was
/// paused for gameplay, pre-empted or stopped) stops the work.
struct ScanWorker<T> {
    results: tokio::sync::mpsc::UnboundedReceiver<T>,
    task: tokio::task::JoinHandle<()>,
}

impl<T: Send + 'static> ScanWorker<T> {
    fn spawn(work: impl futures::Stream<Item = T> + Send + 'static) -> Self {
        use futures::StreamExt;
        let (sender, results) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut work = std::pin::pin!(work);
            while let Some(result) = work.next().await {
                if sender.send(result).is_err() {
                    break;
                }
            }
        });
        Self { results, task }
    }

    /// After the last result: whether the work ended normally.
    async fn settled(&mut self) -> anyhow::Result<()> {
        (&mut self.task)
            .await
            .map_err(|error| anyhow!("Addon scan stopped unexpectedly: {error}"))
    }
}

impl<T> futures::Stream for ScanWorker<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<T>> {
        self.results.poll_recv(cx)
    }
}

impl<T> Drop for ScanWorker<T> {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// How one scan result fared at commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitOutcome {
    Written,
    /// The row already says exactly this.
    Unchanged,
    /// The file or row changed after the snapshot, so the result is dropped.
    Discarded,
    /// The metadata created for it was garbage-collected before the commit
    /// (an addon, instance or server was deleted meanwhile). The pass creates
    /// it again with the gate released and commits once more.
    Orphaned,
}

/// One server addon as listed under the server gate.
#[derive(Debug, Clone)]
struct ServerListedAddon {
    kind: &'static str,
    folder: PathBuf,
    /// Name without the `.disabled` suffix.
    filename: String,
    location: String,
    enabled: bool,
    size: u64,
    stamp: String,
    times: Option<(std::time::SystemTime, std::time::SystemTime)>,
}

impl ServerListedAddon {
    fn disk_name(&self) -> String {
        if self.enabled {
            self.filename.clone()
        } else {
            format!("{}.disabled", self.filename)
        }
    }
}

enum ServerScanOutcome {
    Stamp {
        row: mfcdb::ServerModFileCacheRow,
        addon: ServerListedAddon,
        observed: String,
    },
    Parsed {
        row: Option<mfcdb::ServerModFileCacheRow>,
        addon: ServerListedAddon,
        result: ModFileParseResult,
        metadata_id: String,
    },
}

/// Upsert an instance row and its qualified stamp in one transaction; used by
/// installer publication and by scans alike, so both write identical stamps.
/// With `expected`, the write happens only while the row is still exactly what
/// the scan saw, so a concurrent toggle, delete or install always wins.
#[allow(clippy::too_many_arguments)]
async fn publish_instance_row(
    db: &carbon_repos::db_exec::Db,
    instance_id: InstanceId,
    filename: &str,
    kind: &str,
    enabled: bool,
    metadata_id: &str,
    result: &ModFileParseResult,
    expected: Option<Option<mfcdb::ModFileCacheRow>>,
) -> carbon_repos::db_error::DbResult<bool> {
    let (filename, kind, metadata_id) =
        (filename.to_owned(), kind.to_owned(), metadata_id.to_owned());
    let observed = result.file_stamp.clone();
    let filesize = result.content_len as i32;
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        if let Some(expected) = expected {
            let current = mfcdb::get_mod_file_cache_by_instance_kind_filename_conn(
                &tx,
                *instance_id,
                &kind,
                &filename,
            )?;
            if current != expected {
                return Ok(false);
            }
        }
        mfcdb::upsert_mod_file_cache_conn(
            &tx,
            *instance_id,
            &filename,
            filesize,
            enabled,
            &kind,
            &metadata_id,
            DbDateTime(Utc::now().fixed_offset()),
        )?;
        let metadata = metarepo::get_metadata_by_id_conn(&tx, &metadata_id)?
            .expect("validated owner in the same transaction");
        let stamp = observed
            .as_deref()
            .map(|value| qualified_file_stamp(&metadata, value));
        mfcdb::update_mod_file_stamp_conn(
            &tx,
            *instance_id,
            &kind,
            &filename,
            &metadata_id,
            stamp.as_deref(),
            DbDateTime(Utc::now().fixed_offset()),
        )?;
        tx.commit()?;
        Ok(true)
    })
    .await
}

/// Upsert a server row and its qualified stamp in one transaction. With
/// `expected`, the write happens only while the row is still exactly what the
/// scan saw, so a concurrent toggle, delete or install always wins.
#[allow(clippy::too_many_arguments)]
async fn publish_server_row(
    db: &carbon_repos::db_exec::Db,
    server_id: i32,
    filename: &str,
    location: &str,
    enabled: bool,
    kind: &str,
    metadata_id: &str,
    result: &ModFileParseResult,
    expected: Option<Option<mfcdb::ServerModFileCacheRow>>,
) -> carbon_repos::db_error::DbResult<bool> {
    let (filename, location, kind, metadata_id) = (
        filename.to_owned(),
        location.to_owned(),
        kind.to_owned(),
        metadata_id.to_owned(),
    );
    let observed = result.file_stamp.clone();
    let filesize = result.content_len as i32;
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        if let Some(expected) = expected {
            let current = mfcdb::get_server_mod_file_cache_by_location_conn(
                &tx, server_id, &kind, &location,
            )?;
            if current != expected {
                return Ok(false);
            }
        }
        mfcdb::upsert_server_mod_file_cache_at_location_conn(
            &tx,
            server_id,
            &filename,
            &location,
            filesize,
            enabled,
            &kind,
            &metadata_id,
            DbDateTime(Utc::now().fixed_offset()),
        )?;
        let metadata = metarepo::get_metadata_by_id_conn(&tx, &metadata_id)?
            .expect("validated owner in the same transaction");
        let stamp = observed
            .as_deref()
            .map(|value| qualified_file_stamp(&metadata, value));
        mfcdb::update_server_mod_file_stamp_conn(
            &tx,
            server_id,
            &kind,
            &location,
            &metadata_id,
            stamp.as_deref(),
            DbDateTime(Utc::now().fixed_offset()),
        )?;
        tx.commit()?;
        Ok(true)
    })
    .await
}

/// Scan one instance's addon folders and bring `ModFileCache` in line.
///
/// The instance gate (Play, toggles, deletes, settings, installs) is held only
/// to snapshot rows and folder listings and later to commit, never while files
/// are hashed. Files outside `verify` are trusted on their parser-qualified
/// stamp. An unreadable file or folder never fails the pass: the file stays
/// uncached and rows in folders that could not be listed are kept.
async fn cache_instance(
    app: &App,
    update_notifier: &UpdateNotifier,
    instance_id: InstanceId,
    verify: VerifyScope,
) -> anyhow::Result<()> {
    let activity = app.runtime_lifecycle.admit("local addon scan")?;
    let cancel = activity.cancellation();
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(anyhow!(
            "Addon scan of instance {instance_id} stopped because the launcher is closing"
        )),
        result = scan_instance(app, update_notifier, instance_id, verify) => result,
    };
    drop(activity);
    result
}

async fn snapshot_instance(
    app: &App,
    instance_id: InstanceId,
    scanning_subtask: &crate::managers::vtask::Subtask,
) -> anyhow::Result<InstanceSnapshot> {
    use crate::domain::instance::AddonType;
    let mcm = app.meta_cache_manager();
    let shortpath = {
        let instance_manager = app.instance_manager();
        let instances = instance_manager.instances.read().await;
        let Some(instance) = instances.get(&instance_id) else {
            anyhow::bail!("Cannot verify addons: instance {instance_id} no longer exists");
        };
        instance.shortpath.clone()
    };
    let instance_path = InstancesPath::subpath().get_instance_path(&shortpath);
    let root_path = app.settings_manager().runtime_path.get_root().to_path();
    let mut folders = HashMap::new();
    let mut listed = HashMap::<(String, String), ListedAddon>::new();
    let mut unobserved = HashMap::<String, String>::new();
    let addon_types = AddonType::all();
    scanning_subtask.update_items(0, addon_types.len() as u32);
    for (index, addon_type) in addon_types.iter().copied().enumerate() {
        let kind = addon_type.to_db_string();
        let pathbuf = root_path.join(addon_type.get_folder_path(&instance_path));
        folders.insert(kind, pathbuf.clone());
        scanning_subtask.update_items(index as u32, addon_types.len() as u32);

        match mcm.scan_folder_metadata(&pathbuf).await {
            Ok(metadata) if metadata.is_dir() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            result => {
                let detail = match result {
                    Ok(_) => "expected an addon directory".to_owned(),
                    Err(error) => error.to_string(),
                };
                unobserved.insert(kind.to_owned(), format!("{}: {detail}", pathbuf.display()));
                continue;
            }
        }
        let mut entries = match mcm.scan_read_dir(&pathbuf).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                unobserved.insert(kind.to_owned(), format!("{}: {error}", pathbuf.display()));
                continue;
            }
        };
        loop {
            let entry = match mcm.scan_next_entry(&mut entries, &pathbuf).await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(error) => {
                    unobserved.insert(kind.to_owned(), format!("{}: {error}", pathbuf.display()));
                    break;
                }
            };
            let file_name = entry.file_name();
            let Some(utf8_name) = file_name.to_str() else {
                continue;
            };
            if !listable_addon_name(addon_type, utf8_name) {
                continue;
            }
            let metadata = match mcm.scan_entry_metadata(&entry).await {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    unobserved.insert(
                        kind.to_owned(),
                        format!("{}: {error}", entry.path().display()),
                    );
                    continue;
                }
            };
            if addon_type == AddonType::Worlds {
                if !metadata.is_dir() {
                    continue;
                }
                match crate::managers::instance::installer::publication::world_install_incomplete(
                    &entry.path(),
                )
                .await
                {
                    Ok(false) => {}
                    Ok(true) => continue,
                    Err(error) => {
                        unobserved.insert(
                            kind.to_owned(),
                            format!("{}: {error:#}", entry.path().display()),
                        );
                        continue;
                    }
                }
                listed.insert(
                    (kind.to_owned(), utf8_name.to_owned()),
                    ListedAddon {
                        kind: addon_type,
                        filename: utf8_name.to_owned(),
                        enabled: true,
                        size: metadata.len(),
                        stamp: None,
                        times: None,
                    },
                );
            } else {
                // file || symlink
                if metadata.is_dir() {
                    continue;
                }
                let enabled = !utf8_name.ends_with(".disabled");
                let base = utf8_name.strip_suffix(".disabled").unwrap_or(utf8_name);
                // DirEntry metadata describes a link, while the parser and the
                // final checks describe its target. Links always take the full
                // parse path, whose stamps are consistent with each other.
                let regular = !metadata.file_type().is_symlink();
                listed.insert(
                    (kind.to_owned(), base.to_owned()),
                    ListedAddon {
                        kind: addon_type,
                        filename: base.to_owned(),
                        enabled,
                        size: metadata.len(),
                        stamp: regular.then(|| observed_file_stamp(&metadata)),
                        times: regular.then(|| change_times(&metadata)).flatten(),
                    },
                );
            }
        }
    }
    scanning_subtask.complete_items();

    #[cfg(test)]
    mcm.scan_test_boundary(
        &PathBuf::from(format!("instance-{}", *instance_id)),
        "cache-read",
    )
    .context("Could not read cached addon owners")?;
    let rows = mfcdb::get_mod_files_by_instance(&app.db, *instance_id)
        .await
        .context("Could not read cached addon owners")?;
    Ok(InstanceSnapshot {
        shortpath,
        folders: Arc::new(folders),
        rows,
        listed,
        unobserved,
    })
}

/// What the commits of one instance scan did, for its log line.
#[derive(Default, Clone, Copy)]
struct InstanceCommitCounts {
    written: usize,
    discarded: usize,
    failed: usize,
}

impl InstanceCommitCounts {
    fn add(&mut self, other: Self) {
        self.written += other.written;
        self.discarded += other.discarded;
        self.failed += other.failed;
    }
}

async fn scan_instance(
    app: &App,
    update_notifier: &UpdateNotifier,
    instance_id: InstanceId,
    verify: VerifyScope,
) -> anyhow::Result<()> {
    use crate::domain::instance::AddonType;
    use futures::StreamExt;
    let instance_manager = app.instance_manager();
    let instance_name = instance_manager
        .instances
        .read()
        .await
        .get(&instance_id)
        .map(|instance| instance.shortpath.clone())
        .unwrap_or_else(|| format!("Instance {}", *instance_id));
    let task = VisualTask::new(Translation::CacheTaskLocal { instance_name });
    let _task_id = app.task_manager().spawn_task(&task).await;
    info!("Starting local mod caching for instance {instance_id} (verify {verify:?})");

    // An instance renamed while its files were hashed is scanned again at its
    // new folder: the results name paths that no longer exist.
    let mut counts = InstanceCommitCounts::default();
    // Rows of files that are gone are removed in phase 1 only, never by a
    // batch commit.
    let mut removed_total = 0;
    let mut attempt = 0;
    'scan: loop {
        attempt += 1;
        // Phase 1: snapshot rows and listings under the gate, and remove the
        // rows of files that are gone before anything is read, so the list
        // never shows a removed addon next to its replacement while the rest
        // is read, nor keeps it when the pass is paused for Play.
        let scanning_subtask = task.subtask(Translation::CacheSubtaskScanningFiles);
        let (snapshot, removed) = {
            let _operation = instance_manager.lock_instance(instance_id).await;
            #[cfg(test)]
            instance_manager
                .pause_addon_scan_for_test(instance_id)
                .await;
            let snapshot = snapshot_instance(app, instance_id, &scanning_subtask).await?;
            let stale = snapshot
                .rows
                .iter()
                .filter(|row| {
                    !snapshot.unobserved.contains_key(&row.addon_type)
                        && !snapshot
                            .listed
                            .contains_key(&(row.addon_type.clone(), row.filename.clone()))
                })
                .cloned()
                .collect::<Vec<_>>();
            let removed =
                remove_stale_instance_rows(app, instance_id, &snapshot.folders, stale).await;
            (snapshot, removed)
        };
        if removed != 0 {
            update_notifier.send(CacheEntityId::Instance(instance_id));
            removed_total += removed;
        }
        for (kind, reason) in &snapshot.unobserved {
            warn!(
                "Instance {instance_id}: could not list all {kind}; keeping their entries ({reason})"
            );
        }

        // Phase 2: decide per file, then hash and parse without the gate.
        let mut listed = snapshot.listed.clone();
        let mut work = Vec::new();
        for row in &snapshot.rows {
            let key = (row.addon_type.clone(), row.filename.clone());
            let Some(addon) = listed.remove(&key) else {
                continue;
            };
            if addon.enabled != row.enabled || addon.size != row.filesize as u64 {
                work.push(InstanceScanWork::Parse(Some(row.clone()), addon));
                continue;
            }
            if addon.kind == AddonType::Worlds {
                continue;
            }
            let Some(stamp) = addon.stamp.as_deref() else {
                work.push(InstanceScanWork::Parse(Some(row.clone()), addon));
                continue;
            };
            let observed = row.file_stamp.as_deref().and_then(qualified_observation);
            if verify.contains(addon.kind) {
                if row.file_stamp.is_none() {
                    // A 2.0.40 row: one read settles it, as for stamped rows.
                    work.push(InstanceScanWork::Legacy(row.clone(), addon, true));
                } else {
                    work.push(InstanceScanWork::Verify(row.clone(), addon));
                }
            } else if observed == Some(stamp) {
                continue;
            } else if observed.is_some() {
                // Touched (copied back, toggled on Unix, re-timestamped): one read
                // settles whether the bytes still match before a full parse.
                work.push(InstanceScanWork::Verify(row.clone(), addon));
            } else if row.file_stamp.is_none() {
                work.push(InstanceScanWork::Legacy(row.clone(), addon, false));
            } else {
                work.push(InstanceScanWork::Parse(Some(row.clone()), addon));
            }
        }
        let mut new_files = listed.into_iter().collect::<Vec<_>>();
        new_files.sort_by(|a, b| a.0.cmp(&b.0));
        work.extend(
            new_files
                .into_iter()
                .map(|(_, addon)| InstanceScanWork::Parse(None, addon)),
        );

        // About half the cores, and at least one so a single-core host
        // progresses. The work runs on a worker that keeps going while a batch
        // waits for the gate.
        let parallelism = (available_parallelism().map_or(1, |n| n.get()) / 2).max(1);
        let total = work.len() as u32;
        let finalization_subtask = (total != 0).then(|| {
            let subtask = task.subtask(Translation::CacheSubtaskFinalizingCache);
            subtask.update_items(0, total);
            subtask
        });
        let mut processed = 0;
        let worker_app = app.clone();
        let folders = snapshot.folders.clone();
        let mut results = ScanWorker::spawn(
            futures::stream::iter(work)
                .map(move |item| {
                    run_instance_work(worker_app.clone(), instance_id, folders.clone(), item)
                })
                .buffer_unordered(parallelism),
        );

        // Phase 3, once per batch: re-take the gate only to commit what is
        // still current.
        loop {
            let (batch, finished) = next_commit_batch(&mut results, || {
                processed += 1;
                if let Some(subtask) = &finalization_subtask {
                    subtask.update_items(processed, total);
                }
            })
            .await;
            let Some(committed) =
                commit_instance_results(app, instance_id, &snapshot, batch).await?
            else {
                if attempt < 3 {
                    info!(
                        "Instance {instance_id} was moved during its addon scan; scanning it again"
                    );
                    continue 'scan;
                }
                anyhow::bail!(
                    "Instance {instance_id} was moved or removed while its addons were being scanned"
                );
            };
            if committed.written != 0 {
                update_notifier.send(CacheEntityId::Instance(instance_id));
            }
            counts.add(committed);
            if finished {
                results.settled().await?;
                if let Some(subtask) = &finalization_subtask {
                    subtask.complete_items();
                }
                break 'scan;
            }
        }
    }

    let InstanceCommitCounts {
        written,
        discarded,
        failed,
    } = counts;
    info!(
        "Completed local mod caching for instance {instance_id}: {written} updated, {removed_total} removed, {discarded} changed during the scan, {failed} unreadable"
    );
    Ok(())
}

/// Commit one batch of an instance scan. Results whose new metadata was
/// garbage-collected before the commit get it created again with the gate
/// released and are committed once more. `None`: the instance moved since the
/// snapshot, so these results name paths that no longer exist.
async fn commit_instance_results(
    app: &App,
    instance_id: InstanceId,
    snapshot: &InstanceSnapshot,
    outcomes: Vec<InstanceScanOutcome>,
) -> anyhow::Result<Option<InstanceCommitCounts>> {
    let Some((mut counts, orphaned)) =
        commit_instance_batch(app, instance_id, snapshot, outcomes).await?
    else {
        return Ok(None);
    };
    if orphaned.is_empty() {
        return Ok(Some(counts));
    }
    let mcm = app.meta_cache_manager();
    let mut again = Vec::new();
    for outcome in orphaned {
        let InstanceScanOutcome::Parsed {
            row, addon, result, ..
        } = outcome
        else {
            continue;
        };
        match mcm.ensure_mod_metadata(&result, &addon.filename).await {
            Ok(metadata_id) => again.push(InstanceScanOutcome::Parsed {
                row,
                addon,
                result,
                metadata_id,
            }),
            Err(error) => {
                counts.failed += 1;
                warn!(
                    ?error,
                    "Instance {instance_id}: could not store a scan result"
                );
            }
        }
    }
    let Some((retried, orphaned)) =
        commit_instance_batch(app, instance_id, snapshot, again).await?
    else {
        return Ok(None);
    };
    counts.add(retried);
    if !orphaned.is_empty() {
        counts.failed += orphaned.len();
        warn!(
            "Instance {instance_id}: {} scan result(s) lost their metadata twice; the next scan reads them again",
            orphaned.len()
        );
    }
    Ok(Some(counts))
}

/// Commit one batch of an instance scan under the instance gate. Returns its
/// counts and the results whose new metadata was garbage-collected before
/// the commit: the gate is never held while metadata is created, because an
/// install holding it may be waiting for the icon permit or hash lock that
/// creating metadata takes. `None`: the instance moved since the snapshot.
async fn commit_instance_batch(
    app: &App,
    instance_id: InstanceId,
    snapshot: &InstanceSnapshot,
    outcomes: Vec<InstanceScanOutcome>,
) -> anyhow::Result<Option<(InstanceCommitCounts, Vec<InstanceScanOutcome>)>> {
    #[cfg(test)]
    app.meta_cache_manager()
        .scan_test_pause(Path::new(&format!("instance-{}", *instance_id)), "commit")
        .await;
    let mut counts = InstanceCommitCounts::default();
    let mut orphaned = Vec::new();
    if outcomes.is_empty() {
        return Ok(Some((counts, orphaned)));
    }
    let instance_manager = app.instance_manager();
    let _operation = instance_manager.lock_instance(instance_id).await;
    let current_shortpath = instance_manager
        .instances
        .read()
        .await
        .get(&instance_id)
        .map(|instance| instance.shortpath.clone());
    match current_shortpath {
        Some(current) if current == snapshot.shortpath => {}
        Some(_) => return Ok(None),
        None => anyhow::bail!(
            "Instance {instance_id} was moved or removed while its addons were being scanned"
        ),
    }
    let mut stamps = Vec::new();
    let mut others = Vec::new();
    for outcome in outcomes {
        match outcome {
            InstanceScanOutcome::Failed => counts.failed += 1,
            InstanceScanOutcome::Stamp(row, addon, observed) => stamps.push((row, addon, observed)),
            parsed @ InstanceScanOutcome::Parsed { .. } => others.push(parsed),
        }
    }
    if !stamps.is_empty() {
        let total = stamps.len();
        match commit_instance_stamps(app, &snapshot.folders, stamps).await {
            Ok(stamped) => {
                for committed in stamped {
                    match committed {
                        CommitOutcome::Written => counts.written += 1,
                        CommitOutcome::Unchanged => {}
                        CommitOutcome::Discarded | CommitOutcome::Orphaned => counts.discarded += 1,
                    }
                }
            }
            Err(error) => {
                counts.failed += total;
                warn!(
                    ?error,
                    "Instance {instance_id}: could not store {total} scan result(s)"
                );
            }
        }
    }
    for outcome in others {
        let InstanceScanOutcome::Parsed {
            row,
            addon,
            result,
            metadata_id,
        } = &outcome
        else {
            continue;
        };
        let committed = commit_instance_parsed(
            app,
            instance_id,
            &snapshot.folders,
            row,
            addon,
            result,
            metadata_id,
        )
        .await;
        match committed {
            Ok(CommitOutcome::Written) => counts.written += 1,
            Ok(CommitOutcome::Unchanged) => {}
            Ok(CommitOutcome::Discarded) => counts.discarded += 1,
            Ok(CommitOutcome::Orphaned) => orphaned.push(outcome),
            Err(error) => {
                counts.failed += 1;
                warn!(
                    ?error,
                    "Instance {instance_id}: could not store a scan result"
                );
            }
        }
    }
    Ok(Some((counts, orphaned)))
}

/// Remove the rows of addons whose files are gone. The caller holds the
/// instance gate. Each name is confirmed absent in both spellings by an exact
/// listing, and a row is removed only while it is still exactly what the
/// snapshot saw. Returns how many were removed.
async fn remove_stale_instance_rows(
    app: &App,
    instance_id: InstanceId,
    folders: &HashMap<&'static str, PathBuf>,
    stale: Vec<mfcdb::ModFileCacheRow>,
) -> usize {
    use crate::domain::instance::AddonType;
    let mut listings = FolderListings::default();
    let mut removed = 0;
    for row in stale {
        // A name the scan never lists is stale even if such a file exists.
        if let Some(kind) = AddonType::from_db_string(&row.addon_type)
            .filter(|kind| listable_addon_name(*kind, &row.filename))
        {
            if !listings
                .confirmed_absent(&folders[kind.to_db_string()], &row.filename)
                .await
            {
                continue;
            }
        }
        let expected = row.clone();
        let deleted = app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                let current = mfcdb::get_mod_file_cache_by_id_conn(&tx, &expected.id)?;
                if current.as_ref() != Some(&expected) {
                    return Ok(false);
                }
                mfcdb::delete_mod_file_cache_by_id_conn(&tx, &expected.id)?;
                tx.commit()?;
                Ok(true)
            })
            .await;
        match deleted {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(error) => warn!(
                ?error,
                "Instance {instance_id}: could not remove a stale entry"
            ),
        }
    }
    removed
}

async fn run_instance_work(
    app: App,
    instance_id: InstanceId,
    folders: Arc<HashMap<&'static str, PathBuf>>,
    item: InstanceScanWork,
) -> InstanceScanOutcome {
    let mcm = app.meta_cache_manager();
    let (row, addon) = match item {
        InstanceScanWork::Verify(row, addon) => {
            let path = folders[addon.kind.to_db_string()].join(addon.disk_name());
            match mcm
                .verify_cached_bytes(row.file_stamp.as_deref(), &row.metadata_id, &path)
                .await
            {
                Ok(ByteCheck::Same(observed)) => {
                    return InstanceScanOutcome::Stamp(row, addon, observed);
                }
                Ok(ByteCheck::Different) => (Some(row), addon),
                Err(error) => {
                    warn!(
                        ?error,
                        "Instance {instance_id}: could not verify {}",
                        path.display()
                    );
                    return InstanceScanOutcome::Failed;
                }
            }
        }
        InstanceScanWork::Legacy(row, addon, read) => {
            let path = folders[addon.kind.to_db_string()].join(addon.disk_name());
            let stamp = addon.stamp.clone().unwrap_or_default();
            match mcm
                .settle_legacy_row(
                    &row.metadata_id,
                    row.last_updated_at,
                    &stamp,
                    addon.times,
                    &path,
                    read,
                )
                .await
            {
                Ok(Some(observed)) => return InstanceScanOutcome::Stamp(row, addon, observed),
                Ok(None) => (Some(row), addon),
                Err(error) => {
                    warn!(
                        ?error,
                        "Instance {instance_id}: could not check {}",
                        path.display()
                    );
                    return InstanceScanOutcome::Failed;
                }
            }
        }
        InstanceScanWork::Parse(row, addon) => (row, addon),
    };
    let folder = folders[addon.kind.to_db_string()].clone();
    let parsed = async {
        let result = if addon.kind == crate::domain::instance::AddonType::Worlds {
            mcm.world_dir_parse_result(&folder, &addon.filename).await?
        } else {
            mcm.hash_and_parse_mod_file(&folder, &addon.filename, addon.enabled)
                .await?
        };
        let metadata_id = mcm.ensure_mod_metadata(&result, &addon.filename).await?;
        Ok::<_, anyhow::Error>((result, metadata_id))
    }
    .await;
    match parsed {
        Ok((result, metadata_id)) => InstanceScanOutcome::Parsed {
            row,
            addon,
            result,
            metadata_id,
        },
        Err(error) => {
            warn!(
                ?error,
                "Instance {instance_id}: could not scan {}",
                folder.join(addon.disk_name()).display()
            );
            InstanceScanOutcome::Failed
        }
    }
}

/// Record verified or settled legacy stamps, one per result, in a single
/// transaction for the batch: the first scan after an upgrade from 2.0.40, or
/// after the data folder moved, stamps nearly every row. A result is
/// discarded when its file or row changed after the snapshot. The outcomes
/// are in the order of `stamps`.
async fn commit_instance_stamps(
    app: &App,
    folders: &HashMap<&'static str, PathBuf>,
    stamps: Vec<(mfcdb::ModFileCacheRow, ListedAddon, String)>,
) -> anyhow::Result<Vec<CommitOutcome>> {
    let mut outcomes = vec![CommitOutcome::Discarded; stamps.len()];
    let mut current = Vec::new();
    for (index, (row, addon, observed)) in stamps.into_iter().enumerate() {
        let path = folders[addon.kind.to_db_string()].join(addon.disk_name());
        if current_file_stamp(&path).await.as_deref() == Some(observed.as_str()) {
            current.push((index, row, observed));
        }
    }
    if current.is_empty() {
        return Ok(outcomes);
    }
    let stored = app
        .db
        .write(move |mut conn| {
            let tx = conn.transaction()?;
            let now = DbDateTime(Utc::now().fixed_offset());
            let mut stored = Vec::with_capacity(current.len());
            for (index, row, observed) in current {
                let still = mfcdb::get_mod_file_cache_by_id_conn(&tx, &row.id)?;
                if still.as_ref() != Some(&row) {
                    continue;
                }
                let Some(metadata) = metarepo::get_metadata_by_id_conn(&tx, &row.metadata_id)?
                else {
                    continue;
                };
                let stamp = qualified_file_stamp(&metadata, &observed);
                if row.file_stamp.as_deref() == Some(stamp.as_str()) {
                    stored.push((index, CommitOutcome::Unchanged));
                    continue;
                }
                mfcdb::update_mod_file_stamp_conn(
                    &tx,
                    row.instance_id,
                    &row.addon_type,
                    &row.filename,
                    &row.metadata_id,
                    Some(&stamp),
                    now,
                )?;
                stored.push((index, CommitOutcome::Written));
            }
            tx.commit()?;
            Ok(stored)
        })
        .await?;
    for (index, outcome) in stored {
        outcomes[index] = outcome;
    }
    Ok(outcomes)
}

/// Publish a freshly parsed file. Discarded when the file or row changed
/// after the snapshot (toggle, delete, install, external edit), rather than
/// overwrite newer state. Never creates metadata: the caller holds the
/// instance gate (see [`commit_instance_batch`]).
async fn commit_instance_parsed(
    app: &App,
    instance_id: InstanceId,
    folders: &HashMap<&'static str, PathBuf>,
    row: &Option<mfcdb::ModFileCacheRow>,
    addon: &ListedAddon,
    result: &ModFileParseResult,
    metadata_id: &str,
) -> anyhow::Result<CommitOutcome> {
    let path = folders[addon.kind.to_db_string()].join(addon.disk_name());
    if addon.kind == crate::domain::instance::AddonType::Worlds {
        let is_dir = tokio::fs::metadata(&path)
            .await
            .is_ok_and(|metadata| metadata.is_dir());
        if !is_dir
            || !matches!(
                crate::managers::instance::installer::publication::world_install_incomplete(&path)
                    .await,
                Ok(false)
            )
        {
            return Ok(CommitOutcome::Discarded);
        }
    } else {
        let current = current_file_stamp(&path).await;
        if current.is_none() || current != result.file_stamp {
            return Ok(CommitOutcome::Discarded);
        }
    }
    let published = publish_instance_row(
        &app.db,
        instance_id,
        &addon.filename,
        addon.kind.to_db_string(),
        addon.enabled,
        metadata_id,
        result,
        Some(row.clone()),
    )
    .await;
    match published {
        Ok(true) => Ok(CommitOutcome::Written),
        Ok(false) => Ok(CommitOutcome::Discarded),
        Err(error) if error.is_foreign_key_violation() => Ok(CommitOutcome::Orphaned),
        Err(error) => Err(error.into()),
    }
}

fn cache_local(app: App, rx: LockNotify<CacheTargets>, update_notifier: UpdateNotifier) {
    tokio::spawn(async move {
        let app = &app;
        let update_notifier = &update_notifier;

        LoopWatcher::new(rx).await.loop_interrupt(
            |CacheTargetInfo {
                entity_id,
                is_override,
                is_priority,
                verify,
                generation,
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
                                cache_instance(app, update_notifier, instance_id, verify).await
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
                        biased;
                        _ = wait_for_pause, if !is_override => {
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

                move |targets: &mut CacheTargets| targets.release_pass(entity_id, generation, r)
            }
        ).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) async fn addon_fixture(
        app: &App,
        name: &str,
        kind: crate::domain::instance::AddonType,
        filename: &str,
        bytes: &[u8],
    ) -> (InstanceId, PathBuf) {
        addon_fixture_version(
            app,
            name,
            kind,
            filename,
            bytes,
            crate::domain::instance::info::StandardVersion {
                release: "1.20.1".into(),
                modloaders: Default::default(),
            },
        )
        .await
    }

    pub(super) async fn addon_fixture_version(
        app: &App,
        name: &str,
        kind: crate::domain::instance::AddonType,
        filename: &str,
        bytes: &[u8],
        version: crate::domain::instance::info::StandardVersion,
    ) -> (InstanceId, PathBuf) {
        use crate::domain::instance::info::GameVersion;
        use crate::managers::instance::InstanceVersionSource;
        let group = app.instance_manager().get_default_group().await.unwrap();
        let id = app
            .instance_manager()
            .create_instance(
                group,
                name.into(),
                false,
                InstanceVersionSource::Version(GameVersion::Standard(version)),
                String::new(),
            )
            .await
            .unwrap();
        let shortpath = app.instance_manager().instances.read().await[&id]
            .shortpath
            .clone();
        let root = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        let folder = kind.get_folder_path(&root);
        tokio::fs::create_dir_all(&folder).await.unwrap();
        let path = folder.join(filename);
        tokio::fs::write(&path, bytes).await.unwrap();
        (id, path)
    }

    async fn task11r1_observation_failure(stage: &'static str, ordinary: bool) {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = addon_fixture(
            &app,
            "task11r1-observation",
            crate::domain::instance::AddonType::Mods,
            "one.jar",
            b"one",
        )
        .await;
        tokio::fs::write(path.with_file_name("two.jar"), b"two")
            .await
            .unwrap();
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let mut before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        before.sort_by(|a, b| a.id.cmp(&b.id));
        for owner in &before {
            metarepo::upsert_mr_update_context(
                &app.db,
                &owner.metadata_id,
                "task11r1-reference",
                "healthy",
                DbDateTime(Utc::now().fixed_offset()),
            )
            .await
            .unwrap();
        }
        if ordinary {
            let packs = path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("resourcepacks");
            tokio::fs::create_dir_all(&packs).await.unwrap();
            tokio::fs::write(packs.join("observed.zip"), b"new observed region")
                .await
                .unwrap();
        }
        let fault_path = match stage {
            "entry-stat" => path.clone(),
            "cache-read" => PathBuf::from(format!("instance-{}", *id)),
            _ => path.parent().unwrap().to_owned(),
        };
        {
            let manager = app.meta_cache_manager();
            let mut state = manager.scan_test_state.lock().unwrap();
            state.observations.clear();
            state
                .fail_after
                .insert((fault_path, stage), if stage == "iterate" { 2 } else { 1 });
        }
        let result = if ordinary {
            let (tx, rx) = oneshot::channel();
            app.meta_cache_manager()
                .local_targets
                .send_modify_always(|q| {
                    q.set_priority(CacheTarget {
                        entity_id: CacheEntityId::Instance(id),
                        callback: Some(Box::new(move |r| {
                            let _ = tx.send(r);
                        })),
                    })
                })
                .await;
            tokio::time::timeout(std::time::Duration::from_secs(5), rx)
                .await
                .unwrap()
                .unwrap()
        } else {
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                app.meta_cache_manager().override_caching_and_wait(
                    CacheEntityId::Instance(id),
                    false,
                    false,
                ),
            )
            .await
            .unwrap()
        };
        let mut after = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        if ordinary {
            assert!(
                after
                    .iter()
                    .any(|r| r.addon_type == "resourcepacks" && r.filename == "observed.zip")
            );
            after.retain(|r| r.addon_type == "mods");
        }
        after.sort_by(|a, b| a.id.cmp(&b.id));
        for owner in &before {
            assert!(
                metarepo::get_metadata_by_id(&app.db, &owner.metadata_id)
                    .await
                    .unwrap()
                    .is_some()
            );
            assert_eq!(
                metarepo::get_mr_update_context(&app.db, &owner.metadata_id, "task11r1-reference")
                    .await
                    .unwrap()
                    .unwrap()
                    .update_paths,
                "healthy"
            );
        }
        assert_eq!(
            after
                .iter()
                .map(|r| (&r.id, &r.metadata_id))
                .collect::<Vec<_>>(),
            before
                .iter()
                .map(|r| (&r.id, &r.metadata_id))
                .collect::<Vec<_>>(),
            "incomplete {stage} observation cannot retire healthy owners; callback={result:?}"
        );
        if !ordinary {
            if stage == "cache-read" {
                // Without the rows there is nothing to reconcile against.
                let error = format!("{:#}", result.unwrap_err());
                assert!(
                    error.contains("injected") && error.contains("cached addon owners"),
                    "actionable {stage} error: {error}"
                );
            } else {
                // A folder that cannot be listed keeps its rows but never fails
                // an install or export that does not depend on it.
                result.unwrap_or_else(|error| panic!("{stage} must not fail the pass: {error:#}"));
            }
        }
        assert_eq!(std::fs::read(path).unwrap(), b"one");
    }
    #[tokio::test]
    async fn task11r1_folder_stat_failure_preserves_owners() {
        task11r1_observation_failure("folder-stat", false).await;
    }
    #[tokio::test]
    async fn task11r1_read_dir_failure_preserves_owners() {
        task11r1_observation_failure("read-dir", false).await;
    }
    #[tokio::test]
    async fn task11r1_partial_iteration_preserves_owners() {
        task11r1_observation_failure("iterate", false).await;
    }
    #[tokio::test]
    async fn task11r1_entry_stat_failure_preserves_owners() {
        task11r1_observation_failure("entry-stat", false).await;
    }
    #[tokio::test]
    async fn task11r1_cache_read_failure_is_not_success() {
        task11r1_observation_failure("cache-read", false).await;
    }
    #[tokio::test]
    async fn task11r1_ordinary_incomplete_scan_preserves_owners() {
        task11r1_observation_failure("iterate", true).await;
    }
    #[tokio::test]
    async fn task11r1_missing_target_is_not_verified_success() {
        let app = crate::setup_managers_for_test().await;
        assert!(
            app.meta_cache_manager()
                .override_caching_and_wait(
                    CacheEntityId::Instance(InstanceId(i32::MAX)),
                    false,
                    false
                )
                .await
                .is_err()
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn task11r2_stable_symlink_equal_size_and_changed_target() {
        let app = crate::setup_managers_for_test().await;
        let body = r#"{"schemaVersion":1,"id":"example","version":"1"}"#;
        let bytes = metadata_jar("fabric.mod.json", body);
        let changed = metadata_jar("fabric.mod.json", &body.replace("\"1\"", "\"2\""));
        assert_eq!(bytes.len(), changed.len());
        assert!(bytes.len() < 255);
        let (id, path) = addon_fixture(
            &app,
            "task11r2-symlink",
            crate::domain::instance::AddonType::Mods,
            "example.jar",
            &bytes,
        )
        .await;
        // The link and its extensionless target are both owned by this fixture.
        // Equal link-text/content lengths admit the reviewed reuse branch.
        let target_name = "t".repeat(bytes.len());
        let target = path.parent().unwrap().join(&target_name);
        tokio::fs::rename(&path, &target).await.unwrap();
        std::os::unix::fs::symlink(&target_name, &path).unwrap();
        assert_eq!(
            std::fs::symlink_metadata(&path).unwrap().len(),
            std::fs::metadata(&path).unwrap().len()
        );
        assert_eq!(std::fs::canonicalize(&path).unwrap(), target);
        let scan = || {
            app.meta_cache_manager().override_caching_and_wait(
                CacheEntityId::Instance(id),
                false,
                false,
            )
        };
        scan().await.unwrap();
        let before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        scan()
            .await
            .expect("unchanged admitted symlink must not compare link identity to target identity");
        let unchanged = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(before.id, unchanged.id);
        assert_eq!(before.metadata_id, unchanged.metadata_id);
        let modified = std::fs::metadata(&target).unwrap().modified().unwrap();
        tokio::fs::write(&target, &changed).await.unwrap();
        std::fs::File::options()
            .write(true)
            .open(&target)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        scan().await.unwrap();
        let after = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(after.id, before.id);
        assert_ne!(after.metadata_id, before.metadata_id);
        assert_eq!(
            metarepo::get_metadata_by_id(&app.db, &after.metadata_id)
                .await
                .unwrap()
                .unwrap()
                .version
                .as_deref(),
            Some("2")
        );
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(&path).unwrap(), changed);
    }

    #[tokio::test]
    async fn task11r1_forced_unchanged_verifies_without_parse_or_rewrite() {
        let app = crate::setup_managers_for_test().await;
        let bytes = metadata_jar(
            "fabric.mod.json",
            r#"{"schemaVersion":1,"id":"example","version":"1.0.0"}"#,
        );
        let (id, path) = addon_fixture(
            &app,
            "task11r1-reuse",
            crate::domain::instance::AddonType::Mods,
            "mod.jar",
            &bytes,
        )
        .await;
        // The export pass: byte verification of the mods folder.
        let scan = || {
            app.meta_cache_manager().override_caching_verified_and_wait(
                CacheEntityId::Instance(id),
                VerifyScope::MODS,
                false,
                false,
            )
        };
        scan().await.unwrap();
        let before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        app.meta_cache_manager()
            .scan_test_state
            .lock()
            .unwrap()
            .observations
            .clear();
        scan().await.unwrap();
        let after = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        let manager = app.meta_cache_manager();
        let state = manager.scan_test_state.lock().unwrap();
        assert_eq!(
            state
                .observations
                .get(&(path.clone(), "hash"))
                .copied()
                .unwrap_or(0),
            1,
            "verification reads the actual bytes exactly once"
        );
        assert!(
            state
                .observations
                .get(&(path.clone(), "hash-recheck"))
                .copied()
                .unwrap_or(0)
                > 0,
            "reuse retains final path/content verification"
        );
        assert_eq!(
            state
                .observations
                .get(&(path, "parse"))
                .copied()
                .unwrap_or(0),
            0,
            "qualified equal bytes must skip ZIP/logo/fingerprint parsing"
        );
        assert_eq!(
            before, after,
            "unchanged qualified owner must not be rewritten"
        );
    }
    #[tokio::test]
    async fn task11r1_proven_deletion_and_absent_optional_folders_succeed() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = addon_fixture(
            &app,
            "task11r1-absence",
            crate::domain::instance::AddonType::Mods,
            "one.jar",
            b"one",
        )
        .await;
        tokio::fs::write(path.with_file_name("two.jar"), b"two")
            .await
            .unwrap();
        let scan = || {
            app.meta_cache_manager().override_caching_and_wait(
                CacheEntityId::Instance(id),
                false,
                false,
            )
        };
        scan().await.unwrap();
        let before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .into_iter()
            .find(|r| r.filename == "two.jar")
            .unwrap();
        tokio::fs::remove_file(&path).await.unwrap();
        scan().await.unwrap();
        assert_eq!(
            mfcdb::get_mod_files_by_instance(&app.db, *id)
                .await
                .unwrap(),
            vec![before]
        );
        tokio::fs::remove_file(path.with_file_name("two.jar"))
            .await
            .unwrap();
        tokio::fs::remove_dir(path.parent().unwrap()).await.unwrap();
        scan().await.unwrap();
        assert!(
            mfcdb::get_mod_files_by_instance(&app.db, *id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn task11r1_marker_change_and_legacy_qualification_controls() {
        let app = crate::setup_managers_for_test().await;
        let (id, path) = addon_fixture(
            &app,
            "task11r1-qualification",
            crate::domain::instance::AddonType::Mods,
            "mod.jar",
            b"qualified bytes",
        )
        .await;
        let scan = || {
            app.meta_cache_manager().override_caching_and_wait(
                CacheEntityId::Instance(id),
                false,
                false,
            )
        };
        scan().await.unwrap();
        let before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        // Back-dated, so the legacy control below sees a file that last
        // changed before its row was written.
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap()
            - std::time::Duration::from_secs(2);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let mcm = app.meta_cache_manager();
        mcm.clear_scan_observations();
        scan().await.unwrap();
        let after = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(before.id, after.id);
        assert_eq!(before.metadata_id, after.metadata_id);
        assert_ne!(before.file_stamp, after.file_stamp);
        // A touched qualified file is settled by one read, not a re-parse.
        assert_eq!(mcm.scan_observations(&path, "hash"), 1);
        assert_eq!(mcm.scan_observations(&path, "parse"), 0);

        // A 2.0.40 row (no stamp, written after the file last changed) is
        // stamped from `stat` alone.
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            "mods",
            "mod.jar",
            &before.metadata_id,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        mcm.clear_scan_observations();
        scan().await.unwrap();
        let backfilled = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(mcm.scan_observations(&path, "hash"), 0);
        assert_eq!(mcm.scan_observations(&path, "parse"), 0);
        assert_eq!(backfilled.metadata_id, before.metadata_id);
        assert!(
            backfilled
                .file_stamp
                .as_deref()
                .is_some_and(|stamp| stamp.starts_with("parsed-v1:"))
        );

        // An unknown marker never qualifies: the file is parsed again.
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            "mods",
            "mod.jar",
            &before.metadata_id,
            Some("unqualified-old-marker"),
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        mcm.clear_scan_observations();
        scan().await.unwrap();
        assert_eq!(mcm.scan_observations(&path, "parse"), 1);
    }

    #[tokio::test]
    async fn task11r1_image_loop_retains_same_entity_notification() {
        let targets = LockNotify::new(None::<CacheEntityId>);
        let worker_targets = targets.clone();
        let (entered, mut events) = mpsc::unbounded_channel();
        let release = Arc::new(tokio::sync::Notify::new());
        let worker_release = release.clone();
        let worker = tokio::spawn(async move {
            let mut watcher = LoopWatcher::new(worker_targets).await;
            watcher
                .loop_interrupt(|id| {
                    let entered = entered.clone();
                    let release = worker_release.clone();
                    async move {
                        entered.send(id).unwrap();
                        release.notified().await;
                        |_: &mut Option<CacheEntityId>| false
                    }
                })
                .await;
        });
        let a = CacheEntityId::Instance(InstanceId(1));
        targets.send(Some(a)).await;
        assert_eq!(events.recv().await.unwrap(), a);
        targets.send(Some(a)).await;
        targets.send(Some(a)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        release.notify_one();
        let next = tokio::time::timeout(std::time::Duration::from_millis(200), events.recv()).await;
        assert_eq!(
            next.unwrap(),
            Some(a),
            "a pending same-entity image request must run after the current pass"
        );
        release.notify_one();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), events.recv())
                .await
                .is_err(),
            "pending identical notifications coalesce without replaying completed work"
        );
        let b = CacheEntityId::Instance(InstanceId(2));
        targets.send(Some(b)).await;
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_millis(200), events.recv())
                .await
                .unwrap(),
            Some(b)
        );
        worker.abort();
    }

    #[tokio::test]
    async fn task11_expired_thumbnail_retry_releases_lock() {
        let failures = RwLock::new(HashMap::new());
        assert!(thumbnail_retry_allowed(&failures, &"project").await);
        failures.write().await.insert(
            "project",
            (
                std::time::Instant::now() + std::time::Duration::from_secs(60),
                3,
            ),
        );
        assert!(!thumbnail_retry_allowed(&failures, &"project").await);
        failures.write().await.insert(
            "project",
            (
                std::time::Instant::now() - std::time::Duration::from_secs(1),
                3,
            ),
        );
        let retry = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            thumbnail_retry_allowed(&failures, &"project"),
        )
        .await;
        assert!(
            retry.is_ok(),
            "expired retry must not upgrade its own read lock"
        );
        assert!(retry.unwrap());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), failures.write())
                .await
                .is_ok()
        );
    }

    #[test]
    fn task11_late_request_waits_for_a_pass_that_observed_it() {
        let mut targets = CacheTargets::new();
        let entity = CacheEntityId::Instance(InstanceId(1));
        targets.set_override(CacheTarget {
            entity_id: entity,
            callback: None,
        });
        let first_pass = targets.target().unwrap();
        let (tx, mut rx) = oneshot::channel();
        targets.set_override(CacheTarget {
            entity_id: entity,
            callback: Some(Box::new(move |r| {
                let _ = tx.send(r);
            })),
        });
        targets.release_pass(entity, first_pass.generation, Ok(()));
        assert!(
            matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Empty)),
            "late request cannot complete from an earlier snapshot"
        );
        let next_pass = targets.target().unwrap();
        targets.release_pass(entity, next_pass.generation, Ok(()));
        assert!(rx.try_recv().unwrap().is_ok());
    }

    #[test]
    fn task11_foreground_entities_keep_independent_callbacks() {
        let mut targets = CacheTargets::new();
        let a = CacheEntityId::Instance(InstanceId(101));
        let b = CacheEntityId::Instance(InstanceId(102));
        let mut receivers = Vec::new();
        for entity_id in [a, b, a] {
            let (tx, rx) = oneshot::channel();
            targets.set_override(CacheTarget {
                entity_id,
                callback: Some(Box::new(move |r| {
                    let _ = tx.send(r);
                })),
            });
            receivers.push(rx);
        }
        for rx in &mut receivers {
            assert!(
                matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Empty)),
                "requesting another entity must not cancel an accepted waiter"
            );
        }
        assert_eq!(targets.target().unwrap().entity_id, a);
        targets.release_target(a, Ok(()));
        assert!(receivers[0].try_recv().unwrap().is_ok());
        assert!(receivers[2].try_recv().unwrap().is_ok());
        assert!(matches!(
            receivers[1].try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        assert_eq!(targets.target().unwrap().entity_id, b);
        targets.release_target(b, Ok(()));
        assert!(receivers[1].try_recv().unwrap().is_ok());
    }

    struct RemoteProbe {
        stages: mpsc::UnboundedSender<(CacheEntityId, &'static str)>,
        query: tokio::sync::Notify,
        save: tokio::sync::Notify,
    }
    static REMOTE_PROBE: std::sync::OnceLock<RemoteProbe> = std::sync::OnceLock::new();
    struct ProbeCacher;
    #[async_trait::async_trait]
    impl ModplatformCacher for ProbeCacher {
        const NAME: &'static str = "task11-probe";
        type SaveBundle = ();
        async fn query_platform(
            _app: &App,
            id: CacheEntityId,
            sender: &mut BundleSender<Self::SaveBundle>,
        ) -> anyhow::Result<()> {
            let p = REMOTE_PROBE.get().unwrap();
            p.stages.send((id, "query")).unwrap();
            p.query.notified().await;
            sender.send(());
            Ok(())
        }
        async fn save_batch(_app: &App, id: CacheEntityId, _bundle: ()) {
            let p = REMOTE_PROBE.get().unwrap();
            p.stages.send((id, "save")).unwrap();
            p.save.notified().await;
            p.stages.send((id, "saved")).unwrap();
        }
        async fn cache_icons(_app: &App, _id: CacheEntityId, _notifier: &UpdateNotifier) {}
    }

    #[tokio::test]
    async fn task11_remote_loop_waits_for_query_and_save_while_globally_paused() {
        let app = crate::setup_managers_for_test().await;
        let (stages, mut events) = mpsc::unbounded_channel();
        assert!(
            REMOTE_PROBE
                .set(RemoteProbe {
                    stages,
                    query: tokio::sync::Notify::new(),
                    save: tokio::sync::Notify::new()
                })
                .is_ok()
        );
        let mut paused = app.meta_cache_manager().pause_caching.subscribe();
        app.instance_manager()
            .any_instance_running
            .send_replace(true);
        while !*paused.borrow_and_update() {
            paused.changed().await.unwrap();
        }
        let targets = LockNotify::new(CacheTargets::new());
        let (sender, _) = watch::channel(());
        let notifier = UpdateNotifier {
            target: Arc::new(AtomicI32::new(0)),
            sender: Arc::new(sender),
        };
        cache_modplatform::<ProbeCacher>(app.clone(), targets.clone(), notifier);
        let a = CacheEntityId::Instance(InstanceId(111));
        let b = CacheEntityId::Server(111);
        targets.send_modify_always(|q| q.waiting.push_back(a)).await;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), events.recv())
                .await
                .is_err(),
            "ordinary remote work remains paused"
        );
        let mut receivers = Vec::new();
        for entity_id in [a, b] {
            let (tx, rx) = oneshot::channel();
            receivers.push(rx);
            targets
                .send_modify_always(|q| {
                    q.set_override(CacheTarget {
                        entity_id,
                        callback: Some(Box::new(move |r| {
                            let _ = tx.send(r);
                        })),
                    })
                })
                .await;
        }
        let probe = REMOTE_PROBE.get().unwrap();
        for (i, entity) in [a, b].into_iter().enumerate() {
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(3), events.recv())
                    .await
                    .unwrap()
                    .unwrap(),
                (entity, "query")
            );
            assert!(matches!(
                receivers[i].try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ));
            probe.query.notify_one();
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(3), events.recv())
                    .await
                    .unwrap()
                    .unwrap(),
                (entity, "save")
            );
            assert!(
                matches!(
                    receivers[i].try_recv(),
                    Err(oneshot::error::TryRecvError::Empty)
                ),
                "query completion must still await persistence"
            );
            probe.save.notify_one();
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(3), events.recv())
                    .await
                    .unwrap()
                    .unwrap(),
                (entity, "saved")
            );
            tokio::time::timeout(std::time::Duration::from_secs(3), &mut receivers[i])
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
    }

    #[test]
    fn task11_callback_errors_and_revocation_stay_with_their_entity() {
        let mut q = CacheTargets::new();
        let mut receivers = Vec::new();
        for entity_id in [
            CacheEntityId::Instance(InstanceId(1)),
            CacheEntityId::Server(1),
        ] {
            let (tx, rx) = oneshot::channel();
            receivers.push(rx);
            q.set_override(CacheTarget {
                entity_id,
                callback: Some(Box::new(move |r| {
                    let _ = tx.send(r);
                })),
            });
        }
        q.revoke_target(CacheEntityId::Instance(InstanceId(1)));
        assert!(receivers[0].try_recv().unwrap().is_err());
        assert!(matches!(
            receivers[1].try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        q.release_target(
            CacheEntityId::Server(1),
            Err(anyhow!("fixture query failure")),
        );
        assert!(receivers[1].try_recv().unwrap().is_err());
    }

    #[tokio::test]
    async fn task11_explicit_finalization_ignores_unrelated_global_pause() {
        let app = crate::setup_managers_for_test().await;
        // Set the actual source of the global pause without starting any process.
        let mut paused = app.meta_cache_manager().pause_caching.subscribe();
        app.instance_manager()
            .any_instance_running
            .send_replace(true);
        while !*paused.borrow_and_update() {
            paused.changed().await.unwrap();
        }
        let (id, path) = addon_fixture(
            &app,
            "task11-idle",
            crate::domain::instance::AddonType::Mods,
            "mod.jar",
            b"idle target",
        )
        .await;
        // Hold the real instance gate so caching must yield to the pause branch.
        let manager = app.instance_manager();
        let gate = manager.lock_instance(id).await;
        let request = app.meta_cache_manager().override_caching_and_wait(
            CacheEntityId::Instance(id),
            true,
            true,
        );
        tokio::pin!(request);
        tokio::select! {
            _ = &mut request => panic!("cache must wait for ownership"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(40)) => {}
        }
        drop(gate);
        // Empty provider work after a real local scan: unknown hashes are intentionally ignored here.
        let mut sha = Sha512::new();
        sha.update(b"idle target");
        let now = app.meta_cache_manager().now();
        app.meta_cache_manager()
            .ignored_remote_mr_hashes
            .write()
            .await
            .ignore([hex::encode(sha.finalize())], now);
        let parsed = app
            .meta_cache_manager()
            .hash_and_parse_mod_file(&path.parent().unwrap().to_path_buf(), "mod.jar", true)
            .await
            .unwrap();
        app.meta_cache_manager()
            .ignored_remote_cf_hashes
            .write()
            .await
            .ignore([parsed.murmur2], now);
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), request).await;
        assert!(
            result.is_ok(),
            "explicit local and both remote stages must settle while globally paused"
        );
        result.unwrap().unwrap();
    }

    #[tokio::test]
    async fn task11_instance_scan_retains_equal_names_in_distinct_kinds() {
        use crate::domain::instance::AddonType;
        let app = crate::setup_managers_for_test().await;
        let (id, path) = addon_fixture(
            &app,
            "task11-folders",
            AddonType::ResourcePacks,
            "shared.zip",
            b"resource",
        )
        .await;
        let datapacks = path.parent().unwrap().parent().unwrap().join("datapacks");
        tokio::fs::create_dir_all(&datapacks).await.unwrap();
        tokio::fs::write(datapacks.join("shared.zip"), b"datapack")
            .await
            .unwrap();
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let rows = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        assert_eq!(
            rows.len(),
            2,
            "resource pack and staged datapack must retain independent ownership"
        );
        assert_ne!(rows[0].id, rows[1].id);
        assert_ne!(rows[0].metadata_id, rows[1].metadata_id);
    }

    #[tokio::test]
    async fn task11_instance_scan_uppercase_and_disabled_extensions() {
        use crate::domain::instance::AddonType;
        let app = crate::setup_managers_for_test().await;
        let (id, path) = addon_fixture(
            &app,
            "task11-upper",
            AddonType::ResourcePacks,
            "Pack.ZIP",
            b"resource",
        )
        .await;
        tokio::fs::write(
            path.parent().unwrap().join("Other.ZIP.disabled"),
            b"disabled",
        )
        .await
        .unwrap();
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let rows = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.filename == "Pack.ZIP" && r.enabled));
        assert!(rows.iter().any(|r| r.filename == "Other.ZIP" && !r.enabled));
        let root = path.parent().unwrap().parent().unwrap();
        for (folder, filename) in [
            ("mods", "Mod.JAR"),
            ("shaderpacks", "Shader.ZIP"),
            ("datapacks", "Data.ZIP"),
        ] {
            tokio::fs::create_dir_all(root.join(folder)).await.unwrap();
            tokio::fs::write(root.join(folder).join(filename), b"enabled")
                .await
                .unwrap();
            tokio::fs::write(
                root.join(folder).join(format!(
                    "Disabled.{ext}.disabled",
                    ext = filename.split('.').last().unwrap()
                )),
                b"disabled",
            )
            .await
            .unwrap();
        }
        tokio::fs::create_dir_all(root.join("saves/World.disabled"))
            .await
            .unwrap();
        tokio::fs::write(root.join("saves/World.disabled/level.dat"), b"world")
            .await
            .unwrap();
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let rows = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 9);
        for kind in ["mods", "shaders", "datapacks"] {
            assert_eq!(rows.iter().filter(|r| r.addon_type == kind).count(), 2);
            assert_eq!(
                rows.iter()
                    .filter(|r| r.addon_type == kind && !r.enabled)
                    .count(),
                1
            );
        }
        assert!(
            rows.iter()
                .any(|r| r.addon_type == "worlds" && r.filename == "World.disabled" && r.enabled)
        );
    }

    pub(super) fn metadata_jar(name: &str, body: &str) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(body.as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }

    pub(super) async fn ordinary_scan(app: &App, id: InstanceId) {
        let (tx, rx) = oneshot::channel();
        app.meta_cache_manager()
            .local_targets
            .send_modify_always(|targets| {
                targets.set_priority(CacheTarget {
                    entity_id: CacheEntityId::Instance(id),
                    callback: Some(Box::new(move |r| {
                        let _ = tx.send(r);
                    })),
                });
            })
            .await;
        tokio::time::timeout(std::time::Duration::from_secs(3), rx)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn task11_forced_scan_rehashes_same_size_preserved_time_replacement() {
        use crate::domain::instance::AddonType;
        let app = crate::setup_managers_for_test().await;
        let body = r#"{"schemaVersion":1,"id":"example","version":"1.0.0","name":"Example"}"#;
        let first = metadata_jar("fabric.mod.json", body);
        let second = metadata_jar("fabric.mod.json", &body.replace("1.0.0", "2.0.0"));
        assert_eq!(first.len(), second.len());
        let (id, path) = addon_fixture(
            &app,
            "task11-same-size",
            AddonType::Mods,
            "test.jar",
            &first,
        )
        .await;
        // The export pass: the mods folder is verified byte for byte.
        let cache = || {
            app.meta_cache_manager().override_caching_verified_and_wait(
                CacheEntityId::Instance(id),
                VerifyScope::MODS,
                false,
                false,
            )
        };
        cache().await.unwrap();
        let before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        let old_metadata = metarepo::get_metadata_by_id(&app.db, &before.metadata_id)
            .await
            .unwrap()
            .unwrap();
        metarepo::upsert_mr_mod_cache(
            &app.db,
            hex::encode(&old_metadata.sha512),
            "project".into(),
            "version".into(),
            "title".into(),
            "1".into(),
            "slug".into(),
            "desc".into(),
            "".into(),
            2,
            "".into(),
            "test.jar".into(),
            "https://fixture.invalid/old.jar".into(),
            DbDateTime(Utc::now().fixed_offset()),
            before.metadata_id.clone(),
        )
        .await
        .unwrap();
        let linked = |rows: Vec<mfcdb::MrExportRow>| {
            rows.into_iter()
                .find(|row| row.filename == "test.jar")
                .and_then(|row| row.mr_file_url)
        };
        assert_eq!(
            linked(
                mfcdb::get_instance_mr_export_files(&app.db, *id)
                    .await
                    .unwrap()
            )
            .as_deref(),
            Some("https://fixture.invalid/old.jar"),
            "fixture: the original bytes are linked"
        );
        ordinary_scan(&app, id).await;
        let skipped = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert!(before.file_stamp.is_some());
        assert_eq!(
            skipped.last_updated_at, before.last_updated_at,
            "ordinary unchanged scan must not rewrite/parse the file"
        );
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            "mods",
            "test.jar",
            &before.metadata_id,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        ordinary_scan(&app, id).await;
        assert!(
            mfcdb::get_mod_files_by_instance(&app.db, *id)
                .await
                .unwrap()[0]
                .file_stamp
                .is_some(),
            "legacy NULL marker requires refresh"
        );
        let stamp = std::fs::metadata(&path).unwrap().modified().unwrap();
        tokio::fs::write(&path, &second).await.unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(stamp))
            .unwrap();
        // Simulate even a marker-preserving external replacement: export authority is bytes.
        let preserved_qualification = qualified_file_stamp(
            &old_metadata,
            &observed_file_stamp(&std::fs::metadata(&path).unwrap()),
        );
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            "mods",
            "test.jar",
            &before.metadata_id,
            Some(&preserved_qualification),
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        cache().await.unwrap();
        let after = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(before.id, after.id);
        assert_ne!(
            before.metadata_id, after.metadata_id,
            "equal length and time do not prove content identity"
        );
        let parsed = metarepo::get_metadata_by_id(&app.db, &after.metadata_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(parsed.version.as_deref(), Some("2.0.0"));
        assert_eq!(parsed.modloaders, "fabric");
        assert_eq!(
            linked(
                mfcdb::get_instance_mr_export_files(&app.db, *id)
                    .await
                    .unwrap()
            ),
            None,
            "the replaced bytes must not keep the old Modrinth link"
        );
        cache().await.unwrap();
        let unchanged = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(after.metadata_id, unchanged.metadata_id);
    }

    #[tokio::test]
    async fn task11_neoforge_legacy_metadata_heals_without_replacing_identity() {
        let app = crate::setup_managers_for_test().await;
        let bytes = metadata_jar(
            "META-INF/neoforge.mods.toml",
            "[[mods]]\nmodId = 'example'\nversion = '1.0.0'\ndisplayName = 'Example'\n",
        );
        let (id, path) = addon_fixture(
            &app,
            "task11-heal",
            crate::domain::instance::AddonType::Mods,
            "neo.jar",
            &bytes,
        )
        .await;
        let mcm = app.meta_cache_manager();
        let cache = |verify| {
            mcm.override_caching_verified_and_wait(
                CacheEntityId::Instance(id),
                verify,
                false,
                false,
            )
        };
        cache(VerifyScope::NONE).await.unwrap();
        let before = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        metarepo::upsert_mr_update_context(
            &app.db,
            &before.metadata_id,
            "retained",
            "sentinel",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        let loaders = || async {
            metarepo::get_metadata_by_id(&app.db, &before.metadata_id)
                .await
                .unwrap()
                .unwrap()
                .modloaders
        };

        // 2.0.40 shape: labelled Forge by the old parser, no stamp, written
        // after the file last changed. The jar's mod list is re-read (no
        // content hash) and the label healed.
        metarepo::update_local_modloaders(
            &app.db,
            &before.metadata_id,
            "forge",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            "mods",
            "neo.jar",
            &before.metadata_id,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        mcm.clear_scan_observations();
        ordinary_scan(&app, id).await;
        assert_eq!(loaders().await, "neoforge");
        assert_eq!(mcm.scan_observations(&path, "hash"), 0);
        assert_eq!(mcm.scan_observations(&path, "loader-check"), 1);
        let healed = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(healed.id, before.id);
        assert_eq!(healed.metadata_id, before.metadata_id);
        assert!(
            healed
                .file_stamp
                .as_deref()
                .is_some_and(|stamp| stamp.starts_with("parsed-v1:"))
        );

        // A stamped row whose label no longer matches its stamp is re-parsed
        // when its bytes are verified, which heals it the same way.
        metarepo::update_local_modloaders(
            &app.db,
            &before.metadata_id,
            "forge",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        cache(VerifyScope::MODS).await.unwrap();
        let after = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(after.id, before.id);
        assert_eq!(after.metadata_id, before.metadata_id);
        assert_eq!(loaders().await, "neoforge");
        assert_eq!(
            metarepo::get_mr_update_context(&app.db, &after.metadata_id, "retained")
                .await
                .unwrap()
                .unwrap()
                .update_paths,
            "sentinel"
        );
    }

    /// A Fabric 1.20.1 instance with one scanned mod, for update badge tests.
    async fn release_install_fixture(app: &App, name: &str) -> (InstanceId, String) {
        use crate::domain::instance::{
            AddonType,
            info::{ModLoader, ModLoaderType, StandardVersion},
        };
        let (id, _) = addon_fixture_version(
            app,
            name,
            AddonType::Mods,
            "mod.jar",
            name.as_bytes(),
            StandardVersion {
                release: "1.20.1".into(),
                modloaders: [ModLoader {
                    type_: ModLoaderType::Fabric,
                    version: "fixture".into(),
                }]
                .into_iter()
                .collect(),
            },
        )
        .await;
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let row = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .remove(0);
        (id, row.metadata_id)
    }

    async fn badged(app: &App, id: InstanceId) -> bool {
        app.instance_manager().list_mods(id, None).await.unwrap()[0].has_update
    }

    /// Rows cached with every newer channel (as the CurseForge cacher writes
    /// them) must not badge a Release for betas: the channel floor is applied
    /// when the list is read, so no cache refresh is needed.
    #[tokio::test]
    async fn curseforge_release_install_is_not_badged_for_a_newer_beta() {
        let app = crate::setup_managers_for_test().await;
        let (id, metadata_id) = release_install_fixture(&app, "cf-release-install").await;
        let seed = |paths: &'static str| {
            metarepo::upsert_cf_mod_cache(
                &app.db,
                0,
                4242,
                100,
                "Fixture".into(),
                "1.0.0".into(),
                "fixture".into(),
                "summary".into(),
                "author".into(),
                carbon_platforms::ModChannel::Stable as i32,
                paths.into(),
                DbDateTime(Utc::now().fixed_offset()),
                metadata_id.clone(),
            )
        };

        seed("1.20.1,fabric,beta").await.unwrap();
        assert!(
            !badged(&app, id).await,
            "a Release with only a newer beta is up to date"
        );

        seed("1.20.1,fabric,beta;1.20.1,fabric,stable")
            .await
            .unwrap();
        assert!(badged(&app, id).await, "a newer Release is an update");
    }

    /// Seeds both platforms' cache rows for the fixture's jar, each with its
    /// own channel label and a newer beta path.
    async fn seed_both_platforms(
        app: &App,
        id: InstanceId,
        metadata_id: &str,
        cf: carbon_platforms::ModChannel,
        mr: carbon_platforms::ModChannel,
    ) {
        metarepo::upsert_cf_mod_cache(
            &app.db,
            0,
            4242,
            100,
            "Fixture".into(),
            "1.0.0".into(),
            "fixture".into(),
            "summary".into(),
            "author".into(),
            cf as i32,
            "1.20.1,fabric,beta".into(),
            DbDateTime(Utc::now().fixed_offset()),
            metadata_id.into(),
        )
        .await
        .unwrap();
        let sha512 = hex::encode(
            metarepo::get_metadata_by_id(&app.db, metadata_id)
                .await
                .unwrap()
                .unwrap()
                .sha512,
        );
        metarepo::upsert_mr_mod_cache(
            &app.db,
            sha512,
            "project".into(),
            "installed".into(),
            "Fixture".into(),
            "1.0.0".into(),
            "fixture".into(),
            "description".into(),
            "author".into(),
            mr as i32,
            String::new(),
            "mod.jar".into(),
            "https://fixture.invalid/mod.jar".into(),
            DbDateTime(Utc::now().fixed_offset()),
            metadata_id.into(),
        )
        .await
        .unwrap();
        let context = modrinth::target_compatibility(app, CacheEntityId::Instance(id))
            .await
            .unwrap()
            .key("mods");
        metarepo::upsert_mr_update_context(
            &app.db,
            metadata_id,
            &context,
            "1.20.1,fabric,beta",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
    }

    /// One jar can be a Release on one platform and a Beta on the other. It is
    /// held to its most stable label on both, so neither platform's newer
    /// beta badges it.
    #[tokio::test]
    async fn a_jar_either_platform_calls_a_release_is_not_badged_for_betas() {
        use carbon_platforms::ModChannel::{Beta, Stable};
        for (name, cf, mr, expected) in [
            ("cf-release-mr-beta", Stable, Beta, false),
            ("cf-beta-mr-release", Beta, Stable, false),
            ("cf-beta-mr-beta", Beta, Beta, true),
        ] {
            let app = crate::setup_managers_for_test().await;
            let (id, metadata_id) = release_install_fixture(&app, name).await;
            seed_both_platforms(&app, id, &metadata_id, cf, mr).await;
            assert_eq!(badged(&app, id).await, expected, "{name}");
        }
    }

    #[tokio::test]
    async fn modrinth_release_install_is_not_badged_for_a_newer_beta() {
        let app = crate::setup_managers_for_test().await;
        let (id, metadata_id) = release_install_fixture(&app, "mr-release-install").await;
        let sha512 = hex::encode(
            metarepo::get_metadata_by_id(&app.db, &metadata_id)
                .await
                .unwrap()
                .unwrap()
                .sha512,
        );
        metarepo::upsert_mr_mod_cache(
            &app.db,
            sha512,
            "project".into(),
            "installed".into(),
            "Fixture".into(),
            "1.0.0".into(),
            "fixture".into(),
            "description".into(),
            "author".into(),
            carbon_platforms::ModChannel::Stable as i32,
            String::new(),
            "mod.jar".into(),
            "https://fixture.invalid/mod.jar".into(),
            DbDateTime(Utc::now().fixed_offset()),
            metadata_id.clone(),
        )
        .await
        .unwrap();
        let context = modrinth::target_compatibility(&app, CacheEntityId::Instance(id))
            .await
            .unwrap()
            .key("mods");
        let seed = |paths: &'static str| {
            metarepo::upsert_mr_update_context(
                &app.db,
                &metadata_id,
                &context,
                paths,
                DbDateTime(Utc::now().fixed_offset()),
            )
        };

        seed("1.20.1,fabric,beta").await.unwrap();
        assert!(
            !badged(&app, id).await,
            "a Release with only a newer beta is up to date"
        );

        seed("1.20.1,fabric,beta;1.20.1,fabric,stable")
            .await
            .unwrap();
        assert!(badged(&app, id).await, "a newer Release is an update");
    }

    #[test]
    fn addon_has_extension_ignores_case_and_the_disabled_suffix() {
        // A CurseForge/Modrinth-supplied filename's extension case is not
        // guaranteed — `cache_server_local`'s scan must still recognize it,
        // or an installed `Pack.ZIP` datapack is invisible to it (unlistable,
        // unremovable).
        assert!(addon_has_extension("Pack.ZIP", "zip"));
        assert!(addon_has_extension("cool-mod.JAR", "jar"));
        assert!(addon_has_extension("cool-mod.jar", "jar"));

        // The disabled marker this app appends must not itself count as the
        // extension, and case-insensitivity still applies underneath it.
        assert!(addon_has_extension("Pack.ZIP.disabled", "zip"));
        assert!(addon_has_extension("cool-mod.jar.disabled", "jar"));

        // A mismatched or absent extension is rejected either way.
        assert!(!addon_has_extension("Pack.zip", "jar"));
        assert!(!addon_has_extension("readme.txt", "zip"));
        assert!(!addon_has_extension("no-extension", "jar"));
    }

    // Pins the exact drop-then-`remove_if` eviction pattern
    // `ensure_mod_metadata` runs on `metadata_hash_locks` right before it
    // returns, against a standalone map of the same shape — no DB-backed
    // `App` needed, since the pattern itself (acquire, settle, drop the
    // guard and this call's own `Arc` clone, then `remove_if` on
    // `Arc::strong_count == 1`) is what's under test, not the metadata
    // lookup/insert `ensure_mod_metadata` wraps it around.
    #[tokio::test]
    async fn hash_lock_evicts_once_unreferenced_but_survives_a_concurrent_holder() {
        let locks: dashmap::DashMap<(Vec<u8>, i32), Arc<Mutex<()>>> = dashmap::DashMap::new();
        let key = (vec![1, 2, 3], 42);

        // No concurrent holder: the settled entry must be evicted.
        let lock = locks
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let guard = lock.lock().await;
        drop(guard);
        drop(lock);
        locks.remove_if(&key, |_, v| Arc::strong_count(v) == 1);
        assert!(
            !locks.contains_key(&key),
            "an unreferenced hash lock must be evicted once its holder settles"
        );

        // A concurrent holder — its own `Arc` clone, standing in for a
        // second in-flight `ensure_mod_metadata` call racing the same hash —
        // must keep the entry alive through the first caller's eviction
        // attempt.
        let lock = locks
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let concurrent_holder = lock.clone();
        let guard = lock.lock().await;
        drop(guard);
        drop(lock);
        locks.remove_if(&key, |_, v| Arc::strong_count(v) == 1);
        assert!(
            locks.contains_key(&key),
            "a lock a concurrent caller still holds must never be evicted out from under it"
        );
        drop(concurrent_holder);
    }

    /// `override_caching_and_wait` resolves through a callback that only the
    /// startup cache loops ever fire, so those loops have to be running in
    /// test builds too. Gating them out does not fail anything — it blocks
    /// forever, which is far more expensive to diagnose in CI than a failure.
    /// Exporting an instance goes through this path.
    #[tokio::test]
    async fn override_caching_and_wait_resolves_in_test_builds() {
        let app = crate::setup_managers_for_test().await;

        // A missing instance is enough: the local cacher logs and returns
        // `Ok(())` for it, which still fires the callback this awaits. What
        // is under test is that something fires it at all.
        let waited = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            app.meta_cache_manager().override_caching_and_wait(
                CacheEntityId::Instance(InstanceId(1)),
                false,
                false,
            ),
        )
        .await;

        assert!(
            waited.is_ok(),
            "override_caching_and_wait never resolved — the startup cache \
             loops are not running in this build"
        );
    }
}
