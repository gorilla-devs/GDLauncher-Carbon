use super::*;
use std::io::Write;

fn cf(filename: &str, bytes: &[u8], kind: curseforge::ClassId) -> CurseforgeModInstaller {
    use sha1::Digest;
    let file = serde_json::from_value(serde_json::json!({
        "id": 2, "gameId": 432, "modId": 10, "isAvailable": true,
        "displayName": "fixture", "fileName": filename, "releaseType": 1,
        "fileStatus": 4, "hashes": [{"algo": 1, "value": hex::encode(sha1::Sha1::digest(bytes))}],
        "fileDate": "2026-01-01T00:00:00Z", "fileLength": bytes.len(), "downloadCount": 0,
        "downloadUrl": "https://fixture.invalid/never-requested", "gameVersions": ["1.20.1"],
        "sortableGameVersions": [], "dependencies": [], "fileFingerprint": 0
    }))
    .unwrap();
    CurseforgeModInstaller {
        addon_type: kind,
        file,
        download_url: "https://fixture.invalid/never-requested".into(),
        applied_data: Arc::new(Mutex::new(None)),
    }
}

fn mr(filename: &str, bytes: &[u8]) -> ModrinthModInstaller {
    use sha1::Digest;
    let version = serde_json::from_value(serde_json::json!({
        "name":"fixture", "version_number":"2", "dependencies":[], "game_versions":["1.20.1"],
        "version_type":"release", "loaders":["fabric"], "featured":false, "id":"v2", "project_id":"fixture",
        "author_id":"fixture", "date_published":"2026-01-01T00:00:00Z", "downloads":0,
        "files":[{"hashes":{"sha1":hex::encode(sha1::Sha1::digest(bytes)), "sha512":hex::encode(sha2::Sha512::digest(bytes))},
            "url":"https://fixture.invalid/never-requested", "filename":filename, "primary":true, "size":bytes.len()}]
    })).unwrap();
    let project = serde_json::from_value(serde_json::json!({
        "slug":"fixture", "title":"fixture", "description":"fixture", "categories":[], "client_side":"required", "server_side":"optional",
        "body":"fixture", "additional_categories":[], "donation_urls":[], "project_type":"mod", "downloads":0, "id":"fixture", "team":"fixture",
        "published":"2026-01-01T00:00:00Z", "updated":"2026-01-01T00:00:00Z", "followers":0, "status":"approved",
        "license":{"id":"MIT", "name":"MIT", "url":null}, "versions":["v2"], "game_versions":["1.20.1"], "loaders":["fabric"]
    })).unwrap();
    ModrinthModInstaller::from_version(version, project).unwrap()
}

#[tokio::test]
async fn task05_replace_modrinth_checks_real_artifact_and_kind() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "mr",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    let resource = mr("same.jar", b"v2");
    assert!(!resource.is_already_installed(&app, id).await.unwrap());
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    app.instance_manager()
        .enable_mod(id, row.id.clone(), false)
        .await
        .unwrap();
    let stage = tempfile::tempdir().unwrap();
    let bytes = stage.path().join("artifact");
    std::fs::write(&bytes, b"v2").unwrap();
    publication::publish_file(&resource, &app, id, &bytes, Some(&plan), None)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(path.with_file_name("same.jar.disabled")).unwrap(),
        b"v2"
    );
    assert!(resource.is_already_installed(&app, id).await.unwrap());
    let current = mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
        .await
        .unwrap()
        .unwrap();
    mfcdb::upsert_mod_file_cache(
        &app.db,
        *id,
        "same.jar".into(),
        2,
        false,
        "resourcepacks".into(),
        current.metadata_id,
        carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().fixed_offset()),
    )
    .await
    .unwrap();
    assert!(!resource.is_already_installed(&app, id).await.unwrap());
}

/// Staging lives in the data folder; an instance junctioned or linked to
/// another drive makes both renames of a replacement cross volumes. They
/// become copies, and the receipt must still own (and commit) what landed.
#[tokio::test]
async fn cross_volume_replacement_publishes_copies_and_commits() {
    use crate::util::publish::fault::{CROSS_DEVICE, Step, inject, pending};
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "cross-volume",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    let resource = mr("same.jar", b"v2");
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let stage = tempfile::tempdir().unwrap();
    let bytes = stage.path().join("artifact");
    std::fs::write(&bytes, b"v2").unwrap();
    let backup = stage.path().join("previous-addon");
    inject(Step::NoReplace, &backup, CROSS_DEVICE);
    inject(Step::NoReplace, &path, CROSS_DEVICE);
    publication::publish_file(&resource, &app, id, &bytes, Some(&plan), None)
        .await
        .unwrap();
    // Both renames really crossed volumes: each injected fault was taken.
    assert!(!pending(Step::NoReplace, &backup));
    assert!(!pending(Step::NoReplace, &path));
    assert_eq!(std::fs::read(&path).unwrap(), b"v2");
    assert!(!bytes.exists());
    assert!(!backup.exists(), "committed replacement retires its backup");
    assert!(resource.is_already_installed(&app, id).await.unwrap());
}

fn zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
    let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

#[tokio::test]
async fn task05_replace_same_filename_distinct_artifact_is_not_satisfied() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "identity",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    let installer = cf("same.jar", b"v2", curseforge::ClassId::Mods);
    assert!(!installer.is_already_installed(&app, id).await.unwrap());
}

#[tokio::test]
async fn task05_replace_disabled_different_filename_stays_disabled() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "disabled",
        instance::AddonType::Mods,
        "old.jar",
        b"v1",
    )
    .await;
    app.instance_manager()
        .enable_mod(id, row.id.clone(), false)
        .await
        .unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"v2").unwrap();
    let mut installer = cf("new.jar", b"v2", curseforge::ClassId::Mods).into_installer();
    installer.download_fixture = Some(source.path().into());
    installer
        .install(&app, id, false, Some(row.id.clone()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if installer
                .abort_handle
                .lock()
                .await
                .handle
                .as_ref()
                .unwrap()
                .is_finished()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(path.with_file_name("new.jar.disabled")).unwrap(),
        b"v2"
    );
    assert!(!path.with_file_name("new.jar").exists());
}

#[tokio::test]
async fn task05_shader_explicit_intent_enables_exact_disabled_artifact() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "shader",
        instance::AddonType::Mods,
        "iris.jar",
        b"iris",
    )
    .await;
    app.instance_manager()
        .enable_mod(id, row.id, false)
        .await
        .unwrap();
    let mut installer = cf("iris.jar", b"iris", curseforge::ClassId::Mods).into_installer();
    installer.required_enabled = Some(super::super::shader_loader::ShaderLoaderVerification {
        instance_id: *id,
        mc_version: "1.20.1".into(),
        modloaders: vec![],
        filename: "iris.jar".into(),
        sha1: {
            use sha1::Digest;
            hex::encode(sha1::Sha1::digest(b"iris"))
        },
        size: 4,
    });
    let verification = installer.required_enabled.clone().unwrap();
    assert!(
        !app.instance_manager()
            .verify_shader_loader(verification.clone())
            .await
            .unwrap()
    );
    installer.required_enabled.as_mut().unwrap().mc_version = "wrong-version".into();
    assert!(installer.install(&app, id, false, None).await.is_err());
    assert!(!path.exists());
    installer.required_enabled = Some(verification.clone());
    installer.install(&app, id, false, None).await.unwrap();
    assert!(
        app.instance_manager()
            .verify_shader_loader(verification.clone())
            .await
            .unwrap()
    );
    let mut wrong = verification.clone();
    wrong.sha1 = "00".repeat(20);
    assert!(
        !app.instance_manager()
            .verify_shader_loader(wrong)
            .await
            .unwrap()
    );
    let mut changed_target = verification;
    changed_target.mc_version = "1.21".into();
    assert!(
        app.instance_manager()
            .verify_shader_loader(changed_target)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"iris");
    assert!(!path.with_file_name("iris.jar.disabled").exists());
}

#[tokio::test]
async fn task05_replace_late_destination_keeps_foreign_bytes_and_backup() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "late",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let staging = tempfile::tempdir().unwrap();
    let staged = staging.path().join("artifact");
    std::fs::write(&staged, b"v2").unwrap();
    let pause = app
        .instance_manager()
        .test_hooks
        .arm("addon_before_publish", id);
    let owned_app = Arc::clone(&app);
    let worker = tokio::spawn(async move {
        let resource = cf("same.jar", b"v2", curseforge::ClassId::Mods);
        publication::publish_file(&resource, &owned_app, id, &staged, Some(&plan), None).await
    });
    pause.entered.notified().await;
    assert!(!old.exists());
    std::fs::write(&old, b"late user file").unwrap();
    pause.release.notify_one();
    assert!(worker.await.unwrap().is_err());
    assert_eq!(std::fs::read(old).unwrap(), b"late user file");
    assert_eq!(
        std::fs::read(staging.path().join("previous-addon")).unwrap(),
        b"v1"
    );
    assert!(
        mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn task05_replace_scanner_finishes_before_owned_publication_and_refresh() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "scan",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    let staging = tempfile::tempdir().unwrap();
    let staged = staging.path().join("artifact");
    std::fs::write(&staged, b"v2").unwrap();
    // First refresh gives the old cache a true hash rather than fixture metadata.
    let manager = app.instance_manager();
    let guard = manager.lock_instance(id).await;
    app.meta_cache_manager()
        .refresh_published_addon(
            &guard,
            &old.parent().unwrap().to_path_buf(),
            "same.jar".into(),
            true,
            "mods".into(),
        )
        .await
        .unwrap();
    drop(guard);
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let pause = manager.test_hooks.arm("addon_scan_owned", id);
    let scan_app = Arc::clone(&app);
    let scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .override_caching_and_wait(
                crate::managers::metadata::cache::CacheEntityId::Instance(id),
                false,
                false,
            )
            .await
    });
    pause.entered.notified().await;
    let publish_app = Arc::clone(&app);
    let mut publish = tokio::spawn(async move {
        publication::publish_file(
            &cf("same.jar", b"v2", curseforge::ClassId::Mods),
            &publish_app,
            id,
            &staged,
            Some(&plan),
            None,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut publish)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&old).unwrap(), b"v1");
    pause.release.notify_one();
    scan.await.unwrap().unwrap();
    publish.await.unwrap().unwrap();
    let current = mfcdb::get_instance_mods_full(&app.db, *id).await.unwrap();
    use sha1::Digest;
    assert_eq!(current[0].sha1, sha1::Sha1::digest(b"v2").to_vec());
    assert_eq!(std::fs::read(old).unwrap(), b"v2");
}

#[tokio::test]
async fn task05_world_scanner_waits_for_reservation_and_skips_owned_incomplete_marker() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "world-scan",
        instance::AddonType::Mods,
        "base.jar",
        b"base",
    )
    .await;
    let manager = app.instance_manager();
    let guard = manager.lock_instance(id).await;
    let path = publication::current_path(&app, id).await.unwrap();
    let world = path.get_saves_path().join("NewWorld");
    let archive = tempfile::NamedTempFile::new().unwrap();
    zip(archive.path(), &[("NewWorld/level.dat", b"complete")]);
    let prepared = publication::prepare_world(archive.path(), "archive")
        .await
        .unwrap();
    let reserved = prepared.reserve(&path.get_saves_path()).await.unwrap();
    assert!(publication::world_install_incomplete(&world).await.unwrap());
    let scan_app = Arc::clone(&app);
    let mut scan = tokio::spawn(async move {
        scan_app
            .meta_cache_manager()
            .override_caching_and_wait(
                crate::managers::metadata::cache::CacheEntityId::Instance(id),
                false,
                false,
            )
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut scan)
            .await
            .is_err()
    );
    drop(guard);
    scan.await.unwrap().unwrap();
    assert!(
        mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, "NewWorld")
            .await
            .unwrap()
            .is_none()
    );
    let guard = manager.lock_instance(id).await;
    reserved.finish().await.unwrap();
    drop(guard);
    app.meta_cache_manager()
        .override_caching_and_wait(
            crate::managers::metadata::cache::CacheEntityId::Instance(id),
            false,
            false,
        )
        .await
        .unwrap();
    assert!(
        mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, "NewWorld")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn task05_world_late_arrival_and_partial_failure_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("archive.zip");
    zip(
        &archive,
        &[
            ("World/level.dat", b"download"),
            ("World/region/a.mca", b"region"),
        ],
    );
    let saves = dir.path().join("saves");
    let prepared = publication::prepare_world(&archive, "archive")
        .await
        .unwrap();
    std::fs::create_dir_all(saves.join("World")).unwrap();
    std::fs::write(saves.join("World/level.dat"), b"late").unwrap();
    assert!(prepared.reserve(&saves).await.is_err());
    assert_eq!(
        std::fs::read(saves.join("World/level.dat")).unwrap(),
        b"late"
    );
    let other = dir.path().join("other-saves");
    let reserved = publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&other)
        .await
        .unwrap();
    let target = reserved.destination.clone();
    std::fs::write(target.join("level.dat"), b"arrived after reservation").unwrap();
    assert!(reserved.finish().await.is_err());
    assert_eq!(
        std::fs::read(target.join("level.dat")).unwrap(),
        b"arrived after reservation"
    );
    assert!(
        publication::world_install_incomplete(&target)
            .await
            .unwrap()
    );
    // Restart/retry never treats the marker as overwrite permission.
    assert!(
        publication::prepare_world(&archive, "archive")
            .await
            .unwrap()
            .reserve(&other)
            .await
            .is_err()
    );
}

/// World files copied onto such a share (saves on another drive) land as
/// well: a copied world file gets no flush of its own, as on 2.0.40, so a
/// refused full flush cannot stop it.
#[cfg(unix)]
#[tokio::test]
async fn copied_world_files_land_where_the_drive_refuses_a_full_flush() {
    use crate::util::publish::fault::{CROSS_DEVICE, Step, inject, pending};
    let syncs = crate::util::durable::test_hooks::FileSyncs::refusing_full_flush();
    let dir = tempfile::tempdir().unwrap();
    let reserved = reserve_refusing_world(dir.path()).await;
    let world = dir.path().join("saves").join("World");
    // The first file can't be moved there, so every file is copied.
    inject(Step::NoReplace, &world.join("level.dat"), CROSS_DEVICE);
    reserved.finish().await.unwrap();
    assert!(!pending(Step::NoReplace, &world.join("level.dat")));
    assert!(!publication::world_install_incomplete(&world).await.unwrap());
    for (file, bytes) in [
        ("level.dat", b"level".as_slice()),
        ("region/r.0.0.mca", b"region"),
    ] {
        assert_eq!(std::fs::read(world.join(file)).unwrap(), bytes);
        assert!(!syncs.synced(&world.join(file)), "{file}");
    }
}

#[tokio::test]
async fn task05_world_download_filename_never_occupies_a_save_path() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, existing) = super::super::mods::tests::addon_fixture(
        &app,
        "zip-name",
        instance::AddonType::Worlds,
        "download.zip",
        b"user world",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    zip(source.path(), &[("NewWorld/level.dat", b"new world")]);
    let contents = std::fs::read(source.path()).unwrap();
    let mut installer = cf("download.zip", &contents, curseforge::ClassId::Worlds).into_installer();
    installer.download_fixture = Some(source.path().into());
    installer.install(&app, id, false, None).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        while !installer
            .abort_handle
            .lock()
            .await
            .handle
            .as_ref()
            .unwrap()
            .is_finished()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(existing.join("level.dat")).unwrap(),
        b"user world"
    );
    assert_eq!(
        std::fs::read(existing.with_file_name("NewWorld").join("level.dat")).unwrap(),
        b"new world"
    );
}

#[tokio::test]
async fn task05_replace_locked_refuses_before_mutation() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "locked",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    {
        let manager = app.instance_manager();
        let mut instances = manager.instances.write().await;
        instances
            .get_mut(&id)
            .unwrap()
            .data_mut()
            .unwrap()
            .config
            .modpack = Some(instance::info::ModpackInfo {
            locked: true,
            modpack: instance::info::Modpack::Curseforge(instance::info::CurseforgeModpack {
                project_id: 1,
                file_id: 1,
            }),
        });
    }
    assert!(
        cf("same.jar", b"v2", curseforge::ClassId::Mods)
            .into_installer()
            .install(&app, id, false, Some(row.id.clone()))
            .await
            .unwrap_err()
            .to_string()
            .contains("locked")
    );
    assert!(app.instance_manager().delete_mod(id, row.id).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"v1");
}

#[tokio::test]
async fn task05_world_root_and_wrapper_layouts_and_invalid_archives() {
    for (entries, expected) in [
        (
            vec![
                ("level.dat", b"root".as_slice()),
                ("region/r.mca", b"r".as_slice()),
            ],
            Some("archive"),
        ),
        (
            vec![("Wrapped/level.dat", b"wrapped".as_slice())],
            Some("Wrapped"),
        ),
        (
            vec![
                ("One/level.dat", b"1".as_slice()),
                ("Two/level.dat", b"2".as_slice()),
            ],
            None,
        ),
        (vec![("readme.txt", b"no world".as_slice())], None),
        (vec![], None),
        (
            vec![
                ("level.dat", b"root".as_slice()),
                ("Nested/level.dat", b"nested".as_slice()),
            ],
            None,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = InstancePath::new(dir.path().join("instance-root"));
        let archive = dir.path().join("archive.zip");
        zip(&archive, &entries);
        let installer = cf(
            "archive.zip",
            &std::fs::read(&archive).unwrap(),
            curseforge::ClassId::Worlds,
        );
        let result = installer.post_process(&path, &archive).await;
        if let Some(name) = expected {
            result.unwrap();
            assert!(path.get_saves_path().join(name).join("level.dat").is_file());
            assert!(
                !path
                    .get_saves_path()
                    .join(name)
                    .join(publication::WORLD_INCOMPLETE)
                    .exists()
            );
        } else {
            assert!(result.is_err());
            assert!(!path.get_saves_path().exists());
        }
    }
}

#[tokio::test]
async fn task05_replace_same_path_keeps_new_bytes_and_row_id() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "same-path",
        instance::AddonType::Mods,
        "same.jar",
        b"v1",
    )
    .await;
    app.instance_manager()
        .enable_mod(id, row.id.clone(), false)
        .await
        .unwrap();
    let resource = cf("same.jar", b"v2", curseforge::ClassId::Mods);
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let staging = tempfile::tempdir().unwrap();
    let staged = staging.path().join("artifact");
    std::fs::write(&staged, b"v2").unwrap();
    publication::publish_file(&resource, &app, id, &staged, Some(&plan), None)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(old.with_file_name("same.jar.disabled")).unwrap(),
        b"v2"
    );
    assert!(!old.exists());
    let current = mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!current.enabled);
    assert_ne!(current.metadata_id, row.metadata_id);
}

#[tokio::test]
async fn task05_replace_rechecks_bytes_toggle_and_collision() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "recheck",
        instance::AddonType::Mods,
        "old.jar",
        b"v1",
    )
    .await;
    let resource = cf("new.jar", b"v2", curseforge::ClassId::Mods);
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let staging = tempfile::tempdir().unwrap();
    let staged = staging.path().join("artifact");
    std::fs::write(&staged, b"v2").unwrap();
    std::fs::write(&old, b"edit").unwrap();
    assert!(
        publication::publish_file(&resource, &app, id, &staged, Some(&plan), None)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&old).unwrap(), b"edit");
    std::fs::write(&old, b"v1").unwrap();
    std::fs::write(old.with_file_name("new.jar"), b"foreign").unwrap();
    assert!(
        publication::publish_file(&resource, &app, id, &staged, Some(&plan), None)
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read(old.with_file_name("new.jar")).unwrap(),
        b"foreign"
    );
    std::fs::remove_file(old.with_file_name("new.jar")).unwrap();
    app.instance_manager()
        .enable_mod(id, row.id.clone(), false)
        .await
        .unwrap();
    publication::publish_file(&resource, &app, id, &staged, Some(&plan), None)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(old.with_file_name("new.jar.disabled")).unwrap(),
        b"v2"
    );
    assert!(
        mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn task05_replace_ordinary_identical_disabled_install_preserves_intent() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "noop",
        instance::AddonType::Mods,
        "same.jar",
        b"same",
    )
    .await;
    app.instance_manager()
        .enable_mod(id, row.id, false)
        .await
        .unwrap();
    let resource = cf("same.jar", b"same", curseforge::ClassId::Mods);
    assert!(resource.is_already_installed(&app, id).await.unwrap());
    resource
        .into_installer()
        .install(&app, id, false, None)
        .await
        .unwrap();
    assert!(!path.exists());
    assert_eq!(
        std::fs::read(path.with_file_name("same.jar.disabled")).unwrap(),
        b"same"
    );
}

#[tokio::test]
async fn task05_world_collision_preserves_existing_save() {
    let dir = tempfile::tempdir().unwrap();
    let instance = InstancePath::new(dir.path().to_path_buf());
    let saves = instance.get_saves_path();
    std::fs::create_dir_all(saves.join("MyWorld/region")).unwrap();
    std::fs::write(saves.join("MyWorld/level.dat"), b"user-level").unwrap();
    std::fs::write(saves.join("MyWorld/region/retained.mca"), b"region").unwrap();
    let archive = saves.join("download.zip");
    zip(&archive, &[("MyWorld/level.dat", b"archive-level")]);
    let installer = cf(
        "download.zip",
        &std::fs::read(&archive).unwrap(),
        curseforge::ClassId::Worlds,
    );
    assert!(installer.post_process(&instance, &archive).await.is_err());
    assert_eq!(
        std::fs::read(saves.join("MyWorld/level.dat")).unwrap(),
        b"user-level"
    );
    assert_eq!(
        std::fs::read(saves.join("MyWorld/region/retained.mca")).unwrap(),
        b"region"
    );
}

#[tokio::test]
async fn task05_normal_enabled_identical_artifact_is_a_real_noop() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "enabled-noop",
        instance::AddonType::Mods,
        "same.jar",
        b"same",
    )
    .await;
    let installer = cf("same.jar", b"same", curseforge::ClassId::Mods).into_installer();
    installer.install(&app, id, false, None).await.unwrap();
    assert!(installer.abort_handle.lock().await.handle.is_none());
    assert_eq!(std::fs::read(path).unwrap(), b"same");
    assert_eq!(
        mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
            .await
            .unwrap(),
        Some(row)
    );
}

#[tokio::test]
async fn task05_normal_disabled_zip_remains_disabled_after_real_scan() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "disabled-zip",
        instance::AddonType::ResourcePacks,
        "pack.zip",
        b"zip bytes",
    )
    .await;
    app.instance_manager()
        .enable_mod(id, row.id.clone(), false)
        .await
        .unwrap();
    app.meta_cache_manager()
        .override_caching_and_wait(
            crate::managers::metadata::cache::CacheEntityId::Instance(id),
            false,
            false,
        )
        .await
        .unwrap();
    assert!(!path.exists());
    assert_eq!(
        std::fs::read(path.with_file_name("pack.zip.disabled")).unwrap(),
        b"zip bytes"
    );
    let current = mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!current.enabled);
    assert_eq!(current.addon_type, "resourcepacks");
}

#[tokio::test]
async fn task05_round1_scanner_ignores_saves_files_and_refreshes_changed_mod() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, old) = super::super::mods::tests::addon_fixture(
        &app,
        "saves-files",
        instance::AddonType::Mods,
        "base.jar",
        b"old",
    )
    .await;
    let path = publication::current_path(&app, id).await.unwrap();
    let saves = path.get_saves_path();
    std::fs::create_dir_all(saves.join("World")).unwrap();
    std::fs::write(saves.join("World/level.dat"), b"world").unwrap();
    for name in ["README.txt", ".DS_Store", "old.zip"] {
        std::fs::write(saves.join(name), b"unrelated").unwrap();
    }
    // Strict child-probe failures remain errors; the scanner filters the candidate.
    assert!(
        publication::world_install_incomplete(&saves.join("README.txt"))
            .await
            .is_err()
    );
    std::fs::write(&old, b"changed bytes").unwrap();
    app.meta_cache_manager()
        .override_caching_and_wait(
            crate::managers::metadata::cache::CacheEntityId::Instance(id),
            false,
            false,
        )
        .await
        .unwrap();
    let rows = mfcdb::get_instance_mods_full(&app.db, *id).await.unwrap();
    use sha1::Digest;
    assert_eq!(
        rows.iter().find(|r| r.filename == "base.jar").unwrap().sha1,
        sha1::Sha1::digest(b"changed bytes").to_vec()
    );
    assert!(rows.iter().any(|r| r.filename == "World"));
    for name in ["README.txt", ".DS_Store", "old.zip"] {
        assert_eq!(std::fs::read(saves.join(name)).unwrap(), b"unrelated");
    }
    assert_eq!(
        std::fs::read(saves.join("World/level.dat")).unwrap(),
        b"world"
    );
}

async fn wait_for_install(installer: &Installer, app: &Arc<AppInner>, task_id: VisualTaskId) {
    tokio::time::timeout(Duration::from_secs(20), async {
        while !installer
            .abort_handle
            .lock()
            .await
            .handle
            .as_ref()
            .unwrap()
            .is_finished()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    if let Some(task) = app.task_manager().get_task(task_id).await {
        assert!(
            !matches!(&task.progress, crate::domain::vtask::Progress::Failed(_)),
            "worker reported failure: {task:?}"
        );
    }
}

#[tokio::test]
async fn task05_round1_world_version_installs_separate_copy_keeps_original_row() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "world-version",
        instance::AddonType::Worlds,
        "World",
        b"original",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    zip(source.path(), &[("World/level.dat", b"new version")]);
    let contents = std::fs::read(source.path()).unwrap();
    let mut installer = cf("World.zip", &contents, curseforge::ClassId::Worlds).into_installer();
    installer.download_fixture = Some(source.path().into());
    let task_id = installer
        .install(&app, id, false, Some(row.id.clone()))
        .await
        .expect("world version must install as a separate save");
    wait_for_install(&installer, &app, task_id).await;
    assert_eq!(std::fs::read(old.join("level.dat")).unwrap(), b"original");
    assert_eq!(
        std::fs::read(old.with_file_name("World (copy 1)").join("level.dat")).unwrap(),
        b"new version"
    );
    let retained = app
        .instance_manager()
        .get_owned_mod_file(id, &row.id)
        .await
        .unwrap();
    assert_eq!(retained.filename, "World");
}

#[tokio::test]
async fn task05_round1_enabled_original_replacement_remains_enabled() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "enabled-replacement",
        instance::AddonType::Mods,
        "old.jar",
        b"original",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"new version").unwrap();
    let mut installer = cf("new.jar", b"new version", curseforge::ClassId::Mods).into_installer();
    installer.download_fixture = Some(source.path().into());
    let task_id = installer
        .install(&app, id, false, Some(row.id.clone()))
        .await
        .unwrap();
    wait_for_install(&installer, &app, task_id).await;
    assert!(!old.exists());
    assert_eq!(
        std::fs::read(old.with_file_name("new.jar")).unwrap(),
        b"new version"
    );
    assert!(!old.with_file_name("new.jar.disabled").exists());
    let current = mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, "new.jar")
        .await
        .unwrap()
        .unwrap();
    assert!(current.enabled);
}

#[tokio::test]
async fn task05_round1_world_copy_revalidates_owner_row_and_original_directory() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, original) = super::super::mods::tests::addon_fixture(
        &app,
        "copy-owner",
        instance::AddonType::Worlds,
        "World",
        b"old",
    )
    .await;
    let (other, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "copy-foreign",
        instance::AddonType::Mods,
        "other.jar",
        b"other",
    )
    .await;
    let archive = tempfile::NamedTempFile::new().unwrap();
    zip(archive.path(), &[("World/level.dat", b"new")]);
    let resource = cf(
        "World.zip",
        &std::fs::read(archive.path()).unwrap(),
        curseforge::ClassId::Worlds,
    );
    let plan =
        publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Worlds)
            .await
            .unwrap();
    let manager = app.instance_manager();
    let foreign = manager.lock_instance(other).await;
    assert!(
        publication::publish_world_version(&resource, &app, &foreign, archive.path(), &plan)
            .await
            .is_err()
    );
    drop(foreign);
    let owned = manager.lock_instance(id).await;
    std::fs::rename(&original, original.with_file_name("Kept")).unwrap();
    assert!(
        publication::publish_world_version(&resource, &app, &owned, archive.path(), &plan)
            .await
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
    std::fs::write(&original, b"later ordinary file").unwrap();
    assert!(
        publication::publish_world_version(&resource, &app, &owned, archive.path(), &plan)
            .await
            .unwrap_err()
            .to_string()
            .contains("regular directory")
    );
    std::fs::remove_file(&original).unwrap();
    std::fs::rename(original.with_file_name("Kept"), &original).unwrap();
    mfcdb::delete_mod_file_cache_by_id(&app.db, &row.id)
        .await
        .unwrap();
    assert!(
        publication::publish_world_version(&resource, &app, &owned, archive.path(), &plan)
            .await
            .is_err()
    );
    assert!(!original.with_file_name("World (copy 1)").exists());
    assert_eq!(std::fs::read(original.join("level.dat")).unwrap(), b"old");
}

#[tokio::test]
async fn task05_round1_world_copy_reserves_past_occupied_and_late_destinations() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "copy-race",
        instance::AddonType::Worlds,
        "World.zip",
        b"original",
    )
    .await;
    let saves = old.parent().unwrap();
    std::fs::create_dir(saves.join("World (copy 1)")).unwrap();
    std::fs::write(saves.join("World (copy 1)/level.dat"), b"occupied").unwrap();
    let archive = tempfile::NamedTempFile::new().unwrap();
    zip(archive.path(), &[("level.dat", b"copy")]);
    let mut installer = cf(
        "World.zip",
        &std::fs::read(archive.path()).unwrap(),
        curseforge::ClassId::Worlds,
    )
    .into_installer();
    installer.download_fixture = Some(archive.path().into());
    let pause = app
        .instance_manager()
        .test_hooks
        .arm("world_copy_before_reserve", id);
    let task_id = installer
        .install(&app, id, false, Some(row.id.clone()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), pause.entered.notified())
        .await
        .unwrap();
    let task = app.task_manager().get_task(task_id).await.unwrap();
    assert!(matches!(
        task.name,
        Translation::InstanceTaskInstallWorldCopy { .. }
    ));
    // External save edits never authorize retiring this save. Another actor can
    // also arrive after preparation; only exclusive reservation selects a name.
    std::fs::write(old.join("level.dat"), b"edited while downloading").unwrap();
    std::fs::write(saves.join("World (copy 2)"), b"late file").unwrap();
    pause.release.notify_one();
    wait_for_install(&installer, &app, task_id).await;
    assert_eq!(
        std::fs::read(saves.join("World (copy 3)/level.dat")).unwrap(),
        b"copy"
    );
    assert_eq!(
        std::fs::read(saves.join("World (copy 1)/level.dat")).unwrap(),
        b"occupied"
    );
    assert_eq!(
        std::fs::read(saves.join("World (copy 2)")).unwrap(),
        b"late file"
    );
    assert_eq!(
        std::fs::read(old.join("level.dat")).unwrap(),
        b"edited while downloading"
    );
    assert!(
        app.instance_manager()
            .get_owned_mod_file(id, &row.id)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn task05_round1_world_copy_suffix_limit_retains_all_occupants() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("world.zip");
    zip(&archive, &[("World/level.dat", b"copy")]);
    let saves = root.path().join("saves");
    std::fs::create_dir(&saves).unwrap();
    for n in 1..=100 {
        std::fs::write(saves.join(format!("World (copy {n})")), b"occupied").unwrap();
    }
    let prepared = publication::prepare_world(&archive, "world").await.unwrap();
    // The same bounded reservation primitive is used by the explicit-version worker.
    assert!(
        prepared
            .reserve_copy(&saves)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("100 candidates")
    );
    assert_eq!(std::fs::read_dir(&saves).unwrap().count(), 100);
    for n in 1..=100 {
        assert_eq!(
            std::fs::read(saves.join(format!("World (copy {n})"))).unwrap(),
            b"occupied"
        );
    }
}

#[tokio::test]
async fn task05_round1_world_copy_accepts_distinct_archive_root() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "distinct-copy",
        instance::AddonType::Worlds,
        "Original",
        b"original",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    zip(source.path(), &[("NewRelease/level.dat", b"new version")]);
    let mut installer = cf(
        "release.zip",
        &std::fs::read(source.path()).unwrap(),
        curseforge::ClassId::Worlds,
    )
    .into_installer();
    installer.download_fixture = Some(source.path().into());
    let task_id = installer
        .install(&app, id, false, Some(row.id.clone()))
        .await
        .unwrap();
    wait_for_install(&installer, &app, task_id).await;
    assert_eq!(std::fs::read(old.join("level.dat")).unwrap(), b"original");
    assert_eq!(
        std::fs::read(old.with_file_name("NewRelease (copy 1)").join("level.dat")).unwrap(),
        b"new version"
    );
    assert!(
        app.instance_manager()
            .get_owned_mod_file(id, &row.id)
            .await
            .is_ok()
    );
    assert!(!old.with_file_name("NewRelease").exists());
}
