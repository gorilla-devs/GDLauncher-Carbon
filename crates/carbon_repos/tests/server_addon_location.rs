use carbon_repos::{
    compat::{MigrationSet, OpenVerdict},
    db_exec::test_support::wg,
    dbtypes::DbDateTime,
    repos::mod_file_cache as mfc,
};
use rusqlite::Connection;
fn older() -> MigrationSet {
    let (mut migrations, _) = carbon_repos::get_migrations();
    migrations.migrations.truncate(25);
    migrations
}
/// The schema the older build checks a down-run against.
fn older_schema() -> String {
    let mut conn = Connection::open_in_memory().unwrap();
    older().to_latest(&mut conn).unwrap();
    carbon_repos::schema_dump::dump_schema(&conn).unwrap()
}
/// (id, filename, addonType, metadataId) of every server addon row, by id.
fn server_rows(conn: &Connection) -> Vec<(String, String, String, String)> {
    let mut statement = conn
        .prepare("SELECT id, filename, addonType, metadataId FROM ServerModFileCache ORDER BY id")
        .unwrap();
    statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
fn row(id: &str, filename: &str, kind: &str) -> (String, String, String, String) {
    (id.into(), filename.into(), kind.into(), "shared".into())
}
fn seed(conn: &Connection) {
    conn.execute_batch("PRAGMA foreign_keys=OFF;
    INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES ('shared',1,x'01',x'02','forge');
    INSERT INTO ServerModFileCache(id,serverId,filename,filesize,enabled,addonType,metadataId) VALUES
      ('mod',7,'one.jar',3,0,'mods','shared'), ('pack',7,'pack.zip',3,1,'datapacks','shared');").unwrap();
}
#[test]
fn task07_upgrade_preserves_known_ids_refs_and_same_name_worlds_are_distinct() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    older().to_latest(&mut conn).unwrap();
    seed(&conn);
    let (new, _) = carbon_repos::get_migrations();
    assert_eq!(new.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    let old = mfc::get_server_mod_file_cache_by_id_conn(
        &carbon_repos::db_exec::test_support::rg(&conn),
        "pack",
    )
    .unwrap()
    .unwrap();
    assert_eq!(old.location, "world/datapacks/pack.zip");
    assert_eq!(old.metadata_id, "shared");
    mfc::upsert_server_mod_file_cache_at_location_conn(
        &wg(&mut conn),
        7,
        "pack.zip",
        "B/datapacks/pack.zip",
        3,
        true,
        "datapacks",
        "shared",
        DbDateTime(chrono::Utc::now().fixed_offset()),
    )
    .unwrap();
    let rows = mfc::get_server_mod_files_by_server_conn(
        &carbon_repos::db_exec::test_support::rg(&conn),
        7,
    )
    .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().filter(|row| row.id == "pack").count(), 1);
    assert!(
        rows.iter()
            .any(|row| row.location == "B/datapacks/pack.zip" && row.id != "pack")
    );
    std::fs::write(root.path().join("world-pack.zip"), "old world save bytes").unwrap();
    std::fs::write(root.path().join("B-pack.zip"), "new world save bytes").unwrap();
    // The older build knows only world/datapacks and one row per file name:
    // the row of world B is a scan cache it cannot hold, so the downgrade
    // drops it and keeps the owners it can represent, ids and all.
    assert_eq!(
        older().open(&mut conn, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    assert_eq!(
        server_rows(&conn),
        vec![
            row("mod", "one.jar", "mods"),
            row("pack", "pack.zip", "datapacks")
        ]
    );
    assert_eq!(
        carbon_repos::schema_dump::dump_schema(&conn).unwrap(),
        older_schema()
    );
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        25
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM _migrations", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        25
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("world-pack.zip")).unwrap(),
        "old world save bytes"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("B-pack.zip")).unwrap(),
        "new world save bytes"
    );
}
#[test]
fn task07_representable_downgrade_preserves_rows_and_metadata() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    older().to_latest(&mut conn).unwrap();
    seed(&conn);
    let (new, _) = carbon_repos::get_migrations();
    assert_eq!(new.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    assert_eq!(
        older().open(&mut conn, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    assert_eq!(
        conn.query_row(
            "SELECT enabled FROM ServerModFileCache WHERE id='mod'",
            [],
            |r| r.get::<_, bool>(0)
        )
        .unwrap(),
        false
    );
    assert_eq!(
        conn.query_row(
            "SELECT metadataId FROM ServerModFileCache WHERE id='pack'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "shared"
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ModMetadata WHERE id='shared'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
#[test]
fn task07_fresh_baseline_has_location_and_matches_chain() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    let (new, _) = carbon_repos::get_migrations();
    assert_eq!(new.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    conn.execute("INSERT INTO ServerModFileCache(id,serverId,filename,filesize,addonType,metadataId,location) VALUES ('a',1,'a.zip',3,'datapacks','shared','A/datapacks/a.zip')",[]).unwrap();
}

#[test]
fn task07_old_filename_collision_downgrades_keeping_the_mods_row() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    let (new, _) = carbon_repos::get_migrations();
    new.open(&mut conn, &path).unwrap();
    // `z` sorts last by id, so only the mods preference keeps it.
    conn.execute_batch("PRAGMA foreign_keys=OFF;
      INSERT INTO ServerModFileCache(id,serverId,filename,filesize,addonType,metadataId,location) VALUES
      ('z',1,'same.zip',3,'mods','shared','mods/same.zip'),
      ('b',1,'same.zip',3,'datapacks','shared','world/datapacks/same.zip'),
      ('c',1,'other.zip',3,'datapacks','shared','world/datapacks/other.zip');").unwrap();
    assert_eq!(
        older().open(&mut conn, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    assert_eq!(
        server_rows(&conn),
        vec![
            row("c", "other.zip", "datapacks"),
            row("z", "same.zip", "mods")
        ]
    );
    assert_eq!(
        carbon_repos::schema_dump::dump_schema(&conn).unwrap(),
        older_schema()
    );
}

#[test]
fn task07r1_r4_selected_source_down_keeps_the_row_and_falls_back_to_the_hash_caches() {
    use carbon_repos::repos::mod_metadata as metadata;
    for provider in ["cf", "mr"] {
        for representable in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("fixture.db");
            let mut conn = Connection::open(&path).unwrap();
            older().to_latest(&mut conn).unwrap();
            seed(&conn);
            conn.execute(
                "UPDATE ModMetadata SET sha512=?1 WHERE id='shared'",
                [&[7u8; 64][..]],
            )
            .unwrap();
            let stale = DbDateTime(chrono::Utc::now().fixed_offset());
            if provider == "cf" {
                metadata::upsert_cf_mod_cache_conn(
                    &wg(&mut conn),
                    1,
                    23,
                    1,
                    "title",
                    "one",
                    "slug",
                    "summary",
                    "",
                    0,
                    "",
                    stale,
                    "shared",
                )
                .unwrap();
            } else {
                metadata::upsert_mr_mod_cache_conn(
                    &wg(&mut conn),
                    "hash",
                    "23",
                    "1",
                    "title",
                    "one",
                    "slug",
                    "summary",
                    "",
                    0,
                    "",
                    "one.jar",
                    "https://fixture.invalid/not-requested",
                    stale,
                    "shared",
                )
                .unwrap();
            }
            let (new, _) = carbon_repos::get_migrations();
            new.open(&mut conn, &path).unwrap();
            assert_eq!(
                mfc::publish_server_source_conn(
                    &wg(&mut conn),
                    7,
                    "mods",
                    "mods/one.jar",
                    provider,
                    "23",
                    if representable { "1" } else { "2" },
                    &[7; 64],
                    stale
                )
                .unwrap(),
                1
            );
            std::fs::write(root.path().join("sentinel.jar"), b"healthy shared artifact").unwrap();
            // A source the hash caches do not mirror (version 2) is dropped
            // with its columns like any other: the older build reads identity
            // from the hash caches, as 2.0.40 did.
            assert_eq!(
                older().open(&mut conn, &path).unwrap(),
                OpenVerdict::Downgraded,
                "{provider}, representable: {representable}"
            );
            assert_eq!(
                conn.query_row(
                    "SELECT metadataId FROM ServerModFileCache WHERE id='mod'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "shared"
            );
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM pragma_table_info('ServerModFileCache') WHERE name LIKE 'source%'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            assert_eq!(
                carbon_repos::schema_dump::dump_schema(&conn).unwrap(),
                older_schema()
            );
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM ServerModFileCache WHERE metadataId='shared'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                2
            );
            assert_eq!(
                std::fs::read(root.path().join("sentinel.jar")).unwrap(),
                b"healthy shared artifact"
            );
            if provider == "cf" {
                assert_eq!(
                    metadata::get_cf_cache_by_metadata_conn(&wg(&mut conn), "shared")
                        .unwrap()
                        .unwrap()
                        .file_id,
                    1
                );
            } else {
                assert_eq!(
                    metadata::get_mr_cache_by_metadata_conn(&wg(&mut conn), "shared")
                        .unwrap()
                        .unwrap()
                        .version_id,
                    "1"
                );
            }
            // Upgrading again keeps the row and shows the hash-cache identity.
            assert_eq!(new.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
            let row = mfc::get_server_mods_full_conn(&wg(&mut conn), 7)
                .unwrap()
                .into_iter()
                .find(|row| row.id == "mod")
                .unwrap();
            assert_eq!(row.source_platform, None);
            if provider == "cf" {
                assert_eq!((row.cf_project_id, row.cf_file_id), (Some(23), Some(1)));
            } else {
                assert_eq!(
                    (row.mr_project_id.as_deref(), row.mr_version_id.as_deref()),
                    (Some("23"), Some("1"))
                );
            }
        }
    }
}

#[test]
fn task07r1_r4_scan_preserves_source_only_for_current_physical_hash() {
    let root = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(root.path().join("fixture.db")).unwrap();
    carbon_repos::get_migrations()
        .0
        .to_latest(&mut conn)
        .unwrap();
    conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    for (id, hash) in [
        ("same", [1u8; 64]),
        ("equal", [1u8; 64]),
        ("changed", [2u8; 64]),
    ] {
        carbon_repos::repos::mod_metadata::insert_metadata_conn(
            &wg(&mut conn),
            id,
            1,
            &hash,
            &[0; 20],
            "forge",
            None,
            None,
            None,
            None,
            None,
            DbDateTime(chrono::Utc::now().fixed_offset()),
        )
        .unwrap();
    }
    let stamp = DbDateTime(chrono::Utc::now().fixed_offset());
    mfc::upsert_server_mod_file_cache_at_location_conn(
        &wg(&mut conn),
        7,
        "one.jar",
        "mods/one.jar",
        3,
        false,
        "mods",
        "same",
        stamp,
    )
    .unwrap();
    assert_eq!(
        mfc::publish_server_source_conn(
            &wg(&mut conn),
            7,
            "mods",
            "mods/one.jar",
            "mr",
            "23",
            "2",
            &[1; 64],
            stamp
        )
        .unwrap(),
        1
    );
    let id = mfc::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap()[0]
        .id
        .clone();
    for meta in ["same", "equal", "changed"] {
        mfc::upsert_server_mod_file_cache_at_location_conn(
            &wg(&mut conn),
            7,
            "one.jar",
            "mods/one.jar",
            3,
            false,
            "mods",
            meta,
            stamp,
        )
        .unwrap();
        let row = mfc::get_server_mods_full_conn(&wg(&mut conn), 7)
            .unwrap()
            .remove(0);
        assert_eq!(row.id, id);
        assert!(!row.enabled);
        assert_eq!(
            row.source_version_id.as_deref(),
            if meta == "changed" { None } else { Some("2") }
        );
        assert_eq!(
            row.mr_project_id.as_deref(),
            if meta == "changed" { None } else { Some("23") }
        );
    }
}

/// Fabric API in a server's `mods/`, with its metadata and both platforms'
/// hash caches: CurseForge knows the bytes as project 306612 file 5 and
/// Modrinth as project P7dR8mSH version `hash-version`.
fn fabric_api_known_to_both_platforms(conn: &mut Connection) -> [u8; 64] {
    use carbon_repos::repos::mod_metadata as metadata;
    let sha = [9u8; 64];
    let now = DbDateTime(chrono::Utc::now().fixed_offset());
    metadata::insert_metadata_conn(
        &wg(conn),
        "fabric",
        1,
        &sha,
        &[0; 20],
        "fabric",
        Some("Fabric API"),
        Some("fabric-api"),
        None,
        None,
        None,
        now,
    )
    .unwrap();
    mfc::upsert_server_mod_file_cache_at_location_conn(
        &wg(conn),
        7,
        "fabric-api.jar",
        "mods/fabric-api.jar",
        3,
        true,
        "mods",
        "fabric",
        now,
    )
    .unwrap();
    metadata::upsert_cf_mod_cache_conn(
        &wg(conn),
        1,
        306612,
        5,
        "Fabric API",
        "0.92",
        "fabric-api",
        "",
        "",
        1,
        "",
        now,
        "fabric",
    )
    .unwrap();
    metadata::upsert_mr_mod_cache_conn(
        &wg(conn),
        &hex::encode(sha),
        "P7dR8mSH",
        "hash-version",
        "Fabric API",
        "0.92",
        "fabric-api",
        "",
        "",
        1,
        "",
        "fabric-api.jar",
        "https://fixture.invalid/not-requested",
        now,
        "fabric",
    )
    .unwrap();
    sha
}

fn fresh_db(root: &std::path::Path) -> Connection {
    let mut conn = Connection::open(root.join("fixture.db")).unwrap();
    carbon_repos::get_migrations()
        .0
        .to_latest(&mut conn)
        .unwrap();
    conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    conn
}

/// (cfProjectId, cfFileId, mrProjectId, mrVersionId) of the only row.
fn identity(conn: &mut Connection) -> (Option<i32>, Option<i32>, Option<String>, Option<String>) {
    let mut rows = mfc::get_server_mods_full_conn(&wg(conn), 7).unwrap();
    assert_eq!(rows.len(), 1);
    let row = rows.remove(0);
    (
        row.cf_project_id,
        row.cf_file_id,
        row.mr_project_id,
        row.mr_version_id,
    )
}

/// A copy installed from one platform is still known to the other: the
/// recorded source (the version the user picked) overrides only its own
/// platform, and the other keeps the identity its cache matched by hash, as
/// 2.0.40 returned it.
#[test]
fn recorded_source_overrides_only_its_own_platform() {
    for (platform, project, version, expected) in [
        (
            "mr",
            "P7dR8mSH",
            "picked",
            (
                Some(306612),
                Some(5),
                Some("P7dR8mSH".to_owned()),
                Some("picked".to_owned()),
            ),
        ),
        (
            "cf",
            "306612",
            "77",
            (
                Some(306612),
                Some(77),
                Some("P7dR8mSH".to_owned()),
                Some("hash-version".to_owned()),
            ),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut conn = fresh_db(root.path());
        let sha = fabric_api_known_to_both_platforms(&mut conn);
        assert_eq!(
            mfc::publish_server_source_conn(
                &wg(&mut conn),
                7,
                "mods",
                "mods/fabric-api.jar",
                platform,
                project,
                version,
                &sha,
                DbDateTime(chrono::Utc::now().fixed_offset()),
            )
            .unwrap(),
            1
        );
        assert_eq!(identity(&mut conn), expected, "source {platform}");
    }
}

/// A source recorded for other bytes than the file has now is ignored: both
/// platforms fall back to the hash caches instead of losing their ids.
#[test]
fn source_for_other_bytes_falls_back_to_the_hash_caches() {
    for platform in ["cf", "mr"] {
        let root = tempfile::tempdir().unwrap();
        let mut conn = fresh_db(root.path());
        fabric_api_known_to_both_platforms(&mut conn);
        conn.execute(
            "UPDATE ServerModFileCache SET sourcePlatform=?1, sourceProjectId='1', sourceVersionId='2', sourceSha512=?2",
            rusqlite::params![platform, &[1u8; 64][..]],
        )
        .unwrap();
        assert_eq!(
            identity(&mut conn),
            (
                Some(306612),
                Some(5),
                Some("P7dR8mSH".to_owned()),
                Some("hash-version".to_owned()),
            ),
            "stale {platform} source"
        );
    }
}

/// A row 2.0.40 wrote or a scan found records no source: both platforms'
/// ids come from the hash caches, as before; a copy no platform knows has
/// none. The jar's loader mod id is returned either way.
#[test]
fn row_without_a_source_keeps_the_hash_identity_and_mod_id() {
    let root = tempfile::tempdir().unwrap();
    let mut conn = fresh_db(root.path());
    fabric_api_known_to_both_platforms(&mut conn);
    assert_eq!(
        identity(&mut conn),
        (
            Some(306612),
            Some(5),
            Some("P7dR8mSH".to_owned()),
            Some("hash-version".to_owned()),
        )
    );
    let row = mfc::get_server_mods_full_conn(&wg(&mut conn), 7)
        .unwrap()
        .remove(0);
    assert_eq!(row.source_platform, None);
    assert_eq!(row.modid.as_deref(), Some("fabric-api"));
    assert_eq!(row.cache_cf_project_id, Some(306612));
    assert_eq!(row.cache_mr_project_id.as_deref(), Some("P7dR8mSH"));

    conn.execute_batch("DELETE FROM CurseForgeModCache; DELETE FROM ModrinthModCache;")
        .unwrap();
    assert_eq!(identity(&mut conn), (None, None, None, None));
}
