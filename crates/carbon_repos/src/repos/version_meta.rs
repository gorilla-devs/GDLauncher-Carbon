//! Repository queries for the four version-meta KV-blob caches:
//! `VersionInfoCache`, `PartialVersionInfoCache`, `LwjglMetaCache`,
//! `AssetsMetaCache`. All four share the same shape: a `String` id (PK), a
//! `Bytes` payload, and a `lastUpdatedAt` freshness column that every upsert
//! here sets explicitly (the freshness lint in `tests/query_checker.rs` guards
//! this).

use crate::dbtypes::DbDateTime;
use crate::queries;
use chrono::{DateTime, FixedOffset};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct VersionInfoCacheRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub version_info: Vec<u8>,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct PartialVersionInfoCacheRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub partial_version_info: Vec<u8>,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct LwjglMetaCacheRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub lwjgl: Vec<u8>,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct AssetsMetaCacheRow {
    pub id: String,
    pub last_updated_at: DateTime<FixedOffset>,
    pub assets_index: Vec<u8>,
}

queries! {
    fn get_version_info(id: &str) -> Option<VersionInfoCacheRow> =
        "SELECT id, lastUpdatedAt, versionInfo FROM VersionInfoCache WHERE id = :id";
    fn upsert_version_info(id: &str, version_info: &[u8], updated_at: DbDateTime) -> usize =
        "INSERT INTO VersionInfoCache (id, versionInfo, lastUpdatedAt) VALUES (:id, :version_info, :updated_at)
         ON CONFLICT(id) DO UPDATE SET versionInfo = excluded.versionInfo, lastUpdatedAt = excluded.lastUpdatedAt";

    fn get_partial_version_info(id: &str) -> Option<PartialVersionInfoCacheRow> =
        "SELECT id, lastUpdatedAt, partialVersionInfo FROM PartialVersionInfoCache WHERE id = :id";
    fn upsert_partial_version_info(id: &str, partial_version_info: &[u8], updated_at: DbDateTime) -> usize =
        "INSERT INTO PartialVersionInfoCache (id, partialVersionInfo, lastUpdatedAt) VALUES (:id, :partial_version_info, :updated_at)
         ON CONFLICT(id) DO UPDATE SET partialVersionInfo = excluded.partialVersionInfo, lastUpdatedAt = excluded.lastUpdatedAt";

    fn get_lwjgl_meta(id: &str) -> Option<LwjglMetaCacheRow> =
        "SELECT id, lastUpdatedAt, lwjgl FROM LwjglMetaCache WHERE id = :id";
    fn upsert_lwjgl_meta(id: &str, lwjgl: &[u8], updated_at: DbDateTime) -> usize =
        "INSERT INTO LwjglMetaCache (id, lwjgl, lastUpdatedAt) VALUES (:id, :lwjgl, :updated_at)
         ON CONFLICT(id) DO UPDATE SET lwjgl = excluded.lwjgl, lastUpdatedAt = excluded.lastUpdatedAt";

    fn get_assets_meta(id: &str) -> Option<AssetsMetaCacheRow> =
        "SELECT id, lastUpdatedAt, assetsIndex FROM AssetsMetaCache WHERE id = :id";
    fn upsert_assets_meta(id: &str, assets_index: &[u8], updated_at: DbDateTime) -> usize =
        "INSERT INTO AssetsMetaCache (id, assetsIndex, lastUpdatedAt) VALUES (:id, :assets_index, :updated_at)
         ON CONFLICT(id) DO UPDATE SET assetsIndex = excluded.assetsIndex, lastUpdatedAt = excluded.lastUpdatedAt";
}

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    QUERIES.to_vec()
}
