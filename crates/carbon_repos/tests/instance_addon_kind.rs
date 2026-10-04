use carbon_repos::{
    compat::{MigrationSet, OpenVerdict},
    schema_dump::dump_schema,
};
use rusqlite::Connection;
fn old() -> MigrationSet {
    let (mut set, _) = carbon_repos::get_migrations();
    set.migrations.truncate(26);
    set
}
fn seed(conn: &Connection) {
    conn.execute_batch("PRAGMA foreign_keys=ON;
      INSERT INTO InstanceGroup(id,name,groupIndex) VALUES (1,'fixture',0);
      INSERT INTO Instance(id,name,shortpath,\"index\",groupId) VALUES (1,'fixture','fixture',0,1);
      INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES ('shared',1,x'01',x'02','forge');
      INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId)
      VALUES ('pack',1,'shared.zip',3,0,'resourcepacks','shared');").unwrap();
}
#[test]
fn task11_populated_upgrade_and_representable_down_preserve_owners() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    old().to_latest(&mut conn).unwrap();
    seed(&conn);
    let (new, _) = carbon_repos::get_migrations();
    assert_eq!(new.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    assert_eq!(
        conn.query_row(
            "SELECT fileStamp FROM ModFileCache WHERE id='pack'",
            [],
            |r| r.get::<_, Option<String>>(0)
        )
        .unwrap(),
        None
    );
    assert_eq!(
        old().open(&mut conn, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    assert_eq!(
        conn.query_row(
            "SELECT metadataId FROM ModFileCache WHERE id='pack' AND enabled=0",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "shared"
    );
    assert!(
        conn.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
}
#[test]
fn task11_colliding_owner_down_keeps_one_row_per_file_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let mut conn = Connection::open(&path).unwrap();
    old().to_latest(&mut conn).unwrap();
    seed(&conn);
    let (new, _) = carbon_repos::get_migrations();
    new.open(&mut conn, &path).unwrap();
    // One name in three folders, and one in two folders without a mod: the
    // older schema holds one row per name, so the mods row wins, else the
    // first by id. `zmod` sorts last, so only the mods preference keeps it.
    conn.execute_batch("INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES
      ('data',1,'shared.zip',3,1,'datapacks','shared'),
      ('zmod',1,'shared.zip',3,1,'mods','shared'),
      ('res',1,'other.zip',3,1,'resourcepacks','shared'),
      ('dat',1,'other.zip',3,1,'datapacks','shared')").unwrap();
    assert_eq!(
        old().open(&mut conn, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    let rows: Vec<(String, String, String)> = conn
        .prepare("SELECT id, filename, addonType FROM ModFileCache ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows,
        [
            ("dat", "other.zip", "datapacks"),
            ("zmod", "shared.zip", "mods")
        ]
        .map(|(id, file, kind)| (id.to_owned(), file.to_owned(), kind.to_owned()))
    );
    let reference = {
        let mut fresh = Connection::open_in_memory().unwrap();
        old().to_latest(&mut fresh).unwrap();
        dump_schema(&fresh).unwrap()
    };
    assert_eq!(dump_schema(&conn).unwrap(), reference);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        26
    );
    assert!(
        conn.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
    // Upgrading again keeps the surviving rows.
    assert_eq!(new.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    assert_eq!(
        conn.query_row("SELECT count(*) FROM ModFileCache", [], |r| r
            .get::<_, i32>(0))
            .unwrap(),
        2
    );
}

#[test]
fn task11_context_cache_keeps_foreign_keys_and_negative_freshness() {
    use carbon_repos::{
        db_exec::test_support::{rg, wg},
        dbtypes::DbDateTime,
        repos::{mod_file_cache as files, mod_metadata as metadata},
    };
    let mut conn = Connection::open_in_memory().unwrap();
    carbon_repos::get_migrations()
        .0
        .to_latest(&mut conn)
        .unwrap();
    seed(&conn);
    let now = DbDateTime(chrono::Utc::now().fixed_offset());
    metadata::upsert_mr_mod_cache_conn(
        &wg(&mut conn),
        "01",
        "p",
        "v",
        "title",
        "v",
        "slug",
        "desc",
        "",
        0,
        "",
        "shared.zip",
        "https://fixture.invalid/not-requested",
        now,
        "shared",
    )
    .unwrap();
    for context in ["A", "B"] {
        assert_eq!(
            files::instance_mods_needing_mr_context_conn(&rg(&conn), 1, context, now)
                .unwrap()
                .len(),
            1
        );
    }
    metadata::upsert_mr_update_context_conn(&wg(&mut conn), "shared", "A:resourcepacks", "", now)
        .unwrap();
    let cutoff = DbDateTime((chrono::Utc::now() - chrono::Duration::days(1)).fixed_offset());
    assert!(
        files::instance_mods_needing_mr_context_conn(&rg(&conn), 1, "A", cutoff)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        files::instance_mods_needing_mr_context_conn(&rg(&conn), 1, "B", cutoff)
            .unwrap()
            .len(),
        1
    );
    assert!(
        conn.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
    assert_eq!(
        metadata::upsert_mr_update_context_conn(&wg(&mut conn), "absent", "B", "", now).unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ModFileCache WHERE metadataId='shared'",
            [],
            |r| r.get::<_, i32>(0)
        )
        .unwrap(),
        1
    );
}
