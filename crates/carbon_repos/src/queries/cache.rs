//! Cache queries (HTTP, version info, LWJGL, assets).

use crate::define_query;

// HTTPCache queries
define_query!(FindHttpCache, "SELECT * FROM HTTPCache WHERE url = ?1");
define_query!(
    UpsertHttpCache,
    r#"INSERT OR REPLACE INTO HTTPCache (url, status_code, data, expiresAt, lastModified, etag)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#
);
define_query!(DeleteHttpCache, "DELETE FROM HTTPCache WHERE url = ?1");
define_query!(
    DeleteExpiredHttpCache,
    "DELETE FROM HTTPCache WHERE expiresAt < datetime('now')"
);
define_query!(ClearHttpCache, "DELETE FROM HTTPCache");

// ActiveDownloads queries
define_query!(
    FindActiveDownload,
    "SELECT * FROM ActiveDownloads WHERE url = ?1"
);
define_query!(
    FindActiveDownloadByFileId,
    "SELECT * FROM ActiveDownloads WHERE file_id = ?1"
);
define_query!(ListActiveDownloads, "SELECT * FROM ActiveDownloads");
define_query!(
    CreateActiveDownload,
    "INSERT INTO ActiveDownloads (url, file_id) VALUES (?1, ?2)"
);
define_query!(
    DeleteActiveDownload,
    "DELETE FROM ActiveDownloads WHERE url = ?1"
);
define_query!(
    DeleteActiveDownloadByFileId,
    "DELETE FROM ActiveDownloads WHERE file_id = ?1"
);
define_query!(ClearActiveDownloads, "DELETE FROM ActiveDownloads");

// VersionInfoCache queries
define_query!(
    FindVersionInfoCache,
    "SELECT * FROM VersionInfoCache WHERE id = ?1"
);
define_query!(
    ListVersionInfoCache,
    "SELECT * FROM VersionInfoCache ORDER BY id"
);
define_query!(
    UpsertVersionInfoCache,
    r#"INSERT OR REPLACE INTO VersionInfoCache (id, lastUpdatedAt, versionInfo)
    VALUES (?1, datetime('now'), ?2)"#
);
define_query!(
    DeleteVersionInfoCache,
    "DELETE FROM VersionInfoCache WHERE id = ?1"
);
define_query!(ClearVersionInfoCache, "DELETE FROM VersionInfoCache");

// PartialVersionInfoCache queries
define_query!(
    FindPartialVersionInfoCache,
    "SELECT * FROM PartialVersionInfoCache WHERE id = ?1"
);
define_query!(
    ListPartialVersionInfoCache,
    "SELECT * FROM PartialVersionInfoCache ORDER BY id"
);
define_query!(
    UpsertPartialVersionInfoCache,
    r#"INSERT OR REPLACE INTO PartialVersionInfoCache (id, lastUpdatedAt, partialVersionInfo)
    VALUES (?1, datetime('now'), ?2)"#
);
define_query!(
    DeletePartialVersionInfoCache,
    "DELETE FROM PartialVersionInfoCache WHERE id = ?1"
);
define_query!(
    ClearPartialVersionInfoCache,
    "DELETE FROM PartialVersionInfoCache"
);

// LwjglMetaCache queries
define_query!(
    FindLwjglMetaCache,
    "SELECT * FROM LwjglMetaCache WHERE id = ?1"
);
define_query!(
    ListLwjglMetaCache,
    "SELECT * FROM LwjglMetaCache ORDER BY id"
);
define_query!(
    UpsertLwjglMetaCache,
    r#"INSERT OR REPLACE INTO LwjglMetaCache (id, lastUpdatedAt, lwjgl)
    VALUES (?1, datetime('now'), ?2)"#
);
define_query!(
    DeleteLwjglMetaCache,
    "DELETE FROM LwjglMetaCache WHERE id = ?1"
);
define_query!(ClearLwjglMetaCache, "DELETE FROM LwjglMetaCache");

// AssetsMetaCache queries
define_query!(
    FindAssetsMetaCache,
    "SELECT * FROM AssetsMetaCache WHERE id = ?1"
);
define_query!(
    ListAssetsMetaCache,
    "SELECT * FROM AssetsMetaCache ORDER BY id"
);
define_query!(
    UpsertAssetsMetaCache,
    r#"INSERT OR REPLACE INTO AssetsMetaCache (id, lastUpdatedAt, assetsIndex)
    VALUES (?1, datetime('now'), ?2)"#
);
define_query!(
    DeleteAssetsMetaCache,
    "DELETE FROM AssetsMetaCache WHERE id = ?1"
);
define_query!(ClearAssetsMetaCache, "DELETE FROM AssetsMetaCache");
