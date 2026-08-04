use carbon_repos::db_exec::test_support::wg;
use carbon_repos::dbtypes::{DbDateTime, from_millis};
use carbon_repos::repos::mod_metadata as mm;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    // Mirror production (db_exec opens connections without enabling FK — Plan 3),
    // so cache rows can be seeded without full parent chains.
    conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    (dir, conn)
}

fn ts(ms: i64) -> DbDateTime {
    DbDateTime(from_millis(ms).unwrap())
}

/// A distinct 64-byte sha512 blob (the real hash width mod lookups key on).
fn sha512(seed: u8) -> Vec<u8> {
    vec![seed; 64]
}

fn insert_mfc(conn: &Connection, instance_id: i32, filename: &str, metadata_id: &str) {
    conn.execute(
        "INSERT INTO ModFileCache (id, lastUpdatedAt, instanceId, filename, filesize, enabled, addonType, metadataId)
         VALUES (?, 0, ?, ?, 1, 1, 'mods', ?)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), instance_id, filename, metadata_id],
    )
    .unwrap();
}

// --- ModMetadata: hash lookup + insert --------------------------------------

#[test]
fn insert_and_find_by_64_byte_hashes() {
    let (_d, mut conn) = migrated_db();
    let s512 = sha512(0xAB);
    let s1 = vec![0x11; 20];

    assert_eq!(
        mm::insert_metadata_conn(
            &wg(&mut conn),
            "meta-1",
            42,
            &s512,
            &s1,
            "forge,fabric",
            Some("jei"),
            Some("jei-modid"),
            Some("1.0"),
            Some("desc"),
            Some("auth"),
            ts(7000),
        )
        .unwrap(),
        1
    );

    let row = mm::find_metadata_by_hashes_conn(&wg(&mut conn), &s512, 42)
        .unwrap()
        .expect("row found by (sha512, murmur2)");
    assert_eq!(row.id, "meta-1");
    assert_eq!(row.sha512, s512);
    assert_eq!(row.sha512.len(), 64);
    assert_eq!(row.murmur2, 42);
    assert_eq!(row.name.as_deref(), Some("jei"));
    assert_eq!(row.modloaders, "forge,fabric");
    assert_eq!(row.last_updated_at, ts(7000).0, "freshness column written");

    // Same sha512 but wrong murmur2 -> no match (AND semantics).
    assert!(
        mm::find_metadata_by_hashes_conn(&wg(&mut conn), &s512, 99)
            .unwrap()
            .is_none()
    );
    // Different sha512 -> no match.
    assert!(
        mm::find_metadata_by_hashes_conn(&wg(&mut conn), &sha512(0x01), 42)
            .unwrap()
            .is_none()
    );
}

#[test]
fn insert_metadata_accepts_all_none_optionals() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "m",
        1,
        &sha512(1),
        &[1, 2],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    let row = mm::find_metadata_by_hashes_conn(&wg(&mut conn), &sha512(1), 1)
        .unwrap()
        .unwrap();
    assert_eq!(row.name, None);
    assert_eq!(row.modid, None);
    assert_eq!(row.authors, None);
    assert_eq!(row.modloaders, "");
}

// --- LocalModImageCache -----------------------------------------------------

#[test]
fn insert_local_image_stores_blob() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "m",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    assert_eq!(
        mm::insert_local_image_conn(&wg(&mut conn), "m", &[9, 8, 7]).unwrap(),
        1
    );
    let data: Vec<u8> = conn
        .query_row(
            "SELECT data FROM LocalModImageCache WHERE metadataId = 'm'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(data, vec![9, 8, 7]);
}

// --- GC: only true orphans --------------------------------------------------

#[test]
fn gc_removes_only_true_orphans() {
    let (_d, mut conn) = migrated_db();
    // orphan: no file cache references it
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "orphan",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    // referenced by an instance file cache
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "inst",
        2,
        &sha512(2),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    insert_mfc(&conn, 1, "a.jar", "inst");
    // referenced by a server file cache
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "srv",
        3,
        &sha512(3),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ServerModFileCache (id, lastUpdatedAt, serverId, filename, filesize, enabled, addonType, metadataId)
         VALUES ('sid', 0, 5, 's.jar', 1, 1, 'mods', 'srv')",
        [],
    )
    .unwrap();

    assert_eq!(
        mm::gc_orphan_metadata_conn(&wg(&mut conn)).unwrap(),
        1,
        "only the orphan is deleted"
    );
    assert!(
        mm::find_metadata_by_hashes_conn(&wg(&mut conn), &sha512(1), 1)
            .unwrap()
            .is_none()
    );
    assert!(
        mm::find_metadata_by_hashes_conn(&wg(&mut conn), &sha512(2), 2)
            .unwrap()
            .is_some()
    );
    assert!(
        mm::find_metadata_by_hashes_conn(&wg(&mut conn), &sha512(3), 3)
            .unwrap()
            .is_some()
    );
}

// --- CurseForge cache upsert (composite conflict, mirror-of-PCR) ------------

#[allow(clippy::too_many_arguments)]
fn cf_upsert(
    conn: &impl carbon_repos::db_exec::WriteAccess,
    metadata_id: &str,
    project_id: i32,
    file_id: i32,
    name: &str,
    cached_at: i64,
) -> String {
    mm::upsert_cf_mod_cache_conn(
        conn,
        0,
        project_id,
        file_id,
        name,
        "v",
        "slug",
        "sum",
        "auth",
        2,
        "paths",
        ts(cached_at),
        metadata_id,
    )
    .unwrap()
}

#[test]
fn cf_upsert_inserts_then_composite_conflict_keeps_original_metadata_id() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "meta-A",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "meta-B",
        2,
        &sha512(2),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();

    // First insert under meta-A.
    let returned = cf_upsert(&wg(&mut conn), "meta-A", 500, 900, "first", 100);
    assert_eq!(returned, "meta-A");

    // Second upsert with the SAME (projectId, fileId) but a DIFFERENT metadataId.
    // PCR's DO UPDATE list never set metadataId, so the surviving row keeps
    // meta-A. The returned metadataId must be meta-A (NOT the passed meta-B).
    let returned = cf_upsert(&wg(&mut conn), "meta-B", 500, 900, "second", 200);
    assert_eq!(
        returned, "meta-A",
        "composite conflict preserves the original metadataId"
    );

    // Exactly one CF row, keyed by meta-A, with the refreshed fields.
    let row = mm::get_cf_cache_by_metadata_conn(&wg(&mut conn), "meta-A")
        .unwrap()
        .unwrap();
    assert_eq!(row.name, "second");
    assert_eq!(row.cached_at, ts(200).0);
    assert!(
        mm::get_cf_cache_by_metadata_conn(&wg(&mut conn), "meta-B")
            .unwrap()
            .is_none()
    );
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM CurseForgeModCache", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn cf_get_by_metadata_reads_cached_at() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "m",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    cf_upsert(&wg(&mut conn), "m", 1, 2, "n", 12345);
    let row = mm::get_cf_cache_by_metadata_conn(&wg(&mut conn), "m")
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id, 1);
    assert_eq!(row.file_id, 2);
    assert_eq!(row.cached_at, ts(12345).0);
    assert!(
        mm::get_cf_cache_by_metadata_conn(&wg(&mut conn), "nope")
            .unwrap()
            .is_none()
    );
}

// --- Modrinth cache upsert --------------------------------------------------

#[test]
fn mr_upsert_composite_conflict_keeps_original_metadata_id() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "mA",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "mB",
        2,
        &sha512(2),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();

    let r = mm::upsert_mr_mod_cache_conn(
        &wg(&mut conn),
        "sha",
        "proj",
        "ver",
        "t1",
        "v",
        "slug",
        "d",
        "a",
        2,
        "",
        "f.jar",
        "http://u",
        ts(1),
        "mA",
    )
    .unwrap();
    assert_eq!(r, "mA");

    let r = mm::upsert_mr_mod_cache_conn(
        &wg(&mut conn),
        "sha",
        "proj",
        "ver",
        "t2",
        "v",
        "slug",
        "d",
        "a",
        2,
        "",
        "f.jar",
        "http://u",
        ts(9),
        "mB",
    )
    .unwrap();
    assert_eq!(
        r, "mA",
        "composite (projectId, versionId) conflict preserves metadataId"
    );

    let row = mm::get_mr_cache_by_metadata_conn(&wg(&mut conn), "mA")
        .unwrap()
        .unwrap();
    assert_eq!(row.title, "t2");
    assert_eq!(row.cached_at, ts(9).0);
}

// --- Image upsert: stale-marking + downloaded update ------------------------

#[test]
fn cf_image_upsert_marks_stale_and_download_clears_it() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "m",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();

    // First upsert: inserts stale (upToDate = 0, data NULL).
    assert_eq!(
        mm::upsert_cf_image_conn(&wg(&mut conn), "m", "url-1").unwrap(),
        1
    );
    let (url, data, up): (String, Option<Vec<u8>>, i32) = conn
        .query_row(
            "SELECT url, data, upToDate FROM CurseForgeModImageCache WHERE metadataId = 'm'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(url, "url-1");
    assert_eq!(data, None);
    assert_eq!(up, 0);

    // Download stores the blob and marks it up to date.
    assert_eq!(
        mm::mark_cf_image_downloaded_conn(&wg(&mut conn), "m", &[1, 2, 3]).unwrap(),
        1
    );
    let (data, up): (Option<Vec<u8>>, i32) = conn
        .query_row(
            "SELECT data, upToDate FROM CurseForgeModImageCache WHERE metadataId = 'm'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(data, Some(vec![1, 2, 3]));
    assert_eq!(up, 1);

    // Re-upsert (url change) re-marks stale but preserves the existing blob.
    assert_eq!(
        mm::upsert_cf_image_conn(&wg(&mut conn), "m", "url-2").unwrap(),
        1
    );
    let (url, data, up): (String, Option<Vec<u8>>, i32) = conn
        .query_row(
            "SELECT url, data, upToDate FROM CurseForgeModImageCache WHERE metadataId = 'm'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(url, "url-2");
    assert_eq!(
        data,
        Some(vec![1, 2, 3]),
        "existing blob survives the stale re-mark"
    );
    assert_eq!(up, 0);
}

#[test]
fn mr_image_upsert_and_download() {
    let (_d, mut conn) = migrated_db();
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "m",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    mm::upsert_mr_image_conn(&wg(&mut conn), "m", "u").unwrap();
    mm::mark_mr_image_downloaded_conn(&wg(&mut conn), "m", &[7]).unwrap();
    let (data, up): (Option<Vec<u8>>, i32) = conn
        .query_row(
            "SELECT data, upToDate FROM ModrinthModImageCache WHERE metadataId = 'm'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(data, Some(vec![7]));
    assert_eq!(up, 1);
}

// --- Export enrich queries --------------------------------------------------

#[test]
fn cf_export_enrich_includes_modrinth_cross_reference() {
    let (_d, mut conn) = migrated_db();
    // metadata with BOTH a CF and MR cache row -> cross-reference present.
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "both",
        1,
        &sha512(1),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    cf_upsert(&wg(&mut conn), "both", 111, 222, "cf-name", 1);
    mm::upsert_mr_mod_cache_conn(
        &wg(&mut conn),
        "sha",
        "mrproj",
        "mrver",
        "t",
        "v",
        "mrslug",
        "d",
        "a",
        2,
        "",
        "f",
        "u",
        ts(1),
        "both",
    )
    .unwrap();
    // metadata with only a CF cache row -> mr_* columns NULL.
    mm::insert_metadata_conn(
        &wg(&mut conn),
        "cfonly",
        2,
        &sha512(2),
        &[1],
        "",
        None,
        None,
        None,
        None,
        None,
        ts(1),
    )
    .unwrap();
    cf_upsert(&wg(&mut conn), "cfonly", 333, 444, "cf2", 1);

    let both = mm::get_cf_export_enrich_by_project_conn(&wg(&mut conn), 111).unwrap();
    assert_eq!(both.len(), 1);
    assert_eq!(both[0].name, "cf-name");
    assert_eq!(both[0].urlslug, "slug");
    assert_eq!(both[0].mr_project_id.as_deref(), Some("mrproj"));
    assert_eq!(both[0].mr_version_id.as_deref(), Some("mrver"));
    assert_eq!(both[0].mr_urlslug.as_deref(), Some("mrslug"));

    let cfonly = mm::get_cf_export_enrich_by_project_conn(&wg(&mut conn), 333).unwrap();
    assert_eq!(cfonly.len(), 1);
    assert_eq!(cfonly[0].mr_project_id, None);

    let mr = mm::get_mr_export_enrich_by_project_conn(&wg(&mut conn), "mrproj").unwrap();
    assert_eq!(mr.len(), 1);
    assert_eq!(mr[0].title, "t");
    assert_eq!(mr[0].urlslug, "mrslug");
}
