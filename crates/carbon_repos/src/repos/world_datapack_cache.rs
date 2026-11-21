//! Repository queries for the `WorldDatapackCache` table: the data packs found
//! in each world's `datapacks` folder of an instance.
//!
//! A row points at the shared `ModMetadata` of its zip (NULL for an unpacked
//! folder), so the CurseForge and Modrinth caches and their images are reused
//! as they are. The `id` column has a database default, and `lastUpdatedAt` is
//! written explicitly on every upsert, as the freshness lint requires.

use crate::dbtypes::DbDateTime;
use crate::queries;

use super::mod_file_cache::{ModIconRow, ModUpdateIdsRow};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct WorldDatapackRow {
    pub id: String,
    pub instance_id: i32,
    /// The save's folder name.
    pub world: String,
    /// File or folder name inside the world's `datapacks` folder.
    pub filename: String,
    pub filesize: i32,
    pub is_folder: bool,
    pub metadata_id: Option<String>,
    pub file_stamp: Option<String>,
}

/// A world data pack with its metadata and platform columns, as
/// `ModFullRow` carries them for an instance addon. Metadata and platform
/// columns are `Option`: an unpacked folder has no metadata row.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct WorldDatapackFullRow {
    pub id: String,
    pub world: String,
    pub filename: String,
    pub filesize: i32,
    pub is_folder: bool,
    pub meta_id: Option<String>,
    pub meta_name: Option<String>,
    pub meta_version: Option<String>,
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

queries! {
    fn get_world_datapacks_by_instance(instance_id: i32) -> Vec<WorldDatapackRow> =
        "SELECT id, instanceId, world, filename, filesize, isFolder, metadataId, fileStamp
         FROM WorldDatapackCache WHERE instanceId = :instance_id";
    fn get_world_datapack_by_id(id: &str) -> Option<WorldDatapackRow> =
        "SELECT id, instanceId, world, filename, filesize, isFolder, metadataId, fileStamp
         FROM WorldDatapackCache WHERE id = :id";

    // A conflicting row keeps its id; the scanner passes the stamp it observed.
    fn upsert_world_datapack(instance_id: i32, world: &str, filename: &str, filesize: i32, is_folder: bool, metadata_id: Option<&str>, file_stamp: Option<&str>, updated_at: DbDateTime) -> usize =
        "INSERT INTO WorldDatapackCache (instanceId, world, filename, filesize, isFolder, metadataId, fileStamp, lastUpdatedAt)
         VALUES (:instance_id, :world, :filename, :filesize, :is_folder, :metadata_id, :file_stamp, :updated_at)
         ON CONFLICT(instanceId, world, filename) DO UPDATE SET
           filesize = excluded.filesize,
           isFolder = excluded.isFolder,
           metadataId = excluded.metadataId,
           fileStamp = excluded.fileStamp,
           lastUpdatedAt = excluded.lastUpdatedAt";

    // The packs of a renamed world keep their ids.
    fn rename_world_of_datapacks(instance_id: i32, from: &str, to: &str, updated_at: DbDateTime) -> usize =
        "UPDATE WorldDatapackCache SET world = :to, lastUpdatedAt = :updated_at
         WHERE instanceId = :instance_id AND world = :from";
    fn delete_world_datapack_by_id(id: &str) -> usize =
        "DELETE FROM WorldDatapackCache WHERE id = :id";
    fn delete_world_datapacks_of_world(instance_id: i32, world: &str) -> usize =
        "DELETE FROM WorldDatapackCache WHERE instanceId = :instance_id AND world = :world";

    // LEFT JOIN on the metadata: an unpacked folder has none and is still listed.
    fn get_world_datapacks_full(instance_id: i32) -> Vec<WorldDatapackFullRow> =
        "SELECT f.id AS id, f.world AS world, f.filename AS filename, f.filesize AS filesize,
                f.isFolder AS isFolder,
                m.id AS metaId, m.name AS metaName, m.version AS metaVersion,
                c.projectId AS cfProjectId, c.fileId AS cfFileId, c.name AS cfName,
                c.version AS cfVersion, c.urlslug AS cfUrlslug, c.summary AS cfSummary,
                c.authors AS cfAuthors, c.releaseType AS cfReleaseType, c.updatePaths AS cfUpdatePaths,
                (ci.data IS NOT NULL) AS hasCfImage,
                r.projectId AS mrProjectId, r.versionId AS mrVersionId, r.title AS mrTitle,
                r.version AS mrVersion, r.urlslug AS mrUrlslug, r.description AS mrDescription,
                r.authors AS mrAuthors, r.releaseType AS mrReleaseType, r.updatePaths AS mrUpdatePaths,
                (ri.data IS NOT NULL) AS hasMrImage
         FROM WorldDatapackCache f
         LEFT JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN CurseForgeModImageCache ci ON ci.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id AND lower(r.sha512) = lower(hex(m.sha512))
         LEFT JOIN ModrinthModImageCache ri ON ri.metadataId = m.id
         WHERE f.instanceId = :instance_id";

    fn get_world_datapack_update_ids(id: &str) -> Option<ModUpdateIdsRow> =
        "SELECT c.projectId AS cfProjectId, c.fileId AS cfFileId, c.releaseType AS cfReleaseType,
                r.projectId AS mrProjectId, r.versionId AS mrVersionId, r.releaseType AS mrReleaseType,
                m.modloaders AS modloaders
         FROM WorldDatapackCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN CurseForgeModCache c ON c.metadataId = m.id
         LEFT JOIN ModrinthModCache r ON r.metadataId = m.id AND lower(r.sha512) = lower(hex(m.sha512))
         WHERE f.id = :id";

    fn get_world_datapack_icon_data(id: &str) -> Option<ModIconRow> =
        "SELECT li.data AS localData, ci.data AS cfData, ri.data AS mrData
         FROM WorldDatapackCache f
         JOIN ModMetadata m ON m.id = f.metadataId
         LEFT JOIN LocalModImageCache li ON li.metadataId = m.id
         LEFT JOIN CurseForgeModImageCache ci ON ci.metadataId = m.id
         LEFT JOIN ModrinthModImageCache ri ON ri.metadataId = m.id
         WHERE f.id = :id";
}

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    QUERIES.to_vec()
}
