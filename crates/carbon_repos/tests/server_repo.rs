use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::server::{self as s, IndexShift, ServerPatch};
use chrono::{TimeZone, Utc};
use rusqlite::Connection;

/// Migrates a fresh tempfile, then opens the async `Db` pool over it. The server
/// repo's transaction fns are write-pool wrappers, so the suite drives the real
/// pool and its read/write wrappers.
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

fn now() -> DbDateTime {
    DbDateTime(Utc::now().into())
}

async fn seed_group(db: &Db, name: &str, group_index: i32, lib_pos: Option<i32>) -> i32 {
    s::insert_server_group(db, name.to_owned(), group_index, lib_pos)
        .await
        .unwrap() as i32
}

async fn seed_server(
    db: &Db,
    name: &str,
    shortpath: &str,
    index: i32,
    group_id: i32,
    lib_pos: Option<i32>,
) -> i32 {
    s::insert_server(
        db,
        name.to_owned(),
        shortpath.to_owned(),
        index,
        group_id,
        "1.20.1".to_owned(),
        25565,
        "vanilla".to_owned(),
        None,
        None,
        None,
        None,
        None,
        lib_pos,
        now(),
    )
    .await
    .unwrap() as i32
}

async fn idx_of(db: &Db, id: i32) -> i32 {
    s::get_server(db, id).await.unwrap().unwrap().index
}

async fn indexes_in_group(db: &Db, group_id: i32) -> Vec<(i32, i32)> {
    let mut rows: Vec<(i32, i32)> = s::get_servers_by_group(db, group_id)
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.id, r.index))
        .collect();
    rows.sort_by_key(|(_, idx)| *idx);
    rows
}

#[tokio::test]
async fn insert_server_takes_ddl_defaults_and_millis_date() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    // A concrete millis value so the round-trip is exact.
    let dt = DbDateTime(Utc.timestamp_millis_opt(1_784_557_692_104).unwrap().into());
    let id = s::insert_server(
        &db,
        "srv".into(),
        "sp".into(),
        3,
        g,
        "1.20.1".into(),
        25566,
        "modded".into(),
        Some("forge".into()),
        Some("47.1.0".into()),
        None,
        None,
        None,
        Some(7),
        dt,
    )
    .await
    .unwrap() as i32;

    let row = s::get_server(&db, id).await.unwrap().unwrap();
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
    let (typ, raw): (String, i64) = db
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT typeof(dateCreated), dateCreated FROM Server WHERE id = ?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(typ, "integer");
    assert_eq!(raw, 1_784_557_692_104);
}

#[tokio::test]
async fn last_started_millis_round_trip() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    let id = seed_server(&db, "s", "sp", 0, g, None).await;
    let ts = DbDateTime(Utc.timestamp_millis_opt(1_700_000_000_000).unwrap().into());
    let n = s::set_server_last_started(&db, id, Some(ts)).await.unwrap();
    assert_eq!(n, 1);
    let row = s::get_server(&db, id).await.unwrap().unwrap();
    assert_eq!(row.last_started, Some(ts.0));
}

#[tokio::test]
async fn shift_library_positions_scoped_to_group() {
    // The server-side group-move shift is scoped to the default group, unlike
    // the instance side which shifts across all rows.
    let (_d, db) = migrated_db().await;
    let def = seed_group(&db, "def", 0, None).await;
    let other = seed_group(&db, "other", 1, Some(9)).await;
    // default-group servers at library positions 0..4
    let mut ids: Vec<i32> = Vec::new();
    for p in 0..5 {
        ids.push(seed_server(&db, &format!("n{p}"), &format!("sp{p}"), p, def, Some(p)).await);
    }
    // a server in `other` with a lib pos in the shifted range must NOT move
    let other_srv = seed_server(&db, "o", "spo", 0, other, Some(2)).await;

    let affected = db
        .write(move |conn| {
            let tx = conn.transaction()?;
            let n = s::shift_server_library_positions_down_scoped_conn(&tx, def, 1, 3)?;
            tx.commit()?;
            Ok(n)
        })
        .await
        .unwrap();
    assert_eq!(affected, 2);

    let mut pos = Vec::new();
    for &id in &ids {
        pos.push(s::get_server(&db, id).await.unwrap().unwrap().library_position);
    }
    assert_eq!(pos[0], Some(0));
    assert_eq!(pos[1], Some(1)); // unchanged (not > 1)
    assert_eq!(pos[2], Some(1)); // 2 -> 1
    assert_eq!(pos[3], Some(2)); // 3 -> 2
    assert_eq!(pos[4], Some(4)); // unchanged (not <= 3)
    assert_eq!(
        s::get_server(&db, other_srv).await.unwrap().unwrap().library_position,
        Some(2)
    ); // other group untouched
}

#[tokio::test]
async fn shift_indexes_up_from_and_down_after() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    let mut ids: Vec<i32> = Vec::new();
    for p in 0..4 {
        ids.push(seed_server(&db, &format!("n{p}"), &format!("sp{p}"), p, g, None).await);
    }

    db.write(move |conn| {
        let tx = conn.transaction()?;
        s::shift_server_indexes_up_from_conn(&tx, g, 2)?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(idx_of(&db, ids[0]).await, 0);
    assert_eq!(idx_of(&db, ids[1]).await, 1);
    assert_eq!(idx_of(&db, ids[2]).await, 3);
    assert_eq!(idx_of(&db, ids[3]).await, 4);

    db.write(move |conn| {
        let tx = conn.transaction()?;
        s::shift_server_indexes_down_after_conn(&tx, g, 3)?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(idx_of(&db, ids[3]).await, 3);
}

#[tokio::test]
async fn move_all_servers_to_group_preserves_relative_order() {
    let (_d, db) = migrated_db().await;
    let src = seed_group(&db, "src", 0, None).await;
    let dst = seed_group(&db, "dst", 1, None).await;
    seed_server(&db, "d0", "d0", 0, dst, None).await;
    seed_server(&db, "d1", "d1", 1, dst, None).await;
    let mut moved: Vec<i32> = Vec::new();
    for p in 0..3 {
        moved.push(seed_server(&db, &format!("s{p}"), &format!("s{p}"), p, src, None).await);
    }

    let base_index = s::count_servers_in_group(&db, dst).await.unwrap() as i32;
    assert_eq!(base_index, 2);
    db.write(move |conn| {
        let tx = conn.transaction()?;
        s::move_all_servers_to_group_conn(&tx, src, dst, base_index)?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();

    assert!(s::get_servers_by_group(&db, src).await.unwrap().is_empty());
    let layout = indexes_in_group(&db, dst).await;
    let m: Vec<(i32, i32)> = layout.iter().filter(|(id, _)| moved.contains(id)).copied().collect();
    assert_eq!(m, vec![(moved[0], 2), (moved[1], 3), (moved[2], 4)]);
}

#[tokio::test]
async fn delete_server_group_tx_uses_default_group_base_index() {
    // Server-side oddity: base_index counts the DEFAULT group, not the group
    // being deleted.
    let (_d, db) = migrated_db().await;
    let def = seed_group(&db, "default", 0, None).await;
    let grp = seed_group(&db, "grp", 1, Some(0)).await;
    // default group has 2 servers -> base_index should be 2
    seed_server(&db, "d0", "d0", 0, def, None).await;
    seed_server(&db, "d1", "d1", 1, def, None).await;
    let moved = seed_server(&db, "g0", "g0", 0, grp, None).await;

    let base_index = s::count_servers_in_group(&db, def).await.unwrap() as i32;
    assert_eq!(base_index, 2);
    s::delete_server_group_tx(&db, grp, def, base_index).await.unwrap();

    assert!(s::get_server_group(&db, grp).await.unwrap().is_none());
    let row = s::get_server(&db, moved).await.unwrap().unwrap();
    assert_eq!(row.group_id, def);
    assert_eq!(row.index, 2); // 0 + base_index(2)
}

#[tokio::test]
async fn move_server_tx_runs_shifts_then_final_update() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    let mut ids: Vec<i32> = Vec::new();
    for p in 0..3 {
        ids.push(seed_server(&db, &format!("n{p}"), &format!("sp{p}"), p, g, None).await);
    }

    let shifts = vec![IndexShift::DownExclusive { group_id: g, gt: 0, lt: 2 }];
    s::move_server_tx(&db, shifts, ids[0], g, 1, None).await.unwrap();

    assert_eq!(idx_of(&db, ids[1]).await, 0);
    assert_eq!(idx_of(&db, ids[0]).await, 1);
    assert_eq!(idx_of(&db, ids[2]).await, 2);
    assert_eq!(indexes_in_group(&db, g).await, vec![(ids[1], 0), (ids[0], 1), (ids[2], 2)]);
}

#[tokio::test]
async fn arrange_server_library_tx_stamps_groups_and_servers() {
    let (_d, db) = migrated_db().await;
    let def = seed_group(&db, "default", 5, None).await;
    let folder = seed_group(&db, "folder", 9, None).await;
    let s0 = seed_server(&db, "b", "b", 0, def, Some(0)).await;
    let s1 = seed_server(&db, "a", "a", 1, def, Some(1)).await;

    let groups = vec![
        s::ServerGroupArrange { id: def, group_index: 0, library_position: None, set_library_position: false },
        s::ServerGroupArrange { id: folder, group_index: 1, library_position: Some(0), set_library_position: true },
    ];
    let servers = vec![
        s::ServerArrange { id: s1, index: 1, library_position: Some(1) },
        s::ServerArrange { id: s0, index: 2, library_position: Some(2) },
    ];
    s::arrange_server_library_tx(&db, groups, servers).await.unwrap();

    let dg = s::get_server_group(&db, def).await.unwrap().unwrap();
    assert_eq!(dg.group_index, 0);
    assert_eq!(dg.library_position, None); // default group keeps null lib pos
    let fg = s::get_server_group(&db, folder).await.unwrap().unwrap();
    assert_eq!(fg.group_index, 1);
    assert_eq!(fg.library_position, Some(0));
    assert_eq!(s::get_server(&db, s1).await.unwrap().unwrap().index, 1);
    assert_eq!(s::get_server(&db, s0).await.unwrap().unwrap().index, 2);
    assert_eq!(s::get_server(&db, s0).await.unwrap().unwrap().library_position, Some(2));
}

#[test]
fn server_patch_empty_is_none() {
    assert!(ServerPatch::default().build(1).is_none());
}

#[tokio::test]
async fn server_patch_settings_subset_updates_only_present_fields() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    let id = seed_server(&db, "orig", "sp", 0, g, None).await;

    let patch = ServerPatch {
        name: Some("renamed".into()),
        xmx: Some(4096),
        extra_java_args: Some("-Dx=1".into()),
        auto_restart: Some(true),
        ..Default::default()
    };
    let q = patch.build(id).unwrap();
    let n = db.write(move |conn| Ok(q.execute(conn)?)).await.unwrap();
    assert_eq!(n, 1);

    let row = s::get_server(&db, id).await.unwrap().unwrap();
    assert_eq!(row.name, "renamed");
    assert_eq!(row.xmx, 4096);
    assert_eq!(row.extra_java_args, "-Dx=1");
    assert!(row.auto_restart);
    // untouched columns keep their original values
    assert_eq!(row.xms, 1024);
    assert_eq!(row.port, 25565);
    assert_eq!(row.motd, "A Minecraft Server");
}

#[tokio::test]
async fn server_patch_properties_subset_updates_only_present_fields() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    let id = seed_server(&db, "orig", "sp", 0, g, None).await;

    let patch = ServerPatch {
        port: Some(25599),
        motd: Some("hello".into()),
        max_players: Some(50),
        online_mode: Some(false),
        ..Default::default()
    };
    let q = patch.build(id).unwrap();
    assert_eq!(db.write(move |conn| Ok(q.execute(conn)?)).await.unwrap(), 1);

    let row = s::get_server(&db, id).await.unwrap().unwrap();
    assert_eq!(row.port, 25599);
    assert_eq!(row.motd, "hello");
    assert_eq!(row.max_players, 50);
    assert!(!row.online_mode);
    assert_eq!(row.name, "orig"); // untouched
}

#[tokio::test]
async fn set_game_version_and_modloader_writes_nullables() {
    let (_d, db) = migrated_db().await;
    let g = seed_group(&db, "g", 0, None).await;
    let id = seed_server(&db, "s", "sp", 0, g, None).await;
    let n = s::set_server_game_version_and_modloader(&db, id, "1.21", Some("fabric"), Some("0.15"))
        .await
        .unwrap();
    assert_eq!(n, 1);
    let row = s::get_server(&db, id).await.unwrap().unwrap();
    assert_eq!(row.game_version, "1.21");
    assert_eq!(row.modloader_type.as_deref(), Some("fabric"));
    assert_eq!(row.modloader_version.as_deref(), Some("0.15"));
    // clearing back to NULL
    s::set_server_game_version_and_modloader(&db, id, "1.21", None, None).await.unwrap();
    let row = s::get_server(&db, id).await.unwrap().unwrap();
    assert_eq!(row.modloader_type, None);
    assert_eq!(row.modloader_version, None);
}

#[tokio::test]
async fn group_by_name_and_default_first_ordering() {
    let (_d, db) = migrated_db().await;
    // migration seeds a 'Default' group at groupIndex 0
    let first = s::first_server_group_ordered_by_group_index(&db).await.unwrap().unwrap();
    assert_eq!(first.name, "Default");
    let by_name = s::find_server_group_by_name(&db, "Default").await.unwrap();
    assert!(by_name.is_some());
    assert!(s::find_server_group_by_name(&db, "nope").await.unwrap().is_none());
}
