//! `20261026164859_server_addon_file_stamp`: server addon rows gain the
//! nullable `fileStamp` the instance rows already have, so a rescan can skip
//! files that did not change.

use carbon_repos::{
    compat::{MigrationSet, OpenVerdict},
    db_exec::test_support::{rg, wg},
    dbtypes::DbDateTime,
    repos::mod_file_cache as mfc,
    schema_dump::dump_schema,
};
use rusqlite::Connection;

/// The first `count` migrations: 25 is what 2.0.40 shipped.
fn first(count: usize) -> MigrationSet {
    let (mut set, _) = carbon_repos::get_migrations();
    set.migrations.truncate(count);
    set
}

/// The release right before this migration.
fn before_stamp() -> MigrationSet {
    let (set, _) = carbon_repos::get_migrations();
    let position = set
        .migrations
        .iter()
        .position(|migration| migration.name == "20261026164859_server_addon_file_stamp")
        .expect("the migration is registered");
    first(position)
}

/// Rows as 2.0.40 wrote them for a modded server.
fn seed_2_0_40(conn: &Connection) {
    conn.execute_batch(
        "PRAGMA foreign_keys=OFF;
         INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES ('meta',1,x'01',x'02','fabric');
         INSERT INTO ServerModFileCache(id,serverId,filename,filesize,enabled,addonType,metadataId) VALUES
           ('mod',7,'one.jar',3,1,'mods','meta'), ('off',7,'two.jar',4,0,'mods','meta'),
           ('pack',7,'pack.zip',5,1,'datapacks','meta');",
    )
    .unwrap();
}

fn rows(conn: &Connection) -> Vec<(String, String, bool, String)> {
    let mut statement = conn
        .prepare("SELECT id, location, enabled, metadataId FROM ServerModFileCache ORDER BY id")
        .unwrap();
    statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn a_2_0_40_database_upgrades_with_empty_stamps_and_down_restores_the_old_schema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    first(25).to_latest(&mut conn).unwrap();
    seed_2_0_40(&conn);

    let (current, _) = carbon_repos::get_migrations();
    assert_eq!(
        current.open(&mut conn, &path).unwrap(),
        OpenVerdict::Proceed
    );
    let upgraded = rows(&conn);
    assert_eq!(upgraded.len(), 3);
    for row in mfc::get_server_mod_files_by_server_conn(&rg(&conn), 7).unwrap() {
        assert_eq!(row.file_stamp, None, "{}: no stamp until a scan", row.id);
    }

    // A scan records a stamp; the stamp travels with its row.
    assert_eq!(
        mfc::update_server_mod_file_stamp_conn(
            &wg(&mut conn),
            7,
            "mods",
            "mods/one.jar",
            "meta",
            Some("parsed-v1:fixture|observed"),
            DbDateTime(chrono::Utc::now().fixed_offset()),
        )
        .unwrap(),
        1
    );
    assert_eq!(
        mfc::get_server_mod_file_cache_by_id_conn(&rg(&conn), "mod")
            .unwrap()
            .unwrap()
            .file_stamp
            .as_deref(),
        Some("parsed-v1:fixture|observed")
    );

    // The column is additive: the previous release opens this database as it
    // is and keeps every row.
    assert_eq!(
        before_stamp().open(&mut conn, &path).unwrap(),
        OpenVerdict::Proceed
    );
    assert_eq!(rows(&conn), upgraded);

    // The migration's down script restores exactly the previous schema.
    let (current, _) = carbon_repos::get_migrations();
    let down = current
        .migrations
        .iter()
        .find(|migration| migration.name == "20261026164859_server_addon_file_stamp")
        .and_then(|migration| migration.down_sql)
        .expect("the migration ships a down script");
    conn.execute_batch(down).unwrap();
    let reference = {
        let mut fresh = Connection::open(dir.path().join("reference.db")).unwrap();
        before_stamp().to_latest(&mut fresh).unwrap();
        dump_schema(&fresh).unwrap()
    };
    assert_eq!(dump_schema(&conn).unwrap(), reference);
    assert_eq!(rows(&conn), upgraded);
}

#[test]
fn rewriting_a_server_row_clears_its_stamp() {
    let mut conn = Connection::open_in_memory().unwrap();
    carbon_repos::get_migrations()
        .0
        .to_latest(&mut conn)
        .unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=OFF;
         INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES ('meta',1,x'01',x'02','fabric');
         INSERT INTO ServerModFileCache(id,serverId,filename,filesize,enabled,addonType,metadataId,location)
         VALUES ('mod',7,'one.jar',3,1,'mods','meta','mods/one.jar');",
    )
    .unwrap();
    let now = || DbDateTime(chrono::Utc::now().fixed_offset());
    mfc::update_server_mod_file_stamp_conn(
        &wg(&mut conn),
        7,
        "mods",
        "mods/one.jar",
        "meta",
        Some("parsed-v1:fixture|observed"),
        now(),
    )
    .unwrap();
    // A publication replacing the bytes must not inherit the old stamp.
    mfc::upsert_server_mod_file_cache_at_location_conn(
        &wg(&mut conn),
        7,
        "one.jar",
        "mods/one.jar",
        9,
        true,
        "mods",
        "meta",
        now(),
    )
    .unwrap();
    assert_eq!(
        mfc::get_server_mod_file_cache_by_id_conn(&rg(&conn), "mod")
            .unwrap()
            .unwrap()
            .file_stamp,
        None
    );
}
