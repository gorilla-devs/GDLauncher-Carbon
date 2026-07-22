//! Dedicated execution tests for the `DynamicQuery` construction site used by
//! `carbon_app`'s cache-cleanup routine, which builds SQL over a runtime
//! table name (an identifier cannot be a bound `:param`, so this is exempt
//! from the static checker per registry.rs's `DynamicQuery` doc comment).

use carbon_repos::db_exec::test_support::{rg, wg};
use carbon_repos::db_exec::{ReadGuard, WriteGuard};
use carbon_repos::registry::DynamicQuery;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

#[test]
fn query_scalar_i64_counts_rows_in_a_runtime_named_table() {
    let (_d, conn) = migrated_db();
    conn.execute(
        "INSERT INTO HTTPCache (url, status_code, data) VALUES (?1, 200, ?2)",
        rusqlite::params!["https://a", vec![1u8, 2, 3]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO HTTPCache (url, status_code, data) VALUES (?1, 200, ?2)",
        rusqlite::params!["https://b", vec![4u8, 5, 6]],
    )
    .unwrap();

    let table = "HTTPCache";
    let dq = DynamicQuery {
        sql: format!("SELECT COUNT(*) FROM {table}"),
        params: vec![],
    };
    assert_eq!(dq.query_scalar_i64(&rg(&conn)).unwrap(), 2);
}

#[test]
fn query_scalar_i64_on_empty_table_is_zero() {
    let (_d, conn) = migrated_db();
    let dq = DynamicQuery {
        sql: "SELECT COUNT(*) FROM HTTPCache".to_string(),
        params: vec![],
    };
    assert_eq!(dq.query_scalar_i64(&rg(&conn)).unwrap(), 0);
}

#[test]
fn execute_deletes_a_chunk_from_a_runtime_named_table() {
    let (_d, mut conn) = migrated_db();
    for i in 0..5 {
        conn.execute(
            "INSERT INTO HTTPCache (url, status_code, data) VALUES (?1, 200, ?2)",
            rusqlite::params![format!("https://{i}"), vec![0u8]],
        )
        .unwrap();
    }

    let table = "HTTPCache";
    let chunk = 2;
    let dq = DynamicQuery {
        sql: format!(
            "DELETE FROM {table} WHERE rowid IN (SELECT rowid FROM {table} LIMIT {chunk})"
        ),
        params: vec![],
    };

    assert_eq!(dq.execute(&wg(&mut conn)).unwrap(), 2);
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM HTTPCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 3);

    // Draining loop matches the call site: repeat until a 0-row delete.
    assert_eq!(dq.execute(&wg(&mut conn)).unwrap(), 2);
    assert_eq!(dq.execute(&wg(&mut conn)).unwrap(), 1);
    assert_eq!(dq.execute(&wg(&mut conn)).unwrap(), 0);
}
