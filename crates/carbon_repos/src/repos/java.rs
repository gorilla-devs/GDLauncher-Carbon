//! Repository queries for the `Java` and `JavaProfile` tables.

use crate::queries;

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct JavaRow {
    pub id: String,
    pub path: String,
    pub major: i32,
    pub full_version: String,
    #[column("type")]
    pub r#type: String,
    pub os: String,
    pub arch: String,
    pub vendor: String,
    pub is_valid: bool,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct JavaProfileRow {
    pub name: String,
    pub is_system_profile: bool,
    pub java_id: Option<String>,
}

/// A single `Java.path` value, used to collect the set of java paths a profile
/// links to without materializing whole `JavaRow`s.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct JavaPathRow {
    pub path: String,
}

queries! {
    fn get_all_java() -> Vec<JavaRow> =
        "SELECT id, path, major, fullVersion, type, os, arch, vendor, isValid FROM Java";
    fn get_java_by_id(id: &str) -> Option<JavaRow> =
        "SELECT id, path, major, fullVersion, type, os, arch, vendor, isValid FROM Java WHERE id = :id";
    fn get_java_by_path(path: &str) -> Option<JavaRow> =
        "SELECT id, path, major, fullVersion, type, os, arch, vendor, isValid FROM Java WHERE path = :path";
    fn get_java_by_type(ty: &str) -> Vec<JavaRow> =
        "SELECT id, path, major, fullVersion, type, os, arch, vendor, isValid FROM Java WHERE type = :ty";
    fn count_java() -> i64 =
        "SELECT COUNT(*) FROM Java";
    fn set_java_validity(id: &str, valid: bool) -> usize =
        "UPDATE Java SET isValid = :valid WHERE id = :id";
    fn set_java_validity_by_path(path: &str, valid: bool) -> usize =
        "UPDATE Java SET isValid = :valid WHERE path = :path";
    fn update_java_component(id: &str, major: i32, full_version: &str, arch: &str, os: &str, vendor: &str) -> usize =
        "UPDATE Java SET major = :major, fullVersion = :full_version, arch = :arch, os = :os, vendor = :vendor, isValid = 1 WHERE id = :id";
    fn delete_java(id: &str) -> usize =
        "DELETE FROM Java WHERE id = :id";
    fn delete_java_by_path(path: &str) -> usize =
        "DELETE FROM Java WHERE path = :path";
    fn get_all_profiles() -> Vec<JavaProfileRow> =
        "SELECT name, isSystemProfile, javaId FROM JavaProfile";
    fn get_profile(name: &str) -> Option<JavaProfileRow> =
        "SELECT name, isSystemProfile, javaId FROM JavaProfile WHERE name = :name";
    fn get_profile_linked_java_paths() -> Vec<JavaPathRow> =
        "SELECT j.path FROM JavaProfile p INNER JOIN Java j ON p.javaId = j.id";
    fn upsert_profile(name: &str, is_system: bool) -> usize =
        "INSERT INTO JavaProfile (name, isSystemProfile) VALUES (:name, :is_system)
         ON CONFLICT(name) DO UPDATE SET isSystemProfile = excluded.isSystemProfile";
    fn set_profile_java(name: &str, java_id: Option<&str>) -> usize =
        "UPDATE JavaProfile SET javaId = :java_id WHERE name = :name";
    fn delete_profile(name: &str) -> usize =
        "DELETE FROM JavaProfile WHERE name = :name";
}

/// The `INSERT` executed by `insert_java` and validated by `INSERT_JAVA_CHECK`.
/// Shared by both so the checker always covers the exact SQL the fn runs — a
/// second verbatim copy could drift and silently void the guarantee.
const INSERT_JAVA_SQL: &str =
    "INSERT INTO Java (id, path, major, fullVersion, type, os, arch, vendor, isValid)
         VALUES (:id, :path, :major, :fv, :ty, :os, :arch, :vendor, :valid)";

/// Inserts a `Java` row. Hand-written (not macro-generated) because the
/// macro's arg list only takes scalar params, not a struct.
pub fn insert_java_conn(conn: &rusqlite::Connection, j: &JavaRow) -> Result<usize, rusqlite::Error> {
    let mut st = conn.prepare_cached(INSERT_JAVA_SQL)?;
    st.execute(rusqlite::named_params! {
        ":id": j.id, ":path": j.path, ":major": j.major, ":fv": j.full_version,
        ":ty": j.r#type, ":os": j.os, ":arch": j.arch, ":vendor": j.vendor, ":valid": j.is_valid,
    })
}

/// Pool-routing wrapper for [`insert_java_conn`]: takes the row by value (the
/// executor closure is `'static`) and routes to the writer.
pub async fn insert_java(db: &crate::db_exec::Db, j: JavaRow) -> crate::db_error::DbResult<usize> {
    db.write(move |conn| Ok(insert_java_conn(conn, &j)?)).await
}

/// `QueryCheck` entry for `insert_java`, covering the hand-written fn above so
/// the checker validates it against the migrated schema like every
/// macro-generated query.
const INSERT_JAVA_CHECK: crate::registry::QueryCheck = crate::registry::QueryCheck {
    name: "insert_java",
    sql: INSERT_JAVA_SQL,
    params: &[":id", ":path", ":major", ":fv", ":ty", ":os", ":arch", ":vendor", ":valid"],
    columns: None,
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus
/// the hand-written `insert_java` entry.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    let mut all: Vec<crate::registry::QueryCheck> = QUERIES.to_vec();
    all.push(INSERT_JAVA_CHECK);
    all
}
