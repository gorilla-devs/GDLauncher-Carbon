//! Modpack cache queries.

use crate::define_query;
use crate::models::{
    CurseForgeModpackCache, CurseForgeModpackImageCache, ModpackCacheEntry, ModrinthModpackCache,
    ModrinthModpackImageCache,
};

// CurseForgeModpackCache queries
define_query!(
    FindCurseForgeModpackCache,
    "SELECT * FROM CurseForgeModpackCache WHERE projectId = ?1 AND fileId = ?2",
    (project_id: i32, file_id: i32) -> CurseForgeModpackCache
);
define_query!(
    ListCurseForgeModpackCache,
    "SELECT * FROM CurseForgeModpackCache ORDER BY updatedAt DESC",
    () -> CurseForgeModpackCache
);
define_query!(
    UpsertCurseForgeModpackCache,
    r#"INSERT OR REPLACE INTO CurseForgeModpackCache (projectId, fileId, modpackName, versionName, urlSlug, updatedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))"#,
    (project_id: i32, file_id: i32, modpack_name: &str, version_name: &str, url_slug: &str)
);
define_query!(
    DeleteCurseForgeModpackCache,
    "DELETE FROM CurseForgeModpackCache WHERE projectId = ?1 AND fileId = ?2",
    (project_id: i32, file_id: i32)
);
define_query!(
    CountCurseForgeModpackCache,
    "SELECT COUNT(*) FROM CurseForgeModpackCache",
    () => i32
);

// ModrinthModpackCache queries
define_query!(
    FindModrinthModpackCache,
    "SELECT * FROM ModrinthModpackCache WHERE projectId = ?1 AND versionId = ?2",
    (project_id: &str, version_id: &str) -> ModrinthModpackCache
);
define_query!(
    ListModrinthModpackCache,
    "SELECT * FROM ModrinthModpackCache ORDER BY updatedAt DESC",
    () -> ModrinthModpackCache
);
define_query!(
    UpsertModrinthModpackCache,
    r#"INSERT OR REPLACE INTO ModrinthModpackCache (projectId, versionId, modpackName, versionName, urlSlug, updatedAt)
    VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))"#,
    (project_id: &str, version_id: &str, modpack_name: &str, version_name: &str, url_slug: &str)
);
define_query!(
    DeleteModrinthModpackCache,
    "DELETE FROM ModrinthModpackCache WHERE projectId = ?1 AND versionId = ?2",
    (project_id: &str, version_id: &str)
);
define_query!(
    CountModrinthModpackCache,
    "SELECT COUNT(*) FROM ModrinthModpackCache",
    () => i32
);

// CurseForgeModpackImageCache queries
define_query!(
    FindCurseForgeModpackImageCache,
    "SELECT * FROM CurseForgeModpackImageCache WHERE projectId = ?1 AND fileId = ?2",
    (project_id: i32, file_id: i32) -> CurseForgeModpackImageCache
);
define_query!(
    UpsertCurseForgeModpackImageCache,
    r#"INSERT OR REPLACE INTO CurseForgeModpackImageCache (projectId, fileId, url, data)
    VALUES (?1, ?2, ?3, ?4)"#,
    (project_id: i32, file_id: i32, url: &str, data: Option<&[u8]>)
);
define_query!(
    DeleteCurseForgeModpackImageCache,
    "DELETE FROM CurseForgeModpackImageCache WHERE projectId = ?1 AND fileId = ?2",
    (project_id: i32, file_id: i32)
);
define_query!(
    CountCurseForgeModpackImageCache,
    "SELECT COUNT(*) FROM CurseForgeModpackImageCache",
    () => i32
);

// ModrinthModpackImageCache queries
define_query!(
    FindModrinthModpackImageCache,
    "SELECT * FROM ModrinthModpackImageCache WHERE projectId = ?1 AND versionId = ?2",
    (project_id: &str, version_id: &str) -> ModrinthModpackImageCache
);
define_query!(
    UpsertModrinthModpackImageCache,
    r#"INSERT OR REPLACE INTO ModrinthModpackImageCache (projectId, versionId, url, data)
    VALUES (?1, ?2, ?3, ?4)"#,
    (project_id: &str, version_id: &str, url: &str, data: Option<&[u8]>)
);
define_query!(
    DeleteModrinthModpackImageCache,
    "DELETE FROM ModrinthModpackImageCache WHERE projectId = ?1 AND versionId = ?2",
    (project_id: &str, version_id: &str)
);
define_query!(
    CountModrinthModpackImageCache,
    "SELECT COUNT(*) FROM ModrinthModpackImageCache",
    () => i32
);

// Join queries for modpacks with image availability
define_query!(
    FindCurseForgeModpackCacheWithImage,
    r#"SELECT mpc.modpackName, mpc.versionName, mpc.urlSlug, mpc.updatedAt, img.data IS NOT NULL as hasImage
    FROM CurseForgeModpackCache mpc
    LEFT JOIN CurseForgeModpackImageCache img ON mpc.projectId = img.projectId AND mpc.fileId = img.fileId
    WHERE mpc.projectId = ?1 AND mpc.fileId = ?2"#,
    (project_id: i32, file_id: i32) -> ModpackCacheEntry
);

define_query!(
    FindModrinthModpackCacheWithImage,
    r#"SELECT mpc.modpackName, mpc.versionName, mpc.urlSlug, mpc.updatedAt, img.data IS NOT NULL as hasImage
    FROM ModrinthModpackCache mpc
    LEFT JOIN ModrinthModpackImageCache img ON mpc.projectId = img.projectId AND mpc.versionId = img.versionId
    WHERE mpc.projectId = ?1 AND mpc.versionId = ?2"#,
    (project_id: &str, version_id: &str) -> ModpackCacheEntry
);
