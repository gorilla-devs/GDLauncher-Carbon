//! Mod file and metadata queries.

use crate::define_query;
use crate::models::{
    CurseForgeModCache, CurseForgeModImageCache, LocalModImageCache, ModFileCache,
    ModFileCacheWithCurseforge, ModFileCacheWithMetadata, ModFileCacheWithMetadataAndImages,
    ModFileCacheWithModrinth, ModMetadata, ModrinthModCache, ModrinthModImageCache,
};

// ModFileCache queries
define_query!(FindModFileCache, "SELECT * FROM ModFileCache WHERE id = ?1", (id: &str) -> ModFileCache);
define_query!(
    FindModFileCacheByInstanceAndFilename,
    "SELECT * FROM ModFileCache WHERE instanceId = ?1 AND filename = ?2",
    (instance_id: i32, filename: &str) -> ModFileCache
);
define_query!(
    ListModFileCacheByInstance,
    "SELECT * FROM ModFileCache WHERE instanceId = ?1 ORDER BY filename",
    (instance_id: i32) -> ModFileCache
);
define_query!(
    ListModFileCacheByInstanceAndType,
    "SELECT * FROM ModFileCache WHERE instanceId = ?1 AND addonType = ?2 ORDER BY filename",
    (instance_id: i32, addon_type: &str) -> ModFileCache
);
define_query!(
    CountModFileCacheByInstance,
    "SELECT COUNT(*) FROM ModFileCache WHERE instanceId = ?1",
    (instance_id: i32) => i32
);

define_query!(
    CreateModFileCache,
    r#"INSERT INTO ModFileCache (id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7)"#,
    (id: &str, instance_id: i32, filename: &str, filesize: i32, enabled: bool, addon_type: &str, metadata_id: &str)
);

define_query!(
    UpsertModFileCache,
    r#"INSERT OR REPLACE INTO ModFileCache (id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7)"#,
    (id: &str, instance_id: i32, filename: &str, filesize: i32, enabled: bool, addon_type: &str, metadata_id: &str)
);

define_query!(
    UpdateModFileCacheEnabled,
    "UPDATE ModFileCache SET enabled = ?2, lastUpdatedAt = datetime('now') WHERE id = ?1",
    (id: &str, enabled: bool)
);
define_query!(
    UpdateModFileCacheFilename,
    "UPDATE ModFileCache SET filename = ?2, lastUpdatedAt = datetime('now') WHERE id = ?1",
    (id: &str, filename: &str)
);
define_query!(DeleteModFileCache, "DELETE FROM ModFileCache WHERE id = ?1", (id: &str));
define_query!(
    DeleteModFileCacheByInstance,
    "DELETE FROM ModFileCache WHERE instanceId = ?1",
    (instance_id: i32)
);
define_query!(
    DeleteModFileCacheByInstanceAndFilename,
    "DELETE FROM ModFileCache WHERE instanceId = ?1 AND filename = ?2",
    (instance_id: i32, filename: &str)
);

// ModMetadata queries
define_query!(FindModMetadata, "SELECT * FROM ModMetadata WHERE id = ?1", (id: &str) -> ModMetadata);
define_query!(
    FindModMetadataByMurmur2,
    "SELECT * FROM ModMetadata WHERE murmur2 = ?1",
    (murmur2: i32) -> ModMetadata
);
define_query!(ListModMetadata, "SELECT * FROM ModMetadata ORDER BY name", () -> ModMetadata);

define_query!(
    CreateModMetadata,
    r#"INSERT INTO ModMetadata (id, lastUpdatedAt, murmur2, sha512, sha1, name, modid, version, description, authors, modloaders)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
    (id: &str, murmur2: i32, sha512: &[u8], sha1: &[u8], name: Option<&str>, modid: Option<&str>, version: Option<&str>, description: Option<&str>, authors: Option<&str>, modloaders: &str)
);

define_query!(
    UpsertModMetadata,
    r#"INSERT OR REPLACE INTO ModMetadata (id, lastUpdatedAt, murmur2, sha512, sha1, name, modid, version, description, authors, modloaders)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
    (id: &str, murmur2: i32, sha512: &[u8], sha1: &[u8], name: Option<&str>, modid: Option<&str>, version: Option<&str>, description: Option<&str>, authors: Option<&str>, modloaders: &str)
);

define_query!(DeleteModMetadata, "DELETE FROM ModMetadata WHERE id = ?1", (id: &str));
define_query!(
    DeleteOrphanedModMetadata,
    r#"DELETE FROM ModMetadata WHERE id NOT IN (SELECT metadataId FROM ModFileCache)"#,
    ()
);
define_query!(
    FindModMetadataBySha512AndMurmur2,
    "SELECT * FROM ModMetadata WHERE sha512 = ?1 AND murmur2 = ?2",
    (sha512: &[u8], murmur2: i32) -> ModMetadata
);

// CurseForgeModCache queries
define_query!(
    FindCurseForgeModCache,
    "SELECT * FROM CurseForgeModCache WHERE metadataId = ?1",
    (metadata_id: &str) -> CurseForgeModCache
);
define_query!(
    FindCurseForgeModCacheByProjectFile,
    "SELECT * FROM CurseForgeModCache WHERE projectId = ?1 AND fileId = ?2",
    (project_id: i32, file_id: i32) -> CurseForgeModCache
);
define_query!(
    FindCurseForgeModCacheByMurmur2,
    "SELECT * FROM CurseForgeModCache WHERE murmur2 = ?1",
    (murmur2: i32) -> CurseForgeModCache
);

define_query!(
    UpsertCurseForgeModCache,
    r#"INSERT OR REPLACE INTO CurseForgeModCache
    (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
    (metadata_id: &str, murmur2: i32, project_id: i32, file_id: i32, name: &str, version: &str, urlslug: &str, summary: &str, authors: &str, release_type: i32, update_paths: &str, cached_at: &str)
);

define_query!(
    DeleteCurseForgeModCache,
    "DELETE FROM CurseForgeModCache WHERE metadataId = ?1",
    (metadata_id: &str)
);

// ModrinthModCache queries
define_query!(
    FindModrinthModCache,
    "SELECT * FROM ModrinthModCache WHERE metadataId = ?1",
    (metadata_id: &str) -> ModrinthModCache
);
define_query!(
    FindModrinthModCacheByProjectVersion,
    "SELECT * FROM ModrinthModCache WHERE projectId = ?1 AND versionId = ?2",
    (project_id: &str, version_id: &str) -> ModrinthModCache
);
define_query!(
    FindModrinthModCacheBySha512,
    "SELECT * FROM ModrinthModCache WHERE sha512 = ?1",
    (sha512: &str) -> ModrinthModCache
);

define_query!(
    UpsertModrinthModCache,
    r#"INSERT OR REPLACE INTO ModrinthModCache
    (metadataId, sha512, projectId, versionId, title, version, urlslug, description, authors, releaseType, updatePaths, filename, fileUrl, cachedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
    (metadata_id: &str, sha512: &str, project_id: &str, version_id: &str, title: &str, version: &str, urlslug: &str, description: &str, authors: &str, release_type: i32, update_paths: &str, filename: &str, file_url: &str, cached_at: &str)
);

define_query!(
    DeleteModrinthModCache,
    "DELETE FROM ModrinthModCache WHERE metadataId = ?1",
    (metadata_id: &str)
);

// Queries for checking if a mod from a specific platform exists in an instance
define_query!(
    FindModFileCacheByInstanceAndCfProject,
    r#"SELECT mfc.* FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    WHERE mfc.instanceId = ?1 AND cf.projectId = ?2
    LIMIT 1"#,
    (instance_id: i32, project_id: i32) -> ModFileCache
);

define_query!(
    FindModFileCacheByInstanceAndMrProject,
    r#"SELECT mfc.* FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.instanceId = ?1 AND mr.projectId = ?2
    LIMIT 1"#,
    (instance_id: i32, project_id: &str) -> ModFileCache
);

// Queries for listing mod files with platform-specific metadata (for exports)
define_query!(
    ListModFileCacheWithCurseforgeByInstance,
    r#"SELECT
        mfc.id, mfc.filename, mfc.filesize, mfc.enabled, mfc.addonType,
        cf.projectId as cf_projectId, cf.fileId as cf_fileId
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    WHERE mfc.instanceId = ?1
    ORDER BY mfc.filename"#,
    (instance_id: i32) -> ModFileCacheWithCurseforge
);

define_query!(
    ListModFileCacheWithModrinthByInstance,
    r#"SELECT
        mfc.id, mfc.filename, mfc.filesize, mfc.enabled, mfc.addonType,
        mm.sha512 as mm_sha512, mm.sha1 as mm_sha1,
        mr.projectId as mr_projectId, mr.versionId as mr_versionId, mr.fileUrl as mr_fileUrl
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.instanceId = ?1
    ORDER BY mfc.filename"#,
    (instance_id: i32) -> ModFileCacheWithModrinth
);

// Complex queries for mod file caching with metadata info - legacy for manual from_row
define_query!(
    ListModFilesNeedingCurseForgeUpdate,
    r#"SELECT mm.id, mm.murmur2 FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    WHERE mfc.instanceId = ?1 AND (cf.metadataId IS NULL OR cf.cachedAt < datetime('now', '-1 day'))"#
);

define_query!(
    ListModFilesNeedingModrinthUpdate,
    r#"SELECT mm.id, mm.sha512 FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.instanceId = ?1 AND (mr.metadataId IS NULL OR mr.cachedAt < datetime('now', '-1 day'))"#
);

define_query!(
    ListModFilesWithOutdatedCurseForgeIcons,
    r#"SELECT mfc.filename, cf.projectId, cf.fileId, cfi.metadataId, cfi.url
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    INNER JOIN CurseForgeModImageCache cfi ON cf.metadataId = cfi.metadataId
    WHERE mfc.instanceId = ?1 AND cfi.upToDate = 0"#
);

define_query!(
    ListModFilesWithOutdatedModrinthIcons,
    r#"SELECT mfc.filename, mr.projectId, mr.versionId, mri.metadataId, mri.url
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    INNER JOIN ModrinthModImageCache mri ON mr.metadataId = mri.metadataId
    WHERE mfc.instanceId = ?1 AND mri.upToDate = 0"#
);

// LocalModImageCache queries
define_query!(
    FindLocalModImageCache,
    "SELECT * FROM LocalModImageCache WHERE metadataId = ?1",
    (metadata_id: &str) -> LocalModImageCache
);
define_query!(
    UpsertLocalModImageCache,
    "INSERT OR REPLACE INTO LocalModImageCache (metadataId, data) VALUES (?1, ?2)",
    (metadata_id: &str, data: &[u8])
);
define_query!(
    DeleteLocalModImageCache,
    "DELETE FROM LocalModImageCache WHERE metadataId = ?1",
    (metadata_id: &str)
);

// CurseForgeModImageCache queries
define_query!(
    FindCurseForgeModImageCache,
    "SELECT * FROM CurseForgeModImageCache WHERE metadataId = ?1",
    (metadata_id: &str) -> CurseForgeModImageCache
);
define_query!(
    UpsertCurseForgeModImageCache,
    "INSERT OR REPLACE INTO CurseForgeModImageCache (metadataId, url, data, upToDate) VALUES (?1, ?2, ?3, ?4)",
    (metadata_id: &str, url: &str, data: Option<&[u8]>, up_to_date: i32)
);
define_query!(
    UpdateCurseForgeModImageCacheData,
    "UPDATE CurseForgeModImageCache SET data = ?2, upToDate = ?3 WHERE metadataId = ?1",
    (metadata_id: &str, data: Option<&[u8]>, up_to_date: i32)
);
define_query!(
    DeleteCurseForgeModImageCache,
    "DELETE FROM CurseForgeModImageCache WHERE metadataId = ?1",
    (metadata_id: &str)
);

// ModrinthModImageCache queries
define_query!(
    FindModrinthModImageCache,
    "SELECT * FROM ModrinthModImageCache WHERE metadataId = ?1",
    (metadata_id: &str) -> ModrinthModImageCache
);
define_query!(
    UpsertModrinthModImageCache,
    "INSERT OR REPLACE INTO ModrinthModImageCache (metadataId, url, data, upToDate) VALUES (?1, ?2, ?3, ?4)",
    (metadata_id: &str, url: &str, data: Option<&[u8]>, up_to_date: i32)
);
define_query!(
    UpdateModrinthModImageCacheData,
    "UPDATE ModrinthModImageCache SET data = ?2, upToDate = ?3 WHERE metadataId = ?1",
    (metadata_id: &str, data: Option<&[u8]>, up_to_date: i32)
);
define_query!(
    DeleteModrinthModImageCache,
    "DELETE FROM ModrinthModImageCache WHERE metadataId = ?1",
    (metadata_id: &str)
);

// Complex join queries for fetching mod files with all metadata
define_query!(
    FindModFileCacheWithMetadata,
    r#"SELECT
        mfc.*,
        mm.murmur2 as mm_murmur2, mm.sha512 as mm_sha512, mm.sha1 as mm_sha1, mm.name as mm_name,
        mm.modid as mm_modid, mm.version as mm_version, mm.description as mm_description,
        mm.authors as mm_authors, mm.modloaders as mm_modloaders, mm.lastUpdatedAt as mm_lastUpdatedAt,
        cf.projectId as cf_projectId, cf.fileId as cf_fileId, cf.name as cf_name, cf.version as cf_version,
        cf.urlslug as cf_urlslug, cf.summary as cf_summary, cf.authors as cf_authors, cf.releaseType as cf_releaseType,
        cf.updatePaths as cf_updatePaths, cf.cachedAt as cf_cachedAt, cf.murmur2 as cf_murmur2,
        mr.projectId as mr_projectId, mr.versionId as mr_versionId, mr.title as mr_title, mr.version as mr_version,
        mr.urlslug as mr_urlslug, mr.description as mr_description, mr.authors as mr_authors, mr.releaseType as mr_releaseType,
        mr.updatePaths as mr_updatePaths, mr.filename as mr_filename, mr.fileUrl as mr_fileUrl, mr.cachedAt as mr_cachedAt, mr.sha512 as mr_sha512
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    LEFT JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.id = ?1"#,
    (id: &str) -> ModFileCacheWithMetadata
);

define_query!(
    ListModFileCacheWithMetadataByInstance,
    r#"SELECT
        mfc.id, mfc.filename, mfc.filesize, mfc.enabled, mfc.addonType, mfc.instanceId, mfc.metadataId,
        mm.murmur2 as mm_murmur2, mm.sha512 as mm_sha512, mm.sha1 as mm_sha1, mm.name as mm_name,
        mm.modid as mm_modid, mm.version as mm_version, mm.description as mm_description,
        mm.authors as mm_authors, mm.modloaders as mm_modloaders, mm.lastUpdatedAt as mm_lastUpdatedAt,
        cf.projectId as cf_projectId, cf.fileId as cf_fileId, cf.name as cf_name, cf.version as cf_version,
        cf.urlslug as cf_urlslug, cf.summary as cf_summary, cf.authors as cf_authors, cf.releaseType as cf_releaseType,
        cf.updatePaths as cf_updatePaths, cf.cachedAt as cf_cachedAt, cf.murmur2 as cf_murmur2,
        mr.projectId as mr_projectId, mr.versionId as mr_versionId, mr.title as mr_title, mr.version as mr_version,
        mr.urlslug as mr_urlslug, mr.description as mr_description, mr.authors as mr_authors, mr.releaseType as mr_releaseType,
        mr.updatePaths as mr_updatePaths, mr.filename as mr_filename, mr.fileUrl as mr_fileUrl, mr.cachedAt as mr_cachedAt, mr.sha512 as mr_sha512
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    LEFT JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.instanceId = ?1
    ORDER BY mfc.filename"#,
    (instance_id: i32) -> ModFileCacheWithMetadata
);

// Same query but with addon_type filter
define_query!(
    ListModFileCacheWithMetadataByInstanceAndType,
    r#"SELECT
        mfc.id, mfc.filename, mfc.filesize, mfc.enabled, mfc.addonType, mfc.instanceId, mfc.metadataId,
        mm.murmur2 as mm_murmur2, mm.sha512 as mm_sha512, mm.sha1 as mm_sha1, mm.name as mm_name,
        mm.modid as mm_modid, mm.version as mm_version, mm.description as mm_description,
        mm.authors as mm_authors, mm.modloaders as mm_modloaders, mm.lastUpdatedAt as mm_lastUpdatedAt,
        cf.projectId as cf_projectId, cf.fileId as cf_fileId, cf.name as cf_name, cf.version as cf_version,
        cf.urlslug as cf_urlslug, cf.summary as cf_summary, cf.authors as cf_authors, cf.releaseType as cf_releaseType,
        cf.updatePaths as cf_updatePaths, cf.cachedAt as cf_cachedAt, cf.murmur2 as cf_murmur2,
        mr.projectId as mr_projectId, mr.versionId as mr_versionId, mr.title as mr_title, mr.version as mr_version,
        mr.urlslug as mr_urlslug, mr.description as mr_description, mr.authors as mr_authors, mr.releaseType as mr_releaseType,
        mr.updatePaths as mr_updatePaths, mr.filename as mr_filename, mr.fileUrl as mr_fileUrl, mr.cachedAt as mr_cachedAt, mr.sha512 as mr_sha512
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    LEFT JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.instanceId = ?1 AND mfc.addonType = ?2
    ORDER BY mfc.filename"#,
    (instance_id: i32, addon_type: &str) -> ModFileCacheWithMetadata
);

// Query with image info (checks if images exist)
define_query!(
    ListModFileCacheWithMetadataAndImagesByInstance,
    r#"SELECT
        mfc.id, mfc.filename, mfc.filesize, mfc.enabled, mfc.addonType, mfc.instanceId, mfc.metadataId,
        mm.murmur2 as mm_murmur2, mm.sha512 as mm_sha512, mm.sha1 as mm_sha1, mm.name as mm_name,
        mm.modid as mm_modid, mm.version as mm_version, mm.description as mm_description,
        mm.authors as mm_authors, mm.modloaders as mm_modloaders, mm.lastUpdatedAt as mm_lastUpdatedAt,
        cf.projectId as cf_projectId, cf.fileId as cf_fileId, cf.name as cf_name, cf.version as cf_version,
        cf.urlslug as cf_urlslug, cf.summary as cf_summary, cf.authors as cf_authors, cf.releaseType as cf_releaseType,
        cf.updatePaths as cf_updatePaths, cf.cachedAt as cf_cachedAt, cf.murmur2 as cf_murmur2,
        mr.projectId as mr_projectId, mr.versionId as mr_versionId, mr.title as mr_title, mr.version as mr_version,
        mr.urlslug as mr_urlslug, mr.description as mr_description, mr.authors as mr_authors, mr.releaseType as mr_releaseType,
        mr.updatePaths as mr_updatePaths, mr.filename as mr_filename, mr.fileUrl as mr_fileUrl, mr.cachedAt as mr_cachedAt, mr.sha512 as mr_sha512,
        CASE WHEN lmi.data IS NOT NULL THEN 1 ELSE 0 END as has_local_image,
        CASE WHEN cfi.data IS NOT NULL THEN 1 ELSE 0 END as has_cf_image,
        CASE WHEN mri.data IS NOT NULL THEN 1 ELSE 0 END as has_mr_image
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    LEFT JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    LEFT JOIN LocalModImageCache lmi ON mm.id = lmi.metadataId
    LEFT JOIN CurseForgeModImageCache cfi ON mm.id = cfi.metadataId
    LEFT JOIN ModrinthModImageCache mri ON mm.id = mri.metadataId
    WHERE mfc.instanceId = ?1
    ORDER BY mfc.filename"#,
    (instance_id: i32) -> ModFileCacheWithMetadataAndImages
);

define_query!(
    ListModFileCacheWithMetadataAndImagesByInstanceAndType,
    r#"SELECT
        mfc.id, mfc.filename, mfc.filesize, mfc.enabled, mfc.addonType, mfc.instanceId, mfc.metadataId,
        mm.murmur2 as mm_murmur2, mm.sha512 as mm_sha512, mm.sha1 as mm_sha1, mm.name as mm_name,
        mm.modid as mm_modid, mm.version as mm_version, mm.description as mm_description,
        mm.authors as mm_authors, mm.modloaders as mm_modloaders, mm.lastUpdatedAt as mm_lastUpdatedAt,
        cf.projectId as cf_projectId, cf.fileId as cf_fileId, cf.name as cf_name, cf.version as cf_version,
        cf.urlslug as cf_urlslug, cf.summary as cf_summary, cf.authors as cf_authors, cf.releaseType as cf_releaseType,
        cf.updatePaths as cf_updatePaths, cf.cachedAt as cf_cachedAt, cf.murmur2 as cf_murmur2,
        mr.projectId as mr_projectId, mr.versionId as mr_versionId, mr.title as mr_title, mr.version as mr_version,
        mr.urlslug as mr_urlslug, mr.description as mr_description, mr.authors as mr_authors, mr.releaseType as mr_releaseType,
        mr.updatePaths as mr_updatePaths, mr.filename as mr_filename, mr.fileUrl as mr_fileUrl, mr.cachedAt as mr_cachedAt, mr.sha512 as mr_sha512,
        CASE WHEN lmi.data IS NOT NULL THEN 1 ELSE 0 END as has_local_image,
        CASE WHEN cfi.data IS NOT NULL THEN 1 ELSE 0 END as has_cf_image,
        CASE WHEN mri.data IS NOT NULL THEN 1 ELSE 0 END as has_mr_image
    FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    LEFT JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    LEFT JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    LEFT JOIN LocalModImageCache lmi ON mm.id = lmi.metadataId
    LEFT JOIN CurseForgeModImageCache cfi ON mm.id = cfi.metadataId
    LEFT JOIN ModrinthModImageCache mri ON mm.id = mri.metadataId
    WHERE mfc.instanceId = ?1 AND mfc.addonType = ?2
    ORDER BY mfc.filename"#,
    (instance_id: i32, addon_type: &str) -> ModFileCacheWithMetadataAndImages
);
