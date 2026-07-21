use carbon_repos::checker::{check_freshness, check_module};
use carbon_repos::registry::QueryCheck;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("check.db")).unwrap();
    let (migrations, _count) = carbon_repos::get_migrations();
    migrations.to_latest(&mut conn).unwrap();
    (dir, conn)
}

/// Every registered `QueryCheck` across every repo module. One line per
/// module; java is first, later tasks extend this list — both the schema
/// checker and the freshness lint run over this same aggregated set, so
/// every later task is covered automatically.
fn all_registered_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = Vec::new();
    all.extend(carbon_repos::repos::java::all_queries());
    all.extend(carbon_repos::repos::app_configuration::all_queries());
    all.extend(carbon_repos::repos::frontend_preference::all_queries());
    all.extend(carbon_repos::repos::account::all_queries());
    all.extend(carbon_repos::repos::skin::all_queries());
    all.extend(carbon_repos::repos::active_downloads::all_queries());
    all.extend(carbon_repos::repos::instance::all_queries());
    all.extend(carbon_repos::repos::server::all_queries());
    all.extend(carbon_repos::repos::version_meta::all_queries());
    all.extend(carbon_repos::repos::mod_file_cache::all_queries());
    all.extend(carbon_repos::repos::mod_metadata::all_queries());
    all
}

#[test]
fn all_registered_queries_pass_against_migrated_schema() {
    let (_d, conn) = migrated_db();
    let all = check_module(&conn, &all_registered_queries());
    assert!(all.is_empty(), "query checker violations:\n{}", all.join("\n"));
}

#[test]
fn freshness_lint_passes_for_all_registered_queries() {
    let v = check_freshness(&all_registered_queries());
    assert!(v.is_empty(), "freshness lint violations:\n{}", v.join("\n"));
}

#[test]
fn freshness_lint_catches_planted_failure() {
    // The fence is fence-tested: a fake UPDATE on VersionInfoCache that
    // never sets lastUpdatedAt must be flagged.
    let planted = [QueryCheck {
        name: "bad_freshness_update",
        sql: "UPDATE VersionInfoCache SET versionInfo = :v WHERE id = :id",
        params: &[":v", ":id"],
        columns: None,
    }];
    let v = check_freshness(&planted);
    assert_eq!(
        v.len(),
        1,
        "freshness lint must flag missing lastUpdatedAt, got: {v:?}"
    );
}

#[test]
fn checker_catches_planted_failures() {
    // The fence is fence-tested (spec T11): a broken query MUST be flagged.
    let (_d, conn) = migrated_db();
    let planted = [
        QueryCheck {
            name: "bad_table",
            sql: "SELECT id FROM NotATable",
            params: &[],
            columns: None,
        },
        QueryCheck {
            name: "bad_param",
            sql: "SELECT id FROM Java WHERE id = :id",
            params: &[":wrong"],
            columns: None,
        },
    ];
    let v = check_module(&conn, &planted);
    assert_eq!(v.len(), 2, "checker must flag both planted failures, got: {v:?}");
}
