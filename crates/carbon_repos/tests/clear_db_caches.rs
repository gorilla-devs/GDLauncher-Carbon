//! The maintainer helper must clear the complete addon cache graph, while
//! preserving unrelated populated data. All databases are synthetic files.
use carbon_repos::{compat::OpenVerdict, schema_dump::dump_schema};
use rusqlite::{Connection, types::Value};
use std::{collections::BTreeMap, path::PathBuf};

const HELPER: &str = include_str!("../../../clear_db_caches.sql");
const CACHES: &[&str] = &[
    "ModFileCache",
    "ServerModFileCache",
    "LocalModImageCache",
    "CurseForgeModImageCache",
    "ModrinthModImageCache",
    "ModrinthUpdateCache",
    "CurseForgeModCache",
    "ModrinthModCache",
    "ModMetadata",
];

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    conn: Connection,
}

impl Fixture {
    fn new(baseline: bool, instance: bool, server: bool, caches: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache-clear.db");
        let mut conn = Connection::open(&path).unwrap();
        let migrations = carbon_repos::get_migrations().0;
        if baseline {
            assert_eq!(
                migrations.open(&mut conn, &path).unwrap(),
                OpenVerdict::Proceed
            );
        } else {
            migrations.to_latest(&mut conn).unwrap();
        }
        conn.execute_batch("PRAGMA foreign_keys=ON;
            INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(7,'instances',3);
            INSERT INTO Instance(id,name,shortpath,\"index\",groupId,libraryPosition,favorite) VALUES(11,'instance','synthetic',9,7,4,1);
            INSERT INTO Server(id,name,shortpath,\"index\",groupId,gameVersion,motd,onlineMode) VALUES(12,'server','synthetic-server',5,1,'1.20.1','keep this',0);
            INSERT INTO Account(uuid,username,lastUsed) VALUES('synthetic-account','fixture',1);
            INSERT INTO AppConfiguration(id,releaseChannel,xmx,language,activeAccountUuid) VALUES(0,'stable',4096,'german','synthetic-account');
            INSERT INTO FrontendPreference(key,value) VALUES('task46','preserve');
            INSERT INTO Java(id,path,major,fullVersion,type,os,arch,vendor) VALUES('synthetic-java','fixture-only',21,'21','local','linux','x64','fixture');
            INSERT INTO JavaProfile(name,javaId) VALUES('custom','synthetic-java');
            INSERT INTO HTTPCache(url,status_code,data) VALUES('synthetic-key',200,x'ff');").unwrap();
        if caches {
            conn.execute_batch("INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES('physical',1,x'01',x'02','forge');
                INSERT INTO LocalModImageCache VALUES('physical',x'01');
                INSERT INTO CurseForgeModCache(metadataId,murmur2,projectId,fileId,name,version,urlslug,summary,authors,releaseType,updatePaths,cachedAt) VALUES('physical',1,2,3,'title','v','slug','summary','',0,'',1);
                INSERT INTO CurseForgeModImageCache VALUES('physical','fixture',x'02',1);
                INSERT INTO ModrinthModCache(metadataId,sha512,projectId,versionId,title,version,urlslug,description,authors,releaseType,updatePaths,filename,fileUrl,cachedAt) VALUES('physical','01','project','version','title','v','slug','description','',0,'','renamed.jar','fixture',1);
                INSERT INTO ModrinthModImageCache VALUES('physical','fixture',x'03',1);
                INSERT INTO ModrinthUpdateCache VALUES('physical','mods:forge:1.20.1','paths',1);").unwrap();
            if instance {
                conn.execute_batch("INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId,fileStamp) VALUES('instance-file',11,'renamed.jar',4,0,'mods','physical','marker');").unwrap();
            }
            if server {
                conn.execute_batch("INSERT INTO ServerModFileCache(id,serverId,filename,location,filesize,enabled,addonType,metadataId,sourcePlatform,sourceProjectId,sourceVersionId,sourceSha512) VALUES('server-file',12,'renamed.jar','mods/renamed.jar',4,0,'mods','physical','mr','project','version',x'01');").unwrap();
            }
        }
        assert_fk(&conn);
        Self {
            _dir: dir,
            path,
            conn,
        }
    }
}

fn assert_fk(conn: &Connection) {
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

fn snapshot(conn: &Connection) -> BTreeMap<String, Vec<Vec<Value>>> {
    let names = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    names
        .into_iter()
        .map(|name| {
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM \"{name}\" ORDER BY rowid"))
                .unwrap();
            let columns = stmt.column_count();
            let rows = stmt
                .query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get::<_, Value>(i))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (name, rows)
        })
        .collect()
}

// execute_batch stops at the first failing statement. Its caller must then
// roll back or close: SQLite ABORT alone only undoes the failing statement.
fn clear(conn: &Connection) -> rusqlite::Result<()> {
    assert!(
        conn.is_autocommit(),
        "the helper requires a top-level connection"
    );
    assert_fk(conn);
    let result = conn.execute_batch(HELPER);
    if result.is_err() && !conn.is_autocommit() {
        conn.execute_batch("ROLLBACK;").unwrap();
    }
    result
}

fn successful_clear(
    baseline: bool,
    instance: bool,
    server: bool,
    caches: bool,
    start_fk_off: bool,
) {
    let fixture = Fixture::new(baseline, instance, server, caches);
    let before = snapshot(&fixture.conn);
    let schema = dump_schema(&fixture.conn).unwrap();
    if start_fk_off {
        fixture
            .conn
            .execute_batch("PRAGMA foreign_keys=OFF;")
            .unwrap();
    }
    clear(&fixture.conn).expect("the exact helper must clear valid server and instance references");
    assert!(fixture.conn.is_autocommit());
    assert_eq!(
        fixture
            .conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        1
    );
    assert_fk(&fixture.conn);
    drop(fixture.conn);
    let reopened = Connection::open(&fixture.path).unwrap();
    assert_fk(&reopened);
    assert_eq!(dump_schema(&reopened).unwrap(), schema);
    let after = snapshot(&reopened);
    for (table, rows) in before {
        if CACHES.contains(&table.as_str()) {
            assert!(after[&table].is_empty(), "{table} must be durably cleared");
        } else {
            assert_eq!(
                after[&table], rows,
                "preserve every unrelated {table} row and field"
            );
        }
    }
}

#[test]
fn clear_shared_instance_server_and_all_derived_caches() {
    successful_clear(false, true, true, true, false);
}
#[test]
fn clear_server_only_cache() {
    successful_clear(false, false, true, true, false);
}
#[test]
fn clear_instance_only_cache() {
    successful_clear(false, true, false, true, false);
}
#[test]
fn clear_enables_foreign_keys_before_its_transaction() {
    successful_clear(false, true, true, true, true);
}
#[test]
fn clear_on_fresh_baseline_with_populated_data() {
    successful_clear(true, true, true, true, false);
}
#[test]
fn clear_empty_caches_is_idempotent() {
    successful_clear(false, false, false, false, false);
    let fixture = Fixture::new(false, false, false, false);
    let before = snapshot(&fixture.conn);
    clear(&fixture.conn).unwrap();
    clear(&fixture.conn).unwrap();
    assert_eq!(snapshot(&fixture.conn), before);
}

fn failed_clear(close_without_rollback: bool) {
    let fixture = Fixture::new(false, true, true, true);
    fixture.conn.execute_batch("CREATE TRIGGER task46_late_failure BEFORE DELETE ON ModMetadata BEGIN SELECT RAISE(ABORT,'task46 late failure'); END;").unwrap();
    let before = snapshot(&fixture.conn);
    assert_fk(&fixture.conn);
    let result = if close_without_rollback {
        fixture.conn.execute_batch(HELPER)
    } else {
        clear(&fixture.conn)
    };
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("task46 late failure")
    );
    assert_eq!(
        fixture
            .conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        1
    );
    if !close_without_rollback {
        assert!(fixture.conn.is_autocommit());
        assert_eq!(snapshot(&fixture.conn), before);
    }
    drop(fixture.conn);
    let reopened = Connection::open(&fixture.path).unwrap();
    assert_eq!(
        snapshot(&reopened),
        before,
        "late ABORT must not durably commit any prior delete"
    );
    assert_fk(&reopened);
}

#[test]
fn late_failure_rolls_back_every_cache_and_preserves_healthy_rows() {
    failed_clear(false);
}
#[test]
fn stop_on_error_and_close_rolls_back_without_trailing_commit() {
    failed_clear(true);
}

#[test]
fn current_cache_fk_graph_is_covered_by_populated_fixture() {
    let fixture = Fixture::new(false, true, true, true);
    let mut edges = Vec::new();
    for table in snapshot(&fixture.conn).keys() {
        let mut stmt = fixture
            .conn
            .prepare(&format!("PRAGMA foreign_key_list(\"{table}\")"))
            .unwrap();
        let parents = stmt
            .query_map([], |r| r.get::<_, String>(2))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for parent in parents {
            if CACHES.contains(&parent.as_str()) {
                edges.push((table.clone(), parent));
            }
        }
    }
    edges.sort();
    println!("actual migrated cache FK edges: {edges:?}");
    assert_eq!(
        edges,
        [
            ("CurseForgeModCache", "ModMetadata"),
            ("CurseForgeModImageCache", "CurseForgeModCache"),
            ("LocalModImageCache", "ModMetadata"),
            ("ModFileCache", "ModMetadata"),
            ("ModrinthModCache", "ModMetadata"),
            ("ModrinthModImageCache", "ModrinthModCache"),
            ("ModrinthUpdateCache", "ModMetadata"),
            ("ServerModFileCache", "ModMetadata"),
        ]
        .map(|(child, parent)| (child.to_owned(), parent.to_owned()))
    );
}
