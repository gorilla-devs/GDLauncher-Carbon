//! Repository queries for the `ModFileCache` and `ServerModFileCache` tables.
//!
//! These caches track the mod/addon files installed on an instance or server,
//! linking each to a `ModMetadata` row (and, through it, to the CurseForge /
//! Modrinth platform caches and their images). Relation filters are expressed
//! as JOIN/EXISTS SQL and each related tree is read as ONE flat LEFT JOIN row.
//!
//! Both `id` columns are UUID primary keys with no DB-side default, so the
//! upsert fns generate `uuid::Uuid::new_v4()` themselves. Both `lastUpdatedAt`
//! freshness columns are written explicitly on every upsert/update — the
//! freshness lint in `tests/query_checker.rs` guards this.

use crate::db_error::DbResult;
use crate::db_exec::Db;
use crate::dbtypes::DbDateTime;
use crate::queries;
use chrono::{DateTime, FixedOffset};

// ---------------------------------------------------------------------------
// Plain cache rows (all columns)
// ---------------------------------------------------------------------------

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModFileCacheRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub instance_id: i32,
    pub filename: String,
    pub filesize: i32,
    pub enabled: bool,
    pub addon_type: String,
    pub metadata_id: String,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ServerModFileCacheRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub server_id: i32,
    pub filename: String,
    pub filesize: i32,
    pub enabled: bool,
    pub addon_type: String,
    pub metadata_id: String,
}

// ---------------------------------------------------------------------------
// Flat relation rows replacing the `.with(...)` trees
// ---------------------------------------------------------------------------

/// Flat row replacing the full `.with(metadata.with(logo).with(curseforge...)
/// .with(modrinth...))` tree for the instance mod list (`mods.rs:125`). Base
/// columns are non-`Option`; every `cf_*`/`mr_*` platform column is `Option`
/// (LEFT JOIN), and the three image-presence flags are booleans derived in SQL.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModFullRow {
    pub id: String,
    pub filename: String,
    pub enabled: bool,
    pub addon_type: String,
    pub filesize: i32,
    pub meta_id: String,
    pub meta_name: Option<String>,
    pub modid: Option<String>,
    pub meta_version: Option<String>,
    pub meta_description: Option<String>,
    pub meta_authors: Option<String>,
    pub modloaders: String,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub murmur2: i32,
    #[nullable(false)]
    pub has_local_image: bool,
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
    pub cf_name: Option<String>,
    pub cf_version: Option<String>,
    pub cf_urlslug: Option<String>,
    pub cf_summary: Option<String>,
    pub cf_authors: Option<String>,
    pub cf_release_type: Option<i32>,
    pub cf_update_paths: Option<String>,
    #[nullable(false)]
    pub has_cf_image: bool,
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
    pub mr_title: Option<String>,
    pub mr_version: Option<String>,
    pub mr_urlslug: Option<String>,
    pub mr_description: Option<String>,
    pub mr_authors: Option<String>,
    pub mr_release_type: Option<i32>,
    pub mr_update_paths: Option<String>,
    #[nullable(false)]
    pub has_mr_image: bool,
}

/// Flat row for the server addon list (`server/mod.rs:1724`). Thinner than
/// `ModFullRow`: the server consumer only needs the display name, platform
/// project ids, and image *presence* (relation existence, NOT image-data
/// presence — unlike the instance list).
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ServerAddonFullRow {
    pub id: String,
    pub filename: String,
    pub enabled: bool,
    pub addon_type: String,
    pub filesize: i32,
    pub meta_name: Option<String>,
    #[nullable(false)]
    pub has_local_image: bool,
    pub cf_project_id: Option<i32>,
    #[nullable(false)]
    pub has_cf_image: bool,
    pub mr_project_id: Option<String>,
    #[nullable(false)]
    pub has_mr_image: bool,
}

/// The three raw image blobs for a single file, in platform priority order.
/// Every column is `Option` (LEFT JOIN, and CF/MR image `data` is itself
/// nullable). Used by the mod-icon endpoints.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModIconRow {
    pub local_data: Option<Vec<u8>>,
    pub cf_data: Option<Vec<u8>>,
    pub mr_data: Option<Vec<u8>>,
}

/// `ModMetadata` hashes for files whose platform cache is missing/stale.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct RefreshMetaRow {
    pub metadata_id: String,
    pub murmur2: i32,
    pub sha512: Vec<u8>,
}

/// A CurseForge file+logo needing an icon (re)download.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct CfLogoRefreshRow {
    pub filename: String,
    pub project_id: i32,
    pub file_id: i32,
    pub metadata_id: String,
    pub url: String,
}

/// A Modrinth file+logo needing an icon (re)download.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrLogoRefreshRow {
    pub filename: String,
    pub project_id: String,
    pub version_id: String,
    pub metadata_id: String,
    pub url: String,
}

/// Bare id, for existence checks.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ExistsRow {
    pub id: String,
}

/// A file's CurseForge project/file ids (for update flows).
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModCfIdsRow {
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
}

/// A file's Modrinth project/version ids (for update flows).
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModMrIdsRow {
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
}

/// Both platforms' ids for a file (for the combined update-check flow).
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModUpdateIdsRow {
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
}

/// A metadata `modid`, for shader-loader detection.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModModidRow {
    pub modid: Option<String>,
}

/// A file's metadata + platform names/slugs for share-metadata export.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct InstanceModExportRow {
    pub filename: String,
    pub meta_name: Option<String>,
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
    pub cf_name: Option<String>,
    pub cf_urlslug: Option<String>,
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
    pub mr_title: Option<String>,
    pub mr_urlslug: Option<String>,
}

/// A file's CurseForge project/file ids for the CurseForge archive export.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct CfExportRow {
    pub filename: String,
    pub cf_project_id: Option<i32>,
    pub cf_file_id: Option<i32>,
}

/// A file's hashes + Modrinth file url for the Modrinth archive export.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrExportRow {
    pub filename: String,
    pub filesize: i32,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub mr_file_url: Option<String>,
}

/// A file's hashes + platform presence for the GDLauncher archive export.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct GdlExportRow {
    pub filename: String,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub murmur2: i32,
    pub cf_project_id: Option<i32>,
    pub mr_project_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Static queries
// ---------------------------------------------------------------------------

queries! {
    // --- ModFileCache: plain reads/writes ------------------------------------
    fn get_mod_file_cache_by_id(id: &str) -> Option<ModFileCacheRow> =
        "SELECT id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId
         FROM ModFileCache WHERE id = :id";
    fn get_mod_file_cache_by_instance_filename(instance_id: i32, filename: &str) -> Option<ModFileCacheRow> =
        "SELECT id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId
         FROM ModFileCache WHERE instanceId = :instance_id AND filename = :filename";
    fn get_mod_files_by_instance(instance_id: i32) -> Vec<ModFileCacheRow> =
        "SELECT id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId
         FROM ModFileCache WHERE instanceId = :instance_id";
    fn update_mod_file_enabled(id: &str, enabled: bool, updated_at: DbDateTime) -> usize =
        "UPDATE ModFileCache SET enabled = :enabled, lastUpdatedAt = :updated_at WHERE id = :id";
    fn delete_mod_file_cache_by_id(id: &str) -> usize =
        "DELETE FROM ModFileCache WHERE id = :id";
    fn delete_mod_file_cache_by_instance_filename(instance_id: i32, filename: &str) -> usize =
        "DELETE FROM ModFileCache WHERE instanceId = :instance_id AND filename = :filename";
    fn delete_mod_file_cache_by_instance(instance_id: i32) -> usize =
        "DELETE FROM ModFileCache WHERE instanceId = :instance_id";

    // --- ServerModFileCache: plain reads/writes ------------------------------
    fn get_server_mod_file_cache_by_id(id: &str) -> Option<ServerModFileCacheRow> =
        "SELECT id, lastUpdatedAt, serverId, filename, filesize, enabled, addonType, metadataId
         FROM ServerModFileCache WHERE id = :id";
    fn get_server_mod_files_by_server(server_id: i32) -> Vec<ServerModFileCacheRow> =
        "SELECT id, lastUpdatedAt, serverId, filename, filesize, enabled, addonType, metadataId
         FROM ServerModFileCache WHERE serverId = :server_id";
    fn update_server_mod_file_enabled(id: &str, enabled: bool, updated_at: DbDateTime) -> usize =
        "UPDATE ServerModFileCache SET enabled = :enabled, lastUpdatedAt = :updated_at WHERE id = :id";
    fn delete_server_mod_file_cache_by_id(id: &str) -> usize =
        "DELETE FROM ServerModFileCache WHERE id = :id";

    // --- instance mod list (full flat tree) ----------------------------------
    fn get_instance_mods_full(instance_id: i32) -> Vec<ModFullRow> =
        "SELECT f.id AS id, f.filename AS filename, f.enabled AS enabled,
                f.addonType AS addonType, f.filesize AS filesize,
                m.id AS metaId, m.name AS metaName, m.modid AS modid,
                m.version AS metaVersion, m.description AS metaDescription,
                m.authors AS metaAuthors, m.modloaders AS modloaders,
                m.sha512 AS sha512, m.sha1 AS sha1, m.murmur2 AS murmur2,
                (li.metadataId IS NOT NULL) AS hasLocalImage,
                c.projectId AS cfProjectId, c.fileId AS cfFileId, c.name AS cfName,
                c.version AS cfVersion, c.urlslug AS cfUrlslug, c.summary AS cfSummary,
                c.authors AS cfAuthors, c.releaseType AS cfReleaseType, c.updatePaths AS cfUpdatePaths,
                (ci.data IS NOT NULL) AS hasCfImage,
                r.projectId AS mrProjectId, r.versionId AS mrVersionId, r.title AS mrTitle,
                r.version AS mrVersion, r.urlslug AS mrUrlslug, r.description AS mrDescription,
                r.authors AS mrAuthors, r.releaseType AS mrReleaseType, r.updatePaths AS mrUpdatePaths,
                (ri.data IS NOT NULL) AS hasMrImage
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN LocalModImageCache li ON li.metadataId = m.id
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN CurseForgeModImageCache ci ON ci.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         LEFT JOIN ModrinthModImageCache ri ON ri.metadataId = m.id
         WHERE f.instanceId = :instance_id";

    // --- server addon list (thin flat tree) ----------------------------------
    fn get_server_mods_full(server_id: i32) -> Vec<ServerAddonFullRow> =
        "SELECT f.id AS id, f.filename AS filename, f.enabled AS enabled,
                f.addonType AS addonType, f.filesize AS filesize,
                m.name AS metaName,
                (li.metadataId IS NOT NULL) AS hasLocalImage,
                c.projectId AS cfProjectId, (ci.metadataId IS NOT NULL) AS hasCfImage,
                r.projectId AS mrProjectId, (ri.metadataId IS NOT NULL) AS hasMrImage
         FROM ServerModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN LocalModImageCache li ON li.metadataId = m.id
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN CurseForgeModImageCache ci ON ci.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         LEFT JOIN ModrinthModImageCache ri ON ri.metadataId = m.id
         WHERE f.serverId = :server_id";

    // --- mod-icon image blobs -------------------------------------------------
    fn get_instance_mod_icon_data(id: &str) -> Option<ModIconRow> =
        "SELECT li.data AS localData, ci.data AS cfData, ri.data AS mrData
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN LocalModImageCache li ON li.metadataId = m.id
         LEFT JOIN CurseForgeModImageCache ci ON ci.metadataId = m.id
         LEFT JOIN ModrinthModImageCache ri ON ri.metadataId = m.id
         WHERE f.id = :id";
    fn get_server_mod_icon_data(id: &str) -> Option<ModIconRow> =
        "SELECT li.data AS localData, ci.data AS cfData, ri.data AS mrData
         FROM ServerModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN LocalModImageCache li ON li.metadataId = m.id
         LEFT JOIN CurseForgeModImageCache ci ON ci.metadataId = m.id
         LEFT JOIN ModrinthModImageCache ri ON ri.metadataId = m.id
         WHERE f.id = :id";

    // --- CurseForge refresh: files whose CF cache is missing/stale ------------
    // (instance variant excludes worlds; server variant has no addonType filter)
    fn instance_mods_needing_cf_refresh(instance_id: i32, cutoff: DbDateTime) -> Vec<RefreshMetaRow> =
        "SELECT m.id AS metadataId, m.murmur2 AS murmur2, m.sha512 AS sha512
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         WHERE f.instanceId = :instance_id AND f.addonType <> 'worlds'
           AND (c.metadataId IS NULL OR c.cachedAt <= :cutoff)";
    fn server_mods_needing_cf_refresh(server_id: i32, cutoff: DbDateTime) -> Vec<RefreshMetaRow> =
        "SELECT m.id AS metadataId, m.murmur2 AS murmur2, m.sha512 AS sha512
         FROM ServerModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         WHERE f.serverId = :server_id
           AND (c.metadataId IS NULL OR c.cachedAt <= :cutoff)";

    // --- Modrinth refresh: files whose MR cache is missing/stale --------------
    fn instance_mods_needing_mr_refresh(instance_id: i32, cutoff: DbDateTime) -> Vec<RefreshMetaRow> =
        "SELECT m.id AS metadataId, m.murmur2 AS murmur2, m.sha512 AS sha512
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.instanceId = :instance_id AND f.addonType <> 'worlds'
           AND (r.metadataId IS NULL OR r.cachedAt <= :cutoff)";
    fn server_mods_needing_mr_refresh(server_id: i32, cutoff: DbDateTime) -> Vec<RefreshMetaRow> =
        "SELECT m.id AS metadataId, m.murmur2 AS murmur2, m.sha512 AS sha512
         FROM ServerModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.serverId = :server_id
           AND (r.metadataId IS NULL OR r.cachedAt <= :cutoff)";

    // --- stale CurseForge logos (upToDate = 0) --------------------------------
    fn instance_mods_stale_cf_logo(instance_id: i32) -> Vec<CfLogoRefreshRow> =
        "SELECT f.filename AS filename, c.projectId AS projectId, c.fileId AS fileId,
                ci.metadataId AS metadataId, ci.url AS url
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         JOIN CurseForgeModCache c ON c.metadataId = m.id
         JOIN CurseForgeModImageCache ci ON ci.metadataId = c.metadataId
         WHERE f.instanceId = :instance_id AND ci.upToDate = 0";
    fn server_mods_stale_cf_logo(server_id: i32) -> Vec<CfLogoRefreshRow> =
        "SELECT f.filename AS filename, c.projectId AS projectId, c.fileId AS fileId,
                ci.metadataId AS metadataId, ci.url AS url
         FROM ServerModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         JOIN CurseForgeModCache c ON c.metadataId = m.id
         JOIN CurseForgeModImageCache ci ON ci.metadataId = c.metadataId
         WHERE f.serverId = :server_id AND ci.upToDate = 0";

    // --- stale Modrinth logos (upToDate = 0) ----------------------------------
    fn instance_mods_stale_mr_logo(instance_id: i32) -> Vec<MrLogoRefreshRow> =
        "SELECT f.filename AS filename, r.projectId AS projectId, r.versionId AS versionId,
                ri.metadataId AS metadataId, ri.url AS url
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         JOIN ModrinthModCache r ON r.metadataId = m.id
         JOIN ModrinthModImageCache ri ON ri.metadataId = r.metadataId
         WHERE f.instanceId = :instance_id AND ri.upToDate = 0";
    fn server_mods_stale_mr_logo(server_id: i32) -> Vec<MrLogoRefreshRow> =
        "SELECT f.filename AS filename, r.projectId AS projectId, r.versionId AS versionId,
                ri.metadataId AS metadataId, ri.url AS url
         FROM ServerModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         JOIN ModrinthModCache r ON r.metadataId = m.id
         JOIN ModrinthModImageCache ri ON ri.metadataId = r.metadataId
         WHERE f.serverId = :server_id AND ri.upToDate = 0";

    // --- installer existence checks -------------------------------------------
    fn instance_mod_exists_by_cf_project(instance_id: i32, project_id: i32) -> Option<ExistsRow> =
        "SELECT f.id AS id
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         JOIN CurseForgeModCache c ON c.metadataId = m.id
         WHERE f.instanceId = :instance_id AND c.projectId = :project_id
         LIMIT 1";
    fn instance_mod_exists_by_mr_project(instance_id: i32, project_id: &str) -> Option<ExistsRow> =
        "SELECT f.id AS id
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.instanceId = :instance_id AND r.projectId = :project_id
         LIMIT 1";

    // --- update-flow platform ids ---------------------------------------------
    fn get_instance_mod_cf_ids(id: &str) -> Option<ModCfIdsRow> =
        "SELECT c.projectId AS cfProjectId, c.fileId AS cfFileId
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         WHERE f.id = :id";
    fn get_instance_mod_mr_ids(id: &str) -> Option<ModMrIdsRow> =
        "SELECT r.projectId AS mrProjectId, r.versionId AS mrVersionId
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.id = :id";
    fn get_instance_mod_update_ids(id: &str) -> Option<ModUpdateIdsRow> =
        "SELECT c.projectId AS cfProjectId, c.fileId AS cfFileId,
                r.projectId AS mrProjectId, r.versionId AS mrVersionId
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.id = :id";

    // --- shader-loader detection ----------------------------------------------
    fn get_enabled_instance_mod_modids(instance_id: i32) -> Vec<ModModidRow> =
        "SELECT m.modid AS modid
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         WHERE f.instanceId = :instance_id AND f.enabled = 1";

    // --- export queries -------------------------------------------------------
    fn get_instance_export_mods(instance_id: i32) -> Vec<InstanceModExportRow> =
        "SELECT f.filename AS filename, m.name AS metaName,
                c.projectId AS cfProjectId, c.fileId AS cfFileId, c.name AS cfName, c.urlslug AS cfUrlslug,
                r.projectId AS mrProjectId, r.versionId AS mrVersionId, r.title AS mrTitle, r.urlslug AS mrUrlslug
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.instanceId = :instance_id AND f.addonType = 'mods'";
    fn get_instance_cf_export_files(instance_id: i32) -> Vec<CfExportRow> =
        "SELECT f.filename AS filename, c.projectId AS cfProjectId, c.fileId AS cfFileId
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         WHERE f.instanceId = :instance_id";
    fn get_instance_mr_export_files(instance_id: i32) -> Vec<MrExportRow> =
        "SELECT f.filename AS filename, f.filesize AS filesize,
                m.sha512 AS sha512, m.sha1 AS sha1, r.fileUrl AS mrFileUrl
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.instanceId = :instance_id";
    fn get_instance_gdl_export_files(instance_id: i32) -> Vec<GdlExportRow> =
        "SELECT f.filename AS filename, m.sha512 AS sha512, m.sha1 AS sha1, m.murmur2 AS murmur2,
                c.projectId AS cfProjectId, r.projectId AS mrProjectId
         FROM ModFileCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id
         WHERE f.instanceId = :instance_id";
}

// ---------------------------------------------------------------------------
// Hand-written upserts (client-generated uuid id + explicit freshness column)
// ---------------------------------------------------------------------------

/// SQL executed by `upsert_mod_file_cache`, shared with its `QueryCheck` so the
/// checker validates the exact statement the fn runs.
const UPSERT_MOD_FILE_CACHE_SQL: &str =
    "INSERT INTO ModFileCache (id, instanceId, filename, filesize, enabled, addonType, metadataId, lastUpdatedAt)
     VALUES (:id, :instance_id, :filename, :filesize, :enabled, :addon_type, :metadata_id, :updated_at)
     ON CONFLICT(instanceId, filename) DO UPDATE SET
       filesize = excluded.filesize,
       enabled = excluded.enabled,
       metadataId = excluded.metadataId,
       addonType = excluded.addonType,
       lastUpdatedAt = excluded.lastUpdatedAt";

/// Upserts a `ModFileCache` row on the `(instanceId, filename)` unique key.
/// The `id` is a generated UUID used only for the initial INSERT; a conflicting
/// row keeps its existing id.
#[allow(clippy::too_many_arguments)]
pub fn upsert_mod_file_cache_conn(
    conn: &impl crate::db_exec::WriteAccess,
    instance_id: i32,
    filename: &str,
    filesize: i32,
    enabled: bool,
    addon_type: &str,
    metadata_id: &str,
    updated_at: DbDateTime,
) -> Result<usize, rusqlite::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut st = conn.prepare_cached(UPSERT_MOD_FILE_CACHE_SQL)?;
    st.execute(rusqlite::named_params! {
        ":id": id,
        ":instance_id": instance_id,
        ":filename": filename,
        ":filesize": filesize,
        ":enabled": enabled,
        ":addon_type": addon_type,
        ":metadata_id": metadata_id,
        ":updated_at": updated_at,
    })
}

/// Pool-routing wrapper for [`upsert_mod_file_cache_conn`].
#[allow(clippy::too_many_arguments)]
pub async fn upsert_mod_file_cache(
    db: &Db,
    instance_id: i32,
    filename: String,
    filesize: i32,
    enabled: bool,
    addon_type: String,
    metadata_id: String,
    updated_at: DbDateTime,
) -> DbResult<usize> {
    db.write(move |conn| {
        Ok(upsert_mod_file_cache_conn(
            &conn,
            instance_id,
            &filename,
            filesize,
            enabled,
            &addon_type,
            &metadata_id,
            updated_at,
        )?)
    })
    .await
}

const UPSERT_MOD_FILE_CACHE_CHECK: crate::registry::QueryCheck = crate::registry::QueryCheck {
    name: "upsert_mod_file_cache",
    sql: UPSERT_MOD_FILE_CACHE_SQL,
    params: &[
        ":id",
        ":instance_id",
        ":filename",
        ":filesize",
        ":enabled",
        ":addon_type",
        ":metadata_id",
        ":updated_at",
    ],
    columns: None,
    class: crate::registry::class_of(UPSERT_MOD_FILE_CACHE_SQL),
};

/// SQL executed by `upsert_server_mod_file_cache`.
const UPSERT_SERVER_MOD_FILE_CACHE_SQL: &str =
    "INSERT INTO ServerModFileCache (id, serverId, filename, filesize, enabled, addonType, metadataId, lastUpdatedAt)
     VALUES (:id, :server_id, :filename, :filesize, :enabled, :addon_type, :metadata_id, :updated_at)
     ON CONFLICT(serverId, filename) DO UPDATE SET
       filesize = excluded.filesize,
       enabled = excluded.enabled,
       metadataId = excluded.metadataId,
       addonType = excluded.addonType,
       lastUpdatedAt = excluded.lastUpdatedAt";

/// Upserts a `ServerModFileCache` row on the `(serverId, filename)` unique key.
#[allow(clippy::too_many_arguments)]
pub fn upsert_server_mod_file_cache_conn(
    conn: &impl crate::db_exec::WriteAccess,
    server_id: i32,
    filename: &str,
    filesize: i32,
    enabled: bool,
    addon_type: &str,
    metadata_id: &str,
    updated_at: DbDateTime,
) -> Result<usize, rusqlite::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut st = conn.prepare_cached(UPSERT_SERVER_MOD_FILE_CACHE_SQL)?;
    st.execute(rusqlite::named_params! {
        ":id": id,
        ":server_id": server_id,
        ":filename": filename,
        ":filesize": filesize,
        ":enabled": enabled,
        ":addon_type": addon_type,
        ":metadata_id": metadata_id,
        ":updated_at": updated_at,
    })
}

/// Pool-routing wrapper for [`upsert_server_mod_file_cache_conn`].
#[allow(clippy::too_many_arguments)]
pub async fn upsert_server_mod_file_cache(
    db: &Db,
    server_id: i32,
    filename: String,
    filesize: i32,
    enabled: bool,
    addon_type: String,
    metadata_id: String,
    updated_at: DbDateTime,
) -> DbResult<usize> {
    db.write(move |conn| {
        Ok(upsert_server_mod_file_cache_conn(
            &conn,
            server_id,
            &filename,
            filesize,
            enabled,
            &addon_type,
            &metadata_id,
            updated_at,
        )?)
    })
    .await
}

const UPSERT_SERVER_MOD_FILE_CACHE_CHECK: crate::registry::QueryCheck = crate::registry::QueryCheck {
    name: "upsert_server_mod_file_cache",
    sql: UPSERT_SERVER_MOD_FILE_CACHE_SQL,
    params: &[
        ":id",
        ":server_id",
        ":filename",
        ":filesize",
        ":enabled",
        ":addon_type",
        ":metadata_id",
        ":updated_at",
    ],
    columns: None,
    class: crate::registry::class_of(UPSERT_SERVER_MOD_FILE_CACHE_SQL),
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus the
/// two hand-written upsert entries.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    let mut all: Vec<crate::registry::QueryCheck> = QUERIES.to_vec();
    all.push(UPSERT_MOD_FILE_CACHE_CHECK);
    all.push(UPSERT_SERVER_MOD_FILE_CACHE_CHECK);
    all
}
