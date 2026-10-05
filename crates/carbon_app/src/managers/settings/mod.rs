use self::terms_and_privacy::TermsAndPrivacy;
use super::ManagerRef;
use crate::api::{
    keys::settings::*,
    settings::{CacheCleanupSelection, CacheSizes, FESettingsUpdate},
    translation::Translation,
};
use crate::domain::vtask::VisualTaskId;
use crate::managers::vtask::{Subtask, TaskState, VisualTask};
use anyhow::{anyhow, bail};
use carbon_platforms::{ModChannelWithUsage, ModPlatform};
use carbon_repos::repos::app_configuration::{AppConfigurationPatch, AppConfigurationRow};
use itertools::Itertools;
use reqwest_middleware::ClientWithMiddleware;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tracing::{error, info, warn};

pub mod terms_and_privacy;

pub(crate) struct SettingsManager {
    pub runtime_path: carbon_rt_path::RuntimePath,
    pub terms_and_privacy: TermsAndPrivacy,
    pub gdl_base_api_url: String,
    pub latest_consent_checksum: Option<String>,
    /// Set while a cache cleanup is executing. Prevents concurrent cleanups
    /// from both trying to VACUUM or racing on table DELETEs.
    cleanup_running: Arc<AtomicBool>,
}

impl SettingsManager {
    pub fn new(
        runtime_path: PathBuf,
        http_client: ClientWithMiddleware,
        gdl_base_api_url: String,
        latest_consent_checksum: Option<String>,
    ) -> Self {
        Self {
            runtime_path: carbon_rt_path::RuntimePath::new(runtime_path),
            terms_and_privacy: TermsAndPrivacy::new(http_client, gdl_base_api_url.clone()),
            gdl_base_api_url,
            latest_consent_checksum,
            cleanup_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ManagerRef<'_, SettingsManager> {
    pub async fn get_settings(self) -> anyhow::Result<AppConfigurationRow> {
        carbon_repos::repos::app_configuration::get_app_configuration(&self.app.db)
            .await?
            .ok_or(anyhow!("Can't find this key"))
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_settings(self, incoming_settings: FESettingsUpdate) -> anyhow::Result<()> {
        let mut patch = AppConfigurationPatch::default();

        if let Some(theme) = incoming_settings.theme {
            patch.theme = Some(theme.inner());
        }

        if let Some(language) = incoming_settings.language {
            patch.language = Some(language.inner());
        }

        if let Some(reduced_motion) = incoming_settings.reduced_motion {
            patch.reduced_motion = Some(reduced_motion.inner());
        }

        if let Some(discord_integration) = incoming_settings.discord_integration {
            patch.discord_integration = Some(discord_integration.inner());
        }

        if let Some(release_channel) = incoming_settings.release_channel {
            patch.release_channel = Some(release_channel.inner().into());
        }

        if let Some(launcher_action_on_game_launch) =
            incoming_settings.launcher_action_on_game_launch
        {
            patch.launcher_action_on_game_launch =
                Some(launcher_action_on_game_launch.inner().into());
        }

        if let Some(show_app_close_warning) = incoming_settings.show_app_close_warning.clone() {
            patch.show_app_close_warning = Some(show_app_close_warning.inner());
        }

        if let Some(concurrent_downloads) = incoming_settings.concurrent_downloads {
            patch.concurrent_downloads = Some(concurrent_downloads.inner());
        }

        if let Some(download_dependencies) = incoming_settings.download_dependencies {
            patch.download_dependencies = Some(download_dependencies.inner());
        }

        if let Some(show_featured) = incoming_settings.show_featured {
            patch.show_featured = Some(show_featured.inner());
        }

        // instances_sort_by: Option<Option<InstancesSortBy>>
        // Outer Option = whether to update, Inner Option = null (manual order) or Some(sort criteria)
        if let Some(sort_by) = incoming_settings.instances_sort_by {
            patch.instances_sort_by = Some(sort_by.inner().map(Into::into));
        }

        if let Some(instances_sort_by_asc) = incoming_settings.instances_sort_by_asc {
            patch.instances_sort_by_asc = Some(instances_sort_by_asc.inner());
        }

        // instances_group_by: Option<Option<InstancesGroupBy>>
        // Outer Option = whether to update, Inner Option = null (folders mode) or Some(group criteria)
        if let Some(instances_group_by) = incoming_settings.instances_group_by {
            patch.instances_group_by = Some(instances_group_by.inner().map(Into::into));
        }

        if let Some(instances_group_by_asc) = incoming_settings.instances_group_by_asc {
            patch.instances_group_by_asc = Some(instances_group_by_asc.inner());
        }

        if let Some(instances_duplicate_favorites) = incoming_settings.instances_duplicate_favorites
        {
            patch.instances_duplicate_favorites = Some(instances_duplicate_favorites.inner());
        }

        if let Some(instances_tile_size) = incoming_settings.instances_tile_size {
            patch.instances_tile_size = Some(instances_tile_size.inner().into());
        }

        if let Some(deletion_through_recycle_bin) = incoming_settings.deletion_through_recycle_bin {
            patch.deletion_through_recycle_bin = Some(deletion_through_recycle_bin.inner());
        }

        if let Some(xmx) = incoming_settings.xmx {
            patch.xmx = Some(xmx.inner());
        }

        if let Some(xms) = incoming_settings.xms {
            patch.xms = Some(xms.inner());
        }

        if let Some(game_resolution) = incoming_settings.game_resolution {
            patch.game_resolution = Some(game_resolution.inner().map(Into::into));
        }

        if let Some(java_custom_args) = incoming_settings.java_custom_args {
            patch.java_custom_args = Some(java_custom_args.inner());
        }

        if let Some(pre_launch_hook) = incoming_settings.pre_launch_hook {
            patch.pre_launch_hook = Some(pre_launch_hook.inner());
        }

        if let Some(post_exit_hook) = incoming_settings.post_exit_hook {
            patch.post_exit_hook = Some(post_exit_hook.inner());
        }

        if let Some(wrapper_command) = incoming_settings.wrapper_command {
            patch.wrapper_command = Some(wrapper_command.inner());
        }

        if let Some(auto_manage_java_system_profiles) =
            incoming_settings.auto_manage_java_system_profiles.as_ref()
        {
            patch.auto_manage_java_system_profiles =
                Some(auto_manage_java_system_profiles.clone().inner());
        }

        if let Some(mod_sources) = incoming_settings.mod_sources {
            let mod_sources = mod_sources.inner();

            let platform_blacklist = mod_sources
                .platform_blacklist
                .into_iter()
                .map(ModPlatform::from)
                .map(|p| ModPlatform::as_str(&p))
                .join(",");

            let channels = mod_sources
                .channels
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>();
            ModChannelWithUsage::validate_list(&channels)?;

            let channels_str = ModChannelWithUsage::slice_to_str(&channels);

            patch.mod_platform_blacklist = Some(platform_blacklist);
            patch.mod_channels = Some(channels_str);
        }

        if let Some(terms_and_privacy_accepted) = incoming_settings.terms_and_privacy_accepted {
            // The net effect on the singleton row is `termsAndPrivacyAccepted =
            // true` plus the checksum, regardless of the incoming value. Apply
            // that net effect atomically.
            let _ = terms_and_privacy_accepted.inner();

            // We default to empty value in case our APIs fail so we don't block the user.
            // We are gonna ask again on next run anyway once the APIs are back up
            let latest_consent_sha = self
                .latest_consent_checksum
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();

            patch.terms_and_privacy_accepted = Some(true);
            patch.terms_and_privacy_accepted_checksum = Some(Some(latest_consent_sha));
        }

        if let Some(query) = patch.build() {
            self.app
                .db
                .write(move |conn| Ok(query.execute(&conn)?))
                .await?;
            self.app.invalidate(GET_SETTINGS, None);

            if let Some(show_app_close_warning) = incoming_settings.show_app_close_warning {
                println!(
                    "_SHOW_APP_CLOSE_WARNING_:{}",
                    show_app_close_warning.inner()
                );
            }
        }

        if let Some(auto_manage_java_system_profiles) =
            incoming_settings.auto_manage_java_system_profiles
        {
            if auto_manage_java_system_profiles.inner() {
                super::java::scan_and_sync::sync_system_java_profiles(&self.app.db).await?;
            }
        }

        Ok(())
    }

    pub async fn set(self, patch: AppConfigurationPatch) -> anyhow::Result<()> {
        if let Some(query) = patch.build() {
            self.app
                .db
                .write(move |conn| Ok(query.execute(&conn)?))
                .await?;
        }

        Ok(())
    }

    /// Returns just the SQLite DB file footprint (gdl_conf.db + WAL).
    /// Cheap: two `stat` calls. Drives the bloat banner.
    pub async fn get_db_size(self) -> f64 {
        let db_path = self.runtime_path.join("gdl_conf.db");
        let wal_path = self.runtime_path.join("gdl_conf.db-wal");
        let size_of =
            |p: PathBuf| async move { tokio::fs::metadata(&p).await.map(|m| m.len()).unwrap_or(0) };
        (size_of(db_path).await + size_of(wal_path).await) as f64
    }

    /// Returns the per-scope cache footprint that the cleanup modal exposes:
    /// `gdlauncher` mirrors what the GDLauncher-cache wipe targets (DB files
    /// + temp/ + __gdl_logs__/); `minecraft` mirrors the Minecraft-cache wipe (assets/ +
    /// libraries/ + natives/). Both numbers walk their dirs once on a
    /// blocking thread, so callers should treat this as moderately
    /// expensive — the Settings row and the cleanup modal share the same
    /// query so one walk feeds both.
    pub async fn get_cache_sizes(self) -> CacheSizes {
        let runtime_path = self.runtime_path.clone();
        tokio::task::spawn_blocking(move || {
            let file_len = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let gdlauncher = file_len(&runtime_path.join("gdl_conf.db"))
                + file_len(&runtime_path.join("gdl_conf.db-wal"))
                + dir_size_blocking(runtime_path.get_temp().to_path())
                + dir_size_blocking(runtime_path.join("__gdl_logs__"));
            let minecraft = dir_size_blocking(runtime_path.get_assets().to_path())
                + dir_size_blocking(runtime_path.get_libraries().to_path())
                + dir_size_blocking(runtime_path.join("natives"));
            CacheSizes {
                gdlauncher: gdlauncher as f64,
                minecraft: minecraft as f64,
            }
        })
        .await
        .unwrap_or(CacheSizes {
            gdlauncher: 0.0,
            minecraft: 0.0,
        })
    }

    /// Two-tier cleanup. Spawns a background task that:
    ///
    /// 1. If `gdlauncher`: clears every cache table, wipes `temp/` and
    ///    `__gdl_logs__/`, then VACUUMs.
    /// 2. If `minecraft`: also wipes `assets/`, `libraries/`, `natives/` —
    ///    these are big and opt-in; the next launch re-downloads them and
    ///    re-runs the loader processors for generated files.
    ///
    /// Rejects with an error if another cleanup is already running. The
    /// task is best-effort per step: individual failures are logged but
    /// don't abort the rest of the run.
    ///
    /// Reports progress as a single linear 0..1 covering both the disk
    /// file-by-file removal and the chunked DB row deletes; `VACUUM` is
    /// a separate opaque subtask weighted at 10% of the total so the bar
    /// doesn't sit at 100% while it runs. To compute that progress we
    /// walk the disk dirs once to count files and run COUNT(*) per
    /// table — both bounded by row/file count, not byte size, so they
    /// stay fast even on huge DBs.
    pub async fn cleanup_caches(
        self,
        selection: CacheCleanupSelection,
    ) -> anyhow::Result<VisualTaskId> {
        // Single-writer guard. Two cleanups would race the same VACUUM and
        // double-delete the same rows for no benefit.
        if self
            .cleanup_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            bail!("A cache cleanup is already in progress");
        }

        let task = VisualTask::new(Translation::CacheCleanup);
        task.require_explicit_completion();
        let task_id = self.app.task_manager().spawn_task(&task).await;

        // ---- Subtasks (created BEFORE the mutation returns) ----
        // We enter KnownProgress with a 0/1 placeholder right here, so by
        // the time the frontend receives task_id and starts polling
        // `vtask.getTask`, the very first response is already
        // KnownProgress(0%). Without this, the brief Indeterminate window
        // makes the Progress bar render its full-width shimmer animation
        // — which the user reads as "100%" before it snaps to 0/1%.
        //
        // The single "work" subtask linearizes file deletes and DB
        // row deletes onto the same 0..total_units counter; the
        // opaque VACUUM subtask (only on `gdlauncher`) is weighted at
        // 10% so the bar advances 0→90% during deletes and 90→100%
        // during VACUUM. With one subtask the bar is just
        // delete_progress, which is also fine.
        let work = Arc::new(task.subtask(Translation::CacheCleanup));
        let vacuum_subtask = if selection.gdlauncher {
            work.set_weight(0.9);
            let s = task.subtask(Translation::CacheCleanup);
            s.set_weight(0.1);
            Some(s)
        } else {
            None
        };
        task.edit(|d| d.state = TaskState::KnownProgress).await;
        work.update_items(0, 1);

        let app = self.app.clone();
        let runtime_path = self.runtime_path.clone();
        let cleanup_running = self.cleanup_running.clone();

        tokio::spawn(async move {
            info!("Starting cache cleanup with selection: {selection:?}");

            let _guard = CleanupLease(cleanup_running);

            let result =
                run_cache_cleanup(&app.db, &runtime_path, &selection, work, vacuum_subtask).await;
            // Partial changes are real changes: refresh sizes on every outcome.
            app.invalidate(GET_DB_SIZE, None);
            app.invalidate(GET_CACHE_SIZES, None);
            finish_cache_cleanup(task, result).await;
        });

        Ok(task_id)
    }
}

struct CleanupLease(Arc<AtomicBool>);
impl Drop for CleanupLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[derive(Default, Debug)]
struct CleanupFailures {
    count: usize,
    preview: Vec<String>,
}
impl CleanupFailures {
    fn record(&mut self, context: impl std::fmt::Display, error: impl std::fmt::Display) {
        self.count += 1;
        if self.preview.len() < 20 {
            self.preview.push(format!("{context}: {error}"));
        }
    }
    fn record_io(&mut self, context: impl std::fmt::Display, error: std::io::Error) {
        if let Some(nested) = error
            .get_ref()
            .and_then(|e| e.downcast_ref::<CleanupFailures>())
        {
            self.count += nested.count;
            for preview in &nested.preview {
                if self.preview.len() < 20 {
                    self.preview.push(format!("{context}: {preview}"));
                }
            }
        } else {
            self.record(context, error);
        }
    }
    fn result(self) -> anyhow::Result<()> {
        if self.count == 0 {
            Ok(())
        } else {
            Err(anyhow::Error::new(self))
        }
    }
}
impl std::fmt::Display for CleanupFailures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache cleanup had {} failure(s); retry the selected scopes after resolving access or database errors. {}",
            self.count,
            self.preview.join("; ")
        )
    }
}
impl std::error::Error for CleanupFailures {}
const CLEANUP_TABLES: &[&str] = &[
    "HTTPCache",
    "ModrinthUpdateCache",
    "CurseForgeModImageCache",
    "ModrinthModImageCache",
    "LocalModImageCache",
    "CurseForgeModpackImageCache",
    "ModrinthModpackImageCache",
    "CurseForgeModCache",
    "ModrinthModCache",
    "CurseForgeModpackCache",
    "ModrinthModpackCache",
    "VersionInfoCache",
    "PartialVersionInfoCache",
    "LwjglMetaCache",
    "AssetsMetaCache",
];
async fn run_cache_cleanup(
    db: &carbon_repos::db_exec::Db,
    runtime: &carbon_rt_path::RuntimePath,
    selection: &CacheCleanupSelection,
    work: Arc<Subtask>,
    vacuum: Option<Subtask>,
) -> anyhow::Result<()> {
    let mut failures = CleanupFailures::default();
    if selection.gdlauncher {
        let logs = runtime.join("__gdl_logs__");
        match tokio::task::spawn_blocking(move || crate::logger::cleanup_old_logs_report(&logs, 10))
            .await
        {
            Ok(errors) => {
                for error in errors {
                    failures.record("old log cleanup", error);
                }
            }
            Err(error) => failures.record("old log cleanup worker", error),
        }
    }
    let mut paths = Vec::new();
    if selection.gdlauncher {
        paths.push(runtime.get_temp().to_path());
    }
    if selection.minecraft {
        paths.extend([
            runtime.get_assets().to_path(),
            runtime.get_libraries().to_path(),
            runtime.join("natives"),
        ]);
    }
    let mut total = 0u64;
    let mut count_complete = true;
    for path in &paths {
        match count_files_blocking(path.clone()).await {
            Ok(count) => total = total.saturating_add(count),
            Err(error) => {
                count_complete = false;
                failures.record(format!("count {}", path.display()), error);
            }
        }
    }
    if selection.gdlauncher {
        for table in CLEANUP_TABLES {
            match try_count_table(db, table).await {
                Ok(count) => total = total.saturating_add(count),
                Err(error) => {
                    count_complete = false;
                    failures.record(format!("count table {table}"), error);
                }
            }
        }
    }
    let total = count_complete.then(|| total.try_into().unwrap_or(u32::MAX));
    publish_cleanup_progress(&work, 0, total);
    let progress = Arc::new(AtomicU64::new(0));
    for path in paths {
        if let Err(error) =
            clear_dir_with_progress(path.clone(), progress.clone(), work.clone(), total).await
        {
            failures.record_io(format!("clear {}", path.display()), error);
        }
    }
    if selection.gdlauncher {
        for table in CLEANUP_TABLES {
            let sql =
                format!("DELETE FROM {table} WHERE rowid IN (SELECT rowid FROM {table} LIMIT 200)");
            loop {
                let query = carbon_repos::registry::DynamicQuery {
                    sql: sql.clone(),
                    params: vec![],
                };
                match db.write(move |conn| Ok(query.execute(&conn)?)).await {
                    Ok(0) => break,
                    Ok(count) => {
                        let current =
                            progress.fetch_add(count as u64, Ordering::Relaxed) + count as u64;
                        publish_cleanup_progress(
                            &work,
                            current.try_into().unwrap_or(u32::MAX),
                            total,
                        );
                    }
                    Err(error) => {
                        failures.record(format!("delete table {table}"), error);
                        break;
                    }
                }
            }
        }
        if let Some(vacuum) = vacuum.as_ref() {
            vacuum.start_opaque();
        }
        match db
            .write(|conn| {
                use carbon_repos::db_exec::WriteAccess;
                Ok(conn.execute_batch("VACUUM")?)
            })
            .await
        {
            Ok(()) => {
                if let Some(vacuum) = vacuum {
                    vacuum.complete_opaque();
                }
            }
            Err(error) => failures.record("VACUUM reclaim cache space", error),
        }
    }
    if failures.count == 0 {
        work.complete_items();
    }
    failures.result()
}
/// A failed count leaves required work unknown, even after all counted units are removed.
/// Keep real removal counts internally without publishing a complete partial ratio.
fn publish_cleanup_progress(subtask: &Subtask, current: u32, total: Option<u32>) {
    match total {
        Some(total) => subtask.update_items(current, total),
        None => subtask.start_opaque(),
    }
}
async fn finish_cache_cleanup(task: VisualTask, result: anyhow::Result<()>) {
    match result {
        Ok(()) => task.complete(),
        Err(error) => task.fail(error).await,
    }
}
async fn try_count_table(
    db: &carbon_repos::db_exec::Db,
    table: &str,
) -> carbon_repos::db_error::DbResult<u64> {
    let query = carbon_repos::registry::DynamicQuery {
        sql: format!("SELECT COUNT(*) FROM {table}"),
        params: vec![],
    };
    db.read(move |conn| Ok(query.query_scalar_i64(&conn)?.max(0) as u64))
        .await
}
#[cfg(test)]
async fn count_table(db: &carbon_repos::db_exec::Db, table: &str) -> u64 {
    try_count_table(db, table).await.unwrap()
}

/// Sync recursive walk that sums file byte sizes under `path`. Caller is
/// responsible for running this on a blocking thread (e.g., inside
/// `tokio::task::spawn_blocking`) — the FS calls are blocking.
fn dir_size_blocking(path: PathBuf) -> u64 {
    fn walk(p: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(p) else {
            return 0;
        };
        let mut total = 0u64;
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            } else if meta.is_dir() {
                total = total.saturating_add(walk(&entry.path()));
            }
        }
        total
    }
    if path.exists() { walk(&path) } else { 0 }
}

async fn count_files_blocking(path: PathBuf) -> std::io::Result<u64> {
    tokio::task::spawn_blocking(move || {
        fn walk(path: &Path) -> std::io::Result<u64> {
            let entries = match std::fs::read_dir(path) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
                Err(error) => return Err(error),
            };
            let mut count = 0u64;
            for entry in entries {
                let entry = entry?;
                let kind = entry.file_type()?;
                count = count.saturating_add(if kind.is_dir() {
                    walk(&entry.path())?
                } else {
                    1
                });
            }
            Ok(count)
        }
        walk(&path)
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?
}
/// Continue independent removals, counting only removed or confirmed-absent
/// file postconditions. Iterator, metadata and directory failures remain errors.
async fn clear_dir_with_progress(
    path: PathBuf,
    progress: Arc<AtomicU64>,
    subtask: Arc<Subtask>,
    total: Option<u32>,
) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || {
        fn walk(
            path: &Path,
            progress: &AtomicU64,
            subtask: &Subtask,
            total: Option<u32>,
            failures: &mut CleanupFailures,
        ) {
            let entries = match std::fs::read_dir(path) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
                Err(error) => {
                    failures.record(path.display(), error);
                    return;
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        failures.record(path.display(), error);
                        continue;
                    }
                };
                let child = entry.path();
                // Do not traverse symlinked cache directories outside the selected tree.
                let kind = match entry.file_type() {
                    Ok(kind) => kind,
                    Err(error) => {
                        failures.record(child.display(), error);
                        continue;
                    }
                };
                if kind.is_dir() {
                    walk(&child, progress, subtask, total, failures);
                    if let Err(error) = std::fs::remove_dir(&child) {
                        if error.kind() != std::io::ErrorKind::NotFound {
                            failures.record(child.display(), error);
                        }
                    }
                } else {
                    match std::fs::remove_file(&child) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            failures.record(child.display(), error);
                            continue;
                        }
                    }
                    let count = progress.fetch_add(1, Ordering::Relaxed) + 1;
                    if count % 25 == 0 {
                        publish_cleanup_progress(
                            subtask,
                            count.try_into().unwrap_or(u32::MAX),
                            total,
                        );
                    }
                }
            }
        }
        let mut failures = CleanupFailures::default();
        walk(&path, &progress, &subtask, total, &mut failures);
        publish_cleanup_progress(
            &subtask,
            progress
                .load(Ordering::Relaxed)
                .try_into()
                .unwrap_or(u32::MAX),
            total,
        );
        if failures.count == 0 {
            Ok(())
        } else {
            Err(std::io::Error::other(failures))
        }
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vtask::Progress;

    /// Every table `cleanup_caches`'s `gdlauncher` selection wipes, besides
    /// `HTTPCache` (seeded and asserted separately below). Duplicated here
    /// rather than shared with `cleanup_caches`'s own `TABLES` const (which is
    /// local to its background task) so this test proves the behavior from
    /// the outside, the same way a caller observes it.
    const OTHER_WIPED_TABLES: &[&str] = &[
        "CurseForgeModImageCache",
        "ModrinthModImageCache",
        "LocalModImageCache",
        "CurseForgeModpackImageCache",
        "ModrinthModpackImageCache",
        "CurseForgeModCache",
        "ModrinthModCache",
        "CurseForgeModpackCache",
        "ModrinthModpackCache",
        "VersionInfoCache",
        "PartialVersionInfoCache",
        "LwjglMetaCache",
        "AssetsMetaCache",
    ];

    /// Waits for `task_id` to finish, failing loudly if it errors or never
    /// completes. `wait_with_log` returns as soon as it observes completion,
    /// but the task-list removal that makes `get_task` start returning `None`
    /// runs on a separate task listening to the same signal, so this polls
    /// briefly afterward rather than racing it.
    async fn wait_for_task(app: &crate::managers::App, task_id: VisualTaskId) {
        app.task_manager().wait_with_log(task_id).await.unwrap();
        for _ in 0..50 {
            match app.task_manager().get_task(task_id).await {
                None => return,
                Some(t) => {
                    if let Progress::Failed(e) = &t.progress {
                        panic!("cache cleanup task failed: {e}");
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            }
        }
        panic!("cache cleanup task did not finish within the poll window");
    }

    /// Regression test for the cache-cleanup DB phase (dynamic chunked
    /// `DELETE` + `count_table`, both checker-exempt `DynamicQuery` uses):
    /// seeds a real cache table, runs `cleanup_caches`, and asserts the rows
    /// are actually deleted and every count along the way is correct. No bug
    /// is expected here — this closes a pure regression-protection gap.
    #[tokio::test]
    async fn cleanup_caches_deletes_seeded_rows_and_counts_match() {
        let app = crate::setup_managers_for_test().await;

        for i in 0..3u8 {
            carbon_repos::repos::http_cache::replace_cached(
                &app.db,
                format!("https://example.test/{i}"),
                200,
                vec![i],
                None,
                None,
                None,
            )
            .await
            .unwrap();
        }
        assert_eq!(
            count_table(&app.db, "HTTPCache").await,
            3,
            "count_table must match the seeded row count before cleanup"
        );
        for table in OTHER_WIPED_TABLES {
            assert_eq!(
                count_table(&app.db, table).await,
                0,
                "{table} must start empty in a fresh test database"
            );
        }

        let task_id = app
            .settings_manager()
            .cleanup_caches(CacheCleanupSelection {
                gdlauncher: true,
                minecraft: false,
            })
            .await
            .unwrap();
        wait_for_task(&app, task_id).await;

        assert_eq!(
            count_table(&app.db, "HTTPCache").await,
            0,
            "cleanup_caches must delete every seeded HTTPCache row"
        );
        // Every other wiped table stays empty: the chunked delete's fast path
        // (COUNT = 0 => loop breaks without issuing a DELETE) must not error
        // even though these tables were never seeded.
        for table in OTHER_WIPED_TABLES {
            assert_eq!(
                count_table(&app.db, table).await,
                0,
                "{table} must remain empty after cleanup"
            );
        }
    }
}

#[cfg(all(test, unix))]
mod task21_cleanup_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[tokio::test]
    async fn task21_cleanup_denied_file_is_not_counted_as_removed() {
        let dir = tempfile::tempdir().unwrap();
        let denied = dir.path().join("denied");
        std::fs::create_dir(&denied).unwrap();
        let file = denied.join("kept.cache");
        std::fs::write(&file, b"keep").unwrap();
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o555)).unwrap();
        let probe = std::fs::remove_file(&file);
        assert_eq!(
            probe.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied,
            "permission denial must be effective under runner user"
        );
        let task = VisualTask::new(Translation::CacheCleanup);
        let subtask = Arc::new(task.subtask(Translation::CacheCleanup));
        let progress = Arc::new(AtomicU64::new(0));
        let result =
            clear_dir_with_progress(denied.clone(), progress.clone(), subtask.clone(), Some(1))
                .await;
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(file.exists());
        assert!(
            result.is_err(),
            "failed unlink must surface the affected path"
        );
        assert_eq!(progress.load(Ordering::Relaxed), 0);
    }
    fn db_fixture() -> (
        tempfile::TempDir,
        carbon_repos::db_exec::Db,
        carbon_rt_path::RuntimePath,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        let mut c = rusqlite::Connection::open(&path).unwrap();
        carbon_repos::get_migrations().0.to_latest(&mut c).unwrap();
        drop(c);
        let db = carbon_repos::db_exec::Db::open(&path, 1, true).unwrap();
        let runtime = carbon_rt_path::RuntimePath::new(dir.path().join("runtime"));
        (dir, db, runtime)
    }
    #[tokio::test]
    async fn task21_cleanup_partial_db_error_retains_failure_and_retry_preserves_metadata() {
        use carbon_repos::db_exec::WriteAccess;
        let (_dir, db, runtime) = db_fixture();
        db.write(|c|Ok(c.execute_batch("INSERT INTO HTTPCache(url,status_code,data) VALUES('synthetic',200,x'01');
            INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES('kept',1,x'01',x'02','forge');
            CREATE TRIGGER denied_delete BEFORE DELETE ON HTTPCache BEGIN SELECT RAISE(ABORT,'denied cache table');END;")?)).await.unwrap();
        let task = VisualTask::new(Translation::CacheCleanup);
        task.require_explicit_completion();
        let observer = task.clone();
        let work = Arc::new(task.subtask(Translation::CacheCleanup));
        let running = Arc::new(AtomicBool::new(true));
        let lease = CleanupLease(running.clone());
        let result = run_cache_cleanup(
            &db,
            &runtime,
            &CacheCleanupSelection {
                gdlauncher: true,
                minecraft: false,
            },
            work,
            None,
        )
        .await;
        assert!(
            result
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("HTTPCache")
        );
        finish_cache_cleanup(task, result).await;
        drop(lease);
        assert!(!running.load(Ordering::SeqCst));
        assert!(matches!(
            observer.make_domain_task(VisualTaskId(1)).await.progress,
            crate::domain::vtask::Progress::Failed(_)
        ));
        assert_eq!(try_count_table(&db, "HTTPCache").await.unwrap(), 1);
        assert_eq!(try_count_table(&db, "ModMetadata").await.unwrap(), 1);
        db.write(|c| Ok(c.execute_batch("DROP TRIGGER denied_delete")?))
            .await
            .unwrap();
        let retry = VisualTask::new(Translation::CacheCleanup);
        run_cache_cleanup(
            &db,
            &runtime,
            &CacheCleanupSelection {
                gdlauncher: true,
                minecraft: false,
            },
            Arc::new(retry.subtask(Translation::CacheCleanup)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(try_count_table(&db, "HTTPCache").await.unwrap(), 0);
        assert_eq!(try_count_table(&db, "ModMetadata").await.unwrap(), 1);
    }
    #[tokio::test]
    async fn task21_cleanup_vacuum_failure_and_selected_scope_controls() {
        use carbon_repos::db_exec::WriteAccess;
        let (_dir, db, runtime) = db_fixture();
        let temp = runtime.get_temp().to_path();
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("kept"), b"keep").unwrap();
        let assets = runtime.get_assets().to_path();
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("deleted"), b"delete").unwrap();
        let task = VisualTask::new(Translation::CacheCleanup);
        let work = Arc::new(task.subtask(Translation::CacheCleanup));
        run_cache_cleanup(
            &db,
            &runtime,
            &CacheCleanupSelection {
                gdlauncher: false,
                minecraft: true,
            },
            work,
            None,
        )
        .await
        .unwrap();
        assert!(temp.join("kept").exists());
        assert!(!assets.join("deleted").exists());
        // An actual writer transaction makes SQLite refuse VACUUM.
        db.write(|c| Ok(c.execute_batch("BEGIN IMMEDIATE")?))
            .await
            .unwrap();
        let task = VisualTask::new(Translation::CacheCleanup);
        let observed = task.clone();
        let result = run_cache_cleanup(
            &db,
            &runtime,
            &CacheCleanupSelection {
                gdlauncher: true,
                minecraft: false,
            },
            Arc::new(task.subtask(Translation::CacheCleanup)),
            None,
        )
        .await;
        db.write(|c| Ok(c.execute_batch("ROLLBACK")?))
            .await
            .unwrap();
        assert!(result.as_ref().unwrap_err().to_string().contains("VACUUM"));
        finish_cache_cleanup(task, result).await;
        assert!(matches!(
            observed.make_domain_task(VisualTaskId(1)).await.progress,
            crate::domain::vtask::Progress::Failed(_)
        ));
    }
    #[tokio::test]
    async fn task21_cleanup_writable_missing_and_mixed_directory_controls() {
        let dir = tempfile::tempdir().unwrap();
        let task = VisualTask::new(Translation::CacheCleanup);
        let work = Arc::new(task.subtask(Translation::CacheCleanup));
        let progress = Arc::new(AtomicU64::new(0));
        clear_dir_with_progress(
            dir.path().join("absent"),
            progress.clone(),
            work.clone(),
            Some(1),
        )
        .await
        .unwrap();
        let child = dir.path().join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("deleted"), b"delete").unwrap();
        clear_dir_with_progress(dir.path().to_path_buf(), progress.clone(), work, Some(1))
            .await
            .unwrap();
        assert_eq!(progress.load(Ordering::Relaxed), 1);
        assert!(!child.exists());
        assert_eq!(task.progress_float().await, 1.0);
    }

    async fn failed_count_case(healthy_files: bool, weighted_vacuum: bool) {
        let (_dir, db, runtime) = db_fixture();
        let assets = runtime.get_assets().to_path();
        std::fs::create_dir_all(&assets).unwrap();
        let kept = assets.join("kept.cache");
        std::fs::write(&kept, b"keep").unwrap();
        let libraries = runtime.get_libraries().to_path();
        if healthy_files {
            std::fs::create_dir_all(&libraries).unwrap();
            std::fs::write(libraries.join("removed.cache"), b"remove").unwrap();
        }
        std::fs::set_permissions(&assets, std::fs::Permissions::from_mode(0o000)).unwrap();
        let probe = std::fs::read_dir(&assets);
        let task = VisualTask::new(Translation::CacheCleanup);
        task.require_explicit_completion();
        let work = Arc::new(task.subtask(Translation::CacheCleanup));
        let vacuum = if weighted_vacuum {
            work.set_weight(0.9);
            let vacuum = task.subtask(Translation::CacheCleanup);
            vacuum.set_weight(0.1);
            Some(vacuum)
        } else {
            None
        };
        let result = run_cache_cleanup(
            &db,
            &runtime,
            &CacheCleanupSelection {
                gdlauncher: weighted_vacuum,
                minecraft: true,
            },
            work,
            vacuum,
        )
        .await;
        std::fs::set_permissions(&assets, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            probe.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(kept.exists());
        assert!(
            !libraries.join("removed.cache").exists(),
            "healthy scope must still be cleaned"
        );
        assert!(
            result
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains(&assets.display().to_string())
        );
        assert!(
            !task
                .make_domain_task(VisualTaskId(1))
                .await
                .active_subtasks
                .is_empty(),
            "an omitted failed count must not complete required work"
        );
        assert!(
            task.progress_float().await < 1.0,
            "unknown count must not publish 100%"
        );
        if weighted_vacuum {
            assert!(
                (task.progress_float().await - 0.1).abs() < 0.0001,
                "successful VACUUM retains only its own weight"
            );
        }
        let observer = task.clone();
        finish_cache_cleanup(task, result).await;
        assert!(matches!(
            observer.make_domain_task(VisualTaskId(1)).await.progress,
            crate::domain::vtask::Progress::Failed(_)
        ));
    }
    #[tokio::test]
    async fn task21_r1_failed_count_zero_known_units_stays_incomplete() {
        failed_count_case(false, false).await;
    }
    #[tokio::test]
    async fn task21_r1_failed_count_with_removed_known_subtotal_stays_incomplete() {
        failed_count_case(true, false).await;
    }
    #[tokio::test]
    async fn task21_r1_failed_count_preserves_weighted_vacuum() {
        failed_count_case(true, true).await;
    }
    #[tokio::test]
    async fn task21_r1_empty_and_counted_worker_success_controls() {
        for populated in [false, true] {
            let (_dir, db, runtime) = db_fixture();
            let assets = runtime.get_assets().to_path();
            if populated {
                std::fs::create_dir_all(&assets).unwrap();
                std::fs::write(assets.join("removed.cache"), b"remove").unwrap();
            }
            let task = VisualTask::new(Translation::CacheCleanup);
            let work = Arc::new(task.subtask(Translation::CacheCleanup));
            run_cache_cleanup(
                &db,
                &runtime,
                &CacheCleanupSelection {
                    gdlauncher: false,
                    minecraft: true,
                },
                work.clone(),
                None,
            )
            .await
            .unwrap();
            assert!(!assets.join("removed.cache").exists());
            assert!(
                task.make_domain_task(VisualTaskId(1))
                    .await
                    .active_subtasks
                    .is_empty()
            );
            assert_eq!(task.progress_float().await, 1.0);
        }
    }
}
