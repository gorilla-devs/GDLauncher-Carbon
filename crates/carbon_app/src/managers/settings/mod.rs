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
    cleanup_running: AtomicBool,
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
            cleanup_running: AtomicBool::new(false),
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

        tokio::spawn(async move {
            info!("Starting cache cleanup with selection: {selection:?}");

            // Release the guard no matter how we exit the task.
            struct ReleaseGuard(std::sync::Arc<crate::managers::AppInner>);
            impl Drop for ReleaseGuard {
                fn drop(&mut self) {
                    self.0
                        .settings_manager()
                        .cleanup_running
                        .store(false, Ordering::SeqCst);
                }
            }
            let _guard = ReleaseGuard(app.clone());

            // Trim launcher logs to the last LOGS_KEEP entries before
            // anything else. This is intentionally NOT routed through the
            // wholesale disk-wipe path: recent logs are useful for
            // debugging the previous few launches, so we keep them
            // regardless of which caches the user picked. The
            // op is bounded (a handful of small files) so it doesn't
            // need progress reporting.
            const LOGS_KEEP: usize = 10;
            if selection.gdlauncher {
                let logs_path = runtime_path.join("__gdl_logs__");
                tokio::task::spawn_blocking(move || {
                    crate::logger::cleanup_old_logs(&logs_path, LOGS_KEEP)
                })
                .await
                .ok();
            }

            // Build disk job list. Order matters only for the user-visible
            // log; per-job execution is sequential so progress increments
            // monotonically rather than racing.
            let mut disk_jobs: Vec<(&'static str, PathBuf)> = Vec::new();
            if selection.gdlauncher {
                disk_jobs.push(("temp files", runtime_path.get_temp().to_path()));
            }
            if selection.minecraft {
                disk_jobs.push(("Minecraft assets", runtime_path.get_assets().to_path()));
                disk_jobs.push((
                    "Minecraft libraries",
                    runtime_path.get_libraries().to_path(),
                ));
                disk_jobs.push(("native libraries", runtime_path.join("natives")));
            }

            // Tables to wipe on `gdlauncher`. Listed leaf-to-root so cascade-
            // or-not behaves the same regardless of `PRAGMA foreign_keys`.
            // ModMetadata is deliberately NOT here: it backs installed
            // mods and wiping it would break instance state.
            const TABLES: &[&str] = &[
                "HTTPCache",
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

            // ---- Pre-count work units ----
            // One walk per disk dir, one COUNT per table. Both scan only
            // metadata pages (file entries, rowid index), not row payloads.
            let mut disk_total: u64 = 0;
            for (_, path) in &disk_jobs {
                disk_total = disk_total.saturating_add(count_files_blocking(path.clone()).await);
            }
            let mut db_total: u64 = 0;
            if selection.gdlauncher {
                for table in TABLES {
                    db_total = db_total.saturating_add(count_table(&app.db, table).await);
                }
            }
            let total_units = disk_total + db_total;

            // Cap to u32 for the Subtask wire format. >4B units would
            // mean 4B+ files or rows which we'd never realistically hit;
            // saturating_as keeps the math sane if it ever does.
            let total_u32: u32 = total_units.try_into().unwrap_or(u32::MAX);
            // Replace the placeholder denominator with the real total now
            // that pre-count is done.
            work.update_items(0, total_u32);

            let progress = Arc::new(AtomicU64::new(0));

            // ---- Disk phase ----
            // Sequential per dir. Progress increments per-file, throttled
            // inside the helper so we don't spam the watch channel on
            // big trees.
            for (label, path) in disk_jobs {
                if let Err(e) =
                    clear_dir_with_progress(path.clone(), progress.clone(), work.clone(), total_u32)
                        .await
                {
                    warn!("Failed to clear {label} at {path:?}: {e}");
                }
            }

            // ---- DB phase ----
            if selection.gdlauncher {
                // Chunk size picked to balance per-statement overhead
                // (~1 ms parse/plan + WAL commit) against progress
                // granularity. 200 rows ≈ 10–20 ms per chunk on heavily
                // indexed tables — frequent enough that the bar never
                // appears frozen on multi-million-row HTTPCache wipes,
                // small enough that the extra parse cost is in the noise.
                const CHUNK: u32 = 200;

                for table in TABLES {
                    let sql = format!(
                        "DELETE FROM {table} WHERE rowid IN (SELECT rowid FROM {table} LIMIT {CHUNK})"
                    );
                    loop {
                        let dq = carbon_repos::registry::DynamicQuery {
                            sql: sql.clone(),
                            params: vec![],
                        };
                        match app.db.write(move |conn| Ok(dq.execute(&conn)?)).await {
                            Ok(0) => break,
                            Ok(n) => {
                                let new =
                                    progress.fetch_add(n as u64, Ordering::Relaxed) + n as u64;
                                work.update_items(new.try_into().unwrap_or(u32::MAX), total_u32);
                            }
                            Err(e) => {
                                warn!("Failed to clear {table}: {e}");
                                break;
                            }
                        }
                    }
                }
                work.complete_items();

                // VACUUM reclaims the freelist pages the DELETEs created.
                // Without it the file stays its current size on disk.
                if let Some(vs) = vacuum_subtask.as_ref() {
                    vs.start_opaque();
                }
                if let Err(e) = app
                    .db
                    .write(|conn| Ok(carbon_repos::db_exec::WriteAccess::execute_batch(&conn, "VACUUM")?))
                    .await
                {
                    error!("VACUUM failed: {e}");
                    task.fail(anyhow!("Failed to reclaim cache space: {e}"))
                        .await;
                    return;
                }
                if let Some(vs) = vacuum_subtask.as_ref() {
                    vs.complete_opaque();
                }
            } else {
                // No DB phase: pin the work bar to 100% so the modal
                // doesn't briefly show stale "0%" between disk-done and
                // task-drop.
                work.complete_items();
            }

            info!("Cache cleanup complete");
            app.invalidate(GET_DB_SIZE, None);
        });

        Ok(task_id)
    }
}

/// COUNT(*) on a cache table. Uses the rowid index, so it scales with
/// row count not byte count — fast even on a multi-GB HTTPCache.
async fn count_table(db: &carbon_repos::db_exec::Db, table: &str) -> u64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let table = table.to_string();
    let dq = carbon_repos::registry::DynamicQuery {
        sql,
        params: vec![],
    };
    match db.read(move |conn| Ok(dq.query_scalar_i64(&conn)?)).await {
        Ok(n) => n.max(0) as u64,
        Err(e) => {
            warn!("Failed to count `{table}`: {e}");
            0
        }
    }
}

/// Count files (recursively) under `path`. Reads directory entries +
/// metadata only — bounded by file count, not file size. Missing dirs
/// return 0 silently.
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

async fn count_files_blocking(path: PathBuf) -> u64 {
    tokio::task::spawn_blocking(move || {
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
                    total = total.saturating_add(1);
                } else if meta.is_dir() {
                    total = total.saturating_add(walk(&entry.path()));
                }
            }
            total
        }
        if path.exists() { walk(&path) } else { 0 }
    })
    .await
    .unwrap_or(0)
}

/// Recursively delete files and subdirectories inside `path`, leaving
/// the directory itself in place, while incrementing `progress` per
/// file deleted and pushing throttled `update_items` calls onto
/// `subtask`. `total` is the same upper bound used to seed `subtask`'s
/// progress so all reports share a denominator.
async fn clear_dir_with_progress(
    path: PathBuf,
    progress: Arc<AtomicU64>,
    subtask: Arc<Subtask>,
    total: u32,
) -> std::io::Result<()> {
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(());
    }
    tokio::task::spawn_blocking(move || {
        // Update granularity. One notify per file would flood the
        // watch channel for huge trees; one per 25 keeps the bar
        // ticking smoothly even when removing a handful of large files
        // (where 100-file batches would visibly stall) without burning
        // CPU on assets/ trees with tens of thousands of tiny files.
        const PROGRESS_BATCH: u64 = 25;

        fn walk_delete(p: &Path, progress: &AtomicU64, subtask: &Subtask, total: u32) {
            let Ok(entries) = std::fs::read_dir(p) else {
                return;
            };
            for entry in entries.flatten() {
                let pp = entry.path();
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                if meta.is_dir() {
                    walk_delete(&pp, progress, subtask, total);
                    let _ = std::fs::remove_dir(&pp);
                } else {
                    let _ = std::fs::remove_file(&pp);
                    let n = progress.fetch_add(1, Ordering::Relaxed) + 1;
                    if n % PROGRESS_BATCH == 0 {
                        subtask.update_items(n.try_into().unwrap_or(u32::MAX), total);
                    }
                }
            }
        }
        walk_delete(&path, &progress, &subtask, total);
        // Final flush so the bar reflects the tail < PROGRESS_BATCH
        // worth of files we just removed without notifying about.
        let n = progress.load(Ordering::Relaxed);
        subtask.update_items(n.try_into().unwrap_or(u32::MAX), total);
        Ok::<_, std::io::Error>(())
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?
}
