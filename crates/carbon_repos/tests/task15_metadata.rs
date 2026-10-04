use carbon_repos::{
    db_exec::test_support::wg,
    dbtypes::{DbDateTime, from_millis},
    repos::mod_metadata as mm,
};
use rusqlite::Connection;
fn db() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    carbon_repos::get_migrations().0.to_latest(&mut c).unwrap();
    c.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
    c
}
fn seed(c: &Connection, id: &str, hash: u8) {
    c.execute(
        "INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES (?1,1,?2,x'01','')",
        rusqlite::params![id, vec![hash; 64]],
    )
    .unwrap();
}
fn ts() -> DbDateTime {
    DbDateTime(from_millis(100).unwrap())
}
fn mr(c: &mut Connection, id: &str, hash: u8) -> rusqlite::Result<String> {
    mm::upsert_mr_mod_cache_conn(
        &wg(c),
        &hex::encode(vec![hash; 64]),
        "project",
        "version",
        "title",
        "v",
        "slug",
        "desc",
        "",
        0,
        "",
        &format!("{id}.jar"),
        &format!("https://fixture.invalid/{id}.jar"),
        ts(),
        id,
    )
}
fn cf(c: &mut Connection, id: &str) -> rusqlite::Result<String> {
    mm::upsert_cf_mod_cache_conn(
        &wg(c),
        1,
        1,
        1,
        "title",
        "v",
        "slug",
        "desc",
        "",
        0,
        "",
        ts(),
        id,
    )
}
#[test]
fn task15_distinct_artifacts_in_one_version_keep_exact_owners() {
    for order in [[("a", 1), ("b", 2)], [("b", 2), ("a", 1)]] {
        let mut c = db();
        c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        seed(&c, "a", 1);
        seed(&c, "b", 2);
        for (id, hash) in order {
            assert_eq!(mr(&mut c, id, hash).unwrap(), id);
        }
        for (id, hash) in [("a", 1), ("b", 2)] {
            assert_eq!(mr(&mut c, id, hash).unwrap(), id);
            let r = mm::get_mr_cache_by_metadata_conn(&wg(&mut c), id)
                .unwrap()
                .unwrap();
            assert_eq!(
                (r.sha512, r.filename, r.file_url),
                (
                    hex::encode(vec![hash; 64]),
                    format!("{id}.jar"),
                    format!("https://fixture.invalid/{id}.jar")
                )
            );
        }
    }
}
#[test]
fn task15_mismatched_artifacts_and_late_saves_cannot_publish() {
    let mut c = db();
    seed(&c, "live", 1);
    assert!(mr(&mut c, "live", 2).is_err());
    assert!(mr(&mut c, "absent", 1).is_err());
    assert!(cf(&mut c, "absent").is_err());
    assert_eq!(
        mm::insert_local_image_conn(&wg(&mut c), "absent", b"image").unwrap(),
        0
    );
    assert_eq!(
        mm::upsert_mr_image_conn(&wg(&mut c), "absent", "fixture").unwrap(),
        0
    );
    assert_eq!(
        mm::upsert_cf_image_conn(&wg(&mut c), "absent", "fixture").unwrap(),
        0
    );
    assert_eq!(
        mm::upsert_mr_update_context_conn(&wg(&mut c), "absent", "context", "paths", ts()).unwrap(),
        0
    );
}
#[test]
fn task15_gc_cleans_fk_off_children_and_repairs_dead_cf_owner() {
    let mut c = db();
    seed(&c, "old", 1);
    mr(&mut c, "old", 1).unwrap();
    cf(&mut c, "old").unwrap();
    mm::insert_local_image_conn(&wg(&mut c), "old", b"image").unwrap();
    mm::upsert_mr_image_conn(&wg(&mut c), "old", "fixture").unwrap();
    mm::upsert_cf_image_conn(&wg(&mut c), "old", "fixture").unwrap();
    mm::upsert_mr_update_context_conn(&wg(&mut c), "old", "context", "paths", ts()).unwrap();
    assert_eq!(mm::gc_orphan_metadata_conn(&wg(&mut c)).unwrap(), 1);
    for table in [
        "ModMetadata",
        "LocalModImageCache",
        "ModrinthModCache",
        "CurseForgeModCache",
        "ModrinthModImageCache",
        "CurseForgeModImageCache",
        "ModrinthUpdateCache",
    ] {
        assert_eq!(
            c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i32>(0))
                .unwrap(),
            0,
            "{table}"
        );
    }
    seed(&c, "new", 1);
    assert_eq!(cf(&mut c, "new").unwrap(), "new");
    assert_eq!(mr(&mut c, "new", 1).unwrap(), "new");
    c.execute("DELETE FROM ModMetadata WHERE id='new'", [])
        .unwrap();
    seed(&c, "newer", 1);
    assert_eq!(cf(&mut c, "newer").unwrap(), "newer");
}
#[test]
fn task15_gc_real_sql_uses_set_reference_plan() {
    let c = db();
    let queries = mm::all_queries();
    let mut inspected = 0;
    for q in queries.iter().filter(|q| q.name.contains("gc_")) {
        let mut st = c.prepare(&format!("EXPLAIN QUERY PLAN {}", q.sql)).unwrap();
        let plan = st
            .query_map([], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            !plan.iter().any(|s| s.contains("CORRELATED")),
            "{}: {plan:?}",
            q.name
        );
        inspected += 1;
    }
    assert!(inspected > 0);
}
#[test]
fn task15_file_publication_rejects_collected_owner_even_without_fks() {
    use carbon_repos::repos::mod_file_cache as files;
    let mut c = db();
    seed(&c, "pending", 1);
    mm::gc_orphan_metadata_conn(&wg(&mut c)).unwrap();
    assert!(
        files::upsert_mod_file_cache_conn(
            &wg(&mut c),
            1,
            "a.jar",
            1,
            true,
            "mods",
            "pending",
            ts()
        )
        .is_err()
    );
    assert!(
        files::upsert_server_mod_file_cache_at_location_conn(
            &wg(&mut c),
            1,
            "a.jar",
            "mods/a.jar",
            1,
            true,
            "mods",
            "pending",
            ts()
        )
        .is_err()
    );
}

#[test]
fn task15_gc_preserves_instance_server_and_shared_owners_with_linear_work() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let mut previous = 0;
    for count in [128, 512, 2048] {
        let mut c = db();
        {
            let tx = c.transaction().unwrap();
            for n in 0..count {
                let id = format!("live-{n}");
                seed(&tx, &id, 1);
                let table = if n % 2 == 0 {
                    "ModFileCache"
                } else {
                    "ServerModFileCache"
                };
                let sql = if table == "ModFileCache" {
                    "INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES(?1,1,?1,1,1,'mods',?1)"
                } else {
                    "INSERT INTO ServerModFileCache(id,serverId,filename,location,filesize,enabled,addonType,metadataId) VALUES(?1,1,?1,?1,1,1,'mods',?1)"
                };
                tx.execute(sql, [&id]).unwrap();
                tx.execute("INSERT INTO ModrinthModCache(metadataId,sha512,projectId,versionId,title,version,urlslug,description,authors,releaseType,updatePaths,filename,fileUrl,cachedAt) VALUES(?1,?2,?1,?1,'title','v','slug','desc','',0,'','a.jar','fixture',0)",rusqlite::params![&id,hex::encode(vec![1u8;64])]).unwrap();
                tx.execute("INSERT INTO CurseForgeModCache(metadataId,murmur2,projectId,fileId,name,version,urlslug,summary,authors,releaseType,updatePaths,cachedAt) VALUES(?1,1,?2,?2,'name','v','slug','desc','',0,'',0)",rusqlite::params![&id,n]).unwrap();
                tx.execute("INSERT INTO LocalModImageCache VALUES(?1,x'01')", [&id])
                    .unwrap();
                tx.execute(
                    "INSERT INTO ModrinthModImageCache VALUES(?1,'fixture',x'01',1)",
                    [&id],
                )
                .unwrap();
                tx.execute(
                    "INSERT INTO CurseForgeModImageCache VALUES(?1,'fixture',x'01',1)",
                    [&id],
                )
                .unwrap();
                tx.execute(
                    "INSERT INTO ModrinthUpdateCache VALUES(?1,'context','paths',0)",
                    [&id],
                )
                .unwrap();
            }
            seed(&tx, "orphan", 2);
            tx.commit().unwrap();
        }
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = steps.clone();
        c.progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
        assert_eq!(mm::gc_orphan_metadata_conn(&wg(&mut c)).unwrap(), 1);
        c.progress_handler(0, None::<fn() -> bool>).unwrap();
        let work = steps.load(Ordering::Relaxed);
        println!("actual GC count={count} VM steps={work}");
        if previous > 0 {
            assert!(work < previous * 6, "work must remain approximately linear");
        }
        previous = work;
        let ids = c
            .prepare("SELECT id FROM ModMetadata ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<std::collections::BTreeSet<_>, _>>()
            .unwrap();
        assert_eq!(ids, (0..count).map(|n| format!("live-{n}")).collect());
    }
}

#[test]
fn task15_gc_failure_rolls_back_children_and_parent() {
    let mut c = db();
    seed(&c, "orphan", 1);
    mr(&mut c, "orphan", 1).unwrap();
    mm::insert_local_image_conn(&wg(&mut c), "orphan", b"image").unwrap();
    c.execute_batch("CREATE TRIGGER stop_gc BEFORE DELETE ON ModMetadata BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(mm::gc_orphan_metadata_conn(&wg(&mut c)).is_err());
    for table in ["ModMetadata", "ModrinthModCache", "LocalModImageCache"] {
        assert_eq!(
            c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i32>(0))
                .unwrap(),
            1
        );
    }
}

#[tokio::test]
async fn task15_writer_gc_reinstallation_and_shared_server_reference() {
    use carbon_repos::{
        db_exec::{Db, ReadAccess},
        repos::mod_file_cache as files,
    };
    for foreign_keys in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        {
            let mut c = Connection::open(&path).unwrap();
            carbon_repos::get_migrations().0.to_latest(&mut c).unwrap();
            c.execute_batch("INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(1,'root',0);
          INSERT INTO Instance(id,name,shortpath,\"index\",groupId) VALUES(1,'fixture','fixture',0,1);
          INSERT INTO Server(id,name,shortpath,\"index\",groupId,gameVersion) VALUES(1,'server','server',0,1,'1.20.1');").unwrap();
            seed(&c, "old", 1);
        }
        let db = Db::open(&path, 2, foreign_keys).unwrap();
        files::upsert_mod_file_cache(
            &db,
            1,
            "a.jar".into(),
            1,
            false,
            "mods".into(),
            "old".into(),
            ts(),
        )
        .await
        .unwrap();
        files::upsert_server_mod_file_cache_at_location(
            &db,
            1,
            "a.jar".into(),
            "custom/datapacks/a.jar".into(),
            1,
            true,
            "datapacks".into(),
            "old".into(),
            ts(),
        )
        .await
        .unwrap();
        db.write(|c| {
            mm::upsert_mr_mod_cache_conn(
                &c,
                &hex::encode(vec![1u8; 64]),
                "p",
                "v",
                "t",
                "v",
                "slug",
                "desc",
                "",
                0,
                "",
                "a.jar",
                "fixture",
                ts(),
                "old",
            )?;
            mm::insert_local_image_conn(&c, "old", b"image")?;
            Ok(())
        })
        .await
        .unwrap();
        db.write(|c| {
            c.raw().execute("DELETE FROM ModFileCache", [])?;
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(mm::gc_orphan_metadata(&db).await.unwrap(), 0);
        let server_before = files::get_server_mod_files_by_server(&db, 1).await.unwrap();
        assert_eq!(server_before.len(), 1);
        db.write(|c| {
            c.raw().execute("DELETE FROM ServerModFileCache", [])?;
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(mm::gc_orphan_metadata(&db).await.unwrap(), 1);
        assert!(mm::get_metadata_by_id(&db, "old").await.unwrap().is_none());
        assert_eq!(mm::upsert_mr_image(&db, "old", "late").await.unwrap(), 0);
        assert_eq!(
            mm::upsert_mr_update_context(&db, "old", "context", "paths", ts())
                .await
                .unwrap(),
            0
        );
        db.write(|c| {
            mm::insert_metadata_conn(
                &c,
                "new",
                1,
                &[1; 64],
                &[1; 20],
                "",
                None,
                None,
                None,
                None,
                None,
                ts(),
            )?;
            Ok(())
        })
        .await
        .unwrap();
        files::upsert_mod_file_cache(
            &db,
            1,
            "a.jar".into(),
            1,
            false,
            "mods".into(),
            "new".into(),
            ts(),
        )
        .await
        .unwrap();
        db.write(|c| {
            assert_eq!(
                mm::upsert_mr_mod_cache_conn(
                    &c,
                    &hex::encode(vec![1u8; 64]),
                    "p",
                    "v",
                    "t",
                    "v",
                    "slug",
                    "desc",
                    "",
                    0,
                    "",
                    "a.jar",
                    "fixture",
                    ts(),
                    "new"
                )?,
                "new"
            );
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(
            files::get_instance_mods_full(&db, 1).await.unwrap()[0]
                .mr_project_id
                .as_deref(),
            Some("p")
        );
    }
}

#[test]
fn task15_round01_gc_fk_on_many_orphans_has_linear_work() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let mut samples = Vec::new();
    for count in [128, 512, 2048] {
        let mut c = db();
        c.execute_batch("PRAGMA foreign_keys=ON;
            INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(1,'root',0);
            INSERT INTO Instance(id,name,shortpath,\"index\",groupId) VALUES(1,'instance','instance',0,1);
            INSERT INTO Server(id,name,shortpath,\"index\",groupId,gameVersion) VALUES(1,'server','server',0,1,'1.20.1');").unwrap();
        assert_eq!(
            c.pragma_query_value(None, "foreign_keys", |r| r.get::<_, i32>(0))
                .unwrap(),
            1
        );
        let expected: std::collections::BTreeSet<_> =
            (0..count).map(|n| format!("live-{n}")).collect();
        {
            let tx = c.transaction().unwrap();
            for n in 0..count {
                let id = format!("live-{n}");
                seed(&tx, &id, 1);
                seed(&tx, &format!("orphan-{n}"), 2);
                if n % 2 == 0 {
                    tx.execute("INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES(?1,1,?1,1,1,'mods',?1)", [&id]).unwrap();
                } else {
                    tx.execute("INSERT INTO ServerModFileCache(id,serverId,filename,location,filesize,enabled,addonType,metadataId) VALUES(?1,1,?1,?1,1,1,'mods',?1)", [&id]).unwrap();
                }
            }
            tx.commit().unwrap();
        }
        assert!(
            c.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .query([])
                .unwrap()
                .next()
                .unwrap()
                .is_none()
        );
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = steps.clone();
        c.progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
        assert_eq!(mm::gc_orphan_metadata_conn(&wg(&mut c)).unwrap(), count);
        c.progress_handler(0, None::<fn() -> bool>).unwrap();
        let work = steps.load(Ordering::Relaxed);
        println!("FK ON actual GC live={count} deleted={count} VM steps={work}");
        samples.push(work);
        let ids = c
            .prepare("SELECT id FROM ModMetadata ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<std::collections::BTreeSet<_>, _>>()
            .unwrap();
        assert_eq!(ids, expected);
        for table in ["ModFileCache", "ServerModFileCache"] {
            assert_eq!(
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                (count / 2) as i64
            );
        }
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
    for pair in samples.windows(2) {
        assert!(
            pair[1] < pair[0] * 6,
            "4x live and orphan rows must scale below 6x VM work: {samples:?}"
        );
    }
}
