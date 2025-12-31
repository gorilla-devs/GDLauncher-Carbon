//! Mod file and metadata queries.

use crate::define_query;

// ModFileCache queries
define_query!(FindModFileCache, "SELECT * FROM ModFileCache WHERE id = ?1");
define_query!(
    FindModFileCacheByInstanceAndFilename,
    "SELECT * FROM ModFileCache WHERE instanceId = ?1 AND filename = ?2"
);
define_query!(
    ListModFileCacheByInstance,
    "SELECT * FROM ModFileCache WHERE instanceId = ?1 ORDER BY filename"
);
define_query!(
    ListModFileCacheByInstanceAndType,
    "SELECT * FROM ModFileCache WHERE instanceId = ?1 AND addonType = ?2 ORDER BY filename"
);
define_query!(
    CountModFileCacheByInstance,
    "SELECT COUNT(*) FROM ModFileCache WHERE instanceId = ?1"
);

define_query!(
    CreateModFileCache,
    r#"INSERT INTO ModFileCache (id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7)"#
);

define_query!(
    UpsertModFileCache,
    r#"INSERT OR REPLACE INTO ModFileCache (id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7)"#
);

define_query!(
    UpdateModFileCacheEnabled,
    "UPDATE ModFileCache SET enabled = ?2, lastUpdatedAt = datetime('now') WHERE id = ?1"
);
define_query!(
    UpdateModFileCacheFilename,
    "UPDATE ModFileCache SET filename = ?2, lastUpdatedAt = datetime('now') WHERE id = ?1"
);
define_query!(DeleteModFileCache, "DELETE FROM ModFileCache WHERE id = ?1");
define_query!(
    DeleteModFileCacheByInstance,
    "DELETE FROM ModFileCache WHERE instanceId = ?1"
);
define_query!(
    DeleteModFileCacheByInstanceAndFilename,
    "DELETE FROM ModFileCache WHERE instanceId = ?1 AND filename = ?2"
);

// ModMetadata queries
define_query!(FindModMetadata, "SELECT * FROM ModMetadata WHERE id = ?1");
define_query!(
    FindModMetadataByMurmur2,
    "SELECT * FROM ModMetadata WHERE murmur2 = ?1"
);
define_query!(ListModMetadata, "SELECT * FROM ModMetadata ORDER BY name");

define_query!(
    CreateModMetadata,
    r#"INSERT INTO ModMetadata (id, lastUpdatedAt, murmur2, sha512, sha1, name, modid, version, description, authors, modloaders)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#
);

define_query!(
    UpsertModMetadata,
    r#"INSERT OR REPLACE INTO ModMetadata (id, lastUpdatedAt, murmur2, sha512, sha1, name, modid, version, description, authors, modloaders)
    VALUES (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#
);

define_query!(DeleteModMetadata, "DELETE FROM ModMetadata WHERE id = ?1");
define_query!(
    DeleteOrphanedModMetadata,
    r#"DELETE FROM ModMetadata WHERE id NOT IN (SELECT metadataId FROM ModFileCache)"#
);
define_query!(
    FindModMetadataBySha512AndMurmur2,
    "SELECT * FROM ModMetadata WHERE sha512 = ?1 AND murmur2 = ?2"
);

// CurseForgeModCache queries
define_query!(
    FindCurseForgeModCache,
    "SELECT * FROM CurseForgeModCache WHERE metadataId = ?1"
);
define_query!(
    FindCurseForgeModCacheByProjectFile,
    "SELECT * FROM CurseForgeModCache WHERE projectId = ?1 AND fileId = ?2"
);
define_query!(
    FindCurseForgeModCacheByMurmur2,
    "SELECT * FROM CurseForgeModCache WHERE murmur2 = ?1"
);

define_query!(
    UpsertCurseForgeModCache,
    r#"INSERT OR REPLACE INTO CurseForgeModCache
    (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#
);

define_query!(
    DeleteCurseForgeModCache,
    "DELETE FROM CurseForgeModCache WHERE metadataId = ?1"
);

// ModrinthModCache queries
define_query!(
    FindModrinthModCache,
    "SELECT * FROM ModrinthModCache WHERE metadataId = ?1"
);
define_query!(
    FindModrinthModCacheByProjectVersion,
    "SELECT * FROM ModrinthModCache WHERE projectId = ?1 AND versionId = ?2"
);
define_query!(
    FindModrinthModCacheBySha512,
    "SELECT * FROM ModrinthModCache WHERE sha512 = ?1"
);

define_query!(
    UpsertModrinthModCache,
    r#"INSERT OR REPLACE INTO ModrinthModCache
    (metadataId, sha512, projectId, versionId, title, version, urlslug, description, authors, releaseType, updatePaths, filename, fileUrl, cachedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#
);

define_query!(
    DeleteModrinthModCache,
    "DELETE FROM ModrinthModCache WHERE metadataId = ?1"
);

// Queries for checking if a mod from a specific platform exists in an instance
define_query!(
    FindModFileCacheByInstanceAndCfProject,
    r#"SELECT mfc.* FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN CurseForgeModCache cf ON mm.id = cf.metadataId
    WHERE mfc.instanceId = ?1 AND cf.projectId = ?2
    LIMIT 1"#
);

define_query!(
    FindModFileCacheByInstanceAndMrProject,
    r#"SELECT mfc.* FROM ModFileCache mfc
    INNER JOIN ModMetadata mm ON mfc.metadataId = mm.id
    INNER JOIN ModrinthModCache mr ON mm.id = mr.metadataId
    WHERE mfc.instanceId = ?1 AND mr.projectId = ?2
    LIMIT 1"#
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
    ORDER BY mfc.filename"#
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
    ORDER BY mfc.filename"#
);

// Complex queries for mod file caching with metadata info
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
    "SELECT * FROM LocalModImageCache WHERE metadataId = ?1"
);
define_query!(
    UpsertLocalModImageCache,
    "INSERT OR REPLACE INTO LocalModImageCache (metadataId, data) VALUES (?1, ?2)"
);
define_query!(
    DeleteLocalModImageCache,
    "DELETE FROM LocalModImageCache WHERE metadataId = ?1"
);

// CurseForgeModImageCache queries
define_query!(
    FindCurseForgeModImageCache,
    "SELECT * FROM CurseForgeModImageCache WHERE metadataId = ?1"
);
define_query!(
    UpsertCurseForgeModImageCache,
    "INSERT OR REPLACE INTO CurseForgeModImageCache (metadataId, url, data, upToDate) VALUES (?1, ?2, ?3, ?4)"
);
define_query!(
    UpdateCurseForgeModImageCacheData,
    "UPDATE CurseForgeModImageCache SET data = ?2, upToDate = ?3 WHERE metadataId = ?1"
);
define_query!(
    DeleteCurseForgeModImageCache,
    "DELETE FROM CurseForgeModImageCache WHERE metadataId = ?1"
);

// ModrinthModImageCache queries
define_query!(
    FindModrinthModImageCache,
    "SELECT * FROM ModrinthModImageCache WHERE metadataId = ?1"
);
define_query!(
    UpsertModrinthModImageCache,
    "INSERT OR REPLACE INTO ModrinthModImageCache (metadataId, url, data, upToDate) VALUES (?1, ?2, ?3, ?4)"
);
define_query!(
    UpdateModrinthModImageCacheData,
    "UPDATE ModrinthModImageCache SET data = ?2, upToDate = ?3 WHERE metadataId = ?1"
);
define_query!(
    DeleteModrinthModImageCache,
    "DELETE FROM ModrinthModImageCache WHERE metadataId = ?1"
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
    WHERE mfc.id = ?1"#
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
    ORDER BY mfc.filename"#
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
    ORDER BY mfc.filename"#
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
    ORDER BY mfc.filename"#
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
    ORDER BY mfc.filename"#
);
