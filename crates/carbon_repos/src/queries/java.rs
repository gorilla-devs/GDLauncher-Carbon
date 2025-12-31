//! Java and JavaProfile queries.

use crate::define_query;

// Java read queries
define_query!(FindJavaById, "SELECT * FROM Java WHERE id = ?1");
define_query!(FindJavaByPath, "SELECT * FROM Java WHERE path = ?1");
define_query!(
    ListJavas,
    "SELECT * FROM Java ORDER BY major DESC, fullVersion DESC"
);
define_query!(
    ListValidJavas,
    "SELECT * FROM Java WHERE isValid = 1 ORDER BY major DESC, fullVersion DESC"
);
define_query!(CountJavas, "SELECT COUNT(*) FROM Java");

// Java create/update queries
define_query!(
    CreateJava,
    r#"INSERT INTO Java (id, path, major, fullVersion, type, os, arch, vendor, isValid)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#
);

define_query!(
    UpsertJava,
    r#"INSERT OR REPLACE INTO Java (id, path, major, fullVersion, type, os, arch, vendor, isValid)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#
);

define_query!(
    UpdateJavaValid,
    "UPDATE Java SET isValid = ?2 WHERE id = ?1"
);
define_query!(DeleteJava, "DELETE FROM Java WHERE id = ?1");
define_query!(DeleteJavaByPath, "DELETE FROM Java WHERE path = ?1");

// JavaProfile read queries
define_query!(
    FindJavaProfileByName,
    "SELECT * FROM JavaProfile WHERE name = ?1"
);
define_query!(ListJavaProfiles, "SELECT * FROM JavaProfile ORDER BY name");
define_query!(
    ListSystemJavaProfiles,
    "SELECT * FROM JavaProfile WHERE isSystemProfile = 1 ORDER BY name"
);
define_query!(
    ListUserJavaProfiles,
    "SELECT * FROM JavaProfile WHERE isSystemProfile = 0 ORDER BY name"
);

// JavaProfile with Java join
define_query!(
    FindJavaProfileWithJava,
    r#"SELECT jp.*, j.id as java_id, j.path, j.major, j.fullVersion, j.type, j.os, j.arch, j.vendor, j.isValid
    FROM JavaProfile jp
    LEFT JOIN Java j ON jp.javaId = j.id
    WHERE jp.name = ?1"#
);

define_query!(
    ListJavaProfilesWithJava,
    r#"SELECT jp.*, j.id as java_id, j.path, j.major, j.fullVersion, j.type, j.os, j.arch, j.vendor, j.isValid
    FROM JavaProfile jp
    LEFT JOIN Java j ON jp.javaId = j.id
    ORDER BY jp.name"#
);

// Typed JOIN query - explicitly names columns for JavaProfileWithPath model
define_query!(
    ListJavaProfilesWithJavaPath,
    r#"SELECT
        jp.name,
        jp.isSystemProfile,
        jp.javaId,
        j.path as java_path
    FROM JavaProfile jp
    LEFT JOIN Java j ON jp.javaId = j.id
    ORDER BY jp.name"#
);

// JavaProfile create/update queries
define_query!(
    CreateJavaProfile,
    "INSERT INTO JavaProfile (name, isSystemProfile, javaId) VALUES (?1, ?2, ?3)"
);

define_query!(
    UpsertJavaProfile,
    "INSERT OR REPLACE INTO JavaProfile (name, isSystemProfile, javaId) VALUES (?1, ?2, ?3)"
);

define_query!(
    UpdateJavaProfileJavaId,
    "UPDATE JavaProfile SET javaId = ?2 WHERE name = ?1"
);
define_query!(DeleteJavaProfile, "DELETE FROM JavaProfile WHERE name = ?1");
define_query!(
    DeleteSystemJavaProfiles,
    "DELETE FROM JavaProfile WHERE isSystemProfile = 1"
);
