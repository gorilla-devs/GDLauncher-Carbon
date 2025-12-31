//! Modpack cache queries.

use crate::define_query;

// CurseForgeModpackCache queries
define_query!(
    FindCurseForgeModpackCache,
    "SELECT * FROM CurseForgeModpackCache WHERE projectId = ?1 AND fileId = ?2"
);
define_query!(
    ListCurseForgeModpackCache,
    "SELECT * FROM CurseForgeModpackCache ORDER BY updatedAt DESC"
);
define_query!(
    UpsertCurseForgeModpackCache,
    r#"INSERT OR REPLACE INTO CurseForgeModpackCache (projectId, fileId, modpackName, versionName, urlSlug, updatedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))"#
);
define_query!(
    DeleteCurseForgeModpackCache,
    "DELETE FROM CurseForgeModpackCache WHERE projectId = ?1 AND fileId = ?2"
);
define_query!(
    CountCurseForgeModpackCache,
    "SELECT COUNT(*) FROM CurseForgeModpackCache"
);

// ModrinthModpackCache queries
define_query!(
    FindModrinthModpackCache,
    "SELECT * FROM ModrinthModpackCache WHERE projectId = ?1 AND versionId = ?2"
);
define_query!(
    ListModrinthModpackCache,
    "SELECT * FROM ModrinthModpackCache ORDER BY updatedAt DESC"
);
define_query!(
    UpsertModrinthModpackCache,
    r#"INSERT OR REPLACE INTO ModrinthModpackCache (projectId, versionId, modpackName, versionName, urlSlug, updatedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))"#
);
define_query!(
    DeleteModrinthModpackCache,
    "DELETE FROM ModrinthModpackCache WHERE projectId = ?1 AND versionId = ?2"
);
define_query!(
    CountModrinthModpackCache,
    "SELECT COUNT(*) FROM ModrinthModpackCache"
);

// CurseForgeModpackImageCache queries
define_query!(
    FindCurseForgeModpackImageCache,
    "SELECT * FROM CurseForgeModpackImageCache WHERE projectId = ?1 AND fileId = ?2"
);
define_query!(
    UpsertCurseForgeModpackImageCache,
    r#"INSERT OR REPLACE INTO CurseForgeModpackImageCache (projectId, fileId, url, data)
    VALUES (?1, ?2, ?3, ?4)"#
);
define_query!(
    DeleteCurseForgeModpackImageCache,
    "DELETE FROM CurseForgeModpackImageCache WHERE projectId = ?1 AND fileId = ?2"
);
define_query!(
    CountCurseForgeModpackImageCache,
    "SELECT COUNT(*) FROM CurseForgeModpackImageCache"
);

// ModrinthModpackImageCache queries
define_query!(
    FindModrinthModpackImageCache,
    "SELECT * FROM ModrinthModpackImageCache WHERE projectId = ?1 AND versionId = ?2"
);
define_query!(
    UpsertModrinthModpackImageCache,
    r#"INSERT OR REPLACE INTO ModrinthModpackImageCache (projectId, versionId, url, data)
    VALUES (?1, ?2, ?3, ?4)"#
);
define_query!(
    DeleteModrinthModpackImageCache,
    "DELETE FROM ModrinthModpackImageCache WHERE projectId = ?1 AND versionId = ?2"
);
define_query!(
    CountModrinthModpackImageCache,
    "SELECT COUNT(*) FROM ModrinthModpackImageCache"
);

// Join queries for modpacks with images
define_query!(
    FindCurseForgeModpackCacheWithImage,
    r#"SELECT mpc.*, img.url as img_url, img.data as img_data
    FROM CurseForgeModpackCache mpc
    LEFT JOIN CurseForgeModpackImageCache img ON mpc.projectId = img.projectId AND mpc.fileId = img.fileId
    WHERE mpc.projectId = ?1 AND mpc.fileId = ?2"#
);

define_query!(
    FindModrinthModpackCacheWithImage,
    r#"SELECT mpc.*, img.url as img_url, img.data as img_data
    FROM ModrinthModpackCache mpc
    LEFT JOIN ModrinthModpackImageCache img ON mpc.projectId = img.projectId AND mpc.versionId = img.versionId
    WHERE mpc.projectId = ?1 AND mpc.versionId = ?2"#
);
