//! Delete-order audit under FK enforcement (spec §7.3, plan T1 item 4).
//!
//! With FKs ON the two Restrict edges (`ModFileCache`/`ServerModFileCache` →
//! `ModMetadata`) turn wrong-order deletes into errors. These tests run the
//! real ported delete paths against a `Db` opened with `foreign_keys = ON` to
//! prove: an instance delete cascades its file-cache rows (so the RESTRICT
//! parent-delete is never reached with children present), the manual cleanup
//! that still runs is an idempotent no-op, and `gc_orphan_metadata` only
//! deletes NOT-EXISTS orphans and so never trips the RESTRICT edge.

use carbon_repos::db_exec::{Db, ReadAccess, WriteAccess};
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::instance as inst;
use carbon_repos::repos::mod_file_cache as mfc;
use carbon_repos::repos::mod_metadata as meta;
use rusqlite::Connection;

fn now() -> DbDateTime {
    DbDateTime(chrono::Utc::now().fixed_offset())
}

/// A migrated `Db` with FK enforcement ON — the runtime state the sweep enables.
async fn migrated_fk_on() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
    }
    let db = Db::open(&path, 2, true).unwrap();
    (dir, db)
}

async fn seed_metadata(db: &Db, id: &str) {
    meta::insert_metadata(
        db,
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
    .await
    .unwrap();
}

async fn count(db: &Db, sql: &'static str) -> i64 {
    db.read(move |conn| Ok(conn.query_row(sql, [], |r| r.get(0))?))
        .await
        .unwrap()
}

#[tokio::test]
async fn delete_instance_with_mods_cascades_and_manual_cleanup_is_idempotent() {
    let (_d, db) = migrated_fk_on().await;
    let g = inst::insert_group(&db, "g".into(), 0, None).await.unwrap() as i32;
    let iid = inst::add_instance_tx(&db, "i".into(), "sp".into(), 0, g, None)
        .await
        .unwrap() as i32;
    seed_metadata(&db, "meta1").await;
    mfc::upsert_mod_file_cache(&db, iid, "a.jar".into(), 1, true, "mods".into(), "meta1".into(), now())
        .await
        .unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModFileCache").await, 1);

    // Deleting the instance cascades its ModFileCache rows (instanceId edge is
    // ON DELETE CASCADE); the RESTRICT metadataId edge is never the delete
    // target so it does not fire.
    inst::delete_instance(&db, iid).await.unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModFileCache").await, 0, "cascade must clear cache");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModMetadata").await, 1, "metadata survives (RESTRICT)");

    // The manual cleanup managers still run is now a no-op — and must not error
    // under FK enforcement.
    let removed = mfc::delete_mod_file_cache_by_instance(&db, iid).await.unwrap();
    assert_eq!(removed, 0, "manual cleanup after cascade is an idempotent no-op");

    // Orphaned metadata is then reclaimable without tripping RESTRICT.
    let gced = meta::gc_orphan_metadata(&db).await.unwrap();
    assert_eq!(gced, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModMetadata").await, 0);
}

#[tokio::test]
async fn gc_orphan_metadata_keeps_referenced_rows_under_fk_on() {
    let (_d, db) = migrated_fk_on().await;
    let g = inst::insert_group(&db, "g".into(), 0, None).await.unwrap() as i32;
    let iid = inst::add_instance_tx(&db, "i".into(), "sp".into(), 0, g, None)
        .await
        .unwrap() as i32;
    seed_metadata(&db, "referenced").await;
    seed_metadata(&db, "orphan").await;
    mfc::upsert_mod_file_cache(
        &db,
        iid,
        "a.jar".into(),
        1,
        true,
        "mods".into(),
        "referenced".into(),
        now(),
    )
    .await
    .unwrap();

    // gc deletes only the NOT-EXISTS orphan; deleting the referenced parent
    // would violate the RESTRICT edge, but the NOT-EXISTS guard excludes it, so
    // no error is raised.
    let gced = meta::gc_orphan_metadata(&db).await.unwrap();
    assert_eq!(gced, 1, "only the unreferenced metadata is collected");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModMetadata WHERE id = 'referenced'").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModMetadata WHERE id = 'orphan'").await, 0);

    // Sanity: directly deleting a still-referenced metadata DOES trip RESTRICT,
    // confirming enforcement is actually on for this connection.
    let err = db
        .write(|conn| Ok(conn.execute("DELETE FROM ModMetadata WHERE id = 'referenced'", [])?))
        .await;
    assert!(err.is_err(), "RESTRICT edge must reject deleting a referenced parent");
}
