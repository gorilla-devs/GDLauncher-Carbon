//! Servers survive a lost database: each server's `server.json` follows its
//! row, a fresh database restores every server from it, and a delete removes
//! it first so a half-deleted folder never comes back.

use super::record::{ParsedRecord, RECORD_FILE_NAME, parse_record, record_path};
use super::*;
use carbon_repos::repos::server::ServerRow;
use std::path::{Path, PathBuf};

async fn insert(
    app: &crate::managers::App,
    shortpath: &str,
    modpack: Option<(&str, &str, &str)>,
) -> (ServerId, PathBuf) {
    let group = app.server_manager().get_default_group().await.unwrap();
    let (server_type, loader, loader_version) = match modpack {
        Some(_) => ("modded", Some("fabric"), Some("0.15.11")),
        None => ("vanilla", None, None),
    };
    let id = server_repo::insert_server(
        &app.db,
        format!("{shortpath} name"),
        shortpath.to_owned(),
        0,
        group.0,
        "1.20.1".into(),
        25565,
        server_type.into(),
        loader.map(str::to_owned),
        loader_version.map(str::to_owned),
        modpack.map(|(platform, _, _)| platform.to_owned()),
        modpack.map(|(_, project, _)| project.to_owned()),
        modpack.map(|(_, _, file)| file.to_owned()),
        Some(0),
        DbDateTime(Utc::now().into()),
    )
    .await
    .unwrap();
    let root = app
        .settings_manager()
        .runtime_path
        .get_servers()
        .get_server_path(shortpath)
        .get_root();
    tokio::fs::create_dir_all(root.join("server"))
        .await
        .unwrap();
    (ServerId(id as i32), root)
}

fn servers_dir(app: &crate::managers::App) -> PathBuf {
    app.settings_manager().runtime_path.get_servers().to_path()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// What a restore must bring back: every stored setting and the folder name,
/// without the library's own bookkeeping.
fn settings(row: &ServerRow, groups: &HashMap<i32, String>) -> (ServerRow, Option<String>) {
    (
        ServerRow {
            id: 0,
            index: 0,
            library_position: None,
            group_id: 0,
            last_started: None,
            icon_revision: None,
            ..row.clone()
        },
        groups.get(&row.group_id).cloned(),
    )
}

async fn rows_by_shortpath(
    app: &crate::managers::App,
) -> HashMap<String, (ServerRow, Option<String>)> {
    let default_group = app.server_manager().get_default_group().await.unwrap();
    let groups: HashMap<i32, String> = server_repo::get_all_server_groups(&app.db)
        .await
        .unwrap()
        .into_iter()
        .filter(|group| group.id != default_group.0)
        .map(|group| (group.id, group.name))
        .collect();
    server_repo::get_all_servers(&app.db)
        .await
        .unwrap()
        .iter()
        .map(|row| (row.shortpath.clone(), settings(row, &groups)))
        .collect()
}

async fn record_on_disk(root: &Path) -> Option<record::ServerRecord> {
    match parse_record(&tokio::fs::read(record_path(root)).await.ok()?) {
        ParsedRecord::Valid(record) => Some(record),
        _ => None,
    }
}

/// What a start of the launcher does with servers, in its order: the
/// backfill runs before the instance scan, the server list after it.
async fn start(app: &crate::managers::App) {
    app.server_manager().backfill_server_records().await;
    app.server_manager().load_servers().await.unwrap();
}

async fn root_of(app: &crate::managers::App, id: ServerId) -> PathBuf {
    let row = server_repo::get_server(&app.db, id.0)
        .await
        .unwrap()
        .unwrap();
    app.settings_manager()
        .runtime_path
        .get_servers()
        .get_server_path(&row.shortpath)
        .get_root()
}

/// Waits for a create's background install to end. Offline, or with a game
/// version or pack that does not exist as here, it fails at its first
/// request and writes nothing into the server's folder.
async fn settled(app: &crate::managers::App, id: ServerId) -> ServerState {
    tokio::time::timeout(std::time::Duration::from_secs(120), async {
        loop {
            match app.server_manager().get_server_state(id).await.unwrap() {
                ServerState::Installing(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await
                }
                state => return state,
            }
        }
    })
    .await
    .expect("the install did not end")
}

/// Two servers, one plain and one from a modpack, as a build without
/// records (2.0.40) left them: rows only, written here straight into the
/// database. The next start's backfill writes their records, and the
/// launcher's own actions then edit them and move the pack into a named
/// folder. The database is then lost: a fresh one starts on the same
/// servers folder. Both come back with their settings, folder and pack link,
/// in the library, and the user is told once. The tests below check each
/// action's own record write, and servers created by this build.
#[tokio::test]
async fn servers_come_back_with_their_settings_folder_and_pack_on_a_fresh_database() {
    let before = crate::setup_managers_for_test().await;
    let app = before.app.clone();
    let (plain, plain_root) = insert(&app, "plain_1", None).await;
    let (pack, pack_root) =
        insert(&app, "pack_2", Some(("modrinth", "1KVo5zza", "abcdEFGH"))).await;
    assert!(record_on_disk(&plain_root).await.is_none());
    start(&app).await;
    assert!(record_on_disk(&plain_root).await.is_some());
    assert!(record_on_disk(&pack_root).await.is_some());

    let manager = app.server_manager();
    manager
        .update_server(ServerSettingsUpdate {
            server_id: plain,
            name: Some("Renamed".into()),
            xmx: Some(6144),
            xms: Some(2048),
            extra_java_args: Some(Some("-XX:+UseG1GC".into())),
            auto_restart: Some(true),
        })
        .await
        .unwrap();
    manager
        .update_server_properties(
            plain,
            HashMap::from([
                ("motd".to_owned(), "Welcome".to_owned()),
                ("max-players".to_owned(), "8".to_owned()),
                ("online-mode".to_owned(), "false".to_owned()),
                ("server-port".to_owned(), "25600".to_owned()),
            ]),
        )
        .await
        .unwrap();
    manager.set_favorite(pack, true).await.unwrap();
    let folder = manager.create_server_group("Friends".into()).await.unwrap();
    manager
        .move_server(pack, ServerMoveTarget::EndOfGroup(folder))
        .await
        .unwrap();
    manager
        .rename_server_group(folder, "Best friends".into())
        .await
        .unwrap();
    tokio::fs::write(pack_root.join("icon.png"), b"png")
        .await
        .unwrap();

    let expected = rows_by_shortpath(&app).await;
    assert_eq!(expected.len(), 2);
    assert_eq!(expected["pack_2"].1.as_deref(), Some("Best friends"));
    assert_eq!(expected["plain_1"].0.motd, "Welcome");
    assert_eq!(expected["plain_1"].0.port, 25600);
    let record = record_on_disk(&plain_root).await.unwrap();
    assert_eq!(
        (record.name.as_str(), record.xmx, record.max_players),
        ("Renamed", 6144, 8)
    );

    let after = crate::setup_managers_for_test().await;
    let fresh = after.app.clone();
    copy_dir(&servers_dir(&app), &servers_dir(&fresh));
    start(&fresh).await;

    assert_eq!(rows_by_shortpath(&fresh).await, expected);
    assert_eq!(fresh.server_manager().restored_server_count(), 2);
    let groups = fresh.server_manager().list_groups().await.unwrap();
    let listed: Vec<(&str, Vec<&str>)> = groups
        .iter()
        .map(|group| {
            (
                group.name.as_str(),
                group.servers.iter().map(|s| s.name.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            ("Default", vec!["Renamed"]),
            ("Best friends", vec!["pack_2 name"])
        ]
    );
    let restored_pack = groups[1].servers[0].clone();
    let info = restored_pack.modpack_info.unwrap();
    assert_eq!(
        (
            info.platform.as_str(),
            info.project_id.as_str(),
            info.file_id.as_str()
        ),
        ("modrinth", "1KVo5zza", "abcdEFGH")
    );
    assert!(restored_pack.favorite);
    assert_eq!(restored_pack.icon_revision, Some(1));
    // The folder sits after the default group, at the end of the library.
    assert!(groups[0].servers[0].library_position.is_some());
    assert!(groups[1].library_position.is_some());

    fresh.server_manager().acknowledge_restored_servers();
    assert_eq!(fresh.server_manager().restored_server_count(), 0);
    // Restored once: the next start finds a row for each folder.
    start(&fresh).await;
    assert_eq!(rows_by_shortpath(&fresh).await.len(), 2);
    assert_eq!(fresh.server_manager().restored_server_count(), 0);
}

/// A server folder the user copied by hand comes back as a separate server
/// in its own folder, named after that folder too so the library never
/// shows two servers that look the same, and a stale or broken record of a
/// server that still has its row is rewritten from the row, which wins.
#[tokio::test]
async fn a_copied_folder_is_a_second_server_and_the_row_wins_over_a_stale_record() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (_, root) = insert(&app, "original_1", None).await;
    start(&app).await;
    let copy = servers_dir(&app).join("original_1 - Copy");
    copy_dir(&root, &copy);

    let mut stale = record_on_disk(&root).await.unwrap();
    stale.name = "Edited on an older build".into();
    tokio::fs::write(record_path(&root), stale.to_json().unwrap())
        .await
        .unwrap();
    start(&app).await;

    let rows = rows_by_shortpath(&app).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows["original_1"].0.name, "original_1 name");
    assert_eq!(
        rows["original_1 - Copy"].0.name,
        "original_1 name (original_1 - Copy)"
    );
    assert_eq!(
        record_on_disk(&copy).await.unwrap().name,
        "original_1 name (original_1 - Copy)",
        "the copy's record names it as the library does"
    );
    assert_eq!(
        record_on_disk(&root).await.unwrap().name,
        "original_1 name",
        "the backfill rewrites a record that differs from its row"
    );

    tokio::fs::write(record_path(&root), b"{broken")
        .await
        .unwrap();
    start(&app).await;
    assert_eq!(record_on_disk(&root).await.unwrap().name, "original_1 name");
    assert_eq!(rows_by_shortpath(&app).await.len(), 2);
}

/// Copies lost together with their original, as after a database reset,
/// come back told apart the same way, the original under its own name.
#[tokio::test]
async fn a_copy_lost_with_its_original_comes_back_named_after_its_folder() {
    let before = crate::setup_managers_for_test().await;
    let app = before.app.clone();
    let (_, root) = insert(&app, "original_1", None).await;
    start(&app).await;
    copy_dir(&root, &servers_dir(&app).join("original_1 - Copy"));

    let after = crate::setup_managers_for_test().await;
    let fresh = after.app.clone();
    copy_dir(&servers_dir(&app), &servers_dir(&fresh));
    start(&fresh).await;

    let rows = rows_by_shortpath(&fresh).await;
    assert_eq!(rows["original_1"].0.name, "original_1 name");
    assert_eq!(
        rows["original_1 - Copy"].0.name,
        "original_1 name (original_1 - Copy)"
    );
    assert_eq!(fresh.server_manager().restored_server_count(), 2);
}

/// A malformed, empty or newer record never adds a server and never stops
/// the list from loading.
#[tokio::test]
async fn unreadable_records_are_skipped_and_the_list_still_loads() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "kept_1", None).await;
    start(&app).await;
    let mut newer = serde_json::to_value(record_on_disk(&root).await.unwrap()).unwrap();
    newer["version"] = serde_json::json!(record::RECORD_VERSION + 1);
    for (name, contents) in [
        ("empty_2", b"".to_vec()),
        ("broken_3", b"{\"version\":1".to_vec()),
        ("newer_4", serde_json::to_vec(&newer).unwrap()),
    ] {
        let folder = servers_dir(&app).join(name);
        tokio::fs::create_dir_all(&folder).await.unwrap();
        tokio::fs::write(folder.join(RECORD_FILE_NAME), contents)
            .await
            .unwrap();
    }

    start(&app).await;

    assert_eq!(
        rows_by_shortpath(&app).await.keys().collect::<Vec<_>>(),
        ["kept_1"]
    );
    assert!(app.server_manager.servers.read().await.contains_key(&id));
    assert_eq!(app.server_manager().restored_server_count(), 0);
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// A delete whose folder removal fails (here at its very first step, as a
/// file Windows keeps locked can make it fail part way) leaves a folder
/// without a record, which the next start does not bring back.
#[cfg(unix)]
#[tokio::test]
async fn a_server_whose_folder_could_not_be_removed_does_not_come_back() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "deleted_1", None).await;
    tokio::fs::write(root.join("server").join("world.dat"), b"world")
        .await
        .unwrap();
    start(&app).await;
    assert!(record_on_disk(&root).await.is_some());

    // Write and search only: the folder cannot be listed, so removing it
    // fails before it removes anything.
    set_mode(&root, 0o300);
    if std::fs::read_dir(&root).is_ok() {
        set_mode(&root, 0o755);
        eprintln!("skipped: this user ignores directory permissions");
        return;
    }
    let deleted = app.server_manager().delete_server(id).await;
    set_mode(&root, 0o755);
    assert!(deleted.is_err(), "the folder removal was meant to fail");
    assert!(root.join("server").join("world.dat").exists());
    assert!(!record_path(&root).exists());
    assert!(
        server_repo::get_server(&app.db, id.0)
            .await
            .unwrap()
            .is_none()
    );

    start(&app).await;
    assert!(rows_by_shortpath(&app).await.is_empty());
    assert_eq!(app.server_manager().restored_server_count(), 0);
}

/// When even the record cannot be removed, nothing is deleted, the server
/// stays usable with a failed install's error still on its tile, and the
/// error says why and what to do. Once the file can be removed the same
/// delete goes through.
#[cfg(unix)]
#[tokio::test]
async fn a_record_that_cannot_be_removed_stops_the_delete_before_anything_is_lost() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "locked_1", None).await;
    start(&app).await;
    let failed_install = ServerState::Stopped {
        failed_task: Some(VisualTaskId(41)),
    };
    app.server_manager
        .servers
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .state = failed_install.clone();

    set_mode(&root, 0o500);
    if std::fs::File::create(root.join("probe")).is_ok() {
        let _ = std::fs::remove_file(root.join("probe"));
        set_mode(&root, 0o755);
        eprintln!("skipped: this user ignores directory permissions");
        return;
    }
    let error = app.server_manager().delete_server(id).await.unwrap_err();
    set_mode(&root, 0o755);
    let error = format!("{error:#}");
    assert!(error.contains("Nothing was deleted"), "{error}");
    assert!(
        error.contains("GDLauncher is not allowed to remove it"),
        "a folder without write permission is no program holding the file: {error}"
    );
    assert!(
        server_repo::get_server(&app.db, id.0)
            .await
            .unwrap()
            .is_some()
    );
    assert!(record_path(&root).exists());
    assert_eq!(
        app.server_manager().get_server_state(id).await.unwrap(),
        failed_install
    );

    app.server_manager().delete_server(id).await.unwrap();
    assert!(!root.exists());
    assert!(
        server_repo::get_server(&app.db, id.0)
            .await
            .unwrap()
            .is_none()
    );
}

/// A settings change that reaches its record write after the server was
/// deleted does not bring the record back.
#[tokio::test]
async fn a_record_write_after_a_delete_writes_nothing() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "gone_1", None).await;
    start(&app).await;
    app.server_manager().delete_server(id).await.unwrap();
    tokio::fs::create_dir_all(&root).await.unwrap();

    app.server_manager().sync_server_record(id).await;

    assert!(!record_path(&root).exists());
}

/// A server created on this build has its record as soon as the create
/// returns, before its install has run: a reset before the next start, with
/// no backfill in between, still brings it back with its folder.
#[tokio::test]
async fn a_server_created_here_has_its_record_before_any_restart() {
    let before = crate::setup_managers_for_test().await;
    let app = before.app.clone();
    let manager = app.server_manager();
    let folder = manager.create_server_group("Friends".into()).await.unwrap();
    let id = manager
        .create_server(
            folder,
            "Made here".into(),
            "gdl-test-no-such-version".into(),
            Some(25570),
            None,
            None,
        )
        .await
        .unwrap();
    let root = root_of(&app, id).await;

    let record = record_on_disk(&root).await.expect("written by the create");
    assert_eq!(
        (
            record.name.as_str(),
            record.game_version.as_str(),
            record.server_type.as_str(),
            record.port,
            record.group.as_deref(),
        ),
        (
            "Made here",
            "gdl-test-no-such-version",
            "vanilla",
            25570,
            Some("Friends")
        )
    );
    // The install fails here; it leaves the record and writes nothing else.
    assert!(matches!(
        settled(&app, id).await,
        ServerState::Stopped {
            failed_task: Some(_)
        }
    ));
    assert_eq!(record_on_disk(&root).await, Some(record));

    let after = crate::setup_managers_for_test().await;
    let fresh = after.app.clone();
    copy_dir(&servers_dir(&app), &servers_dir(&fresh));
    start(&fresh).await;
    let groups = fresh.server_manager().list_groups().await.unwrap();
    let restored: Vec<(&str, &str, i32)> = groups
        .iter()
        .flat_map(|group| {
            group
                .servers
                .iter()
                .map(move |server| (group.name.as_str(), server.name.as_str(), server.port))
        })
        .collect();
    assert_eq!(restored, [("Friends", "Made here", 25570)]);
}

/// A server created from a modpack has its record, with the pack link, as
/// soon as the create returns. A failed install leaves it as it was.
#[tokio::test]
async fn a_server_created_from_a_modpack_has_its_record_and_pack_link_at_once() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let manager = app.server_manager();
    let group = manager.get_default_group().await.unwrap();
    let id = manager
        .create_server_from_modpack(
            group,
            "Pack here".into(),
            modpack::ServerModpackSource::Modrinth {
                project_id: "gdl-test-project".into(),
                version_id: "gdl-test-version".into(),
            },
            None,
            None,
        )
        .await
        .unwrap();
    let root = root_of(&app, id).await;

    let record = record_on_disk(&root).await.expect("written by the create");
    assert_eq!(
        (
            record.name.as_str(),
            record.server_type.as_str(),
            record.modpack_platform.as_deref(),
            record.modpack_project_id.as_deref(),
            record.modpack_file_id.as_deref(),
            record.group.as_deref(),
        ),
        (
            "Pack here",
            "modded",
            Some("modrinth"),
            Some("gdl-test-project"),
            Some("gdl-test-version"),
            None
        )
    );
    assert!(matches!(
        settled(&app, id).await,
        ServerState::Stopped {
            failed_task: Some(_)
        }
    ));
    assert_eq!(record_on_disk(&root).await, Some(record));
}

/// The end of a modpack install stores the pack's game version, loader and
/// port, and the record follows at once instead of keeping "unknown".
#[tokio::test]
async fn a_finished_modpack_install_rewrites_the_record_with_the_pack_settings() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "pack_1", Some(("modrinth", "1KVo5zza", "abcdEFGH"))).await;
    server_repo::set_server_game_version_and_modloader(&app.db, id.0, "unknown", None, None)
        .await
        .unwrap();
    start(&app).await;
    assert_eq!(record_on_disk(&root).await.unwrap().game_version, "unknown");

    app.server_manager()
        .store_pack_result(
            id,
            &modpack::ServerPackResult {
                game_version: "1.20.1".into(),
                modloader_type: Some("neoforge".into()),
                modloader_version: Some("20.4.237".into()),
            },
            async { Ok(25590) },
        )
        .await
        .unwrap();

    let record = record_on_disk(&root).await.unwrap();
    assert_eq!(
        (
            record.game_version.as_str(),
            record.modloader_type.as_deref(),
            record.modloader_version.as_deref(),
            record.port,
        ),
        ("1.20.1", Some("neoforge"), Some("20.4.237"), 25590)
    );
}

/// A pack whose properties step fails, a read-only `server.properties` for
/// one, is still installed: the row and the record name its game version
/// and loader, not the ones it replaced, and keep the old port.
#[tokio::test]
async fn a_pack_install_whose_properties_step_fails_still_stores_its_version() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "pack_2", Some(("modrinth", "1KVo5zza", "abcdEFGH"))).await;
    start(&app).await;

    let error = app
        .server_manager()
        .store_pack_result(
            id,
            &modpack::ServerPackResult {
                game_version: "1.21.1".into(),
                modloader_type: Some("neoforge".into()),
                modloader_version: Some("21.1.77".into()),
            },
            async { Err(anyhow::anyhow!("properties refused")) },
        )
        .await
        .unwrap_err();
    assert_eq!(format!("{error:#}"), "properties refused");

    let row = server_repo::get_server(&app.db, id.0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            row.game_version.as_str(),
            row.modloader_type.as_deref(),
            row.modloader_version.as_deref(),
            row.port,
        ),
        ("1.21.1", Some("neoforge"), Some("21.1.77"), 25565)
    );
    let record = record_on_disk(&root).await.unwrap();
    assert_eq!(
        (
            record.game_version.as_str(),
            record.modloader_type.as_deref(),
            record.modloader_version.as_deref(),
            record.port,
        ),
        ("1.21.1", Some("neoforge"), Some("21.1.77"), 25565)
    );
}

/// Every action that changes a setting the record keeps rewrites it right
/// away, each checked before the next action could write it too.
#[tokio::test]
async fn each_settings_change_rewrites_the_record_right_away() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "edited_1", None).await;
    start(&app).await;
    let manager = app.server_manager();
    let on_disk = || async { record_on_disk(&root).await.unwrap() };

    manager
        .update_server(ServerSettingsUpdate {
            server_id: id,
            name: Some("Renamed".into()),
            xmx: Some(6144),
            xms: Some(2048),
            extra_java_args: Some(Some("-XX:+UseG1GC".into())),
            auto_restart: Some(true),
        })
        .await
        .unwrap();
    let record = on_disk().await;
    assert_eq!(
        (
            record.name.as_str(),
            record.xmx,
            record.xms,
            record.extra_java_args.as_str(),
            record.auto_restart
        ),
        ("Renamed", 6144, 2048, "-XX:+UseG1GC", true)
    );

    manager
        .update_server_properties(
            id,
            HashMap::from([
                ("motd".to_owned(), "Welcome".to_owned()),
                ("server-port".to_owned(), "25600".to_owned()),
            ]),
        )
        .await
        .unwrap();
    let record = on_disk().await;
    assert_eq!((record.motd.as_str(), record.port), ("Welcome", 25600));

    manager.set_favorite(id, true).await.unwrap();
    assert!(on_disk().await.favorite);

    let folder = manager.create_server_group("Friends".into()).await.unwrap();
    manager
        .move_server(id, ServerMoveTarget::EndOfGroup(folder))
        .await
        .unwrap();
    assert_eq!(on_disk().await.group.as_deref(), Some("Friends"));

    manager
        .rename_server_group(folder, "Best friends".into())
        .await
        .unwrap();
    assert_eq!(on_disk().await.group.as_deref(), Some("Best friends"));

    // Its servers go back to the default group, and so do their records.
    manager.delete_server_group(folder).await.unwrap();
    assert_eq!(on_disk().await.group, None);

    manager.set_favorite(id, false).await.unwrap();
    assert!(!on_disk().await.favorite);
}

/// A quit that begins while the first start of this build is still busy
/// does not stop the backfill: every server keeps a record to come back
/// from after going back to 2.0.40 and resetting there.
#[tokio::test]
async fn the_backfill_still_writes_the_records_once_a_quit_began() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (_, root) = insert(&app, "quit_1", None).await;
    assert!(
        app.runtime_lifecycle
            .begin_shutdown(
                "quit",
                crate::managers::runtime_lifecycle::ShutdownPurpose::Quit
            )
            .unwrap()
    );

    app.server_manager().backfill_server_records().await;

    assert!(record_on_disk(&root).await.is_some());
}

/// While the data folder is being moved nothing is written into it; the
/// next start writes the records in the new place.
#[tokio::test]
async fn the_backfill_writes_nothing_while_the_data_folder_moves() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "moving_1", None).await;
    app.runtime_lifecycle.begin_relocation("move").unwrap();

    app.server_manager().backfill_server_records().await;
    app.server_manager().sync_server_record(id).await;
    assert!(!record_path(&root).exists());

    app.runtime_lifecycle.cancel_relocation("move").unwrap();
    app.server_manager().backfill_server_records().await;
    assert!(record_on_disk(&root).await.is_some());
}

/// A data folder on a macOS network share refuses std's F_FULLFSYNC (the SMB
/// client answers ENOTSUP), so a record flushed that way was never written
/// there and a reset brought none of its servers back. Records are flushed
/// through `util::durable`, which falls back to the plain fsync such a share
/// supports. The test stands in for that share on every platform.
#[cfg(unix)]
#[tokio::test]
async fn records_are_written_where_the_drive_refuses_a_full_flush() {
    let env = crate::setup_managers_for_test().await;
    let app = env.app.clone();
    let (id, root) = insert(&app, "share_1", None).await;
    let syncs = crate::util::durable::test_hooks::FileSyncs::refusing_full_flush();

    app.server_manager().backfill_server_records().await;
    assert!(record_on_disk(&root).await.is_some());
    assert!(syncs.synced(&record_path(&root)), "backfill");

    app.server_manager().set_favorite(id, true).await.unwrap();
    assert!(record_on_disk(&root).await.unwrap().favorite);
    assert!(syncs.synced(&record_path(&root)), "settings change");
}

#[test]
fn each_reason_a_record_cannot_be_removed_has_its_own_way_out() {
    use std::io::ErrorKind;
    let permission = remove_record_advice(ErrorKind::PermissionDenied);
    assert!(permission.contains("not allowed"), "{permission}");
    assert!(
        permission.contains("remove the file yourself"),
        "{permission}"
    );
    let read_only = remove_record_advice(ErrorKind::ReadOnlyFilesystem);
    assert!(read_only.contains("drive"), "{read_only}");
    // A Windows sharing violation has no kind of its own.
    let busy = remove_record_advice(ErrorKind::Other);
    assert!(busy.contains("Close any program"), "{busy}");
}
