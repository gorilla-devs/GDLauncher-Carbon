use carbon_repos::checker::check_module;
use carbon_repos::registry::QueryCheck;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("check.db")).unwrap();
    let (migrations, _count) = carbon_repos::get_migrations();
    migrations.to_latest(&mut conn).unwrap();
    (dir, conn)
}

#[test]
fn all_registered_queries_pass_against_migrated_schema() {
    let (_d, conn) = migrated_db();
    // one line per repo module; java is first, later plans extend this list
    let mut all: Vec<String> = check_module(&conn, &carbon_repos::repos::java::all_queries());
    all.extend(check_module(
        &conn,
        &carbon_repos::repos::app_configuration::all_queries(),
    ));
    all.extend(check_module(
        &conn,
        &carbon_repos::repos::frontend_preference::all_queries(),
    ));
    all.extend(check_module(&conn, &carbon_repos::repos::account::all_queries()));
    all.extend(check_module(&conn, &carbon_repos::repos::skin::all_queries()));
    all.extend(check_module(
        &conn,
        &carbon_repos::repos::active_downloads::all_queries(),
    ));
    assert!(all.is_empty(), "query checker violations:\n{}", all.join("\n"));
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
