use carbon_repos::dbtypes::{from_millis, DbDateTime};
use carbon_repos::repos::version_meta as vm;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

#[test]
fn version_info_upsert_inserts_then_updates_payload_and_freshness() {
    let (_d, mut conn) = migrated_db();
    let t1 = DbDateTime(from_millis(1_700_000_000_000).unwrap());
    assert_eq!(
        vm::upsert_version_info_conn(&wg(&mut conn), "1.20.1", b"payload-v1", t1).unwrap(),
        1
    );
    let row = vm::get_version_info_conn(&wg(&mut conn), "1.20.1").unwrap().unwrap();
    assert_eq!(row.id, "1.20.1");
    assert_eq!(row.version_info, b"payload-v1");
    assert_eq!(row.last_updated_at, t1.0);

    // Conflict on id updates both the payload AND lastUpdatedAt.
    let t2 = DbDateTime(from_millis(1_784_557_692_104).unwrap());
    vm::upsert_version_info_conn(&wg(&mut conn), "1.20.1", b"payload-v2", t2).unwrap();
    let row = vm::get_version_info_conn(&wg(&mut conn), "1.20.1").unwrap().unwrap();
    assert_eq!(row.version_info, b"payload-v2");
    assert_eq!(row.last_updated_at, t2.0);
}

#[test]
fn version_info_get_missing_returns_none() {
    let (_d, mut conn) = migrated_db();
    assert!(vm::get_version_info_conn(&wg(&mut conn), "nope").unwrap().is_none());
}

#[test]
fn partial_version_info_upsert_inserts_then_updates_payload_and_freshness() {
    let (_d, mut conn) = migrated_db();
    let t1 = DbDateTime(from_millis(1_700_000_000_000).unwrap());
    assert_eq!(
        vm::upsert_partial_version_info_conn(&wg(&mut conn), "forge-47.2.0", b"partial-v1", t1).unwrap(),
        1
    );
    let row = vm::get_partial_version_info_conn(&wg(&mut conn), "forge-47.2.0")
        .unwrap()
        .unwrap();
    assert_eq!(row.partial_version_info, b"partial-v1");
    assert_eq!(row.last_updated_at, t1.0);

    let t2 = DbDateTime(from_millis(1_784_557_692_104).unwrap());
    vm::upsert_partial_version_info_conn(&wg(&mut conn), "forge-47.2.0", b"partial-v2", t2).unwrap();
    let row = vm::get_partial_version_info_conn(&wg(&mut conn), "forge-47.2.0")
        .unwrap()
        .unwrap();
    assert_eq!(row.partial_version_info, b"partial-v2");
    assert_eq!(row.last_updated_at, t2.0);
}

#[test]
fn lwjgl_meta_upsert_inserts_then_updates_payload_and_freshness() {
    let (_d, mut conn) = migrated_db();
    let t1 = DbDateTime(from_millis(1_700_000_000_000).unwrap());
    assert_eq!(
        vm::upsert_lwjgl_meta_conn(&wg(&mut conn), "1.20-lwjgl3", b"lwjgl-v1", t1).unwrap(),
        1
    );
    let row = vm::get_lwjgl_meta_conn(&wg(&mut conn), "1.20-lwjgl3").unwrap().unwrap();
    assert_eq!(row.lwjgl, b"lwjgl-v1");
    assert_eq!(row.last_updated_at, t1.0);

    let t2 = DbDateTime(from_millis(1_784_557_692_104).unwrap());
    vm::upsert_lwjgl_meta_conn(&wg(&mut conn), "1.20-lwjgl3", b"lwjgl-v2", t2).unwrap();
    let row = vm::get_lwjgl_meta_conn(&wg(&mut conn), "1.20-lwjgl3").unwrap().unwrap();
    assert_eq!(row.lwjgl, b"lwjgl-v2");
    assert_eq!(row.last_updated_at, t2.0);
}

#[test]
fn assets_meta_upsert_inserts_then_updates_payload_and_freshness() {
    let (_d, mut conn) = migrated_db();
    let t1 = DbDateTime(from_millis(1_700_000_000_000).unwrap());
    assert_eq!(
        vm::upsert_assets_meta_conn(&wg(&mut conn), "17", b"assets-v1", t1).unwrap(),
        1
    );
    let row = vm::get_assets_meta_conn(&wg(&mut conn), "17").unwrap().unwrap();
    assert_eq!(row.assets_index, b"assets-v1");
    assert_eq!(row.last_updated_at, t1.0);

    let t2 = DbDateTime(from_millis(1_784_557_692_104).unwrap());
    vm::upsert_assets_meta_conn(&wg(&mut conn), "17", b"assets-v2", t2).unwrap();
    let row = vm::get_assets_meta_conn(&wg(&mut conn), "17").unwrap().unwrap();
    assert_eq!(row.assets_index, b"assets-v2");
    assert_eq!(row.last_updated_at, t2.0);
}

#[test]
fn assets_meta_get_missing_returns_none() {
    let (_d, mut conn) = migrated_db();
    assert!(vm::get_assets_meta_conn(&wg(&mut conn), "nope").unwrap().is_none());
}

fn wg(c: &mut Connection) -> carbon_repos::db_exec::WriteGuard<'_> {
    carbon_repos::db_exec::WriteGuard::new(c)
}
