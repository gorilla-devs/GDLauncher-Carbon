use carbon_repos::db_exec::{Db, ReadAccess};
use carbon_repos::repos::app_configuration as ac;
use carbon_repos::repos::instance as i;
use rusqlite::Connection;

/// Migrates a fresh tempfile, then opens the async `Db` pool over it. The
/// instance repo's transaction fns (`add_instance_tx`, `move_instance_tx`, …)
/// are write-pool wrappers, so the whole suite drives the real pool and its
/// read/write wrappers.
async fn migrated_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
    }
    let db = Db::open(&path, 2, false).unwrap();
    (dir, db)
}

/// Seed a group and return its id.
async fn seed_group(db: &Db, name: &str, group_index: i32) -> i32 {
    i::insert_group(db, name.to_owned(), group_index, None)
        .await
        .unwrap() as i32
}

async fn seed_instance(
    db: &Db,
    name: &str,
    shortpath: &str,
    index: i32,
    group_id: i32,
    lib_pos: Option<i32>,
) -> i32 {
    i::add_instance_tx(
        db,
        name.to_owned(),
        shortpath.to_owned(),
        index,
        group_id,
        lib_pos,
    )
    .await
    .unwrap() as i32
}

async fn idx_of(db: &Db, id: i32) -> i32 {
    i::get_instance(db, id).await.unwrap().unwrap().index
}

async fn indexes_in_group(db: &Db, group_id: i32) -> Vec<(i32, i32)> {
    // (id, index) ordered by index asc
    i::get_instances_by_group_ordered_by_index(db, group_id)
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.id, r.index))
        .collect()
}

#[tokio::test]
async fn insert_instance_takes_ddl_defaults() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0).await;
    let id = seed_instance(&db, "n", "sp", 3, g, Some(7)).await;
    let row = i::get_instance(&db, id).await.unwrap().unwrap();
    assert_eq!(row.name, "n");
    assert_eq!(row.shortpath, "sp");
    assert_eq!(row.index, 3);
    assert_eq!(row.group_id, g);
    assert_eq!(row.library_position, Some(7));
    // favorite / hasPackUpdate default to false
    assert!(!row.favorite);
    assert!(!row.has_pack_update);
}

#[tokio::test]
async fn add_instance_tx_replaces_same_shortpath() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0).await;
    let first = seed_instance(&db, "a", "dup", 0, g, None).await;
    let second = seed_instance(&db, "b", "dup", 1, g, None).await;
    assert_ne!(first, second);
    // only the second row survives at that shortpath
    assert!(i::get_instance(&db, first).await.unwrap().is_none());
    let row = i::get_instance_by_shortpath(&db, "dup")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.id, second);
    assert_eq!(row.name, "b");
}

#[tokio::test]
async fn shift_library_positions_down_exact_layout() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0).await;
    // library positions 0..5
    let mut ids: Vec<i32> = Vec::new();
    for p in 0..5 {
        ids.push(seed_instance(&db, &format!("n{p}"), &format!("sp{p}"), p, g, Some(p)).await);
    }

    // shift (1, 3] down by one -> positions 2 and 3 become 1 and 2
    let affected = db
        .write(|mut conn| {
            let tx = conn.transaction()?;
            let n = i::shift_instance_library_positions_down_conn(&tx, 1, 3)?;
            tx.commit()?;
            Ok(n)
        })
        .await
        .unwrap();
    assert_eq!(affected, 2);

    let mut pos = Vec::new();
    for &id in &ids {
        pos.push(
            i::get_instance(&db, id)
                .await
                .unwrap()
                .unwrap()
                .library_position,
        );
    }
    assert_eq!(pos[0], Some(0));
    assert_eq!(pos[1], Some(1)); // unchanged (not > 1)
    assert_eq!(pos[2], Some(1)); // 2 -> 1
    assert_eq!(pos[3], Some(2)); // 3 -> 2
    assert_eq!(pos[4], Some(4)); // unchanged (not <= 3)
}

#[tokio::test]
async fn shift_indexes_up_from_and_down_after() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0).await;
    let mut ids: Vec<i32> = Vec::new();
    for p in 0..4 {
        ids.push(seed_instance(&db, &format!("n{p}"), &format!("sp{p}"), p, g, None).await);
    }

    // increment index for all with index >= 2
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        i::shift_instance_indexes_up_from_conn(&tx, g, 2)?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(idx_of(&db, ids[0]).await, 0);
    assert_eq!(idx_of(&db, ids[1]).await, 1);
    assert_eq!(idx_of(&db, ids[2]).await, 3);
    assert_eq!(idx_of(&db, ids[3]).await, 4);

    // decrement index for all with index > 3
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        i::shift_instance_indexes_down_after_conn(&tx, g, 3)?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(idx_of(&db, ids[3]).await, 3);
    assert_eq!(idx_of(&db, ids[2]).await, 3); // was 3, not > 3, unchanged
}

#[tokio::test]
async fn move_all_instances_to_group_preserves_relative_order() {
    let (_d, db) = migrated_db().await;
    let src = seed_group(&db, "src", 0).await;
    let dst = seed_group(&db, "dst", 1).await;
    // dst already has two instances at index 0,1
    seed_instance(&db, "d0", "d0", 0, dst, None).await;
    seed_instance(&db, "d1", "d1", 1, dst, None).await;
    // src has three at 0,1,2
    let mut s: Vec<i32> = Vec::new();
    for p in 0..3 {
        s.push(seed_instance(&db, &format!("s{p}"), &format!("s{p}"), p, src, None).await);
    }

    let base_index = i::count_instances_in_group(&db, dst).await.unwrap() as i32;
    assert_eq!(base_index, 2);
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        i::move_all_instances_to_group_conn(&tx, src, dst, base_index)?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();

    // src is empty, dst holds all five with the moved ones after the originals
    assert!(
        i::get_instances_by_group(&db, src)
            .await
            .unwrap()
            .is_empty()
    );
    let layout = indexes_in_group(&db, dst).await;
    let moved: Vec<(i32, i32)> = layout
        .iter()
        .filter(|(id, _)| s.contains(id))
        .copied()
        .collect();
    assert_eq!(moved, vec![(s[0], 2), (s[1], 3), (s[2], 4)]);
}

#[tokio::test]
async fn delete_group_tx_moves_then_deletes() {
    let (_d, db) = migrated_db().await;
    let def = seed_group(&db, "default", 0).await;
    let grp = seed_group(&db, "grp", 1).await;
    seed_instance(&db, "d0", "d0", 0, def, None).await;
    let moved = seed_instance(&db, "g0", "g0", 0, grp, None).await;

    i::delete_group_tx(&db, grp, def).await.unwrap();

    assert!(i::get_group(&db, grp).await.unwrap().is_none());
    let row = i::get_instance(&db, moved).await.unwrap().unwrap();
    assert_eq!(row.group_id, def);
    // Appended after the destination index, with a valid root position.
    assert_eq!(row.index, 1);
}

#[tokio::test]
async fn create_default_group_tx_points_config_at_new_group() {
    let (_d, db) = migrated_db().await;
    // AppConfiguration singleton must exist for the UPDATE to hit a row
    ac::insert_app_configuration(&db, "stable".into(), 2048, None)
        .await
        .unwrap();

    let gid = i::create_default_group_tx(&db, 0).await.unwrap();
    let group = i::get_group(&db, gid).await.unwrap().unwrap();
    assert_eq!(group.name, "localize➽default");
    assert_eq!(group.library_position, None);

    let default: Option<i32> = db
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT defaultInstanceGroup FROM AppConfiguration WHERE id = 0",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(default, Some(gid));
}

#[tokio::test]
async fn move_instance_tx_runs_shifts_then_final_update() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0).await;
    let mut ids: Vec<i32> = Vec::new();
    for p in 0..3 {
        ids.push(seed_instance(&db, &format!("n{p}"), &format!("sp{p}"), p, g, None).await);
    }

    // Move the instance at index 0 forward to before index 2 (same group):
    // shift (0, 2) exclusive down (index 1 -> 0), final index = target - 1 = 1.
    let shifts = vec![i::IndexShift::DownExclusive {
        group_id: g,
        gt: 0,
        lt: 2,
    }];
    i::move_instance_tx(&db, shifts, ids[0], g, 1, None)
        .await
        .unwrap();

    assert_eq!(idx_of(&db, ids[1]).await, 0); // was 1 -> 0
    assert_eq!(idx_of(&db, ids[0]).await, 1); // moved to target - 1
    assert_eq!(idx_of(&db, ids[2]).await, 2); // unchanged
    assert_eq!(
        indexes_in_group(&db, g).await,
        vec![(ids[1], 0), (ids[0], 1), (ids[2], 2)]
    );
}

#[tokio::test]
async fn register_instance_tx_preserves_an_existing_shortpath_owner() {
    let (_dir, db) = migrated_db().await;
    let group = seed_group(&db, "group", 0).await;
    let original =
        i::register_instance_tx(&db, "original".into(), "reserved".into(), 0, group, Some(0))
            .await
            .unwrap() as i32;
    let before = i::get_instance(&db, original).await.unwrap().unwrap();
    let conflict = i::register_instance_tx(
        &db,
        "replacement".into(),
        "reserved".into(),
        1,
        group,
        Some(1),
    )
    .await;
    assert!(conflict.is_err());
    assert_eq!(
        i::get_instance(&db, original).await.unwrap().unwrap(),
        before
    );
    assert_eq!(i::get_all_instances(&db).await.unwrap().len(), 1);
}

#[tokio::test]
async fn task15_unlink_allocates_separate_sparse_root_coordinates() {
    let (_d, db) = migrated_db().await;
    let root = seed_group(&db, "root", 0).await;
    let folder = seed_group(&db, "folder", 1).await;
    i::set_group_library_position(&db, folder, Some(500))
        .await
        .unwrap();
    let existing = seed_instance(&db, "existing", "existing", 90, root, Some(-10)).await;
    let a = seed_instance(&db, "a", "a", -7, folder, None).await;
    let b = seed_instance(&db, "b", "b", -7, folder, None).await;
    let c = seed_instance(&db, "c", "c", 80, folder, None).await;
    i::delete_group_tx(&db, folder, root).await.unwrap();
    for (n, id) in [a, b, c].into_iter().enumerate() {
        let row = i::get_instance(&db, id).await.unwrap().unwrap();
        assert_eq!(
            (row.group_id, row.index, row.library_position),
            (root, 91 + n as i32, Some(501 + n as i32))
        );
    }
    let row = i::get_instance(&db, existing).await.unwrap().unwrap();
    assert_eq!((row.index, row.library_position), (90, Some(-10)));
}

#[tokio::test]
async fn task15_unlink_overflow_and_late_failure_roll_back() {
    for failure in ["overflow", "trigger", "self"] {
        let (_d, db) = migrated_db().await;
        let root = seed_group(&db, "root", 0).await;
        let folder = seed_group(&db, "folder", 1).await;
        seed_instance(
            &db,
            "existing",
            "existing",
            if failure == "overflow" { i32::MAX } else { 0 },
            root,
            Some(0),
        )
        .await;
        let a = seed_instance(&db, "a", "a", 0, folder, None).await;
        let b = seed_instance(&db, "b", "b", 1, folder, None).await;
        if failure == "trigger" {
            db.write(move |c| {c.raw().execute_batch("CREATE TRIGGER stop_unlink BEFORE DELETE ON InstanceGroup BEGIN SELECT RAISE(ABORT,'fixture'); END;")?; Ok(())}).await.unwrap();
        }
        assert!(
            i::delete_group_tx(&db, folder, if failure == "self" { folder } else { root })
                .await
                .is_err(),
            "{failure}"
        );
        assert!(i::get_group(&db, folder).await.unwrap().is_some());
        for id in [a, b] {
            let row = i::get_instance(&db, id).await.unwrap().unwrap();
            assert_eq!((row.group_id, row.library_position), (folder, None));
        }
    }
}

#[tokio::test]
async fn task15_concurrent_and_empty_unlinks_preserve_root_order_after_reopen() {
    let (dir, db) = migrated_db().await;
    db.write(|c| {
        c.raw().execute_batch("PRAGMA foreign_keys=ON")?;
        Ok(())
    })
    .await
    .unwrap();
    let root = seed_group(&db, "root", 0).await;
    let first = seed_group(&db, "first", 1).await;
    let second = seed_group(&db, "second", 2).await;
    let empty = seed_group(&db, "empty", 3).await;
    let original = seed_instance(&db, "existing", "existing", -5, root, Some(-3)).await;
    let a = seed_instance(&db, "a", "a", 99, first, None).await;
    let b = seed_instance(&db, "b", "b", -99, second, None).await;
    let (a_result, b_result) = tokio::join!(
        i::delete_group_tx(&db, first, root),
        i::delete_group_tx(&db, second, root)
    );
    a_result.unwrap();
    b_result.unwrap();
    i::delete_group_tx(&db, empty, root).await.unwrap();
    let conn = Connection::open(dir.path().join("t.db")).unwrap();
    let positions: std::collections::BTreeSet<(i32, i32)> = conn
        .prepare("SELECT \"index\",libraryPosition FROM Instance WHERE id IN (?1,?2)")
        .unwrap()
        .query_map([a, b], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        positions,
        std::collections::BTreeSet::from([(-4, -2), (-3, -1)])
    );
    let row = i::get_instance(&db, original).await.unwrap().unwrap();
    assert_eq!((row.index, row.library_position), (-5, Some(-3)));
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
