use carbon_repos::db_exec::test_support::wg;
use carbon_repos::dbtypes::{DbDateTime, from_millis};
use carbon_repos::repos::mod_metadata as mm;
use carbon_repos::repos::world_datapack_cache as wdc;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    // Cascades are the behaviour under test, so keep foreign keys enforced.
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(1,'g',0);
         INSERT INTO Instance(id,name,shortpath,\"index\",groupId) VALUES(7,'i','p',0,1);",
    )
    .unwrap();
    (dir, conn)
}

fn ts(ms: i64) -> DbDateTime {
    DbDateTime(from_millis(ms).unwrap())
}

fn insert_metadata(conn: &Connection, id: &str) {
    conn.execute(
        "INSERT INTO ModMetadata (id, murmur2, sha512, sha1, name, modloaders)
         VALUES (?, 1, ?, ?, 'Pack', 'datapack')",
        rusqlite::params![id, id.as_bytes(), id.as_bytes()],
    )
    .unwrap();
}

fn count(conn: &Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM WorldDatapackCache", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn upsert_twice_on_one_place_keeps_one_row_and_its_id() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m1");
    insert_metadata(&conn, "m2");

    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "a.zip",
        10,
        false,
        Some("m1"),
        Some("s1"),
        ts(1),
    )
    .unwrap();
    let first = wdc::get_world_datapacks_by_instance_conn(&wg(&mut conn), 7).unwrap();
    assert_eq!(first.len(), 1);

    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "a.zip",
        20,
        false,
        Some("m2"),
        None,
        ts(2),
    )
    .unwrap();
    let second = wdc::get_world_datapacks_by_instance_conn(&wg(&mut conn), 7).unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].id, first[0].id);
    assert_eq!(second[0].filesize, 20);
    assert_eq!(second[0].metadata_id.as_deref(), Some("m2"));
    assert_eq!(second[0].file_stamp, None);

    let by_id = wdc::get_world_datapack_by_id_conn(&wg(&mut conn), &first[0].id)
        .unwrap()
        .unwrap();
    assert_eq!(by_id, second[0]);

    // Same file name in another world is another place.
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "Other",
        "a.zip",
        20,
        false,
        Some("m2"),
        None,
        ts(3),
    )
    .unwrap();
    assert_eq!(count(&conn), 2);
}

#[test]
fn deleting_the_instance_removes_its_rows() {
    let (_d, mut conn) = migrated_db();
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "folder",
        0,
        true,
        None,
        None,
        ts(1),
    )
    .unwrap();
    assert_eq!(count(&conn), 1);
    conn.execute("DELETE FROM Instance WHERE id = 7", [])
        .unwrap();
    assert_eq!(count(&conn), 0);
}

#[test]
fn deleting_metadata_removes_rows_pointing_at_it() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m1");
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "a.zip",
        10,
        false,
        Some("m1"),
        None,
        ts(1),
    )
    .unwrap();
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "folder",
        0,
        true,
        None,
        None,
        ts(1),
    )
    .unwrap();
    conn.execute("DELETE FROM ModMetadata WHERE id = 'm1'", [])
        .unwrap();
    let rows = wdc::get_world_datapacks_by_instance_conn(&wg(&mut conn), 7).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].filename, "folder");
}

#[test]
fn gc_keeps_metadata_referenced_only_by_a_world_row() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "kept");
    insert_metadata(&conn, "orphan");
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "a.zip",
        10,
        false,
        Some("kept"),
        None,
        ts(1),
    )
    .unwrap();
    mm::gc_orphan_metadata_conn(&wg(&mut conn)).unwrap();
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM ModMetadata ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(ids, ["kept"]);
    assert_eq!(count(&conn), 1);
}

#[test]
fn delete_by_id_and_by_world() {
    let (_d, mut conn) = migrated_db();
    for (w, f) in [("A", "1.zip"), ("A", "2.zip"), ("B", "1.zip")] {
        wdc::upsert_world_datapack_conn(&wg(&mut conn), 7, w, f, 1, false, None, None, ts(1))
            .unwrap();
    }
    let rows = wdc::get_world_datapacks_by_instance_conn(&wg(&mut conn), 7).unwrap();
    let one = rows
        .iter()
        .find(|r| r.world == "A" && r.filename == "1.zip")
        .unwrap();
    assert_eq!(
        wdc::delete_world_datapack_by_id_conn(&wg(&mut conn), &one.id).unwrap(),
        1
    );
    assert_eq!(
        wdc::delete_world_datapacks_of_world_conn(&wg(&mut conn), 7, "A").unwrap(),
        1
    );
    assert_eq!(count(&conn), 1);
}

#[test]
fn full_rows_keep_unpacked_folders_and_carry_platform_data() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m1");
    conn.execute(
        "INSERT INTO CurseForgeModCache
           (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
         VALUES ('m1', 1, 5, 6, 'CF', 'v', 'slug', 's', 'a', 1, '', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO CurseForgeModImageCache VALUES ('m1', 'u', x'01', 1)",
        [],
    )
    .unwrap();
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "a.zip",
        10,
        false,
        Some("m1"),
        None,
        ts(1),
    )
    .unwrap();
    wdc::upsert_world_datapack_conn(
        &wg(&mut conn),
        7,
        "World",
        "folder",
        0,
        true,
        None,
        None,
        ts(1),
    )
    .unwrap();

    let mut rows = wdc::get_world_datapacks_full_conn(&wg(&mut conn), 7).unwrap();
    rows.sort_by(|a, b| a.filename.cmp(&b.filename));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].meta_name.as_deref(), Some("Pack"));
    assert_eq!(rows[0].cf_project_id, Some(5));
    assert!(rows[0].has_cf_image);
    assert!(!rows[0].has_mr_image);
    assert!(rows[1].is_folder);
    assert_eq!(rows[1].meta_name, None);

    let id = rows[0].id.clone();
    let ids = wdc::get_world_datapack_update_ids_conn(&wg(&mut conn), &id)
        .unwrap()
        .unwrap();
    assert_eq!(ids.cf_file_id, Some(6));
    assert_eq!(ids.modloaders, "datapack");
    let icon = wdc::get_world_datapack_icon_data_conn(&wg(&mut conn), &id)
        .unwrap()
        .unwrap();
    assert_eq!(icon.cf_data, Some(vec![1]));
}

use carbon_repos::db_exec::test_support::rg;
use carbon_repos::repos::mod_file_cache as mfc;

/// The zip `a.zip` of metadata `m1` in two worlds, and a folder pack that
/// has no metadata.
fn world_pack(conn: &mut Connection) {
    insert_metadata(conn, "m1");
    for (world, filename, is_folder, metadata) in [
        ("World", "a.zip", false, Some("m1")),
        ("Other", "a.zip", false, Some("m1")),
        ("World", "folder", true, None),
    ] {
        wdc::upsert_world_datapack_conn(
            &wg(conn),
            7,
            world,
            filename,
            10,
            is_folder,
            metadata,
            None,
            ts(1),
        )
        .unwrap();
    }
}

fn cached_on_both_platforms(conn: &mut Connection, cached_at: i64) {
    mm::upsert_cf_mod_cache_conn(
        &wg(conn),
        1,
        5,
        6,
        "CF",
        "v",
        "slug",
        "s",
        "a",
        1,
        "",
        ts(cached_at),
        "m1",
    )
    .unwrap();
    mm::upsert_mr_mod_cache_conn(
        &wg(conn),
        &hex::encode("m1"),
        "p",
        "v",
        "MR",
        "v",
        "slug",
        "d",
        "a",
        1,
        "",
        "a.zip",
        "https://fixture.invalid/a.zip",
        ts(cached_at),
        "m1",
    )
    .unwrap();
}

#[test]
fn a_world_pack_without_platform_data_is_due_for_both_lookups() {
    let (_d, mut conn) = migrated_db();
    world_pack(&mut conn);
    let cf = mfc::instance_mods_needing_cf_refresh_conn(&rg(&conn), 7, ts(5_000)).unwrap();
    assert_eq!(
        cf.iter()
            .map(|r| r.metadata_id.as_str())
            .collect::<Vec<_>>(),
        ["m1"]
    );
    let mr =
        mfc::instance_mods_needing_mr_context_conn(&rg(&conn), 7, "instance", "world", ts(5_000))
            .unwrap();
    assert_eq!(
        mr.iter()
            .map(|r| (r.metadata_id.as_str(), r.addon_type.as_str()))
            .collect::<Vec<_>>(),
        [("m1", "world_datapacks")]
    );
}

#[test]
fn a_world_pack_with_fresh_platform_data_is_not_due() {
    let (_d, mut conn) = migrated_db();
    world_pack(&mut conn);
    cached_on_both_platforms(&mut conn, 10_000);
    mm::upsert_mr_update_context_conn(&wg(&mut conn), "m1", "world:datapacks", "", ts(10_000))
        .unwrap();
    assert!(
        mfc::instance_mods_needing_cf_refresh_conn(&rg(&conn), 7, ts(5_000))
            .unwrap()
            .is_empty()
    );
    assert!(
        mfc::instance_mods_needing_mr_context_conn(&rg(&conn), 7, "instance", "world", ts(5_000))
            .unwrap()
            .is_empty()
    );
    // Update paths stored under the instance's key do not answer for a world.
    mm::upsert_mr_update_context_conn(&wg(&mut conn), "m1", "world:datapacks", "", ts(1)).unwrap();
    mm::upsert_mr_update_context_conn(&wg(&mut conn), "m1", "instance:datapacks", "", ts(10_000))
        .unwrap();
    assert_eq!(
        mfc::instance_mods_needing_mr_context_conn(&rg(&conn), 7, "instance", "world", ts(5_000))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn an_addon_that_is_also_a_world_pack_is_due_under_each_context() {
    let (_d, mut conn) = migrated_db();
    world_pack(&mut conn);
    cached_on_both_platforms(&mut conn, 10_000);
    mfc::upsert_mod_file_cache_conn(
        &wg(&mut conn),
        7,
        "a.zip",
        10,
        true,
        "datapacks",
        "m1",
        ts(1),
    )
    .unwrap();
    let due = |conn: &Connection| {
        let mut due: Vec<String> = mfc::instance_mods_needing_mr_context_conn(
            &rg(conn),
            7,
            "instance",
            "world",
            ts(5_000),
        )
        .unwrap()
        .into_iter()
        .map(|r| r.addon_type)
        .collect();
        due.sort();
        due
    };
    assert_eq!(due(&conn), ["datapacks", "world_datapacks"]);
    mm::upsert_mr_update_context_conn(&wg(&mut conn), "m1", "instance:datapacks", "", ts(10_000))
        .unwrap();
    assert_eq!(due(&conn), ["world_datapacks"]);
    mm::upsert_mr_update_context_conn(&wg(&mut conn), "m1", "world:datapacks", "", ts(10_000))
        .unwrap();
    assert!(due(&conn).is_empty());
}

#[test]
fn stale_logos_of_world_packs_are_refreshed() {
    let (_d, mut conn) = migrated_db();
    world_pack(&mut conn);
    cached_on_both_platforms(&mut conn, 10_000);
    mm::upsert_cf_image_conn(&wg(&mut conn), "m1", "https://fixture.invalid/cf.png").unwrap();
    mm::upsert_mr_image_conn(&wg(&mut conn), "m1", "https://fixture.invalid/mr.png").unwrap();
    let cf = mfc::instance_mods_stale_cf_logo_conn(&rg(&conn), 7).unwrap();
    assert_eq!(
        cf.iter()
            .map(|r| (r.filename.as_str(), r.metadata_id.as_str(), r.project_id))
            .collect::<Vec<_>>(),
        [("a.zip", "m1", 5)]
    );
    let mr = mfc::instance_mods_stale_mr_logo_conn(&rg(&conn), 7).unwrap();
    assert_eq!(
        mr.iter()
            .map(|r| (
                r.filename.as_str(),
                r.metadata_id.as_str(),
                r.project_id.as_str()
            ))
            .collect::<Vec<_>>(),
        [("a.zip", "m1", "p")]
    );
    mm::mark_cf_image_downloaded_conn(&wg(&mut conn), "m1", &[1]).unwrap();
    mm::mark_mr_image_downloaded_conn(&wg(&mut conn), "m1", &[1]).unwrap();
    assert!(
        mfc::instance_mods_stale_cf_logo_conn(&rg(&conn), 7)
            .unwrap()
            .is_empty()
    );
    assert!(
        mfc::instance_mods_stale_mr_logo_conn(&rg(&conn), 7)
            .unwrap()
            .is_empty()
    );
}
