use carbon_repos::dbtypes::{from_millis, DbDateTime};
use carbon_repos::repos::modpack_cache as mp;
use rusqlite::Connection;
use carbon_repos::db_exec::test_support::wg;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

fn ts(ms: i64) -> DbDateTime {
    DbDateTime(from_millis(ms).unwrap())
}

// --- CurseForge modpack cache: composite get + upsert -----------------------

#[test]
fn cf_get_missing_returns_none() {
    let (_d, mut conn) = migrated_db();
    assert!(mp::get_cf_modpack_conn(&wg(&mut conn), 1, 2).unwrap().is_none());
}

#[test]
fn cf_upsert_inserts_then_get_reads_it_back_with_no_logo() {
    let (_d, mut conn) = migrated_db();
    assert_eq!(
        mp::upsert_cf_modpack_conn(&wg(&mut conn), 100, 200, "RLCraft", "2.9.3", "rlcraft", ts(1000)).unwrap(),
        1
    );

    let row = mp::get_cf_modpack_conn(&wg(&mut conn), 100, 200).unwrap().expect("row inserted");
    assert_eq!(row.project_id, 100);
    assert_eq!(row.file_id, 200);
    assert_eq!(row.modpack_name, "RLCraft");
    assert_eq!(row.version_name, "2.9.3");
    assert_eq!(row.url_slug, "rlcraft");
    assert_eq!(row.updated_at, ts(1000).0);
    assert_eq!(row.logo_data, None, "no image row yet -> LEFT JOIN nulls");
    assert!(!row.has_logo);
}

#[test]
fn cf_upsert_refreshes_updated_at_on_conflict() {
    let (_d, mut conn) = migrated_db();
    mp::upsert_cf_modpack_conn(&wg(&mut conn), 1, 1, "A", "v1", "a", ts(1_000)).unwrap();
    let first = mp::get_cf_modpack_conn(&wg(&mut conn), 1, 1).unwrap().unwrap();
    assert_eq!(first.updated_at, ts(1_000).0);

    // Re-upsert the SAME composite key with a later updated_at: the row must
    // refresh in place (still exactly one row), and updatedAt must change --
    // this is the freshness column the 7-day cache-expiry gate reads.
    mp::upsert_cf_modpack_conn(&wg(&mut conn), 1, 1, "A2", "v2", "a2", ts(2_000)).unwrap();
    let second = mp::get_cf_modpack_conn(&wg(&mut conn), 1, 1).unwrap().unwrap();
    assert_eq!(second.modpack_name, "A2");
    assert_eq!(second.version_name, "v2");
    assert_eq!(second.url_slug, "a2");
    assert_eq!(second.updated_at, ts(2_000).0, "updatedAt must be refreshed by the upsert");
    assert_ne!(second.updated_at, first.updated_at);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM CurseForgeModpackCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "conflict updates in place, does not duplicate");
}

// --- CurseForge modpack image cache ------------------------------------------

#[test]
fn cf_image_upsert_then_get_logo_and_joined_read() {
    let (_d, mut conn) = migrated_db();
    mp::upsert_cf_modpack_conn(&wg(&mut conn), 5, 9, "Pack", "v", "slug", ts(1)).unwrap();

    // No image row yet.
    assert!(mp::get_cf_modpack_logo_conn(&wg(&mut conn), 5, 9).unwrap().is_none());
    let joined = mp::get_cf_modpack_conn(&wg(&mut conn), 5, 9).unwrap().unwrap();
    assert!(!joined.has_logo);
    assert_eq!(joined.logo_data, None);

    // Insert image with data.
    assert_eq!(
        mp::upsert_cf_modpack_image_conn(&wg(&mut conn), 5, 9, "http://icon", Some(&[1, 2, 3])).unwrap(),
        1
    );
    let img = mp::get_cf_modpack_logo_conn(&wg(&mut conn), 5, 9).unwrap().unwrap();
    assert_eq!(img.data, Some(vec![1, 2, 3]));

    let joined = mp::get_cf_modpack_conn(&wg(&mut conn), 5, 9).unwrap().unwrap();
    assert!(joined.has_logo);
    assert_eq!(joined.logo_data, Some(vec![1, 2, 3]));

    // Guarded shape: an image row can exist with url set but data cleared
    // (e.g. a refresh attempt that found no url this time) -- has_logo stays
    // true (the row still exists) even though logo_data is now None.
    assert_eq!(
        mp::upsert_cf_modpack_image_conn(&wg(&mut conn), 5, 9, "", None).unwrap(),
        1
    );
    let joined = mp::get_cf_modpack_conn(&wg(&mut conn), 5, 9).unwrap().unwrap();
    assert!(joined.has_logo, "row still exists after re-upsert with no bytes");
    assert_eq!(joined.logo_data, None);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM CurseForgeModpackImageCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "conflict updates in place, does not duplicate");
}

// --- Modrinth modpack cache: composite (String) get + upsert -----------------

#[test]
fn mr_get_missing_returns_none() {
    let (_d, mut conn) = migrated_db();
    assert!(mp::get_mr_modpack_conn(&wg(&mut conn), "p", "v").unwrap().is_none());
}

#[test]
fn mr_upsert_inserts_then_get_reads_it_back() {
    let (_d, mut conn) = migrated_db();
    assert_eq!(
        mp::upsert_mr_modpack_conn(&wg(&mut conn), "1KVo5zza", "HH3vor7X", "Fabulously Optimized", "6.0", "fabulously-optimized", ts(500))
            .unwrap(),
        1
    );
    let row = mp::get_mr_modpack_conn(&wg(&mut conn), "1KVo5zza", "HH3vor7X").unwrap().expect("row inserted");
    assert_eq!(row.project_id, "1KVo5zza");
    assert_eq!(row.version_id, "HH3vor7X");
    assert_eq!(row.modpack_name, "Fabulously Optimized");
    assert_eq!(row.version_name, "6.0");
    assert_eq!(row.url_slug, "fabulously-optimized");
    assert_eq!(row.updated_at, ts(500).0);
    assert!(!row.has_logo);
    assert_eq!(row.logo_data, None);
}

#[test]
fn mr_upsert_refreshes_updated_at_on_conflict() {
    let (_d, mut conn) = migrated_db();
    mp::upsert_mr_modpack_conn(&wg(&mut conn), "p", "v", "A", "v1", "a", ts(1_000)).unwrap();
    mp::upsert_mr_modpack_conn(&wg(&mut conn), "p", "v", "A2", "v2", "a2", ts(2_000)).unwrap();
    let row = mp::get_mr_modpack_conn(&wg(&mut conn), "p", "v").unwrap().unwrap();
    assert_eq!(row.modpack_name, "A2");
    assert_eq!(row.updated_at, ts(2_000).0);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ModrinthModpackCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// --- Modrinth modpack image cache --------------------------------------------

#[test]
fn mr_image_upsert_then_get_logo_and_joined_read() {
    let (_d, mut conn) = migrated_db();
    mp::upsert_mr_modpack_conn(&wg(&mut conn), "p", "v", "Pack", "v", "slug", ts(1)).unwrap();

    assert!(mp::get_mr_modpack_logo_conn(&wg(&mut conn), "p", "v").unwrap().is_none());

    assert_eq!(
        mp::upsert_mr_modpack_image_conn(&wg(&mut conn), "p", "v", "http://icon", Some(&[9, 8])).unwrap(),
        1
    );
    let img = mp::get_mr_modpack_logo_conn(&wg(&mut conn), "p", "v").unwrap().unwrap();
    assert_eq!(img.data, Some(vec![9, 8]));

    let joined = mp::get_mr_modpack_conn(&wg(&mut conn), "p", "v").unwrap().unwrap();
    assert!(joined.has_logo);
    assert_eq!(joined.logo_data, Some(vec![9, 8]));

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ModrinthModpackImageCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// --- Two independent composite-key rows never collide ------------------------

#[test]
fn distinct_composite_keys_do_not_collide() {
    let (_d, mut conn) = migrated_db();
    mp::upsert_cf_modpack_conn(&wg(&mut conn), 1, 1, "one", "v", "s", ts(1)).unwrap();
    mp::upsert_cf_modpack_conn(&wg(&mut conn), 1, 2, "two", "v", "s", ts(1)).unwrap();
    mp::upsert_cf_modpack_conn(&wg(&mut conn), 2, 1, "three", "v", "s", ts(1)).unwrap();

    assert_eq!(mp::get_cf_modpack_conn(&wg(&mut conn), 1, 1).unwrap().unwrap().modpack_name, "one");
    assert_eq!(mp::get_cf_modpack_conn(&wg(&mut conn), 1, 2).unwrap().unwrap().modpack_name, "two");
    assert_eq!(mp::get_cf_modpack_conn(&wg(&mut conn), 2, 1).unwrap().unwrap().modpack_name, "three");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM CurseForgeModpackCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

