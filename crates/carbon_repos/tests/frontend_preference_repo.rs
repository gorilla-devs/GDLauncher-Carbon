use carbon_repos::db_exec::test_support::wg;
use carbon_repos::dbtypes::{DbDateTime, from_millis};
use carbon_repos::repos::frontend_preference as fp;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

#[test]
fn upsert_inserts_then_updates_value_and_updated_at() {
    let (_d, mut conn) = migrated_db();
    let t1 = DbDateTime(from_millis(1_700_000_000_000).unwrap());
    assert_eq!(
        fp::upsert_preference_conn(&wg(&mut conn), "last_seen_version", "1.0.0", t1).unwrap(),
        1
    );
    let row = fp::get_preference_conn(&wg(&mut conn), "last_seen_version")
        .unwrap()
        .unwrap();
    assert_eq!(row.key, "last_seen_version");
    assert_eq!(row.value, "1.0.0");
    assert_eq!(row.updated_at, t1.0);

    // Conflict on key updates value AND updatedAt.
    let t2 = DbDateTime(from_millis(1_784_557_692_104).unwrap());
    fp::upsert_preference_conn(&wg(&mut conn), "last_seen_version", "2.0.0", t2).unwrap();
    let row = fp::get_preference_conn(&wg(&mut conn), "last_seen_version")
        .unwrap()
        .unwrap();
    assert_eq!(row.value, "2.0.0");
    assert_eq!(row.updated_at, t2.0);
}

#[test]
fn get_missing_returns_none() {
    let (_d, mut conn) = migrated_db();
    assert!(
        fp::get_preference_conn(&wg(&mut conn), "nope")
            .unwrap()
            .is_none()
    );
}

#[test]
fn delete_removes_row() {
    let (_d, mut conn) = migrated_db();
    let t = DbDateTime(from_millis(1_700_000_000_000).unwrap());
    fp::upsert_preference_conn(&wg(&mut conn), "onboarding_tips_seen", "[]", t).unwrap();
    assert_eq!(
        fp::delete_preference_conn(&wg(&mut conn), "onboarding_tips_seen").unwrap(),
        1
    );
    assert!(
        fp::get_preference_conn(&wg(&mut conn), "onboarding_tips_seen")
            .unwrap()
            .is_none()
    );
    // Deleting a missing key affects 0 rows.
    assert_eq!(
        fp::delete_preference_conn(&wg(&mut conn), "onboarding_tips_seen").unwrap(),
        0
    );
}
