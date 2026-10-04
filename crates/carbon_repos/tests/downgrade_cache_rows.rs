//! A database this build migrated must open in an older build that has the
//! compat runner. Migrations 26-28 narrow three scan caches; their stored downs
//! drop the cache rows the older schema cannot hold instead of refusing the
//! downgrade, and leave every row it can hold alone.

use carbon_repos::{
    compat::{MigrationSet, OpenVerdict},
    schema_dump::dump_schema,
};
use rusqlite::Connection;

fn first(count: usize) -> MigrationSet {
    let (mut set, _) = carbon_repos::get_migrations();
    set.migrations.truncate(count);
    set
}

fn strings(conn: &Connection, sql: &str) -> Vec<String> {
    conn.prepare(sql)
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Data normal use of this build creates, which the older schema cannot hold.
fn seed_current(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO InstanceGroup(id,name,groupIndex) VALUES (1,'root',0);
         INSERT INTO Instance(id,name,shortpath,\"index\",groupId) VALUES (1,'pack','pack',0,1);
         INSERT INTO Server(id,name,shortpath,\"index\",groupId,gameVersion) VALUES (7,'smp','smp',0,1,'1.21.1');
         INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES
           ('jei',1,x'0a',x'01','neoforge'), ('vt',2,x'0b',x'02',''),
           ('fresh',3,x'0c',x'03',''), ('cr-a',4,x'0d',x'04','fabric'), ('cr-b',5,x'0e',x'05','fabric');
         -- A server whose level-name is survival keeps its datapacks there, and
         -- an older copy of the same pack sits in world/.
         INSERT INTO ServerModFileCache(id,serverId,filename,filesize,enabled,addonType,metadataId,location) VALUES
           ('srv-vt-survival',7,'vt.zip',1,1,'datapacks','vt','survival/datapacks/vt.zip'),
           ('srv-vt-world',7,'vt.zip',1,1,'datapacks','vt','world/datapacks/vt.zip');
         -- Installed from CurseForge before the hash cache knew the file.
         INSERT INTO ServerModFileCache(id,serverId,filename,filesize,enabled,addonType,metadataId,location,
           sourcePlatform,sourceProjectId,sourceVersionId,sourceSha512) VALUES
           ('srv-jei',7,'jei.jar',1,0,'mods','jei','mods/jei.jar','cf','238222','5101',x'0a');
         -- One file name in two instance folders.
         INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES
           ('inst-fresh-res',1,'fresh.zip',1,1,'resourcepacks','fresh'),
           ('inst-fresh-data',1,'fresh.zip',1,1,'datapacks','fresh'),
           ('inst-jei',1,'jei.jar',1,1,'mods','jei');
         -- Two files of one Modrinth version, each with its image.
         INSERT INTO ModrinthModCache(metadataId,sha512,projectId,versionId,title,version,urlslug,description,
           authors,releaseType,updatePaths,filename,fileUrl,cachedAt) VALUES
           ('cr-a','0d','create','v6','Create','6','create','','',0,'','a.jar','https://fixture.invalid/a',1),
           ('cr-b','0e','create','v6','Create','6','create','','',0,'','b.jar','https://fixture.invalid/b',2);
         INSERT INTO ModrinthModImageCache(metadataId,url,data,upToDate) VALUES
           ('cr-a','https://fixture.invalid/a.png',x'01',1), ('cr-b','https://fixture.invalid/b.png',x'02',1);",
    )
    .unwrap();
}

#[test]
fn an_older_build_opens_a_migrated_database_and_drops_only_the_cache_rows_it_cannot_hold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gdl_conf.db");
    let mut conn = Connection::open(&path).unwrap();
    // A 2.0.40 database, upgraded by this build.
    first(25).to_latest(&mut conn).unwrap();
    let (current, count) = carbon_repos::get_migrations();
    assert_eq!(
        current.open(&mut conn, &path).unwrap(),
        OpenVerdict::Proceed
    );
    seed_current(&conn);

    // Every older build with the compat runner opens a copy: 28 overlays the
    // additive 29, 26 and 27 run the downs above them.
    assert_eq!(count, 29);
    for (older, verdict) in [
        (28, OpenVerdict::Proceed),
        (27, OpenVerdict::Downgraded),
        (26, OpenVerdict::Downgraded),
    ] {
        let copy_path = dir.path().join(format!("copy-{older}.db"));
        std::fs::copy(&path, &copy_path).unwrap();
        let mut copy = Connection::open(&copy_path).unwrap();
        assert_eq!(
            first(older).open(&mut copy, &copy_path).unwrap(),
            verdict,
            "a build with {older} migrations"
        );
    }

    assert_eq!(
        first(25).open(&mut conn, &path).unwrap(),
        OpenVerdict::Downgraded
    );
    let reference = {
        let mut fresh = Connection::open_in_memory().unwrap();
        first(25).to_latest(&mut fresh).unwrap();
        dump_schema(&fresh).unwrap()
    };
    assert_eq!(dump_schema(&conn).unwrap(), reference);
    assert_eq!(
        strings(
            &conn,
            "SELECT id||':'||filename||':'||addonType||':'||enabled FROM ServerModFileCache ORDER BY id"
        ),
        ["srv-jei:jei.jar:mods:0", "srv-vt-world:vt.zip:datapacks:1"]
    );
    assert_eq!(
        strings(
            &conn,
            "SELECT id||':'||filename||':'||addonType FROM ModFileCache ORDER BY id"
        ),
        [
            "inst-fresh-data:fresh.zip:datapacks",
            "inst-jei:jei.jar:mods"
        ]
    );
    assert_eq!(
        strings(&conn, "SELECT metadataId FROM ModrinthModCache"),
        ["cr-b"]
    );
    assert_eq!(
        strings(&conn, "SELECT metadataId FROM ModrinthModImageCache"),
        ["cr-b"]
    );
    // Shared metadata, the instance and the server stay.
    assert_eq!(
        strings(&conn, "SELECT id FROM ModMetadata ORDER BY id"),
        ["cr-a", "cr-b", "fresh", "jei", "vt"]
    );
    assert_eq!(strings(&conn, "SELECT name FROM Server"), ["smp"]);
    assert!(
        conn.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );

    // Coming back to this build migrates forward again with the rows kept.
    assert_eq!(
        current.open(&mut conn, &path).unwrap(),
        OpenVerdict::Proceed
    );
    assert_eq!(
        strings(
            &conn,
            "SELECT id||':'||location FROM ServerModFileCache ORDER BY id"
        ),
        [
            "srv-jei:mods/jei.jar",
            "srv-vt-world:world/datapacks/vt.zip"
        ]
    );
}
