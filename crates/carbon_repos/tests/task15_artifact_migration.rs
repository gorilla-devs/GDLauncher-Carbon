use carbon_repos::{
    compat::{MigrationSet, OpenVerdict},
    db_exec::test_support::wg,
    dbtypes::{DbDateTime, from_millis},
    repos::{mod_file_cache as files, mod_metadata as mm},
    schema_dump::dump_schema,
};
use rusqlite::Connection;
fn old(n: usize) -> MigrationSet {
    let mut s = carbon_repos::get_migrations().0;
    s.migrations.truncate(n);
    s
}
fn seed(c: &Connection) {
    c.execute_batch("PRAGMA foreign_keys=ON;
    INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(1,'root',0);
    INSERT INTO Instance(id,name,shortpath,\"index\",groupId,libraryPosition) VALUES(1,'instance','fixture',0,1,0);
    INSERT INTO Server(id,name,shortpath,\"index\",groupId,gameVersion) VALUES(1,'server','server',0,1,'1.20.1');
    INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES('physical',1,x'01',x'02','forge');
    INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES('instance-file',1,'renamed.jar',4,0,'mods','physical');
    INSERT INTO ServerModFileCache(id,serverId,filename,filesize,enabled,addonType,metadataId) VALUES('server-file',1,'a.jar',4,1,'mods','physical');
    INSERT INTO ModrinthModCache(metadataId,sha512,projectId,versionId,title,version,urlslug,description,authors,releaseType,updatePaths,filename,fileUrl,cachedAt) VALUES('physical','01','project','version','title','v','slug','desc','',0,'','a.jar','https://fixture.invalid/a.jar',0);
    INSERT INTO LocalModImageCache VALUES('physical',x'01');
    INSERT INTO ModrinthModImageCache VALUES('physical','fixture',x'02',1);").unwrap();
}
fn identities(c: &Connection) -> (String, String, i32) {
    (
        c.query_row(
            "SELECT id||':'||metadataId||':'||enabled FROM ModFileCache",
            [],
            |r| r.get(0),
        )
        .unwrap(),
        c.query_row(
            "SELECT id||':'||metadataId||':'||enabled FROM ServerModFileCache",
            [],
            |r| r.get(0),
        )
        .unwrap(),
        c.query_row("SELECT count(*) FROM ModrinthModImageCache", [], |r| {
            r.get(0)
        })
        .unwrap(),
    )
}
fn fk(c: &Connection) {
    assert!(
        c.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
}
#[test]
fn task15_populated_legacy_current_and_representable_down_preserve_identities() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut c = Connection::open(&path).unwrap();
    old(25).to_latest(&mut c).unwrap();
    seed(&c);
    let before = identities(&c);
    let current = carbon_repos::get_migrations().0;
    assert_eq!(current.open(&mut c, &path).unwrap(), OpenVerdict::Proceed);
    c.execute_batch("UPDATE ServerModFileCache SET sourcePlatform='mr',sourceProjectId='project',sourceVersionId='version',sourceSha512=x'01'").unwrap();
    assert_eq!(identities(&c), before);
    fk(&c);
    assert_eq!(
        old(27).open(&mut c, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    assert_eq!(identities(&c), before);
    fk(&c);
    assert_eq!(
        old(25).open(&mut c, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    assert_eq!(identities(&c), before);
    fk(&c);
    assert_eq!(current.open(&mut c, &path).unwrap(), OpenVerdict::Proceed);
    assert_eq!(identities(&c), before);
}
#[test]
fn task15_artifact_collision_down_refuses_without_mutating_current_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut c = Connection::open(&path).unwrap();
    old(25).to_latest(&mut c).unwrap();
    seed(&c);
    carbon_repos::get_migrations()
        .0
        .open(&mut c, &path)
        .unwrap();
    c.execute_batch("INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES('second',2,x'02',x'03','forge');
 INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES('second-file',1,'b.jar',4,1,'mods','second')").unwrap();
    mm::upsert_mr_mod_cache_conn(
        &wg(&mut c),
        "02",
        "project",
        "version",
        "title",
        "v",
        "slug",
        "desc",
        "",
        0,
        "",
        "b.jar",
        "https://fixture.invalid/b.jar",
        DbDateTime(from_millis(1).unwrap()),
        "second",
    )
    .unwrap();
    let schema = dump_schema(&c).unwrap();
    let rows = c
        .query_row(
            "SELECT group_concat(id||':'||metadataId) FROM ModFileCache",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap();
    assert!(matches!(
        old(27).open(&mut c, &path).unwrap(),
        OpenVerdict::Refuse(RefusalKind::DowngradeFailed { .. })
    ));
    assert_eq!(dump_schema(&c).unwrap(), schema);
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        29
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM _migrations", [], |r| r
            .get::<_, i32>(0))
            .unwrap(),
        29
    );
    assert_eq!(
        c.query_row(
            "SELECT group_concat(id||':'||metadataId) FROM ModFileCache",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        rows
    );
    fk(&c);
}
#[test]
fn task15_legacy_fresh_mismatch_is_unusable_but_reversibly_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut c = Connection::open(&path).unwrap();
    old(25).to_latest(&mut c).unwrap();
    seed(&c);
    c.execute_batch("UPDATE ModrinthModCache SET sha512='02',cachedAt=9999999999999; UPDATE ModFileCache SET enabled=1").unwrap();
    carbon_repos::get_migrations()
        .0
        .open(&mut c, &path)
        .unwrap();
    assert!(
        mm::get_mr_cache_by_metadata_conn(&wg(&mut c), "physical")
            .unwrap()
            .is_none()
    );
    let row = files::get_instance_mr_export_files_conn(&wg(&mut c), 1)
        .unwrap()
        .remove(0);
    assert!(row.mr_file_url.is_none());
    assert_eq!(
        files::instance_mods_needing_mr_context_conn(
            &wg(&mut c),
            1,
            "context",
            DbDateTime(from_millis(1).unwrap())
        )
        .unwrap()
        .len(),
        1
    );
    assert_eq!(
        c.query_row("SELECT sha512 FROM ModrinthModCache", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "02"
    );
    assert_eq!(
        old(27).open(&mut c, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    fk(&c);
}
#[test]
fn task15_interrupted_artifact_step_rolls_back_and_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut c = Connection::open(&path).unwrap();
    old(27).to_latest(&mut c).unwrap();
    c.execute_batch(
        "CREATE INDEX ServerModFileCache_metadataId_idx ON ServerModFileCache(serverId)",
    )
    .unwrap();
    let schema = dump_schema(&c).unwrap();
    assert!(
        carbon_repos::get_migrations()
            .0
            .open(&mut c, &path)
            .is_err()
    );
    assert_eq!(dump_schema(&c).unwrap(), schema);
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        27
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM _migrations", [], |r| r
            .get::<_, i32>(0))
            .unwrap(),
        27
    );
    c.execute_batch("DROP INDEX ServerModFileCache_metadataId_idx")
        .unwrap();
    drop(c);
    let mut c = Connection::open(&path).unwrap();
    assert_eq!(
        carbon_repos::get_migrations()
            .0
            .open(&mut c, &path)
            .unwrap(),
        OpenVerdict::Proceed
    );
    fk(&c);
}
