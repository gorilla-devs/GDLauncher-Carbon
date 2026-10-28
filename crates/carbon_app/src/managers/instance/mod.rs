pub mod datapacks;
mod deletion;
use self::export::InstanceExportManager;
use self::importer::InstanceImportManager;
use self::log::GameLog;
use self::run::{LaunchState, PersistenceManager, RunningInstance};
use super::ManagerRef;
use super::metadata::cache;
use super::modplatforms::curseforge::CurseForge;
use super::orphan_pid;
use super::vtask::{TaskState, VisualTask};
use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::info::{GameVersion, InstanceIcon, Modpack};
use crate::domain::instance::{
    self as domain, GameLogId, GroupId, InstanceFolder, InstanceId, InstanceModpackInfo,
};
use crate::domain::java::{SYSTEM_JAVA_PROFILE_NAME_PREFIX, SystemJavaProfileName};
use crate::domain::vtask::VisualTaskId;
use crate::livenesstracker::LivenessTracker;
use crate::managers::instance::modpack::PackVersionFile;
use anyhow::bail;
use anyhow::{Context, anyhow};
use carbon_platforms::ModPlatform;
use carbon_platforms::curseforge::filters::{ModFileParameters, ModParameters};
use carbon_platforms::modrinth::search::{ProjectID, VersionID};
use carbon_repos::repos::instance::{self as instance_repo, IndexShift};
use chrono::{DateTime, Utc};
use daedalus::minecraft::MinecraftJavaProfile;
use dashmap::DashMap;
use domain::info;
use fs_extra::dir::CopyOptions;
use futures::StreamExt;
use futures::future::BoxFuture;
use futures::{Future, join};
use instance_repo::InstanceRow as CachedInstance;
use serde::Serialize;
use serde_json::error::Category as JsonErrorType;
use specta::Type;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, io, ops::Deref, path::PathBuf};
use sysinfo::{Pid, ProcessesToUpdate, System};
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard, RwLock, watch};
use tracing::{error, info, trace, warn};
use unicode_segmentation::UnicodeSegmentation;

#[cfg(test)]
mod lifecycle_tests;

pub mod explore;
pub mod export;
pub mod importer;
pub mod installer;
pub mod log;
pub mod modpack;
mod mods;
pub use mods::KeptAddonCopy;
pub mod run;
mod schema;
pub mod shader_loader;

#[derive(Debug)]
pub struct InstanceManager {
    #[cfg(test)]
    test_hooks: lifecycle_tests::Hooks,
    pub(crate) instances: RwLock<HashMap<InstanceId, Instance>>,
    index_lock: Mutex<()>,
    // seperate lock to prevent a deadlock with the index lock
    path_lock: Mutex<()>,
    // Shared ownership gate for existing-instance config writers and root lifecycle.
    // Always acquire before a map guard; never wait for this gate holding the map.
    instance_op_locks: Arc<DashMap<InstanceId, Arc<Mutex<()>>>>,
    // Keeps an instance folder in place while a duplicate copies it without
    // the gate (see `hold_root`). Order: gate, then this, then path_lock.
    instance_root_locks: Arc<DashMap<InstanceId, Arc<RwLock<()>>>>,
    // Playtime a failed instance.json write could not save, added to the
    // instance's next playtime write instead of being lost.
    unsaved_playtime: DashMap<InstanceId, u32>,
    loaded_icon: Mutex<Option<(String, Vec<u8>)>>,
    persistence_manager: PersistenceManager,
    import_manager: InstanceImportManager,
    export_manager: InstanceExportManager,
    game_logs: RwLock<
        HashMap<
            GameLogId,
            (
                InstanceId,
                chrono::DateTime<chrono::Local>,
                watch::Receiver<GameLog>,
            ),
        >,
    >,
    // One independent disk-history initialization per instance; never tied to live logs.
    log_history: DashMap<InstanceId, Arc<Mutex<bool>>>,
    /// Stops the installed-version check of a running version change (see
    /// `cancel_version_change_check`).
    version_change_checks: modpack::VersionChangeChecks,
    modpack_info_semaphore: Mutex<()>,
    pub any_instance_running: Arc<watch::Sender<bool>>,
    instance_running_tracker: Arc<LivenessTracker>,
    /// Results of the last completed (or failed-but-partial) run of
    /// [`modpack::check_pack_origin`](ManagerRef::check_pack_origin) per
    /// instance — read by `modpack::origin_verdict_for` when building a
    /// repair preview. In-memory only: a restart clears it, same as every
    /// other manager cache in this struct.
    pub(crate) origin_checks: RwLock<HashMap<InstanceId, modpack::origin_check::OriginResults>>,
}

/// Keeps a per-ID gate stable for every holder and queued waiter. Retire an
/// idle entry only while holding its DashMap shard exclusively: a concurrent
/// lookup either clones this gate before retirement or creates a new idle one.
#[derive(Debug)]
pub(crate) struct InstanceOperationGuard {
    locks: Arc<DashMap<InstanceId, Arc<Mutex<()>>>>,
    id: InstanceId,
    lock: Option<Arc<Mutex<()>>>,
    guard: Option<tokio::sync::OwnedMutexGuard<()>>,
    #[cfg(test)]
    drop_barrier: Option<Arc<std::sync::Barrier>>,
}

impl InstanceOperationGuard {
    pub(crate) fn instance_id(&self) -> InstanceId {
        self.id
    }
}

impl Drop for InstanceOperationGuard {
    fn drop(&mut self) {
        self.guard.take();
        let departing = self
            .lock
            .take()
            .expect("operation owns its gate until drop");
        retire_if_idle(&self.locks, self.id, departing);
        #[cfg(test)]
        if let Some(barrier) = &self.drop_barrier {
            barrier.wait();
        }
    }
}

/// Drops `departing`, one holder's reference to `id`'s entry, and retires the
/// entry when no other holder or waiter has it.
fn retire_if_idle<T>(locks: &DashMap<InstanceId, Arc<T>>, id: InstanceId, departing: Arc<T>) {
    locks.remove_if(&id, |_, lock| {
        let same_gate = Arc::ptr_eq(lock, &departing);
        // Release the wrapper reference while the shard is exclusively
        // held. Concurrent last departures cannot both leave that release
        // until after checking the count and accidentally miss retirement.
        drop(departing);
        same_gate && Arc::strong_count(lock) == 1
    });
}

/// Keeps an instance folder where it is (see
/// [`hold_root`](InstanceManager::hold_root)). Retired like
/// [`InstanceOperationGuard`].
#[derive(Debug)]
pub(crate) struct InstanceRootGuard {
    locks: Arc<DashMap<InstanceId, Arc<RwLock<()>>>>,
    id: InstanceId,
    lock: Option<Arc<RwLock<()>>>,
    shared: Option<tokio::sync::OwnedRwLockReadGuard<()>>,
    exclusive: Option<tokio::sync::OwnedRwLockWriteGuard<()>>,
}

impl Drop for InstanceRootGuard {
    fn drop(&mut self) {
        self.shared.take();
        self.exclusive.take();
        let departing = self
            .lock
            .take()
            .expect("a folder hold owns its lock until drop");
        retire_if_idle(&self.locks, self.id, departing);
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceManager {
    #[cfg(test)]
    pub(crate) async fn pause_addon_scan_for_test(&self, id: InstanceId) {
        self.test_hooks.pause("addon_scan_owned", id).await;
    }

    /// Lock before reading the current config/path. Global map guards must
    /// remain short; release them before path/DB/filesystem/network waits.
    /// Rename/delete take path_lock after this gate, never in reverse order,
    /// and a folder hold (`hold_root`) between the two.
    pub(crate) async fn lock_instance(&self, id: InstanceId) -> InstanceOperationGuard {
        let lock = self
            .instance_op_locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let mut operation = InstanceOperationGuard {
            locks: self.instance_op_locks.clone(),
            id,
            lock: Some(lock),
            guard: None,
            #[cfg(test)]
            drop_barrier: self.test_hooks.drop_barrier(id),
        };
        operation.guard = Some(operation.lock.as_ref().unwrap().clone().lock_owned().await);
        operation
    }

    /// Keeps the instance's folder where it is, taken while holding its gate
    /// (`operation`). A duplicate shares it and then releases the gate, so
    /// copying a large folder never holds up Play, settings, logs or a game's
    /// exit on the original. Deleting the folder or a rename that moves it
    /// takes it exclusively, so they wait for those copies to finish rather
    /// than break them. Writers hold the gate while they hold this, so a
    /// shared hold taken under the gate never waits.
    pub(crate) async fn hold_root(
        &self,
        operation: &InstanceOperationGuard,
        exclusive: bool,
    ) -> InstanceRootGuard {
        let id = operation.instance_id();
        let lock = self
            .instance_root_locks
            .entry(id)
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone();
        let mut root = InstanceRootGuard {
            locks: self.instance_root_locks.clone(),
            id,
            lock: Some(lock),
            shared: None,
            exclusive: None,
        };
        if exclusive {
            root.exclusive = Some(root.lock.as_ref().unwrap().clone().write_owned().await);
        } else {
            root.shared = Some(root.lock.as_ref().unwrap().clone().read_owned().await);
        }
        root
    }

    pub fn new() -> Self {
        let any_instance_running = Arc::new(watch::channel(false).0);

        Self {
            #[cfg(test)]
            test_hooks: Default::default(),
            instances: RwLock::new(HashMap::new()),
            index_lock: Mutex::new(()),
            path_lock: Mutex::new(()),
            instance_op_locks: Arc::new(DashMap::new()),
            instance_root_locks: Arc::new(DashMap::new()),
            unsaved_playtime: DashMap::new(),
            loaded_icon: Mutex::new(None),
            persistence_manager: PersistenceManager::new(),
            import_manager: InstanceImportManager::new(),
            export_manager: InstanceExportManager::new(),
            game_logs: RwLock::new(HashMap::new()),
            log_history: DashMap::new(),
            version_change_checks: DashMap::new(),
            modpack_info_semaphore: Mutex::new(()),
            any_instance_running: any_instance_running.clone(),
            instance_running_tracker: LivenessTracker::new(move |count| {
                drop(any_instance_running.send_replace(count != 0))
            }),
            origin_checks: RwLock::new(HashMap::new()),
        }
    }
}

const MAX_PATH: usize = if cfg!(windows) { 260 } else { 4096 };
const ILLEGAL_CHARS: &[char] = &['/', ':', '\\', '<', '>', '*', '|', '"', '?', '^'];
const ILLEGAL_NAMES: &[&str] = &[
    "con", "prn", "aux", "clock$", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
    "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Filename of the on-disk marker recording an instance's current game JVM
/// pid. Lives directly under the instance's root directory
/// (`InstancePath::get_root`, the parent of the `instance/` data dir) rather
/// than inside the data dir, so a reinstall/repair that wipes `instance/`
/// doesn't touch it, and deleting the instance (which removes the whole
/// root) cleans it up for free. Both the writer (`run`, right after the game
/// process is spawned) and the reader (`reconcile_running_instances_under`,
/// on every core startup) compute the pidfile path from the same
/// `InstancePath::get_root()` / directory-scan root, so they always agree
/// on the location.
///
/// This file is the only channel by which a restarted core can learn that a
/// game it did not spawn is still running: `LaunchState` lives in memory and
/// dies with the process that held it, so without the pidfile a game that
/// outlived the launcher would be invisible to the instance it belongs to.
pub(crate) const PID_FILE_NAME: &str = ".gdl_instance.pid";

fn sanitize_name(name: &str) -> String {
    let mut sanitized = name.trim().to_string();

    if ILLEGAL_NAMES.contains(&(&name.to_lowercase() as &str)) {
        sanitized = format!("_{}", sanitized);
    }

    if sanitized.starts_with('.') || sanitized.starts_with('~') {
        sanitized.replace_range(0..1, "_");
    }

    if sanitized.ends_with('.') || sanitized.ends_with('~') {
        sanitized.replace_range(sanitized.len() - 1.., "_");
    }

    sanitized
        .chars()
        .map(|c| if ILLEGAL_CHARS.contains(&c) { '_' } else { c })
        .collect()
}

fn truncate_name(name: &str, instance_path: &Path) -> String {
    // Reserve three decimal digits for collision suffixes. Component limits
    // apply independently of MAX_PATH (bytes on Unix, UTF-16 units on Windows).
    let available_length = 255usize
        .saturating_sub(3)
        .min(MAX_PATH.saturating_sub(path_length(instance_path).saturating_add(1 + 3 + 1)));
    let mut length = 0;
    name.graphemes(true)
        .take_while(|g| {
            length += path_length(Path::new(g));
            length <= available_length
        })
        .collect()
}

#[cfg(windows)]
fn path_length(path: &Path) -> usize {
    path.as_os_str().encode_wide().count()
}

#[cfg(not(windows))]
fn path_length(path: &Path) -> usize {
    path.as_os_str().len()
}

async fn stage_instance_icon(
    root: &Path,
    icon: Option<(String, Vec<u8>)>,
) -> anyhow::Result<InstanceIcon> {
    match icon {
        Some((name, data)) => {
            // The caller's basename is metadata, never a staging destination.
            // Keep recognized formats without decoding or restricting image bytes.
            let extension = Path::new(&name)
                .extension()
                .and_then(|s| s.to_str())
                .map(str::to_ascii_lowercase)
                .filter(|s| {
                    matches!(
                        s.as_str(),
                        "png"
                            | "jpg"
                            | "jpeg"
                            | "gif"
                            | "webp"
                            | "svg"
                            | "avif"
                            | "ico"
                            | "bmp"
                            | "tif"
                            | "tiff"
                            | "apng"
                            | "heic"
                            | "heif"
                            | "jxl"
                    )
                })
                .unwrap_or_else(|| "png".into());
            let path = format!("icon.{extension}");
            tokio::fs::write(root.join(&path), data)
                .await
                .context("saving instance icon")?;
            Ok(InstanceIcon::RelativePath(path))
        }
        None => Ok(InstanceIcon::Default),
    }
}

/// Characters illegal in a Windows filename — and unsafe or meaningless in a
/// path component on any other OS — that `sanitize_icon_filename` strips from
/// a URL-derived icon name before it touches disk.
const ILLEGAL_FILENAME_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Longest filename `download_icon` will write to disk, regardless of how
/// long the URL's own last path segment is.
const MAX_ICON_FILENAME_LEN: usize = 64;

/// Turn a URL path segment into a filename safe to write directly to disk:
/// every character illegal in a Windows filename becomes `_`, the result is
/// capped to `MAX_ICON_FILENAME_LEN` characters, and a result that is empty
/// or only dots (`.`, `..` — a bare `.`/`..` segment carries no name at all,
/// and a leading `..` on a case that ever reached this far would otherwise
/// walk out of the icon directory) falls back to a fixed safe name rather
/// than passing through unchanged. The URL a `download_icon` filename is
/// derived from is caller-supplied, so none of this is optional.
fn sanitize_icon_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if ILLEGAL_FILENAME_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .take(MAX_ICON_FILENAME_LEN)
        .collect();

    match sanitized.as_str() {
        "" | "." | ".." => "icon".to_string(),
        _ => sanitized,
    }
}

/// The filename `download_icon` should use, derived from `url`'s own last
/// path segment — `path_segments()` excludes `?query` and `#fragment` by
/// construction, so `.../icon.png?width=64` yields `icon.png`, never
/// `icon.png?width=64`. `None` when the URL has no non-empty path segment to
/// use (e.g. it ends in `/`), so the caller can fall back to the response's
/// `Content-Type` instead.
fn icon_filename_from_url(url: &reqwest::Url) -> Option<String> {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|segment| !segment.is_empty())
        .map(sanitize_icon_filename)
}

/// The file extension implied by a response's `Content-Type` header value
/// (`image/png` -> `png`), lowercased. `png` if the header is absent,
/// unparseable, or has no `/subtype` to read one from.
fn extension_from_content_type(content_type: Option<&str>) -> String {
    content_type
        .and_then(|value| value.split(';').next())
        .and_then(|mime| mime.trim().rsplit_once('/'))
        .map(|(_, subtype)| subtype.trim().to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "png".to_string())
}

/// The filename `download_icon` actually writes to disk: `url_segment` (from
/// `icon_filename_from_url`) verbatim when it already carries an extension
/// (contains a `.`), otherwise with one appended from `content_type` — and
/// the same content-type-derived extension, on a plain `icon` stem, when
/// there was no usable URL segment at all. A URL segment with no extension
/// (`.../noext`) must not silently lose the response's real content type the
/// way falling back only on "no segment at all" would.
fn icon_filename(url_segment: Option<String>, content_type: Option<&str>) -> String {
    match url_segment {
        Some(name) if name.contains('.') => name,
        Some(name) => format!("{name}.{}", extension_from_content_type(content_type)),
        None => format!("icon.{}", extension_from_content_type(content_type)),
    }
}

impl<'s> ManagerRef<'s, InstanceManager> {
    pub async fn launch_background_tasks(self) {
        if let Err(e) = self.scan_instances().await {
            error!("failed to scan instances: {e:?}");
            self.app.runtime_lifecycle.reconciliation_finished(Err(e));
        }
        self.import_manager().launch_background_tasks();
    }

    pub async fn scan_instances(self) -> anyhow::Result<()> {
        let runtime_activity = self
            .app
            .runtime_lifecycle
            .admit("instance ownership rescan")?;
        // A quit, restart or update cancels this. The scan then stops before
        // its next step or instance folder instead of making the quit wait
        // for a large library on a slow disk: every folder it started is
        // finished, and the next start scans them all again.
        let closing = runtime_activity.cancellation();
        let ownership = self
            .app
            .runtime_lifecycle
            .reconcile_root(
                &self
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_instances()
                    .to_path(),
                PID_FILE_NAME,
            )
            .await;
        let ownership_reconciled = match ownership {
            Ok(()) => true,
            Err(error) => {
                self.app
                    .runtime_lifecycle
                    .reconciliation_finished(Err(error));
                false
            }
        };
        let scan_start = std::time::Instant::now();

        // Before scanning populates the in-memory instance map, reconcile
        // every on-disk instance's pidfile against the live process table.
        // `LaunchState` is in-memory only (never persisted), so the scan
        // below yields every instance as freshly Inactive and the pidfile is
        // the only surviving record that a game is still running — from a
        // session that ended without stopping it, most often because the user
        // closed the launcher. Each one found is applied to its instance as
        // it is inserted below, so the map is never briefly wrong about it.
        let adopted = if ownership_reconciled && !closing.is_cancelled() {
            self.reconcile_running_instances().await
        } else {
            // Preserve unknown pid evidence; best-effort cleanup must not make
            // the next startup falsely quiescent. Config scanning remains usable.
            HashMap::new()
        };

        let instance_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        #[cfg(test)]
        self.test_hooks.pause("scan_admission", InstanceId(0)).await;
        let mut stream = match tokio::fs::read_dir(instance_path).await {
            Ok(stream) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };

        let updates_semaphore = Arc::new(tokio::sync::Semaphore::new(20));
        let mut scanned_count: u32 = 0;
        // Instances that came out of this scan running a game this core does
        // not own. Collected because nothing else will ever observe those
        // processes exiting — there is no `child.wait()` behind them — so
        // they need the liveness poller started for them.
        let mut adopted_ids: Vec<InstanceId> = Vec::new();
        let mut interrupted = false;

        while let Some(dir) = stream.next_entry().await? {
            if closing.is_cancelled() {
                interrupted = true;
                break;
            }
            let path = dir.path();

            let shortpath = path
                .file_name()
                .expect("path given to scan_instance should never have a null filename")
                .to_str()
                .expect("current GDL versions only support UTF8 paths")
                .to_string();

            // One directory's failure (an add_instance/get_default_group
            // error — scan_instance itself never fails, converting a bad
            // config into an Invalid instance instead) must not abort the
            // whole scan: instances already adopted from earlier entries
            // would then never reach the watcher spawn below, and every
            // directory after this one would silently go unscanned too.
            // Isolated and logged instead so the rest of the scan keeps
            // going.
            let entry_result: anyhow::Result<Option<InstanceId>> = async {
                // Resolve current root ownership, not the scan's starting DB
                // snapshot. Creation/rename hold path_lock through publication.
                // Never await an ID gate while holding path_lock: release it,
                // acquire the gate, then revalidate under path ownership.
                let (_operation, path_admission, cached) = loop {
                    let path_admission = self.path_lock.lock().await;
                    let cached =
                        instance_repo::get_instance_by_shortpath(&self.app.db, &shortpath).await?;
                    let Some(row) = cached else {
                        // No identity exists yet. Keep root ownership through
                        // config validation, non-replacing registration and map
                        // publication; another producer cannot claim it midway.
                        break (None, path_admission, None);
                    };
                    drop(path_admission);
                    let operation = self.lock_instance(InstanceId(row.id)).await;
                    let path_admission = self.path_lock.lock().await;
                    let current =
                        instance_repo::get_instance_by_shortpath(&self.app.db, &shortpath).await?;
                    if current.as_ref().map(|row| row.id) == Some(row.id) {
                        break (Some(operation), path_admission, current);
                    }
                    // The observed identity moved or was retired while we
                    // waited. Re-admit the current owner before touching JSON.
                    drop(path_admission);
                    drop(operation);
                };
                if let Some(row) = &cached {
                    let instances = self.instances.read().await;
                    if instances.get(&InstanceId(row.id)).is_some_and(|instance| {
                        instance.shortpath != shortpath
                            || matches!(
                                instance.data().map(|data| &data.state),
                                Ok(LaunchState::Deleting)
                            )
                    }) {
                        return Ok(None);
                    }
                }

                // A setup replacement a crash interrupted takes effect (or
                // goes) before anything reads this instance's setup.
                // Replacements are written under the instance gate, which
                // this scan holds for a known instance, so none is being
                // written.
                if tokio::fs::symlink_metadata(&path)
                    .await
                    .is_ok_and(|metadata| metadata.is_dir())
                {
                    if let Err(error) = modpack::session::settle_replacement(&path).await {
                        tracing::warn!(?error, "unfinished setup replacement left as is");
                    }
                }

                let Some(mut instance) = self
                    .scan_instance(shortpath.clone(), path.clone(), cached.as_ref())
                    .await?
                else {
                    return Ok(None);
                };
                let is_valid = matches!(&instance.type_, InstanceType::Valid(_));
                let instance_id = match &cached {
                    Some(cached) => InstanceId(cached.id),
                    None => {
                        let InstanceType::Valid(data) = &instance.type_ else {
                            return Ok(None);
                        };
                        self.add_instance(
                            data.config.name.clone(),
                            instance.shortpath.clone(),
                            self.get_default_group().await?,
                        )
                        .await?
                    }
                };

                let mut instances = self.instances.write().await;
                // Invalid startup entries need recovery tiles, but an invalid
                // rescan must not discard an existing launch's state/kill owner.
                if !is_valid
                    && instances.get(&instance_id).is_some_and(|old| {
                        old.data()
                            .is_ok_and(|data| !matches!(data.state, LaunchState::Inactive { .. }))
                    })
                {
                    return Ok(None);
                }

                if let (
                    Instance {
                        type_: InstanceType::Valid(data),
                        ..
                    },
                    Some(Instance {
                        type_: InstanceType::Valid(old_data),
                        ..
                    }),
                ) = (&mut instance, instances.remove(&instance_id))
                {
                    data.state = old_data.state;
                }

                // A game this core did not spawn, still running: present the
                // instance as Running with no channel to signal and no log to
                // read (see `RunningInstance::adopted`). Guarded on the state
                // rather than applied unconditionally so a rescan can never
                // replace a state this core does own — the line above carries an
                // already-Running instance forward, and overwriting it here would
                // throw away its `kill_tx` and strand the process.
                if let Some(game) = adopted.get(&instance.shortpath).copied() {
                    if let InstanceType::Valid(data) = &mut instance.type_ {
                        if matches!(data.state, LaunchState::Inactive { .. }) {
                            data.state = LaunchState::Running(RunningInstance::adopted_record(
                                game.verified_process,
                                game.start_time,
                                Utc::now(),
                            ));
                            self.app
                                .runtime_lifecycle
                                .adopted_record_published(game.verified_process);
                            adopted_ids.push(instance_id);
                        }
                    }
                }

                instances.insert(instance_id, instance);
                drop(instances);
                drop(path_admission);
                drop(_operation);
                if !is_valid {
                    return Ok(None);
                }

                self.app
                    .meta_cache_manager()
                    .queue_caching(
                        crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                        false,
                    )
                    .await;

                Ok(Some(instance_id))
            }
            .await;

            let instance_id = match entry_result {
                Ok(Some(instance_id)) => instance_id,
                Ok(None) => continue,
                Err(e) => {
                    warn!(
                        "failed to scan instance directory {}: {e:?}",
                        path.display()
                    );
                    continue;
                }
            };

            scanned_count += 1;

            let app = self.app.clone();
            let updates_semaphore = Arc::clone(&updates_semaphore);
            tokio::task::spawn(async move {
                let _permit = updates_semaphore.acquire().await.unwrap();

                trace!("Instance modpack update for {instance_id}",);

                // ignore errors
                let (_, _) = join!(
                    app.instance_manager()
                        .check_curseforge_modpack_updates(instance_id),
                    app.instance_manager()
                        .check_modrinth_modpack_updates(instance_id),
                );

                tokio::time::sleep(Duration::from_millis(10)).await;
            });
        }

        tracing::debug!(
            "[startup-timing] scan_instances scanned {} instance dir(s), total {:.2}s",
            scanned_count,
            scan_start.elapsed().as_secs_f64()
        );

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        if !adopted_ids.is_empty() {
            self.watch_adopted_instances(adopted_ids);
        }
        if interrupted {
            // Only a move of the runtime folder needs every folder checked.
            // It is refused, naming this, until GDLauncher starts again.
            tracing::info!(
                "Instance scan stopped after {scanned_count} folder(s): GDLauncher is closing"
            );
            self.app
                .runtime_lifecycle
                .reconciliation_finished(Err(anyhow!(
                    "The instance scan stopped early because GDLauncher began closing"
                )));
        }

        Ok(())
    }

    /// Scan the given path as an instance folder.
    ///
    /// If cached is Some an Instance will always be returned, though it may be missing files.
    /// If cached is None an Instance will only be returned if a config file is present.
    async fn scan_instance(
        self,
        shortpath: String,
        path: PathBuf,
        cached: Option<&CachedInstance>,
    ) -> anyhow::Result<Option<Instance>> {
        let config_path = path.join("instance.json");

        let config_text = match tokio::fs::read_to_string(config_path.clone()).await {
            Ok(x) => x,
            Err(e) => {
                // if we aren't already tracking this instance just ignore it.
                if cached.is_some() {
                    let invalid_type = match e.kind() {
                        io::ErrorKind::NotFound => InvalidConfiguration::NoFile,
                        _ => InvalidConfiguration::IoError(e.to_string()),
                    };

                    return Ok(Some(Instance {
                        shortpath: shortpath.clone(),
                        type_: InstanceType::Invalid(invalid_type),
                    }));
                } else {
                    return Ok(None);
                }
            }
        };

        #[cfg(test)]
        self.test_hooks.pause_path("scan_read", &shortpath).await;
        #[cfg(test)]
        if let Some(cached) = cached {
            self.test_hooks.pause("scan", InstanceId(cached.id)).await;
        }

        match schema::parse_and_update_instance_config(self.app.clone(), &config_text, config_path)
            .await
        {
            Ok(config) => {
                let icon_revision = match &config.icon {
                    InstanceIcon::Default => None,
                    InstanceIcon::RelativePath(_) => Some(1),
                };

                let instance = InstanceData {
                    favorite: cached.map(|cached| cached.favorite).unwrap_or(false),
                    config,
                    state: run::LaunchState::Inactive { failed_task: None },
                    modpack_update_curseforge: None,
                    modpack_update_modrinth: None,
                    icon_revision,
                };

                Ok(Some(Instance {
                    shortpath: shortpath.clone(),
                    type_: InstanceType::Valid(instance),
                }))
            }
            Err(e) => {
                let try_downcast = e.downcast_ref::<serde_json::Error>();
                let error = InvalidConfiguration::Invalid(ConfigurationParseError {
                    type_: try_downcast
                        .map(|e| match e.classify() {
                            JsonErrorType::Data => ConfigurationParseErrorType::Data,
                            JsonErrorType::Syntax => ConfigurationParseErrorType::Syntax,
                            JsonErrorType::Eof => ConfigurationParseErrorType::Eof,
                            JsonErrorType::Io => unreachable!(),
                        })
                        .unwrap_or(ConfigurationParseErrorType::Unknown),
                    line: try_downcast.map(|e| e.line()).unwrap_or_default() as u32, // will panic with more lines but that dosen't really seem like a problem
                    message: try_downcast
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| e.to_string()),
                    config_text,
                });

                Ok(Some(Instance {
                    shortpath,
                    type_: InstanceType::Invalid(error),
                }))
            }
        }
    }

    /// Reconcile every on-disk instance's pidfile against the live process
    /// table, returning the games still running — by shortpath, because this
    /// runs before any instance id exists — and discarding stale or reused
    /// pids. Entirely best-effort: every failure is logged and swallowed so
    /// this can never fail or delay startup scanning.
    async fn reconcile_running_instances(self) -> HashMap<String, AdoptedGame> {
        let instances_root = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        reconcile_running_instances_under(&instances_root).await
    }

    pub async fn list_groups(self) -> anyhow::Result<Vec<ListGroup>> {
        let (groups, all_instances) = self
            .app
            .db
            .read(|conn| {
                // reads share one WAL snapshot
                let snap = conn.snapshot()?;
                let groups = instance_repo::get_all_groups_ordered_by_group_index_conn(&snap)?;
                let instances = instance_repo::get_all_instances_ordered_by_index_conn(&snap)?;
                Ok((groups, instances))
            })
            .await?;

        // Nest instances under their group, preserving the index-asc order the
        // query returned them in.
        let mut instances_by_group: HashMap<i32, Vec<CachedInstance>> = HashMap::new();
        for instance in all_instances {
            instances_by_group
                .entry(instance.group_id)
                .or_default()
                .push(instance);
        }

        let active_instances = self.instances.read().await;
        Ok(groups
            .into_iter()
            .map(|group| ListGroup {
                id: GroupId(group.id),
                name: group.name,
                library_position: group.library_position,
                instances: instances_by_group
                    .remove(&group.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(
                        |instance| match active_instances.get(&InstanceId(instance.id)) {
                            Some(data) => Some((instance, &data.type_)),
                            None => None,
                        },
                    )
                    .map(|(instance, status)| ListInstance {
                        id: InstanceId(instance.id),
                        group_id: GroupId(instance.group_id),
                        index: instance.index,
                        library_position: instance.library_position,
                        name: instance.name,
                        favorite: instance.favorite,
                        icon_revision: match &status {
                            InstanceType::Valid(data) => data.icon_revision,
                            InstanceType::Invalid(_) => None,
                        },
                        status: match status {
                            InstanceType::Valid(status) => {
                                ListInstanceStatus::Valid(ValidListInstance {
                                    mc_version: match &status.config.game_configuration.version {
                                        Some(GameVersion::Standard(version)) => {
                                            Some(version.release.clone())
                                        }
                                        Some(GameVersion::Custom(name)) => Some(name.clone()),
                                        None => None,
                                    },
                                    modloader: match &status.config.game_configuration.version {
                                        Some(GameVersion::Standard(version)) => {
                                            match version.modloaders.iter().next() {
                                                Some(modloader) => Some(modloader.type_),
                                                None => None,
                                            }
                                        }
                                        Some(GameVersion::Custom(_)) => None,
                                        None => None,
                                    },
                                    modloader_version: match &status
                                        .config
                                        .game_configuration
                                        .version
                                    {
                                        Some(GameVersion::Standard(version)) => version
                                            .modloaders
                                            .iter()
                                            .next()
                                            .map(|m| m.version.clone()),
                                        Some(GameVersion::Custom(_)) => None,
                                        None => None,
                                    },
                                    modpack: status
                                        .config
                                        .modpack
                                        .as_ref()
                                        .map(|modpack| modpack.modpack.clone()),
                                    state: (&status.state).into(),
                                })
                            }
                            InstanceType::Invalid(status) => {
                                ListInstanceStatus::Invalid(match status {
                                    InvalidConfiguration::NoFile => {
                                        InvalidListInstance::JsonMissing
                                    }
                                    InvalidConfiguration::Invalid(error) => {
                                        InvalidListInstance::JsonError(error.clone())
                                    }
                                    InvalidConfiguration::IoError(error) => {
                                        InvalidListInstance::Other(error.clone())
                                    }
                                })
                            }
                        },
                        locked: match status {
                            InstanceType::Valid(status) => status
                                .config
                                .modpack
                                .as_ref()
                                .map(|modpack| modpack.locked)
                                .unwrap_or(false),
                            InstanceType::Invalid(status) => false,
                        },
                        last_played: match status {
                            InstanceType::Valid(status) => status.config.last_played,
                            InstanceType::Invalid(status) => None,
                        },
                        date_created: match status {
                            InstanceType::Valid(status) => status.config.date_created,
                            InstanceType::Invalid(status) => DateTime::default(),
                        },
                        date_updated: match status {
                            InstanceType::Valid(status) => status.config.date_updated,
                            InstanceType::Invalid(status) => DateTime::default(),
                        },
                        seconds_played: match status {
                            InstanceType::Valid(status) => status.config.seconds_played,
                            InstanceType::Invalid(status) => 0,
                        },
                    })
                    .collect::<Vec<_>>(),
            })
            .collect::<Vec<_>>())
    }

    /// Move the given group to a position in the library based on the target.
    /// Groups can now be interleaved with ungrouped instances using libraryPosition.
    pub async fn move_group(self, group: GroupId, target: GroupMoveTarget) -> anyhow::Result<()> {
        // lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        // Get the group we're moving
        let group_id = *group;
        let moving_group = instance_repo::get_group(&self.app.db, group_id)
            .await?
            .ok_or_else(|| anyhow!("GroupId is not in database, this should never happen"))?;

        let start_pos = moving_group.library_position;

        // Determine the target libraryPosition based on the target type
        let target_pos = match target {
            GroupMoveTarget::BeforeGroup(target_group_id) => {
                let target_group_id = *target_group_id;
                let target_group = instance_repo::get_group(&self.app.db, target_group_id)
                    .await?
                    .ok_or_else(|| anyhow!("Target GroupId is not in database"))?;

                target_group.library_position.ok_or_else(|| {
                    anyhow!("Target group has no libraryPosition (is it the default group?)")
                })?
            }
            GroupMoveTarget::BeforeInstance(instance_id) => {
                // Instance must be in the default group (ungrouped) — detected via
                // library_position being set (only default-group instances have one).
                let instance_id = *instance_id;
                let instance = instance_repo::get_instance(&self.app.db, instance_id)
                    .await?
                    .ok_or_else(|| anyhow!("InstanceId is not in database"))?;

                instance.library_position.ok_or_else(|| {
                    anyhow!(
                        "Can only position a group before ungrouped instances (instances in default group)"
                    )
                })?
            }
            GroupMoveTarget::EndOfLibrary => {
                // Find the maximum libraryPosition across ungrouped instances and groups.
                // library_position is only set on default-group instances, so the
                // filter alone is sufficient (no need to join against the default group).
                let (max_instance_pos, max_group_pos) = self
                    .app
                    .db
                    .read(|conn| {
                        // reads share one WAL snapshot
                        let snap = conn.snapshot()?;
                        let inst = instance_repo::max_library_position_instance_conn(&snap)?
                            .and_then(|i| i.library_position);
                        let grp = instance_repo::max_library_position_group_conn(&snap)?
                            .and_then(|g| g.library_position);
                        Ok((inst, grp))
                    })
                    .await?;

                let max_pos = max_instance_pos
                    .unwrap_or(0)
                    .max(max_group_pos.unwrap_or(0));
                max_pos + 1
            }
        };

        let Some(start_pos) = start_pos else {
            // Group has no libraryPosition (shouldn't happen for non-default groups)
            bail!("Group has no libraryPosition - cannot move");
        };

        if start_pos == target_pos {
            return Ok(());
        }

        // Shift libraryPositions of items between start and target
        if start_pos < target_pos {
            // Moving forward: shift items in (start, target] down by 1 (groups
            // and ungrouped instances), then place the group at target - 1.
            // Three writes run in one writer dispatch so no other write
            // interleaves; they run in ONE transaction —
            // all-or-nothing: a failure rolls the whole group back and readers
            // never observe an intermediate state. `_conn` forms on the tx guard.
            self.app
                .db
                .write(move |mut conn| {
                    let tx = conn.transaction()?;
                    instance_repo::shift_group_library_positions_down_conn(
                        &tx,
                        start_pos,
                        target_pos - 1,
                    )?;
                    instance_repo::shift_instance_library_positions_down_conn(
                        &tx,
                        start_pos,
                        target_pos - 1,
                    )?;
                    instance_repo::set_group_library_position_conn(
                        &tx,
                        group_id,
                        Some(target_pos - 1),
                    )?;
                    tx.commit()?;
                    Ok(())
                })
                .await?;
        } else {
            // Moving backward: shift items in [target, start) up by 1, then
            // place the group at target.
            // Three writes run in one writer dispatch so no other write
            // interleaves; they run in ONE transaction —
            // all-or-nothing: a failure rolls the whole group back and readers
            // never observe an intermediate state. `_conn` forms on the tx guard.
            self.app
                .db
                .write(move |mut conn| {
                    let tx = conn.transaction()?;
                    instance_repo::shift_group_library_positions_up_conn(
                        &tx, target_pos, start_pos,
                    )?;
                    instance_repo::shift_instance_library_positions_up_conn(
                        &tx, target_pos, start_pos,
                    )?;
                    instance_repo::set_group_library_position_conn(
                        &tx,
                        group_id,
                        Some(target_pos),
                    )?;
                    tx.commit()?;
                    Ok(())
                })
                .await?;
        }

        // Also keep groupIndex in sync for backwards compatibility: it's the
        // legacy ordering field, restamped here from the library-position order.
        let all_groups =
            instance_repo::get_groups_with_library_position_ordered(&self.app.db).await?;

        // Interleaved app logic: restamp every group's index from the ordered
        // in-memory list. Runs in one writer dispatch, so no other write
        // interleaves; they run in ONE transaction —
        // all-or-nothing: a failure rolls the whole group back and readers
        // never observe an intermediate state. `_conn` forms on the tx guard.
        self.app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                for (idx, g) in all_groups.iter().enumerate() {
                    instance_repo::set_group_index_conn(&tx, g.id, idx as i32)?;
                }
                tx.commit()?;
                Ok(())
            })
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        Ok(())
    }

    /// Move the given instance to the index directly before `target` in the target instance group.
    /// If `target` is None, move to the end of the instance group.
    /// Also handles libraryPosition for instances in the default group (library root).
    pub async fn move_instance(
        self,
        instance: InstanceId,
        target: InstanceMoveTarget,
    ) -> anyhow::Result<()> {
        // Materialize the default group before taking index_lock (see create_group).
        let default_group_id = self.get_default_group().await?;

        // lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        let instance_id = *instance;
        let (start_group, start_idx, start_library_pos) = {
            let instance = instance_repo::get_instance(&self.app.db, instance_id)
                .await?
                .ok_or_else(|| {
                    anyhow!("InstanceId is not in database, this should never happen")
                })?;

            (
                GroupId(instance.group_id),
                instance.index,
                instance.library_position,
            )
        };

        let (target_group, target_idx, target_library_pos) = match target {
            InstanceMoveTarget::Before(target) => {
                let target_id = *target;
                let inst = instance_repo::get_instance(&self.app.db, target_id)
                    .await?
                    .ok_or_else(|| {
                        anyhow!("InstanceId is not in database, this should never happen")
                    })?;

                (GroupId(inst.group_id), inst.index, inst.library_position)
            }
            InstanceMoveTarget::BeginningOfGroup(group) => {
                let group_val = *group;
                // If target is default group, find the minimum libraryPosition
                let lib_pos = if group == default_group_id {
                    let min_pos = instance_repo::min_library_position_instance_in_group(
                        &self.app.db,
                        group_val,
                    )
                    .await?
                    .and_then(|i| i.library_position);

                    // If no instances with libraryPosition, start at 0
                    Some(min_pos.unwrap_or(0))
                } else {
                    None
                };

                // Indices are prepend-allocated (see `next_instance_index`), so
                // the "beginning" is strictly less than the current minimum, not
                // a fixed 0.
                let min_idx: Option<i32> =
                    instance_repo::min_index_instance_in_group(&self.app.db, group_val)
                        .await?
                        .map(|i| i.index);

                let target_idx = min_idx.map(|n| n - 1).unwrap_or(0);
                (group, target_idx, lib_pos)
            }
            InstanceMoveTarget::EndOfGroup(group) => {
                let group_val = *group;
                let target_idx =
                    instance_repo::max_index_instance_in_group(&self.app.db, group_val)
                        .await?
                        .map(|row| {
                            row.index
                                .checked_add(1)
                                .ok_or_else(|| anyhow!("instance index overflow"))
                        })
                        .transpose()?
                        .unwrap_or(0);

                // If target is default group, find the maximum libraryPosition + 1
                let lib_pos = if group == default_group_id {
                    let (max_instance_pos, max_group_pos) = self
                        .app
                        .db
                        .read(move |conn| {
                            // reads share one WAL snapshot
                            let snap = conn.snapshot()?;
                            let inst = instance_repo::max_library_position_instance_in_group_conn(
                                &snap, group_val,
                            )?
                            .and_then(|i| i.library_position);
                            let grp = instance_repo::max_library_position_group_conn(&snap)?
                                .and_then(|g| g.library_position);
                            Ok((inst, grp))
                        })
                        .await?;

                    let max_pos = max_instance_pos
                        .unwrap_or(0)
                        .max(max_group_pos.unwrap_or(0));
                    Some(max_pos + 1)
                } else {
                    None
                };

                (group, target_idx, lib_pos)
            }
            InstanceMoveTarget::BeforeGroup(group_id) => {
                // Position instance before a folder (at library root level)
                // Get the folder's libraryPosition
                let folder_id = *group_id;
                let target_folder = instance_repo::get_group(&self.app.db, folder_id)
                    .await?
                    .ok_or_else(|| anyhow!("GroupId is not in database"))?;

                let lib_pos = target_folder
                    .library_position
                    .ok_or_else(|| anyhow!("Target folder has no libraryPosition"))?;

                // The instance will be moved to the default group with target libraryPosition
                let default_id = *default_group_id;
                let target_idx =
                    instance_repo::max_index_instance_in_group(&self.app.db, default_id)
                        .await?
                        .map(|row| {
                            row.index
                                .checked_add(1)
                                .ok_or_else(|| anyhow!("instance index overflow"))
                        })
                        .transpose()?
                        .unwrap_or(0);

                (default_group_id, target_idx, Some(lib_pos))
            }
        };

        let index_shifts: Vec<IndexShift> = if start_group == target_group {
            match (start_idx, target_idx) {
                (start, target) if start < target => vec![IndexShift::DownExclusive {
                    group_id: *target_group,
                    gt: start,
                    lt: target,
                }],
                (start, target) if start > target => vec![IndexShift::UpRange {
                    group_id: *target_group,
                    gte: target,
                    lt: start,
                }],
                _ if start_library_pos == target_library_pos => return Ok(()),
                _ => Vec::new(),
            }
        } else {
            vec![
                IndexShift::DownAfter {
                    group_id: *start_group,
                    gt: start_idx,
                },
                IndexShift::UpFrom {
                    group_id: *target_group,
                    gte: target_idx,
                },
            ]
        };

        // When moving forward in the same group, the source ends up at target - 1
        // because we're shifting items in (start, target) down, not including target
        let final_idx = if start_group == target_group && start_idx < target_idx {
            target_idx - 1
        } else {
            target_idx
        };

        // Determine the new libraryPosition
        let new_library_pos = if target_group == default_group_id {
            // Moving to default group: need to set libraryPosition
            target_library_pos
        } else {
            // Moving to a folder: clear libraryPosition
            None
        };

        let default_id = *default_group_id;
        // If moving TO default group and inserting before an item, shift library positions
        if target_group == default_group_id {
            if let Some(target_lib_pos) = target_library_pos {
                // Only shift if we have a target position (not end of group with no items)
                if start_library_pos != Some(target_lib_pos) {
                    // Two shifts run in one writer dispatch so no other write
                    // interleaves; they run in ONE transaction —
                    // all-or-nothing: a failure rolls the whole group back and readers
                    // never observe an intermediate state. `_conn` forms on the tx guard.
                    self.app
                        .db
                        .write(move |mut conn| {
                            let tx = conn.transaction()?;
                            // Shift libraryPosition of items at or after target_lib_pos
                            // (not the instance we're moving), and the groups too.
                            instance_repo::shift_instance_library_positions_up_in_group_except_conn(
                                &tx,
                                default_id,
                                target_lib_pos,
                                instance_id,
                            )?;
                            instance_repo::shift_all_group_library_positions_up_from_conn(
                                &tx,
                                target_lib_pos,
                            )?;
                            tx.commit()?;
                            Ok(())
                        })
                        .await?;
                }
            }
        }

        // If moving FROM default group, shift library positions to fill the gap
        if start_group == default_group_id && target_group != default_group_id {
            if let Some(start_lib_pos) = start_library_pos {
                // Two shifts run in one writer dispatch so no other write
                // interleaves; they run in ONE transaction —
                // all-or-nothing: a failure rolls the whole group back and readers
                // never observe an intermediate state. `_conn` forms on the tx guard.
                self.app
                    .db
                    .write(move |mut conn| {
                        let tx = conn.transaction()?;
                        // Decrement libraryPosition of items after start_lib_pos,
                        // groups included.
                        instance_repo::shift_instance_library_positions_down_in_group_conn(
                            &tx,
                            default_id,
                            start_lib_pos,
                        )?;
                        instance_repo::shift_all_group_library_positions_down_after_conn(
                            &tx,
                            start_lib_pos,
                        )?;
                        tx.commit()?;
                        Ok(())
                    })
                    .await?;
            }
        }

        let final_group = *target_group;
        instance_repo::move_instance_tx(
            &self.app.db,
            index_shifts,
            instance_id,
            final_group,
            final_idx,
            new_library_pos,
        )
        .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        // Auto-dissolve a non-default group left empty or with a single
        // instance after moving one out: a one-instance folder is pointless,
        // so its last instance also returns to the default group.
        if start_group != default_group_id && start_group != target_group {
            let start_group_id = *start_group;
            let remaining_count =
                instance_repo::count_instances_in_group(&self.app.db, start_group_id).await?;

            if remaining_count == 0 {
                // Delete the now-empty group
                instance_repo::delete_group(&self.app.db, start_group_id).await?;
                // GET_GROUPS already invalidated above, but invalidate again after deletion
                self.app.invalidate(GET_GROUPS, None);
            } else if remaining_count == 1 {
                if let Some(last) =
                    instance_repo::first_instance_in_group(&self.app.db, start_group_id).await?
                {
                    // Moving the last instance out empties the group, which the
                    // recursive call's branch above then deletes. Release the
                    // index lock first since move_instance re-acquires it.
                    drop(_index_lock);
                    Box::pin(self.move_instance(
                        InstanceId(last.id),
                        InstanceMoveTarget::EndOfGroup(default_group_id),
                    ))
                    .await?;
                }
            }
        }

        Ok(())
    }

    pub fn get_default_group(self) -> BoxFuture<'s, anyhow::Result<GroupId>> {
        Box::pin(async move {
            static DEFAULT_MUTEX: Mutex<()> = Mutex::const_new(());

            let groupid = self
                .app
                .settings_manager()
                .get_settings()
                .await?
                .default_instance_group;

            match groupid {
                Some(groupid) => {
                    let group = instance_repo::get_group(&self.app.db, groupid).await?;

                    match group {
                        Some(x) => Ok(GroupId(x.id)),
                        None => bail!(
                            "invalid database state: default group specified in configuration, but missing from groups"
                        ),
                    }
                }
                None => {
                    match DEFAULT_MUTEX.try_lock() {
                        Ok(_lock) => {
                            let index = self.next_group_index().await?;
                            let index_value = index.value;

                            let group_id =
                                instance_repo::create_default_group_tx(&self.app.db, index_value)
                                    .await?;

                            Ok(GroupId(group_id))
                        }
                        Err(_) => {
                            // Wait for the lock to finish, some other thread probably
                            // wrote the group to the DB at this point, so just retry getting it from the db.
                            let _ = DEFAULT_MUTEX.lock().await;
                            self.get_default_group().await
                        }
                    }
                }
            }
        })
    }

    pub async fn create_group(self, name: String) -> anyhow::Result<GroupId> {
        let index = self.next_group_index().await?;

        let existing = {
            let name = name.clone();
            instance_repo::find_group_by_name(&self.app.db, &name).await?
        };

        if let Some(group) = existing {
            return Ok(GroupId(group.id));
        }

        // Instances in the default group (and only those) have library_position set,
        // so filtering by LibraryPosition IS NOT NULL avoids needing default_group_id
        // here — which would otherwise deadlock via get_default_group re-acquiring
        // index_lock on a fresh database.
        let (max_instance_pos, max_group_pos) = self
            .app
            .db
            .read(|conn| {
                // reads share one WAL snapshot
                let snap = conn.snapshot()?;
                let inst = instance_repo::max_library_position_instance_conn(&snap)?
                    .and_then(|i| i.library_position);
                let grp = instance_repo::max_library_position_group_conn(&snap)?
                    .and_then(|g| g.library_position);
                Ok((inst, grp))
            })
            .await?;

        let next_library_pos = max_instance_pos
            .unwrap_or(0)
            .max(max_group_pos.unwrap_or(0))
            + 1;

        let index_value = index.value;
        let group_id =
            instance_repo::insert_group(&self.app.db, name, index_value, Some(next_library_pos))
                .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(GroupId(group_id as i32))
    }

    /// Create a group at a specific library position, shifting existing items to make room.
    pub async fn create_group_at_position(
        self,
        name: String,
        target_position: i32,
    ) -> anyhow::Result<GroupId> {
        let index = self.next_group_index().await?;

        // Shift all items (groups and ungrouped instances) with library_position >= target_position up by 1
        // (library_position is only set on default-group instances, so no need to filter by group_id
        // — which would require materializing the default group and re-acquiring index_lock.)
        let index_value = index.value;
        // Interleaved app logic: shift existing items up then insert the new
        // group at the freed position. Runs in one writer dispatch, so no other
        // write interleaves; they run in ONE transaction —
        // all-or-nothing: a failure rolls the whole group back and readers
        // never observe an intermediate state. `_conn` forms on the tx guard.
        let group_id = self
            .app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                instance_repo::shift_all_instance_library_positions_up_conn(&tx, target_position)?;
                instance_repo::shift_all_group_library_positions_up_from_conn(
                    &tx,
                    target_position,
                )?;
                // Create the group at the target position
                let id = instance_repo::insert_group_conn(
                    &tx,
                    &name,
                    index_value,
                    Some(target_position),
                )?;
                tx.commit()?;
                Ok(id)
            })
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(GroupId(group_id as i32))
    }

    pub async fn rename_group(self, group: GroupId, name: String) -> anyhow::Result<()> {
        let group_id = *group;
        instance_repo::set_group_name(&self.app.db, group_id, &name).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Generate a unique folder name by appending (1), (2), etc. if needed.
    async fn generate_unique_folder_name(&self, base_name: &str) -> anyhow::Result<String> {
        // Check if base name exists
        let existing = {
            let base_name = base_name.to_string();
            instance_repo::find_group_by_name(&self.app.db, &base_name).await?
        };

        if existing.is_none() {
            return Ok(base_name.to_string());
        }

        // Find next available number
        let mut counter = 1;
        loop {
            let candidate = format!("{} ({})", base_name, counter);
            let exists = {
                let candidate = candidate.clone();
                instance_repo::find_group_by_name(&self.app.db, &candidate).await?
            };

            if exists.is_none() {
                return Ok(candidate);
            }
            counter += 1;
        }
    }

    /// Create a new folder (group) from a list of instances.
    /// The folder is named "New Folder" by default, with (1), (2), etc. appended if needed.
    /// If target_instance_id is provided and is at the library root, the folder is created
    /// at that instance's position instead of at the end.
    pub async fn create_folder_from_instances(
        self,
        instance_ids: Vec<InstanceId>,
        target_instance_id: Option<InstanceId>,
    ) -> anyhow::Result<GroupId> {
        if instance_ids.is_empty() {
            bail!("Cannot create folder from empty list of instances");
        }

        // Generate a unique folder name
        let folder_name = self.generate_unique_folder_name("New Folder").await?;

        // Determine the target library position if target instance is at library root
        let target_library_pos = if let Some(target_id) = target_instance_id {
            let default_group_id = self.get_default_group().await?;
            let target_id = *target_id;
            let target_instance = instance_repo::get_instance(&self.app.db, target_id).await?;

            // Only use position if instance exists, is in default group, and has a library_position
            target_instance
                .filter(|i| i.group_id == *default_group_id)
                .and_then(|i| i.library_position)
        } else {
            None
        };

        // Create group at target position or at end
        let group_id = match target_library_pos {
            Some(pos) => self.create_group_at_position(folder_name, pos).await?,
            None => self.create_group(folder_name).await?,
        };

        // Move all instances to the new group
        for instance_id in instance_ids {
            self.move_instance(instance_id, InstanceMoveTarget::EndOfGroup(group_id))
                .await?;
        }

        Ok(group_id)
    }

    /// Arrange all ungrouped instances (in default group) and folders by the given criteria.
    /// This is a one-off arrange operation that reassigns library positions.
    /// Used in folders mode only (instancesGroupBy = null).
    pub async fn arrange_library(self, sort_by: LibrarySortCriteria) -> anyhow::Result<()> {
        let default_group_id = self.get_default_group().await?;

        // Lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        // Get all instances in the default group (ungrouped instances)
        let default_id = *default_group_id;
        let instances = instance_repo::get_instances_by_group(&self.app.db, default_id).await?;

        // Get instance data for sorting
        let active_instances = self.instances.read().await;

        // Build a sortable list of (instance_id, name, last_played, seconds_played)
        let mut sortable_instances: Vec<(i32, String, Option<DateTime<Utc>>, u32)> = instances
            .iter()
            .map(|inst| {
                let instance_data = active_instances.get(&InstanceId(inst.id));
                let (last_played, seconds_played) = match instance_data {
                    Some(data) => match &data.type_ {
                        InstanceType::Valid(valid) => {
                            (valid.config.last_played, valid.config.seconds_played)
                        }
                        InstanceType::Invalid(_) => (None, 0),
                    },
                    None => (None, 0),
                };
                (inst.id, inst.name.clone(), last_played, seconds_played)
            })
            .collect();

        drop(active_instances);

        // Sort based on criteria
        match sort_by {
            LibrarySortCriteria::Name => {
                sortable_instances.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            }
            LibrarySortCriteria::LastPlayed => {
                sortable_instances.sort_by(|a, b| match (&b.2, &a.2) {
                    (Some(b_date), Some(a_date)) => b_date.cmp(a_date),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
                });
            }
            LibrarySortCriteria::MostPlayed => {
                sortable_instances.sort_by(|a, b| {
                    b.3.cmp(&a.3)
                        .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
                });
            }
            LibrarySortCriteria::DateCreated => {
                // Use instance id as proxy for creation order (lower id = older)
                sortable_instances.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }

        // Fetch and sort non-default groups (folders) by name. They appear
        // after ungrouped instances in the library listing.
        let groups = instance_repo::get_all_groups(&self.app.db).await?;

        let mut sortable_groups: Vec<(i32, String)> = groups
            .iter()
            .filter(|g| g.id != *default_group_id)
            .map(|g| (g.id, g.name.clone()))
            .collect();
        sortable_groups.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        // Folders always come first in the library, followed by ungrouped
        // instances. The frontend enforces the folders-first rule at sort
        // time; we mirror it here so the DB positions agree with what the
        // user sees.
        //
        // The frontend sorts top-level items by `library_position ?? index`
        // (or `libraryPosition ?? 10000` for folders). Writing `index`
        // alone is invisible whenever `library_position` is set, so we
        // stamp both: positions `0..M-1` for folders, `M..M+N-1` for
        // ungrouped instances.
        let mut group_updates = Vec::new();
        // Default group has no library_position (it's not rendered as a
        // top-level folder); keep group_index at 0 as before.
        group_updates.push(instance_repo::GroupArrange {
            id: *default_group_id,
            group_index: 0,
            library_position: None,
            set_library_position: false,
        });
        for (i, (group_id, _)) in sortable_groups.iter().enumerate() {
            let p = i as i32;
            group_updates.push(instance_repo::GroupArrange {
                id: *group_id,
                group_index: (i + 1) as i32,
                library_position: Some(p),
                set_library_position: true,
            });
        }

        let instance_base = sortable_groups.len() as i32;
        let mut instance_updates = Vec::new();
        for (i, (instance_id, _, _, _)) in sortable_instances.iter().enumerate() {
            let p = instance_base + i as i32;
            instance_updates.push(instance_repo::InstanceArrange {
                id: *instance_id,
                index: i as i32,
                library_position: Some(p),
            });
        }

        instance_repo::arrange_library_tx(&self.app.db, group_updates, instance_updates).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Arrange all instances within a specific folder by the given criteria.
    /// This is a one-off arrange operation that reassigns indices within the folder.
    /// Used in folders mode only (instancesGroupBy = null).
    pub async fn arrange_group(
        self,
        group_id: GroupId,
        sort_by: LibrarySortCriteria,
    ) -> anyhow::Result<()> {
        // Lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        // Get all instances in the specified group
        let group_id_val = *group_id;
        let instances = instance_repo::get_instances_by_group(&self.app.db, group_id_val).await?;

        // Get instance data for sorting
        let active_instances = self.instances.read().await;

        // Build a sortable list of (instance_id, name, last_played, seconds_played)
        let mut sortable_instances: Vec<(i32, String, Option<DateTime<Utc>>, u32)> = instances
            .iter()
            .map(|inst| {
                let instance_data = active_instances.get(&InstanceId(inst.id));
                let (last_played, seconds_played) = match instance_data {
                    Some(data) => match &data.type_ {
                        InstanceType::Valid(valid) => {
                            (valid.config.last_played, valid.config.seconds_played)
                        }
                        InstanceType::Invalid(_) => (None, 0),
                    },
                    None => (None, 0),
                };
                (inst.id, inst.name.clone(), last_played, seconds_played)
            })
            .collect();

        drop(active_instances);

        // Sort based on criteria
        match sort_by {
            LibrarySortCriteria::Name => {
                sortable_instances.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            }
            LibrarySortCriteria::LastPlayed => {
                sortable_instances.sort_by(|a, b| match (&b.2, &a.2) {
                    (Some(b_date), Some(a_date)) => b_date.cmp(a_date),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
                });
            }
            LibrarySortCriteria::MostPlayed => {
                sortable_instances.sort_by(|a, b| {
                    b.3.cmp(&a.3)
                        .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
                });
            }
            LibrarySortCriteria::DateCreated => {
                // Use instance id as proxy for creation order (lower id = older)
                sortable_instances.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }

        // Update indices for all instances in the group
        let updates: Vec<(i32, i32)> = sortable_instances
            .iter()
            .enumerate()
            .map(|(new_index, (instance_id, _, _, _))| (*instance_id, new_index as i32))
            .collect();

        if !updates.is_empty() {
            instance_repo::set_instance_indexes_tx(&self.app.db, updates).await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Register a new identity without replacing an existing shortpath row.
    /// Production callers hold path_lock through DB and map publication.
    /// Does not invalidate.
    async fn add_instance(
        self,
        name: String,
        shortpath: String,
        group: GroupId,
    ) -> anyhow::Result<InstanceId> {
        // Materialize the default group before taking index_lock via
        // next_instance_index (see create_group).
        let default_group_id = self.get_default_group().await?;

        let index = self.next_instance_index(group).await?;
        let library_position = if group == default_group_id {
            // New instances/folders appear at the top of the library.
            // Pick a value strictly smaller than every existing
            // library_position across both ungrouped instances and groups.
            let default_id = *default_group_id;
            let (min_instance_pos, min_group_pos) = self
                .app
                .db
                .read(move |conn| {
                    // reads share one WAL snapshot
                    let snap = conn.snapshot()?;
                    let inst = instance_repo::min_library_position_instance_in_group_conn(
                        &snap, default_id,
                    )?
                    .and_then(|i| i.library_position);
                    let grp = instance_repo::min_library_position_group_conn(&snap)?
                        .and_then(|g| g.library_position);
                    Ok((inst, grp))
                })
                .await?;

            let current_min = match (min_instance_pos, min_group_pos) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };

            Some(current_min.map(|n| n - 1).unwrap_or(0))
        } else {
            None
        };

        let index_value = index.value;
        let group_id = *group;
        let new_id = instance_repo::register_instance_tx(
            &self.app.db,
            name,
            shortpath,
            index_value,
            group_id,
            library_position,
        )
        .await?;

        Ok(InstanceId(new_id as i32))
    }

    /// Remove an instance from the database without checking if it exists.
    /// Does not invalidate, and leaves the mod metadata it no longer uses for
    /// the caller to collect (`gc_mod_metadata`) outside path_lock.
    ///
    /// Deletes the instance and its `ModFileCache` rows in the same write
    /// transaction (`delete_instance_tx`) rather than relying on the
    /// `ON DELETE CASCADE` edge alone: the FK sweep falls back to leaving
    /// foreign keys off for the whole session when it meets a violation it
    /// cannot repair, and `GDL_DISABLE_FK_ENFORCEMENT=1` selects the same
    /// state, so on those sessions the cascade never fires. Mirrors the
    /// equivalent server-delete fix.
    async fn remove_instance(self, instance: InstanceId) -> anyhow::Result<()> {
        let instance_id = *instance;
        instance_repo::delete_instance_tx(&self.app.db, instance_id).await?;
        Ok(())
    }

    pub async fn set_favorite(self, instance_id: InstanceId, favorite: bool) -> anyhow::Result<()> {
        const MAX_FAVORITES: usize = 10;

        let mut instances = self.instances.write().await;

        // Verify instance exists
        if !instances.contains_key(&instance_id) {
            return Err(InvalidInstanceIdError(instance_id).into());
        }

        // If setting as favorite, check the limit before taking mutable borrow
        if favorite {
            let current_favorite_count = instances
                .iter()
                .filter(|(id, inst)| {
                    **id != instance_id
                        && matches!(&inst.type_, InstanceType::Valid(data) if data.favorite)
                })
                .count();

            if current_favorite_count >= MAX_FAVORITES {
                bail!(
                    "Maximum number of favorites ({}) reached. Remove a favorite before adding a new one.",
                    MAX_FAVORITES
                );
            }
        }

        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;
        // An instance whose instance.json is missing or broken keeps its
        // favorite only in the library row, which its tile and the dock show,
        // so it can still be taken off the dock.
        if let InstanceType::Valid(data) = &mut instance.type_ {
            data.favorite = favorite;
        }
        drop(instances);

        // Update database for target instance
        let id = *instance_id;
        instance_repo::set_instance_favorite(&self.app.db, id, favorite).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        Ok(())
    }

    /// Requires path_lock. DB rows reserve their component even if their
    /// folder is temporarily absent; creation must not publish into that identity.
    async fn next_available_folder(self, name: &str) -> anyhow::Result<(String, PathBuf)> {
        let reserved = instance_repo::get_all_instances(&self.app.db)
            .await?
            .into_iter()
            .map(|row| row.shortpath)
            .collect::<HashSet<_>>();
        self.next_folder_excluding(name, &reserved)
    }

    fn next_folder(&self, name: &str) -> anyhow::Result<(String, PathBuf)> {
        self.next_folder_excluding(name, &HashSet::new())
    }

    fn next_folder_excluding(
        &self,
        name: &str,
        reserved: &HashSet<String>,
    ) -> anyhow::Result<(String, PathBuf)> {
        if name.is_empty() {
            bail!("Attempted to find an instance directory name for an unnamed instance");
        }

        let mut sanitized_name = sanitize_name(name);
        let instance_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        if instance_path.exists() && !instance_path.is_dir() {
            bail!("GDL instances path is not a directory. Please move the file blocking it.");
        }

        sanitized_name = truncate_name(&sanitized_name, &instance_path);
        if sanitized_name.is_empty() {
            bail!("Instance name or parent path leaves no usable folder component");
        }

        for i in 0..1000 {
            let current_name = if i == 0 {
                sanitized_name.clone()
            } else {
                format!("{}{}", sanitized_name, i)
            };

            let full_path = instance_path.join(&current_name);
            if !full_path.exists() && !reserved.contains(&current_name) {
                return Ok((current_name, full_path));
            }
        }

        bail!("Unable to create a unique folder name after 1000 attempts")
    }

    pub async fn load_icon(self, icon: PathBuf) -> anyhow::Result<(String, Vec<u8>)> {
        let data = tokio::fs::read(icon.clone())
            .await
            .with_context(|| format!("Reading file `{}`", icon.to_string_lossy()))?;

        let extension = match icon.extension() {
            Some(ext) => ext,
            None => OsStr::new(""),
        };

        let icon_name = PathBuf::from("icon")
            .with_extension(extension)
            .to_string_lossy()
            .to_string();

        Ok((icon_name, data))
    }

    pub async fn download_icon(self, url: String) -> anyhow::Result<(String, Vec<u8>)> {
        // The URL is caller-supplied. Only follow it over http(s) so a `file://`
        // (or other) scheme can't be used to read arbitrary local resources.
        let parsed = reqwest::Url::parse(&url).context("invalid icon url")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            anyhow::bail!("icon url must be http or https");
        }

        const MAX_ICON_BYTES: usize = 10 * 1024 * 1024;

        let response = self
            .app
            .reqwest_client
            .get(&url)
            .send()
            .await?
            .error_for_status()?;

        // The parsed URL's own last path segment, never the raw query
        // string: a URL like `.../icon.png?width=64` must yield `icon.png`,
        // not `icon.png?width=64` — `path_segments()` already excludes
        // `?query` and `#fragment` by construction. `icon_filename` falls
        // back to the response's declared content type both when the URL
        // has no usable segment at all (e.g. it ends in `/`) and when its
        // segment has no extension of its own (e.g. `.../noext`).
        let icon_name = icon_filename(
            icon_filename_from_url(&parsed),
            response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
        );

        // Read with a hard cap instead of `.bytes()` so a large (or endless)
        // body can't exhaust memory.
        let mut stream = response.bytes_stream();
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if data.len() + chunk.len() > MAX_ICON_BYTES {
                anyhow::bail!("icon exceeds the {MAX_ICON_BYTES} byte limit");
            }
            data.extend_from_slice(&chunk);
        }

        Ok((icon_name, data))
    }

    /// [`download_icon`](Self::download_icon), but for callers to whom an
    /// icon is decorative rather than essential — a modpack archive import
    /// that shouldn't fail just because the platform-supplied icon URL has
    /// rotted. Any failure is `tracing::warn!`-logged and yields `None`
    /// rather than propagating, leaving the caller free to fall back to
    /// another icon source or import without one.
    pub async fn try_download_icon(self, url: &str) -> Option<(String, Vec<u8>)> {
        match self.download_icon(url.to_string()).await {
            Ok(icon) => Some(icon),
            Err(e) => {
                tracing::warn!("Failed to download icon, continuing without it: {e}");
                None
            }
        }
    }

    pub async fn set_loaded_icon(self, icon: (String, Vec<u8>)) {
        *self.loaded_icon.lock().await = Some(icon);
    }

    pub async fn create_instance(
        self,
        group: GroupId,
        name: String,
        use_loaded_icon: bool,
        version: InstanceVersionSource,
        notes: String,
    ) -> anyhow::Result<InstanceId> {
        let icon = match use_loaded_icon {
            true => self.loaded_icon.lock().await.take(),
            false => None,
        };

        self.create_instance_ext(group, name, icon, None, None, version, notes, |_| async {
            Ok(())
        })
        .await
    }

    #[tracing::instrument(skip(self, icon, initializer))]
    pub async fn create_instance_ext<F, I>(
        self,
        group: GroupId,
        name: String,
        icon: Option<(String, Vec<u8>)>,
        seconds_played: Option<u32>,
        last_played: Option<DateTime<Utc>>,
        version: InstanceVersionSource,
        notes: String,
        initializer: F,
    ) -> anyhow::Result<InstanceId>
    where
        F: FnOnce(PathBuf) -> I,
        I: Future<Output = anyhow::Result<()>>,
    {
        let activity = self.app.runtime_lifecycle.admit_owned(
            "borrowed create/import instance setup",
            crate::managers::runtime_lifecycle::ActivityPolicy::Installer,
        )?;
        crate::managers::runtime_lifecycle::run_borrowed_owned(
            activity,
            self.create_instance_ext_owned(
                group,
                name,
                icon,
                seconds_played,
                last_played,
                version,
                notes,
                initializer,
            ),
        )
        .await
    }

    async fn create_instance_ext_owned<F, I>(
        self,
        group: GroupId,
        name: String,
        icon: Option<(String, Vec<u8>)>,
        seconds_played: Option<u32>,
        last_played: Option<DateTime<Utc>>,
        version: InstanceVersionSource,
        notes: String,
        initializer: F,
    ) -> anyhow::Result<InstanceId>
    where
        F: FnOnce(PathBuf) -> I,
        I: Future<Output = anyhow::Result<()>>,
    {
        trace!("Creating instance");

        let tmpdir = self
            .app
            .settings_manager()
            .runtime_path
            .get_temp()
            .maketmpdir()
            .await?;

        tokio::fs::create_dir(tmpdir.join("instance")).await?;

        let icon = stage_instance_icon(&tmpdir, icon).await?;

        let (version, modpack, pack_locked) = match version {
            InstanceVersionSource::Version(version) => (Some(version), None, false),
            InstanceVersionSource::Modpack(modpack, locked) => (None, Some(modpack), locked),
            InstanceVersionSource::ModpackWithKnownVersion(version, modpack, locked) => {
                (Some(version), Some(modpack), locked)
            }
        };

        let info = info::Instance {
            name: name.clone(),
            icon,
            date_created: Utc::now(),
            date_updated: Utc::now(),
            last_played,
            seconds_played: seconds_played.unwrap_or(0),
            modpack: modpack.clone().map(|modpack| info::ModpackInfo {
                modpack,
                locked: pack_locked,
            }),
            game_configuration: info::GameConfig {
                version,
                global_java_args: true,
                extra_java_args: None,
                memory: None,
                java_override: None,
                game_resolution: None,
            },
            pre_launch_hook: None,
            post_exit_hook: None,
            wrapper_command: None,
            mod_sources: None,
            notes,
        };

        let json = schema::make_instance_config(info.clone())?;
        tokio::fs::write(tmpdir.join("instance.json"), json)
            .await
            .context("writing instance json")?;

        let setup_path = tmpdir.join(".setup");
        tokio::fs::create_dir(&setup_path)
            .await
            .context("writing setup marker")?;

        if let Some(modpack) = modpack {
            let pack_version_text = serde_json::to_string(&PackVersionFile::from(modpack))?;
            tokio::fs::write(
                setup_path.join("change-pack-version.json"),
                &pack_version_text,
            )
            .await?;
        }

        trace!("Running extended instance initializer");
        initializer(tmpdir.to_path_buf()).await?;
        trace!("Finished extended instance initializer");
        // Only this private producer knows that an empty setup really means a
        // version-only install. A scanner or copied root cannot infer it.
        let imported_complete = setup_path.join("modpack-complete").try_exists()?;
        let archive = ["curseforge", "modrinth", "gdlpack"]
            .iter()
            .any(|name| setup_path.join(name).is_file());
        let kind = if imported_complete {
            modpack::session::Kind::ImportedComplete
        } else if archive || info.modpack.is_some() {
            modpack::session::Kind::Pack
        } else {
            modpack::session::Kind::VersionOnly
        };
        let mut setup_session = modpack::session::Session::new(
            &info,
            if imported_complete || archive {
                modpack::session::Producer::Import
            } else {
                modpack::session::Producer::Create
            },
            kind,
            None,
        )?;
        setup_session.requested_pack =
            match tokio::fs::read_to_string(setup_path.join("change-pack-version.json")).await {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
        if imported_complete {
            setup_session.phase = modpack::session::Phase::Committed;
        }
        setup_session.create(&tmpdir).await?;

        tokio::fs::create_dir_all(
            self.app
                .settings_manager()
                .runtime_path
                .get_instances()
                .to_path(),
        )
        .await?;

        trace!("Locking path_lock");
        let path_lock = self.path_lock.lock().await;
        let (shortpath, path) = self.next_available_folder(&name).await?;

        tmpdir
            .try_rename_or_move(&path)
            .await
            .context("moving tmpdir to instance location")?;

        trace!("Created instance folder at '{path:?}'. Registering under path_lock");
        #[cfg(test)]
        self.test_hooks.pause("create_publish", InstanceId(0)).await;

        let id = self
            .add_instance(name.clone(), shortpath.clone(), group)
            .await?;

        // Registration may survive a crash before this write. Such an unbound
        // record remains recoverable; scanners never grant it the current ID.
        setup_session.bind_private(&path, id).await?;

        trace!("Adding instance to instances list");

        let icon_revision = match &info.icon {
            InstanceIcon::Default => None,
            InstanceIcon::RelativePath(_) => Some(1),
        };

        self.instances.write().await.insert(
            id,
            Instance {
                shortpath: shortpath.clone(),
                type_: InstanceType::Valid(InstanceData {
                    favorite: false,
                    config: info,
                    state: run::LaunchState::Inactive { failed_task: None },
                    modpack_update_curseforge: None,
                    modpack_update_modrinth: None,
                    icon_revision,
                }),
            },
        );

        drop(path_lock);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        info!({ shortpath = ?shortpath }, "Created new instance '{name}' (id {})", *id);

        Ok(id)
    }

    pub async fn update_instance(
        self,
        update: domain::InstanceSettingsUpdate,
    ) -> anyhow::Result<Option<VisualTaskId>> {
        let activity = self.app.runtime_lifecycle.admit_owned(
            format!("instance {} settings", update.instance_id),
            crate::managers::runtime_lifecycle::ActivityPolicy::CoreWork,
        )?;
        let app = self.app.clone();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |activity| {
            Box::pin(async move {
                app.instance_manager()
                    .update_instance_owned(update, activity)
                    .await
            })
        })
        .await?
    }

    async fn update_instance_owned(
        self,
        update: domain::InstanceSettingsUpdate,
        runtime_activity: &mut Option<crate::managers::runtime_lifecycle::RuntimeActivity>,
    ) -> anyhow::Result<Option<VisualTaskId>> {
        let _instance_guard = self.lock_instance(update.instance_id).await;

        // Snapshot only after taking ownership; disk writes never hold the map.
        let (mut shortpath, config, mut path, icon_revision) = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&update.instance_id)
                .ok_or(InvalidInstanceIdError(update.instance_id))?;

            let data = instance.type_.data()?;
            if matches!(data.state, LaunchState::Deleting) {
                bail!("cannot update an instance that is being deleted");
            }

            let changes_root_or_version = update
                .name
                .as_ref()
                .is_some_and(|name| name != &data.config.name)
                || update.version.is_some()
                || update.modloader.is_some();
            if changes_root_or_version && !matches!(data.state, LaunchState::Inactive { .. }) {
                bail!(
                    "cannot rename or reinstall while this instance is queued, preparing or running"
                );
            }
            let path = self
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&instance.shortpath)
                .get_root();

            (
                instance.shortpath.clone(),
                data.config.clone(),
                path,
                data.icon_revision,
            )
            // Read guard released here - other instances can now operate
        };

        // A rename that moves the folder waits, before writing anything, for
        // duplicates still copying it (see `hold_root`).
        let moves_root = update
            .name
            .as_ref()
            .is_some_and(|name| *name != config.name && *name != shortpath);
        let rename_root = if moves_root {
            Some(self.hold_root(&_instance_guard, true).await)
        } else {
            None
        };

        // A new version or loader choice replaces a pending setup that only
        // waits for its game version install (an unfinished or failed
        // version-only install of a new, edited or imported instance,
        // including one 2.0.40 left without a record, or a pack whose files
        // are fully installed, whether this instance installed it, the
        // instance a duplicate copied it from did, or 2.0.40 did and left no
        // record): it holds nothing the new choice could lose, as in 2.0.40.
        // Checked before instance.json is written, so a refusal changes
        // nothing.
        const WAITS_FOR_SETUP: &str = "This instance has an unfinished install or update. Press Play to finish it before changing the game version or loader.";
        let changes_version = update.version.is_some() || update.modloader.is_some();
        if changes_version {
            // A replacement whose publication failed (see
            // `settle_replacement`) is the pending setup: publish it first,
            // under the gate, so the check below sees it. One that still
            // can't be published is waited for like any pending setup.
            modpack::session::settle_replacement(&path)
                .await
                .context(WAITS_FOR_SETUP)?;
        }
        let mut supersede = false;
        if changes_version && path.join(".setup").try_exists()? {
            let mut pending =
                modpack::session::Session::load_or_adopt_legacy(&path, update.instance_id, &config)
                    .await
                    .with_context(|| {
                        mods::unreadable_setup_refusal(&config, mods::PendingChange::GameVersion)
                    })?;
            if let Some(found) = &mut pending {
                // A content-free record that is not this instance's own (a
                // duplicate copied it, it was written before the instance had
                // an id, or the folder kept it from an earlier registration,
                // such as before a database reset) becomes this instance's
                // own first, as Play would make it, so the instance is not
                // stuck repeating an install that failed before.
                if found.binds_to(update.instance_id)
                    && found
                        .supersedable_once_continued(&path, update.instance_id)
                        .await
                        .unwrap_or_else(|error| {
                            tracing::warn!(?error, "copied setup cannot be verified");
                            false
                        })
                {
                    match found.qualify_copy(&path, update.instance_id).await {
                        Ok(qualified) => *found = qualified,
                        Err(error) => {
                            tracing::warn!(?error, "copied setup cannot be continued")
                        }
                    }
                }
                supersede = found
                    .supersedable(&path, update.instance_id)
                    .await
                    .unwrap_or_else(|error| {
                        tracing::warn!(?error, "pending setup cannot be verified");
                        false
                    })
                    || found
                        .copied_pack_installed(&path, update.instance_id)
                        .await
                        .unwrap_or_else(|error| {
                            tracing::warn!(?error, "copied pack setup cannot be verified");
                            false
                        });
            } else {
                supersede = modpack::session::legacy_pack_applied(&path)
                    .await
                    .unwrap_or_else(|error| {
                        tracing::warn!(?error, "legacy pack setup cannot be verified");
                        false
                    });
            }
            if !supersede {
                let refusal = mods::pending_setup_refusal(
                    &path,
                    update.instance_id,
                    &config,
                    pending.as_ref(),
                    mods::PendingChange::GameVersion,
                )
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(?error, "pending setup cannot be described");
                    WAITS_FOR_SETUP.into()
                });
                bail!("{refusal}");
            }
        }

        #[cfg(test)]
        self.test_hooks.pause("settings", update.instance_id).await;

        // Phase 2: Apply all updates WITHOUT holding global lock
        let old_name = config.name.clone();
        let mut info = config;

        // Check what changed before consuming values (for conditional invalidation)
        let name_or_icon_changed = update.name.is_some() || update.use_loaded_icon.is_some();
        let modpack_lock_changed = update.modpack_locked.is_some();

        let mut new_icon_revision = icon_revision;

        if let Some(use_loaded_icon) = update.use_loaded_icon {
            let icon = match (use_loaded_icon, self.loaded_icon.lock().await.take()) {
                (true, Some((ipath, data))) => {
                    tokio::fs::write(path.join(&ipath), data)
                        .await
                        .context("saving instance icon")?;

                    if let InstanceIcon::RelativePath(oldpath) = &info.icon {
                        if *oldpath != ipath {
                            tokio::fs::remove_file(path.join(oldpath))
                                .await
                                .context("removing old instance icon")?;
                        }
                    }

                    InstanceIcon::RelativePath(ipath)
                }
                _ => InstanceIcon::Default,
            };

            info.icon = icon;
            new_icon_revision = match info.icon {
                InstanceIcon::Default => None,
                InstanceIcon::RelativePath(_) => Some(icon_revision.unwrap_or(1) + 1),
            };
        }

        if let Some(name) = update.name.clone() {
            info.name = name;
        }

        if let Some(notes) = update.notes {
            info.notes = notes;
        }

        if let Some(pre_launch_hook) = update.pre_launch_hook {
            info.pre_launch_hook = pre_launch_hook;
        }

        if let Some(post_exit_hook) = update.post_exit_hook {
            info.post_exit_hook = post_exit_hook;
        }

        if let Some(wrapper_command) = update.wrapper_command {
            info.wrapper_command = wrapper_command;
        }

        if let Some(java_override) = update.java_override {
            info!(?java_override, "Updating java override");
            info.game_configuration.java_override = java_override;
        }

        let mut need_reinstall = false;

        if let Some(version) = update.version {
            info.game_configuration.version =
                Some(info::GameVersion::Standard(info::StandardVersion {
                    release: version,
                    modloaders: match &info.game_configuration.version {
                        Some(info::GameVersion::Standard(info::StandardVersion {
                            modloaders,
                            ..
                        })) => modloaders.clone(),
                        _ => HashSet::new(),
                    },
                }));
            need_reinstall = true;
        }

        if let Some(modloader) = update.modloader {
            info.game_configuration.version =
                Some(info::GameVersion::Standard(info::StandardVersion {
                    release: match &info.game_configuration.version {
                        Some(info::GameVersion::Standard(info::StandardVersion {
                            release,
                            ..
                        })) => release.clone(),
                        _ => bail!("custom versions are not yet supported"),
                    },
                    modloaders: match modloader {
                        Some(modloader) => HashSet::from([modloader]),
                        None => HashSet::new(),
                    },
                }));
            need_reinstall = true;
        }

        if let Some(global_java_args) = update.global_java_args {
            info.game_configuration.global_java_args = global_java_args;
        }

        if let Some(extra_java_args) = update.extra_java_args {
            info.game_configuration.extra_java_args = extra_java_args;
        }

        if let Some(game_resolution) = update.game_resolution {
            info.game_configuration.game_resolution = game_resolution;
        }

        if let Some(memory) = update.memory {
            info.game_configuration.memory = memory;
        }

        if let Some(mod_sources) = update.mod_sources {
            info.mod_sources = mod_sources;
        }

        if let Some(modpack_locked) = update.modpack_locked {
            if let Some(modpack_locked) = modpack_locked {
                if let Some(modpack) = &mut info.modpack {
                    modpack.locked = modpack_locked;
                }
            } else {
                info.modpack = None;
            }
        }

        info.date_updated = Utc::now();

        let json = schema::make_instance_config(info.clone())?;

        self.app
            .settings_manager()
            .runtime_path
            .get_temp()
            .write_file_atomic(path.join("instance.json"), json)
            .await?;

        // A display-name edit always updates the library row, even when
        // its existing folder already has the requested name. Retain root
        // ownership through both DB and map publication.
        let mut rename_path_guard = None;
        if let Some(name) = update.name {
            if name != old_name {
                rename_path_guard = Some(self.path_lock.lock().await);
                if shortpath != name {
                    let (new_shortpath, new_path) = self.next_available_folder(&name).await?;
                    tokio::fs::rename(&path, &new_path).await?;
                    shortpath = new_shortpath;
                    path = new_path;
                }
                instance_repo::set_instance_name_and_shortpath(
                    &self.app.db,
                    *update.instance_id,
                    &name,
                    &shortpath,
                )
                .await?;
            }
        }

        // Phase 3: Update in-memory state under write lock (FAST - just assignment)
        {
            let mut instances = self.instances.write().await;
            let instance = instances
                .get_mut(&update.instance_id)
                .ok_or(InvalidInstanceIdError(update.instance_id))?;

            instance.shortpath = shortpath;

            if let InstanceType::Valid(data) = &mut instance.type_ {
                data.config = info;
                data.icon_revision = new_icon_revision;
            }
            // Write lock released here
        }

        drop(rename_path_guard);
        drop(rename_root);

        // Send conditional invalidations (no lock needed)
        // Only invalidate queries that are actually affected by the changes
        if name_or_icon_changed || modpack_lock_changed {
            // Name/icon/lock changes affect instance list display
            // (lock status shows in ListInstance.locked field)
            self.app.invalidate(GET_GROUPS, None);
            self.app.invalidate(GET_ALL_INSTANCES, None);
        }

        // Always invalidate instance details since settings changed
        self.app
            .invalidate(INSTANCE_DETAILS, Some(update.instance_id.0.into()));

        if need_reinstall {
            let request = modpack::session::Request {
                producer: modpack::session::Producer::Settings,
                kind: modpack::session::Kind::VersionOnly,
                files: Vec::new(),
                options: None,
                // Preparation retires the superseded setup under this same gate.
                reset: supersede,
                replaces: None,
                warnings: Vec::new(),
            };
            let (_handle, task_id) = self
                .prepare_game_admitted(
                    update.instance_id,
                    None,
                    None,
                    true,
                    _instance_guard,
                    runtime_activity.take().unwrap(),
                    Some(request),
                )
                .await?;
            return Ok(Some(task_id));
        }

        Ok(None)
    }

    /// Publish only the pack/version fields owned by preparation, merging
    /// against the current gated config. Preserve a user's detach and lock edit.
    pub(super) async fn publish_modpack_config(
        self,
        instance_id: InstanceId,
        prepared: info::Instance,
        original_modpack: Option<info::ModpackInfo>,
    ) -> anyhow::Result<()> {
        let operation = self.lock_instance(instance_id).await;
        self.publish_modpack_config_owned(&operation, prepared, original_modpack)
            .await
    }

    async fn publish_modpack_config_owned(
        self,
        operation: &InstanceOperationGuard,
        prepared: info::Instance,
        original_modpack: Option<info::ModpackInfo>,
    ) -> anyhow::Result<()> {
        let instance_id = operation.instance_id();
        let (shortpath, mut config) = self.current_config(instance_id).await?;
        let current_identity = config.modpack.as_ref().map(|pack| &pack.modpack);
        let original_identity = original_modpack.as_ref().map(|pack| &pack.modpack);
        if current_identity == original_identity {
            let locked = config
                .modpack
                .as_ref()
                .map(|pack| pack.locked)
                .unwrap_or(true);
            config.modpack = prepared.modpack.map(|mut pack| {
                pack.locked = locked;
                pack
            });
        }
        config.game_configuration.version = prepared.game_configuration.version;
        self.persist_config(instance_id, &shortpath, config).await
    }

    /// Requires lock_instance for this ID. Reject the recovering deletion state
    /// as well as an absent identity before accessing its filesystem root.
    async fn current_config(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<(String, info::Instance)> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;
        let data = instance.data()?;
        if matches!(data.state, LaunchState::Deleting) {
            bail!("cannot update an instance that is being deleted");
        }
        Ok((instance.shortpath.clone(), data.config.clone()))
    }

    /// Requires lock_instance for this ID through both persistence and publish.
    async fn persist_config(
        self,
        instance_id: InstanceId,
        shortpath: &str,
        config: info::Instance,
    ) -> anyhow::Result<()> {
        self.write_config(shortpath, &config).await?;
        self.publish_config(instance_id, config).await
    }

    /// Writes `config` to the instance.json of the folder `shortpath`.
    async fn write_config(self, shortpath: &str, config: &info::Instance) -> anyhow::Result<()> {
        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path()
            .join(shortpath);
        let json = schema::make_instance_config(config.clone())?;
        self.app
            .settings_manager()
            .runtime_path
            .get_temp()
            .write_file_atomic(path.join("instance.json"), json)
            .await?;
        self.instances
            .write()
            .await
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?
            .data_mut()?
            .config = config;
        Ok(())
    }

    pub async fn update_playtime(
        self,
        instance_id: InstanceId,
        added_seconds: u32,
    ) -> anyhow::Result<()> {
        let _operation = self.lock_instance(instance_id).await;
        let (shortpath, mut config) = self.current_config(instance_id).await?;
        // Seconds an earlier write could not save ride along with this one.
        let seconds = self
            .unsaved_playtime
            .remove(&instance_id)
            .map_or(0, |(_, unsaved)| unsaved)
            .saturating_add(added_seconds);
        config.last_played = Some(Utc::now());
        config.seconds_played = config.seconds_played.saturating_add(seconds);
        if let Err(error) = self.write_config(&shortpath, &config).await {
            // The library keeps showing what instance.json holds; the seconds
            // wait for the next playtime write (a minute later while the game
            // runs, or the next session's) instead of being lost. 2.0.40
            // carried them the same way; a restart before then loses them.
            let mut unsaved = self.unsaved_playtime.entry(instance_id).or_default();
            *unsaved = unsaved.saturating_add(seconds);
            return Err(error);
        }
        self.publish_config(instance_id, config).await?;
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn delete_instance(&self, instance_id: InstanceId) -> anyhow::Result<()> {
        let app = self.app.clone();

        // Run the actual deletion in a spawned task so the rspc call returns
        // promptly, but log failures: a dropped one leaves the instance to
        // re-appear on the next list.
        deletion::spawn_deletion(
            &self.app.runtime_lifecycle,
            format!("delete instance {instance_id}"),
            async move {
                if let Err(e) = app.instance_manager()._delete_instance(instance_id).await {
                    tracing::error!("Failed to delete instance {}: {:#}", instance_id.0, e);

                    // The instance was put into `LaunchState::Deleting` before the
                    // deletion failed. Without restoring it, the frontend stays
                    // stuck on the deleting/loading state forever. Reset it back to
                    // Inactive so the UI recovers.
                    let instance_name = {
                        let instance_manager = app.instance_manager();
                        let mut instances = instance_manager.instances.write().await;
                        instances.get_mut(&instance_id).and_then(|instance| {
                            if let InstanceType::Valid(data) = &mut instance.type_ {
                                // Only the mid-delete failure path leaves the instance in Deleting.
                                // A refusal to delete a running/preparing instance returns Err while
                                // that live state is still set, so it must not be clobbered here (that
                                // would drop the kill channel and orphan the running game).
                                if matches!(data.state, LaunchState::Deleting) {
                                    data.state = LaunchState::Inactive { failed_task: None };
                                }
                                Some(data.config.name.clone())
                            } else {
                                None
                            }
                        })
                    };

                    // Surface the error to the UI so the caller realizes the
                    // instance still exists.
                    app.invalidate(GET_GROUPS, None);
                    app.invalidate(GET_ALL_INSTANCES, None);
                    app.invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

                    // Push a dedicated failure event so the frontend can show a
                    // toast (the rspc call already returned Ok before the spawned
                    // deletion ran, so the caller's onError never fires).
                    app.invalidate(
                        DELETE_INSTANCE_FAILED,
                        Some(serde_json::json!({
                            "instanceId": instance_id.0,
                            "instanceName": instance_name,
                            "error": format!("{e:#}"),
                        })),
                    );
                }
            },
        )?;

        Ok(())
    }

    async fn _delete_instance(self, instance_id: InstanceId) -> anyhow::Result<()> {
        let operation = self.lock_instance(instance_id).await;

        let mut instances = self.instances.write().await;
        let Some(instance) = instances.get_mut(&instance_id) else {
            // Already gone — e.g. a concurrent delete (double-clicked confirm)
            // finished first. Deleting an absent instance is a no-op success,
            // not a failure to surface to the user.
            return Ok(());
        };

        let instance_shortpath = instance.shortpath.clone();
        // A folder whose instance.json is missing, unreadable or broken shows
        // as an invalid tile. It has no launch state to check, so deleting it
        // only removes the folder, if still there, and its library entry.
        let valid = match &mut instance.type_ {
            InstanceType::Valid(data) => {
                // A running/preparing instance must be stopped first (tearing it down
                // would kill the game and remove its directory mid-session). A second,
                // redundant delete (double-clicked confirm) is a no-op — see
                // `DeletePrecheck` — so it never resets the state the in-progress delete
                // owns nor reports a spurious failure.
                match delete_precheck(&data.state) {
                    DeletePrecheck::Proceed => {}
                    DeletePrecheck::AlreadyInProgress => return Ok(()),
                    DeletePrecheck::InUse => {
                        return Err(anyhow!(
                            "Instance {instance_id} cannot be deleted while it is preparing or running; stop it first"
                        ));
                    }
                }

                data.state = LaunchState::Deleting;
                true
            }
            InstanceType::Invalid(_) => false,
        };
        drop(instances);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        // Waits, showing Deleting, for duplicates still copying the folder.
        let _root = self.hold_root(&operation, true).await;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path()
            .join(&instance_shortpath as &str);

        #[cfg(test)]
        self.test_hooks.pause("delete", instance_id).await;

        let should_go_to_trash = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .deletion_through_recycle_bin;

        // The removal runs without path_lock, so a slow Recycle Bin or a
        // shell prompt never holds up creating, importing, renaming or
        // duplicating other instances. Until the retirement below, this
        // instance's library row keeps its folder name reserved.
        tokio::task::spawn_blocking(move || {
            if !valid
                && std::fs::symlink_metadata(&path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                return Ok(());
            }
            if should_go_to_trash {
                trash::delete(&path)?;
            } else {
                std::fs::remove_dir_all(&path)?;
            }

            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // Get the instance's group_id before deleting, so we can check if the group becomes empty
        let id = *instance_id;
        let group_id = instance_repo::get_instance(&self.app.db, id)
            .await?
            .map(|inst| GroupId(inst.group_id));

        {
            // The row and the loaded entry go together, so whoever holds
            // path_lock sees the folder name either reserved or free.
            let _path_guard = self.path_lock.lock().await;
            self.remove_instance(instance_id).await?;
            self.instances.write().await.remove(&instance_id);
            // Retired before the logs, so a Logs tab still loading this
            // instance's past logs adds none back (see `get_logs`).
            self.log_history.remove(&instance_id);
            self.game_logs
                .write()
                .await
                .retain(|_, (id, _, _)| *id != instance_id);
            self.unsaved_playtime.remove(&instance_id);
        }
        self.app.meta_cache_manager().gc_mod_metadata().await;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        // Auto-delete empty non-default groups after deleting the last instance
        if let Some(group_id) = group_id {
            let default_group_id = self.get_default_group().await?;

            if group_id != default_group_id {
                let gid = *group_id;
                let remaining_count =
                    instance_repo::count_instances_in_group(&self.app.db, gid).await?;

                if remaining_count == 0 {
                    instance_repo::delete_group(&self.app.db, gid).await?;
                    self.app.invalidate(GET_GROUPS, None);
                }
            }
        }

        Ok(())
    }

    /// # Locks
    /// - [InstanceManager::instances] (w) — only briefly at start (read) and
    ///   end (write); not held across the multi-second filesystem copy.
    /// - the original's gate — only for the snapshot; its folder hold
    ///   ([`hold_root`](InstanceManager::hold_root)) lasts until publication.
    pub async fn duplicate_instance(
        self,
        instance_id: InstanceId,
        name: String,
    ) -> anyhow::Result<InstanceId> {
        let _runtime_activity = self
            .app
            .runtime_lifecycle
            .admit(format!("duplicate instance {instance_id}"))?;
        let operation = self.lock_instance(instance_id).await;
        // The folder stays where it is until the copy is published: deleting
        // the original, or a rename that moves its folder, waits for it.
        let _root = self.hold_root(&operation, false).await;
        // First snapshot what we need from `instances` under a read lock and
        // release it before the long fs copy. Holding `instances` across the
        // copy serializes every other op (prepare_game, list_mods, etc.) for
        // the entire duration of duplication.
        let (mut new_info, src_shortpath) = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;
            let data = instance.data()?;
            if matches!(
                data.state,
                LaunchState::Queued(_) | LaunchState::Preparing(_)
            ) {
                bail!("wait for active setup before duplicating the instance");
            }

            if matches!(data.state, LaunchState::Deleting) {
                bail!("cannot duplicate an instance that is being deleted");
            }
            (data.config.clone(), instance.shortpath.clone())
        };
        // The copy takes what the folder holds as it goes, as in 2.0.40, and
        // never holds up Play, settings, logs or the game's exit on the
        // original: only the snapshot above needs its gate.
        drop(operation);
        #[cfg(test)]
        self.test_hooks.pause("duplicate_copy", instance_id).await;

        let id = *instance_id;
        let group_id = instance_repo::get_instance(&self.app.db, id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "instance was not listed in db while being present in internal list"
                )
            })?
            .group_id;

        let _path_guard = self.path_lock.lock().await;
        let (new_shortpath, new_path) = self.next_available_folder(&src_shortpath).await?;
        new_info.name = name;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&src_shortpath)
            .get_root();

        let tmpdir = self
            .app
            .settings_manager()
            .runtime_path
            .get_temp()
            .maketmpdir()
            .await?;

        let path2 = path.clone();
        let tmpdir2 = tmpdir.to_path_buf();
        let tmppath = tmpdir.join(
            path.file_name()
                .expect("instance path cannot end in .. or be empty"),
        );
        tokio::task::spawn_blocking(move || {
            fs_extra::dir::copy(path2, tmpdir2, &CopyOptions::new())
        })
        .await??;

        let json = schema::make_instance_config(new_info.clone())?;

        let icon_revision = match &new_info.icon {
            InstanceIcon::Default => None,
            InstanceIcon::RelativePath(_) => Some(1),
        };

        // Write the new config inside the copied directory so the rename below carries it.
        // Writing to `tmpdir` would place it beside the directory, where it is discarded,
        // leaving the duplicate with the source's instance.json (its old name and metadata).
        tokio::fs::write(&tmppath.join("instance.json"), json).await?;

        // `fs_extra::dir::copy` above copied the whole source root, pidfile
        // included if the source instance happened to be running — a game's
        // pid (and its verified start time) belongs to that one process,
        // never to a duplicate that has not launched anything. Left in
        // place, a future startup reconcile pass could adopt the duplicate
        // as if it were running the source's game, or Stop on the duplicate
        // could signal the source's real, unrelated game process.
        orphan_pid::remove_pid_file_strict(&tmppath, PID_FILE_NAME).await?;
        tokio::fs::rename(&tmppath, &new_path).await?;

        let id = self
            .add_instance(
                new_info.name.clone(),
                new_shortpath.clone(),
                GroupId(group_id),
            )
            .await?;

        if let Some(mut session) = modpack::session::Session::load(&new_path).await? {
            // This is provenance of the copy, not authority to run it. The
            // source owner remains until original-input qualification finishes.
            session.copied_to = Some(id.0);
            modpack::session::atomic_json(
                &new_path.join(".setup").join(modpack::session::RECORD),
                &session,
                false,
            )
            .await?;
        }

        // Only now reacquire the write lock to insert the new entry.
        let mut instances = self.instances.write().await;
        instances.insert(
            id,
            Instance {
                shortpath: new_shortpath,
                type_: InstanceType::Valid(InstanceData {
                    favorite: false,
                    config: new_info,
                    state: run::LaunchState::Inactive { failed_task: None },
                    modpack_update_curseforge: None,
                    modpack_update_modrinth: None,
                    icon_revision,
                }),
            },
        );

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .meta_cache_manager()
            .queue_caching(
                crate::managers::metadata::cache::CacheEntityId::Instance(id),
                false,
            )
            .await;

        Ok(id)
    }

    pub async fn open_folder(
        self,
        instance_id: InstanceId,
        folder: InstanceFolder,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        let path = match folder {
            InstanceFolder::Root => path.get_root().to_path_buf(),
            InstanceFolder::Data => path.get_data_path().to_path_buf(),
            InstanceFolder::Mods => path.get_mods_path().to_path_buf(),
            InstanceFolder::Configs => path.get_config_path().to_path_buf(),
            InstanceFolder::Screenshots => path.get_screenshots_path().to_path_buf(),
            InstanceFolder::Saves => path.get_saves_path().to_path_buf(),
            InstanceFolder::Logs => path.get_logs_path().to_path_buf(),
            InstanceFolder::CrashReports => path.get_crash_reports_path().to_path_buf(),
            InstanceFolder::ResourcePacks => path.get_resourcepacks_path().to_path_buf(),
            InstanceFolder::TexturePacks => path.get_texturepacks_path().to_path_buf(),
            InstanceFolder::ShaderPacks => path.get_shaderpacks_path().to_path_buf(),
        };

        if !path.is_file() && !path.is_dir() {
            tokio::fs::create_dir_all(&path).await.with_context(|| {
                format!("Creating instance folder at `{}`", path.to_string_lossy())
            })?;
        } else if path.is_file() && !path.is_dir() {
            bail!("Path is a file, not a directory");
        }

        opener::open(path)?;

        Ok(())
    }

    /// Delete an instance group and move all contained instances into the default group.
    pub async fn delete_group(self, group: GroupId) -> anyhow::Result<()> {
        // lock indexes before checking for instances to make sure none can be moved or created.
        let _index_lock = self.index_lock.lock().await;

        let group_id = *group;
        let any_instances =
            instance_repo::count_instances_in_group(&self.app.db, group_id).await? != 0;

        // a default group will be created if get_default_group is called, so
        // we check if any instances exist before creating it to avoid making an
        // empty group every time a group is deleted.
        if any_instances {
            let default_group = self.get_default_group().await?;

            let default_id = *default_group;
            instance_repo::delete_group_tx(&self.app.db, group_id, default_id).await?;
        } else {
            instance_repo::delete_group(&self.app.db, group_id).await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        Ok(())
    }

    /// Delete an instance group and all instances it contains.
    pub async fn delete_group_with_instances(self, group: GroupId) -> anyhow::Result<()> {
        let app = self.app.clone();
        deletion::spawn_deletion(
            &self.app.runtime_lifecycle,
            format!("delete instance group {group}"),
            async move {
                app.instance_manager()
                    ._delete_group_with_instances(group)
                    .await
            },
        )?
        .await?
    }

    async fn _delete_group_with_instances(self, group: GroupId) -> anyhow::Result<()> {
        // Get all instances in the group
        let group_id = *group;
        let instances_in_group =
            instance_repo::get_instances_by_group(&self.app.db, group_id).await?;

        // Refuse the whole operation if any contained instance is preparing or running, before
        // deleting anything. Otherwise the per-instance delete below returns an error for the
        // running instance while the group row is removed
        // anyway, orphaning that instance: it keeps running with a dangling group id and vanishes
        // from list_groups (SQLite foreign keys are not enforced on this connection, so the group
        // row deletes regardless of the referencing row). Mirror the single-instance guard.
        {
            let instances = self.instances.read().await;
            for instance in &instances_in_group {
                let instance_id = InstanceId(instance.id);
                if let Some(InstanceType::Valid(data)) =
                    instances.get(&instance_id).map(|i| &i.type_)
                {
                    if !matches!(data.state, LaunchState::Inactive { .. }) {
                        bail!(
                            "Instance group cannot be deleted while instance {instance_id} is preparing or running; stop it first"
                        );
                    }
                }
            }
        }

        // Every instance is inactive; delete each one, awaiting so a failure aborts before the
        // group row is removed rather than being spawned and ignored.
        for instance in instances_in_group {
            let instance_id = InstanceId(instance.id);
            if let Err(e) = self
                .app
                .instance_manager()
                ._delete_instance(instance_id)
                .await
            {
                // _delete_instance set the instance to Deleting before it failed. Unlike the
                // single-instance path (delete_instance), nothing here restores it, so reset it
                // to Inactive — otherwise the UI leaves it stuck on the deleting spinner and it
                // can be neither deleted nor played until the app restarts.
                {
                    let mut instances = self.instances.write().await;
                    if let Some(instance) = instances.get_mut(&instance_id) {
                        if let InstanceType::Valid(data) = &mut instance.type_ {
                            if matches!(data.state, LaunchState::Deleting) {
                                data.state = LaunchState::Inactive { failed_task: None };
                            }
                        }
                    }
                }
                self.app.invalidate(GET_GROUPS, None);
                self.app.invalidate(GET_ALL_INSTANCES, None);
                return Err(e);
            }
        }

        // Delete the group record. _delete_instance auto-removes an emptied non-default group
        // after deleting its last instance, so the row may already be gone here; a plain DELETE
        // is idempotent (no error on zero rows), so a group already removed on the happy path
        // does not surface a spurious error toast on a successful deletion.
        instance_repo::delete_group(&self.app.db, group_id).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        Ok(())
    }

    /// Best-effort check of whether a user-set Java override resolves to a major
    /// version incompatible with `required_profile`, using only already-scanned
    /// Javas (no probing at details-fetch time). Returns false when there is no
    /// override, no requirement, or the override's Java can't be identified
    /// cheaply — a Path override to an unscanned binary is left for the
    /// launch-time warning rather than probed here.
    async fn java_override_mismatch(
        &self,
        java_override: &Option<info::JavaOverride>,
        required_profile: Option<SystemJavaProfileName>,
    ) -> bool {
        let (Some(required), Some(java_override)) = (required_profile, java_override) else {
            return false;
        };

        let resolved_version = match java_override {
            info::JavaOverride::Path(Some(path)) => {
                let Ok(all) = self.app.java_manager().get_available_javas().await else {
                    return false;
                };
                all.values()
                    .flatten()
                    .find(|java| java.component.path == *path)
                    .map(|java| java.component.version.clone())
            }
            info::JavaOverride::Profile(Some(name)) => {
                let java_manager = self.app.java_manager();
                let (Ok(profiles), Ok(all)) = (
                    java_manager.get_java_profiles().await,
                    java_manager.get_available_javas().await,
                ) else {
                    return false;
                };
                profiles
                    .iter()
                    .find(|profile| profile.name == *name)
                    .and_then(|profile| profile.java_id.as_ref())
                    .and_then(|java_id| all.values().flatten().find(|java| java.id == *java_id))
                    .map(|java| java.component.version.clone())
            }
            _ => None,
        };

        resolved_version
            .map(|version| !required.is_java_version_compatible(&version))
            .unwrap_or(false)
    }

    pub async fn instance_details(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<domain::InstanceDetails> {
        let mut details = {
            let instances = self.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?;

            let instance = match &instance.type_ {
                InstanceType::Invalid(_) => bail!(InvalidInstanceDataError),
                InstanceType::Valid(x) => x,
            };

            let icon_revision = match &instance.config.icon {
                InstanceIcon::Default => None,
                InstanceIcon::RelativePath(_) => instance.icon_revision,
            };

            let mc_version = match &instance.config.game_configuration.version {
                Some(info::GameVersion::Standard(version)) => Some(version.release.clone()),
                Some(info::GameVersion::Custom(custom)) => Some(custom.clone()),
                None => None,
            };

            domain::InstanceDetails {
                id: instance_id,
                favorite: instance.favorite,
                name: instance.config.name.clone(),
                version: mc_version,
                modpack: instance.config.modpack.clone(),
                global_java_args: instance.config.game_configuration.global_java_args,
                extra_java_args: instance.config.game_configuration.extra_java_args.clone(),
                memory: instance.config.game_configuration.memory,
                game_resolution: instance.config.game_configuration.game_resolution.clone(),
                last_played: instance.config.last_played,
                seconds_played: instance.config.seconds_played as u32,
                modloaders: match &instance.config.game_configuration.version {
                    Some(info::GameVersion::Standard(version)) => {
                        version.modloaders.iter().map(Clone::clone).collect()
                    }
                    Some(info::GameVersion::Custom(_)) => Vec::new(), // todo
                    None => Vec::new(),
                },
                java_override: instance.config.game_configuration.java_override.clone(),
                required_java_profile: None,
                java_override_mismatch: false,
                state: (&instance.state).into(),
                notes: instance.config.notes.clone(),
                icon_revision,
                has_pack_update: instance.modpack_update_curseforge.unwrap_or(false)
                    || instance.modpack_update_modrinth.unwrap_or(false),
                pre_launch_hook: instance.config.pre_launch_hook.clone(),
                post_exit_hook: instance.config.post_exit_hook.clone(),
                wrapper_command: instance.config.wrapper_command.clone(),
            }
        };
        #[cfg(test)]
        self.test_hooks.pause("details", instance_id).await;

        let mut mc_manifest = None;

        if details.version.is_some() {
            let manifest = self.app.minecraft_manager().get_minecraft_manifest().await;
            mc_manifest = manifest.ok();
        }

        let required_java_profile_name = details.version.clone().and_then(|version| {
            let manifest = mc_manifest.as_ref()?;
            let required_java = manifest
                .versions
                .iter()
                .find(|profile| profile.id == version)
                .and_then(|version| version.java_profile.clone())?;

            SystemJavaProfileName::try_from(required_java).ok()
        });
        let required_java_profile = required_java_profile_name.as_ref().map(|v| v.to_string());

        details.required_java_profile = required_java_profile;
        details.java_override_mismatch = self
            .java_override_mismatch(&details.java_override, required_java_profile_name)
            .await;
        Ok(details)
    }

    pub async fn get_modpack_info(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<InstanceModpackInfo>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let instance = match &instance.type_ {
            InstanceType::Invalid(_) => bail!(InvalidInstanceDataError),
            InstanceType::Valid(x) => x,
        };

        let modpack = match &instance.config.modpack {
            Some(modpack) => modpack.clone(),
            None => {
                return Ok(None);
            }
        };

        drop(instances);

        let _guard = self.modpack_info_semaphore.lock().await;

        let modpack_info = match modpack.modpack {
            info::Modpack::Curseforge(curseforge) => {
                cache::curseforge::modpack::get_modpack_metadata(&self.app, curseforge).await?
            }
            info::Modpack::Modrinth(modrinth) => {
                cache::modrinth::modpack::get_modpack_metadata(&self.app, modrinth).await?
            }
        };

        Ok(Some(modpack_info))
    }

    pub async fn get_modpack_icon(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let instance = match &instance.type_ {
            InstanceType::Invalid(_) => bail!(InvalidInstanceDataError),
            InstanceType::Valid(x) => x,
        };

        let modpack = match &instance.config.modpack {
            Some(modpack) => modpack.clone(),
            None => {
                return Ok(None);
            }
        };

        drop(instances);

        let _guard = self.modpack_info_semaphore.lock().await;

        let modpack_info = match modpack.modpack {
            info::Modpack::Curseforge(curseforge) => {
                cache::curseforge::modpack::get_modpack_icon(&self.app, curseforge).await?
            }
            info::Modpack::Modrinth(modrinth) => {
                cache::modrinth::modpack::get_modpack_icon(&self.app, modrinth).await?
            }
        };

        Ok(Some(modpack_info))
    }

    pub async fn instance_icon(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<(String, Vec<u8>)>> {
        let instances = self.instances.read().await;

        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            return Ok(None);
        };

        match &data.config.icon {
            InstanceIcon::Default => Ok(None),
            InstanceIcon::RelativePath(icon_path) => {
                let path = self
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_instances()
                    .to_path()
                    .join(&instance.shortpath)
                    .join(icon_path);
                let icon = tokio::fs::read(path).await?;

                Ok(Some((icon_path.clone(), icon)))
            }
        }
    }

    async fn next_group_index(self) -> anyhow::Result<IdLock<'s, i32>> {
        let guard = self.manager.index_lock.lock().await;

        let count = instance_repo::count_groups(&self.app.db).await?;

        Ok(IdLock {
            value: count as i32,
            guard,
        })
    }

    async fn next_instance_index(self, group: GroupId) -> anyhow::Result<IdLock<'s, i32>> {
        let guard = self.manager.index_lock.lock().await;

        // Newly created instances appear at the TOP of their group, so we
        // pick an index strictly smaller than the current minimum. Sort is
        // ascending on `index`, so a smaller value sorts first.
        let group_id = *group;
        let min_index: Option<i32> =
            instance_repo::min_index_instance_in_group(&self.app.db, group_id)
                .await?
                .map(|i| i.index);

        Ok(IdLock {
            value: min_index.map(|n| n - 1).unwrap_or(0),
            guard,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListGroup {
    pub id: GroupId,
    pub name: String,
    pub library_position: Option<i32>,
    pub instances: Vec<ListInstance>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListInstance {
    pub id: InstanceId,
    pub group_id: GroupId,
    pub index: i32,
    pub library_position: Option<i32>,
    pub name: String,
    pub favorite: bool,
    pub status: ListInstanceStatus,
    pub icon_revision: Option<u32>,
    pub last_played: Option<DateTime<Utc>>,
    pub locked: bool,
    pub date_created: DateTime<Utc>,
    pub date_updated: DateTime<Utc>,
    pub seconds_played: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ListInstanceStatus {
    Valid(ValidListInstance),
    Invalid(InvalidListInstance),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValidListInstance {
    pub mc_version: Option<String>,
    pub modloader: Option<info::ModLoaderType>,
    pub modloader_version: Option<String>,
    pub modpack: Option<Modpack>,
    pub state: domain::LaunchState,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidListInstance {
    JsonMissing,
    JsonError(ConfigurationParseError),
    Other(String),
}

/// Lock used to prevent race conditions when modifying group or instance indexes
struct IdLock<'a, V: Copy + Clone> {
    value: V,
    guard: MutexGuard<'a, ()>,
}

impl Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for GroupId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for InstanceId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum InstanceMoveTarget {
    Before(InstanceId),
    BeginningOfGroup(GroupId),
    EndOfGroup(GroupId),
    BeforeGroup(GroupId), // Position instance before a folder (at library root level)
}

pub enum GroupMoveTarget {
    BeforeGroup(GroupId),
    BeforeInstance(InstanceId), // Instance must be in default group (ungrouped)
    EndOfLibrary,
}

#[derive(Debug, Clone, Copy)]
pub enum LibrarySortCriteria {
    Name,
    LastPlayed,
    MostPlayed,
    DateCreated,
}

#[derive(Debug)]
pub struct Instance {
    pub shortpath: String,
    pub type_: InstanceType,
}

#[derive(Debug)]
pub enum InstanceType {
    Valid(InstanceData),
    Invalid(InvalidConfiguration),
}

impl InstanceType {
    pub fn data(&self) -> Result<&InstanceData, InvalidInstanceDataError> {
        match self {
            Self::Valid(data) => Ok(data),
            Self::Invalid(_) => Err(InvalidInstanceDataError),
        }
    }

    pub fn data_mut(&mut self) -> Result<&mut InstanceData, InvalidInstanceDataError> {
        match self {
            Self::Valid(data) => Ok(data),
            Self::Invalid(_) => Err(InvalidInstanceDataError),
        }
    }
}

impl Instance {
    pub fn data(&self) -> Result<&InstanceData, InvalidInstanceDataError> {
        self.type_.data()
    }

    pub fn data_mut(&mut self) -> Result<&mut InstanceData, InvalidInstanceDataError> {
        self.type_.data_mut()
    }
}

#[derive(Debug)]
pub enum InvalidConfiguration {
    NoFile,
    Invalid(ConfigurationParseError),
    IoError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationParseError {
    pub type_: ConfigurationParseErrorType,
    pub message: String,
    pub line: u32,
    pub config_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Type, Serialize)]
pub enum ConfigurationParseErrorType {
    Syntax,
    Data,
    Eof,
    Unknown,
}

#[derive(Debug)]
pub enum Late<T> {
    Loading,
    Ready(T),
}

#[derive(Debug)]
pub struct InstanceData {
    favorite: bool,
    config: info::Instance,
    state: run::LaunchState,
    modpack_update_curseforge: Option<bool>,
    modpack_update_modrinth: Option<bool>,
    icon_revision: Option<u32>,
}

impl InstanceData {
    /// The game version this instance is configured to launch, if it has one.
    pub fn game_version(&self) -> Option<&GameVersion> {
        self.config.game_configuration.version.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Mod {
    id: String,
    filename: OsString,
    enabled: bool,
    modloaders: Vec<domain::info::ModLoaderType>,
    metadata: domain::ModFileMetadata,
}

#[derive(Debug)]
pub enum InstanceVersionSource {
    Version(info::GameVersion),
    /// (version, modpack, locked)
    Modpack(info::Modpack, bool),
    /// (version, modpack, locked)
    ModpackWithKnownVersion(info::GameVersion, info::Modpack, bool),
}

#[derive(Error, Debug)]
#[error("attempted to use invalid InstanceId {0}")]
pub struct InvalidInstanceIdError(InstanceId);

#[derive(Error, Debug)]
#[error("attempted to use invalid GroupId {0}")]
pub struct InvalidGroupIdError(GroupId);

#[derive(Error, Debug)]
#[error("attempted to get data of an invalid instance")]
pub struct InvalidInstanceDataError;

/// A game process found still running when the core started: one this core
/// did not spawn, belonging to an instance that must therefore be presented
/// as running without a handle to the process behind it.
#[derive(Debug, Clone, Copy)]
struct AdoptedGame {
    pid: u32,
    /// Taken from the pidfile's mtime, which is written immediately after the
    /// spawn and so approximates the launch time to within a second — near
    /// enough to keep playtime accounting roughly right across a restart
    /// instead of resetting it.
    start_time: DateTime<Utc>,
    /// The sysinfo start time (seconds since epoch) `reconcile_pid` already
    /// verified `pid` against, carried forward rather than re-derived. Every
    /// later re-check of this pid (the adopted-instance poller,
    /// `kill_instance`) verifies against this exact value — not a bare pid
    /// lookup — since the game can exit and have its pid recycled by an
    /// unrelated process at any point after this reconcile pass.
    verified_start_time: u64,
    verified_process: orphan_pid::ProcessRecord,
}

/// Reconcile every instance directory directly under `instances_root`
/// against the live process table, returning the games that are still
/// running keyed by shortpath. Free function (not tied to
/// `ManagerRef`/`InstanceManager`) so it is unit-testable against a plain
/// temp directory laid out like the real instances root, without needing a
/// full `App`.
///
/// Deliberately the opposite of `ServerManager::clean_up_orphaned_servers`,
/// which kills what it finds: a pid sysinfo confirms is still alive AND
/// still looks like java is the user's own game, left running by a session
/// that ended without it — most often because they closed the launcher. It
/// is adopted rather than killed, and its pidfile is kept, since that file
/// stays the only record of the process for as long as it lives. Anything
/// else (dead pid, or a live pid the process table says is no longer java,
/// meaning the number was reused) just has its stale pidfile removed.
async fn reconcile_running_instances_under(instances_root: &Path) -> HashMap<String, AdoptedGame> {
    reconcile_running_instances_under_observed(instances_root, orphan_pid::observe_process).await
}

async fn reconcile_running_instances_under_observed(
    instances_root: &Path,
    observe: impl Fn(u32) -> orphan_pid::ProcessObservation,
) -> HashMap<String, AdoptedGame> {
    let mut adopted: HashMap<String, AdoptedGame> = HashMap::new();

    let mut entries = match tokio::fs::read_dir(instances_root).await {
        Ok(entries) => entries,
        Err(e) => {
            warn!(
                "Failed to read instances directory {} to reconcile running games: {}",
                instances_root.display(),
                e
            );
            return adopted;
        }
    };

    // Pass 1: read every instance's pidfile (best-effort, one small file
    // read each) and collect the recorded pids up front, so the process
    // table only needs a single targeted refresh for this whole pass
    // instead of a full system scan per instance.
    let mut recorded: Vec<(String, PathBuf, Option<orphan_pid::ProcessRecord>)> = Vec::new();
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(e) => {
                warn!(
                    "Failed to read an instances directory entry while reconciling running games: {}",
                    e
                );
                break;
            }
        };

        // Each entry here is exactly `instances_root.join(shortpath)`, the
        // same root `InstancePath::get_root()` computes for this instance.
        let root = entry.path();
        let Some(shortpath) = root.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let pid = match orphan_pid::read_process_file(&root, PID_FILE_NAME).await {
            Ok(pid) => pid,
            Err(e) => {
                warn!(
                    "Failed to read pidfile for instance {} at {}: {} — retaining recovery evidence",
                    shortpath,
                    root.display(),
                    e
                );
                // A pidfile that fails to parse can never be reconciled —
                // there is no recorded pid to check against the process
                // table — so it will only fail the same way again on every
                // future startup unless it is cleaned up here, exactly like
                // a stale-but-parseable one is by `RemoveStale`/`NotOurs`
                // below.
                // Malformed present identity remains recoverable; never downgrade.
                None
            }
        };
        recorded.push((shortpath.to_string(), root, pid));
    }

    for (shortpath, root, record) in recorded {
        let Some(record) = record else {
            continue;
        };
        let observed = observe(record.pid);
        match orphan_pid::identity_status(record, observed, true) {
            orphan_pid::IdentityStatus::GoneOrReused => {
                orphan_pid::remove_process_file_if_matches(&root, PID_FILE_NAME, record).await;
            }
            orphan_pid::IdentityStatus::Unknown => {
                warn!("Instance {shortpath} process ownership unconfirmed; marker retained");
            }
            orphan_pid::IdentityStatus::Same => {
                let verified_process = orphan_pid::verified_record(record, observed, true)
                    .expect("Same verified process");
                // Persist a conservative legacy upgrade before carrying it into
                // polling/lifecycle, so later restarts use the same stable token.
                if verified_process != record {
                    orphan_pid::write_process_file(&root, PID_FILE_NAME, verified_process).await;
                }
                let start_time =
                    tokio::fs::metadata(orphan_pid::pid_file_path(&root, PID_FILE_NAME))
                        .await
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(DateTime::<Utc>::from)
                        .unwrap_or_else(Utc::now);
                adopted.insert(
                    shortpath,
                    AdoptedGame {
                        pid: record.pid,
                        start_time,
                        verified_start_time: record.start_time.unwrap_or(0),
                        verified_process,
                    },
                );
            }
        }
    }

    adopted
}

/// Outcome of checking whether an instance may be deleted, given its launch state.
#[derive(Debug, PartialEq, Eq)]
enum DeletePrecheck {
    /// Inactive — safe to delete.
    Proceed,
    /// Already being deleted. A double-clicked confirm spawns two delete tasks;
    /// the second must be a no-op so it doesn't reset the `Deleting` state the
    /// first task owns (which would open a window for a launch of a
    /// being-deleted instance) or report a spurious failure.
    AlreadyInProgress,
    /// Queued/Preparing/Running — refuse; the instance must be stopped first,
    /// otherwise the game is killed and its directory removed mid-session.
    InUse,
}

fn delete_precheck(state: &LaunchState) -> DeletePrecheck {
    match state {
        LaunchState::Inactive { .. } => DeletePrecheck::Proceed,
        LaunchState::Deleting => DeletePrecheck::AlreadyInProgress,
        LaunchState::Queued(_) | LaunchState::Preparing(_) | LaunchState::Running(_) => {
            DeletePrecheck::InUse
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, time::Duration};

    use super::domain;
    use carbon_repos::db_exec::Db;
    use carbon_repos::repos::instance as instance_repo;
    use unicode_segmentation::UnicodeSegmentation;

    use crate::{
        domain::instance::{InstanceSettingsUpdate, info},
        managers::instance::{
            GroupId, GroupMoveTarget, InstanceId, InstanceMoveTarget, ListGroup, ListInstance,
            ListInstanceStatus, ValidListInstance,
        },
    };

    use super::InstanceVersionSource;

    #[test]
    fn delete_precheck_classifies_launch_states() {
        use super::{DeletePrecheck, LaunchState, delete_precheck};
        use crate::domain::vtask::VisualTaskId;

        assert_eq!(
            delete_precheck(&LaunchState::Inactive { failed_task: None }),
            DeletePrecheck::Proceed
        );
        // The fix: a delete already in progress is a no-op, not a refusal — so a
        // double-clicked confirm can't reset the state the first task owns.
        assert_eq!(
            delete_precheck(&LaunchState::Deleting),
            DeletePrecheck::AlreadyInProgress
        );
        assert_eq!(
            delete_precheck(&LaunchState::Queued(VisualTaskId(1))),
            DeletePrecheck::InUse
        );
        assert_eq!(
            delete_precheck(&LaunchState::Preparing(VisualTaskId(1))),
            DeletePrecheck::InUse
        );
    }

    #[tokio::test]
    async fn move_groups() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        async fn get_ordered_groups(db: &Db) -> anyhow::Result<Vec<GroupId>> {
            Ok(instance_repo::get_all_groups_ordered_by_group_index(db)
                .await?
                .into_iter()
                .map(|group| GroupId(group.id))
                .collect())
        }

        let mut groups = [
            app.instance_manager()
                .create_group(String::from("move0"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move1"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move2"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move3"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move4"))
                .await?,
        ];

        // move 1 to 1 (do nothing)
        app.instance_manager()
            .move_group(groups[1], GroupMoveTarget::BeforeGroup(groups[1]))
            .await?;
        assert_eq!(groups[..], get_ordered_groups(&app.db).await?[..]);

        // move 1 to 3 as if dragged
        app.instance_manager()
            .move_group(groups[1], GroupMoveTarget::BeforeGroup(groups[3]))
            .await?;
        groups = [groups[0], groups[2], groups[1], groups[3], groups[4]];
        assert_eq!(groups[..], get_ordered_groups(&app.db).await?[..]);

        // move 3 back to 1
        app.instance_manager()
            .move_group(groups[3], GroupMoveTarget::BeforeGroup(groups[1]))
            .await?;
        groups = [groups[0], groups[3], groups[1], groups[2], groups[4]];
        assert_eq!(groups[..], get_ordered_groups(&app.db).await?[..]);

        // move 1 to end of list
        app.instance_manager()
            .move_group(groups[1], GroupMoveTarget::EndOfLibrary)
            .await?;
        groups = [groups[0], groups[2], groups[3], groups[4], groups[1]];
        assert_eq!(groups[..], get_ordered_groups(&app.db).await?[..]);

        // move 4 to beginning of list
        app.instance_manager()
            .move_group(groups[4], GroupMoveTarget::BeforeGroup(groups[0]))
            .await?;
        groups = [groups[4], groups[0], groups[1], groups[2], groups[3]];
        assert_eq!(groups[..], get_ordered_groups(&app.db).await?[..]);

        Ok(())
    }

    #[tokio::test]
    async fn move_instances() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        async fn get_ordered_instances(db: &Db, group: GroupId) -> anyhow::Result<Vec<InstanceId>> {
            let group_id = *group;
            Ok(
                instance_repo::get_instances_by_group_ordered_by_index(db, group_id)
                    .await?
                    .into_iter()
                    .map(|instance| InstanceId(instance.id))
                    .collect(),
            )
        }

        let [group0, group1] = [
            app.instance_manager()
                .create_group(String::from("group0"))
                .await?,
            app.instance_manager()
                .create_group(String::from("group1"))
                .await?,
        ];

        let mk_instance = |shortpath: &'static str, group| {
            let app = &app;
            async move {
                let id = app
                    .instance_manager()
                    .add_instance(shortpath.to_string(), shortpath.to_string(), group)
                    .await?;

                Ok::<_, anyhow::Error>(id)
            }
        };

        let mut group0_instances = [
            mk_instance("g0i0", group0.clone()).await?,
            mk_instance("g0i1", group0.clone()).await?,
            mk_instance("g0i2", group0.clone()).await?,
        ];
        // New instances prepend within their group, so DB-ascending order
        // is the reverse of creation order.
        group0_instances.reverse();

        let mut group1_instances = [
            mk_instance("g1i0", group1.clone()).await?,
            mk_instance("g1i1", group1.clone()).await?,
        ];
        group1_instances.reverse();

        // move 1 to 1 (do nothing)
        app.instance_manager()
            .move_instance(
                group0_instances[1],
                InstanceMoveTarget::Before(group0_instances[1]),
            )
            .await?;

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        // move 1 to end of list
        app.instance_manager()
            .move_instance(group0_instances[1], InstanceMoveTarget::EndOfGroup(group0))
            .await?;

        group0_instances = [
            group0_instances[0],
            group0_instances[2],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        // move 0 to end of list
        app.instance_manager()
            .move_instance(group0_instances[0], InstanceMoveTarget::EndOfGroup(group0))
            .await?;

        group0_instances = [
            group0_instances[1],
            group0_instances[2],
            group0_instances[0],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        // move 2 back to 0
        app.instance_manager()
            .move_instance(
                group0_instances[2],
                InstanceMoveTarget::Before(group0_instances[0]),
            )
            .await?;

        group0_instances = [
            group0_instances[2],
            group0_instances[0],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        app.instance_manager()
            .move_instance(
                group0_instances[2],
                InstanceMoveTarget::BeginningOfGroup(group0),
            )
            .await?;

        group0_instances = [
            group0_instances[2],
            group0_instances[0],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        // move 0:1 to 1:1
        app.instance_manager()
            .move_instance(
                group0_instances[1],
                InstanceMoveTarget::Before(group1_instances[1]),
            )
            .await?;

        let group1_instances = [
            group1_instances[0],
            group0_instances[1],
            group1_instances[1],
        ];

        let group0_instances = [group0_instances[0], group0_instances[2]];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        assert_eq!(
            group1_instances[..],
            get_ordered_instances(&app.db, group1).await?[..],
        );

        // move 0:0 to end of group 1. group0 is then left with a single
        // instance, which auto-dissolves: the last instance returns to the
        // default group and the now-empty group0 is deleted.
        let surviving_instance = group0_instances[1];

        app.instance_manager()
            .move_instance(group0_instances[0], InstanceMoveTarget::EndOfGroup(group1))
            .await?;

        let group1_instances = [
            group1_instances[0],
            group1_instances[1],
            group1_instances[2],
            group0_instances[0],
        ];

        let group0_instances: [InstanceId; 0] = [];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.db, group0).await?[..],
        );

        assert_eq!(
            group1_instances[..],
            get_ordered_instances(&app.db, group1).await?[..],
        );

        // the dissolved group's last instance returns to the default group
        let default_group = app.instance_manager().get_default_group().await?;
        assert!(
            get_ordered_instances(&app.db, default_group)
                .await?
                .contains(&surviving_instance)
        );

        Ok(())
    }

    #[tokio::test]
    async fn delete_group() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let default_group = app.instance_manager().get_default_group().await?;
        let group = app
            .instance_manager()
            .create_group(String::from("foo"))
            .await?;
        app.instance_manager()
            .add_instance(String::from("baz"), String::from("baz"), default_group)
            .await?;
        app.instance_manager()
            .add_instance(String::from("bar"), String::from("bar"), group)
            .await?;

        let instance = instance_repo::get_instance_by_shortpath(&app.db, "bar")
            .await?
            .unwrap();

        assert_eq!(instance.index, 0);
        assert_eq!(instance.group_id, *group);

        app.instance_manager().delete_group(group).await?;

        let instance = instance_repo::get_instance_by_shortpath(&app.db, "bar")
            .await?
            .unwrap();

        // index should be `1` due to instance already present in default group.
        assert_eq!(instance.index, 1);
        assert_eq!(
            instance.group_id,
            *app.instance_manager().get_default_group().await?
        );

        Ok(())
    }

    #[tokio::test]
    async fn delete_group_empty() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let group_count = instance_repo::count_groups(&app.db).await?;

        // assert no default group exists
        assert_eq!(group_count, 0);

        let group = app
            .instance_manager()
            .create_group(String::from("foo"))
            .await?;

        let group_count = instance_repo::count_groups(&app.db).await?;

        // assert only the created group exists
        assert_eq!(group_count, 1);

        app.instance_manager().delete_group(group).await?;

        let group_count = instance_repo::count_groups(&app.db).await?;

        // assert the default group was not created while deleting the new group
        assert_eq!(group_count, 0);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "currently failing intermittently (probably due to restart_in_place)"]
    async fn instance_crud() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        // create
        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        let mut list = app.instance_manager().list_groups().await?;
        let mut expected = vec![ListGroup {
            id: default_group.id,
            name: default_group.name.clone(),
            library_position: None, // Default group has no library position
            instances: vec![ListInstance {
                id: instance_id,
                group_id: default_group.id,
                index: 0,
                library_position: list[0].instances[0].library_position, // Use actual value from DB
                name: String::from("test"),
                favorite: false,
                icon_revision: None,
                status: ListInstanceStatus::Valid(ValidListInstance {
                    mc_version: Some(String::from("1.7.10")),
                    modloader: None,
                    modloader_version: None,
                    modpack: None,
                    state: domain::LaunchState::Inactive { failed_task: None },
                }),
                locked: false,
                last_played: None,
                date_created: list[0].instances[0].date_created,
                date_updated: list[0].instances[0].date_updated,
                seconds_played: 0,
            }],
        }];

        assert_eq!(list, expected);

        // check that it was persisted
        app.restart_in_place().await;

        // wait for instance scan
        tokio::time::sleep(Duration::from_millis(100)).await;

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // update
        app.instance_manager()
            .update_instance(InstanceSettingsUpdate {
                instance_id,
                name: Some(String::from("test2")),
                use_loaded_icon: None,
                notes: None,
                version: None,
                modloader: None,
                global_java_args: None,
                extra_java_args: None,
                memory: None,
                java_override: None,
                pre_launch_hook: None,
                post_exit_hook: None,
                wrapper_command: None,
                game_resolution: None,
                modpack_locked: None,
                mod_sources: None,
            })
            .await?;

        expected[0].instances[0].name = String::from("test2");

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // check that it was persisted
        app.restart_in_place().await;

        // wait for instance scan
        tokio::time::sleep(Duration::from_millis(100)).await;

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // delete
        app.instance_manager().delete_instance(instance_id).await?;
        expected[0].instances.clear();

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // check that it was persisted
        app.restart_in_place().await;

        // wait for instance scan
        tokio::time::sleep(Duration::from_millis(100)).await;

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_modpack_info() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];
        let curseforge_instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("curseforge instance"),
                false,
                InstanceVersionSource::Modpack(
                    info::Modpack::Curseforge(info::CurseforgeModpack {
                        // RLCraft
                        project_id: 285109,
                        file_id: 4612979,
                    }),
                    true,
                ),
                String::new(),
            )
            .await?;

        let modrinth_instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("modrinth instance"),
                false,
                InstanceVersionSource::Modpack(
                    info::Modpack::Modrinth(info::ModrinthModpack {
                        // Fabulously Optimized
                        project_id: String::from("1KVo5zza"),
                        version_id: String::from("HH3vor7X"),
                    }),
                    true,
                ),
                String::new(),
            )
            .await?;

        async fn count(db: &carbon_repos::db_exec::Db, table: &'static str) -> anyhow::Result<i64> {
            Ok(db
                .read(move |conn| {
                    use carbon_repos::db_exec::ReadAccess;
                    // Runtime table name: a dynamic-SQL read through the read guard.
                    Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?)
                })
                .await?)
        }

        assert_eq!(count(&app.db, "CurseForgeModpackCache").await?, 0);
        assert_eq!(count(&app.db, "ModrinthModpackCache").await?, 0);

        app.instance_manager()
            .get_modpack_info(curseforge_instance_id)
            .await?;

        app.instance_manager()
            .get_modpack_info(modrinth_instance_id)
            .await?;

        assert_eq!(count(&app.db, "CurseForgeModpackCache").await?, 1);
        assert_eq!(count(&app.db, "ModrinthModpackCache").await?, 1);
        assert_eq!(count(&app.db, "CurseForgeModpackImageCache").await?, 1);
        assert_eq!(count(&app.db, "ModrinthModpackImageCache").await?, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_next_folder_ascii() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let (instance_name, _) = app.instance_manager().next_folder("some_instance")?;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];
        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "some_instance");

        let (instance_name, _) = app.instance_manager().next_folder("some_instance")?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "some_instance1");

        let (instance_name, _) = app.instance_manager().next_folder("some_instance")?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "some_instance2");

        Ok(())
    }

    #[tokio::test]
    async fn text_next_folder_basic_unicode() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];

        let (instance_name, _) = app.instance_manager().next_folder("ɀɃɏɔɮ˞˳̸")?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "ɀɃɏɔɮ˞˳̸");

        let (instance_name, _) = app
            .instance_manager()
            .next_folder("Cozy Cottage 𝘸𝘪𝘵𝘩 𝘴𝘢𝘶𝘤𝘦 🧂")?;

        assert_eq!(instance_name, "Cozy Cottage 𝘸𝘪𝘵𝘩 𝘴𝘢𝘶𝘤𝘦 🧂");

        Ok(())
    }

    #[tokio::test]
    async fn test_next_folder_unicode() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];

        // Although the following two strings look the same, they are not.
        // Different filesystems handle it differently

        let e_with_1_byte = "é"; // precomposed (U+00E9)
        let e_with_2_bytes = "é"; // decomposed (U+0065 U+0301)

        let (instance_name, _) = app.instance_manager().next_folder(e_with_1_byte)?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, e_with_1_byte);

        let (instance_name, _) = app.instance_manager().next_folder(e_with_2_bytes)?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        let comparison = if std::env::consts::OS == "macos" {
            format!("{}{}", e_with_2_bytes, "1") // macos saves as decomposed
        } else {
            e_with_2_bytes.to_string()
        };

        assert_eq!(instance_name, comparison);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "TODO: fix"]
    async fn test_next_folder_long_input() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;
        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];

        let e_with_2_bytes = "é"; // decomposed (U+0065 U+0301)

        // long string should be truncated with graphemes and not code points
        let mut long_string = String::new();
        for _ in 0..100 {
            long_string.push_str(e_with_2_bytes);
        }

        let (instance_name, _) = app.instance_manager().next_folder(&*long_string)?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name.graphemes(true).count(), 28);
        assert_eq!(instance_name.len(), 84); // UTF8 3 bytes * 28 allowed graphemes

        Ok(())
    }

    // --- startup pid reconciliation (instance-specific) ------------------
    //
    // The pidfile read/write/remove lifecycle and the `reconcile_pid` /
    // `is_live_java_process` decision logic themselves are tested generically
    // in `managers::orphan_pid`. These exercise
    // `reconcile_running_instances_under` end to end against a plain temp
    // directory laid out like the real instances root, using the actual
    // `.gdl_instance.pid` file name.

    /// The start time sysinfo currently reports for `pid`, via a targeted
    /// refresh — the same lookup the real writer sites do right after
    /// spawning a process, so tests can write a pidfile that reconciliation
    /// will actually verify as matching.
    fn live_start_time(pid: u32) -> u64 {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(
            pid,
        )]));
        super::orphan_pid::process_start_time(&system, pid)
            .expect("pid must be alive to look up its start time")
    }

    #[tokio::test]
    async fn reconcile_running_instances_retains_a_corrupt_pidfile() {
        // Malformed ownership remains recoverable; it never grants adoption.
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        tokio::fs::write(
            super::orphan_pid::pid_file_path(&instance_root, super::PID_FILE_NAME),
            b"not-a-pid",
        )
        .await
        .unwrap();

        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        assert!(adopted.is_empty(), "a corrupt pidfile must not be adopted");
        assert!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .is_err()
        );
        assert!(instance_root.join(super::PID_FILE_NAME).exists());
    }

    #[tokio::test]
    async fn reconcile_running_instances_removes_pidfile_for_a_dead_recorded_pid() {
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        let dead_pid = u32::MAX - 100;
        super::orphan_pid::write_pid_file(
            &instance_root,
            super::PID_FILE_NAME,
            dead_pid,
            1_700_000_000,
        )
        .await;

        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        assert!(adopted.is_empty(), "a dead pid must not be adopted");
        assert_eq!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .unwrap(),
            None,
            "a dead recorded pid's pidfile must be removed"
        );
    }

    #[tokio::test]
    async fn reconcile_running_instances_removes_pidfile_for_a_live_non_java_pid_without_killing() {
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        // Our own pid: alive for the duration of the test, but not java.
        let own_pid = std::process::id();
        super::orphan_pid::write_pid_file(
            &instance_root,
            super::PID_FILE_NAME,
            own_pid,
            live_start_time(own_pid),
        )
        .await;

        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        assert!(
            adopted.is_empty(),
            "a live pid that is not java is a reused pid number, never a game to adopt"
        );
        assert_eq!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .unwrap(),
            None,
            "a live non-java recorded pid's pidfile must still be removed"
        );
        // Confirms reconciliation never tried to kill anything here: this
        // test's own process is still alive to make this assertion at all.
        assert!(
            sysinfo::System::new_all()
                .process(sysinfo::Pid::from_u32(own_pid))
                .is_some()
        );
    }

    /// A live process whose name is `java`, without needing a JRE installed:
    /// `sysinfo` reports the executable's own name, so copying any
    /// long-running binary to a file called `java` and running that produces
    /// a process indistinguishable from a game JVM as far as
    /// `is_live_java_process` is concerned — which is precisely the input the
    /// adoption branch turns on.
    ///
    /// Unix-only, like the `sleep` it borrows. The branch it covers is
    /// platform-independent.
    #[cfg(unix)]
    async fn spawn_fake_jvm(dir: &std::path::Path) -> tokio::process::Child {
        super::orphan_pid::spawn_fake_java(dir).await
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reconcile_running_instances_adopts_a_live_game_instead_of_killing_it() {
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        let bin_dir = tempfile::tempdir().unwrap();
        let mut fake_jvm = spawn_fake_jvm(bin_dir.path()).await;
        let pid = fake_jvm.id().expect("fake jvm must have a pid");
        let start_time = live_start_time(pid);

        super::orphan_pid::write_pid_file(&instance_root, super::PID_FILE_NAME, pid, start_time)
            .await;

        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        let game = adopted.get("some-instance").expect(
            "a live game JVM whose recorded start time matches must be adopted, not killed",
        );
        assert_eq!(game.pid, pid);
        // The verified start time is carried into `AdoptedGame` rather than
        // dropped: it is what `RunningInstance::adopted` threads onward so
        // later re-checks (the poller, `kill_instance`) can tell this same
        // process apart from whatever the OS hands the pid to next.
        assert_eq!(game.verified_start_time, start_time);

        // The process is the user's session and must survive being found.
        assert!(
            sysinfo::System::new_all()
                .process(sysinfo::Pid::from_u32(pid))
                .is_some(),
            "reconciliation killed the adopted game process"
        );
        // And the pidfile has to stay: for as long as that process lives it
        // is the only record of it, and a later startup needs it too.
        assert_eq!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .unwrap(),
            Some((pid, Some(start_time))),
            "an adopted game's pidfile must be left in place"
        );

        fake_jvm.kill().await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reconcile_running_instances_retains_a_legacy_pidfile_for_a_live_game_without_adopting_it()
     {
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        let bin_dir = tempfile::tempdir().unwrap();
        let mut fake_jvm = spawn_fake_jvm(bin_dir.path()).await;
        let pid = fake_jvm.id().expect("fake jvm must have a pid");

        // A pre-fix single-line pidfile: pid only, no start time to verify.
        tokio::fs::write(
            super::orphan_pid::pid_file_path(&instance_root, super::PID_FILE_NAME),
            pid.to_string(),
        )
        .await
        .unwrap();

        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        assert!(
            adopted.is_empty(),
            "a legacy pidfile can never prove identity, so it must never be adopted even though the pid is alive and java-looking"
        );
        assert_eq!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .unwrap(),
            Some((pid, None)),
            "an unverifiable legacy pidfile remains recoverable"
        );

        fake_jvm.kill().await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reconcile_running_instances_retains_a_pidfile_with_mismatched_start_time_without_adopting_it()
     {
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        let bin_dir = tempfile::tempdir().unwrap();
        let mut fake_jvm = spawn_fake_jvm(bin_dir.path()).await;
        let pid = fake_jvm.id().expect("fake jvm must have a pid");
        let real_start_time = live_start_time(pid);

        // A pidfile recording a start time far from the process's actual one
        // — as if this pid had been reused by an unrelated process after the
        // launcher's own JVM (which really did start at `real_start_time`
        // minus a day) had already exited.
        super::orphan_pid::write_pid_file(
            &instance_root,
            super::PID_FILE_NAME,
            pid,
            real_start_time.saturating_sub(86_400),
        )
        .await;

        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        assert!(
            adopted.is_empty(),
            "a start-time mismatch means the pid was reused; it must never be adopted"
        );
        assert_eq!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .unwrap(),
            if cfg!(target_os = "linux") {
                Some((pid, Some(real_start_time.saturating_sub(86_400))))
            } else {
                None
            },
            "Linux legacy clock mismatch remains unconfirmed; other platforms keep their existing mismatch policy"
        );

        fake_jvm.kill().await.unwrap();
    }

    #[tokio::test]
    async fn reconcile_running_instances_ignores_instances_with_no_pidfile() {
        let dir = tempfile::tempdir().unwrap();
        let instance_root = dir.path().join("some-instance");
        tokio::fs::create_dir_all(&instance_root).await.unwrap();

        // Must not panic/fail when there's simply nothing to reconcile.
        let adopted = super::reconcile_running_instances_under(dir.path()).await;

        assert!(adopted.is_empty());
        assert_eq!(
            super::orphan_pid::read_pid_file(&instance_root, super::PID_FILE_NAME)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn reconcile_running_instances_under_tolerates_a_missing_directory() {
        // Pointing at a directory that doesn't exist at all (e.g. a fresh
        // install with no instances yet) must not panic or hang.
        let bogus_root = std::path::Path::new("/nonexistent/gdl-test-instances-root");
        let adopted = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            super::reconcile_running_instances_under(bogus_root),
        )
        .await
        .expect("reconcile_running_instances_under must not hang on a missing directory");

        assert!(adopted.is_empty());
    }

    // --- duplicate_instance ------------------------------------------------

    #[tokio::test]
    async fn duplicate_instance_does_not_carry_the_source_pidfile() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                "source".to_string(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        let source_root = {
            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let shortpath = instances[&instance_id].shortpath.clone();
            app.settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath)
                .get_root()
        };

        // As if the source instance were running: the pidfile lives directly
        // under the instance root, a sibling of the `instance/` data dir that
        // `fs_extra::dir::copy` copies wholesale.
        super::orphan_pid::write_pid_file(&source_root, super::PID_FILE_NAME, 4242, 1_700_000_000)
            .await;

        let duplicate_id = app
            .instance_manager()
            .duplicate_instance(instance_id, "duplicate".to_string())
            .await?;

        let duplicate_root = {
            let instance_manager = app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let shortpath = instances[&duplicate_id].shortpath.clone();
            app.settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath)
                .get_root()
        };

        assert_eq!(
            super::orphan_pid::read_pid_file(&duplicate_root, super::PID_FILE_NAME).await?,
            None,
            "a duplicate must never carry the source's pidfile — it would let a future \
             reconcile pass adopt the duplicate as if it were running the source's game, \
             or let Stop on the duplicate signal the source's real process"
        );
        assert_eq!(
            super::orphan_pid::read_pid_file(&source_root, super::PID_FILE_NAME).await?,
            Some((4242, Some(1_700_000_000))),
            "duplicating an instance must not disturb the source's own pidfile"
        );

        Ok(())
    }

    // --- scan_instances ------------------------------------------------

    /// Write a minimal, valid `instance.json` directly to `dir` — the same
    /// on-disk shape `scan_instances` reads, without going through
    /// `create_instance` (which needs a resolvable game version) or the DB.
    async fn write_test_instance_json(dir: &std::path::Path, name: &str) {
        tokio::fs::create_dir_all(dir).await.unwrap();
        let config = info::Instance {
            name: name.to_string(),
            icon: info::InstanceIcon::Default,
            date_created: chrono::Utc::now(),
            date_updated: chrono::Utc::now(),
            last_played: None,
            seconds_played: 0,
            modpack: None,
            game_configuration: info::GameConfig {
                version: None,
                global_java_args: true,
                extra_java_args: None,
                memory: None,
                java_override: None,
                game_resolution: None,
            },
            pre_launch_hook: None,
            post_exit_hook: None,
            wrapper_command: None,
            mod_sources: None,
            notes: String::new(),
        };
        let json = super::schema::make_instance_config(config).unwrap();
        tokio::fs::write(dir.join("instance.json"), json)
            .await
            .unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scan_instances_isolates_one_bad_entry_from_the_rest_of_the_scan() -> anyhow::Result<()>
    {
        use super::{InstanceType, LaunchState};
        use carbon_repos::repos::app_configuration::AppConfigurationPatch;

        let app = crate::setup_managers_for_test().await;

        // App startup fires its own scan_instances pass in the background
        // (unawaited — see managers/mod.rs). Nothing exists on disk for it
        // to find yet, so it is a no-op, but without waiting for it here it
        // can land at an arbitrary point during this test's own setup and
        // race the scans below over the same instances directory.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let instances_dir = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        let good_root = instances_dir.join("instance-a");
        write_test_instance_json(&good_root, "instance-a").await;

        // Registers "instance-a" in the DB (cached) and creates the default
        // group before anything is broken.
        app.instance_manager().scan_instances().await?;

        // Break group resolution for any directory the DB does not already
        // know about: exactly what `add_instance` needs to register a *new*
        // directory. Deterministic and order-independent — unlike a
        // filesystem permission trick, it fails the same way on every call
        // regardless of directory iteration order or already-open handles,
        // and it leaves "instance-a" (already cached, so it never calls
        // `get_default_group`) completely unaffected.
        app.settings_manager()
            .set(AppConfigurationPatch {
                default_instance_group: Some(Some(999_999)),
                ..Default::default()
            })
            .await?;

        // As if a game were still running from a previous session: gives the
        // already-cached "instance-a" something to adopt on the next scan.
        let bin_dir = tempfile::tempdir().unwrap();
        let mut fake_jvm = spawn_fake_jvm(bin_dir.path()).await;
        let pid = fake_jvm.id().expect("fake jvm must have a pid");
        let start_time = live_start_time(pid);
        super::orphan_pid::write_pid_file(&good_root, super::PID_FILE_NAME, pid, start_time).await;

        // A brand new, never-before-seen directory: uncached, so it is the
        // one that actually reaches (and fails on) the broken group
        // resolution above.
        let bad_root = instances_dir.join("instance-b");
        write_test_instance_json(&bad_root, "instance-b").await;

        app.instance_manager().scan_instances().await?;

        let instance_manager = app.instance_manager();
        let instances = instance_manager.instances.read().await;
        let a = instances
            .values()
            .find(|i| i.shortpath == "instance-a")
            .expect("instance-a must still be tracked");
        let InstanceType::Valid(data) = &a.type_ else {
            panic!("instance-a became invalid");
        };
        assert!(
            matches!(&data.state, LaunchState::Running(r) if r.is_adopted()),
            "instance-a must still be adopted even though a later directory in the \
             same scan failed to register"
        );
        drop(instances);

        assert!(
            *instance_manager.any_instance_running.borrow(),
            "the liveness poller for the adopted instance must have been spawned \
             even though a later directory in the same scan failed"
        );

        fake_jvm.kill().await.unwrap();
        Ok(())
    }

    // --- download_icon filename derivation ------------------------------

    #[test]
    fn icon_filename_from_url_strips_the_query_string() {
        use super::icon_filename_from_url;

        let url = reqwest::Url::parse("https://cdn/x/icon.png?width=64").unwrap();
        assert_eq!(icon_filename_from_url(&url).as_deref(), Some("icon.png"));
    }

    #[test]
    fn icon_filename_from_url_is_none_when_the_path_has_no_final_segment() {
        use super::icon_filename_from_url;

        let url = reqwest::Url::parse("https://cdn/").unwrap();
        assert_eq!(icon_filename_from_url(&url), None);
    }

    #[test]
    fn sanitize_icon_filename_replaces_illegal_characters_and_never_produces_a_path() {
        use super::sanitize_icon_filename;

        let sanitized = sanitize_icon_filename("a\\b?.png");
        assert_eq!(sanitized, "a_b_.png");
        assert!(
            !sanitized.contains(['/', '\\']),
            "a sanitized icon filename must never contain a path separator: {sanitized:?}"
        );
    }

    #[test]
    fn sanitize_icon_filename_caps_length() {
        use super::sanitize_icon_filename;

        let long = "a".repeat(200);
        assert_eq!(sanitize_icon_filename(&long).chars().count(), 64);
    }

    #[test]
    fn sanitize_icon_filename_never_passes_through_a_bare_dot_segment() {
        use super::sanitize_icon_filename;

        // A `.`/`..` segment carries no name of its own, and `..` doctored
        // through unchanged would claim (falsely) to be a safe filename
        // while actually walking out of whatever directory it's joined to.
        assert_eq!(sanitize_icon_filename("."), "icon");
        assert_eq!(sanitize_icon_filename(".."), "icon");
        assert_eq!(sanitize_icon_filename(""), "icon");
    }

    #[test]
    fn extension_from_content_type_reads_the_subtype() {
        use super::extension_from_content_type;

        assert_eq!(extension_from_content_type(Some("image/png")), "png");
        assert_eq!(
            extension_from_content_type(Some("image/png; charset=utf-8")),
            "png"
        );
        assert_eq!(extension_from_content_type(None), "png");
        assert_eq!(extension_from_content_type(Some("garbage")), "png");
    }

    #[test]
    fn icon_filename_keeps_a_url_segment_that_already_has_an_extension() {
        use super::icon_filename;

        assert_eq!(
            icon_filename(Some("icon.png".to_string()), Some("image/jpeg")),
            "icon.png",
            "an extension already present in the URL must win over Content-Type"
        );
    }

    #[test]
    fn icon_filename_appends_a_content_type_extension_to_an_extensionless_segment() {
        use super::icon_filename;

        // The regression this closes: a URL segment with no extension of its
        // own (`.../noext`) must not silently drop the response's real
        // Content-Type just because *some* segment was found — only "no
        // segment at all" is supposed to reach for a bare `icon.<ext>`.
        assert_eq!(
            icon_filename(Some("noext".to_string()), Some("image/png")),
            "noext.png"
        );
        assert_eq!(
            icon_filename(Some("noext".to_string()), None),
            "noext.png",
            "an extensionless segment with no Content-Type either must still fall back to png"
        );
    }

    #[test]
    fn icon_filename_falls_back_to_a_bare_icon_stem_with_no_url_segment_at_all() {
        use super::icon_filename;

        assert_eq!(icon_filename(None, Some("image/png")), "icon.png");
        assert_eq!(icon_filename(None, None), "icon.png");
    }
}

#[cfg(test)]
mod task18_icon_tests {
    use super::*;
    #[tokio::test]
    async fn task18_icon_staging_control_names_are_owned_paths() {
        for basename in [
            ".setup",
            "instance.json",
            "icon.png",
            "noext",
            "embedded.png",
        ] {
            let root = tempfile::tempdir().unwrap();
            let url =
                reqwest::Url::parse(&format!("https://fixture.invalid/{basename}?size=64#icon"))
                    .unwrap();
            let supplied = icon_filename(icon_filename_from_url(&url), Some("image/png"));
            let bytes = hex::decode("89504e470d0a1a0a0000000d4948445200000001000000010804000000b51c0c020000000b4944415478da63fcff1f0003030200efa79a1f0000000049454e44ae426082").unwrap();
            let icon = stage_instance_icon(root.path(), Some((supplied, bytes.to_vec())))
                .await
                .unwrap();
            let info = info::Instance {
                name: "fixture".into(),
                icon: icon.clone(),
                date_created: Utc::now(),
                date_updated: Utc::now(),
                last_played: None,
                seconds_played: 0,
                modpack: None,
                game_configuration: info::GameConfig {
                    version: None,
                    global_java_args: true,
                    extra_java_args: None,
                    memory: None,
                    java_override: None,
                    game_resolution: None,
                },
                pre_launch_hook: None,
                post_exit_hook: None,
                wrapper_command: None,
                mod_sources: None,
                notes: String::new(),
            };
            let json = schema::make_instance_config(info).unwrap();
            std::fs::write(root.path().join("instance.json"), &json).unwrap();
            std::fs::create_dir(root.path().join(".setup")).unwrap();
            let InstanceIcon::RelativePath(path) = icon else {
                panic!("missing icon")
            };
            assert_eq!(
                std::fs::read(root.path().join(&path)).unwrap(),
                bytes,
                "{basename}"
            );
            assert_ne!(path, "instance.json");
            assert_ne!(path, ".setup");
            assert_eq!(
                std::fs::read_to_string(root.path().join("instance.json")).unwrap(),
                json
            );
        }
    }
    #[tokio::test]
    async fn task18_icon_missing_and_supported_svg_keep_usable_storage() {
        let root = tempfile::tempdir().unwrap();
        assert!(matches!(
            stage_instance_icon(root.path(), None).await.unwrap(),
            InstanceIcon::Default
        ));
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1"/></svg>"#;
        let icon = stage_instance_icon(root.path(), Some(("embedded.svg".into(), svg.to_vec())))
            .await
            .unwrap();
        let InstanceIcon::RelativePath(path) = icon else {
            panic!("missing SVG")
        };
        assert_eq!(path, "icon.svg");
        assert_eq!(std::fs::read(root.path().join(path)).unwrap(), svg);
    }
}

#[cfg(test)]
mod task22_tests {
    use super::*;
    fn record() -> orphan_pid::ProcessRecord {
        orphan_pid::ProcessRecord {
            pid: 4242,
            start_time: Some(1700000000),
            identity: Some(orphan_pid::ProcessIdentity::LinuxBootTicks {
                boot_id: uuid::Uuid::parse_str("cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80").unwrap(),
                start_ticks: 123456,
            }),
        }
    }
    #[tokio::test]
    async fn task22_actual_adoption_roundtrip_survives_clock_step() {
        let root = tempfile::tempdir().unwrap();
        let instance = root.path().join("owned");
        tokio::fs::create_dir(&instance).await.unwrap();
        let record = record();
        orphan_pid::write_process_file(&instance, PID_FILE_NAME, record).await;
        for epoch in [1699999970, 1700000030] {
            let adopted = reconcile_running_instances_under_observed(root.path(), |pid| {
                assert_eq!(pid, 4242);
                orphan_pid::ProcessObservation::Present {
                    process: orphan_pid::LiveProc {
                        is_java: true,
                        start_time: epoch,
                    },
                    identity: record.identity,
                }
            })
            .await;
            assert_eq!(adopted["owned"].verified_process, record);
            assert_eq!(
                orphan_pid::read_process_file(&instance, PID_FILE_NAME)
                    .await
                    .unwrap(),
                Some(record)
            );
        }
    }
    #[tokio::test]
    async fn task22_actual_adoption_unknown_and_legacy_mismatch_retains_markers() {
        let root = tempfile::tempdir().unwrap();
        let instance = root.path().join("owned");
        tokio::fs::create_dir(&instance).await.unwrap();
        let record = record();
        orphan_pid::write_process_file(&instance, PID_FILE_NAME, record).await;
        assert!(
            reconcile_running_instances_under_observed(root.path(), |_| {
                orphan_pid::ProcessObservation::Unknown
            })
            .await
            .is_empty()
        );
        assert_eq!(
            orphan_pid::read_process_file(&instance, PID_FILE_NAME)
                .await
                .unwrap(),
            Some(record)
        );
        let legacy = orphan_pid::ProcessRecord::legacy(4242, 1700000000);
        orphan_pid::write_process_file(&instance, PID_FILE_NAME, legacy).await;
        let observed = orphan_pid::ProcessObservation::Present {
            process: orphan_pid::LiveProc {
                is_java: true,
                start_time: 1700000030,
            },
            identity: record.identity,
        };
        assert!(
            reconcile_running_instances_under_observed(root.path(), |_| observed)
                .await
                .is_empty()
        );
        #[cfg(target_os = "linux")]
        assert_eq!(
            orphan_pid::read_process_file(&instance, PID_FILE_NAME)
                .await
                .unwrap(),
            Some(legacy)
        );
        #[cfg(not(target_os = "linux"))]
        assert_eq!(
            orphan_pid::read_process_file(&instance, PID_FILE_NAME)
                .await
                .unwrap(),
            None
        );
        tokio::fs::write(
            instance.join(PID_FILE_NAME),
            b"4242\n1700000000\nunknown-extension",
        )
        .await
        .unwrap();
        assert!(
            reconcile_running_instances_under_observed(root.path(), |_| panic!(
                "Malformed extension cannot query/adopt via legacy fallback"
            ))
            .await
            .is_empty()
        );
        assert!(instance.join(PID_FILE_NAME).exists());
    }
    #[tokio::test]
    async fn task22_actual_legacy_upgrade_requires_java_epoch_proof_and_reuse_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let instance = root.path().join("owned");
        tokio::fs::create_dir(&instance).await.unwrap();
        let record = record();
        orphan_pid::write_process_file(
            &instance,
            PID_FILE_NAME,
            orphan_pid::ProcessRecord::legacy(4242, 1700000000),
        )
        .await;
        let adopted = reconcile_running_instances_under_observed(root.path(), |_| {
            orphan_pid::ProcessObservation::Present {
                process: orphan_pid::LiveProc {
                    is_java: true,
                    start_time: 1700000000,
                },
                identity: record.identity,
            }
        })
        .await;
        assert_eq!(adopted["owned"].verified_process, record);
        assert_eq!(
            orphan_pid::read_process_file(&instance, PID_FILE_NAME)
                .await
                .unwrap(),
            Some(record)
        );
        assert!(
            reconcile_running_instances_under_observed(root.path(), |_| {
                orphan_pid::ProcessObservation::Absent
            })
            .await
            .is_empty()
        );
        assert!(!instance.join(PID_FILE_NAME).exists());
    }
}
