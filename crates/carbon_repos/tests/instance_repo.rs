use carbon_repos::repos::app_configuration as ac;
use carbon_repos::repos::instance as i;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

/// Seed a group and `n` instances in it with sequential indexes `0..n` and,
/// optionally, library positions equal to the index. Returns the group id.
fn seed_group(conn: &Connection, name: &str, group_index: i32) -> i32 {
    i::insert_group(conn, name, group_index, None).unwrap() as i32
}

fn seed_instance(
    conn: &mut Connection,
    name: &str,
    shortpath: &str,
    index: i32,
    group_id: i32,
    lib_pos: Option<i32>,
) -> i32 {
    i::add_instance_tx(conn, name, shortpath, index, group_id, lib_pos).unwrap() as i32
}

fn idx_of(conn: &Connection, id: i32) -> i32 {
    i::get_instance(conn, id).unwrap().unwrap().index
}

fn indexes_in_group(conn: &Connection, group_id: i32) -> Vec<(i32, i32)> {
    // (id, index) ordered by index asc
    i::get_instances_by_group_ordered_by_index(conn, group_id)
        .unwrap()
        .into_iter()
        .map(|r| (r.id, r.index))
        .collect()
}

#[test]
fn insert_instance_takes_ddl_defaults() {
    let (_d, mut conn) = migrated_db();
    let g = seed_group(&conn, "g", 0);
    let id = seed_instance(&mut conn, "n", "sp", 3, g, Some(7));
    let row = i::get_instance(&conn, id).unwrap().unwrap();
    assert_eq!(row.name, "n");
    assert_eq!(row.shortpath, "sp");
    assert_eq!(row.index, 3);
    assert_eq!(row.group_id, g);
    assert_eq!(row.library_position, Some(7));
    // favorite / hasPackUpdate default to false
    assert!(!row.favorite);
    assert!(!row.has_pack_update);
}

#[test]
fn add_instance_tx_replaces_same_shortpath() {
    let (_d, mut conn) = migrated_db();
    let g = seed_group(&conn, "g", 0);
    let first = seed_instance(&mut conn, "a", "dup", 0, g, None);
    let second = seed_instance(&mut conn, "b", "dup", 1, g, None);
    assert_ne!(first, second);
    // only the second row survives at that shortpath
    assert!(i::get_instance(&conn, first).unwrap().is_none());
    let row = i::get_instance_by_shortpath(&conn, "dup").unwrap().unwrap();
    assert_eq!(row.id, second);
    assert_eq!(row.name, "b");
}

#[test]
fn shift_library_positions_down_exact_layout() {
    let (_d, mut conn) = migrated_db();
    let g = seed_group(&conn, "g", 0);
    // library positions 0..5
    let ids: Vec<i32> = (0..5)
        .map(|p| seed_instance(&mut conn, &format!("n{p}"), &format!("sp{p}"), p, g, Some(p)))
        .collect();

    // shift (1, 3] down by one -> positions 2 and 3 become 1 and 2
    let affected = conn
        .transaction()
        .map(|tx| {
            let n = i::shift_instance_library_positions_down(&tx, 1, 3).unwrap();
            tx.commit().unwrap();
            n
        })
        .unwrap();
    assert_eq!(affected, 2);

    let pos = |idx: usize| {
        i::get_instance(&conn, ids[idx])
            .unwrap()
            .unwrap()
            .library_position
    };
    assert_eq!(pos(0), Some(0));
    assert_eq!(pos(1), Some(1)); // unchanged (not > 1)
    assert_eq!(pos(2), Some(1)); // 2 -> 1
    assert_eq!(pos(3), Some(2)); // 3 -> 2
    assert_eq!(pos(4), Some(4)); // unchanged (not <= 3)
}

#[test]
fn shift_indexes_up_from_and_down_after() {
    let (_d, mut conn) = migrated_db();
    let g = seed_group(&conn, "g", 0);
    let ids: Vec<i32> = (0..4)
        .map(|p| seed_instance(&mut conn, &format!("n{p}"), &format!("sp{p}"), p, g, None))
        .collect();

    // increment index for all with index >= 2
    conn.transaction()
        .map(|tx| {
            i::shift_instance_indexes_up_from(&tx, g, 2).unwrap();
            tx.commit().unwrap();
        })
        .unwrap();
    assert_eq!(idx_of(&conn, ids[0]), 0);
    assert_eq!(idx_of(&conn, ids[1]), 1);
    assert_eq!(idx_of(&conn, ids[2]), 3);
    assert_eq!(idx_of(&conn, ids[3]), 4);

    // decrement index for all with index > 3
    conn.transaction()
        .map(|tx| {
            i::shift_instance_indexes_down_after(&tx, g, 3).unwrap();
            tx.commit().unwrap();
        })
        .unwrap();
    assert_eq!(idx_of(&conn, ids[3]), 3);
    assert_eq!(idx_of(&conn, ids[2]), 3); // was 3, not > 3, unchanged
}

#[test]
fn move_all_instances_to_group_preserves_relative_order() {
    let (_d, mut conn) = migrated_db();
    let src = seed_group(&conn, "src", 0);
    let dst = seed_group(&conn, "dst", 1);
    // dst already has two instances at index 0,1
    seed_instance(&mut conn, "d0", "d0", 0, dst, None);
    seed_instance(&mut conn, "d1", "d1", 1, dst, None);
    // src has three at 0,1,2
    let s: Vec<i32> = (0..3)
        .map(|p| seed_instance(&mut conn, &format!("s{p}"), &format!("s{p}"), p, src, None))
        .collect();

    let base_index = i::count_instances_in_group(&conn, dst).unwrap() as i32;
    assert_eq!(base_index, 2);
    conn.transaction()
        .map(|tx| {
            i::move_all_instances_to_group(&tx, src, dst, base_index).unwrap();
            tx.commit().unwrap();
        })
        .unwrap();

    // src is empty, dst holds all five with the moved ones after the originals
    assert!(i::get_instances_by_group(&conn, src).unwrap().is_empty());
    let layout = indexes_in_group(&conn, dst);
    let moved: Vec<(i32, i32)> = layout.iter().filter(|(id, _)| s.contains(id)).copied().collect();
    assert_eq!(moved, vec![(s[0], 2), (s[1], 3), (s[2], 4)]);
}

#[test]
fn delete_group_tx_moves_then_deletes() {
    let (_d, mut conn) = migrated_db();
    let def = seed_group(&conn, "default", 0);
    let grp = seed_group(&conn, "grp", 1);
    seed_instance(&mut conn, "d0", "d0", 0, def, None);
    let moved = seed_instance(&mut conn, "g0", "g0", 0, grp, None);

    // base_index computed the instance-side way: count of the group BEING deleted
    let base_index = i::count_instances_in_group(&conn, grp).unwrap() as i32;
    assert_eq!(base_index, 1);
    i::delete_group_tx(&mut conn, grp, def, base_index).unwrap();

    assert!(i::get_group(&conn, grp).unwrap().is_none());
    let row = i::get_instance(&conn, moved).unwrap().unwrap();
    assert_eq!(row.group_id, def);
    // index shifted by base_index (1) from its original 0
    assert_eq!(row.index, 1);
}

#[test]
fn create_default_group_tx_points_config_at_new_group() {
    let (_d, mut conn) = migrated_db();
    // AppConfiguration singleton must exist for the UPDATE to hit a row
    ac::insert_app_configuration(&conn, "stable", 2048, None).unwrap();

    let gid = i::create_default_group_tx(&mut conn, 0).unwrap();
    let group = i::get_group(&conn, gid).unwrap().unwrap();
    assert_eq!(group.name, "localize➽default");
    assert_eq!(group.library_position, None);

    let default: Option<i32> = conn
        .query_row(
            "SELECT defaultInstanceGroup FROM AppConfiguration WHERE id = 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(default, Some(gid));
}

#[test]
fn move_instance_tx_runs_shifts_then_final_update() {
    let (_d, mut conn) = migrated_db();
    let g = seed_group(&conn, "g", 0);
    let ids: Vec<i32> = (0..3)
        .map(|p| seed_instance(&mut conn, &format!("n{p}"), &format!("sp{p}"), p, g, None))
        .collect();

    // Move the instance at index 0 forward to before index 2 (same group):
    // shift (0, 2) exclusive down (index 1 -> 0), final index = target - 1 = 1.
    let shifts = [i::IndexShift::DownExclusive {
        group_id: g,
        gt: 0,
        lt: 2,
    }];
    i::move_instance_tx(&mut conn, &shifts, ids[0], g, 1, None).unwrap();

    assert_eq!(idx_of(&conn, ids[1]), 0); // was 1 -> 0
    assert_eq!(idx_of(&conn, ids[0]), 1); // moved to target - 1
    assert_eq!(idx_of(&conn, ids[2]), 2); // unchanged
    assert_eq!(indexes_in_group(&conn, g), vec![(ids[1], 0), (ids[0], 1), (ids[2], 2)]);
}
