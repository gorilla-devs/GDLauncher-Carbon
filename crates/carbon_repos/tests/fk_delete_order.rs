//! Delete-order audit under FK enforcement (spec §7.3, plan T1 item 4).
//!
//! With FKs ON the two Restrict edges (`ModFileCache`/`ServerModFileCache` →
//! `ModMetadata`) turn wrong-order deletes into errors. These tests run the
//! real ported delete paths against a connection with `foreign_keys = ON` to
//! prove: an instance delete cascades its file-cache rows (so the RESTRICT
//! parent-delete is never reached with children present), the manual cleanup
//! that still runs is an idempotent no-op, and `gc_orphan_metadata` only
//! deletes NOT-EXISTS orphans and so never trips the RESTRICT edge.

use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::instance as inst;
use carbon_repos::repos::mod_file_cache as mfc;
use carbon_repos::repos::mod_metadata as meta;
use rusqlite::Connection;

fn now() -> DbDateTime {
    DbDateTime(chrono::Utc::now().fixed_offset())
}

/// A migrated DB with FK enforcement ON — the runtime state the sweep enables.
fn migrated_fk_on() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    conn.pragma_update(None, "foreign_keys", &"ON").unwrap();
    (dir, conn)
}

fn seed_metadata(conn: &Connection, id: &str) {
    meta::insert_metadata(
        conn,
        id,
        1,
        &[0u8],
        &[0u8],
        "",
        Some("n"),
        None,
        None,
        None,
        None,
        now(),
    )
    .unwrap();
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn delete_instance_with_mods_cascades_and_manual_cleanup_is_idempotent() {
    let (_d, mut conn) = migrated_fk_on();
    let g = inst::insert_group(&conn, "g", 0, None).unwrap() as i32;
    let iid = inst::add_instance_tx(&mut conn, "i", "sp", 0, g, None).unwrap() as i32;
    seed_metadata(&conn, "meta1");
    mfc::upsert_mod_file_cache(&conn, iid, "a.jar", 1, true, "mods", "meta1", now()).unwrap();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM ModFileCache"), 1);

    // Deleting the instance cascades its ModFileCache rows (instanceId edge is
    // ON DELETE CASCADE); the RESTRICT metadataId edge is never the delete
    // target so it does not fire.
    inst::delete_instance(&conn, iid).unwrap();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM ModFileCache"), 0, "cascade must clear cache");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM ModMetadata"), 1, "metadata survives (RESTRICT)");

    // The manual cleanup managers still run is now a no-op — and must not error
    // under FK enforcement.
    let removed = mfc::delete_mod_file_cache_by_instance(&conn, iid).unwrap();
    assert_eq!(removed, 0, "manual cleanup after cascade is an idempotent no-op");

    // Orphaned metadata is then reclaimable without tripping RESTRICT.
    let gced = meta::gc_orphan_metadata(&conn).unwrap();
    assert_eq!(gced, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM ModMetadata"), 0);
}

#[test]
fn gc_orphan_metadata_keeps_referenced_rows_under_fk_on() {
    let (_d, mut conn) = migrated_fk_on();
    let g = inst::insert_group(&conn, "g", 0, None).unwrap() as i32;
    let iid = inst::add_instance_tx(&mut conn, "i", "sp", 0, g, None).unwrap() as i32;
    seed_metadata(&conn, "referenced");
    seed_metadata(&conn, "orphan");
    mfc::upsert_mod_file_cache(&conn, iid, "a.jar", 1, true, "mods", "referenced", now()).unwrap();

    // gc deletes only the NOT-EXISTS orphan; deleting the referenced parent
    // would violate the RESTRICT edge, but the NOT-EXISTS guard excludes it, so
    // no error is raised.
    let gced = meta::gc_orphan_metadata(&conn).unwrap();
    assert_eq!(gced, 1, "only the unreferenced metadata is collected");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM ModMetadata WHERE id = 'referenced'"), 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM ModMetadata WHERE id = 'orphan'"), 0);

    // Sanity: directly deleting a still-referenced metadata DOES trip RESTRICT,
    // confirming enforcement is actually on for this connection.
    let err = conn.execute("DELETE FROM ModMetadata WHERE id = 'referenced'", []);
    assert!(err.is_err(), "RESTRICT edge must reject deleting a referenced parent");
}
