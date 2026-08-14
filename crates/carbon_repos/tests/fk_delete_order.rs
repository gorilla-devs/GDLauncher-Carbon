//! Delete-order audit under FK enforcement.
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
use carbon_repos::repos::server as srv;
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

/// A migrated `Db` with FK enforcement OFF — the state `sweep_and_decide`
/// falls back to for the whole session when it finds a violation it cannot
/// repair, and what `GDL_DISABLE_FK_ENFORCEMENT=1` selects. No referential
/// action fires here, so every cleanup a delete depends on has to be issued by
/// the code itself.
async fn migrated_fk_off() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
    }
    let db = Db::open(&path, 2, false).unwrap();
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
    mfc::upsert_mod_file_cache(
        &db,
        iid,
        "a.jar".into(),
        1,
        true,
        "mods".into(),
        "meta1".into(),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModFileCache").await, 1);

    // Deleting the instance cascades its ModFileCache rows (instanceId edge is
    // ON DELETE CASCADE); the RESTRICT metadataId edge is never the delete
    // target so it does not fire.
    inst::delete_instance(&db, iid).await.unwrap();
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ModFileCache").await,
        0,
        "cascade must clear cache"
    );
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ModMetadata").await,
        1,
        "metadata survives (RESTRICT)"
    );

    // The manual cleanup managers still run is now a no-op — and must not error
    // under FK enforcement.
    let removed = mfc::delete_mod_file_cache_by_instance(&db, iid)
        .await
        .unwrap();
    assert_eq!(
        removed, 0,
        "manual cleanup after cascade is an idempotent no-op"
    );

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
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM ModMetadata WHERE id = 'referenced'"
        )
        .await,
        1
    );
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ModMetadata WHERE id = 'orphan'").await,
        0
    );

    // Sanity: directly deleting a still-referenced metadata DOES trip RESTRICT,
    // confirming enforcement is actually on for this connection.
    let err = db
        .write(|conn| Ok(conn.execute("DELETE FROM ModMetadata WHERE id = 'referenced'", [])?))
        .await;
    assert!(
        err.is_err(),
        "RESTRICT edge must reject deleting a referenced parent"
    );
}

#[tokio::test]
async fn deleting_an_instance_clears_its_file_cache_without_fk_enforcement() {
    // `remove_instance` (carbon_app) calls `delete_instance_tx` rather than
    // relying on the cascade alone: under FK enforcement the cascade already
    // clears these rows (see `delete_instance_with_mods_cascades_and_manual_cleanup_is_idempotent`
    // above), but `sweep_and_decide` falls back to leaving foreign keys off
    // for the whole session when it meets a violation it cannot repair, and
    // `GDL_DISABLE_FK_ENFORCEMENT=1` selects the same state — on those
    // sessions nothing else would clear `ModFileCache`, which in turn keeps
    // its referenced `ModMetadata` row alive forever (gc_orphan_metadata only
    // reclaims metadata nothing still references).
    let (_d, db) = migrated_fk_off().await;
    let g = inst::insert_group(&db, "g".into(), 0, None).await.unwrap() as i32;
    let iid = inst::add_instance_tx(&db, "i".into(), "sp".into(), 0, g, None)
        .await
        .unwrap() as i32;
    seed_metadata(&db, "meta1").await;
    mfc::upsert_mod_file_cache(
        &db,
        iid,
        "a.jar".into(),
        1,
        true,
        "mods".into(),
        "meta1".into(),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModFileCache").await, 1);

    inst::delete_instance_tx(&db, iid).await.unwrap();

    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ModFileCache").await,
        0,
        "the instance's cache rows must be cleared without relying on the cascade"
    );

    // Orphaned metadata is then reclaimable, exactly as it is on the FK-ON path.
    let gced = meta::gc_orphan_metadata(&db).await.unwrap();
    assert_eq!(gced, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ModMetadata").await, 0);
}

#[tokio::test]
async fn deleting_a_server_clears_its_file_cache_without_fk_enforcement() {
    // The instance path issues its own cleanup alongside the cascade; the server
    // path relied on the cascade alone, so on an FK-OFF session the rows were
    // left behind. Server ids are AUTOINCREMENT, so these can never be adopted
    // by a later server — they simply accumulate.
    let (_d, db) = migrated_fk_off().await;
    let g = srv::insert_server_group(&db, "g".into(), 0, None)
        .await
        .unwrap() as i32;
    let sid = srv::insert_server(
        &db,
        "s".into(),
        "sp".into(),
        0,
        g,
        "1.20.1".into(),
        25565,
        "vanilla".into(),
        None,
        None,
        None,
        None,
        None,
        None,
        now(),
    )
    .await
    .unwrap() as i32;
    seed_metadata(&db, "meta1").await;
    mfc::upsert_server_mod_file_cache(
        &db,
        sid,
        "a.jar".into(),
        1,
        true,
        "mods".into(),
        "meta1".into(),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ServerModFileCache").await,
        1
    );

    srv::delete_server_tx(&db, sid).await.unwrap();

    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ServerModFileCache").await,
        0,
        "the server's cache rows must be cleared without relying on the cascade"
    );
}

/// An in-flight CurseForge metadata cache write, landing after the instance
/// that owned the mod was deleted, fails on the
/// `CurseForgeModCache.metadataId -> ModMetadata.id` foreign key.
///
/// This is the DB-level half of a real defect seen in a full e2e run (48
/// violations logged from `metadata/cache/mod.rs`'s `save_batch`). The cache
/// pass captures `metadata_id` values up front and then does a CurseForge
/// network round trip before saving; if the instance is deleted inside that
/// window, `remove_instance` -> `delete_instance_tx` -> `gc_orphan_metadata`
/// reclaims the now-unreferenced `ModMetadata` row, and the save lands on a
/// parent that no longer exists.
///
/// Deterministic on purpose: the production race is a timing window, but the
/// *consequence* of it is exactly this ordering, so it can be asserted without
/// racing anything. The app-side handler must treat this as a benign "the mod
/// is gone, nothing to cache" outcome — what it must NOT do is what it does
/// today, which is add the mod's murmur2 to `ignored_remote_cf_hashes` and
/// refuse to cache that fingerprint for the rest of the session.
#[tokio::test]
async fn cf_cache_write_for_gced_metadata_violates_the_fk() {
    let (_d, db) = migrated_fk_on().await;

    let g = inst::insert_group(&db, "g".into(), 0, None).await.unwrap() as i32;
    let iid = inst::add_instance_tx(&db, "i".into(), "sp".into(), 0, g, None)
        .await
        .unwrap() as i32;
    seed_metadata(&db, "meta1").await;
    mfc::upsert_mod_file_cache(
        &db,
        iid,
        "mod.jar".into(),
        1,
        true,
        "mod".into(),
        "meta1".into(),
        now(),
    )
    .await
    .unwrap();

    // What the cache pass is holding while it waits on CurseForge.
    let in_flight_metadata_id = "meta1";

    // The window closes: the instance goes away and its metadata is reclaimed.
    inst::delete_instance_tx(&db, iid).await.unwrap();
    meta::gc_orphan_metadata(&db).await.unwrap();
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM ModMetadata WHERE id = 'meta1'").await,
        0,
        "precondition: the orphaned metadata row must actually be reclaimed"
    );

    // The save lands.
    let result = meta::upsert_cf_mod_cache(
        &db,
        1,
        100,
        200,
        "name".into(),
        "1.0.0".into(),
        "slug".into(),
        "summary".into(),
        "authors".into(),
        1,
        "".into(),
        now(),
        in_flight_metadata_id.to_string(),
    )
    .await;

    let err = result.expect_err(
        "caching a mod whose ModMetadata was GC'd must not silently succeed — \
         if this starts passing, the FK edge changed and the app-side handling \
         below needs revisiting",
    );
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("foreign key"),
        "expected a FOREIGN KEY violation, got: {msg}"
    );

    // The classifier the app-side handler branches on must recognise this
    // exact error. If it stops doing so, the CurseForge cacher silently goes
    // back to blacklisting the mod's fingerprint for the whole session.
    assert!(
        err.is_foreign_key_violation(),
        "DbError::is_foreign_key_violation must classify the real error the \
         cache write produces, got: {msg}"
    );
}

/// The FK classifier must not swallow unrelated write failures — those are
/// the ones the CurseForge cacher *should* still treat as a bad mod.
#[test]
fn non_fk_db_errors_are_not_classified_as_foreign_key_violations() {
    use carbon_repos::db_error::DbError;

    assert!(!DbError::Closed.is_foreign_key_violation());
    assert!(!DbError::Conversion("bad row".into()).is_foreign_key_violation());
    assert!(!DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows).is_foreign_key_violation());
}
