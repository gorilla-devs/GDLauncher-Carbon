//! Local addon scans: what they read, which gate they hold while reading, and
//! how they treat files they cannot read. Instance and server scans share the
//! same snapshot / hash / commit structure.

use super::tests::{addon_fixture, metadata_jar, ordinary_scan};
use super::*;
use crate::domain::instance::AddonType;
use std::time::Duration;

async fn pass(app: &App, id: InstanceId, verify: VerifyScope) -> anyhow::Result<()> {
    tokio::time::timeout(
        Duration::from_secs(10),
        app.meta_cache_manager().override_caching_verified_and_wait(
            CacheEntityId::Instance(id),
            verify,
            false,
            false,
        ),
    )
    .await
    .expect("an addon pass must settle")
}

async fn rows(app: &App, id: InstanceId) -> Vec<mfcdb::ModFileCacheRow> {
    let mut rows = mfcdb::get_mod_files_by_instance(&app.db, *id)
        .await
        .unwrap();
    rows.sort_by(|a, b| (&a.addon_type, &a.filename).cmp(&(&b.addon_type, &b.filename)));
    rows
}

fn row<'a>(rows: &'a [mfcdb::ModFileCacheRow], filename: &str) -> &'a mfcdb::ModFileCacheRow {
    rows.iter()
        .find(|row| row.filename == filename)
        .unwrap_or_else(|| panic!("no row for {filename}"))
}

fn qualified(row: &mfcdb::ModFileCacheRow) -> bool {
    row.file_stamp
        .as_deref()
        .is_some_and(|stamp| stamp.starts_with("parsed-v1:"))
}

fn at(seconds: i64) -> DbDateTime {
    DbDateTime(
        chrono::DateTime::from_timestamp(seconds, 0)
            .unwrap()
            .fixed_offset(),
    )
}

fn settings_note(id: InstanceId) -> crate::domain::instance::InstanceSettingsUpdate {
    crate::domain::instance::InstanceSettingsUpdate {
        instance_id: id,
        name: None,
        use_loaded_icon: None,
        notes: Some("edited while addons were being scanned".into()),
        version: None,
        modloader: None,
        java_override: None,
        global_java_args: None,
        extra_java_args: None,
        memory: None,
        pre_launch_hook: None,
        post_exit_hook: None,
        wrapper_command: None,
        game_resolution: None,
        mod_sources: None,
        modpack_locked: None,
    }
}

#[tokio::test]
async fn installer_pass_reads_no_unchanged_stamped_addon() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(
        &app,
        "scan-installer-pass",
        AddonType::Mods,
        "mod-0.jar",
        b"mod 0",
    )
    .await;
    let mods = first.parent().unwrap().to_owned();
    let root = mods.parent().unwrap().to_owned();
    let mut existing = vec![first];
    for i in 1..40 {
        let path = mods.join(format!("mod-{i}.jar"));
        std::fs::write(&path, format!("mod {i}")).unwrap();
        existing.push(path);
    }
    std::fs::create_dir_all(root.join("resourcepacks")).unwrap();
    let pack = root.join("resourcepacks/pack.zip");
    std::fs::write(&pack, b"a large resource pack").unwrap();
    existing.push(pack);
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;
    assert_eq!(before.len(), existing.len());

    // What an install's finalization pass sees: one newly published file.
    let published = mods.join("new.jar");
    std::fs::write(&published, b"new").unwrap();
    let mcm = app.meta_cache_manager();
    mcm.clear_scan_observations();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    for path in &existing {
        assert_eq!(
            mcm.scan_observations(path, "hash"),
            0,
            "{} is unchanged and must not be read",
            path.display()
        );
    }
    assert_eq!(mcm.scan_observations(&published, "hash"), 1);
    let after = rows(&app, id).await;
    assert_eq!(after.len(), before.len() + 1);
    for old in &before {
        assert_eq!(row(&after, &old.filename), old);
    }
}

#[tokio::test]
async fn export_pass_reads_each_mod_once_and_no_packs() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(&app, "scan-export-pass", AddonType::Mods, "a.jar", b"a").await;
    let mods = first.parent().unwrap().to_owned();
    let root = mods.parent().unwrap().to_owned();
    let mut linked = vec![first];
    for name in ["b.jar", "c.jar"] {
        std::fs::write(mods.join(name), name).unwrap();
        linked.push(mods.join(name));
    }
    let mut packs = Vec::new();
    for (folder, name) in [("resourcepacks", "pack.zip"), ("shaderpacks", "shader.zip")] {
        std::fs::create_dir_all(root.join(folder)).unwrap();
        std::fs::write(root.join(folder).join(name), name).unwrap();
        packs.push(root.join(folder).join(name));
    }
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;
    assert_eq!(before.len(), 5);

    let mcm = app.meta_cache_manager();
    mcm.clear_scan_observations();
    pass(&app, id, VerifyScope::MODS).await.unwrap();
    for path in &linked {
        assert_eq!(mcm.scan_observations(path, "hash"), 1, "{}", path.display());
        assert_eq!(
            mcm.scan_observations(path, "parse"),
            0,
            "{}",
            path.display()
        );
    }
    for path in &packs {
        assert_eq!(mcm.scan_observations(path, "hash"), 0, "{}", path.display());
    }
    assert_eq!(
        rows(&app, id).await,
        before,
        "verified bytes rewrite nothing"
    );
}

/// The fixtures of an unrelated-unreadable-file report: none of them may fail
/// a pass, and none of them may be linked.
#[cfg(unix)]
#[tokio::test]
async fn export_pass_keeps_broken_links_unreadable_jars_and_file_folders_local() {
    use std::os::unix::fs::PermissionsExt;
    let app = crate::setup_managers_for_test().await;
    let (id, good) =
        addon_fixture(&app, "scan-bad-files", AddonType::Mods, "good.jar", b"good").await;
    let mods = good.parent().unwrap().to_owned();
    let root = mods.parent().unwrap().to_owned();
    let locked = mods.join("locked.jar");
    std::fs::write(&locked, b"locked bytes").unwrap();
    std::fs::create_dir_all(root.join("resourcepacks")).unwrap();
    std::fs::write(root.join("resourcepacks/pack.zip"), b"pack").unwrap();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;

    std::os::unix::fs::symlink("missing-target.jar", mods.join("broken.jar")).unwrap();
    let fresh = mods.join("fresh-locked.jar");
    std::fs::write(&fresh, b"never readable").unwrap();
    for path in [&locked, &fresh] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    std::fs::remove_dir_all(root.join("resourcepacks")).unwrap();
    std::fs::write(root.join("resourcepacks"), b"a file where a folder belongs").unwrap();
    // Permission bits do not stop root; the unreadable half is then moot.
    let unreadable = std::fs::read(&locked).is_err();

    let result = pass(&app, id, VerifyScope::MODS).await;
    for path in [&locked, &fresh] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    result.expect("unrelated unreadable files must not fail the pass");
    let after = rows(&app, id).await;
    assert_eq!(row(&after, "good.jar"), row(&before, "good.jar"));
    assert!(after.iter().all(|row| row.filename != "broken.jar"));
    assert_eq!(
        row(&after, "pack.zip"),
        row(&before, "pack.zip"),
        "rows of a folder that cannot be listed are kept"
    );
    if unreadable {
        assert_eq!(row(&after, "locked.jar"), row(&before, "locked.jar"));
        assert!(after.iter().all(|row| row.filename != "fresh-locked.jar"));
    }
}

#[tokio::test]
async fn scan_releases_the_instance_gate_while_hashing() {
    let app = crate::setup_managers_for_test().await;
    let (id, kept) = addon_fixture(&app, "scan-gate", AddonType::Mods, "kept.jar", b"kept").await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let kept_row = row(&rows(&app, id).await, "kept.jar").clone();
    let hashed = kept.with_file_name("hashed.jar");
    std::fs::write(&hashed, b"hashed while the gate is free").unwrap();

    let mcm = app.meta_cache_manager();
    let pause = mcm.arm_scan_pause(&hashed, "hash");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the scan reaches the hash boundary");

    let manager = app.instance_manager();
    // Play's preparation starts by taking exactly this gate.
    let gate = tokio::time::timeout(Duration::from_millis(100), manager.lock_instance(id))
        .await
        .expect("the instance gate is free while files are hashed");
    drop(gate);
    tokio::time::timeout(
        Duration::from_secs(1),
        manager.enable_mod(id, kept_row.id.clone(), false),
    )
    .await
    .expect("a toggle does not wait for hashing")
    .unwrap();
    tokio::time::timeout(
        Duration::from_secs(1),
        manager.update_instance(settings_note(id)),
    )
    .await
    .expect("a settings save does not wait for hashing")
    .unwrap();

    pause.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let after = rows(&app, id).await;
    assert!(after.iter().any(|row| row.filename == "hashed.jar"));
    let toggled = row(&after, "kept.jar");
    assert!(!toggled.enabled, "the toggle made during the scan is kept");
    assert_eq!(toggled.metadata_id, kept_row.metadata_id);
}

#[tokio::test]
async fn commit_discards_results_for_files_changed_after_hashing() {
    for action in ["rename", "replace", "delete", "toggle"] {
        let app = crate::setup_managers_for_test().await;
        let (id, toggled) = addon_fixture(
            &app,
            &format!("scan-discard-{action}"),
            AddonType::Mods,
            "toggled.jar",
            b"old",
        )
        .await;
        pass(&app, id, VerifyScope::NONE).await.unwrap();
        let toggled_row = row(&rows(&app, id).await, "toggled.jar").clone();
        let fresh = toggled.with_file_name("fresh.jar");
        std::fs::write(&fresh, b"fresh bytes").unwrap();
        std::fs::write(&toggled, b"new and longer content").unwrap();

        let mcm = app.meta_cache_manager();
        let pause = mcm.arm_scan_pause(Path::new(&format!("instance-{}", *id)), "commit");
        let scan_app = app.clone();
        let scan = tokio::spawn(async move {
            scan_app
                .meta_cache_manager()
                .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
            .await
            .expect("both files were hashed");
        match action {
            "rename" => {
                std::fs::rename(&fresh, fresh.with_file_name("fresh.jar.disabled")).unwrap()
            }
            "replace" => std::fs::write(&fresh, b"replaced after it was hashed").unwrap(),
            "delete" => std::fs::remove_file(&fresh).unwrap(),
            "toggle" => app
                .instance_manager()
                .enable_mod(id, toggled_row.id.clone(), false)
                .await
                .unwrap(),
            _ => unreachable!(),
        }
        pause.release.notify_one();
        tokio::time::timeout(Duration::from_secs(5), scan)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let after = rows(&app, id).await;
        let current = row(&after, "toggled.jar");
        if action == "toggle" {
            assert!(!current.enabled, "the toggle made during the scan is kept");
            assert_eq!(current.id, toggled_row.id);
            assert_eq!(
                current.metadata_id, toggled_row.metadata_id,
                "a result for the old spelling must not overwrite the toggled row"
            );
            assert!(after.iter().any(|row| row.filename == "fresh.jar"));
        } else {
            assert!(
                after.iter().all(|row| row.filename != "fresh.jar"),
                "{action}: no row may describe bytes that are gone"
            );
            assert_ne!(current.metadata_id, toggled_row.metadata_id);
        }
    }
}

/// Renaming an instance moves its folder; a pass that hashed the old folder
/// scans the new one before it reports success.
#[tokio::test]
async fn an_instance_renamed_mid_scan_is_scanned_again_at_its_new_folder() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(&app, "scan-rename", AddonType::Mods, "a.jar", b"a").await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    std::fs::write(first.with_file_name("fresh.jar"), b"fresh").unwrap();

    let mcm = app.meta_cache_manager();
    let pause = mcm.arm_scan_pause(Path::new(&format!("instance-{}", *id)), "commit");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the scan hashed the old folder");
    let mut rename = settings_note(id);
    rename.notes = None;
    rename.name = Some("scan-renamed".into());
    app.instance_manager()
        .update_instance(rename)
        .await
        .unwrap();
    assert!(!first.exists(), "the instance folder moved");
    pause.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let after = rows(&app, id).await;
    assert!(after.iter().any(|row| row.filename == "a.jar"));
    assert!(
        after.iter().any(|row| row.filename == "fresh.jar"),
        "the file added before the rename is cached from the new folder"
    );
}

#[tokio::test]
async fn shutdown_stops_a_scan_waiting_at_the_hash_boundary() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(&app, "scan-cancel", AddonType::Mods, "a.jar", b"a").await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let slow = first.with_file_name("slow.jar");
    std::fs::write(&slow, b"slow").unwrap();
    let pause = app.meta_cache_manager().arm_scan_pause(&slow, "hash");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the scan reaches the hash boundary");

    use crate::managers::runtime_lifecycle::{ShutdownPhase, ShutdownPurpose};
    app.runtime_lifecycle
        .prepare_shutdown("scan-quit", ShutdownPurpose::Quit, async { Ok(()) })
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), scan)
        .await
        .expect("a cancelled scan returns promptly")
        .unwrap();
    assert!(result.is_err(), "the waiter learns the scan did not finish");
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status = app
                .runtime_lifecycle
                .shutdown_status("scan-quit", ShutdownPurpose::Quit)
                .unwrap();
            match status.phase {
                ShutdownPhase::Ready | ShutdownPhase::Committed => break,
                ShutdownPhase::Unresolved => panic!("shutdown blocked: {:?}", status.blockers),
                ShutdownPhase::Draining => tokio::time::sleep(Duration::from_millis(10)).await,
            }
        }
    })
    .await
    .expect("shutdown settles once the scan stops");
}

#[tokio::test]
async fn upgraded_rows_are_stamped_from_stat_without_reading_files() {
    let app = crate::setup_managers_for_test().await;
    let body = r#"{"schemaVersion":1,"id":"example","version":"1.0.0"}"#;
    let (id, first) = addon_fixture(
        &app,
        "scan-upgrade",
        AddonType::Mods,
        "mod-0.jar",
        &metadata_jar("fabric.mod.json", body),
    )
    .await;
    let mods = first.parent().unwrap().to_owned();
    let mut paths = vec![first];
    for i in 1..6 {
        let path = mods.join(format!("mod-{i}.jar"));
        let body = body.replace("example", &format!("example{i}"));
        std::fs::write(&path, metadata_jar("fabric.mod.json", &body)).unwrap();
        paths.push(path);
    }
    let pack_folder = mods.parent().unwrap().join("resourcepacks");
    std::fs::create_dir_all(&pack_folder).unwrap();
    std::fs::write(pack_folder.join("pack.zip"), b"pack").unwrap();
    paths.push(pack_folder.join("pack.zip"));
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;

    // 2.0.40 left every row without a stamp, written after its file changed.
    let written = DbDateTime(Utc::now().fixed_offset());
    for row in &before {
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            &row.addon_type,
            &row.filename,
            &row.metadata_id,
            None,
            written.clone(),
        )
        .await
        .unwrap();
    }
    let mcm = app.meta_cache_manager();
    mcm.clear_scan_observations();
    ordinary_scan(&app, id).await;
    for path in &paths {
        assert_eq!(mcm.scan_observations(path, "hash"), 0, "{}", path.display());
        assert_eq!(
            mcm.scan_observations(path, "parse"),
            0,
            "{}",
            path.display()
        );
    }
    let after = rows(&app, id).await;
    for (old, new) in before.iter().zip(&after) {
        assert_eq!((&old.id, &old.metadata_id), (&new.id, &new.metadata_id));
        assert!(
            qualified(new),
            "{} gets a parser-qualified stamp",
            new.filename
        );
    }
}

/// Rows whose file looks newer than the row (a data folder that was copied,
/// migrated or restored gives every file newer times) are settled by one read
/// of the bytes, not a full hash and parse; only bytes that really differ are
/// parsed again.
#[tokio::test]
async fn upgraded_rows_newer_than_their_row_are_settled_by_one_read() {
    let app = crate::setup_managers_for_test().await;
    let (id, modified) =
        addon_fixture(&app, "scan-upgrade-changed", AddonType::Mods, "a.jar", b"a").await;
    let changed = modified.with_file_name("b.jar");
    std::fs::write(&changed, b"b").unwrap();
    let replaced = modified.with_file_name("c.jar");
    std::fs::write(&replaced, b"c").unwrap();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;

    // a.jar was modified after its row was last written.
    let a = row(&before, "a.jar");
    mfcdb::update_mod_file_stamp(
        &app.db,
        *id,
        "mods",
        "a.jar",
        &a.metadata_id,
        None,
        at(946_684_800),
    )
    .await
    .unwrap();
    // b.jar keeps an old modification time, but changed (Unix) or was
    // created (Windows) after its row was last written.
    std::fs::File::options()
        .write(true)
        .open(&changed)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(978_307_200)),
        )
        .unwrap();
    let b = row(&before, "b.jar");
    mfcdb::update_mod_file_stamp(
        &app.db,
        *id,
        "mods",
        "b.jar",
        &b.metadata_id,
        None,
        at(1_262_304_000),
    )
    .await
    .unwrap();
    // c.jar was really replaced, keeping its size.
    std::fs::write(&replaced, b"C").unwrap();
    let c = row(&before, "c.jar");
    mfcdb::update_mod_file_stamp(
        &app.db,
        *id,
        "mods",
        "c.jar",
        &c.metadata_id,
        None,
        at(946_684_800),
    )
    .await
    .unwrap();

    let mcm = app.meta_cache_manager();
    mcm.clear_scan_observations();
    ordinary_scan(&app, id).await;
    for path in [&modified, &changed] {
        assert_eq!(mcm.scan_observations(path, "hash"), 1, "{}", path.display());
        assert_eq!(
            mcm.scan_observations(path, "parse"),
            0,
            "{} is settled without a parse",
            path.display()
        );
    }
    assert_eq!(mcm.scan_observations(&replaced, "parse"), 1);
    let after = rows(&app, id).await;
    assert!(after.iter().all(qualified));
    for name in ["a.jar", "b.jar"] {
        assert_eq!(
            row(&after, name).metadata_id,
            row(&before, name).metadata_id,
            "{name} keeps its metadata"
        );
    }
    assert_ne!(
        row(&after, "c.jar").metadata_id,
        c.metadata_id,
        "new bytes get new metadata"
    );
}

/// An export's pass on an upgraded instance reads each mod once to settle
/// its 2.0.40 row, rather than hashing and parsing it again.
#[tokio::test]
async fn an_export_pass_settles_upgraded_mods_with_one_read() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) =
        addon_fixture(&app, "scan-upgrade-export", AddonType::Mods, "a.jar", b"a").await;
    let second = first.with_file_name("b.jar");
    std::fs::write(&second, b"b").unwrap();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;
    for row in &before {
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            &row.addon_type,
            &row.filename,
            &row.metadata_id,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
    }
    let mcm = app.meta_cache_manager();
    mcm.clear_scan_observations();
    pass(&app, id, VerifyScope::MODS).await.unwrap();
    for path in [&first, &second] {
        assert_eq!(
            mcm.scan_observations(path, "hash"),
            1,
            "{} is read even though its times are old",
            path.display()
        );
        assert_eq!(
            mcm.scan_observations(path, "parse"),
            0,
            "{}",
            path.display()
        );
    }
    let after = rows(&app, id).await;
    assert!(after.iter().all(qualified));
    for (old, new) in before.iter().zip(&after) {
        assert_eq!((&old.id, &old.metadata_id), (&new.id, &new.metadata_id));
    }
}

/// Start a background pass of `id` (as at launcher start or after an
/// import); it may be paused, pre-empted or stopped.
async fn start_background_pass(app: &App, id: InstanceId) {
    app.meta_cache_manager()
        .local_targets
        .send_modify_always(|targets| {
            targets.set_priority(CacheTarget {
                entity_id: CacheEntityId::Instance(id),
                callback: None,
            });
        })
        .await;
}

/// Wait until `check` holds, polling the database.
async fn eventually<F: std::future::Future<Output = bool>>(
    what: &str,
    mut check: impl FnMut() -> F,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !check().await {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what}"));
}

/// A pass that is pre-empted (here by an install's pass; Play and opening
/// another instance's Addons tab drop it the same way) keeps what it already
/// read: results are committed in batches while the rest of the folder is
/// still being read, and the next pass reads only what was left. Seeded like
/// a 2.0.40 data folder that was copied to a new computer, where every file
/// is newer than its row and must be read once.
#[tokio::test]
async fn an_interrupted_pass_keeps_what_it_already_read() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(
        &app,
        "scan-interrupted",
        AddonType::Mods,
        "mod-00.jar",
        b"mod 0",
    )
    .await;
    let mods = first.parent().unwrap().to_owned();
    let mut settled = vec![first];
    for i in 1..(SCAN_COMMIT_BATCH * 2 + 3) {
        let path = mods.join(format!("mod-{i:02}.jar"));
        std::fs::write(&path, format!("mod {i}")).unwrap();
        settled.push(path);
    }
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    for row in rows(&app, id).await {
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            &row.addon_type,
            &row.filename,
            &row.metadata_id,
            None,
            at(946_684_800),
        )
        .await
        .unwrap();
    }
    // A file without a row is queued after every row, so every settled file
    // has been read by the time the pass stops at this one.
    let slow = mods.join("slow.jar");
    std::fs::write(&slow, b"slow").unwrap();

    let mcm = app.meta_cache_manager();
    let pause = mcm.arm_scan_pause(&slow, "hash");
    start_background_pass(&app, id).await;
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the pass reaches the last file");
    eventually(
        "files read before the pause are committed while the pass still runs",
        || async {
            let current = rows(&app, id).await;
            current.len() == settled.len() && current.iter().all(qualified)
        },
    )
    .await;
    tokio::time::timeout(
        Duration::from_millis(100),
        app.instance_manager().lock_instance(id),
    )
    .await
    .expect("the instance gate is free while the pass reads");

    // An install's pass pre-empts the background pass, which is dropped
    // while it is still reading.
    mcm.clear_scan_observations();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    for path in &settled {
        assert_eq!(
            mcm.scan_observations(path, "hash"),
            0,
            "{} was already committed and is not read again",
            path.display()
        );
    }
    assert_eq!(mcm.scan_observations(&slow, "hash"), 1);
    let after = rows(&app, id).await;
    assert_eq!(after.len(), settled.len() + 1);
    assert!(after.iter().all(qualified));
    drop(pause);
}

/// A freshly imported pack fills the addon list while it is read, instead of
/// showing "No addons installed" until the last file is done.
#[tokio::test]
async fn a_new_folder_fills_in_while_it_is_read() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(
        &app,
        "scan-fills-in",
        AddonType::Mods,
        "mod-00.jar",
        b"mod 0",
    )
    .await;
    let mods = first.parent().unwrap().to_owned();
    let mut others = vec![first];
    for i in 1..20 {
        let path = mods.join(format!("mod-{i:02}.jar"));
        std::fs::write(&path, format!("mod {i}")).unwrap();
        others.push(path);
    }
    let slow = mods.join("slow.jar");
    std::fs::write(&slow, b"slow").unwrap();
    let mcm = app.meta_cache_manager();
    let pause = mcm.arm_scan_pause(&slow, "hash");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the pass reaches the paused file");
    // Files are read in no particular order, several at a time (one at a time
    // on a single-core machine): every file read before the paused one shows
    // up while the pass is still running.
    let read_before = |mcm: &MetaCacheManager| {
        others
            .iter()
            .filter(|path| mcm.scan_observations(path, "hash") > 0)
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };
    let concurrent = available_parallelism().map_or(1, |n| n.get()) / 2 >= 2;
    eventually("files read so far are listed while the pass runs", || {
        let expected = read_before(&mcm);
        let app = &app;
        async move {
            let current = rows(app, id).await;
            (!concurrent || expected.len() == 20)
                && expected
                    .iter()
                    .all(|name| current.iter().any(|row| &row.filename == name))
        }
    })
    .await;
    assert!(
        rows(&app, id)
            .await
            .iter()
            .all(|row| row.filename != "slow.jar")
    );
    tokio::time::timeout(
        Duration::from_millis(100),
        app.instance_manager().lock_instance(id),
    )
    .await
    .expect("the instance gate is free while the pass reads");
    pause.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(rows(&app, id).await.len(), 21);
}

/// Renaming an addon by case only leaves one row under its new name. On
/// case-insensitive filesystems (Windows, macOS) the old name still resolves
/// to the renamed file, so only an exact listing proves the old row stale.
#[tokio::test]
async fn a_case_only_rename_leaves_one_row() {
    let app = crate::setup_managers_for_test().await;
    let (id, path) = addon_fixture(&app, "scan-case", AddonType::Mods, "JEI.jar", b"jei").await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    std::fs::rename(&path, path.with_file_name("jei.jar")).unwrap();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let after = rows(&app, id).await;
    assert_eq!(
        after
            .iter()
            .map(|row| row.filename.as_str())
            .collect::<Vec<_>>(),
        vec!["jei.jar"]
    );
}

#[tokio::test]
async fn stale_rows_are_judged_by_exact_listing() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("mods");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("jei.jar"), b"jei").unwrap();
    std::fs::write(folder.join("off.jar.disabled"), b"off").unwrap();
    let mut listings = FolderListings::default();
    assert!(listings.confirmed_absent(&folder, "JEI.jar").await);
    assert!(!listings.confirmed_absent(&folder, "jei.jar").await);
    assert!(!listings.confirmed_absent(&folder, "off.jar").await);
    assert!(
        listings
            .confirmed_absent(&dir.path().join("missing"), "a.jar")
            .await,
        "a missing folder holds nothing"
    );
    let not_a_folder = dir.path().join("file");
    std::fs::write(&not_a_folder, b"").unwrap();
    assert!(
        !listings.confirmed_absent(&not_a_folder, "a.jar").await,
        "a folder that cannot be listed keeps its rows"
    );
}

/// A folder behind a link to a folder that cannot be found (a drive that is
/// not plugged in) is not known to be empty, so its rows are kept.
#[tokio::test]
async fn a_folder_behind_an_unavailable_link_keeps_its_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mods = dir.path().join("mods");
    crate::managers::server::world::link_folder(&dir.path().join("unplugged"), &mods);
    let mut listings = FolderListings::default();
    assert!(!listings.confirmed_absent(&mods, "a.jar").await);
    assert!(!listings.confirmed_absent(&mods.join("sub"), "a.jar").await);
    assert!(
        listings
            .confirmed_absent(&dir.path().join("missing"), "a.jar")
            .await
    );
}

async fn server_fixture(app: &App, name: &str) -> (i32, PathBuf) {
    let group = app.server_manager().get_default_group().await.unwrap();
    let id = carbon_repos::repos::server::insert_server(
        &app.db,
        name.into(),
        name.into(),
        0,
        group.0,
        "1.20.1".into(),
        25565,
        "fabric".into(),
        Some("fabric".into()),
        Some("0.15.0".into()),
        None,
        None,
        None,
        None,
        DbDateTime(Utc::now().fixed_offset()),
    )
    .await
    .unwrap() as i32;
    let data = app
        .settings_manager()
        .runtime_path
        .get_servers()
        .get_server_path(name)
        .get_data_path();
    std::fs::create_dir_all(data.join("mods")).unwrap();
    (id, data)
}

async fn server_scan(app: &App, id: i32, name: &str) {
    tokio::time::timeout(
        Duration::from_secs(10),
        app.meta_cache_manager().cache_server_local(id, name),
    )
    .await
    .expect("a server scan must settle")
    .unwrap();
}

async fn server_rows(app: &App, id: i32) -> Vec<mfcdb::ServerModFileCacheRow> {
    let mut rows = mfcdb::get_server_mod_files_by_server(&app.db, id)
        .await
        .unwrap();
    rows.sort_by(|a, b| a.location.cmp(&b.location));
    rows
}

#[tokio::test]
async fn server_rescan_reads_only_changed_files() {
    let app = crate::setup_managers_for_test().await;
    let (id, data) = server_fixture(&app, "scan-server-rescan").await;
    let (a, b) = (data.join("mods/a.jar"), data.join("mods/b.jar"));
    std::fs::write(&a, b"first a").unwrap();
    std::fs::write(&b, b"first b").unwrap();
    server_scan(&app, id, "scan-server-rescan").await;
    let before = server_rows(&app, id).await;
    assert_eq!(before.len(), 2);
    assert!(before.iter().all(|row| row.file_stamp.is_some()));

    let mcm = app.meta_cache_manager();
    mcm.clear_scan_observations();
    server_scan(&app, id, "scan-server-rescan").await;
    assert_eq!(mcm.scan_observations(&a, "hash"), 0);
    assert_eq!(mcm.scan_observations(&b, "hash"), 0);
    assert_eq!(server_rows(&app, id).await, before);

    // Same size, new bytes, new modification time.
    let modified = std::fs::metadata(&a).unwrap().modified().unwrap();
    std::fs::write(&a, b"other a").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&a)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified + Duration::from_secs(2)))
        .unwrap();
    mcm.clear_scan_observations();
    server_scan(&app, id, "scan-server-rescan").await;
    assert_eq!(mcm.scan_observations(&b, "hash"), 0);
    assert!(mcm.scan_observations(&a, "hash") >= 1);
    let after = server_rows(&app, id).await;
    assert_ne!(after[0].metadata_id, before[0].metadata_id);
    assert_eq!(after[0].id, before[0].id);
    assert_eq!(after[1], before[1]);
}

#[tokio::test]
async fn server_scan_releases_the_server_gate_while_hashing() {
    let app = crate::setup_managers_for_test().await;
    let (id, data) = server_fixture(&app, "scan-server-gate").await;
    let jar = data.join("mods/a.jar");
    std::fs::write(&jar, b"a").unwrap();
    let pause = app.meta_cache_manager().arm_scan_pause(&jar, "hash");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .cache_server_local(id, "scan-server-gate")
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the scan reaches the hash boundary");
    // Start, Stop, toggles and the addon list all take this gate.
    let gate = app
        .server_manager()
        .get_op_lock(crate::domain::server::ServerId(id));
    let guard = tokio::time::timeout(Duration::from_millis(100), gate.lock())
        .await
        .expect("the server gate is free while files are hashed");
    drop(guard);
    pause.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(server_rows(&app, id).await.len(), 1);
}

#[tokio::test]
async fn server_scan_keeps_ambiguous_and_unreadable_locations_and_prunes_the_rest() {
    let app = crate::setup_managers_for_test().await;
    let (id, data) = server_fixture(&app, "scan-server-tolerant").await;
    let mods = data.join("mods");
    for name in ["a.jar", "foo.jar", "gone.jar", "locked.jar"] {
        std::fs::write(mods.join(name), name).unwrap();
    }
    server_scan(&app, id, "scan-server-tolerant").await;
    let before = server_rows(&app, id).await;
    assert_eq!(before.len(), 4);
    let location = |rows: &[mfcdb::ServerModFileCacheRow], name: &str| {
        rows.iter()
            .find(|row| row.location == format!("mods/{name}"))
            .cloned()
    };

    // foo.jar is copied back next to a disabled copy.
    std::fs::write(mods.join("foo.jar.disabled"), b"foo.jar").unwrap();
    // locked.jar is touched (so it must be read) and then made unreadable.
    let locked = mods.join("locked.jar");
    let modified = std::fs::metadata(&locked).unwrap().modified().unwrap();
    std::fs::File::options()
        .write(true)
        .open(&locked)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified + Duration::from_secs(5)))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    let unreadable = std::fs::read(&locked).is_err();
    std::fs::remove_file(mods.join("gone.jar")).unwrap();
    std::fs::write(mods.join("new.jar"), b"new").unwrap();

    let scan = tokio::time::timeout(
        Duration::from_secs(10),
        app.meta_cache_manager()
            .cache_server_local(id, "scan-server-tolerant"),
    )
    .await
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    scan.expect("one duplicate or unreadable jar must not fail the scan");
    let after = server_rows(&app, id).await;
    assert_eq!(location(&after, "a.jar"), location(&before, "a.jar"));
    assert_eq!(
        location(&after, "foo.jar"),
        location(&before, "foo.jar"),
        "an enabled/disabled duplicate keeps its row"
    );
    if unreadable {
        assert_eq!(
            location(&after, "locked.jar"),
            location(&before, "locked.jar")
        );
    }
    assert!(
        location(&after, "gone.jar").is_none(),
        "a removed file is pruned"
    );
    assert!(
        location(&after, "new.jar").is_some(),
        "a new file is cached"
    );
}

#[tokio::test]
async fn server_scan_with_unusable_level_name_still_scans_mods_and_keeps_datapacks() {
    let app = crate::setup_managers_for_test().await;
    let (id, data) = server_fixture(&app, "scan-server-level").await;
    std::fs::write(data.join("mods/a.jar"), b"a").unwrap();
    std::fs::create_dir_all(data.join("world/datapacks")).unwrap();
    std::fs::write(data.join("world/datapacks/pack.zip"), b"pack").unwrap();
    server_scan(&app, id, "scan-server-level").await;
    let before = server_rows(&app, id).await;
    assert_eq!(before.len(), 2);

    std::fs::write(data.join("server.properties"), "level-name=../elsewhere\n").unwrap();
    std::fs::remove_file(data.join("mods/a.jar")).unwrap();
    std::fs::write(data.join("mods/b.jar"), b"b").unwrap();
    server_scan(&app, id, "scan-server-level").await;
    let after = server_rows(&app, id).await;
    assert!(
        after.iter().all(|row| row.location != "mods/a.jar"),
        "mods are still pruned"
    );
    assert!(
        after.iter().any(|row| row.location == "mods/b.jar"),
        "mods are still scanned"
    );
    assert_eq!(
        after
            .iter()
            .find(|row| row.location == "world/datapacks/pack.zip"),
        before
            .iter()
            .find(|row| row.location == "world/datapacks/pack.zip"),
        "datapack rows are kept while the world cannot be resolved"
    );
    // The Addons tab still lists the mods; only datapacks wait for the fix.
    let listed = app
        .server_manager()
        .list_server_addons(crate::domain::server::ServerId(id))
        .await
        .expect("an unusable level-name does not fail the addon list");
    assert_eq!(
        listed
            .iter()
            .map(|addon| addon.filename.as_str())
            .collect::<Vec<_>>(),
        vec!["b.jar"]
    );
}

/// A server whose `mods/` is shared with other servers and whose world is on
/// another drive (symlinks, and junctions on Windows), with a world name the
/// server saved escaped (`World \#2`), as 2.0.40 handled it: the scan lists
/// both folders through the links and prunes what is gone, toggling and
/// deleting act on the files there, a drive that is unplugged keeps the
/// rows, and the datapack recovery offer is empty rather than an error on
/// every visit to the Addons tab.
#[tokio::test]
async fn server_scan_and_addon_actions_follow_linked_folders() {
    use crate::domain::server::{ServerId, ServerState};
    use crate::managers::server::world::link_folder;
    let app = crate::setup_managers_for_test().await;
    let name = "scan-server-linked";
    let (id, data) = server_fixture(&app, name).await;
    app.server_manager.servers.write().await.insert(
        ServerId(id),
        crate::managers::server::ServerData {
            shortpath: name.into(),
            state: ServerState::Stopped { failed_task: None },
            handle: None,
            last_log_id: None,
        },
    );
    let elsewhere = app
        .settings_manager()
        .runtime_path
        .to_path_buf()
        .join("elsewhere");
    let shared = elsewhere.join("shared-mods");
    let world = elsewhere.join("World #2");
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::create_dir_all(world.join("datapacks")).unwrap();
    std::fs::write(shared.join("a.jar"), b"a").unwrap();
    std::fs::write(shared.join("b.jar"), b"b").unwrap();
    std::fs::write(world.join("datapacks/pack.zip"), b"pack").unwrap();
    std::fs::remove_dir(data.join("mods")).unwrap();
    link_folder(&shared, &data.join("mods"));
    link_folder(&world, &data.join("World #2"));
    std::fs::write(data.join("server.properties"), "level-name=World \\#2\n").unwrap();

    server_scan(&app, id, name).await;
    let locations = |rows: &[mfcdb::ServerModFileCacheRow]| {
        rows.iter()
            .map(|row| row.location.clone())
            .collect::<Vec<_>>()
    };
    let rows = server_rows(&app, id).await;
    assert_eq!(
        locations(&rows),
        vec!["World #2/datapacks/pack.zip", "mods/a.jar", "mods/b.jar"]
    );
    std::fs::remove_file(shared.join("b.jar")).unwrap();
    server_scan(&app, id, name).await;
    let rows = server_rows(&app, id).await;
    assert_eq!(
        locations(&rows),
        vec!["World #2/datapacks/pack.zip", "mods/a.jar"]
    );
    let listed = app
        .server_manager()
        .list_server_addons(ServerId(id))
        .await
        .unwrap();
    assert_eq!(listed.len(), 2);

    let row = |location: &str| {
        rows.iter()
            .find(|row| row.location == location)
            .unwrap()
            .id
            .clone()
    };
    let servers = app.server_manager();
    servers
        .enable_server_addon(ServerId(id), row("mods/a.jar"), false)
        .await
        .unwrap();
    assert!(shared.join("a.jar.disabled").is_file());
    servers
        .enable_server_addon(ServerId(id), row("mods/a.jar"), true)
        .await
        .unwrap();
    assert!(shared.join("a.jar").is_file());
    servers
        .delete_server_addon(ServerId(id), row("World #2/datapacks/pack.zip"))
        .await
        .unwrap();
    assert!(!world.join("datapacks/pack.zip").exists());
    assert!(world.join("datapacks").is_dir());

    // The shared mods are on a drive that is not plugged in: the folder is
    // not known to be empty, so its rows stay.
    let unplugged = elsewhere.join("unplugged-mods");
    std::fs::rename(&shared, &unplugged).unwrap();
    server_scan(&app, id, name).await;
    assert_eq!(locations(&server_rows(&app, id).await), vec!["mods/a.jar"]);
    std::fs::rename(&unplugged, &shared).unwrap();

    assert!(
        servers
            .legacy_datapacks(ServerId(id))
            .await
            .unwrap()
            .is_empty()
    );
    std::fs::write(data.join("server.properties"), "level-name=../elsewhere\n").unwrap();
    assert!(
        servers
            .legacy_datapacks(ServerId(id))
            .await
            .unwrap()
            .is_empty(),
        "an unusable world name leaves the offer empty"
    );
}

/// A world named `SMP: Season 2`, saved by the server as
/// `level-name=SMP\: Season 2`, keeps its datapacks working outside Windows,
/// where `:` is an ordinary character in a folder name.
#[cfg(not(windows))]
#[tokio::test]
async fn a_world_name_with_a_colon_keeps_its_datapacks_working() {
    use crate::domain::server::{ServerId, ServerState};
    let app = crate::setup_managers_for_test().await;
    let name = "scan-server-colon";
    let (id, data) = server_fixture(&app, name).await;
    app.server_manager.servers.write().await.insert(
        ServerId(id),
        crate::managers::server::ServerData {
            shortpath: name.into(),
            state: ServerState::Stopped { failed_task: None },
            handle: None,
            last_log_id: None,
        },
    );
    let datapacks = data.join("SMP: Season 2/datapacks");
    std::fs::create_dir_all(&datapacks).unwrap();
    std::fs::write(datapacks.join("pack.zip"), b"pack").unwrap();
    std::fs::write(
        data.join("server.properties"),
        "level-name=SMP\\: Season 2\n",
    )
    .unwrap();

    server_scan(&app, id, name).await;
    let rows = server_rows(&app, id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].location, "SMP: Season 2/datapacks/pack.zip");
    let servers = app.server_manager();
    let listed = servers.list_server_addons(ServerId(id)).await.unwrap();
    assert_eq!(listed.len(), 1);
    servers
        .enable_server_addon(ServerId(id), rows[0].id.clone(), false)
        .await
        .unwrap();
    assert!(datapacks.join("pack.zip.disabled").is_file());
    servers
        .delete_server_addon(ServerId(id), rows[0].id.clone())
        .await
        .unwrap();
    assert!(!datapacks.join("pack.zip.disabled").exists());
}

/// Files read so far are listed while the scan runs, and a removed jar is
/// pruned before anything is read.
#[tokio::test]
async fn server_scan_commits_what_it_read_while_it_runs() {
    let app = crate::setup_managers_for_test().await;
    let (id, data) = server_fixture(&app, "scan-server-batches").await;
    let gone = data.join("mods/gone.jar");
    std::fs::write(&gone, b"gone").unwrap();
    server_scan(&app, id, "scan-server-batches").await;
    std::fs::remove_file(&gone).unwrap();
    let mut others = Vec::new();
    for i in 0..10 {
        let path = data.join(format!("mods/mod-{i}.jar"));
        std::fs::write(&path, format!("mod {i}")).unwrap();
        others.push(path);
    }
    let slow = data.join("mods/slow.jar");
    std::fs::write(&slow, b"slow").unwrap();
    let mcm = app.meta_cache_manager();
    let pause = mcm.arm_scan_pause(&slow, "hash");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .cache_server_local(id, "scan-server-batches")
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .expect("the scan reaches the paused file");
    assert!(
        server_rows(&app, id)
            .await
            .iter()
            .all(|row| row.location != "mods/gone.jar"),
        "a removed jar is pruned before anything is read"
    );
    let concurrent = available_parallelism().map_or(1, |n| n.get()) / 2 >= 2;
    eventually("files read so far are listed while the scan runs", || {
        let expected = others
            .iter()
            .filter(|path| mcm.scan_observations(path, "hash") > 0)
            .map(|path| format!("mods/{}", path.file_name().unwrap().to_string_lossy()))
            .collect::<Vec<_>>();
        let app = &app;
        async move {
            let current = server_rows(app, id).await;
            (!concurrent || expected.len() == 10)
                && expected
                    .iter()
                    .all(|location| current.iter().any(|row| &row.location == location))
        }
    })
    .await;
    pause.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(server_rows(&app, id).await.len(), 11);
}

/// A jar whose Fabric metadata names an icon. Creating its metadata scales
/// the icon, which takes the launcher-wide icon permit.
fn icon_jar(id: &str) -> Vec<u8> {
    use std::io::Write;
    let mut icon = Vec::new();
    image::RgbaImage::new(4, 4)
        .write_to(
            &mut std::io::Cursor::new(&mut icon),
            image::ImageFormat::Png,
        )
        .unwrap();
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("fabric.mod.json", options).unwrap();
    zip.write_all(
        format!(r#"{{"schemaVersion":1,"id":"{id}","version":"1.0.0","icon":"icon.png"}}"#)
            .as_bytes(),
    )
    .unwrap();
    zip.start_file("icon.png", options).unwrap();
    zip.write_all(&icon).unwrap();
    zip.finish().unwrap().into_inner()
}

async fn entered(pause: &ScanPause, what: &str) {
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
        .await
        .unwrap_or_else(|_| panic!("{what}"));
}

fn spawn_instance_pass(app: &App, id: InstanceId) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let app = app.clone();
    tokio::spawn(async move {
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
    })
}

/// An install publishes its file under the instance gate and creates its
/// metadata there, which takes the icon permit. A pass whose batch is waiting
/// for that gate keeps its own work going, so the work holding the permit
/// finishes and lets the install through, and then the pass.
#[tokio::test]
async fn an_install_gets_the_icon_permit_while_a_scan_batch_waits_for_the_gate() {
    let app = crate::setup_managers_for_test().await;
    let (id, kept) = addon_fixture(
        &app,
        "scan-permit-install",
        AddonType::Mods,
        "a-kept.jar",
        b"kept",
    )
    .await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let mods = kept.parent().unwrap().to_owned();
    // A changed file is read first and makes a batch due; a new jar with an
    // icon holds the permit while it is scaled.
    std::fs::write(&kept, b"kept, changed since the last scan").unwrap();
    std::fs::write(mods.join("b-icon.jar"), icon_jar("scanned")).unwrap();

    let mcm = app.meta_cache_manager();
    let holding = mcm.arm_scan_pause(Path::new("b-icon.jar"), "icon-scale");
    let commit = mcm.arm_scan_pause(Path::new(&format!("instance-{}", *id)), "commit");
    let scan = spawn_instance_pass(&app, id);
    entered(&holding, "the scan holds the icon permit").await;
    entered(&commit, "a batch is due").await;

    // The install holds the gate, then needs the permit for its own icon.
    std::fs::write(mods.join("installed.jar"), icon_jar("installed")).unwrap();
    let (held, gate_held) = tokio::sync::oneshot::channel();
    let install_app = app.clone();
    let install_mods = mods.clone();
    let install = tokio::spawn(async move {
        let gate = install_app.instance_manager().lock_instance(id).await;
        held.send(()).unwrap();
        install_app
            .meta_cache_manager()
            .refresh_published_addon(
                &gate,
                &install_mods,
                "installed.jar".into(),
                true,
                "mods".into(),
            )
            .await
    });
    gate_held.await.unwrap();
    // The batch now waits for the gate the install holds...
    commit.release.notify_one();
    tokio::time::sleep(Duration::from_millis(100)).await;
    // ...while the work holding the permit goes on.
    holding.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), install)
        .await
        .expect("the install gets the icon permit")
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .expect("the scan commits once the install is done")
        .unwrap()
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(1),
        app.instance_manager().lock_instance(id),
    )
    .await
    .expect("the instance gate is free");
    let after = rows(&app, id).await;
    assert_eq!(
        after
            .iter()
            .map(|row| row.filename.as_str())
            .collect::<Vec<_>>(),
        vec!["a-kept.jar", "b-icon.jar", "installed.jar"]
    );
}

/// Deleting an addon or an instance collects metadata no row owns yet,
/// including what a running pass created for results it has not committed.
/// The pass creates it again with the gate released (it may have to wait for
/// the icon permit) and still stores the result.
#[tokio::test]
async fn a_scan_result_whose_metadata_was_collected_is_stored_without_holding_the_gate() {
    let app = crate::setup_managers_for_test().await;
    let (id, changed) = addon_fixture(
        &app,
        "scan-permit-collected",
        AddonType::Mods,
        "a-icon.jar",
        &icon_jar("first"),
    )
    .await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = row(&rows(&app, id).await, "a-icon.jar").clone();
    let mods = changed.parent().unwrap().to_owned();
    std::fs::write(&changed, icon_jar("first-updated")).unwrap();
    let new = mods.join("b-icon.jar");
    std::fs::write(&new, icon_jar("other")).unwrap();

    let mcm = app.meta_cache_manager();
    let hashing = mcm.arm_scan_pause(&new, "hash");
    let commit = mcm.arm_scan_pause(Path::new(&format!("instance-{}", *id)), "commit");
    let scan = spawn_instance_pass(&app, id);
    entered(&commit, "a-icon.jar's result is due").await;
    // The pass's work goes on while the batch is held: b-icon.jar now takes
    // the icon permit.
    let holding = mcm.arm_scan_pause(Path::new("b-icon.jar"), "icon-scale");
    entered(&hashing, "b-icon.jar is being read").await;
    hashing.release.notify_one();
    entered(&holding, "b-icon.jar holds the icon permit").await;

    mcm.gc_mod_metadata().await;
    commit.release.notify_one();
    tokio::time::sleep(Duration::from_millis(100)).await;
    tokio::time::timeout(
        Duration::from_secs(1),
        app.instance_manager().lock_instance(id),
    )
    .await
    .expect("metadata is never created while the gate is held");
    holding.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .expect("the scan finishes")
        .unwrap()
        .unwrap();
    let after = rows(&app, id).await;
    let updated = row(&after, "a-icon.jar");
    assert_eq!(updated.id, before.id);
    assert_ne!(updated.metadata_id, before.metadata_id);
    assert!(
        metarepo::get_metadata_by_id(&app.db, &updated.metadata_id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(after.iter().any(|row| row.filename == "b-icon.jar"));
}

/// A pack updated by hand: the old jars are deleted and their replacements
/// copied in. The old entries leave the list before anything is read, so it
/// never shows both versions while the new files are read, nor keeps the old
/// ones through a play session when Play pauses the pass.
#[tokio::test]
async fn removed_addons_leave_the_list_before_anything_is_read() {
    let app = crate::setup_managers_for_test().await;
    let (id, old) = addon_fixture(
        &app,
        "scan-removed-first",
        AddonType::Mods,
        "jei-1.0.jar",
        b"jei 1.0",
    )
    .await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let mods = old.parent().unwrap().to_owned();
    std::fs::remove_file(&old).unwrap();
    std::fs::write(mods.join("jei-1.1.jar"), b"jei 1.1").unwrap();
    for i in 0..(SCAN_COMMIT_BATCH + 8) {
        std::fs::write(mods.join(format!("mod-{i:02}.jar")), format!("mod {i}")).unwrap();
    }
    // Files are read in name order; this one is read last.
    let slow = mods.join("zz-slow.jar");
    std::fs::write(&slow, b"slow").unwrap();
    let pause = app.meta_cache_manager().arm_scan_pause(&slow, "hash");
    let scan = spawn_instance_pass(&app, id);
    entered(&pause, "the pass reaches the last file").await;
    assert!(
        rows(&app, id)
            .await
            .iter()
            .all(|row| row.filename != "jei-1.0.jar"),
        "the removed jar is gone before its replacement is read"
    );
    eventually(
        "files read so far are listed while the pass runs",
        || async { rows(&app, id).await.len() > SCAN_COMMIT_BATCH },
    )
    .await;
    pause.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(rows(&app, id).await.len(), SCAN_COMMIT_BATCH + 10);
}

/// The server version of the install test above: a server addon install
/// holds the server gate (Start, Stop and Kill wait on it) while it creates
/// the new addon's metadata.
#[tokio::test]
async fn a_server_install_gets_the_icon_permit_while_a_scan_batch_waits_for_the_gate() {
    let app = crate::setup_managers_for_test().await;
    let (id, data) = server_fixture(&app, "scan-server-permit-install").await;
    std::fs::write(data.join("mods/a-plain.jar"), b"plain").unwrap();
    std::fs::write(data.join("mods/b-icon.jar"), icon_jar("scanned")).unwrap();
    let staged = data.join("staged-installed.jar");
    std::fs::write(&staged, icon_jar("installed")).unwrap();

    let mcm = app.meta_cache_manager();
    let holding = mcm.arm_scan_pause(Path::new("b-icon.jar"), "icon-scale");
    let commit = mcm.arm_scan_pause(Path::new(&format!("server-{id}")), "commit");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .cache_server_local(id, "scan-server-permit-install")
            .await
    });
    entered(&holding, "the scan holds the icon permit").await;
    entered(&commit, "a batch is due").await;

    let (held, gate_held) = tokio::sync::oneshot::channel();
    let install_app = app.clone();
    let install = tokio::spawn(async move {
        let gate = install_app
            .server_manager()
            .get_op_lock(crate::domain::server::ServerId(id));
        let _guard = gate.lock().await;
        held.send(()).unwrap();
        install_app
            .meta_cache_manager()
            .prepare_server_addon_metadata(&staged, "installed.jar")
            .await
    });
    gate_held.await.unwrap();
    commit.release.notify_one();
    tokio::time::sleep(Duration::from_millis(100)).await;
    holding.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), install)
        .await
        .expect("the install gets the icon permit")
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .expect("the scan commits once the install is done")
        .unwrap()
        .unwrap();
    let gate = app
        .server_manager()
        .get_op_lock(crate::domain::server::ServerId(id));
    tokio::time::timeout(Duration::from_secs(1), gate.lock())
        .await
        .expect("Start, Stop and Kill can take the server gate");
    assert_eq!(server_rows(&app, id).await.len(), 2);
}

/// The server version of the collected-metadata test above: removing a
/// server addon collects metadata a running scan has not committed yet.
#[tokio::test]
async fn a_server_scan_result_whose_metadata_was_collected_is_stored_without_holding_the_gate() {
    let app = crate::setup_managers_for_test().await;
    let name = "scan-server-permit-collected";
    let (id, data) = server_fixture(&app, name).await;
    let changed = data.join("mods/a-icon.jar");
    std::fs::write(&changed, icon_jar("first")).unwrap();
    server_scan(&app, id, name).await;
    let before = server_rows(&app, id).await[0].clone();
    std::fs::write(&changed, icon_jar("first-updated")).unwrap();
    let new = data.join("mods/b-icon.jar");
    std::fs::write(&new, icon_jar("other")).unwrap();

    let mcm = app.meta_cache_manager();
    let hashing = mcm.arm_scan_pause(&new, "hash");
    let commit = mcm.arm_scan_pause(Path::new(&format!("server-{id}")), "commit");
    let scan_app = app.clone();
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .cache_server_local(id, "scan-server-permit-collected")
            .await
    });
    entered(&commit, "a-icon.jar's result is due").await;
    let holding = mcm.arm_scan_pause(Path::new("b-icon.jar"), "icon-scale");
    entered(&hashing, "b-icon.jar is being read").await;
    hashing.release.notify_one();
    entered(&holding, "b-icon.jar holds the icon permit").await;

    mcm.gc_mod_metadata().await;
    commit.release.notify_one();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let gate = app
        .server_manager()
        .get_op_lock(crate::domain::server::ServerId(id));
    drop(
        tokio::time::timeout(Duration::from_secs(1), gate.lock())
            .await
            .expect("metadata is never created while the server gate is held"),
    );
    holding.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), scan)
        .await
        .expect("the scan finishes")
        .unwrap()
        .unwrap();
    let after = server_rows(&app, id).await;
    assert_eq!(after.len(), 2);
    assert_eq!(after[0].id, before.id);
    assert_ne!(after[0].metadata_id, before.metadata_id);
    assert!(
        metarepo::get_metadata_by_id(&app.db, &after[0].metadata_id)
            .await
            .unwrap()
            .is_some()
    );
}

/// A data folder on an external or network drive that is mounted again
/// under another device number keeps every stamp, and so does one on a FAT or
/// exFAT drive, whose files Linux numbers again after every reboot or
/// remount: neither number is part of a stamp, so nothing is read again.
#[cfg(unix)]
#[test]
fn file_stamps_leave_out_the_device_and_inode_numbers() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.jar");
    std::fs::write(&path, b"a").unwrap();
    let metadata = std::fs::metadata(&path).unwrap();
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok());
    assert_eq!(
        observed_file_stamp(&metadata),
        format!(
            "{:?}:{}:{}:{}",
            modified,
            metadata.len(),
            metadata.ctime(),
            metadata.ctime_nsec()
        )
    );
}

/// A new addon is read twice to hash and parse it: once for its hashes and
/// the murmur2 length, once for murmur2 and a SHA-512 that confirms both
/// reads saw the same bytes. There is no third read.
#[tokio::test]
async fn a_new_addon_is_read_twice() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(&app, "scan-two-reads", AddonType::Mods, "a.jar", b"a").await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let body = r#"{"schemaVersion":1,"id":"example","version":"1.0.0"}"#;
    let new = first.with_file_name("new.jar");
    std::fs::write(&new, metadata_jar("fabric.mod.json", body)).unwrap();
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    assert_eq!(whole_file_reads(&new), 2);
    let rows = rows(&app, id).await;
    assert!(qualified(row(&rows, "new.jar")));
    let metadata = metarepo::get_metadata_by_id(&app.db, &row(&rows, "new.jar").metadata_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(metadata.modid.as_deref(), Some("example"));
}

/// A file rewritten between the two reads of its hash and parse is not stored
/// with the first read's hashes, even when its stamp looks the same, as on
/// Windows, which has no change time.
#[tokio::test]
async fn an_addon_rewritten_between_its_two_reads_is_not_stored() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(&app, "scan-rewritten", AddonType::Mods, "a.jar", b"a").await;
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let new = first.with_file_name("new.jar");
    std::fs::write(&new, b"first bytes").unwrap();
    let mcm = app.meta_cache_manager();
    let parsing = mcm.arm_scan_pause(&new, "parse");
    let scan = spawn_instance_pass(&app, id);
    entered(&parsing, "the parse of new.jar").await;
    let modified = std::fs::metadata(&new).unwrap().modified().unwrap();
    std::fs::write(&new, b"other bytes").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&new)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    parsing.release.notify_one();
    tokio::time::timeout(Duration::from_secs(10), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // The next pass reads the file as it is now.
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let rows = rows(&app, id).await;
    let metadata = metarepo::get_metadata_by_id(&app.db, &row(&rows, "new.jar").metadata_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(metadata.sha512, Sha512::digest(b"other bytes").to_vec());
}

/// Upgraded rows are stamped together, one commit per batch, and a file
/// changed after it was settled keeps no stamp while the others get theirs.
#[tokio::test]
async fn upgraded_rows_are_stamped_together_except_files_changed_meanwhile() {
    let app = crate::setup_managers_for_test().await;
    let (id, first) = addon_fixture(
        &app,
        "scan-upgrade-batch",
        AddonType::Mods,
        "mod-0.jar",
        b"0",
    )
    .await;
    let mods = first.parent().unwrap().to_owned();
    let mut paths = vec![first];
    for i in 1..6 {
        let path = mods.join(format!("mod-{i}.jar"));
        std::fs::write(&path, format!("{i}")).unwrap();
        paths.push(path);
    }
    pass(&app, id, VerifyScope::NONE).await.unwrap();
    let before = rows(&app, id).await;
    let written = DbDateTime(Utc::now().fixed_offset());
    for row in &before {
        mfcdb::update_mod_file_stamp(
            &app.db,
            *id,
            &row.addon_type,
            &row.filename,
            &row.metadata_id,
            None,
            written,
        )
        .await
        .unwrap();
    }
    let mcm = app.meta_cache_manager();
    let commit = mcm.arm_scan_pause(Path::new(&format!("instance-{}", *id)), "commit");
    let scan = spawn_instance_pass(&app, id);
    entered(&commit, "the commit of the settled rows").await;
    // Touched after it was settled: its observed stamp is no longer current.
    let touched = &paths[3];
    let modified = std::fs::metadata(touched).unwrap().modified().unwrap();
    std::fs::File::options()
        .write(true)
        .open(touched)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified + Duration::from_secs(5)))
        .unwrap();
    commit.release.notify_one();
    tokio::time::timeout(Duration::from_secs(10), scan)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let after = rows(&app, id).await;
    for row in &after {
        if row.filename == "mod-3.jar" {
            assert_eq!(row.file_stamp, None, "a changed file keeps no stamp");
        } else {
            assert!(qualified(row), "{} gets its stamp", row.filename);
        }
    }
}
