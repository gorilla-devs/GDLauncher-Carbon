//! Repository queries for `ModMetadata`, the CurseForge / Modrinth platform
//! mod caches (`CurseForgeModCache`, `ModrinthModCache`) and their image caches
//! (`LocalModImageCache`, `CurseForgeModImageCache`, `ModrinthModImageCache`).
//!
//! `ModMetadata` is the physical-content hub every file links to. Modrinth
//! artifacts refresh by verified metadata owner; distinct hashes in a version
//! remain distinct. CurseForge retains its project/file composite identity and
//! returns the surviving healthy owner for image attachment. Dead conflicts
//! alone are repaired transactionally, including in supported FK-off sessions.
//!
//! `ModMetadata.lastUpdatedAt` is written explicitly on insert. `ModMetadata`
//! is created once and never updated.

use crate::db_error::DbResult;
use crate::db_exec::Db;
use crate::dbtypes::DbDateTime;
use crate::queries;
use chrono::{DateTime, FixedOffset};

// ---------------------------------------------------------------------------
// Row structs
// ---------------------------------------------------------------------------

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ModMetadataRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub murmur2: i32,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub name: Option<String>,
    pub modid: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: String,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct CfModCacheRow {
    pub metadata_id: String,
    pub murmur2: i32,
    pub project_id: i32,
    pub file_id: i32,
    pub name: String,
    pub version: String,
    pub urlslug: String,
    pub summary: String,
    pub authors: String,
    pub release_type: i32,
    pub update_paths: String,
    pub cached_at: DateTime<FixedOffset>,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrModCacheRow {
    pub metadata_id: String,
    pub sha512: String,
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub version: String,
    pub urlslug: String,
    pub description: String,
    pub authors: String,
    pub release_type: i32,
    pub update_paths: String,
    pub filename: String,
    pub file_url: String,
    pub cached_at: DateTime<FixedOffset>,
}

/// The `metadataId` a composite-key upsert settled on, read back via
/// `RETURNING` so the caller can attach the image row to the surviving row.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MetadataIdRow {
    pub metadata_id: String,
}

/// A CurseForge cache row enriched with its metadata's Modrinth cross-reference,
/// for the share-metadata export (`export/mod.rs`). The `mr_*` columns are
/// `Option` (LEFT JOIN — a mod may have only a CurseForge cache row).
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct CfExportEnrichRow {
    pub project_id: i32,
    pub name: String,
    pub urlslug: String,
    pub mr_project_id: Option<String>,
    pub mr_version_id: Option<String>,
    pub mr_urlslug: Option<String>,
}

/// A Modrinth cache row's display name + slug for the share-metadata export.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrExportEnrichRow {
    pub project_id: String,
    pub title: String,
    pub urlslug: String,
}

// ---------------------------------------------------------------------------
// Static queries
// ---------------------------------------------------------------------------

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrUpdateContextRow {
    pub metadata_id: String,
    pub context: String,
    pub update_paths: String,
    pub cached_at: DateTime<FixedOffset>,
}

queries! {
    fn begin_cache_savepoint() -> usize = "SAVEPOINT metadata_cache_write";
    fn release_cache_savepoint() -> usize = "RELEASE metadata_cache_write";
    fn rollback_cache_savepoint() -> usize = "ROLLBACK TO metadata_cache_write";
    fn get_mr_update_context(metadata_id: &str, context: &str) -> Option<MrUpdateContextRow> =
        "SELECT metadataId, context, updatePaths, cachedAt FROM ModrinthUpdateCache
         WHERE metadataId=:metadata_id AND context=:context";
    fn upsert_mr_update_context(metadata_id: &str, context: &str, paths: &str, updated_at: DbDateTime) -> usize =
        "INSERT INTO ModrinthUpdateCache(metadataId,context,updatePaths,cachedAt)
         SELECT id,:context,:paths,:updated_at FROM ModMetadata WHERE id=:metadata_id
         ON CONFLICT(metadataId,context) DO UPDATE SET updatePaths=excluded.updatePaths,cachedAt=excluded.cachedAt";

    fn delete_dead_cf_images(project_id: i32, file_id: i32) -> usize =
        "DELETE FROM CurseForgeModImageCache WHERE metadataId IN (SELECT metadataId FROM CurseForgeModCache WHERE projectId=:project_id AND fileId=:file_id AND metadataId NOT IN (SELECT id FROM ModMetadata))";
    fn delete_dead_cf_cache(project_id: i32, file_id: i32) -> usize =
        "DELETE FROM CurseForgeModCache WHERE projectId=:project_id AND fileId=:file_id AND metadataId NOT IN (SELECT id FROM ModMetadata)";
    // --- ModMetadata ----------------------------------------------------------
    fn get_metadata_by_id(id: &str) -> Option<ModMetadataRow> =
        "SELECT id, lastUpdatedAt, murmur2, sha512, sha1, name, modid, version, description, authors, modloaders FROM ModMetadata WHERE id = :id";
    fn find_metadata_by_hashes(sha512: &[u8], murmur2: i32) -> Option<ModMetadataRow> =
        "SELECT id, lastUpdatedAt, murmur2, sha512, sha1, name, modid, version, description, authors, modloaders
         FROM ModMetadata WHERE sha512 = :sha512 AND murmur2 = :murmur2";
    fn insert_metadata(
        id: &str,
        murmur2: i32,
        sha512: &[u8],
        sha1: &[u8],
        modloaders: &str,
        name: Option<&str>,
        modid: Option<&str>,
        version: Option<&str>,
        description: Option<&str>,
        authors: Option<&str>,
        updated_at: DbDateTime
    ) -> usize =
        "INSERT INTO ModMetadata (id, lastUpdatedAt, murmur2, sha512, sha1, modloaders, name, modid, version, description, authors)
         VALUES (:id, :updated_at, :murmur2, :sha512, :sha1, :modloaders, :name, :modid, :version, :description, :authors)";
    fn update_local_modloaders(id: &str, modloaders: &str, updated_at: DbDateTime) -> usize =
        "UPDATE ModMetadata SET modloaders = :modloaders, lastUpdatedAt = :updated_at WHERE id = :id";
    fn gc_local_images() -> usize =
        "DELETE FROM LocalModImageCache WHERE metadataId NOT IN (SELECT id FROM ModMetadata WHERE id IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL))";
    fn gc_cf_images() -> usize =
        "DELETE FROM CurseForgeModImageCache WHERE metadataId NOT IN (SELECT id FROM ModMetadata WHERE id IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL)) OR metadataId NOT IN (SELECT metadataId FROM CurseForgeModCache)";
    fn gc_mr_images() -> usize =
        "DELETE FROM ModrinthModImageCache WHERE metadataId NOT IN (SELECT id FROM ModMetadata WHERE id IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL)) OR metadataId NOT IN (SELECT metadataId FROM ModrinthModCache)";
    fn gc_mr_contexts() -> usize =
        "DELETE FROM ModrinthUpdateCache WHERE metadataId NOT IN (SELECT id FROM ModMetadata WHERE id IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL))";
    fn gc_cf_cache() -> usize =
        "DELETE FROM CurseForgeModCache WHERE metadataId NOT IN (SELECT id FROM ModMetadata WHERE id IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL))";
    fn gc_mr_cache() -> usize =
        "DELETE FROM ModrinthModCache WHERE metadataId NOT IN (SELECT id FROM ModMetadata WHERE id IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL))";
    fn gc_orphan_metadata_rows() -> usize =
        "DELETE FROM ModMetadata WHERE id NOT IN (SELECT metadataId FROM ModFileCache WHERE metadataId IS NOT NULL UNION SELECT metadataId FROM ServerModFileCache WHERE metadataId IS NOT NULL)";

    // --- LocalModImageCache ---------------------------------------------------
    fn insert_local_image(metadata_id: &str, data: &[u8]) -> usize =
        "INSERT INTO LocalModImageCache (metadataId, data) SELECT id, :data FROM ModMetadata WHERE id = :metadata_id";

    // --- CurseForgeModCache: freshness pre-check read -------------------------
    fn get_cf_cache_by_metadata(metadata_id: &str) -> Option<CfModCacheRow> =
        "SELECT metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt
         FROM CurseForgeModCache WHERE metadataId = :metadata_id";

    // --- ModrinthModCache: freshness pre-check read ---------------------------
    fn get_mr_cache_by_metadata(metadata_id: &str) -> Option<MrModCacheRow> =
        "SELECT metadataId, sha512, projectId, versionId, title, version, urlslug, description, authors, releaseType, updatePaths, filename, fileUrl, cachedAt
         FROM ModrinthModCache WHERE metadataId = :metadata_id AND EXISTS (SELECT 1 FROM ModMetadata m WHERE m.id=metadataId AND lower(ModrinthModCache.sha512)=lower(hex(m.sha512)))";

    // --- CurseForge image cache ----------------------------------------------
    // Upsert marks the image stale (needs download): sets url, clears freshness.
    // On conflict `data` is intentionally left untouched (a previously cached
    // blob survives until the next download refreshes it).
    fn upsert_cf_image(metadata_id: &str, url: &str) -> usize =
        "INSERT INTO CurseForgeModImageCache (metadataId, url, data, upToDate)
         SELECT p.metadataId, :url, NULL, 0 FROM CurseForgeModCache p JOIN ModMetadata m ON m.id=p.metadataId WHERE p.metadataId=:metadata_id
         ON CONFLICT(metadataId) DO UPDATE SET url = excluded.url, upToDate = 0";
    fn mark_cf_image_downloaded(metadata_id: &str, data: &[u8]) -> usize =
        "UPDATE CurseForgeModImageCache SET upToDate = 1, data = :data WHERE metadataId = :metadata_id AND metadataId IN (SELECT p.metadataId FROM CurseForgeModCache p JOIN ModMetadata m ON m.id=p.metadataId)";

    // --- Modrinth image cache -------------------------------------------------
    fn upsert_mr_image(metadata_id: &str, url: &str) -> usize =
        "INSERT INTO ModrinthModImageCache (metadataId, url, data, upToDate)
         SELECT p.metadataId, :url, NULL, 0 FROM ModrinthModCache p JOIN ModMetadata m ON m.id=p.metadataId WHERE p.metadataId=:metadata_id
         ON CONFLICT(metadataId) DO UPDATE SET url = excluded.url, upToDate = 0";
    fn mark_mr_image_downloaded(metadata_id: &str, data: &[u8]) -> usize =
        "UPDATE ModrinthModImageCache SET upToDate = 1, data = :data WHERE metadataId = :metadata_id AND metadataId IN (SELECT p.metadataId FROM ModrinthModCache p JOIN ModMetadata m ON m.id=p.metadataId)";

    // --- export cross-reference (per-project-id; caller loops the id list) -----
    fn get_cf_export_enrich_by_project(project_id: i32) -> Vec<CfExportEnrichRow> =
        "SELECT c.projectId AS projectId, c.name AS name, c.urlslug AS urlslug,
                r.projectId AS mrProjectId, r.versionId AS mrVersionId, r.urlslug AS mrUrlslug
         FROM CurseForgeModCache c
         LEFT JOIN ModrinthModCache r ON r.metadataId = c.metadataId
         WHERE c.projectId = :project_id";
    fn get_mr_export_enrich_by_project(project_id: &str) -> Vec<MrExportEnrichRow> =
        "SELECT projectId AS projectId, title AS title, urlslug AS urlslug
         FROM ModrinthModCache WHERE projectId = :project_id";
}

// ---------------------------------------------------------------------------
// Hand-written composite-key upserts (RETURNING the surviving metadataId)
// ---------------------------------------------------------------------------

/// SQL executed by `upsert_cf_mod_cache`, shared with its `QueryCheck`. The
/// conflict target is the composite `(projectId, fileId)` unique key — NOT the
/// `metadataId` PK. `metadataId` is deliberately absent from the `DO UPDATE
/// SET` list, so a conflicting row keeps its own `metadataId`, which
/// `RETURNING` reads back.
const UPSERT_CF_MOD_CACHE_SQL: &str =
    "INSERT INTO CurseForgeModCache
       (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
     VALUES
       (:metadata_id, :murmur2, :project_id, :file_id, :name, :version, :urlslug, :summary, :authors, :release_type, :update_paths, :cached_at)
     ON CONFLICT(projectId, fileId) DO UPDATE SET
       murmur2 = excluded.murmur2,
       projectId = excluded.projectId,
       fileId = excluded.fileId,
       name = excluded.name,
       version = excluded.version,
       urlslug = excluded.urlslug,
       summary = excluded.summary,
       authors = excluded.authors,
       releaseType = excluded.releaseType,
       updatePaths = excluded.updatePaths,
       cachedAt = excluded.cachedAt
     RETURNING metadataId";

/// Upserts a `CurseForgeModCache` row on the composite `(projectId, fileId)`
/// key and returns the `metadataId` of the surviving row.
#[allow(clippy::too_many_arguments)]
pub fn upsert_cf_mod_cache_conn(
    conn: &impl crate::db_exec::WriteAccess,
    murmur2: i32,
    project_id: i32,
    file_id: i32,
    name: &str,
    version: &str,
    urlslug: &str,
    summary: &str,
    authors: &str,
    release_type: i32,
    update_paths: &str,
    cached_at: DbDateTime,
    metadata_id: &str,
) -> Result<String, rusqlite::Error> {
    require_metadata(conn, metadata_id)?;
    with_cache_savepoint(conn, || {
        // Only dead owners are repairable; a healthy shared owner is retained.
        delete_dead_cf_images_conn(conn, project_id, file_id)?;
        delete_dead_cf_cache_conn(conn, project_id, file_id)?;
        use crate::from_row::FromRow;
        let mut st = conn.prepare_cached(UPSERT_CF_MOD_CACHE_SQL)?;
        let row = st.query_row(
            rusqlite::named_params! {
                ":metadata_id": metadata_id,
                ":murmur2": murmur2,
                ":project_id": project_id,
                ":file_id": file_id,
                ":name": name,
                ":version": version,
                ":urlslug": urlslug,
                ":summary": summary,
                ":authors": authors,
                ":release_type": release_type,
                ":update_paths": update_paths,
                ":cached_at": cached_at,
            },
            MetadataIdRow::from_row,
        )?;
        Ok(row.metadata_id)
    })
}

/// Pool-routing wrapper for [`upsert_cf_mod_cache_conn`].
#[allow(clippy::too_many_arguments)]
pub async fn upsert_cf_mod_cache(
    db: &Db,
    murmur2: i32,
    project_id: i32,
    file_id: i32,
    name: String,
    version: String,
    urlslug: String,
    summary: String,
    authors: String,
    release_type: i32,
    update_paths: String,
    cached_at: DbDateTime,
    metadata_id: String,
) -> DbResult<String> {
    db.write(move |conn| {
        Ok(upsert_cf_mod_cache_conn(
            &conn,
            murmur2,
            project_id,
            file_id,
            &name,
            &version,
            &urlslug,
            &summary,
            &authors,
            release_type,
            &update_paths,
            cached_at,
            &metadata_id,
        )?)
    })
    .await
}

const UPSERT_CF_MOD_CACHE_CHECK: crate::registry::QueryCheck = crate::registry::QueryCheck {
    name: "upsert_cf_mod_cache",
    sql: UPSERT_CF_MOD_CACHE_SQL,
    params: &[
        ":metadata_id",
        ":murmur2",
        ":project_id",
        ":file_id",
        ":name",
        ":version",
        ":urlslug",
        ":summary",
        ":authors",
        ":release_type",
        ":update_paths",
        ":cached_at",
    ],
    columns: None,
    class: crate::registry::class_of(UPSERT_CF_MOD_CACHE_SQL),
    routes_write: true,
};

/// SQL executed by `upsert_mr_mod_cache`, shared with its `QueryCheck`. Conflict
/// target is the physical metadata owner, after verifying the artifact hash.
const UPSERT_MR_MOD_CACHE_SQL: &str =
    "INSERT INTO ModrinthModCache
       (metadataId, sha512, projectId, versionId, title, version, urlslug, description, authors, releaseType, updatePaths, filename, fileUrl, cachedAt)
     VALUES
       (:metadata_id, :sha512, :project_id, :version_id, :title, :version, :urlslug, :description, :authors, :release_type, :update_paths, :filename, :file_url, :cached_at)
     ON CONFLICT(metadataId) DO UPDATE SET
       sha512 = excluded.sha512,
       projectId = excluded.projectId,
       versionId = excluded.versionId,
       title = excluded.title,
       version = excluded.version,
       urlslug = excluded.urlslug,
       description = excluded.description,
       authors = excluded.authors,
       releaseType = excluded.releaseType,
       updatePaths = excluded.updatePaths,
       filename = excluded.filename,
       fileUrl = excluded.fileUrl,
       cachedAt = excluded.cachedAt
     WHERE (ModrinthModCache.projectId=excluded.projectId AND ModrinthModCache.versionId=excluded.versionId)
        OR lower(ModrinthModCache.sha512) != lower(excluded.sha512)
     RETURNING metadataId";

/// Refreshes the exact artifact owned by `metadataId`; healthy different
/// project/version associations are never silently retargeted.
#[allow(clippy::too_many_arguments)]
pub fn upsert_mr_mod_cache_conn(
    conn: &impl crate::db_exec::WriteAccess,
    sha512: &str,
    project_id: &str,
    version_id: &str,
    title: &str,
    version: &str,
    urlslug: &str,
    description: &str,
    authors: &str,
    release_type: i32,
    update_paths: &str,
    filename: &str,
    file_url: &str,
    cached_at: DbDateTime,
    metadata_id: &str,
) -> Result<String, rusqlite::Error> {
    require_metadata(conn, metadata_id)?;
    let owner = get_metadata_by_id_conn(conn, metadata_id)?.unwrap();
    if !hex::encode(&owner.sha512).eq_ignore_ascii_case(sha512) {
        return Err(rusqlite::Error::InvalidParameterName(
            "Modrinth artifact hash does not match its physical owner".into(),
        ));
    }
    use crate::from_row::FromRow;
    let mut st = conn.prepare_cached(UPSERT_MR_MOD_CACHE_SQL)?;
    let row = st.query_row(
        rusqlite::named_params! {
            ":metadata_id": metadata_id,
            ":sha512": sha512,
            ":project_id": project_id,
            ":version_id": version_id,
            ":title": title,
            ":version": version,
            ":urlslug": urlslug,
            ":description": description,
            ":authors": authors,
            ":release_type": release_type,
            ":update_paths": update_paths,
            ":filename": filename,
            ":file_url": file_url,
            ":cached_at": cached_at,
        },
        MetadataIdRow::from_row,
    )?;
    Ok(row.metadata_id)
}

/// Pool-routing wrapper for [`upsert_mr_mod_cache_conn`].
#[allow(clippy::too_many_arguments)]
pub async fn upsert_mr_mod_cache(
    db: &Db,
    sha512: String,
    project_id: String,
    version_id: String,
    title: String,
    version: String,
    urlslug: String,
    description: String,
    authors: String,
    release_type: i32,
    update_paths: String,
    filename: String,
    file_url: String,
    cached_at: DbDateTime,
    metadata_id: String,
) -> DbResult<String> {
    db.write(move |conn| {
        Ok(upsert_mr_mod_cache_conn(
            &conn,
            &sha512,
            &project_id,
            &version_id,
            &title,
            &version,
            &urlslug,
            &description,
            &authors,
            release_type,
            &update_paths,
            &filename,
            &file_url,
            cached_at,
            &metadata_id,
        )?)
    })
    .await
}

const UPSERT_MR_MOD_CACHE_CHECK: crate::registry::QueryCheck = crate::registry::QueryCheck {
    name: "upsert_mr_mod_cache",
    sql: UPSERT_MR_MOD_CACHE_SQL,
    params: &[
        ":metadata_id",
        ":sha512",
        ":project_id",
        ":version_id",
        ":title",
        ":version",
        ":urlslug",
        ":description",
        ":authors",
        ":release_type",
        ":update_paths",
        ":filename",
        ":file_url",
        ":cached_at",
    ],
    columns: None,
    class: crate::registry::class_of(UPSERT_MR_MOD_CACHE_SQL),
    routes_write: true,
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus the
/// two hand-written composite-key upsert entries.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    let mut all: Vec<crate::registry::QueryCheck> = QUERIES.to_vec();
    all.push(UPSERT_CF_MOD_CACHE_CHECK);
    all.push(UPSERT_MR_MOD_CACHE_CHECK);
    all
}

/// All dependent cleanup and owner retirement share a writer savepoint, including
/// callers already inside a transaction. FK-off sessions have identical behavior.
pub fn gc_orphan_metadata_conn(conn: &impl crate::db_exec::WriteAccess) -> rusqlite::Result<usize> {
    with_cache_savepoint(conn, || {
        gc_local_images_conn(conn)?;
        gc_cf_images_conn(conn)?;
        gc_mr_images_conn(conn)?;
        gc_mr_contexts_conn(conn)?;
        gc_cf_cache_conn(conn)?;
        gc_mr_cache_conn(conn)?;
        gc_orphan_metadata_rows_conn(conn)
    })
}

pub async fn gc_orphan_metadata(db: &Db) -> DbResult<usize> {
    db.write(|conn| Ok(gc_orphan_metadata_conn(&conn)?)).await
}

pub(crate) fn require_metadata(
    conn: &impl crate::db_exec::ReadAccess,
    id: &str,
) -> rusqlite::Result<()> {
    if get_metadata_by_id_conn(conn, id)?.is_none() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY),
            Some("FOREIGN KEY constraint failed: metadata owner disappeared before publication; retry".into()),
        ));
    }
    Ok(())
}

fn with_cache_savepoint<T>(
    conn: &impl crate::db_exec::WriteAccess,
    run: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    begin_cache_savepoint_conn(conn)?;
    match run() {
        Ok(value) => {
            release_cache_savepoint_conn(conn)?;
            Ok(value)
        }
        Err(error) => {
            rollback_cache_savepoint_conn(conn)?;
            release_cache_savepoint_conn(conn)?;
            Err(error)
        }
    }
}
