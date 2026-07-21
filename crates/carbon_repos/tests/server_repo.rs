use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::server::{self as s, IndexShift, ServerPatch};
use chrono::{TimeZone, Utc};
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

fn now() -> DbDateTime {
    DbDateTime(Utc::now().into())
}

fn seed_group(conn: &Connection, name: &str, group_index: i32, lib_pos: Option<i32>) -> i32 {
    s::insert_server_group(conn, name, group_index, lib_pos).unwrap() as i32
}

#[allow(clippy::too_many_arguments)]
fn seed_server(
    conn: &Connection,
    name: &str,
    shortpath: &str,
    index: i32,
    group_id: i32,
    lib_pos: Option<i32>,
) -> i32 {
    s::insert_server(
        conn, name, shortpath, index, group_id, "1.20.1", 25565, "vanilla", None, None, None, None,
        None, lib_pos, now(),
    )
    .unwrap() as i32
}

fn idx_of(conn: &Connection, id: i32) -> i32 {
    s::get_server(conn, id).unwrap().unwrap().index
}

fn indexes_in_group(conn: &Connection, group_id: i32) -> Vec<(i32, i32)> {
    let mut rows: Vec<(i32, i32)> = s::get_servers_by_group(conn, group_id)
        .unwrap()
        .into_iter()
        .map(|r| (r.id, r.index))
        .collect();
    rows.sort_by_key(|(_, idx)| *idx);
    rows
}

#[test]
fn insert_server_takes_ddl_defaults_and_millis_date() {
    let (_d, conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    // A concrete millis value so the round-trip is exact.
    let dt = DbDateTime(Utc.timestamp_millis_opt(1_784_557_692_104).unwrap().into());
    let id = s::insert_server(
        &conn, "srv", "sp", 3, g, "1.20.1", 25566, "modded", Some("forge"), Some("47.1.0"), None,
        None, None, Some(7), dt,
    )
    .unwrap() as i32;

    let row = s::get_server(&conn, id).unwrap().unwrap();
    assert_eq!(row.name, "srv");
    assert_eq!(row.shortpath, "sp");
    assert_eq!(row.index, 3);
    assert_eq!(row.group_id, g);
    assert_eq!(row.port, 25566);
    assert_eq!(row.server_type, "modded");
    assert_eq!(row.modloader_type.as_deref(), Some("forge"));
    assert_eq!(row.modloader_version.as_deref(), Some("47.1.0"));
    assert_eq!(row.library_position, Some(7));
    assert_eq!(row.date_created, dt.0);
    // DDL defaults
    assert!(!row.favorite);
    assert_eq!(row.motd, "A Minecraft Server");
    assert_eq!(row.max_players, 20);
    assert!(row.online_mode);
    assert_eq!(row.xmx, 2048);
    assert_eq!(row.xms, 1024);
    assert_eq!(row.extra_java_args, "");
    assert!(!row.auto_restart);
    assert_eq!(row.provider_type, "local");
    assert_eq!(row.last_started, None);
    assert_eq!(row.icon_revision, None);

    // Stored as INTEGER millis, not TEXT (DbDateTime rejects TEXT on read).
    let (typ, raw): (String, i64) = conn
        .query_row(
            "SELECT typeof(dateCreated), dateCreated FROM Server WHERE id = ?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(typ, "integer");
    assert_eq!(raw, 1_784_557_692_104);
}

#[test]
fn last_started_millis_round_trip() {
    let (_d, conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    let id = seed_server(&conn, "s", "sp", 0, g, None);
    let ts = DbDateTime(Utc.timestamp_millis_opt(1_700_000_000_000).unwrap().into());
    let n = s::set_server_last_started(&conn, id, Some(ts)).unwrap();
    assert_eq!(n, 1);
    let row = s::get_server(&conn, id).unwrap().unwrap();
    assert_eq!(row.last_started, Some(ts.0));
}

#[test]
fn shift_library_positions_scoped_to_group() {
    // The server-side group-move shift is scoped to the default group, unlike
    // the instance side which shifts across all rows.
    let (_d, mut conn) = migrated_db();
    let def = seed_group(&conn, "def", 0, None);
    let other = seed_group(&conn, "other", 1, Some(9));
    // default-group servers at library positions 0..4
    let ids: Vec<i32> = (0..5)
        .map(|p| seed_server(&conn, &format!("n{p}"), &format!("sp{p}"), p, def, Some(p)))
        .collect();
    // a server in `other` with a lib pos in the shifted range must NOT move
    let other_srv = seed_server(&conn, "o", "spo", 0, other, Some(2));

    let affected = conn
        .transaction()
        .map(|tx| {
            let n = s::shift_server_library_positions_down_scoped(&tx, def, 1, 3).unwrap();
            tx.commit().unwrap();
            n
        })
        .unwrap();
    assert_eq!(affected, 2);

    let pos = |id: i32| s::get_server(&conn, id).unwrap().unwrap().library_position;
    assert_eq!(pos(ids[0]), Some(0));
    assert_eq!(pos(ids[1]), Some(1)); // unchanged (not > 1)
    assert_eq!(pos(ids[2]), Some(1)); // 2 -> 1
    assert_eq!(pos(ids[3]), Some(2)); // 3 -> 2
    assert_eq!(pos(ids[4]), Some(4)); // unchanged (not <= 3)
    assert_eq!(pos(other_srv), Some(2)); // other group untouched
}

#[test]
fn shift_indexes_up_from_and_down_after() {
    let (_d, mut conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    let ids: Vec<i32> = (0..4)
        .map(|p| seed_server(&conn, &format!("n{p}"), &format!("sp{p}"), p, g, None))
        .collect();

    conn.transaction()
        .map(|tx| {
            s::shift_server_indexes_up_from(&tx, g, 2).unwrap();
            tx.commit().unwrap();
        })
        .unwrap();
    assert_eq!(idx_of(&conn, ids[0]), 0);
    assert_eq!(idx_of(&conn, ids[1]), 1);
    assert_eq!(idx_of(&conn, ids[2]), 3);
    assert_eq!(idx_of(&conn, ids[3]), 4);

    conn.transaction()
        .map(|tx| {
            s::shift_server_indexes_down_after(&tx, g, 3).unwrap();
            tx.commit().unwrap();
        })
        .unwrap();
    assert_eq!(idx_of(&conn, ids[3]), 3);
}

#[test]
fn move_all_servers_to_group_preserves_relative_order() {
    let (_d, mut conn) = migrated_db();
    let src = seed_group(&conn, "src", 0, None);
    let dst = seed_group(&conn, "dst", 1, None);
    seed_server(&conn, "d0", "d0", 0, dst, None);
    seed_server(&conn, "d1", "d1", 1, dst, None);
    let moved: Vec<i32> = (0..3)
        .map(|p| seed_server(&conn, &format!("s{p}"), &format!("s{p}"), p, src, None))
        .collect();

    let base_index = s::count_servers_in_group(&conn, dst).unwrap() as i32;
    assert_eq!(base_index, 2);
    conn.transaction()
        .map(|tx| {
            s::move_all_servers_to_group(&tx, src, dst, base_index).unwrap();
            tx.commit().unwrap();
        })
        .unwrap();

    assert!(s::get_servers_by_group(&conn, src).unwrap().is_empty());
    let layout = indexes_in_group(&conn, dst);
    let m: Vec<(i32, i32)> = layout.iter().filter(|(id, _)| moved.contains(id)).copied().collect();
    assert_eq!(m, vec![(moved[0], 2), (moved[1], 3), (moved[2], 4)]);
}

#[test]
fn delete_server_group_tx_uses_default_group_base_index() {
    // Server-side oddity: base_index counts the DEFAULT group, not the group
    // being deleted.
    let (_d, conn) = migrated_db();
    let def = seed_group(&conn, "default", 0, None);
    let grp = seed_group(&conn, "grp", 1, Some(0));
    // default group has 2 servers -> base_index should be 2
    seed_server(&conn, "d0", "d0", 0, def, None);
    seed_server(&conn, "d1", "d1", 1, def, None);
    let moved = seed_server(&conn, "g0", "g0", 0, grp, None);

    let base_index = s::count_servers_in_group(&conn, def).unwrap() as i32;
    assert_eq!(base_index, 2);
    let mut conn = conn;
    s::delete_server_group_tx(&mut conn, grp, def, base_index).unwrap();

    assert!(s::get_server_group(&conn, grp).unwrap().is_none());
    let row = s::get_server(&conn, moved).unwrap().unwrap();
    assert_eq!(row.group_id, def);
    assert_eq!(row.index, 2); // 0 + base_index(2)
}

#[test]
fn move_server_tx_runs_shifts_then_final_update() {
    let (_d, conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    let ids: Vec<i32> = (0..3)
        .map(|p| seed_server(&conn, &format!("n{p}"), &format!("sp{p}"), p, g, None))
        .collect();

    let shifts = [IndexShift::DownExclusive { group_id: g, gt: 0, lt: 2 }];
    let mut conn = conn;
    s::move_server_tx(&mut conn, &shifts, ids[0], g, 1, None).unwrap();

    assert_eq!(idx_of(&conn, ids[1]), 0);
    assert_eq!(idx_of(&conn, ids[0]), 1);
    assert_eq!(idx_of(&conn, ids[2]), 2);
    assert_eq!(indexes_in_group(&conn, g), vec![(ids[1], 0), (ids[0], 1), (ids[2], 2)]);
}

#[test]
fn arrange_server_library_tx_stamps_groups_and_servers() {
    let (_d, conn) = migrated_db();
    let def = seed_group(&conn, "default", 5, None);
    let folder = seed_group(&conn, "folder", 9, None);
    let s0 = seed_server(&conn, "b", "b", 0, def, Some(0));
    let s1 = seed_server(&conn, "a", "a", 1, def, Some(1));

    let groups = [
        s::ServerGroupArrange { id: def, group_index: 0, library_position: None, set_library_position: false },
        s::ServerGroupArrange { id: folder, group_index: 1, library_position: Some(0), set_library_position: true },
    ];
    let servers = [
        s::ServerArrange { id: s1, index: 1, library_position: Some(1) },
        s::ServerArrange { id: s0, index: 2, library_position: Some(2) },
    ];
    let mut conn = conn;
    s::arrange_server_library_tx(&mut conn, &groups, &servers).unwrap();

    let dg = s::get_server_group(&conn, def).unwrap().unwrap();
    assert_eq!(dg.group_index, 0);
    assert_eq!(dg.library_position, None); // default group keeps null lib pos
    let fg = s::get_server_group(&conn, folder).unwrap().unwrap();
    assert_eq!(fg.group_index, 1);
    assert_eq!(fg.library_position, Some(0));
    assert_eq!(s::get_server(&conn, s1).unwrap().unwrap().index, 1);
    assert_eq!(s::get_server(&conn, s0).unwrap().unwrap().index, 2);
    assert_eq!(s::get_server(&conn, s0).unwrap().unwrap().library_position, Some(2));
}

#[test]
fn server_patch_empty_is_none() {
    assert!(ServerPatch::default().build(1).is_none());
}

#[test]
fn server_patch_settings_subset_updates_only_present_fields() {
    let (_d, conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    let id = seed_server(&conn, "orig", "sp", 0, g, None);

    let patch = ServerPatch {
        name: Some("renamed".into()),
        xmx: Some(4096),
        extra_java_args: Some("-Dx=1".into()),
        auto_restart: Some(true),
        ..Default::default()
    };
    let q = patch.build(id).unwrap();
    let n = q.execute(&conn).unwrap();
    assert_eq!(n, 1);

    let row = s::get_server(&conn, id).unwrap().unwrap();
    assert_eq!(row.name, "renamed");
    assert_eq!(row.xmx, 4096);
    assert_eq!(row.extra_java_args, "-Dx=1");
    assert!(row.auto_restart);
    // untouched columns keep their original values
    assert_eq!(row.xms, 1024);
    assert_eq!(row.port, 25565);
    assert_eq!(row.motd, "A Minecraft Server");
}

#[test]
fn server_patch_properties_subset_updates_only_present_fields() {
    let (_d, conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    let id = seed_server(&conn, "orig", "sp", 0, g, None);

    let patch = ServerPatch {
        port: Some(25599),
        motd: Some("hello".into()),
        max_players: Some(50),
        online_mode: Some(false),
        ..Default::default()
    };
    let q = patch.build(id).unwrap();
    assert_eq!(q.execute(&conn).unwrap(), 1);

    let row = s::get_server(&conn, id).unwrap().unwrap();
    assert_eq!(row.port, 25599);
    assert_eq!(row.motd, "hello");
    assert_eq!(row.max_players, 50);
    assert!(!row.online_mode);
    assert_eq!(row.name, "orig"); // untouched
}

#[test]
fn set_game_version_and_modloader_writes_nullables() {
    let (_d, conn) = migrated_db();
    let g = seed_group(&conn, "g", 0, None);
    let id = seed_server(&conn, "s", "sp", 0, g, None);
    let n = s::set_server_game_version_and_modloader(&conn, id, "1.21", Some("fabric"), Some("0.15"))
        .unwrap();
    assert_eq!(n, 1);
    let row = s::get_server(&conn, id).unwrap().unwrap();
    assert_eq!(row.game_version, "1.21");
    assert_eq!(row.modloader_type.as_deref(), Some("fabric"));
    assert_eq!(row.modloader_version.as_deref(), Some("0.15"));
    // clearing back to NULL
    s::set_server_game_version_and_modloader(&conn, id, "1.21", None, None).unwrap();
    let row = s::get_server(&conn, id).unwrap().unwrap();
    assert_eq!(row.modloader_type, None);
    assert_eq!(row.modloader_version, None);
}

#[test]
fn group_by_name_and_default_first_ordering() {
    let (_d, conn) = migrated_db();
    // migration seeds a 'Default' group at groupIndex 0
    let first = s::first_server_group_ordered_by_group_index(&conn).unwrap().unwrap();
    assert_eq!(first.name, "Default");
    let by_name = s::find_server_group_by_name(&conn, "Default").unwrap();
    assert!(by_name.is_some());
    assert!(s::find_server_group_by_name(&conn, "nope").unwrap().is_none());
}
