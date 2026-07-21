use carbon_repos::checker::{
    check_manifests, check_module, check_nullability, check_query_plans,
};
use carbon_repos::registry::QueryCheck;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("check.db")).unwrap();
    let (migrations, _count) = carbon_repos::get_migrations();
    migrations.to_latest(&mut conn).unwrap();
    (dir, conn)
}

/// Every registered `QueryCheck` across every repo module. Every checker rule
/// runs over this same aggregated set, so every repo is covered automatically.
fn all_registered_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = Vec::new();
    all.extend(carbon_repos::repos::java::all_queries());
    all.extend(carbon_repos::repos::app_configuration::all_queries());
    all.extend(carbon_repos::repos::frontend_preference::all_queries());
    all.extend(carbon_repos::repos::http_cache::all_queries());
    all.extend(carbon_repos::repos::account::all_queries());
    all.extend(carbon_repos::repos::skin::all_queries());
    all.extend(carbon_repos::repos::active_downloads::all_queries());
    all.extend(carbon_repos::repos::instance::all_queries());
    all.extend(carbon_repos::repos::server::all_queries());
    all.extend(carbon_repos::repos::version_meta::all_queries());
    all.extend(carbon_repos::repos::mod_file_cache::all_queries());
    all.extend(carbon_repos::repos::mod_metadata::all_queries());
    all.extend(carbon_repos::repos::modpack_cache::all_queries());
    all
}

#[test]
fn all_registered_queries_pass_against_migrated_schema() {
    let (_d, conn) = migrated_db();
    let all = check_module(&conn, &all_registered_queries());
    assert!(all.is_empty(), "query checker violations:\n{}", all.join("\n"));
}

#[test]
fn manifest_freshness_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_manifests(&conn, &all_registered_queries());
    assert!(v.is_empty(), "freshness lint violations:\n{}", v.join("\n"));
}

#[test]
fn nullability_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_nullability(&conn, &all_registered_queries());
    assert!(v.is_empty(), "nullability lint violations:\n{}", v.join("\n"));
}

#[test]
fn query_plan_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_query_plans(&conn, &all_registered_queries());
    assert!(v.is_empty(), "query plan lint violations:\n{}", v.join("\n"));
}

// ---------------------------------------------------------------------------
// Planted-failure self-tests: every rule is fence-tested with a query that must
// be flagged, so a broken checker can't silently pass everything.
// ---------------------------------------------------------------------------

#[test]
fn checker_catches_planted_structural_failures() {
    let (_d, conn) = migrated_db();
    let planted = [
        // Unknown table → does not prepare.
        QueryCheck {
            name: "bad_table",
            sql: "SELECT id FROM NotATable",
            params: &[],
            columns: None,
        },
        // Declared param the SQL does not bind.
        QueryCheck {
            name: "bad_declared_param",
            sql: "SELECT id FROM Java WHERE id = :id",
            params: &[":wrong"],
            columns: None,
        },
        // SQL param the registry does not declare (exact set equality flags a
        // bound parameter with no matching declaration).
        QueryCheck {
            name: "undeclared_param",
            sql: "SELECT id FROM Java WHERE id = :id",
            params: &[],
            columns: None,
        },
    ];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter().any(|m| m.contains("bad_table")),
        "must flag unknown table, got: {v:?}"
    );
    assert!(
        v.iter().any(|m| m.contains("bad_declared_param")),
        "must flag a declared param missing from SQL, got: {v:?}"
    );
    assert!(
        v.iter().any(|m| m.contains("undeclared_param")),
        "must flag an undeclared SQL param, got: {v:?}"
    );
}

#[test]
fn checker_does_not_flag_question_mark_in_string_literal() {
    // A multi-param query with a literal '?' inside a string value must NOT be
    // flagged for a positional placeholder: the scan skips string literals.
    let (_d, conn) = migrated_db();
    let ok = [QueryCheck {
        name: "literal_question_mark",
        sql: "SELECT id FROM Java WHERE major = :major OR os = 'what?'",
        params: &[":major", ":unused_but_present"],
        columns: None,
    }];
    // The '?'-in-literal rule must not fire; only the param-set mismatch for the
    // deliberately-unused declared name should show up.
    let v = check_module(&conn, &ok);
    assert!(
        !v.iter().any(|m| m.contains("positional")),
        "literal '?' must not be read as a positional placeholder, got: {v:?}"
    );
}

#[test]
fn checker_flags_real_positional_param_in_multiparam_query() {
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "positional_multiparam",
        sql: "SELECT id FROM Java WHERE major = :major AND arch = ?",
        params: &[":major", ":arch"],
        columns: None,
    }];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter().any(|m| m.contains("positional")),
        "must flag a bare '?' in a multi-param query, got: {v:?}"
    );
}

#[test]
fn manifest_freshness_lint_catches_planted_failure() {
    // An UPDATE on a freshness table that never sets lastUpdatedAt must be
    // flagged by the authorizer-driven manifest lint.
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "bad_freshness_update",
        sql: "UPDATE VersionInfoCache SET versionInfo = :v WHERE id = :id",
        params: &[":v", ":id"],
        columns: None,
    }];
    let v = check_manifests(&conn, &planted);
    assert_eq!(
        v.len(),
        1,
        "freshness lint must flag missing lastUpdatedAt, got: {v:?}"
    );
}

#[test]
fn manifest_freshness_lint_catches_upsert_missing_freshness() {
    // The DO UPDATE SET branch of an upsert must also carry the freshness
    // column — here it sets only `versionInfo`, so it must be flagged.
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "bad_freshness_upsert",
        sql: "INSERT INTO VersionInfoCache (id, versionInfo, lastUpdatedAt) VALUES (:id, :v, :t) \
              ON CONFLICT(id) DO UPDATE SET versionInfo = excluded.versionInfo",
        params: &[":id", ":v", ":t"],
        columns: None,
    }];
    let v = check_manifests(&conn, &planted);
    assert_eq!(
        v.len(),
        1,
        "freshness lint must flag an upsert whose DO UPDATE omits lastUpdatedAt, got: {v:?}"
    );
}

mod planted_rows {
    use carbon_repos::queries;

    // A nullable source column (`Account.accessToken`) declared non-null.
    #[derive(carbon_macro::FromRow, Debug, PartialEq)]
    pub struct BadNullableRow {
        pub access_token: String,
    }

    // A SQL expression with no resolvable origin, declared non-null and without
    // an explicit override.
    #[derive(carbon_macro::FromRow, Debug, PartialEq)]
    pub struct BadExprRow {
        pub flag: bool,
    }

    queries! {
        fn bad_nullable(uuid: &str) -> Option<BadNullableRow> =
            "SELECT accessToken FROM Account WHERE uuid = :uuid";
        fn bad_expr(id: &str) -> Option<BadExprRow> =
            "SELECT (id IS NOT NULL) AS flag FROM Java WHERE id = :id";
    }
}

#[test]
fn nullability_lint_catches_nullable_source_declared_non_null() {
    let (_d, conn) = migrated_db();
    let v = check_nullability(&conn, planted_rows::QUERIES);
    assert!(
        v.iter().any(|m| m.contains("bad_nullable") && m.contains("access")),
        "must flag a nullable source column declared non-null, got: {v:?}"
    );
}

#[test]
fn nullability_lint_catches_unmarked_expression_column() {
    let (_d, conn) = migrated_db();
    let v = check_nullability(&conn, planted_rows::QUERIES);
    assert!(
        v.iter().any(|m| m.contains("bad_expr") && m.contains("expression")),
        "must flag an expression column that is neither Option nor overridden, got: {v:?}"
    );
}

#[test]
fn query_plan_lint_catches_planted_full_scan() {
    // ModMetadata is a guarded table with no index on `name`, so a WHERE on it
    // full-scans and must be flagged.
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "scan_mod_metadata",
        sql: "SELECT id FROM ModMetadata WHERE name = :name",
        params: &[":name"],
        columns: None,
    }];
    let v = check_query_plans(&conn, &planted);
    assert!(
        v.iter().any(|m| m.contains("scan_mod_metadata") && m.contains("ModMetadata")),
        "must flag a full scan of a guarded table, got: {v:?}"
    );
}
