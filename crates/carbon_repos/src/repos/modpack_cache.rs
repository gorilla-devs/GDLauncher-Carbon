//! Repository queries for the CurseForge / Modrinth modpack metadata caches
//! and their logo image caches. Both platforms key their modpack row on a
//! composite PK (`(projectId, fileId)` for CurseForge, `(projectId,
//! versionId)` for Modrinth — a String key, unlike every other domain's
//! integer/uuid PK) and hang a 1:1 optional image row off that same
//! composite key.
//!
//! `updatedAt` is the 7-day freshness column these caches exist for; it is
//! written explicitly on every upsert, in both the insert and the `DO UPDATE
//! SET` branch — the freshness lint in `tests/query_checker.rs` guards this.
//! The 7-day compare itself (`updated_at + 7d > now`) lives in the manager, in
//! Rust — `updated_at` here is just a plain `DateTime<FixedOffset>`.
//!
//! The joined "with logo" read uses a `LEFT JOIN`:
//! one row exposes the (possibly absent) image's `data` blob
//! flattened to `Option<Vec<u8>>`, plus a separate `has_logo` flag (the image
//! *row* exists, independent of whether its `data` was ever downloaded) — the
//! two are different questions call sites ask: "do we have image bytes to
//! serve" (`logo_data.is_some()`) vs "should we still touch the image cache on
//! a refresh even with no new bytes this time" (`has_logo`).

use crate::dbtypes::DbDateTime;
use crate::queries;
use chrono::{DateTime, FixedOffset};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct CfModpackWithLogoRow {
    pub project_id: i32,
    pub file_id: i32,
    pub modpack_name: String,
    pub version_name: String,
    pub url_slug: String,
    pub updated_at: DateTime<FixedOffset>,
    pub logo_data: Option<Vec<u8>>,
    #[nullable(false)]
    pub has_logo: bool,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct CfModpackImageRow {
    pub data: Option<Vec<u8>>,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrModpackWithLogoRow {
    pub project_id: String,
    pub version_id: String,
    pub modpack_name: String,
    pub version_name: String,
    pub url_slug: String,
    pub updated_at: DateTime<FixedOffset>,
    pub logo_data: Option<Vec<u8>>,
    #[nullable(false)]
    pub has_logo: bool,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct MrModpackImageRow {
    pub data: Option<Vec<u8>>,
}

queries! {
    // --- CurseForge modpack cache ---------------------------------------------
    fn get_cf_modpack(project_id: i32, file_id: i32) -> Option<CfModpackWithLogoRow> =
        "SELECT c.projectId AS projectId, c.fileId AS fileId,
                c.modpackName AS modpackName, c.versionName AS versionName, c.urlSlug AS urlSlug,
                c.updatedAt AS updatedAt,
                i.data AS logoData, (i.projectId IS NOT NULL) AS hasLogo
         FROM CurseForgeModpackCache c
         LEFT JOIN CurseForgeModpackImageCache i ON i.projectId = c.projectId AND i.fileId = c.fileId
         WHERE c.projectId = :project_id AND c.fileId = :file_id";
    fn upsert_cf_modpack(
        project_id: i32,
        file_id: i32,
        modpack_name: &str,
        version_name: &str,
        url_slug: &str,
        updated_at: DbDateTime
    ) -> usize =
        "INSERT INTO CurseForgeModpackCache (projectId, fileId, modpackName, versionName, urlSlug, updatedAt)
         VALUES (:project_id, :file_id, :modpack_name, :version_name, :url_slug, :updated_at)
         ON CONFLICT(projectId, fileId) DO UPDATE SET
           modpackName = excluded.modpackName,
           versionName = excluded.versionName,
           urlSlug = excluded.urlSlug,
           updatedAt = excluded.updatedAt";

    // --- CurseForge modpack image cache ---------------------------------------
    fn get_cf_modpack_logo(project_id: i32, file_id: i32) -> Option<CfModpackImageRow> =
        "SELECT data FROM CurseForgeModpackImageCache WHERE projectId = :project_id AND fileId = :file_id";
    fn upsert_cf_modpack_image(project_id: i32, file_id: i32, url: &str, data: Option<&[u8]>) -> usize =
        "INSERT INTO CurseForgeModpackImageCache (projectId, fileId, url, data)
         VALUES (:project_id, :file_id, :url, :data)
         ON CONFLICT(projectId, fileId) DO UPDATE SET url = excluded.url, data = excluded.data";

    // --- Modrinth modpack cache ------------------------------------------------
    fn get_mr_modpack(project_id: &str, version_id: &str) -> Option<MrModpackWithLogoRow> =
        "SELECT c.projectId AS projectId, c.versionId AS versionId,
                c.modpackName AS modpackName, c.versionName AS versionName, c.urlSlug AS urlSlug,
                c.updatedAt AS updatedAt,
                i.data AS logoData, (i.projectId IS NOT NULL) AS hasLogo
         FROM ModrinthModpackCache c
         LEFT JOIN ModrinthModpackImageCache i ON i.projectId = c.projectId AND i.versionId = c.versionId
         WHERE c.projectId = :project_id AND c.versionId = :version_id";
    fn upsert_mr_modpack(
        project_id: &str,
        version_id: &str,
        modpack_name: &str,
        version_name: &str,
        url_slug: &str,
        updated_at: DbDateTime
    ) -> usize =
        "INSERT INTO ModrinthModpackCache (projectId, versionId, modpackName, versionName, urlSlug, updatedAt)
         VALUES (:project_id, :version_id, :modpack_name, :version_name, :url_slug, :updated_at)
         ON CONFLICT(projectId, versionId) DO UPDATE SET
           modpackName = excluded.modpackName,
           versionName = excluded.versionName,
           urlSlug = excluded.urlSlug,
           updatedAt = excluded.updatedAt";

    // --- Modrinth modpack image cache -------------------------------------------
    fn get_mr_modpack_logo(project_id: &str, version_id: &str) -> Option<MrModpackImageRow> =
        "SELECT data FROM ModrinthModpackImageCache WHERE projectId = :project_id AND versionId = :version_id";
    fn upsert_mr_modpack_image(project_id: &str, version_id: &str, url: &str, data: Option<&[u8]>) -> usize =
        "INSERT INTO ModrinthModpackImageCache (projectId, versionId, url, data)
         VALUES (:project_id, :version_id, :url, :data)
         ON CONFLICT(projectId, versionId) DO UPDATE SET url = excluded.url, data = excluded.data";
}

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    QUERIES.to_vec()
}
