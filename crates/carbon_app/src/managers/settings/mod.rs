use self::terms_and_privacy::TermsAndPrivacy;
use super::ManagerRef;
use crate::api::{
    keys::settings::*,
    settings::{CacheBreakdown, CacheCleanupSelection, FESettingsUpdate},
    translation::Translation,
};
use crate::domain::vtask::VisualTaskId;
use crate::managers::vtask::VisualTask;
use anyhow::{anyhow, bail};
use carbon_platforms::{ModChannelWithUsage, ModPlatform};
use carbon_repos::db::app_configuration;
use carbon_repos::pcr::raw;
use futures::future::join_all;
use itertools::Itertools;
use reqwest_middleware::ClientWithMiddleware;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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
    pub async fn get_settings(self) -> anyhow::Result<carbon_repos::db::app_configuration::Data> {
        self.app
            .prisma_client
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(anyhow!("Can't find this key"))
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_settings(self, incoming_settings: FESettingsUpdate) -> anyhow::Result<()> {
        let db = &self.app.prisma_client;
        let mut queries = vec![];

        if let Some(theme) = incoming_settings.theme {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::theme::set(theme.inner())],
            ));
        }

        if let Some(language) = incoming_settings.language {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::language::set(language.inner())],
            ));
        }

        if let Some(reduced_motion) = incoming_settings.reduced_motion {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::reduced_motion::set(
                    reduced_motion.inner(),
                )],
            ));
        }

        if let Some(discord_integration) = incoming_settings.discord_integration {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::discord_integration::set(
                    discord_integration.inner(),
                )],
            ));
        }

        if let Some(release_channel) = incoming_settings.release_channel {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::release_channel::set(
                    release_channel.inner().into(),
                )],
            ));
        }

        if let Some(launcher_action_on_game_launch) =
            incoming_settings.launcher_action_on_game_launch
        {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::launcher_action_on_game_launch::set(
                    launcher_action_on_game_launch.inner().into(),
                )],
            ));
        }

        if let Some(show_app_close_warning) = incoming_settings.show_app_close_warning.clone() {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::show_app_close_warning::set(
                    show_app_close_warning.inner(),
                )],
            ));
        }

        if let Some(concurrent_downloads) = incoming_settings.concurrent_downloads {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::concurrent_downloads::set(
                    concurrent_downloads.inner(),
                )],
            ));
        }

        if let Some(download_dependencies) = incoming_settings.download_dependencies {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::download_dependencies::set(
                    download_dependencies.inner(),
                )],
            ));
        }

        if let Some(show_featured) = incoming_settings.show_featured {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::show_featured::set(show_featured.inner())],
            ));
        }

        // instances_sort_by: Option<Option<InstancesSortBy>>
        // Outer Option = whether to update, Inner Option = null (manual order) or Some(sort criteria)
        if let Some(sort_by) = incoming_settings.instances_sort_by {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::instances_sort_by::set(
                    sort_by.inner().map(Into::into),
                )],
            ));
        }

        if let Some(instances_sort_by_asc) = incoming_settings.instances_sort_by_asc {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::instances_sort_by_asc::set(
                    instances_sort_by_asc.inner(),
                )],
            ));
        }

        // instances_group_by: Option<Option<InstancesGroupBy>>
        // Outer Option = whether to update, Inner Option = null (folders mode) or Some(group criteria)
        if let Some(instances_group_by) = incoming_settings.instances_group_by {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::instances_group_by::set(
                    instances_group_by.inner().map(Into::into),
                )],
            ));
        }

        if let Some(instances_group_by_asc) = incoming_settings.instances_group_by_asc {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::instances_group_by_asc::set(
                    instances_group_by_asc.inner(),
                )],
            ));
        }

        if let Some(instances_duplicate_favorites) = incoming_settings.instances_duplicate_favorites
        {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::instances_duplicate_favorites::set(
                    instances_duplicate_favorites.inner(),
                )],
            ));
        }

        if let Some(instances_tile_size) = incoming_settings.instances_tile_size {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::instances_tile_size::set(
                    instances_tile_size.inner().into(),
                )],
            ));
        }

        if let Some(deletion_through_recycle_bin) = incoming_settings.deletion_through_recycle_bin {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::deletion_through_recycle_bin::set(
                    deletion_through_recycle_bin.inner(),
                )],
            ));
        }

        if let Some(xmx) = incoming_settings.xmx {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::xmx::set(xmx.inner())],
            ));
        }

        if let Some(xms) = incoming_settings.xms {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::xms::set(xms.inner())],
            ));
        }

        if let Some(game_resolution) = incoming_settings.game_resolution {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::game_resolution::set(
                    game_resolution.inner().map(Into::into),
                )],
            ));
        }

        if let Some(java_custom_args) = incoming_settings.java_custom_args {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::java_custom_args::set(
                    java_custom_args.inner(),
                )],
            ));
        }

        if let Some(pre_launch_hook) = incoming_settings.pre_launch_hook {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::pre_launch_hook::set(
                    pre_launch_hook.inner(),
                )],
            ));
        }

        if let Some(post_exit_hook) = incoming_settings.post_exit_hook {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::post_exit_hook::set(
                    post_exit_hook.inner(),
                )],
            ));
        }

        if let Some(wrapper_command) = incoming_settings.wrapper_command {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::wrapper_command::set(
                    wrapper_command.inner(),
                )],
            ));
        }

        if let Some(auto_manage_java_system_profiles) =
            incoming_settings.auto_manage_java_system_profiles.as_ref()
        {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::auto_manage_java_system_profiles::set(
                    auto_manage_java_system_profiles.clone().inner(),
                )],
            ));
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

            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![
                    app_configuration::mod_platform_blacklist::set(platform_blacklist),
                    app_configuration::mod_channels::set(channels_str),
                ],
            ));
        }

        if let Some(terms_and_privacy_accepted) = incoming_settings.terms_and_privacy_accepted {
            let terms_and_privacy_accepted = terms_and_privacy_accepted.inner();
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::terms_and_privacy_accepted::set(
                    terms_and_privacy_accepted,
                )],
            ));

            // We default to empty value in case our APIs fail so we don't block the user.
            // We are gonna ask again on next run anyway once the APIs are back up
            let latest_consent_sha = self
                .latest_consent_checksum
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();

            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![
                    app_configuration::terms_and_privacy_accepted::set(true),
                    app_configuration::terms_and_privacy_accepted_checksum::set(Some(
                        latest_consent_sha,
                    )),
                ],
            ));
        }

        if !queries.is_empty() {
            db._batch(queries).await?;
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
                super::java::scan_and_sync::sync_system_java_profiles(db).await?;
            }
        }

        Ok(())
    }

    pub async fn set(self, value: app_configuration::SetParam) -> anyhow::Result<()> {
        self.app
            .prisma_client
            .app_configuration()
            .update(app_configuration::id::equals(0), vec![value])
            .exec()
            .await?;

        Ok(())
    }

    /// Returns the total clearable cache footprint: the sum of every item
    /// the cleanup dialog can clear. Uses the per-table row-bytes
    /// approximation (same as the modal's items) so the settings row, the
    /// modal header, and the sum of items all agree.
    ///
    /// Intentionally excludes DB overhead (indexes, non-cache tables,
    /// freelist, WAL) — those aren't cache content and the user can't
    /// selectively clear them. They do get reclaimed when the user clears
    /// everything + VACUUM runs, so the actual freed space is a bit more
    /// than this number predicts (underpromise/overdeliver).
    ///
    /// Delegates to `get_cache_breakdown` so the settings row and the modal
    /// share a single cached query result — open the modal after visiting
    /// settings and the breakdown renders instantly.
    pub async fn get_total_cache_size(self) -> f64 {
        self.get_cache_breakdown().await.total_size
    }

    /// Returns just the SQLite DB file footprint (gdl_conf.db + WAL).
    /// Separate from `get_total_cache_size` because the bloat banner uses
    /// this specifically — the banner's job is to flag *DB* bloat (the
    /// original HTTPCache bug), not legitimate disk usage like `assets/`
    /// and `libraries/`. Cheap: just two `stat` calls.
    pub async fn get_db_size(self) -> f64 {
        db_total_bytes(&self.runtime_path).await
    }

    /// Returns approximate sizes (in bytes) of every clearable cache, both
    /// in the SQLite DB and on disk. Used to populate the cleanup dialog so
    /// users can see what they'd reclaim before clicking. Sizes are
    /// approximations — DB sizes sum `length(col)` per row (close to the
    /// actual page footprint but excludes index pages and free pages); disk
    /// sizes walk and sum file lengths.
    pub async fn get_cache_breakdown(self) -> CacheBreakdown {
        let prisma = &self.app.prisma_client;

        async fn table_bytes(prisma: &carbon_repos::db::PrismaClient, sql: &str) -> f64 {
            #[derive(serde::Deserialize)]
            struct Row {
                bytes: Option<i64>,
            }
            match prisma
                ._query_raw::<Row>(carbon_repos::pcr::raw::Raw::new(sql, vec![]))
                .exec()
                .await
            {
                Ok(rows) => rows
                    .into_iter()
                    .next()
                    .and_then(|r| r.bytes)
                    .map(|b| b as f64)
                    .unwrap_or(0.0),
                Err(e) => {
                    warn!("Failed to size cache via `{sql}`: {e}");
                    0.0
                }
            }
        }

        // Each query sums per-row length() across all relevant columns.
        // For tables with binary blob columns, that's the dominant cost.
        let (
            http_cache,
            curseforge_mod_metadata,
            curseforge_mod_icons,
            curseforge_modpack_metadata,
            curseforge_modpack_icons,
            modrinth_mod_metadata,
            modrinth_mod_icons,
            modrinth_modpack_metadata,
            modrinth_modpack_icons,
            local_mod_icons,
            mc_version_manifests,
            modloader_versions,
            lwjgl_configs,
            asset_indices,
        ) = tokio::join!(
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(url) + length(data) + IFNULL(length(etag), 0) + IFNULL(length(lastModified), 0)), 0) AS bytes FROM HTTPCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(metadataId) + length(name) + length(version) + length(urlslug) + length(summary) + length(authors) + length(updatePaths)), 0) AS bytes FROM CurseForgeModCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(metadataId) + length(url) + IFNULL(length(data), 0)), 0) AS bytes FROM CurseForgeModImageCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(modpackName) + length(versionName) + length(urlSlug)), 0) AS bytes FROM CurseForgeModpackCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(url) + IFNULL(length(data), 0)), 0) AS bytes FROM CurseForgeModpackImageCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(metadataId) + length(sha512) + length(projectId) + length(versionId) + length(title) + length(version) + length(urlslug) + length(description) + length(authors) + length(updatePaths) + length(filename) + length(fileUrl)), 0) AS bytes FROM ModrinthModCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(metadataId) + length(url) + IFNULL(length(data), 0)), 0) AS bytes FROM ModrinthModImageCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(projectId) + length(versionId) + length(modpackName) + length(versionName) + length(urlSlug)), 0) AS bytes FROM ModrinthModpackCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(projectId) + length(versionId) + length(url) + IFNULL(length(data), 0)), 0) AS bytes FROM ModrinthModpackImageCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(metadataId) + length(data)), 0) AS bytes FROM LocalModImageCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(id) + length(versionInfo)), 0) AS bytes FROM VersionInfoCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(id) + length(partialVersionInfo)), 0) AS bytes FROM PartialVersionInfoCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(id) + length(lwjgl)), 0) AS bytes FROM LwjglMetaCache"
            ),
            table_bytes(
                prisma,
                "SELECT IFNULL(SUM(length(id) + length(assetsIndex)), 0) AS bytes FROM AssetsMetaCache"
            ),
        );

        let runtime_path = self.runtime_path.clone();
        let (temp_files, old_logs, mc_assets, mc_libraries, mc_natives) = tokio::join!(
            dir_size(runtime_path.get_temp().to_path()),
            dir_size(runtime_path.join("__gdl_logs__")),
            dir_size(runtime_path.get_assets().to_path()),
            dir_size(runtime_path.get_libraries().to_path()),
            dir_size(runtime_path.join("natives")),
        );

        // Sum of every clearable item — the same total the user sees when
        // they tick all the checkboxes. Per-table row-data bytes (not DB
        // file stat), so indexes / freelist / WAL / non-cache tables like
        // ModMetadata aren't counted. Keeps the settings row, modal
        // header, item sums, and action button all consistent.
        let total_size = http_cache
            + curseforge_mod_metadata
            + curseforge_mod_icons
            + curseforge_modpack_metadata
            + curseforge_modpack_icons
            + modrinth_mod_metadata
            + modrinth_mod_icons
            + modrinth_modpack_metadata
            + modrinth_modpack_icons
            + local_mod_icons
            + mc_version_manifests
            + modloader_versions
            + lwjgl_configs
            + asset_indices
            + temp_files
            + old_logs
            + mc_assets
            + mc_libraries
            + mc_natives;

        CacheBreakdown {
            http_cache,
            curseforge_mod_metadata,
            curseforge_mod_icons,
            curseforge_modpack_metadata,
            curseforge_modpack_icons,
            modrinth_mod_metadata,
            modrinth_mod_icons,
            modrinth_modpack_metadata,
            modrinth_modpack_icons,
            local_mod_icons,
            mc_version_manifests,
            modloader_versions,
            lwjgl_configs,
            asset_indices,
            temp_files,
            old_logs,
            mc_assets,
            mc_libraries,
            mc_natives,
            total_size,
        }
    }

    /// Spawns a background task that clears every cache marked `true` in
    /// `selection`. Order of operations:
    ///   1. Disk caches first, in parallel (they're independent).
    ///   2. Per-table DB DELETEs, sequentially (Prisma's connection_limit=1
    ///      would serialize them anyway; ordering image caches before their
    ///      FK parents avoids redundant cascade work).
    ///   3. A single `VACUUM` at the end iff any DB cache was selected. SQLite
    ///      atomically swaps the file, so the Prisma client survives.
    ///
    /// Rejects with an error if another cleanup is already running — we don't
    /// want two tasks contending on the same DB lock or competing VACUUMs.
    ///
    /// Best-effort per-step: individual failures are logged and don't abort
    /// the rest of the run.
    pub async fn cleanup_caches(
        self,
        selection: CacheCleanupSelection,
    ) -> anyhow::Result<VisualTaskId> {
        // Single-writer guard. `compare_exchange` atomically flips false→true
        // and returns Err if another cleanup already set it.
        if self
            .cleanup_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            bail!("A cache cleanup is already in progress");
        }

        let task = VisualTask::new(Translation::CacheCleanup);
        let task_id = self.app.task_manager().spawn_task(&task).await;

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

            // ---- Disk caches (parallel) ----
            let mut disk_jobs: Vec<(&'static str, PathBuf)> = Vec::new();
            if selection.temp_files {
                disk_jobs.push(("temp files", runtime_path.get_temp().to_path()));
            }
            if selection.old_logs {
                disk_jobs.push(("old logs", runtime_path.join("__gdl_logs__")));
            }
            if selection.mc_assets {
                disk_jobs.push(("Minecraft assets", runtime_path.get_assets().to_path()));
            }
            if selection.mc_libraries {
                disk_jobs.push((
                    "Minecraft libraries",
                    runtime_path.get_libraries().to_path(),
                ));
            }
            if selection.mc_natives {
                disk_jobs.push(("native libraries", runtime_path.join("natives")));
            }

            let disk_futures = disk_jobs.into_iter().map(|(label, path)| {
                let subtask = task.subtask(Translation::CacheCleanupClearingDisk);
                async move {
                    subtask.start_opaque();
                    if let Err(e) = clear_dir_contents(&path).await {
                        warn!("Failed to clear {label} at {path:?}: {e}");
                    }
                    subtask.complete_opaque();
                }
            });
            join_all(disk_futures).await;

            // ---- DB tables ----
            // Each entry: (sql, label, was_selected). Cascade-aware order:
            // image caches first so their FK parents can be wiped after.
            let db_targets: Vec<(&'static str, &'static str, bool)> = vec![
                ("DELETE FROM HTTPCache", "HTTPCache", selection.http_cache),
                (
                    "DELETE FROM CurseForgeModImageCache",
                    "CurseForgeModImageCache",
                    selection.curseforge_mod_icons,
                ),
                (
                    "DELETE FROM CurseForgeModpackImageCache",
                    "CurseForgeModpackImageCache",
                    selection.curseforge_modpack_icons,
                ),
                (
                    "DELETE FROM ModrinthModImageCache",
                    "ModrinthModImageCache",
                    selection.modrinth_mod_icons,
                ),
                (
                    "DELETE FROM ModrinthModpackImageCache",
                    "ModrinthModpackImageCache",
                    selection.modrinth_modpack_icons,
                ),
                (
                    "DELETE FROM LocalModImageCache",
                    "LocalModImageCache",
                    selection.local_mod_icons,
                ),
                (
                    "DELETE FROM CurseForgeModCache",
                    "CurseForgeModCache",
                    selection.curseforge_mod_metadata,
                ),
                (
                    "DELETE FROM CurseForgeModpackCache",
                    "CurseForgeModpackCache",
                    selection.curseforge_modpack_metadata,
                ),
                (
                    "DELETE FROM ModrinthModCache",
                    "ModrinthModCache",
                    selection.modrinth_mod_metadata,
                ),
                (
                    "DELETE FROM ModrinthModpackCache",
                    "ModrinthModpackCache",
                    selection.modrinth_modpack_metadata,
                ),
                (
                    "DELETE FROM VersionInfoCache",
                    "VersionInfoCache",
                    selection.mc_version_manifests,
                ),
                (
                    "DELETE FROM PartialVersionInfoCache",
                    "PartialVersionInfoCache",
                    selection.modloader_versions,
                ),
                (
                    "DELETE FROM LwjglMetaCache",
                    "LwjglMetaCache",
                    selection.lwjgl_configs,
                ),
                (
                    "DELETE FROM AssetsMetaCache",
                    "AssetsMetaCache",
                    selection.asset_indices,
                ),
            ];

            let mut any_db_cleared = false;
            for (sql, label, selected) in db_targets {
                if !selected {
                    continue;
                }
                let subtask = task.subtask(Translation::CacheCleanupClearingTable);
                subtask.start_opaque();
                match app
                    .prisma_client
                    ._execute_raw(carbon_repos::pcr::raw::Raw::new(sql, vec![]))
                    .exec()
                    .await
                {
                    Ok(n) => info!("Cleared {label}: {n} rows"),
                    Err(e) => warn!("Failed to clear {label}: {e}"),
                }
                subtask.complete_opaque();
                any_db_cleared = true;
            }

            // ---- VACUUM ----
            // Only worthwhile if we actually deleted DB rows. Without VACUUM
            // the file stays its current size (freelist grows); space is
            // reused on subsequent inserts, but disk size doesn't drop until
            // next user-triggered cleanup.
            if any_db_cleared {
                let vacuum_subtask = task.subtask(Translation::CacheCleanupVacuuming);
                vacuum_subtask.start_opaque();
                if let Err(e) = app.prisma_client._execute_raw(raw!("VACUUM")).exec().await {
                    error!("VACUUM failed: {e}");
                    task.fail(anyhow!("Failed to reclaim cache space: {e}"))
                        .await;
                    return;
                }
                vacuum_subtask.complete_opaque();

                // Index creation is intentionally NOT done here. It happens
                // exclusively on the next startup, gated on `db_file_bytes <=
                // 2 GB`. That keeps index-build cost (which holds the writer
                // lock) off the user-visible cleanup task and ensures it only
                // runs on a DB small enough to make the build fast.
            }

            info!("Cache cleanup complete");
            app.invalidate(GET_TOTAL_CACHE_SIZE, None);
            app.invalidate(GET_DB_SIZE, None);
            app.invalidate(GET_CACHE_BREAKDOWN, None);
        });

        Ok(task_id)
    }
}

/// Recursively sums file sizes under `path`. Missing dir → 0.
async fn dir_size(path: PathBuf) -> f64 {
    // Sync walk on a blocking thread — async file IO has no efficiency win
    // for size queries and adds complexity for recursion.
    tokio::task::spawn_blocking(move || {
        fn walk(p: &Path) -> u64 {
            let Ok(entries) = std::fs::read_dir(p) else {
                return 0;
            };
            let mut total = 0u64;
            for entry in entries.flatten() {
                let Ok(meta) = entry.metadata() else { continue };
                if meta.is_file() {
                    total = total.saturating_add(meta.len());
                } else if meta.is_dir() {
                    total = total.saturating_add(walk(&entry.path()));
                }
            }
            total
        }
        walk(&path) as f64
    })
    .await
    .unwrap_or(0.0)
}

/// Total on-disk DB footprint (gdl_conf.db + WAL sidecar).
async fn db_total_bytes(runtime_path: &carbon_rt_path::RuntimePath) -> f64 {
    let db_path = runtime_path.join("gdl_conf.db");
    let wal_path = runtime_path.join("gdl_conf.db-wal");
    let size_of =
        |p: PathBuf| async move { tokio::fs::metadata(&p).await.map(|m| m.len()).unwrap_or(0) };
    (size_of(db_path).await + size_of(wal_path).await) as f64
}

/// Recursively delete files and subdirectories inside `path`, but leave the
/// directory itself in place. Skips silently if the directory doesn't exist.
async fn clear_dir_contents(path: &Path) -> std::io::Result<()> {
    if !tokio::fs::try_exists(path).await.unwrap_or(false) {
        return Ok(());
    }
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let entries = std::fs::read_dir(&path)?;
        for entry in entries.flatten() {
            let p = entry.path();
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                let _ = std::fs::remove_dir_all(&p);
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
        Ok::<_, std::io::Error>(())
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?
}
