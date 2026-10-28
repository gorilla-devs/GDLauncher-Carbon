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
        exact_dependency: true,
        previous: None,
        project_name: "fixture".into(),
        addon_type: kind,
        file,
        download_url: "https://fixture.invalid/never-requested".into(),
        catalogue: None,
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
    // A separate kind no longer overwrites this disabled mod's owner row.
    assert!(resource.is_already_installed(&app, id).await.unwrap());
    assert_eq!(
        mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
            .len(),
        2
    );
    mfcdb::delete_mod_file_cache_by_id(&app.db, &row.id)
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
    // A save that already has the name is never written into: the world
    // goes in beside it, and the caller learns under which name.
    std::fs::create_dir_all(saves.join("World")).unwrap();
    std::fs::write(saves.join("World/level.dat"), b"late").unwrap();
    let reserved = publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&saves)
        .await
        .unwrap();
    assert_eq!(
        reserved.renamed(),
        vec![("World".to_owned(), "World (copy 1)".to_owned())]
    );
    reserved.finish().await.unwrap();
    assert_eq!(
        std::fs::read(saves.join("World/level.dat")).unwrap(),
        b"late"
    );
    assert_eq!(
        std::fs::read(saves.join("World (copy 1)/level.dat")).unwrap(),
        b"download"
    );
    // Something put into a reserved save meanwhile stays; only what the
    // install placed goes, and the save is no longer marked, so it shows.
    let other = dir.path().join("other-saves");
    let reserved = publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&other)
        .await
        .unwrap();
    let target = reserved.destinations()[0].clone();
    std::fs::create_dir(target.join("region")).unwrap();
    std::fs::write(target.join("region/a.mca"), b"arrived after reservation").unwrap();
    let error = reserved.finish().await.unwrap_err();
    assert!(format!("{error:#}").contains("was kept"), "{error:#}");
    assert_eq!(
        std::fs::read(target.join("region/a.mca")).unwrap(),
        b"arrived after reservation"
    );
    assert!(!target.join("level.dat").exists());
    assert!(
        !publication::world_install_incomplete(&target)
            .await
            .unwrap()
    );
    // A retry never writes into it either.
    publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&other)
        .await
        .unwrap()
        .finish()
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(target.join("region/a.mca")).unwrap(),
        b"arrived after reservation"
    );
    assert_eq!(
        std::fs::read(other.join("World (copy 1)/region/a.mca")).unwrap(),
        b"region"
    );
}

/// The world of [`a_world_installs_on_a_drive_that_refuses_a_full_flush`],
/// reserved in `dir`'s saves.
#[cfg(unix)]
async fn reserve_refusing_world(dir: &std::path::Path) -> publication::ReservedWorld {
    let archive = dir.join("archive.zip");
    zip(
        &archive,
        &[
            ("World/level.dat", b"level"),
            ("World/region/r.0.0.mca", b"region"),
        ],
    );
    let saves = dir.join("saves");
    std::fs::create_dir_all(&saves).unwrap();
    publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&saves)
        .await
        .unwrap()
}

/// A data folder on a macOS network share refuses std's F_FULLFSYNC (the SMB
/// client answers ENOTSUP), and 2.0.40 installed worlds there. The install
/// marker is synced through `util::durable`, which falls back to the plain
/// fsync such a share supports, so the world still installs. The test stands
/// in for that share on every platform.
#[cfg(unix)]
#[tokio::test]
async fn a_world_installs_on_a_drive_that_refuses_a_full_flush() {
    let syncs = crate::util::durable::test_hooks::FileSyncs::refusing_full_flush();
    let dir = tempfile::tempdir().unwrap();
    let reserved = reserve_refusing_world(dir.path()).await;
    let world = dir.path().join("saves").join("World");
    assert!(
        syncs.synced(&world.join(publication::WORLD_INCOMPLETE)),
        "the install marker is synced through the helper"
    );
    reserved.finish().await.unwrap();
    assert!(!publication::world_install_incomplete(&world).await.unwrap());
    assert_eq!(std::fs::read(world.join("level.dat")).unwrap(), b"level");
    assert_eq!(
        std::fs::read(world.join("region/r.0.0.mca")).unwrap(),
        b"region"
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

/// Which saves an archive becomes. 2.0.40 extracted it into the saves
/// folder as it was, so every world folder in it became a save; extras
/// beside a world (a README, `__MACOSX` from Finder's Compress, a resource
/// pack) are left out now, and a world at the archive's root is no longer
/// spilled into the saves folder.
#[tokio::test]
async fn task05_world_root_and_wrapper_layouts_and_invalid_archives() {
    for (entries, expected) in [
        (
            vec![("level.dat", b"root"), ("region/r.mca", b"r")],
            vec!["archive"],
            vec![("archive/region/r.mca", b"r")],
        ),
        (
            vec![("Wrapped/level.dat", b"wrapped")],
            vec!["Wrapped"],
            vec![("Wrapped/level.dat", b"wrapped")],
        ),
        // A map pack: every world is a save of its own.
        (
            vec![("One/level.dat", b"1"), ("Two/level.dat", b"2")],
            vec!["One", "Two"],
            vec![("One/level.dat", b"1"), ("Two/level.dat", b"2")],
        ),
        (vec![("readme.txt", b"no world")], vec![], vec![]),
        (vec![], vec![], vec![]),
        // A level.dat inside a world is part of it.
        (
            vec![("level.dat", b"root"), ("Nested/level.dat", b"nested")],
            vec!["archive"],
            vec![("archive/Nested/level.dat", b"nested")],
        ),
        (
            vec![
                ("Map/level.dat", b"map"),
                ("Map/backup/level.dat", b"backup"),
            ],
            vec!["Map"],
            vec![("Map/backup/level.dat", b"backup")],
        ),
        // Whatever sits beside the world is left out.
        (
            vec![
                ("World/level.dat", b"world"),
                ("World/region/r.0.0.mca", b"r"),
                ("README.txt", b"credits"),
                ("Resources.zip", b"pack"),
                (".DS_Store", b"finder"),
                ("__MACOSX/World/._level.dat", b"fork"),
                ("__MACOSX/._World", b"fork"),
                ("Extras/notes.txt", b"notes"),
            ],
            vec!["World"],
            vec![("World/region/r.0.0.mca", b"r")],
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
        if expected.is_empty() {
            let error = result.unwrap_err();
            // An empty zip is not even read as an archive.
            assert!(
                entries.is_empty() || format!("{error:#}").contains("no Minecraft world"),
                "{error:#}"
            );
            assert!(!path.get_saves_path().exists());
            continue;
        }
        result.unwrap();
        let mut saves = std::fs::read_dir(path.get_saves_path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        saves.sort();
        assert_eq!(saves, expected, "{entries:?}");
        for name in expected {
            let save = path.get_saves_path().join(name);
            assert!(save.join("level.dat").is_file());
            assert!(!save.join(publication::WORLD_INCOMPLETE).exists());
        }
        for (file, bytes) in contents {
            assert_eq!(
                std::fs::read(path.get_saves_path().join(file)).unwrap(),
                bytes
            );
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

/// Installing a world whose save name is taken (the same map again, to
/// reset it or for its next release, or any map called "New World") installs
/// it beside the existing save, which stays as it is.
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
    installer.post_process(&instance, &archive).await.unwrap();
    assert_eq!(
        std::fs::read(saves.join("MyWorld/level.dat")).unwrap(),
        b"user-level"
    );
    assert_eq!(
        std::fs::read(saves.join("MyWorld/region/retained.mca")).unwrap(),
        b"region"
    );
    assert_eq!(
        std::fs::read(saves.join("MyWorld (copy 1)/level.dat")).unwrap(),
        b"archive-level"
    );
}

/// ENOSPC, or ERROR_DISK_FULL on Windows.
const DISK_FULL: i32 = if cfg!(windows) { 112 } else { 28 };

/// A world install that fails part way (a full disk, a file another program
/// holds) takes back what it placed: no hidden half save stays behind to
/// block the next attempt, which installs the world under its own name.
#[tokio::test]
async fn world_install_that_fails_part_way_leaves_nothing_and_a_retry_installs_it() {
    use crate::util::publish::fault::{CROSS_DEVICE, Step, inject};
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("archive.zip");
    zip(
        &archive,
        &[
            ("World/a.mca", b"region a"),
            ("World/b.mca", b"region b"),
            ("World/level.dat", b"level"),
        ],
    );
    let saves = dir.path().join("saves");
    let reserved = publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&saves)
        .await
        .unwrap();
    let target = reserved.destinations()[0].clone();
    // Saves on another drive are copied; the second copy runs out of room.
    inject(Step::NoReplace, &target.join("a.mca"), CROSS_DEVICE);
    publication::world_fault::inject(&target.join("b.mca"), DISK_FULL);
    let receipt = Arc::new(Mutex::new(None));
    let error = reserved
        .finish_owned(dir.path().to_path_buf(), &receipt)
        .await
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("Could not install the world \"World\""),
        "{error:#}"
    );
    assert!(!target.exists(), "nothing of the failed install is left");
    assert!(receipt.lock().await.is_none());
    let reserved = publication::prepare_world(&archive, "archive")
        .await
        .unwrap()
        .reserve(&saves)
        .await
        .unwrap();
    assert!(reserved.renamed().is_empty());
    reserved.finish().await.unwrap();
    assert_eq!(std::fs::read(target.join("b.mca")).unwrap(), b"region b");
    assert!(!target.join(publication::WORLD_INCOMPLETE).exists());
}

/// A save an install left unfinished when GDLauncher quit or crashed during
/// it is hidden from the world list. The next install of the same world
/// replaces it when it holds nothing but that world's files, whole or cut
/// short; one that was opened or changed since, or that an install still
/// running owns, stays as it is and the world goes in beside it.
#[tokio::test]
async fn an_unfinished_world_install_is_replaced_by_a_retry_unless_it_was_used() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("archive.zip");
    zip(
        &archive,
        &[
            ("World/level.dat", b"level"),
            ("World/region/r.0.0.mca", b"0123456789"),
        ],
    );
    let prepare = || publication::prepare_world(&archive, "archive");
    for used in [None, Some("session.lock"), Some("level.dat")] {
        // Named after the file that marks the save as used; a debug-printed
        // Option would put quotes in the name, which Windows refuses.
        let saves = dir
            .path()
            .join(format!("saves-{}", used.unwrap_or("unused")));
        let reserved = prepare().await.unwrap().reserve(&saves).await.unwrap();
        let leftover = reserved.destinations()[0].clone();
        // GDLauncher stopped: the reservation is no install's any more.
        drop(reserved);
        std::fs::write(leftover.join("level.dat"), b"level").unwrap();
        std::fs::create_dir(leftover.join("region")).unwrap();
        std::fs::write(leftover.join("region/r.0.0.mca"), b"01234").unwrap();
        if let Some(file) = used {
            std::fs::write(leftover.join(file), b"played").unwrap();
        }
        assert!(
            publication::world_install_incomplete(&leftover)
                .await
                .unwrap()
        );
        let reserved = prepare().await.unwrap().reserve(&saves).await.unwrap();
        let destination = reserved.destinations()[0].clone();
        reserved.finish().await.unwrap();
        if let Some(file) = used {
            assert_eq!(destination, saves.join("World (copy 1)"));
            assert_eq!(std::fs::read(leftover.join(file)).unwrap(), b"played");
            assert_eq!(
                std::fs::read(leftover.join("region/r.0.0.mca")).unwrap(),
                b"01234"
            );
            assert!(
                publication::world_install_incomplete(&leftover)
                    .await
                    .unwrap()
            );
        } else {
            assert_eq!(destination, leftover);
            assert!(
                !publication::world_install_incomplete(&leftover)
                    .await
                    .unwrap()
            );
        }
        assert_eq!(
            std::fs::read(destination.join("region/r.0.0.mca")).unwrap(),
            b"0123456789"
        );
    }
    // An install still running keeps its save.
    let saves = dir.path().join("saves-live");
    let running = prepare().await.unwrap().reserve(&saves).await.unwrap();
    let other = prepare().await.unwrap().reserve(&saves).await.unwrap();
    assert_eq!(other.destinations(), vec![saves.join("World (copy 1)")]);
    other.finish().await.unwrap();
    running.finish().await.unwrap();
    assert_eq!(
        std::fs::read(saves.join("World/region/r.0.0.mca")).unwrap(),
        b"0123456789"
    );
}

/// A world that cannot be moved into the instance (the saves folder is on
/// another drive, or a virus scanner still holds a file just extracted) is
/// copied, and its receipt still proves the new save is unchanged.
#[tokio::test]
async fn world_files_that_cannot_be_moved_are_copied_and_the_receipt_still_holds() {
    use crate::util::publish::fault::{CROSS_DEVICE, IN_USE, Step, inject, pending};
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("archive.zip");
    zip(
        &archive,
        &[
            ("World/a.mca", b"region a"),
            ("World/b.mca", b"region b"),
            ("World/level.dat", b"level"),
        ],
    );
    for (code, saves) in [
        (IN_USE, dir.path().join("in-use")),
        (CROSS_DEVICE, dir.path().join("other-drive")),
    ] {
        let reserved = publication::prepare_world(&archive, "archive")
            .await
            .unwrap()
            .reserve(&saves)
            .await
            .unwrap();
        let target = reserved.destinations()[0].clone();
        inject(Step::NoReplace, &target.join("a.mca"), code);
        let receipt = Arc::new(Mutex::new(None));
        reserved
            .finish_owned(dir.path().to_path_buf(), &receipt)
            .await
            .unwrap();
        assert!(!pending(Step::NoReplace, &target.join("a.mca")));
        for (file, bytes) in [
            ("a.mca", b"region a".as_slice()),
            ("b.mca", b"region b"),
            ("level.dat", b"level"),
        ] {
            assert_eq!(std::fs::read(target.join(file)).unwrap(), bytes);
        }
        let receipt = receipt.lock().await;
        let receipt = receipt.as_ref().expect("a complete install has a receipt");
        receipt.validate_commit().await.unwrap();
        std::fs::write(target.join("b.mca"), b"played in").unwrap();
        assert!(receipt.validate_commit().await.is_err());
    }
}

/// A map pack installs each of its worlds, and an install that fails later
/// (a dependency, the cache) takes all of them back.
#[tokio::test]
async fn a_map_pack_installs_each_world_and_rolls_back_as_one() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "map-pack",
        instance::AddonType::Mods,
        "base.jar",
        b"base",
    )
    .await;
    let path = publication::current_path(&app, id).await.unwrap();
    let saves = path.get_saves_path();
    let archive = tempfile::NamedTempFile::new().unwrap();
    zip(
        archive.path(),
        &[
            ("Pack/One/level.dat", b"one"),
            ("Pack/Two/level.dat", b"two"),
            ("Pack/README.txt", b"readme"),
        ],
    );
    let receipt = Arc::new(Mutex::new(None));
    let operation = app.instance_manager().lock_instance(id).await;
    let reserved = publication::prepare_world(archive.path(), "Pack")
        .await
        .unwrap()
        .reserve(&saves)
        .await
        .unwrap();
    assert_eq!(reserved.names(), vec!["One", "Two"]);
    reserved
        .finish_owned(path.get_root(), &receipt)
        .await
        .unwrap();
    drop(operation);
    assert_eq!(std::fs::read(saves.join("One/level.dat")).unwrap(), b"one");
    assert_eq!(std::fs::read(saves.join("Two/level.dat")).unwrap(), b"two");
    publication::rollback_receipt(&app, id, &receipt)
        .await
        .unwrap();
    assert!(!saves.join("One").exists());
    assert!(!saves.join("Two").exists());
}

/// Installing a world whose save name is taken (the same map again, or one
/// shipped as "New World") installs it beside that save instead of failing
/// after the download, and tells the user under which name.
#[tokio::test]
async fn a_world_whose_save_name_is_taken_installs_beside_it_and_says_where() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, existing) = super::super::mods::tests::addon_fixture(
        &app,
        "name-taken",
        instance::AddonType::Worlds,
        "MyWorld",
        b"my progress",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    zip(source.path(), &[("MyWorld/level.dat", b"fresh map")]);
    let contents = std::fs::read(source.path()).unwrap();
    let mut installer = cf("MyWorld.zip", &contents, curseforge::ClassId::Worlds).into_installer();
    installer.download_fixture = Some(source.path().into());
    let task = installer.install(&app, id, false, None).await.unwrap();
    wait_for_install(&installer, &app, task).await;
    assert!(matches!(
        settled_outcome(&app, task).await,
        crate::managers::vtask::TaskOutcome::Succeeded
    ));
    assert_eq!(
        std::fs::read(existing.join("level.dat")).unwrap(),
        b"my progress"
    );
    assert_eq!(
        std::fs::read(
            existing
                .with_file_name("MyWorld (copy 1)")
                .join("level.dat")
        )
        .unwrap(),
        b"fresh map"
    );
    let rows = mfcdb::get_instance_mods_full(&app.db, *id).await.unwrap();
    assert!(rows.iter().any(|row| row.filename == "MyWorld (copy 1)"));
    let warnings = app.task_manager().get_task_warnings().await;
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].id, task);
    assert_eq!(warnings[0].subject.as_deref(), Some("fixture"));
    assert_eq!(
        warnings[0].warnings,
        vec![crate::domain::vtask::TaskWarning::WorldNameTaken {
            name: "MyWorld".into(),
            saved_as: "MyWorld (copy 1)".into(),
        }]
    );
}

/// A large map unpacks before the install takes the instance, so Play,
/// addon changes and scans of that instance do not wait for it.
#[tokio::test]
async fn a_world_unpacks_without_holding_the_instance() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "unpack-outside-gate",
        instance::AddonType::Mods,
        "base.jar",
        b"base",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    zip(source.path(), &[("Map/level.dat", b"map")]);
    let contents = std::fs::read(source.path()).unwrap();
    let mut installer = cf("Map.zip", &contents, curseforge::ClassId::Worlds).into_installer();
    installer.download_fixture = Some(source.path().into());
    let identity = installer.inner.lock().await.id();
    let pause = app
        .instance_manager()
        .test_hooks
        .arm_path("addon_graph_before_member_publish", &identity);
    let task = installer.install(&app, id, false, None).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), pause.entered.notified())
        .await
        .unwrap();
    let gate = app.instance_manager().lock_instance(id).await;
    pause.release.notify_one();
    let root = app.settings_manager().runtime_path.get_root().to_path();
    let unpacked = || {
        std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(publication::STAGING_PREFIX)
            })
            .filter_map(|staging| std::fs::read_dir(staging.path()).ok())
            .flatten()
            .filter_map(Result::ok)
            .any(|extraction| extraction.path().join("Map/level.dat").is_file())
    };
    tokio::time::timeout(Duration::from_secs(20), async {
        while !unpacked() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the map unpacks while the instance is held by something else");
    drop(gate);
    wait_for_install(&installer, &app, task).await;
    let saves = publication::current_path(&app, id)
        .await
        .unwrap()
        .get_saves_path();
    assert_eq!(std::fs::read(saves.join("Map/level.dat")).unwrap(), b"map");
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
    // Windows reports a path below a file as missing, so there the probe just
    // finds no marker.
    let probe = publication::world_install_incomplete(&saves.join("README.txt")).await;
    if cfg!(windows) {
        assert!(matches!(probe, Ok(false)), "{probe:?}");
    } else {
        assert!(probe.is_err(), "{probe:?}");
    }
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
            .contains("\"World (copy 1)\" to \"World (copy 100)\"")
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

#[tokio::test]
async fn task06_installer_abort_awaits_owned_file_work() {
    let installer =
        Arc::new(cf("fixture.jar", b"test", curseforge::ClassId::Mods).into_installer());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private-io");
    let worker_path = path.clone();
    let (arrive, arrived) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    installer
        .abort_handle
        .lock()
        .await
        .spawn(move |cancel| async move {
            cancel.cancelled().await;
            // Explicit barrier represents already-owned work that must settle before rollback.
            arrive.send(()).unwrap();
            released.await.unwrap();
            tokio::fs::write(worker_path, b"settled").await.unwrap();
        });
    let owner = installer.clone();
    let mut abort = tokio::spawn(async move {
        owner.abort(None).await.unwrap();
    });
    arrived.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut abort)
            .await
            .is_err(),
        "abort cannot acknowledge before owned IO settles"
    );
    release.send(()).unwrap();
    abort.await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"settled");
    assert!(installer.abort_handle.lock().await.aborted);
    // Repeat callers also observe the same settled completion without an AbortHandle race.
    installer.abort(None).await.unwrap();
}

#[tokio::test]
async fn task06_installer_missing_ack_does_not_certify_settlement() {
    let (sender, receiver) = watch::channel(false);
    drop(sender);
    assert!(wait_install_completion(Some(receiver)).await.is_err());
}

#[tokio::test]
async fn task06_installer_abort_settles_admission_before_return() {
    let installer =
        Arc::new(cf("fixture.jar", b"test", curseforge::ClassId::Mods).into_installer());
    let admission = installer.admission.lock().await;
    let owner = installer.clone();
    let mut abort = tokio::spawn(async move {
        owner.abort(None).await.unwrap();
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut abort)
            .await
            .is_err()
    );
    assert!(installer.abort_handle.lock().await.aborted);
    drop(admission);
    abort.await.unwrap();
}

mod round1_graph {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::{Notify, Semaphore, oneshot};
    #[derive(Clone, Copy)]
    enum Satisfaction {
        Missing,
        Present,
        Error,
    }
    struct Spec {
        name: &'static str,
        rollbacks: Option<Arc<std::sync::atomic::AtomicUsize>>,
        rollback_hook: Option<Arc<Hook>>,
        child: Option<Arc<Spec>>,
        dependency_error: bool,
        satisfaction: Satisfaction,
        url: Option<String>,
        hook: Option<Arc<Hook>>,
    }
    struct Hook {
        entered: Notify,
        release: Semaphore,
    }
    struct Resource(Arc<Spec>);
    #[async_trait::async_trait]
    impl ResourceInstaller for Resource {
        fn id(&self) -> String {
            self.0.name.into()
        }
        fn display_name(&self) -> String {
            self.id()
        }
        async fn downloadable(&self, path: &InstancePath) -> Option<Downloadable> {
            self.0.url.as_ref().map(|url| {
                Downloadable::new(url, self.get_install_path(path))
                    .with_size(4)
                    .with_checksum(Some(Checksum::Sha1(
                        "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".into(),
                    )))
            })
        }
        fn get_install_path(&self, path: &InstancePath) -> PathBuf {
            path.get_mods_path().join(format!("{}.jar", self.0.name))
        }
        async fn post_process(&self, _: &InstancePath, _: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
        fn dependencies(
            &self,
            _: &Arc<AppInner>,
            _: InstanceId,
            _: &InstanceData,
            _: ModChannel,
        ) -> DependencyIterator {
            let mut entries: Vec<ResourceInstallerGetter> = Vec::new();
            if let Some(child) = self.0.child.clone() {
                entries.push(Box::new(move |_| {
                    Box::pin(async move {
                        Some(Ok(Box::new(Resource(child)) as BoxedResourceInstaller))
                    })
                }));
            }
            if self.0.dependency_error {
                let hook = self.0.hook.clone();
                entries.push(Box::new(move |_| {
                    Box::pin(async move {
                        if let Some(hook) = hook {
                            hook.entered.notify_one();
                            hook.release.acquire().await.unwrap().forget();
                        }
                        Some(Err(anyhow::anyhow!("fixture required dependency failed")))
                    })
                }));
            }
            DependencyIterator::new(entries.into_iter())
        }
        async fn is_already_installed(
            &self,
            app: &Arc<AppInner>,
            id: InstanceId,
        ) -> anyhow::Result<bool> {
            if self.0.url.is_some() && publication::is_satisfied(self, app, id).await? {
                return Ok(true);
            }
            if !self.0.dependency_error {
                if let Some(hook) = &self.0.hook {
                    hook.entered.notify_one();
                    hook.release.acquire().await.unwrap().forget();
                }
            }
            match self.0.satisfaction {
                Satisfaction::Missing => Ok(false),
                Satisfaction::Present => Ok(true),
                Satisfaction::Error => bail!("fixture post-dependency satisfaction error"),
            }
        }
        async fn rollback(&self, _: &mut InstanceData) -> anyhow::Result<()> {
            if let Some(count) = &self.0.rollbacks {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            if let Some(hook) = &self.0.rollback_hook {
                hook.entered.notify_one();
                hook.release.acquire().await.unwrap().forget();
            }
            Ok(())
        }
    }
    async fn held() -> (
        String,
        oneshot::Receiver<()>,
        oneshot::Sender<()>,
        Arc<std::sync::atomic::AtomicBool>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/graph", listener.local_addr().unwrap());
        let (accepted, arrived) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen = closed.clone();
        let fixture = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(socket.read_u8().await.unwrap());
            }
            assert!(request.starts_with(b"GET /graph HTTP/1.1\r\n"));
            accepted.send(()).unwrap();
            let mut byte = [0];
            tokio::select! {
                result=socket.read(&mut byte) => { assert_eq!(result.unwrap(),0); seen.store(true,std::sync::atomic::Ordering::SeqCst); }
                _=released => { let _=socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest").await; }
            }
        });
        (url, arrived, release, closed, fixture)
    }
    async fn run(mode: &str) {
        let app = crate::setup_managers_for_test().await;
        let (id, _, _) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            &format!("round1-{mode}"),
            instance::AddonType::Mods,
            "existing.jar",
            b"existing",
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let final_c = path.get_mods_path().join("c.jar");
        let (url, arrived, release, closed, fixture) = held().await;
        let hook = Arc::new(Hook {
            entered: Notify::new(),
            release: Semaphore::new(0),
        });
        let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cleanup = Arc::new(Hook {
            entered: Notify::new(),
            release: Semaphore::new(0),
        });
        let c = Arc::new(Spec {
            name: "c",
            rollbacks: None,
            rollback_hook: None,
            child: None,
            dependency_error: false,
            satisfaction: Satisfaction::Missing,
            url: Some(url),
            hook: None,
        });
        let b = Arc::new(Spec {
            name: "b",
            rollbacks: Some(rollbacks.clone()),
            rollback_hook: if mode.starts_with("cleanup-") {
                Some(cleanup.clone())
            } else {
                None
            },
            child: Some(c),
            dependency_error: false,
            satisfaction: if mode == "mid-error" {
                Satisfaction::Error
            } else if mode == "satisfied-mid" || mode == "success" {
                // The success control uses an actually satisfied no-artifact
                // ancestor; ACK alone is no longer semantic satisfaction.
                Satisfaction::Present
            } else {
                Satisfaction::Missing
            },
            url: None,
            hook: if mode == "mid-error" {
                Some(hook.clone())
            } else {
                None
            },
        });
        let a = Arc::new(Spec {
            name: "a",
            rollbacks: None,
            rollback_hook: None,
            child: Some(b),
            dependency_error: mode == "dependency-error",
            satisfaction: if mode == "post-error" || mode.starts_with("cleanup-") {
                Satisfaction::Error
            } else if mode == "dependency-error" {
                Satisfaction::Present
            } else {
                Satisfaction::Missing
            },
            url: None,
            hook: if mode == "mid-error" {
                None
            } else {
                Some(hook.clone())
            },
        });
        let installer = Arc::new(Installer::new(Box::new(Resource(a))));
        let parent = Arc::new(Mutex::new(VisualTask::new(
            Translation::InstanceTaskInstallMod {
                mod_name: "fixture".into(),
                instance_name: "fixture".into(),
            },
        )));
        let worker = installer.clone();
        let app_clone = app.clone();
        let admission = tokio::spawn(async move {
            worker
                .install_inner(
                    &app_clone,
                    id,
                    &path,
                    &parent,
                    &Arc::new(InstallGraph::default()),
                    true,
                    None,
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(10), arrived)
            .await
            .unwrap()
            .unwrap();
        hook.entered.notified().await;
        if mode == "finished-mid" {
            let handle = {
                let context = installer.rollback_context.lock().await;
                let context = context.as_ref().unwrap();
                let deps = context.processed_deps.lock().await;
                let state = deps[0].abort_handle.lock().await;
                state.handle.as_ref().unwrap().clone()
            };
            tokio::time::timeout(Duration::from_secs(15), async {
                while !handle.is_finished() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        let middle = installer.ownership.children.lock().unwrap()[0].clone();
        let leaf = middle.children.lock().unwrap()[0].clone();
        if mode == "satisfied-mid" {
            assert!(
                middle.owner.lock().await.completion.is_none(),
                "satisfied ancestor must exercise the no-worker route"
            );
        }
        if mode == "drop-admission" {
            admission.abort();
            assert!(admission.await.unwrap_err().is_cancelled());
            let error = installer.abort(None).await.unwrap_err();
            assert!(format!("{error:#}").contains("settlement unknown"));
            assert!(
                *leaf
                    .owner
                    .lock()
                    .await
                    .completion
                    .as_ref()
                    .unwrap()
                    .borrow()
            );
            assert!(leaf.cancel.is_cancelled());
            fixture.await.unwrap();
            assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
            assert!(!final_c.exists());
            assert_eq!(
                rollbacks.load(std::sync::atomic::Ordering::SeqCst),
                0,
                "Unknown must forbid parent rollback"
            );
            return;
        }
        hook.release.add_permits(1);
        if mode.starts_with("cleanup-") {
            cleanup.entered.notified().await;
            let owner = installer.clone();
            let mut abort = tokio::spawn(async move { owner.abort(None).await });
            assert!(
                tokio::time::timeout(Duration::from_millis(100), &mut abort)
                    .await
                    .is_err(),
                "abort must wait for the admitted error cleanup"
            );
            if mode == "cleanup-drop" {
                admission.abort();
                assert!(admission.await.unwrap_err().is_cancelled());
                cleanup.release.add_permits(2);
                let outcome = abort.await.unwrap();
                assert!(
                    outcome.is_err(),
                    "dropped error cleanup must remain Unknown, not certified complete"
                );
                assert!(format!("{:#}", outcome.unwrap_err()).contains("settlement unknown"));
                assert_eq!(
                    rollbacks.load(std::sync::atomic::Ordering::SeqCst),
                    1,
                    "uncertainty must forbid a second parent rollback"
                );
            } else {
                cleanup.release.add_permits(2);
                let error = admission.await.unwrap().unwrap_err();
                assert!(
                    format!("{error:#}").contains("fixture post-dependency satisfaction error")
                );
                abort.await.unwrap().unwrap();
            }
            assert!(
                *leaf
                    .owner
                    .lock()
                    .await
                    .completion
                    .as_ref()
                    .unwrap()
                    .borrow()
            );
            fixture.await.unwrap();
            assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
            assert!(!final_c.exists());
            return;
        }
        let result = tokio::time::timeout(Duration::from_secs(15), admission)
            .await
            .unwrap()
            .unwrap();
        let should_error = matches!(mode, "post-error" | "mid-error" | "dependency-error");
        if should_error {
            let error = format!("{:#}", result.as_ref().unwrap_err());
            let expected = if mode == "dependency-error" {
                "fixture required dependency failed"
            } else {
                "fixture post-dependency satisfaction error"
            };
            assert!(
                error.contains(expected),
                "original admission error must survive settlement: {error}"
            );
        }
        if mode == "missing-ack" {
            result.unwrap();
            leaf.owner.lock().await.handle.as_ref().unwrap().abort();
            let error = installer.abort(None).await.unwrap_err();
            assert!(format!("{error:#}").contains("without acknowledging IO settlement"));
            fixture.await.unwrap();
            assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
            assert!(!final_c.exists());
            assert_eq!(
                rollbacks.load(std::sync::atomic::Ordering::SeqCst),
                0,
                "Unknown must forbid parent rollback"
            );
            return;
        }
        if mode == "success" {
            result.unwrap();
            release.send(()).unwrap();
            fixture.await.unwrap();
            wait_install_completion(leaf.owner.lock().await.completion.clone())
                .await
                .unwrap();
            wait_install_completion(middle.owner.lock().await.completion.clone())
                .await
                .unwrap();
            wait_install_completion(installer.abort_handle.lock().await.completion.clone())
                .await
                .unwrap();
            assert!(!leaf.cancel.is_cancelled());
            assert_eq!(std::fs::read(&final_c).unwrap(), b"test");
            return;
        }
        if !should_error {
            result.as_ref().unwrap();
            installer.abort(None).await.unwrap();
        }
        assert!(
            *leaf
                .owner
                .lock()
                .await
                .completion
                .as_ref()
                .unwrap()
                .borrow(),
            "parent must observe actual leaf worker ACK"
        );
        assert!(leaf.cancel.is_cancelled());
        // Socket closure is observed before fixture release; it is a concrete transport
        // consequence of actual descendant cancellation, not an injected graph ACK.
        let stopped = tokio::time::timeout(Duration::from_millis(300), async {
            while !closed.load(std::sync::atomic::Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_ok();
        let _ = release.send(());
        fixture.await.unwrap();
        if !stopped {
            let _ = tokio::time::timeout(Duration::from_secs(5), async {
                while !final_c.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await;
        }
        assert_eq!(
            result.is_err(),
            should_error,
            "required dependency error must not become satisfied success"
        );
        assert!(
            stopped,
            "{mode}: parent returned without cancelling its admitted descendant"
        );
        assert!(
            !final_c.exists(),
            "descendant published after parent settlement"
        );
    }
    // These run multi-threaded: the metadata cache hashes through
    // carbon_scheduler::cpu_block, whose WaitFuture calls block_in_place when
    // dropped mid-hash, and that panics while a current_thread runtime shuts down.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_dropped_error_cleanup_remains_unknown() {
        run("cleanup-drop").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_concurrent_abort_waits_for_error_cleanup() {
        run("cleanup-complete").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_missing_worker_ack_forbids_rollback() {
        run("missing-ack").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_dropped_admission_reports_unknown() {
        run("drop-admission").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_required_dependency_success() {
        run("success").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_finished_middle_settles_leaf() {
        run("finished-mid").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_satisfied_middle_settles_leaf() {
        run("satisfied-mid").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_post_dependency_error_settles_leaf() {
        run("post-error").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_failed_child_admission_retains_leaf() {
        run("mid-error").await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task06_round1_graph_dependency_error_is_not_satisfied_success() {
        run("dependency-error").await;
    }

    /// Answers every request with the fixture artifact (`test`).
    pub(super) async fn serve_artifacts() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    while !request.ends_with(b"\r\n\r\n") {
                        match socket.read_u8().await {
                            Ok(byte) => request.push(byte),
                            Err(_) => return,
                        }
                    }
                    let _ = socket
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest")
                        .await;
                });
            }
        });
        base
    }

    /// An addon requiring three more in a chain, all downloaded and published.
    const CHAIN: [&str; 4] = ["root-addon", "dep-one", "dep-two", "dep-three"];

    fn chain(base: &str, rollbacks: &Arc<std::sync::atomic::AtomicUsize>) -> Arc<Spec> {
        let mut child = None;
        for name in CHAIN.into_iter().rev() {
            child = Some(Arc::new(Spec {
                name,
                rollbacks: Some(rollbacks.clone()),
                rollback_hook: None,
                child,
                dependency_error: false,
                satisfaction: Satisfaction::Missing,
                url: Some(format!("{base}/{name}")),
                hook: None,
            }));
        }
        child.unwrap()
    }

    /// Install the chain and wait until its task settles.
    async fn install_chain(
        app: &Arc<AppInner>,
        id: InstanceId,
        rollbacks: &Arc<std::sync::atomic::AtomicUsize>,
    ) -> crate::managers::vtask::TaskOutcome {
        let base = serve_artifacts().await;
        let installer = Installer::new(Box::new(Resource(chain(&base, rollbacks))));
        let task = installer.install(app, id, true, None).await.unwrap();
        tokio::time::timeout(
            Duration::from_secs(30),
            wait_install_completion(installer.abort_handle.lock().await.completion.clone()),
        )
        .await
        .expect("the install settles")
        .unwrap();
        for _ in 0..200 {
            let outcome = app.task_manager().get_task_outcome(task).await;
            if !matches!(outcome, crate::managers::vtask::TaskOutcome::Running(_)) {
                return outcome;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the install task never settled");
    }

    fn assert_chain_published(
        mods: &std::path::Path,
        outcome: &crate::managers::vtask::TaskOutcome,
        rollbacks: &std::sync::atomic::AtomicUsize,
    ) {
        match outcome {
            crate::managers::vtask::TaskOutcome::Succeeded => {}
            crate::managers::vtask::TaskOutcome::Failed(task) => {
                panic!("the install failed: {task:?}")
            }
            _ => panic!("the install did not succeed"),
        }
        assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 0);
        for name in CHAIN {
            assert_eq!(
                std::fs::read(mods.join(format!("{name}.jar"))).unwrap(),
                b"test"
            );
        }
    }

    /// Each member's finalization pass trusts unchanged addons on their
    /// stamps, and hashes what it must with the instance gate released.
    #[tokio::test]
    async fn dependency_chain_reads_no_existing_addon_and_leaves_play_free() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, first) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            "graph-large-instance",
            instance::AddonType::Mods,
            "existing-0.jar",
            b"existing 0",
        )
        .await;
        let mods = first.parent().unwrap().to_owned();
        let mut existing = vec![first];
        for i in 1..200 {
            let path = mods.join(format!("existing-{i}.jar"));
            std::fs::write(&path, format!("existing {i}")).unwrap();
            existing.push(path);
        }
        let mcm = app.meta_cache_manager();
        mcm.override_caching_and_wait(
            crate::managers::metadata::cache::CacheEntityId::Instance(id),
            false,
            false,
        )
        .await
        .unwrap();
        // A file the user dropped in that no scan has read yet: the first
        // finalization pass hashes it, and stops there until released.
        let dropped = mods.join("dropped.jar");
        std::fs::write(&dropped, b"dropped by the user").unwrap();
        let pause = mcm.arm_scan_pause(&dropped, "hash");
        mcm.clear_scan_observations();

        let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let install = {
            let app = app.clone();
            let rollbacks = rollbacks.clone();
            tokio::spawn(async move { install_chain(&app, id, &rollbacks).await })
        };
        tokio::time::timeout(Duration::from_secs(20), pause.entered.notified())
            .await
            .expect("a finalization pass reaches the unread file");
        // Play's preparation starts by taking this gate.
        let gate = tokio::time::timeout(
            Duration::from_secs(1),
            app.instance_manager().lock_instance(id),
        )
        .await
        .expect("Play gets the instance gate while the pass hashes");
        drop(gate);
        pause.release.notify_one();

        let outcome = install.await.unwrap();
        assert_chain_published(&mods, &outcome, &rollbacks);
        for path in &existing {
            assert_eq!(
                mcm.scan_observations(path, "hash"),
                0,
                "{} was already cached and must not be read again",
                path.display()
            );
        }
    }

    /// One unrelated file the launcher cannot read never rolls back an install.
    #[tokio::test]
    async fn dependency_chain_commits_despite_unrelated_unreadable_files() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, first) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            "graph-bad-files",
            instance::AddonType::Mods,
            "existing.jar",
            b"existing",
        )
        .await;
        let mods = first.parent().unwrap().to_owned();
        let root = mods.parent().unwrap().to_owned();
        std::fs::write(root.join("resourcepacks"), b"a file where a folder belongs").unwrap();
        let locked = mods.join("locked.jar");
        std::fs::write(&locked, b"never readable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::os::unix::fs::symlink("missing-target.jar", mods.join("broken.jar")).unwrap();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let outcome = install_chain(&app, id, &rollbacks).await;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        assert_chain_published(&mods, &outcome, &rollbacks);
        let rows = mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap();
        for name in CHAIN {
            assert!(
                rows.iter().any(|row| row.filename == format!("{name}.jar")),
                "{name} is cached"
            );
        }
        assert!(rows.iter().all(|row| row.filename != "broken.jar"));
    }

    /// People link an instance, its `instance` folder or its `mods` folder
    /// elsewhere (a symlink, or a junction on Windows) to share mods between
    /// instances or to move one big instance to another drive, and 2.0.40
    /// followed those links. Installing with dependencies, updating,
    /// disabling, enabling and deleting all keep working through each link,
    /// the files land in its target, and the link itself stays.
    async fn assert_addons_work_through_link(
        link_dir: fn(&std::path::Path, &std::path::Path),
        linked: &str,
    ) {
        let app = crate::setup_managers_for_test().await;
        let (id, row, _) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            &format!("linked-{linked}"),
            instance::AddonType::Mods,
            "existing.jar",
            b"v1",
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let link = match linked {
            "root" => path.get_root(),
            "instance" => path.get_data_path(),
            _ => path.get_mods_path(),
        };
        let target = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join("elsewhere")
            .join(linked);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::rename(&link, &target).unwrap();
        link_dir(&link, &target);
        let moved = match linked {
            "root" => target.join("instance").join("mods"),
            "instance" => target.join("mods"),
            _ => target.clone(),
        };

        let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let outcome = install_chain(&app, id, &rollbacks).await;
        assert_chain_published(&moved, &outcome, &rollbacks);

        let update = mr("existing.jar", b"v2");
        let plan =
            publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
                .await
                .unwrap();
        let stage = tempfile::tempdir().unwrap();
        let bytes = stage.path().join("artifact");
        std::fs::write(&bytes, b"v2").unwrap();
        publication::publish_file(&update, &app, id, &bytes, Some(&plan), None)
            .await
            .unwrap();
        assert_eq!(std::fs::read(moved.join("existing.jar")).unwrap(), b"v2");
        assert!(update.is_already_installed(&app, id).await.unwrap());

        let manager = app.instance_manager();
        manager.enable_mod(id, row.id.clone(), false).await.unwrap();
        assert!(moved.join("existing.jar.disabled").is_file());
        manager.enable_mod(id, row.id.clone(), true).await.unwrap();
        assert!(moved.join("existing.jar").is_file());
        manager.delete_mod(id, row.id.clone()).await.unwrap();
        assert!(!moved.join("existing.jar").exists());
        assert!(
            mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    fn symlink(link: &std::path::Path, target: &std::path::Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn linked_instance_folders_keep_every_addon_action_working() {
        for linked in ["root", "instance", "mods"] {
            assert_addons_work_through_link(symlink, linked).await;
        }
    }

    #[cfg(windows)]
    fn junction(link: &std::path::Path, target: &std::path::Path) {
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed for {}", link.display());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn junctioned_instance_folders_keep_every_addon_action_working() {
        for linked in ["root", "instance", "mods"] {
            assert_addons_work_through_link(junction, linked).await;
        }
    }

    /// A linked folder whose target is gone (an unplugged drive), or a file
    /// where the folder belongs, refuses with a reason and a way out.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_dangling_folder_link_or_a_file_refuses_with_a_reason() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, existing) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            "dangling-link",
            instance::AddonType::Mods,
            "existing.jar",
            b"v1",
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let resourcepacks = path.get_resourcepacks_path();
        std::os::unix::fs::symlink(path.get_root().join("unplugged"), &resourcepacks).unwrap();
        let error = publication::checked_folder(&path, instance::AddonType::ResourcePacks)
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("is a link to a folder that cannot be found"),
            "{error:#}"
        );
        std::fs::write(path.get_shaderpacks_path(), b"a file").unwrap();
        let error = publication::checked_folder(&path, instance::AddonType::Shaders)
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("is a file, not a folder"),
            "{error:#}"
        );
        // Other addon folders are unaffected.
        assert_eq!(
            publication::checked_folder(&path, instance::AddonType::Mods)
                .await
                .unwrap(),
            existing.parent().unwrap()
        );
    }

    /// Saves shared between instances through a link (a symlink, or a
    /// junction on Windows): a world installs into the link's target, another
    /// version of a world installs beside it as a copy, a copy a failed
    /// install rolls back leaves the target again, and Delete removes a world
    /// from there. The link itself stays.
    async fn assert_worlds_work_through_linked_saves(
        link_dir: fn(&std::path::Path, &std::path::Path),
    ) {
        let app = crate::setup_managers_for_test().await;
        let (id, row, original) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            "linked-saves",
            instance::AddonType::Worlds,
            "World",
            b"original",
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let saves = original.parent().unwrap().to_path_buf();
        let target = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join("shared-saves");
        std::fs::rename(&saves, &target).unwrap();
        link_dir(&saves, &target);

        let archive = tempfile::NamedTempFile::new().unwrap();
        zip(archive.path(), &[("NewWorld/level.dat", b"new world")]);
        let mut installer = cf(
            "NewWorld.zip",
            &std::fs::read(archive.path()).unwrap(),
            curseforge::ClassId::Worlds,
        )
        .into_installer();
        installer.download_fixture = Some(archive.path().into());
        let task_id = installer.install(&app, id, false, None).await.unwrap();
        wait_for_install(&installer, &app, task_id).await;
        assert_eq!(
            std::fs::read(target.join("NewWorld/level.dat")).unwrap(),
            b"new world"
        );

        let version = tempfile::NamedTempFile::new().unwrap();
        zip(version.path(), &[("World/level.dat", b"new version")]);
        let mut installer = cf(
            "World.zip",
            &std::fs::read(version.path()).unwrap(),
            curseforge::ClassId::Worlds,
        )
        .into_installer();
        installer.download_fixture = Some(version.path().into());
        let task_id = installer
            .install(&app, id, false, Some(row.id.clone()))
            .await
            .unwrap();
        wait_for_install(&installer, &app, task_id).await;
        assert_eq!(
            std::fs::read(target.join("World (copy 1)/level.dat")).unwrap(),
            b"new version"
        );
        assert_eq!(
            std::fs::read(target.join("World/level.dat")).unwrap(),
            b"original"
        );

        let receipt = Arc::new(Mutex::new(None));
        let operation = app.instance_manager().lock_instance(id).await;
        let reserved = publication::prepare_world(version.path(), "World")
            .await
            .unwrap()
            .reserve_copy(&saves)
            .await
            .unwrap();
        let rolled_back = target.join(reserved.destinations()[0].file_name().unwrap());
        reserved
            .finish_owned(path.get_root(), &receipt)
            .await
            .unwrap();
        drop(operation);
        assert!(rolled_back.join("level.dat").is_file());
        publication::rollback_receipt(&app, id, &receipt)
            .await
            .unwrap();
        assert!(!rolled_back.exists());

        app.instance_manager()
            .delete_mod(id, row.id.clone())
            .await
            .unwrap();
        assert!(!target.join("World").exists());
        assert_eq!(
            std::fs::read(target.join("World (copy 1)/level.dat")).unwrap(),
            b"new version"
        );
        assert!(
            std::fs::symlink_metadata(&saves)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn worlds_install_copy_roll_back_and_delete_through_a_linked_saves_folder() {
        assert_worlds_work_through_linked_saves(symlink).await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn worlds_install_copy_roll_back_and_delete_through_a_junctioned_saves_folder() {
        assert_worlds_work_through_linked_saves(junction).await;
    }

    /// The copy an update, a version switch or an install would replace is
    /// there twice (as `name` and `name.disabled`, one row): GDLauncher does
    /// not pick one, says so, and points to Delete, which removes one copy
    /// at a time. Nothing changes.
    #[tokio::test]
    async fn updating_or_installing_over_two_copies_explains_the_way_out() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            "twin-update",
            instance::AddonType::Mods,
            "twin.jar",
            b"v1",
        )
        .await;
        let disabled = path.with_file_name("twin.jar.disabled");
        std::fs::write(&disabled, b"old").unwrap();
        let expect = |error: anyhow::Error, verb: &str| {
            let text = format!("{error:#}");
            assert!(
                text.contains("Both twin.jar and twin.jar.disabled are in")
                    && text.contains(&format!("which copy to {verb}"))
                    && text.contains("Delete in the Addons tab removes one copy at a time"),
                "{text}"
            );
        };
        let staged = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(staged.path(), b"v2").unwrap();
        let mut installer = cf("twin-2.jar", b"v2", curseforge::ClassId::Mods).into_installer();
        installer.download_fixture = Some(staged.path().into());
        expect(
            installer
                .install(&app, id, false, Some(row.id.clone()))
                .await
                .unwrap_err(),
            "update",
        );
        expect(
            mr("twin.jar", b"v2")
                .is_already_installed(&app, id)
                .await
                .unwrap_err(),
            "use",
        );
        assert!(!path.with_file_name("twin-2.jar").exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"v1");
        assert_eq!(std::fs::read(&disabled).unwrap(), b"old");
    }

    /// A mod that is a link to a jar shared with other instances is not
    /// replaced in place (the jar it points to may be in use elsewhere):
    /// updating it, switching its version or installing over it says it is
    /// a link and points to Delete, which removes only the link. Nothing
    /// changes, and the shared jar stays.
    #[cfg(unix)]
    #[tokio::test]
    async fn updating_or_installing_over_a_linked_mod_explains_the_way_out() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, path) = crate::managers::instance::mods::tests::addon_fixture(
            &app,
            "link-update",
            instance::AddonType::Mods,
            "shared.jar",
            b"v1",
        )
        .await;
        let shared = tempfile::tempdir().unwrap();
        let target = shared.path().join("shared.jar");
        std::fs::rename(&path, &target).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        let expect = |error: anyhow::Error| {
            let text = format!("{error:#}");
            assert!(
                text.contains("shared.jar in ")
                    && text.contains("is a link to a file stored elsewhere")
                    && text.contains("Delete it in the Addons tab")
                    && text.contains("only the link is removed"),
                "{text}"
            );
        };
        let download = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(download.path(), b"v2").unwrap();
        let mut installer = cf("shared-2.jar", b"v2", curseforge::ClassId::Mods).into_installer();
        installer.download_fixture = Some(download.path().into());
        expect(
            installer
                .install(&app, id, false, Some(row.id.clone()))
                .await
                .unwrap_err(),
        );
        expect(
            mr("shared.jar", b"v2")
                .is_already_installed(&app, id)
                .await
                .unwrap_err(),
        );
        // A download that would land on the link's name.
        let stage = tempfile::tempdir().unwrap();
        let staged = stage.path().join("artifact");
        std::fs::write(&staged, b"v2").unwrap();
        expect(
            publication::publish_file(&mr("shared.jar", b"v2"), &app, id, &staged, None, None)
                .await
                .unwrap_err(),
        );
        assert!(!path.with_file_name("shared-2.jar").exists());
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"v1");
        // The way out: Delete removes the link, then the install works.
        app.instance_manager()
            .delete_mod(id, row.id.clone())
            .await
            .unwrap();
        publication::publish_file(&mr("shared.jar", b"v2"), &app, id, &staged, None, None)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"v2");
        assert!(
            !std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"v1");
    }
}

#[tokio::test]
async fn task10_required_disabled_project_is_not_satisfied() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "required-disabled",
        instance::AddonType::Mods,
        "dep.jar",
        b"dep",
    )
    .await;
    carbon_repos::repos::mod_metadata::upsert_cf_mod_cache(
        &app.db,
        0,
        42,
        7,
        "dep".into(),
        "1".into(),
        "dep".into(),
        "".into(),
        "".into(),
        1,
        "".into(),
        carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().fixed_offset()),
        row.metadata_id,
    )
    .await
    .unwrap();
    app.instance_manager()
        .enable_mod(id, row.id, false)
        .await
        .unwrap();
    let mut resource = cf("parent.jar", b"parent", curseforge::ClassId::Mods);
    resource.file.dependencies = serde_json::from_value(serde_json::json!([
        {"modId":42,"relationType":3}
    ]))
    .unwrap();
    let getter = {
        let manager = app.instance_manager();
        let instances = manager.instances.read().await;
        resource
            .dependencies(&app, id, instances[&id].data().unwrap(), ModChannel::Stable)
            .next()
            .unwrap()
    };
    let result = getter(std::collections::HashMap::new()).await;
    assert!(
        result.is_some(),
        "disabled required project must not be silently skipped"
    );
    assert!(
        result
            .unwrap()
            .err()
            .is_some_and(|error| error.to_string().contains("disabled"))
    );
    assert!(!path.exists());
    assert_eq!(
        std::fs::read(path.with_file_name("dep.jar.disabled")).unwrap(),
        b"dep"
    );
}

struct Task10ReplacementResource {
    resource: CurseforgeModInstaller,
    discovered: Arc<std::sync::atomic::AtomicBool>,
}
#[async_trait::async_trait]
impl ResourceInstaller for Task10ReplacementResource {
    fn id(&self) -> String {
        self.resource.id()
    }
    fn display_name(&self) -> String {
        self.resource.display_name()
    }
    fn get_install_path(&self, path: &InstancePath) -> PathBuf {
        self.resource.get_install_path(path)
    }
    async fn downloadable(&self, path: &InstancePath) -> Option<Downloadable> {
        self.resource.downloadable(path).await
    }
    async fn post_process(
        &self,
        path: &InstancePath,
        staged: &std::path::Path,
    ) -> anyhow::Result<()> {
        self.resource.post_process(path, staged).await
    }
    async fn is_already_installed(
        &self,
        app: &Arc<AppInner>,
        id: InstanceId,
    ) -> anyhow::Result<bool> {
        self.resource.is_already_installed(app, id).await
    }
    async fn rollback(&self, _: &mut InstanceData) -> anyhow::Result<()> {
        Ok(())
    }
    fn dependencies(
        &self,
        _: &Arc<AppInner>,
        _: InstanceId,
        _: &InstanceData,
        _: ModChannel,
    ) -> DependencyIterator {
        self.discovered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let getter: ResourceInstallerGetter = Box::new(|_| {
            Box::pin(async { Some(Err(anyhow::anyhow!("required library unavailable"))) })
        });
        DependencyIterator::new(vec![getter].into_iter())
    }
}

#[tokio::test]
async fn task10_replacement_preserves_old_on_required_resolution_failure() {
    let app = crate::setup_managers_for_test().await;
    assert!(
        app.settings_manager()
            .get_settings()
            .await
            .unwrap()
            .download_dependencies
    );
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "update-required",
        instance::AddonType::Mods,
        "parent.jar",
        b"old",
    )
    .await;
    let discovered = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut installer = Installer::new(Box::new(Task10ReplacementResource {
        resource: cf("parent.jar", b"new", curseforge::ClassId::Mods),
        discovered: discovered.clone(),
    }));
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"new").unwrap();
    installer.download_fixture = Some(source.path().into());
    let result = installer.install(&app, id, false, Some(row.id)).await;
    let completion = installer.abort_handle.lock().await.completion.clone();
    wait_install_completion(completion).await.unwrap();
    assert!(
        discovered.load(std::sync::atomic::Ordering::SeqCst),
        "normal replacement must discover required dependencies when setting enabled"
    );
    assert_eq!(std::fs::read(path).unwrap(), b"old");
    assert!(
        result.is_err()
            || app
                .task_manager()
                .get_task(result.unwrap())
                .await
                .is_some_and(|t| matches!(t.progress, crate::domain::vtask::Progress::Failed(_)))
    );
}

#[test]
fn task10_required_catalogue_rejects_incompatible_and_accepts_later_match() {
    use domain::instance::info::{ModLoader, ModLoaderType};
    let target = (
        "1.20.1".into(),
        [ModLoader {
            type_: ModLoaderType::Fabric,
            version: "0.15".into(),
        }]
        .into_iter()
        .collect(),
    );
    let mut incompatible = mr("wrong.jar", b"wrong").version;
    incompatible.game_versions = vec!["1.21".into()];
    incompatible.loaders = vec!["forge".into()];
    let versions = vec![incompatible];
    let result = select_dependency(&versions, Some(&target), false, ModChannel::Stable, |v| {
        (
            &v.game_versions,
            &v.loaders,
            ModChannel::from(v.version_type),
        )
    });
    assert!(
        result.is_err(),
        "a nonempty incompatible catalogue is not a compatible dependency"
    );
}

#[test]
fn task10_loader_independent_pack_selects_matching_game() {
    let target = ("1.20.1".into(), std::collections::HashSet::new());
    let mut wrong = mr("wrong.zip", b"wrong").version;
    wrong.game_versions = vec!["1.21".into()];
    let versions = vec![wrong, mr("correct.zip", b"correct").version];
    let selected = select_dependency(&versions, Some(&target), true, ModChannel::Stable, |v| {
        (
            &v.game_versions,
            &v.loaders,
            ModChannel::from(v.version_type),
        )
    })
    .unwrap();
    assert_eq!(selected.files[0].filename, "correct.zip");
}

#[tokio::test]
async fn task10_streamed_progress_arrives_before_transfer_eof() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let app = crate::setup_managers_for_test().await;
    let (id, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "byte-progress",
        instance::AddonType::Mods,
        "existing.jar",
        b"existing",
    )
    .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/artifact", listener.local_addr().unwrap());
    let (sent, received) = tokio::sync::oneshot::channel();
    let fixture = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        assert!(request.starts_with(b"GET /artifact HTTP/1.1\r\n"));
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 524288\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        socket.write_all(&vec![42; 262144]).await.unwrap();
        let _ = sent.send(());
        let mut byte = [0];
        let _ = socket.read(&mut byte).await;
    });
    let mut resource = cf("large.jar", &vec![42; 524288], curseforge::ClassId::Mods);
    resource.download_url = url;
    let installer = resource.into_installer();
    let task = installer.install(&app, id, false, None).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), received)
        .await
        .unwrap()
        .unwrap();
    let observed = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let current = app.task_manager().get_task(task).await.unwrap();
            if current.downloaded > 0 && current.downloaded < 524288 {
                break current;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    installer.abort(None).await.unwrap();
    fixture.await.unwrap();
    assert!(
        observed.is_ok(),
        "real streamed byte progress must reach the visual task before EOF"
    );
}

#[tokio::test]
async fn task10_receipt_rollback_restores_old_and_preserves_later_edits() {
    for mode in ["owned", "edited", "equal-later-occupant"] {
        let app = crate::setup_managers_for_test().await;
        let (id, row, old) = super::super::mods::tests::addon_fixture(
            &app,
            mode,
            instance::AddonType::Mods,
            "same.jar",
            b"old",
        )
        .await;
        let plan =
            publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
                .await
                .unwrap();
        let resource = cf("same.jar", b"new", curseforge::ClassId::Mods);
        let stage = tempfile::tempdir().unwrap();
        let staged = stage.path().join("artifact");
        std::fs::write(&staged, b"new").unwrap();
        let receipt = Arc::new(Mutex::new(None));
        publication::publish_file_owned(
            &resource,
            &app,
            id,
            &staged,
            Some(&plan),
            None,
            &receipt,
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read(stage.path().join("previous-addon")).unwrap(),
            b"old"
        );
        if mode == "edited" {
            std::fs::write(&old, b"user edit").unwrap();
        }
        if mode == "equal-later-occupant" {
            let other = stage.path().join("other");
            std::fs::write(&other, b"new").unwrap();
            // Windows has no inode in the stamp, only the times. NTFS keeps
            // them at clock-tick resolution and tunnels the replaced name's
            // creation time onto this file, so equal bytes written within the
            // same tick look like the publication; a real later file was
            // written at another time.
            #[cfg(windows)]
            std::fs::OpenOptions::new()
                .write(true)
                .open(&other)
                .unwrap()
                .set_modified(std::time::SystemTime::now() - Duration::from_secs(3600))
                .unwrap();
            std::fs::rename(other, &old).unwrap();
        }
        let result = publication::rollback_receipt(&app, id, &receipt).await;
        if mode == "owned" {
            result.unwrap();
            assert_eq!(std::fs::read(&old).unwrap(), b"old");
            assert!(
                mfcdb::get_mod_file_cache_by_id(&app.db, &row.id)
                    .await
                    .unwrap()
                    .is_some()
            );
        } else {
            assert!(result.is_err());
            assert_eq!(
                std::fs::read(&old).unwrap(),
                if mode == "edited" {
                    b"user edit".as_slice()
                } else {
                    b"new".as_slice()
                }
            );
            assert_eq!(
                std::fs::read(stage.path().join("previous-addon")).unwrap(),
                b"old"
            );
        }
    }
}

#[tokio::test]
async fn task10_setup_policy_preserves_initial_and_imported_but_refuses_pending() {
    use super::super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, _, _) = super::super::mods::tests::addon_fixture(
        &app,
        "setup-policy",
        instance::AddonType::Mods,
        "base.jar",
        b"base",
    )
    .await;
    let path = publication::current_path(&app, id).await.unwrap();
    let original = Session::load(&path.get_root()).await.unwrap().unwrap();
    let record = path.get_root().join(".setup/setup-session.json");
    {
        let manager = app.instance_manager();
        manager
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()
            .unwrap()
            .config
            .name = "independent name edit".into();
    }
    app.instance_manager()
        .ensure_modpack_not_locked(id)
        .await
        .unwrap();
    let mut imported = original.clone();
    imported.producer = Producer::Import;
    imported.kind = Kind::ImportedComplete;
    imported.phase = Phase::Committed;
    std::fs::write(&record, serde_json::to_vec(&imported).unwrap()).unwrap();
    app.instance_manager()
        .ensure_modpack_not_locked(id)
        .await
        .unwrap();
    // A record naming an id the folder no longer has (a database reset gave
    // it a new one) is continued as this instance's own, so edits go ahead
    // as beside its own, without touching the record.
    let mut renumbered = original.clone();
    renumbered.instance_id = Some(*id + 999);
    let renumbered = serde_json::to_vec(&renumbered).unwrap();
    std::fs::write(&record, &renumbered).unwrap();
    app.instance_manager()
        .ensure_modpack_not_locked(id)
        .await
        .unwrap();
    assert_eq!(std::fs::read(&record).unwrap(), renumbered);
    for mode in ["pack", "repair", "unknown"] {
        let mut session = original.clone();
        match mode {
            "pack" => session.kind = Kind::Pack,
            "repair" => session.producer = Producer::Repair,
            _ => {}
        }
        std::fs::write(
            &record,
            if mode == "unknown" {
                b"invalid".to_vec()
            } else {
                serde_json::to_vec(&session).unwrap()
            },
        )
        .unwrap();
        assert!(
            app.instance_manager()
                .ensure_modpack_not_locked(id)
                .await
                .is_err(),
            "{mode}"
        );
    }
    std::fs::write(&record, serde_json::to_vec(&original).unwrap()).unwrap();
    let ambiguous_target = path.get_root().join(".setup/staging-packinfo.json");
    std::fs::write(&ambiguous_target, b"{}").unwrap();
    assert!(
        app.instance_manager()
            .ensure_modpack_not_locked(id)
            .await
            .is_err()
    );
    std::fs::remove_file(ambiguous_target).unwrap();
    // The first install of a new instance, or Play, holds no addon folder:
    // edits go ahead while it is queued or preparing, as in 2.0.40. A
    // deletion still holds them.
    for state in [
        super::super::run::LaunchState::Queued(VisualTaskId(900)),
        super::super::run::LaunchState::Preparing(VisualTaskId(900)),
        super::super::run::LaunchState::Deleting,
    ] {
        let deleting = matches!(state, super::super::run::LaunchState::Deleting);
        {
            app.instance_manager()
                .instances
                .write()
                .await
                .get_mut(&id)
                .unwrap()
                .data_mut()
                .unwrap()
                .state = state;
        }
        assert_eq!(
            app.instance_manager()
                .ensure_modpack_not_locked(id)
                .await
                .is_err(),
            deleting
        );
    }
}

/// Play pressed while an addon still downloads: the install was admitted on
/// an idle instance, so it publishes and commits while the game prepares, as
/// in 2.0.40, instead of failing and removing the new file again.
#[tokio::test]
async fn install_admitted_before_play_finishes_while_the_game_prepares() {
    let app = crate::setup_managers_for_test().await;
    let (id, _, base) = super::super::mods::tests::addon_fixture(
        &app,
        "play-during-install",
        instance::AddonType::Mods,
        "base.jar",
        b"base",
    )
    .await;
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"new").unwrap();
    let mut installer = cf("new.jar", b"new", curseforge::ClassId::Mods).into_installer();
    installer.download_fixture = Some(source.path().into());
    let identity = installer.inner.lock().await.id();
    let pause = app
        .instance_manager()
        .test_hooks
        .arm_path("addon_graph_before_member_publish", &identity);
    let task = installer.install(&app, id, false, None).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), pause.entered.notified())
        .await
        .unwrap();
    app.instance_manager()
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()
        .unwrap()
        .state = super::super::run::LaunchState::Preparing(VisualTaskId(901));
    pause.release.notify_one();
    wait_for_install(&installer, &app, task).await;
    assert_eq!(
        std::fs::read(base.with_file_name("new.jar")).unwrap(),
        b"new"
    );
    assert!(matches!(
        settled_outcome(&app, task).await,
        crate::managers::vtask::TaskOutcome::Succeeded
    ));
}

/// A task's outcome once it stopped running.
async fn settled_outcome(
    app: &Arc<AppInner>,
    task: VisualTaskId,
) -> crate::managers::vtask::TaskOutcome {
    let mut outcome = app.task_manager().get_task_outcome(task).await;
    for _ in 0..400 {
        if !matches!(outcome, crate::managers::vtask::TaskOutcome::Running(_)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
        outcome = app.task_manager().get_task_outcome(task).await;
    }
    outcome
}

/// The install attempts' staging folders left in the data folder.
fn staging_left(app: &Arc<AppInner>) -> Vec<String> {
    std::fs::read_dir(app.settings_manager().runtime_path.get_root().to_path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(publication::STAGING_PREFIX))
        .collect()
}

/// Every update and every install that fails after its download used to
/// leave a hidden `.addon-install-*` folder in the data folder, the failed
/// ones with the whole download in it. Neither leaves one now.
#[tokio::test]
async fn updates_and_failed_installs_leave_no_staging_in_the_data_folder() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "staging-cleanup",
        instance::AddonType::Mods,
        "old.jar",
        b"original",
    )
    .await;
    // An update: the replaced version waits in staging until it commits.
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"new version").unwrap();
    let mut installer = cf("new.jar", b"new version", curseforge::ClassId::Mods).into_installer();
    installer.download_fixture = Some(source.path().into());
    let task = installer
        .install(&app, id, false, Some(row.id.clone()))
        .await
        .unwrap();
    wait_for_install(&installer, &app, task).await;
    assert_eq!(
        std::fs::read(old.with_file_name("new.jar")).unwrap(),
        b"new version"
    );
    assert!(!old.exists());
    assert_eq!(staging_left(&app), Vec::<String>::new(), "after an update");

    // An install refused once downloaded: another file has its name.
    let taken = old.with_file_name("taken.jar");
    std::fs::write(&taken, b"someone else's").unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"mine").unwrap();
    let mut installer = cf("taken.jar", b"mine", curseforge::ClassId::Mods).into_installer();
    installer.download_fixture = Some(source.path().into());
    let task = installer.install(&app, id, false, None).await.unwrap();
    tokio::time::timeout(
        Duration::from_secs(20),
        wait_install_completion(installer.abort_handle.lock().await.completion.clone()),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        settled_outcome(&app, task).await,
        crate::managers::vtask::TaskOutcome::Failed(_)
    ));
    assert_eq!(std::fs::read(&taken).unwrap(), b"someone else's");
    assert_eq!(
        staging_left(&app),
        Vec::<String>::new(),
        "after a failed install"
    );
}

/// At startup no install runs: what earlier sessions left goes, except a
/// replaced addon that was never put back, which stays with its folder.
#[tokio::test]
async fn startup_sweep_keeps_only_replaced_addons() {
    let root = tempfile::tempdir().unwrap();
    let make = |folder: &str, files: &[&str]| {
        let folder = root.path().join(folder);
        std::fs::create_dir_all(&folder).unwrap();
        for file in files {
            std::fs::write(folder.join(file), file.as_bytes()).unwrap();
        }
        folder
    };
    let downloaded = make(".addon-install-a1", &["artifact"]);
    let empty = make(".addon-install-a2", &[]);
    let extracted = make(".addon-install-a3", &["artifact"]);
    make(".addon-install-a3/.tmp-extract/World", &["level.dat"]);
    let replaced = make(".addon-install-a4", &["artifact", "previous-addon"]);
    let server = make(".server-addon-s1", &["0-lib.jar", "1-rails.jar"]);
    let server_replaced = make(".server-addon-s2", &["0-rails.jar", "previous-0"]);
    let unrelated = make("instances", &["artifact"]);
    let lookalike = make(".addon-installer", &["artifact"]);
    std::fs::write(root.path().join(".addon-install-file"), b"not a folder").unwrap();

    publication::sweep_stale_staging(root.path()).await;

    for gone in [&downloaded, &empty, &extracted, &server] {
        assert!(!gone.exists(), "{}", gone.display());
    }
    assert_eq!(
        std::fs::read(replaced.join("previous-addon")).unwrap(),
        b"previous-addon"
    );
    assert!(!replaced.join("artifact").exists());
    assert_eq!(
        std::fs::read(server_replaced.join("previous-0")).unwrap(),
        b"previous-0"
    );
    assert!(!server_replaced.join("0-rails.jar").exists());
    assert!(unrelated.join("artifact").exists());
    assert!(lookalike.join("artifact").exists());
    assert!(root.path().join(".addon-install-file").exists());
}

/// Rolling back an update that renamed the addon puts the earlier version
/// back first. When that fails, the new version stays in the instance, and
/// the error says where the earlier one is: the instance never has neither.
#[tokio::test]
async fn rollback_that_cannot_put_the_old_version_back_keeps_the_new_one() {
    use crate::util::publish::fault::{IN_USE, Step, inject, pending};
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "rollback-order",
        instance::AddonType::Mods,
        "old.jar",
        b"old",
    )
    .await;
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let stage = tempfile::tempdir().unwrap();
    let staged = stage.path().join("artifact");
    std::fs::write(&staged, b"new").unwrap();
    let receipt = Arc::new(Mutex::new(None));
    publication::publish_file_owned(
        &cf("new.jar", b"new", curseforge::ClassId::Mods),
        &app,
        id,
        &staged,
        Some(&plan),
        None,
        &receipt,
        None,
    )
    .await
    .unwrap();
    inject(Step::NoReplace, &old, IN_USE);
    let error = publication::rollback_receipt(&app, id, &receipt)
        .await
        .unwrap_err();
    assert!(!pending(Step::NoReplace, &old), "the restore was tried");
    let backup = stage.path().join("previous-addon");
    assert!(
        format!("{error:#}").contains(&backup.display().to_string()),
        "{error:#}"
    );
    assert_eq!(
        std::fs::read(old.with_file_name("new.jar")).unwrap(),
        b"new"
    );
    assert_eq!(std::fs::read(&backup).unwrap(), b"old");
    assert!(!old.exists());
}

#[tokio::test]
async fn task10_current_future_sizes_are_bounded() {
    let app = crate::setup_managers_for_test().await;
    let resource = cf("test.jar", b"new", curseforge::ClassId::Mods);
    let stage = tempfile::NamedTempFile::new().unwrap();
    let future =
        publication::publish_file(&resource, &app, InstanceId(1), stage.path(), None, None);
    let publication_size = std::mem::size_of_val(&future);
    println!("publish_file future bytes: {publication_size}");
    assert!(publication_size < 131072);
    drop(future);
    let installer = resource.into_installer();
    let future = installer.install(&app, InstanceId(1), false, None);
    let installer_size = std::mem::size_of_val(&future);
    println!("Installer::install future bytes: {installer_size}");
    assert!(installer_size < 262144);
}

mod task10_graph {
    use super::*;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[derive(Clone)]
    struct Node {
        name: String,
        children: Vec<String>,
        url: String,
        all: Arc<HashMap<String, Vec<String>>>,
        exact: bool,
    }
    #[async_trait::async_trait]
    impl ResourceInstaller for Node {
        fn exact_dependency(&self) -> bool {
            self.exact
        }
        fn id(&self) -> String {
            format!("fixture:{}", self.name)
        }
        fn display_name(&self) -> String {
            self.name.clone()
        }
        fn get_install_path(&self, path: &InstancePath) -> PathBuf {
            path.get_mods_path()
                .join(format!("{}.jar", self.name.replace(':', "-")))
        }
        async fn downloadable(&self, path: &InstancePath) -> Option<Downloadable> {
            use sha1::Digest;
            Some(
                Downloadable::new(
                    format!("{}/{}", self.url, self.name),
                    self.get_install_path(path),
                )
                .with_size(4)
                .with_checksum(Some(Checksum::Sha1(hex::encode(sha1::Sha1::digest(
                    b"test",
                ))))),
            )
        }
        async fn post_process(&self, _: &InstancePath, _: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
        async fn is_already_installed(
            &self,
            app: &Arc<AppInner>,
            id: InstanceId,
        ) -> anyhow::Result<bool> {
            publication::is_satisfied(self, app, id).await
        }
        async fn rollback(&self, _: &mut InstanceData) -> anyhow::Result<()> {
            Ok(())
        }
        fn dependencies(
            &self,
            _: &Arc<AppInner>,
            _: InstanceId,
            _: &InstanceData,
            _: ModChannel,
        ) -> DependencyIterator {
            let children = self
                .children
                .iter()
                .map(|name| {
                    let name = name.clone();
                    let all = self.all.clone();
                    let url = self.url.clone();
                    Box::new(move |selections: HashMap<String, String>| {
                        Box::pin(async move {
                            let exact = !name.ends_with(":*");
                            let name = if exact {
                                name
                            } else {
                                let project = name.trim_end_matches(":*");
                                selections
                                    .get(&format!("fixture:{project}"))
                                    .map(|id| id.trim_start_matches("fixture:").to_owned())
                                    .unwrap_or_else(|| format!("{project}:2"))
                            };
                            let node = Node {
                                children: all.get(&name).cloned().unwrap_or_default(),
                                name,
                                url,
                                all,
                                exact,
                            };
                            Some(Ok(Box::new(node) as BoxedResourceInstaller))
                        })
                            as Pin<
                                Box<
                                    dyn Future<
                                            Output = Option<anyhow::Result<BoxedResourceInstaller>>,
                                        > + Send,
                                >,
                            >
                    }) as ResourceInstallerGetter
                })
                .collect::<Vec<_>>();
            DependencyIterator::new(children.into_iter())
        }
    }
    async fn round1(mode: &str) {
        let app = crate::setup_managers_for_test().await;
        let satisfied = mode.starts_with("satisfied");
        let (id, row, old) = super::super::super::mods::tests::addon_fixture(
            &app,
            mode,
            instance::AddonType::Mods,
            "a-1.jar",
            if satisfied { b"test" } else { b"old" },
        )
        .await;
        let manager = app.instance_manager();
        let operation = manager.lock_instance(id).await;
        app.meta_cache_manager()
            .refresh_published_addon(
                &operation,
                &old.parent().unwrap().to_path_buf(),
                row.filename.clone(),
                true,
                row.addon_type.clone(),
            )
            .await
            .unwrap();
        drop(operation);
        let children = if satisfied {
            vec!["b:1".to_owned(), "c:1".to_owned()]
        } else {
            vec![if mode == "self-exact" { "a:1" } else { "a:*" }.to_owned()]
        };
        let all = Arc::new(HashMap::from([("a:1".into(), children.clone())]));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = CancellationToken::new();
        let fixture_stop = stop.clone();
        let fixture = tokio::spawn(async move {
            loop {
                let (mut socket, _) = tokio::select! { value = listener.accept() => value.unwrap(), _ = fixture_stop.cancelled() => break };
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                }
                assert!(request.starts_with(b"GET /"));
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest",
                    )
                    .await;
            }
        });
        let installer = Arc::new(Installer::new(Box::new(Node {
            name: "a:1".into(),
            children,
            url,
            all,
            exact: true,
        })));
        let paused_identity = if satisfied {
            "fixture:c:1"
        } else {
            "fixture:a:1"
        };
        let pause = (satisfied || mode == "self-cancel").then(|| {
            manager
                .test_hooks
                .arm_path("addon_graph_before_member_publish", paused_identity)
        });
        let accepted = tokio::time::timeout(
            Duration::from_secs(2),
            installer.install(&app, id, true, if satisfied { None } else { Some(row.id) }),
        )
        .await;
        if accepted.is_err() {
            stop.cancel();
            fixture.await.unwrap();
            assert!(
                installer.abort(None).await.is_err(),
                "dropped blocked admission remains Unknown"
            );
            panic!(
                "consistent self-cycle must finish admission without recursively locking the root resource"
            );
        }
        let task = accepted.unwrap().unwrap();
        let ownership = installer.ownership.clone();
        if let Some(pause) = pause {
            tokio::time::timeout(Duration::from_secs(10), pause.entered.notified())
                .await
                .unwrap();
            if satisfied {
                assert_eq!(
                    std::fs::read(old.with_file_name("b-1.jar")).unwrap(),
                    b"test",
                    "B must already have published before C fails"
                );
                let first = ownership.children.lock().unwrap()[0].clone();
                wait_install_completion(first.owner.lock().await.completion.clone())
                    .await
                    .unwrap();
                if mode == "satisfied-edited" {
                    std::fs::write(old.with_file_name("b-1.jar"), b"later edit").unwrap();
                }
                if mode == "satisfied-unknown" {
                    let child = ownership.children.lock().unwrap()[1].clone();
                    child.owner.lock().await.handle.as_ref().unwrap().abort();
                } else if mode != "satisfied-success" {
                    std::fs::write(old.with_file_name("c-1.jar"), b"later occupant").unwrap();
                }
                drop(installer); // Actual platform callers drop their local Installer on acceptance.
                pause.release.notify_one();
            } else {
                let worker = installer.clone();
                let mut abort = tokio::spawn(async move { worker.abort(None).await });
                assert!(
                    tokio::time::timeout(Duration::from_millis(50), &mut abort)
                        .await
                        .is_err()
                );
                pause.release.notify_one();
                tokio::time::timeout(Duration::from_secs(10), abort)
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
            }
        }
        let children = ownership.children.lock().unwrap().clone();
        for (index, child) in children.into_iter().enumerate() {
            let settled =
                wait_install_completion(child.owner.lock().await.completion.clone()).await;
            if mode == "satisfied-unknown" && index == 1 {
                assert!(settled.is_err());
            } else {
                settled.unwrap();
            }
        }
        wait_install_completion(ownership.owner.lock().await.completion.clone())
            .await
            .unwrap();
        stop.cancel();
        fixture.await.unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        if mode == "satisfied-success" {
            assert_eq!(std::fs::read(&old).unwrap(), b"test");
            assert_eq!(
                std::fs::read(old.with_file_name("b-1.jar")).unwrap(),
                b"test"
            );
            assert_eq!(
                std::fs::read(old.with_file_name("c-1.jar")).unwrap(),
                b"test"
            );
            assert!(matches!(
                app.task_manager().get_task_outcome(task).await,
                crate::managers::vtask::TaskOutcome::Succeeded
            ));
        } else if mode == "satisfied-unknown" {
            assert_eq!(
                std::fs::read(old.with_file_name("b-1.jar")).unwrap(),
                b"test",
                "Unknown forbids whole-operation rollback"
            );
            assert!(!old.with_file_name("c-1.jar").exists());
            assert!(matches!(
                app.task_manager().get_task_outcome(task).await,
                crate::managers::vtask::TaskOutcome::Failed(_)
            ));
        } else if satisfied {
            assert!(matches!(
                app.task_manager().get_task(task).await.unwrap().progress,
                crate::domain::vtask::Progress::Failed(_)
            ));
            assert_eq!(std::fs::read(&old).unwrap(), b"test");
            assert_eq!(
                std::fs::read(old.with_file_name("c-1.jar")).unwrap(),
                b"later occupant"
            );
            if mode == "satisfied-edited" {
                assert_eq!(
                    std::fs::read(old.with_file_name("b-1.jar")).unwrap(),
                    b"later edit"
                );
            } else {
                assert!(
                    !old.with_file_name("b-1.jar").exists(),
                    "failed satisfied-root graph must roll back its completed sibling B"
                );
            }
        } else if mode == "self-cancel" {
            assert_eq!(std::fs::read(&old).unwrap(), b"old");
        } else {
            assert_eq!(std::fs::read(&old).unwrap(), b"test");
            assert!(app.task_manager().get_task(task).await.is_none());
        }
    }
    #[tokio::test]
    async fn task10_r1_satisfied_root_finalizer_commits_usable_children() {
        round1("satisfied-success").await;
    }
    #[tokio::test]
    async fn task10_r1_satisfied_root_unknown_forbids_sibling_rollback() {
        round1("satisfied-unknown").await;
    }
    #[tokio::test]
    async fn task10_r1_satisfied_root_rolls_back_completed_sibling() {
        round1("satisfied").await;
    }
    #[tokio::test]
    async fn task10_r1_satisfied_root_retains_edited_sibling() {
        round1("satisfied-edited").await;
    }
    #[tokio::test]
    async fn task10_r1_exact_root_self_cycle_completes() {
        round1("self-exact").await;
    }
    #[tokio::test]
    async fn task10_r1_unpinned_root_self_cycle_completes() {
        round1("self-unpinned").await;
    }
    #[tokio::test]
    async fn task10_r1_root_self_cycle_cancellation_settles() {
        round1("self-cancel").await;
    }

    async fn run(mode: &str) {
        let app = crate::setup_managers_for_test().await;
        let (id, row, old) = super::super::super::mods::tests::addon_fixture(
            &app,
            mode,
            instance::AddonType::Mods,
            "a-1.jar",
            b"old",
        )
        .await;
        // The shared fixture seeds synthetic metadata; settle its real bytes before
        // capture so a descendant's normal whole-instance cache scan is not a
        // manufactured replacement-identity change.
        let manager = app.instance_manager();
        let operation = manager.lock_instance(id).await;
        app.meta_cache_manager()
            .refresh_published_addon(
                &operation,
                &old.parent().unwrap().to_path_buf(),
                row.filename.clone(),
                true,
                row.addon_type.clone(),
            )
            .await
            .unwrap();
        drop(operation);
        let row = manager.get_owned_mod_file(id, &row.id).await.unwrap();
        let edges: HashMap<String, Vec<String>> = match mode {
            "cycle" => [("a:1", vec!["b:1"]), ("b:1", vec!["a:1"])]
                .into_iter()
                .map(|(a, b)| (a.into(), b.into_iter().map(str::to_owned).collect()))
                .collect(),
            "older-cycle" => [("a:1", vec!["b:1"]), ("b:1", vec!["a:*"])]
                .into_iter()
                .map(|(a, b)| (a.into(), b.into_iter().map(str::to_owned).collect()))
                .collect(),
            "late-pin" | "early-pin" => [
                (
                    "a:1",
                    if mode == "late-pin" {
                        vec!["b:1", "c:1"]
                    } else {
                        vec!["c:1", "b:1"]
                    },
                ),
                ("b:1", vec!["p:*"]),
                ("c:1", vec!["p:1"]),
                ("p:2", vec!["obsolete:1"]),
            ]
            .into_iter()
            .map(|(a, b)| (a.into(), b.into_iter().map(str::to_owned).collect()))
            .collect(),
            "conflict" => [("a:1", vec!["b:1", "b:2"])]
                .into_iter()
                .map(|(a, b)| (a.into(), b.into_iter().map(str::to_owned).collect()))
                .collect(),
            _ => [
                ("a:1", vec!["b:1", "c:1"]),
                ("b:1", vec!["d:1"]),
                ("c:1", vec!["d:1"]),
            ]
            .into_iter()
            .map(|(a, b)| (a.into(), b.into_iter().map(str::to_owned).collect()))
            .collect(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = CancellationToken::new();
        let shutdown = stop.clone();
        let requests = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen = requests.clone();
        let fail = mode == "nested-failure";
        let fixture = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! { r = listener.accept() => r, _ = shutdown.cancelled() => break };
                let (mut socket, _) = accepted.unwrap();
                let mut request = Vec::new();
                loop {
                    match socket.read_u8().await {
                        Ok(byte) => request.push(byte),
                        Err(_) => break,
                    }
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                if request.is_empty() {
                    continue;
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with("GET /"));
                let name = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .trim_start_matches('/')
                    .to_owned();
                seen.lock().unwrap().push(name.clone());
                let bytes: &[u8] = if fail && name == "d:1" {
                    b"fail"
                } else {
                    b"test"
                };
                let _ = socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n")
                    .await;
                let _ = socket.write_all(bytes).await;
            }
        });
        let root = Node {
            name: "a:1".into(),
            children: edges["a:1"].clone(),
            url,
            all: Arc::new(edges),
            exact: true,
        };
        let installer = Installer::new(Box::new(root));
        let pause = matches!(mode, "published-rollback" | "edited-dependency").then(|| {
            app.instance_manager()
                .test_hooks
                .arm("addon_graph_before_root_publish", id)
        });
        let accepted = installer
            .install(&app, id, true, Some(row.id.clone()))
            .await;
        if let Ok(task) = accepted {
            if let Some(pause) = pause {
                tokio::time::timeout(Duration::from_secs(20), pause.entered.notified())
                    .await
                    .unwrap();
                assert!(
                    app.task_manager().get_task(task).await.is_some(),
                    "required descendants do not imply root success"
                );
                assert_eq!(std::fs::read(&old).unwrap(), b"old");
                assert_eq!(
                    std::fs::read(old.with_file_name("d-1.jar")).unwrap(),
                    b"test"
                );
                std::fs::write(&old, b"later original edit").unwrap();
                if mode == "edited-dependency" {
                    std::fs::write(old.with_file_name("d-1.jar"), b"later dependency edit")
                        .unwrap();
                }
                pause.release.notify_one();
            }
            tokio::time::timeout(
                Duration::from_secs(20),
                wait_install_completion(installer.abort_handle.lock().await.completion.clone()),
            )
            .await
            .unwrap()
            .unwrap();
            tokio::time::sleep(Duration::from_millis(250)).await;
            let state = app.task_manager().get_task(task).await;
            let failed = state.as_ref().is_some_and(|task| {
                matches!(task.progress, crate::domain::vtask::Progress::Failed(_))
            });
            assert_eq!(
                failed,
                matches!(
                    mode,
                    "nested-failure" | "published-rollback" | "edited-dependency"
                ),
                "{mode}: {state:?}"
            );
        } else {
            panic!("{mode}: unexpected admission error: {accepted:?}");
        }
        stop.cancel();
        fixture.await.unwrap();
        if matches!(mode, "published-rollback" | "edited-dependency") {
            assert_eq!(std::fs::read(&old).unwrap(), b"later original edit");
            assert!(!old.with_file_name("b-1.jar").exists());
            assert!(!old.with_file_name("c-1.jar").exists());
            if mode == "edited-dependency" {
                assert_eq!(
                    std::fs::read(old.with_file_name("d-1.jar")).unwrap(),
                    b"later dependency edit"
                );
            } else {
                assert!(!old.with_file_name("d-1.jar").exists());
            }
        } else if mode == "nested-failure" {
            assert_eq!(std::fs::read(&old).unwrap(), b"old");
            assert_eq!(
                std::fs::read_dir(old.parent().unwrap()).unwrap().count(),
                1,
                "no graph artifact may publish after rejected metadata/acquisition"
            );
        } else {
            assert_eq!(std::fs::read(&old).unwrap(), b"test");
            let expected = if matches!(mode, "cycle" | "older-cycle" | "conflict") {
                2
            } else {
                4
            };
            assert_eq!(
                requests.lock().unwrap().len(),
                expected,
                "shared/back-edge dependencies must download once"
            );
            assert_eq!(
                std::fs::read_dir(old.parent().unwrap()).unwrap().count(),
                expected
            );
            if mode == "conflict" {
                // Only one version of b is installed, and the user is told.
                assert!(!requests.lock().unwrap().iter().any(|name| name == "b:2"));
                let warnings = app.task_manager().get_task_warnings().await;
                assert_eq!(warnings.len(), 1, "{warnings:?}");
                assert_eq!(
                    warnings[0].warnings,
                    vec![crate::domain::vtask::TaskWarning::ConflictingVersions {
                        name: "b:1".into(),
                        platform: crate::domain::vtask::DependencyPlatform::Modrinth,
                        project_id: "b".into(),
                        installed: "b:1".into(),
                    }]
                );
            }
            if matches!(mode, "late-pin" | "early-pin") {
                let requests = requests.lock().unwrap();
                assert!(requests.iter().any(|name| name == "p:1"));
                assert!(
                    !requests
                        .iter()
                        .any(|name| name == "p:2" || name == "obsolete:1"),
                    "obsolete provisional closure must never acquire"
                );
            }
        }
    }
    #[tokio::test]
    async fn task10_graph_late_root_failure_rolls_back_published_descendants() {
        run("published-rollback").await;
    }
    #[tokio::test]
    async fn task10_graph_late_root_failure_retains_edited_descendant() {
        run("edited-dependency").await;
    }
    #[tokio::test]
    async fn task10_graph_unpinned_then_exact_replans() {
        run("late-pin").await;
    }
    #[tokio::test]
    async fn task10_graph_exact_then_unpinned_reuses_pin() {
        run("early-pin").await;
    }
    #[tokio::test]
    async fn task10_graph_unpinned_cycle_reuses_older_root() {
        run("older-cycle").await;
    }
    #[tokio::test]
    async fn task10_graph_cycle_completes_all_artifacts() {
        run("cycle").await;
    }
    #[tokio::test]
    async fn task10_graph_diamond_deduplicates_verified_artifacts() {
        run("diamond").await;
    }
    #[tokio::test]
    async fn task10_graph_nested_failure_preserves_old_before_publication() {
        run("nested-failure").await;
    }
    #[tokio::test]
    async fn task10_graph_conflicting_pins_install_one_version_and_say_so() {
        run("conflict").await;
    }

    /// Two installs on one instance at once can share a dependency: the
    /// second finds the one the first published, not yet committed, and
    /// relies on it. When the first then fails late, its rollback keeps that
    /// dependency (2.0.40 never removed one) instead of breaking the second.
    #[tokio::test]
    async fn late_failure_keeps_a_dependency_another_install_relies_on() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, old) = super::super::super::mods::tests::addon_fixture(
            &app,
            "shared-dependency",
            instance::AddonType::Mods,
            "a-1.jar",
            b"old",
        )
        .await;
        let manager = app.instance_manager();
        let operation = manager.lock_instance(id).await;
        app.meta_cache_manager()
            .refresh_published_addon(
                &operation,
                &old.parent().unwrap().to_path_buf(),
                row.filename.clone(),
                true,
                row.addon_type.clone(),
            )
            .await
            .unwrap();
        drop(operation);
        let row = manager.get_owned_mod_file(id, &row.id).await.unwrap();
        let all: Arc<HashMap<String, Vec<String>>> = Arc::new(HashMap::from([
            ("a:1".to_owned(), vec!["d:1".to_owned()]),
            ("x:1".to_owned(), vec!["d:1".to_owned()]),
        ]));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = CancellationToken::new();
        let shutdown = stop.clone();
        let fixture = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! { r = listener.accept() => r, _ = shutdown.cancelled() => break };
                let (mut socket, _) = accepted.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    match socket.read_u8().await {
                        Ok(byte) => request.push(byte),
                        Err(_) => break,
                    }
                }
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest",
                    )
                    .await;
            }
        });
        let node = |name: &str| Node {
            name: name.into(),
            children: all[name].clone(),
            url: url.clone(),
            all: all.clone(),
            exact: true,
        };
        let pause = manager
            .test_hooks
            .arm("addon_graph_before_root_publish", id);
        let first = Installer::new(Box::new(node("a:1")));
        let first_task = first
            .install(&app, id, true, Some(row.id.clone()))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(20), pause.entered.notified())
            .await
            .unwrap();
        let dependency = old.with_file_name("d-1.jar");
        assert_eq!(std::fs::read(&dependency).unwrap(), b"test");

        // The second install runs meanwhile and relies on that dependency.
        let second = Installer::new(Box::new(node("x:1")));
        let second_task = second.install(&app, id, true, None).await.unwrap();
        tokio::time::timeout(
            Duration::from_secs(20),
            wait_install_completion(second.abort_handle.lock().await.completion.clone()),
        )
        .await
        .unwrap()
        .unwrap();
        let outcome = super::settled_outcome(&app, second_task).await;
        assert!(
            matches!(outcome, crate::managers::vtask::TaskOutcome::Succeeded),
            "{:?}",
            app.task_manager().get_task(second_task).await
        );
        assert_eq!(
            std::fs::read(old.with_file_name("x-1.jar")).unwrap(),
            b"test"
        );

        // The first fails late: the file it updates changed meanwhile.
        std::fs::write(&old, b"later original edit").unwrap();
        pause.release.notify_one();
        tokio::time::timeout(
            Duration::from_secs(20),
            wait_install_completion(first.abort_handle.lock().await.completion.clone()),
        )
        .await
        .unwrap()
        .unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(matches!(
            app.task_manager()
                .get_task(first_task)
                .await
                .unwrap()
                .progress,
            crate::domain::vtask::Progress::Failed(_)
        ));
        stop.cancel();
        fixture.await.unwrap();
        assert_eq!(std::fs::read(&old).unwrap(), b"later original edit");
        assert_eq!(
            std::fs::read(&dependency).unwrap(),
            b"test",
            "the dependency the second install relies on stays"
        );
    }
}

#[tokio::test]
async fn task10_owned_world_receipt_retains_edits_and_only_removes_new_copy() {
    for mode in ["owned", "edited", "extra", "equal-later-world"] {
        let app = crate::setup_managers_for_test().await;
        let (id, _, original) = super::super::mods::tests::addon_fixture(
            &app,
            mode,
            instance::AddonType::Worlds,
            "Existing",
            b"original",
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let archive = tempfile::NamedTempFile::new().unwrap();
        zip(
            archive.path(),
            &[("level.dat", b"downloaded"), ("data.bin", b"new")],
        );
        let receipt = Arc::new(Mutex::new(None));
        let manager = app.instance_manager();
        let operation = manager.lock_instance(id).await;
        let reserved = publication::prepare_world(archive.path(), "Existing")
            .await
            .unwrap()
            .reserve_copy(original.parent().unwrap())
            .await
            .unwrap();
        let new = reserved.destinations()[0].clone();
        reserved
            .finish_owned(path.get_root(), &receipt)
            .await
            .unwrap();
        drop(operation);
        if mode == "edited" {
            std::fs::write(new.join("data.bin"), b"player edit").unwrap();
        }
        if mode == "extra" {
            std::fs::write(new.join("later-player-file"), b"keep").unwrap();
        }
        if mode == "equal-later-world" {
            std::fs::rename(&new, original.parent().unwrap().join("moved-copy")).unwrap();
            std::fs::create_dir(&new).unwrap();
            std::fs::write(new.join("level.dat"), b"downloaded").unwrap();
            std::fs::write(new.join("data.bin"), b"new").unwrap();
        }
        let result = publication::rollback_receipt(&app, id, &receipt).await;
        assert_eq!(result.is_ok(), mode == "owned");
        assert_eq!(new.exists(), mode != "owned");
        assert_eq!(
            std::fs::read(original.join("level.dat")).unwrap(),
            b"original"
        );
        if mode == "edited" {
            assert_eq!(std::fs::read(new.join("data.bin")).unwrap(), b"player edit");
        }
        if mode == "extra" {
            assert_eq!(
                std::fs::read(new.join("later-player-file")).unwrap(),
                b"keep"
            );
        }
    }
}

#[tokio::test]
async fn task10_different_filename_rollback_restores_original_cache_identity() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, old) = super::super::mods::tests::addon_fixture(
        &app,
        "receipt-identity",
        instance::AddonType::Mods,
        "old.jar",
        b"old",
    )
    .await;
    let plan = publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::Mods)
        .await
        .unwrap();
    let stage = tempfile::tempdir().unwrap();
    let staged = stage.path().join("artifact");
    std::fs::write(&staged, b"new").unwrap();
    let receipt = Arc::new(Mutex::new(None));
    publication::publish_file_owned(
        &cf("new.jar", b"new", curseforge::ClassId::Mods),
        &app,
        id,
        &staged,
        Some(&plan),
        None,
        &receipt,
        None,
    )
    .await
    .unwrap();
    publication::rollback_receipt(&app, id, &receipt)
        .await
        .unwrap();
    assert_eq!(std::fs::read(&old).unwrap(), b"old");
    assert!(!old.with_file_name("new.jar").exists());
    assert_eq!(
        app.instance_manager()
            .get_owned_mod_file(id, &row.id)
            .await
            .unwrap()
            .filename,
        "old.jar"
    );
}

#[tokio::test]
async fn task10_required_exact_pin_matches_physical_enabled_artifact() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "pin",
        instance::AddonType::Mods,
        "library.jar",
        b"library",
    )
    .await;
    let manager = app.instance_manager();
    let operation = manager.lock_instance(id).await;
    let metadata = app
        .meta_cache_manager()
        .refresh_published_addon(
            &operation,
            &path.parent().unwrap().to_path_buf(),
            row.filename.clone(),
            true,
            row.addon_type.clone(),
        )
        .await
        .unwrap();
    drop(operation);
    carbon_repos::repos::mod_metadata::upsert_mr_mod_cache(
        &app.db,
        publication::file_digest(&path).await.unwrap(),
        "project".into(),
        "P1".into(),
        "Library".into(),
        "1".into(),
        "library".into(),
        "".into(),
        "".into(),
        1,
        "".into(),
        row.filename.clone(),
        "".into(),
        carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().fixed_offset()),
        metadata,
    )
    .await
    .unwrap();
    assert!(
        required_project(&app, id, None, Some("project"), Some("P1"), None)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        required_project(&app, id, None, Some("other"), Some("P1"), None)
            .await
            .unwrap()
            .is_none()
    );
    // Another version with other bytes appeared while the plan downloaded P2.
    let other = Downloadable::new("", path.clone())
        .with_checksum(Some(Checksum::Sha1(hex::encode(
            <sha1::Sha1 as sha1::Digest>::digest(b"another build"),
        ))))
        .with_size(13);
    let error = required_project(&app, id, None, Some("project"), Some("P2"), Some(&other))
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Another version of a required dependency, library.jar"),
        "{error:#}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"library");
    std::fs::write(&path, b"later edit").unwrap();
    assert!(
        required_project(&app, id, None, Some("project"), Some("P1"), None)
            .await
            .is_err()
    );
}

/// Modrinth's hash lookup links a jar to one version, and some libraries
/// attach the same jar to several versions (libjf on 26.1.2 and 26.2). A
/// dependency the plan downloaded at v2 whose row the lookup linked to v1 is
/// the planned artifact, so the graph that published it keeps it.
#[tokio::test]
async fn a_dependency_linked_to_another_version_of_the_same_jar_is_kept() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "shared-jar",
        instance::AddonType::Mods,
        "library.jar",
        b"library",
    )
    .await;
    let manager = app.instance_manager();
    let operation = manager.lock_instance(id).await;
    let metadata = app
        .meta_cache_manager()
        .refresh_published_addon(
            &operation,
            &path.parent().unwrap().to_path_buf(),
            row.filename.clone(),
            true,
            row.addon_type.clone(),
        )
        .await
        .unwrap();
    drop(operation);
    // The hash lookup answered v1; the plan downloaded v2 (`mr` fixture).
    carbon_repos::repos::mod_metadata::upsert_mr_mod_cache(
        &app.db,
        publication::file_digest(&path).await.unwrap(),
        "fixture".into(),
        "v1".into(),
        "Library".into(),
        "1".into(),
        "library".into(),
        "".into(),
        "".into(),
        1,
        "".into(),
        row.filename.clone(),
        "".into(),
        carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().fixed_offset()),
        metadata,
    )
    .await
    .unwrap();
    validate_required_resource(&mr("library.jar", b"library"), &app, id)
        .await
        .unwrap();
    // A v2 with other bytes is still another version of the dependency.
    let error = validate_required_resource(&mr("library.jar", b"library v2"), &app, id)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Another version of a required dependency"),
        "{error:#}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"library");
}

#[tokio::test]
async fn task10_explicit_dependency_setting_false_keeps_replacement_opt_out() {
    let app = crate::setup_managers_for_test().await;
    app.settings_manager()
        .set_settings(
            serde_json::from_value(serde_json::json!({"downloadDependencies":{"Set":false}}))
                .unwrap(),
        )
        .await
        .unwrap();
    let (id, row, path) = super::super::mods::tests::addon_fixture(
        &app,
        "dependency-opt-out",
        instance::AddonType::Mods,
        "parent.jar",
        b"old",
    )
    .await;
    let discovered = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut installer = Installer::new(Box::new(Task10ReplacementResource {
        resource: cf("parent.jar", b"new", curseforge::ClassId::Mods),
        discovered: discovered.clone(),
    }));
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"new").unwrap();
    installer.download_fixture = Some(source.path().into());
    let task = installer
        .install(&app, id, false, Some(row.id))
        .await
        .unwrap();
    wait_install_completion(installer.abort_handle.lock().await.completion.clone())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(!discovered.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(std::fs::read(path).unwrap(), b"new");
    assert!(app.task_manager().get_task(task).await.is_none());
}

struct Task10PlanningResource {
    entered: Arc<tokio::sync::Notify>,
    dropped: Arc<std::sync::atomic::AtomicBool>,
}
#[async_trait::async_trait]
impl ResourceInstaller for Task10PlanningResource {
    fn id(&self) -> String {
        "planning:root:1".into()
    }
    fn display_name(&self) -> String {
        "planning".into()
    }
    fn get_install_path(&self, path: &InstancePath) -> PathBuf {
        path.get_mods_path().join("planning.jar")
    }
    async fn downloadable(&self, _: &InstancePath) -> Option<Downloadable> {
        None
    }
    async fn post_process(&self, _: &InstancePath, _: &std::path::Path) -> anyhow::Result<()> {
        Ok(())
    }
    async fn is_already_installed(&self, _: &Arc<AppInner>, _: InstanceId) -> anyhow::Result<bool> {
        Ok(false)
    }
    async fn rollback(&self, _: &mut InstanceData) -> anyhow::Result<()> {
        Ok(())
    }
    fn dependencies(
        &self,
        _: &Arc<AppInner>,
        _: InstanceId,
        _: &InstanceData,
        _: ModChannel,
    ) -> DependencyIterator {
        let entered = self.entered.clone();
        let dropped = self.dropped.clone();
        let getter: ResourceInstallerGetter = Box::new(move |_| {
            Box::pin(async move {
                struct Guard(Arc<std::sync::atomic::AtomicBool>);
                impl Drop for Guard {
                    fn drop(&mut self) {
                        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                let _guard = Guard(dropped);
                entered.notify_one();
                std::future::pending().await
            })
        });
        DependencyIterator::new(vec![getter].into_iter())
    }
}
/// Dependency planning settles like the rest of an addon install:
/// - "abort": abort drains it, and the install reports the cancellation.
/// - "drop-request", "drop-request-quit": since the install is retained by the
///   runtime (b8e8cf7af), a caller that stops waiting for `install` does not
///   abandon the planning. It keeps running under the retained owner, with a
///   known settlement, until abort or quitting the app drains it.
/// - "drop-admission": dropping the admission future itself drops the planning
///   and leaves its settlement unknown, so abort refuses to certify it.
#[tokio::test]
async fn task10_planning_uses_existing_admission_settlement_and_unknown() {
    use crate::managers::runtime_lifecycle::{ShutdownPhase, ShutdownPurpose};
    use std::sync::atomic::Ordering::SeqCst;
    for mode in [
        "abort",
        "drop-request",
        "drop-request-quit",
        "drop-admission",
    ] {
        let app = crate::setup_managers_for_test().await;
        let (id, _, _) = super::super::mods::tests::addon_fixture(
            &app,
            "planning-admission",
            instance::AddonType::Mods,
            "existing.jar",
            b"existing",
        )
        .await;
        let entered = Arc::new(tokio::sync::Notify::new());
        let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let installer = Arc::new(Installer::new(Box::new(Task10PlanningResource {
            entered: entered.clone(),
            dropped: dropped.clone(),
        })));
        let worker = installer.clone();
        let owned_app = app.clone();
        let admission = tokio::spawn(async move {
            if mode == "drop-admission" {
                worker.install_owned(&owned_app, id, true, None).await
            } else {
                worker.install(&owned_app, id, true, None).await
            }
        });
        tokio::time::timeout(Duration::from_secs(10), entered.notified())
            .await
            .unwrap();
        if mode == "abort" {
            installer.abort(None).await.unwrap();
            assert!(admission.await.unwrap().is_err());
            assert!(dropped.load(SeqCst), "{mode}");
            continue;
        }
        admission.abort();
        assert!(admission.await.unwrap_err().is_cancelled());
        if mode == "drop-admission" {
            assert!(dropped.load(SeqCst), "{mode}");
            let error = installer.abort(None).await.unwrap_err();
            assert!(format!("{error:#}").contains("settlement unknown"));
            continue;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !dropped.load(SeqCst),
            "{mode}: the retained install still owns its planning"
        );
        if mode == "drop-request" {
            installer.abort(None).await.unwrap();
            assert!(dropped.load(SeqCst), "{mode}");
            continue;
        }
        app.runtime_lifecycle
            .prepare_shutdown("planning", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while app
                .runtime_lifecycle
                .shutdown_status("planning", ShutdownPurpose::Quit)
                .unwrap()
                .phase
                == ShutdownPhase::Draining
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(dropped.load(SeqCst), "{mode}: quitting drains the planning");
        assert_eq!(
            app.runtime_lifecycle
                .shutdown_status("planning", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            ShutdownPhase::Ready,
            "{mode}: the retained planning settles, so quitting is not blocked"
        );
    }
}

#[tokio::test]
async fn task10_staged_datapack_update_keeps_existing_world_copy_independent() {
    use super::super::datapacks::CopyDatapack;
    let app = crate::setup_managers_for_test().await;
    let archive = tempfile::NamedTempFile::new().unwrap();
    zip(
        archive.path(),
        &[(
            "pack.mcmeta",
            br#"{"pack":{"pack_format":15,"description":"first"}}"#,
        )],
    );
    let bytes = std::fs::read(archive.path()).unwrap();
    let (id, row, staged) = super::super::mods::tests::addon_fixture(
        &app,
        "staged-update",
        instance::AddonType::DataPacks,
        "pack.zip",
        &bytes,
    )
    .await;
    let path = publication::current_path(&app, id).await.unwrap();
    let world = path.get_saves_path().join("Selected");
    std::fs::create_dir_all(&world).unwrap();
    std::fs::write(world.join("level.dat"), b"world").unwrap();
    let selected = app
        .instance_manager()
        .datapack_worlds(id)
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.instance_manager()
        .copy_datapack(CopyDatapack {
            instance_id: *id,
            mod_id: row.id.clone(),
            world: selected,
        })
        .await
        .unwrap();
    let plan =
        publication::ReplacementPlan::capture(&app, id, &row.id, instance::AddonType::DataPacks)
            .await
            .unwrap();
    let updated = tempfile::NamedTempFile::new().unwrap();
    zip(
        updated.path(),
        &[(
            "pack.mcmeta",
            br#"{"pack":{"pack_format":15,"description":"updated"}}"#,
        )],
    );
    let updated_bytes = std::fs::read(updated.path()).unwrap();
    publication::publish_file(
        &cf("pack.zip", &updated_bytes, curseforge::ClassId::Datapacks),
        &app,
        id,
        updated.path(),
        Some(&plan),
        None,
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(staged).unwrap(), updated_bytes);
    assert_eq!(
        std::fs::read(world.join("datapacks/pack.zip")).unwrap(),
        bytes
    );
}

#[tokio::test]
async fn task10_publication_rechecks_repair_admitted_after_acquisition() {
    use super::super::modpack::session::{Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, row, original) = super::super::mods::tests::addon_fixture(
        &app,
        "late-repair",
        instance::AddonType::Mods,
        "original.jar",
        b"old",
    )
    .await;
    let path = publication::current_path(&app, id).await.unwrap();
    let mut installer = cf("original.jar", b"new", curseforge::ClassId::Mods).into_installer();
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"new").unwrap();
    installer.download_fixture = Some(source.path().into());
    let pause = app
        .instance_manager()
        .test_hooks
        .arm("addon_graph_before_root_publish", id);
    let task = installer
        .install(&app, id, false, Some(row.id))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), pause.entered.notified())
        .await
        .unwrap();
    let manager = app.instance_manager();
    let operation = manager.lock_instance(id).await;
    let mut session = Session::load(&path.get_root()).await.unwrap().unwrap();
    session.producer = Producer::Repair;
    std::fs::write(
        path.get_root().join(".setup/setup-session.json"),
        serde_json::to_vec(&session).unwrap(),
    )
    .unwrap();
    drop(operation);
    pause.release.notify_one();
    wait_install_completion(installer.abort_handle.lock().await.completion.clone())
        .await
        .unwrap();
    assert_eq!(std::fs::read(original).unwrap(), b"old");
    assert!(matches!(
        app.task_manager().get_task(task).await.unwrap().progress,
        crate::domain::vtask::Progress::Failed(_)
    ));
}

#[test]
fn task10_required_catalogue_normal_loader_empty_and_unknown_controls() {
    use domain::instance::info::{ModLoader, ModLoaderType};
    let target = (
        "1.20.1".into(),
        [ModLoader {
            type_: ModLoaderType::Fabric,
            version: "0.15".into(),
        }]
        .into_iter()
        .collect(),
    );
    let mut wrong = mr("wrong.jar", b"wrong").version;
    wrong.loaders = vec!["forge".into()];
    let versions = vec![wrong, mr("correct.jar", b"correct").version];
    fn describe(v: &modrinth::version::Version) -> (&Vec<String>, &Vec<String>, ModChannel) {
        (
            &v.game_versions,
            &v.loaders,
            ModChannel::from(v.version_type),
        )
    }
    let selected = select_dependency(
        &versions,
        Some(&target),
        false,
        ModChannel::Stable,
        describe,
    )
    .unwrap();
    assert_eq!(selected.files[0].filename, "correct.jar");
    assert!(
        select_dependency(
            &versions[..0],
            Some(&target),
            false,
            ModChannel::Stable,
            describe
        )
        .is_err()
    );
    assert!(select_dependency(&versions, None, false, ModChannel::Stable, describe).is_err());
}

#[tokio::test]
async fn task18_addon_runtime_cancel_waits_for_tree_io_and_preserves_normal_completion() {
    use crate::managers::runtime_lifecycle::{
        ActivityPolicy, RuntimeLifecycle, ShutdownPhase, ShutdownPurpose, spawn_retained,
    };
    for cancelled in [false, true] {
        let installer = cf("fixture.jar", b"fixture", curseforge::ClassId::Mods).into_installer();
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let activity = lifecycle
            .admit_owned("addon tree", ActivityPolicy::Installer)
            .unwrap();
        let cancel = activity.cancellation();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settled");
        let worker_path = path.clone();
        let (arrive, arrived) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        installer
            .abort_handle
            .lock()
            .await
            .spawn(move |cancel| async move {
                if cancelled {
                    cancel.cancelled().await;
                }
                arrive.send(()).unwrap();
                released.await.unwrap();
                tokio::fs::write(worker_path, b"tree mutation and rollback settled")
                    .await
                    .unwrap();
            });
        let ownership = installer.ownership.clone();
        let worker = spawn_retained(activity, move |slot| {
            Box::pin(async move {
                ownership.settle_runtime(slot, cancel).await;
            })
        });
        if cancelled {
            lifecycle
                .prepare_shutdown("addon", ShutdownPurpose::Quit, async { Ok(()) })
                .unwrap();
        }
        arrived.await.unwrap();
        assert!(!worker.is_finished());
        assert!(!path.exists());
        if cancelled {
            assert!(
                lifecycle
                    .commit_prepared_shutdown("addon", ShutdownPurpose::Quit)
                    .is_err()
            );
        }
        release.send(()).unwrap();
        worker.await.unwrap();
        assert_eq!(
            std::fs::read(path).unwrap(),
            b"tree mutation and rollback settled"
        );
        if cancelled {
            tokio::time::timeout(Duration::from_secs(1), async {
                while lifecycle
                    .shutdown_status("addon", ShutdownPurpose::Quit)
                    .unwrap()
                    .phase
                    != ShutdownPhase::Ready
                {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        } else {
            lifecycle.begin_relocation("normal").unwrap();
        }
    }
}

#[tokio::test]
async fn task18_addon_lost_tree_ack_retains_runtime_unknown() {
    use crate::managers::runtime_lifecycle::{
        ActivityPolicy, RuntimeLifecycle, ShutdownPhase, ShutdownPurpose, spawn_retained,
    };
    let installer = cf("fixture.jar", b"fixture", curseforge::ClassId::Mods).into_installer();
    let (sender, receiver) = watch::channel(false);
    drop(sender);
    installer.abort_handle.lock().await.completion = Some(receiver);
    let lifecycle = Arc::new(RuntimeLifecycle::default());
    lifecycle.reconciliation_finished(Ok(()));
    let activity = lifecycle
        .admit_owned("unknown addon tree", ActivityPolicy::Installer)
        .unwrap();
    let cancel = activity.cancellation();
    let ownership = installer.ownership.clone();
    spawn_retained(activity, move |slot| {
        Box::pin(async move {
            ownership.settle_runtime(slot, cancel).await;
        })
    })
    .await
    .unwrap();
    lifecycle
        .prepare_shutdown("unknown-addon", ShutdownPurpose::Quit, async { Ok(()) })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        while lifecycle
            .shutdown_status("unknown-addon", ShutdownPurpose::Quit)
            .unwrap()
            .phase
            == ShutdownPhase::Draining
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        lifecycle
            .shutdown_status("unknown-addon", ShutdownPurpose::Quit)
            .unwrap()
            .phase,
        ShutdownPhase::Unresolved
    );
    assert!(
        lifecycle
            .commit_prepared_shutdown("unknown-addon", ShutdownPurpose::Quit)
            .is_err()
    );
}

/// One dependency rule (installed-dependency-revalidated): an enabled copy
/// satisfies, a disabled-only copy refuses a fresh install but counts as
/// present on an update, and an unresolvable missing dependency is skipped
/// with a named warning. Getters run against a recording stand-in for the
/// platform APIs, so "no request" is asserted rather than assumed.
mod dependency_rule {
    use super::*;
    use crate::domain::instance::info::{GameVersion, ModLoader, ModLoaderType, StandardVersion};
    use crate::domain::vtask::{DependencyPlatform, SkippedDependencyReason, TaskWarning};
    use carbon_repos::dbtypes::DbDateTime;
    use carbon_repos::repos::mod_metadata as metarepo;
    use dependency::{Catalogue, LoaderFit};
    use std::collections::HashMap;

    const SODIUM: &str = "AANobbMI";

    #[derive(Default)]
    struct Fixture {
        cf_mods: HashMap<i32, curseforge::Mod>,
        cf_files: HashMap<i32, Vec<curseforge::File>>,
        mr_versions: HashMap<String, modrinth::version::Version>,
        mr_projects: HashMap<String, modrinth::project::Project>,
        /// Answer every lookup with this HTTP status instead.
        status: Option<u16>,
        /// Only the lookups whose call starts with this prefix get `status`.
        down: Option<&'static str>,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl Fixture {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
        fn answer<T: Clone>(&self, call: String, value: Option<&T>) -> anyhow::Result<T> {
            use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};
            let failing = self
                .status
                .filter(|_| self.down.is_none_or(|prefix| call.starts_with(prefix)));
            self.calls.lock().unwrap().push(call);
            let status = match (failing, value) {
                (Some(status), _) => status,
                (None, Some(value)) => return Ok(value.clone()),
                (None, None) => 404,
            };
            Err(RequestError {
                context: RequestContext::none(),
                error: RequestErrorDetails::UnexpectedStatus {
                    status: reqwest::StatusCode::from_u16(status).unwrap(),
                    details: None,
                },
            }
            .into())
        }
    }

    #[async_trait::async_trait]
    impl Catalogue for Fixture {
        async fn cf_mod(&self, project: i32) -> anyhow::Result<curseforge::Mod> {
            self.answer(format!("cf_mod:{project}"), self.cf_mods.get(&project))
        }
        async fn cf_file(&self, project: i32, file: i32) -> anyhow::Result<curseforge::File> {
            let found = self
                .cf_files
                .get(&project)
                .and_then(|files| files.iter().find(|f| f.id == file));
            self.answer(format!("cf_file:{project}:{file}"), found)
        }
        async fn cf_files(
            &self,
            project: i32,
            release: &str,
            _: i32,
        ) -> anyhow::Result<carbon_platforms::curseforge::CurseForgeResponse<Vec<curseforge::File>>>
        {
            let files: Vec<_> = self
                .cf_files
                .get(&project)
                .into_iter()
                .flatten()
                .filter(|file| file.game_versions.iter().any(|v| v == release))
                .cloned()
                .collect();
            self.answer(format!("cf_files:{project}"), Some(&files))
                .map(|data| carbon_platforms::curseforge::CurseForgeResponse {
                    data,
                    pagination: None,
                })
        }
        async fn mr_version(&self, version: &str) -> anyhow::Result<modrinth::version::Version> {
            self.answer(
                format!("mr_version:{version}"),
                self.mr_versions.get(version),
            )
        }
        async fn mr_project(&self, project: &str) -> anyhow::Result<modrinth::project::Project> {
            self.answer(
                format!("mr_project:{project}"),
                self.mr_projects.get(project),
            )
        }
        async fn mr_versions(
            &self,
            filters: ProjectVersionsFilters,
        ) -> anyhow::Result<Vec<modrinth::version::Version>> {
            let loaders = filters.loaders.clone();
            let mut versions: Vec<_> = self
                .mr_versions
                .values()
                .filter(|v| v.project_id == *filters.project_id)
                .filter(|v| {
                    loaders
                        .as_ref()
                        .is_none_or(|loaders| v.loaders.iter().any(|l| loaders.contains(l)))
                })
                .cloned()
                .collect();
            // Newest first, as Modrinth answers.
            versions.sort_by(|a, b| {
                b.date_published
                    .cmp(&a.date_published)
                    .then_with(|| a.id.cmp(&b.id))
            });
            self.answer(
                format!("mr_versions:{}:{:?}", *filters.project_id, loaders),
                Some(&versions),
            )
        }
    }

    fn mr_version(
        project: &str,
        id: &str,
        filename: &str,
        bytes: &[u8],
        games: &[&str],
        loaders: &[&str],
        dependencies: serde_json::Value,
    ) -> modrinth::version::Version {
        use sha1::Digest;
        serde_json::from_value(serde_json::json!({
            "name": id, "version_number": id, "dependencies": dependencies, "game_versions": games,
            "version_type": "release", "loaders": loaders, "featured": false, "id": id, "project_id": project,
            "author_id": "fixture", "date_published": "2026-01-01T00:00:00Z", "downloads": 0,
            "files": [{"hashes": {"sha1": hex::encode(sha1::Sha1::digest(bytes)), "sha512": hex::encode(sha2::Sha512::digest(bytes))},
                "url": "https://fixture.invalid/never-requested", "filename": filename, "primary": true, "size": bytes.len()}]
        }))
        .unwrap()
    }

    fn mr_project(id: &str, title: &str, project_type: &str) -> modrinth::project::Project {
        serde_json::from_value(serde_json::json!({
            "slug": id, "title": title, "description": "", "categories": [], "client_side": "required", "server_side": "optional",
            "body": "", "additional_categories": [], "donation_urls": [], "project_type": project_type, "downloads": 0, "id": id, "team": "fixture",
            "published": "2026-01-01T00:00:00Z", "updated": "2026-01-01T00:00:00Z", "followers": 0, "status": "approved",
            "license": {"id": "MIT", "name": "MIT", "url": null}, "versions": [], "game_versions": [], "loaders": []
        }))
        .unwrap()
    }

    fn cf_file(
        project: i32,
        id: i32,
        filename: &str,
        games: &[&str],
        downloadable: bool,
    ) -> curseforge::File {
        serde_json::from_value(serde_json::json!({
            "id": id, "gameId": 432, "modId": project, "isAvailable": true,
            "displayName": filename, "fileName": filename, "releaseType": 1,
            "fileStatus": 4, "hashes": [], "fileDate": "2026-01-01T00:00:00Z", "fileLength": 4, "downloadCount": 0,
            "downloadUrl": if downloadable { serde_json::json!("https://fixture.invalid/never-requested") } else { serde_json::Value::Null },
            "gameVersions": games, "sortableGameVersions": [], "dependencies": [], "fileFingerprint": 0
        }))
        .unwrap()
    }

    fn cf_mod(id: i32, name: &str) -> curseforge::Mod {
        serde_json::from_value(serde_json::json!({
            "id": id, "gameId": 432, "name": name, "slug": name, "summary": "", "status": 4,
            "links": {"websiteUrl": null, "wikiUrl": null, "issuesUrl": null, "sourceUrl": null},
            "downloadCount": 0, "isFeatured": false, "primaryCategoryId": 0, "categories": [], "classId": 6,
            "authors": [], "logo": null, "screenshots": [], "mainFileId": 0, "latestFiles": [], "latestFilesIndexes": [],
            "dateCreated": "2026-01-01T00:00:00Z", "dateModified": "2026-01-01T00:00:00Z", "dateReleased": "2026-01-01T00:00:00Z",
            "allowModDistribution": true, "gamePopularityRank": 0, "isAvailable": true, "thumbsUpCount": 0
        }))
        .unwrap()
    }

    fn mr_parent(
        project_type: &str,
        filename: &str,
        bytes: &[u8],
        dependencies: serde_json::Value,
        catalogue: &Arc<Fixture>,
    ) -> ModrinthModInstaller {
        let mut resource = ModrinthModInstaller::from_version(
            mr_version(
                "parent",
                "parent-2",
                filename,
                bytes,
                &["1.20.1"],
                &["fabric"],
                dependencies,
            ),
            mr_project("parent", "Parent", project_type),
        )
        .unwrap();
        resource.catalogue = Some(catalogue.clone());
        resource
    }

    fn cf_parent(
        filename: &str,
        bytes: &[u8],
        requires: i32,
        catalogue: &Arc<Fixture>,
    ) -> CurseforgeModInstaller {
        let mut resource = cf(filename, bytes, curseforge::ClassId::Mods);
        resource.file.dependencies =
            serde_json::from_value(serde_json::json!([{"modId": requires, "relationType": 3}]))
                .unwrap();
        resource.catalogue = Some(catalogue.clone());
        resource
    }

    async fn instance(
        app: &Arc<AppInner>,
        name: &str,
        release: &str,
        loader: Option<ModLoaderType>,
    ) -> InstanceId {
        let group = app.instance_manager().get_default_group().await.unwrap();
        app.instance_manager()
            .create_instance(
                group,
                name.to_owned(),
                false,
                super::super::super::InstanceVersionSource::Version(GameVersion::Standard(
                    StandardVersion {
                        release: release.to_owned(),
                        modloaders: loader
                            .map(|type_| ModLoader {
                                type_,
                                version: "0".into(),
                            })
                            .into_iter()
                            .collect(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap()
    }

    /// A copy in the mods folder whose metadata is linked to platform projects.
    async fn installed(
        app: &Arc<AppInner>,
        id: InstanceId,
        filename: &str,
        enabled: bool,
        cf: Option<(i32, i32, &str)>,
        mr: Option<(&str, &str, &str)>,
    ) -> mfcdb::ModFileCacheRow {
        installed_copy(app, id, filename, enabled, cf, mr, None).await
    }

    /// A jar the scan could not link to any platform, with its mod id.
    async fn unlinked(
        app: &Arc<AppInner>,
        id: InstanceId,
        filename: &str,
        modid: Option<&str>,
    ) -> mfcdb::ModFileCacheRow {
        installed_copy(app, id, filename, true, None, None, modid).await
    }

    async fn installed_copy(
        app: &Arc<AppInner>,
        id: InstanceId,
        filename: &str,
        enabled: bool,
        cf: Option<(i32, i32, &str)>,
        mr: Option<(&str, &str, &str)>,
        modid: Option<&str>,
    ) -> mfcdb::ModFileCacheRow {
        let path = publication::current_path(app, id).await.unwrap();
        let folder = path.get_mods_path();
        std::fs::create_dir_all(&folder).unwrap();
        let physical = if enabled {
            filename.to_owned()
        } else {
            format!("{filename}.disabled")
        };
        std::fs::write(folder.join(physical), filename.as_bytes()).unwrap();
        let metadata = format!("{}-{filename}", id.0);
        let now = || DbDateTime(chrono::Utc::now().fixed_offset());
        metarepo::insert_metadata(
            &app.db,
            &metadata,
            0,
            metadata.as_bytes(),
            metadata.as_bytes(),
            "fabric",
            None,
            modid,
            None,
            None,
            None,
            now(),
        )
        .await
        .unwrap();
        mfcdb::upsert_mod_file_cache(
            &app.db,
            *id,
            filename.to_owned(),
            filename.len() as i32,
            enabled,
            "mods".into(),
            metadata.clone(),
            now(),
        )
        .await
        .unwrap();
        if let Some((project, file, name)) = cf {
            metarepo::upsert_cf_mod_cache(
                &app.db,
                0,
                project,
                file,
                name.into(),
                "1".into(),
                name.into(),
                "".into(),
                "".into(),
                1,
                "".into(),
                now(),
                metadata.clone(),
            )
            .await
            .unwrap();
        }
        if let Some((project, version, title)) = mr {
            metarepo::upsert_mr_mod_cache(
                &app.db,
                hex::encode(metadata.as_bytes()),
                project.into(),
                version.into(),
                title.into(),
                "1".into(),
                title.into(),
                "".into(),
                "".into(),
                1,
                "".into(),
                filename.into(),
                "".into(),
                now(),
                metadata.clone(),
            )
            .await
            .unwrap();
        }
        mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, filename)
            .await
            .unwrap()
            .unwrap()
    }

    async fn first_getter(
        app: &Arc<AppInner>,
        id: InstanceId,
        resource: &dyn ResourceInstaller,
    ) -> Option<anyhow::Result<BoxedResourceInstaller>> {
        let getter = {
            let manager = app.instance_manager();
            let instances = manager.instances.read().await;
            resource
                .dependencies(app, id, instances[&id].data().unwrap(), ModChannel::Stable)
                .next()
                .unwrap()
        };
        getter(HashMap::new()).await
    }

    async fn plan(
        app: &Arc<AppInner>,
        id: InstanceId,
        root: impl ResourceInstaller + Send + 'static,
        replacing: bool,
    ) -> anyhow::Result<planning::Resolution> {
        let root: BoxedResourceInstaller = Box::new(root);
        planning::resolve(
            Arc::new(Mutex::new(root)),
            app,
            id,
            replacing,
            &CancellationToken::new(),
        )
        .await
    }

    async fn planned_ids(resolution: &planning::Resolution) -> Vec<(String, Vec<String>)> {
        let mut ids = Vec::new();
        for (parent, children) in &resolution.plan {
            let mut child_ids = Vec::new();
            for child in children {
                child_ids.push(child.lock().await.id());
            }
            ids.push((parent.clone(), child_ids));
        }
        ids
    }

    fn modrinth_requires(project: Option<&str>, version: Option<&str>) -> serde_json::Value {
        serde_json::json!([{"project_id": project, "version_id": version, "file_name": null, "dependency_type": "required"}])
    }

    // --- Getter: what the instance already has ------------------------------

    #[tokio::test]
    async fn enabled_curseforge_dependency_is_satisfied_without_any_lookup() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "cf-enabled", "1.12.2", Some(ModLoaderType::Forge)).await;
        installed(
            &app,
            id,
            "Baubles-1.12-1.5.2.jar",
            true,
            Some((227083, 2518667, "Baubles")),
            None,
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 227083, &catalogue);
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());
    }

    #[tokio::test]
    async fn modrinth_pin_of_another_version_keeps_the_installed_copy() {
        // Iris 1.7.6 pins Sodium 0.5.12-beta.2 (ryOMVRuG) while 0.5.13 is installed.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "mr-pin", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed(
            &app,
            id,
            "sodium-0.5.13.jar",
            true,
            None,
            Some((SODIUM, "OihdIimA", "Sodium")),
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("ryOMVRuG")),
            &catalogue,
        );
        assert!(first_getter(&app, id, &iris).await.is_none());
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());
    }

    #[tokio::test]
    async fn enabled_copy_next_to_an_old_disabled_copy_is_satisfied() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(
            &app,
            "enabled-and-old",
            "1.20.1",
            Some(ModLoaderType::Fabric),
        )
        .await;
        installed(
            &app,
            id,
            "fabric-api-old.jar",
            false,
            Some((306612, 1, "Fabric API")),
            None,
        )
        .await;
        installed(
            &app,
            id,
            "fabric-api-new.jar",
            true,
            Some((306612, 2, "Fabric API")),
            None,
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 306612, &catalogue);
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert!(catalogue.calls().is_empty());
    }

    #[tokio::test]
    async fn two_enabled_copies_are_satisfied() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "two-enabled", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed(
            &app,
            id,
            "sodium-a.jar",
            true,
            None,
            Some((SODIUM, "A", "Sodium")),
        )
        .await;
        installed(
            &app,
            id,
            "sodium-b.jar",
            true,
            None,
            Some((SODIUM, "B", "Sodium")),
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let parent = mr_parent(
            "mod",
            "parent.jar",
            b"parent",
            modrinth_requires(Some(SODIUM), None),
            &catalogue,
        );
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert!(catalogue.calls().is_empty());
    }

    #[tokio::test]
    async fn enabled_copy_with_stale_hash_is_satisfied() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "stale-hash", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed(
            &app,
            id,
            "library.jar",
            true,
            Some((42, 7, "Library")),
            None,
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        std::fs::write(
            path.get_mods_path().join("library.jar"),
            b"edited after the scan",
        )
        .unwrap();
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 42, &catalogue);
        assert!(first_getter(&app, id, &parent).await.is_none());
        // The probe never rewrites the user's file.
        assert_eq!(
            std::fs::read(path.get_mods_path().join("library.jar")).unwrap(),
            b"edited after the scan"
        );
    }

    #[tokio::test]
    async fn copy_that_cannot_be_inspected_uses_its_recorded_state() {
        // Both library.jar and library.jar.disabled exist: selected_file refuses
        // to pick one, but the scanner recorded an enabled copy.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "ambiguous", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed(
            &app,
            id,
            "library.jar",
            true,
            Some((42, 7, "Library")),
            None,
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        std::fs::write(path.get_mods_path().join("library.jar.disabled"), b"old").unwrap();
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 42, &catalogue);
        assert!(first_getter(&app, id, &parent).await.is_none());
    }

    #[tokio::test]
    async fn quilted_fabric_api_satisfies_fabric_api_on_quilt() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "qfapi", "1.20.1", Some(ModLoaderType::Quilt)).await;
        installed(
            &app,
            id,
            "qfapi.jar",
            true,
            None,
            Some(("qvIfYCYJ", "Q1", "QFAPI")),
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let parent = mr_parent(
            "mod",
            "parent.jar",
            b"parent",
            modrinth_requires(Some("P7dR8mSH"), None),
            &catalogue,
        );
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert!(catalogue.calls().is_empty());
    }

    #[tokio::test]
    async fn disabled_only_copy_refuses_a_fresh_install_and_names_it() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "disabled-only", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed(
            &app,
            id,
            "library.jar",
            false,
            Some((42, 7, "Library")),
            None,
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 42, &catalogue);
        let error = first_getter(&app, id, &parent)
            .await
            .unwrap()
            .err()
            .unwrap();
        let message = error.to_string();
        assert!(message.contains("disabled"), "{message}");
        assert!(
            message.contains("Library") && message.contains("library.jar"),
            "{message}"
        );
        assert!(message.contains("Enable it in Addons"), "{message}");
        let error = plan(
            &app,
            id,
            cf_parent("parent.jar", b"parent", 42, &catalogue),
            false,
        )
        .await
        .err()
        .unwrap();
        assert!(
            error.to_string().contains("installed but disabled"),
            "{error:#}"
        );
        assert!(catalogue.calls().is_empty());
    }

    #[tokio::test]
    async fn disabled_only_copy_counts_as_present_on_an_update() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(
            &app,
            "disabled-update",
            "1.20.1",
            Some(ModLoaderType::Fabric),
        )
        .await;
        installed(
            &app,
            id,
            "library.jar",
            false,
            Some((42, 7, "Library")),
            None,
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 42, &catalogue);
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, true).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert!(resolution.skipped.is_empty());
        assert!(catalogue.calls().is_empty());
    }

    // --- Selection of what is downloaded --------------------------------------

    fn target(release: &str, loader: Option<ModLoaderType>) -> DependencyTarget {
        (
            release.to_owned(),
            loader
                .map(|type_| ModLoader {
                    type_,
                    version: "0".into(),
                })
                .into_iter()
                .collect(),
        )
    }

    fn select_cf<'a>(
        files: &'a [curseforge::File],
        target: &DependencyTarget,
    ) -> anyhow::Result<&'a curseforge::File> {
        select_dependency(files, Some(target), false, ModChannel::Stable, |f| {
            (
                &f.game_versions,
                &f.game_versions,
                ModChannel::from(f.release_type),
            )
        })
    }

    fn select_mr<'a>(
        versions: &'a [modrinth::version::Version],
        target: &DependencyTarget,
    ) -> anyhow::Result<&'a modrinth::version::Version> {
        select_dependency(versions, Some(target), false, ModChannel::Stable, |v| {
            (
                &v.game_versions,
                &v.loaders,
                ModChannel::from(v.version_type),
            )
        })
    }

    #[test]
    fn untagged_legacy_curseforge_file_is_accepted_on_forge_1_12_2() {
        let files = [cf_file(
            227083,
            2518667,
            "Baubles-1.12-1.5.2.jar",
            &["1.12.2"],
            true,
        )];
        let selected = select_cf(&files, &target("1.12.2", Some(ModLoaderType::Forge))).unwrap();
        assert_eq!(selected.id, 2518667);
    }

    #[test]
    fn fabric_only_modrinth_version_is_accepted_on_quilt() {
        let versions = [mr_version(
            "P7dR8mSH",
            "F1",
            "fabric-api.jar",
            b"f",
            &["1.20.1"],
            &["fabric"],
            serde_json::json!([]),
        )];
        assert!(select_mr(&versions, &target("1.20.1", Some(ModLoaderType::Quilt))).is_ok());
    }

    #[test]
    fn forge_tagged_file_is_accepted_on_neoforge_1_20_1_only() {
        let files = [cf_file(1, 1, "lib-1.20.1.jar", &["1.20.1", "Forge"], true)];
        assert!(select_cf(&files, &target("1.20.1", Some(ModLoaderType::Neoforge))).is_ok());
        let files = [cf_file(1, 2, "lib-1.21.1.jar", &["1.21.1", "Forge"], true)];
        assert!(select_cf(&files, &target("1.21.1", Some(ModLoaderType::Neoforge))).is_err());
    }

    #[test]
    fn forge_only_file_for_another_release_is_still_rejected_on_fabric() {
        // GPT-0071: never fall back to an incompatible artifact.
        let files = [
            cf_file(1, 1, "lib-forge-1.21.jar", &["1.21", "Forge"], true),
            cf_file(1, 2, "lib-forge-1.20.1.jar", &["1.20.1", "Forge"], true),
        ];
        assert!(select_cf(&files, &target("1.20.1", Some(ModLoaderType::Fabric))).is_err());
        let versions = [mr_version(
            "p",
            "v",
            "lib.jar",
            b"l",
            &["1.21"],
            &["forge"],
            serde_json::json!([]),
        )];
        assert!(select_mr(&versions, &target("1.20.1", Some(ModLoaderType::Fabric))).is_err());
    }

    #[test]
    fn untagged_file_is_only_a_fallback_and_never_loads_on_vanilla() {
        let files = [
            cf_file(1, 1, "lib-untagged.jar", &["1.20.1"], true),
            cf_file(1, 2, "lib-fabric.jar", &["1.20.1", "Fabric"], true),
        ];
        let fabric = target("1.20.1", Some(ModLoaderType::Fabric));
        assert_eq!(select_cf(&files, &fabric).unwrap().id, 2);
        assert!(select_cf(&files[..1], &target("1.20.1", None)).is_err());
    }

    #[test]
    fn loader_fit_table() {
        let tags = |tags: &[&str]| tags.iter().map(|t| t.to_string()).collect::<Vec<_>>();
        let quilt = dependency::accepted_loader_tags("1.20.1", [ModLoaderType::Quilt]);
        assert_eq!(
            dependency::loader_fit(&tags(&["1.20.1", "Fabric"]), &quilt),
            LoaderFit::Tagged
        );
        assert_eq!(
            dependency::loader_fit(&tags(&["1.20.1", "Forge"]), &quilt),
            LoaderFit::Foreign
        );
        assert_eq!(
            dependency::loader_fit(&tags(&["1.12.2"]), &quilt),
            LoaderFit::Untagged
        );
        assert_eq!(
            dependency::loader_fit(&tags(&["paper"]), &quilt),
            LoaderFit::Foreign
        );
        let neo = dependency::accepted_loader_tags("1.20.1", [ModLoaderType::Neoforge]);
        assert_eq!(
            dependency::loader_fit(&tags(&["Forge"]), &neo),
            LoaderFit::Tagged
        );
        let neo = dependency::accepted_loader_tags("1.21.1", [ModLoaderType::Neoforge]);
        assert_eq!(
            dependency::loader_fit(&tags(&["Forge"]), &neo),
            LoaderFit::Foreign
        );
        assert_eq!(
            dependency::loader_fit(&tags(&["1.20.1"]), &[]),
            LoaderFit::Foreign
        );
    }

    // --- Planning --------------------------------------------------------------

    #[tokio::test]
    async fn installed_dependency_is_neither_fetched_nor_walked() {
        // Sodium's own metadata would pull in another library if it were walked.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "no-walk", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed(
            &app,
            id,
            "sodium.jar",
            true,
            None,
            Some((SODIUM, "OihdIimA", "Sodium")),
        )
        .await;
        let mut fixture = Fixture::default();
        fixture.mr_versions.insert(
            "OihdIimA".into(),
            mr_version(
                SODIUM,
                "OihdIimA",
                "sodium.jar",
                b"s",
                &["1.20.1"],
                &["fabric"],
                modrinth_requires(Some("transitive"), None),
            ),
        );
        fixture
            .mr_projects
            .insert(SODIUM.into(), mr_project(SODIUM, "Sodium", "mod"));
        let catalogue = Arc::new(fixture);
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("ryOMVRuG")),
            &catalogue,
        );
        let root_id = iris.id();
        let resolution = plan(&app, id, iris, false).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert!(resolution.skipped.is_empty());
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());
    }

    #[tokio::test]
    async fn missing_pinned_dependency_downloads_the_exact_pin() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "pin-download", "1.20.1", Some(ModLoaderType::Fabric)).await;
        let mut fixture = Fixture::default();
        for (version, filename) in [
            ("ryOMVRuG", "sodium-0.5.12-beta.2.jar"),
            ("OihdIimA", "sodium-0.5.13.jar"),
        ] {
            fixture.mr_versions.insert(
                version.into(),
                mr_version(
                    SODIUM,
                    version,
                    filename,
                    b"s",
                    &["1.20.1"],
                    &["fabric"],
                    serde_json::json!([]),
                ),
            );
        }
        fixture
            .mr_projects
            .insert(SODIUM.into(), mr_project(SODIUM, "Sodium", "mod"));
        let catalogue = Arc::new(fixture);
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("ryOMVRuG")),
            &catalogue,
        );
        let root_id = iris.id();
        let resolution = plan(&app, id, iris, false).await.unwrap();
        let planned = planned_ids(&resolution).await;
        let root = planned
            .iter()
            .find(|(parent, _)| *parent == root_id)
            .unwrap();
        assert_eq!(root.1, vec![format!("modrinth:{SODIUM}:ryOMVRuG")]);
        assert!(resolution.skipped.is_empty());
        assert!(
            !catalogue
                .calls()
                .iter()
                .any(|call| call.starts_with("mr_versions")),
            "a pin is downloaded as is: {:?}",
            catalogue.calls()
        );
    }

    #[tokio::test]
    async fn missing_untagged_library_resolves_on_forge_1_12_2() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "legacy-forge", "1.12.2", Some(ModLoaderType::Forge)).await;
        let mut fixture = Fixture::default();
        fixture.cf_mods.insert(227083, cf_mod(227083, "Baubles"));
        fixture.cf_files.insert(
            227083,
            vec![cf_file(
                227083,
                2518667,
                "Baubles-1.12-1.5.2.jar",
                &["1.12.2"],
                true,
            )],
        );
        let catalogue = Arc::new(fixture);
        let parent = cf_parent("parent.jar", b"parent", 227083, &catalogue);
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, false).await.unwrap();
        let planned = planned_ids(&resolution).await;
        let root = planned
            .iter()
            .find(|(parent, _)| *parent == root_id)
            .unwrap();
        assert_eq!(root.1, vec!["curseforge:227083:2518667".to_owned()]);
    }

    #[tokio::test]
    async fn unresolvable_dependency_is_skipped_with_its_name() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(
            &app,
            "not-downloadable",
            "1.20.1",
            Some(ModLoaderType::Forge),
        )
        .await;
        let mut fixture = Fixture::default();
        fixture.cf_mods.insert(9, cf_mod(9, "Private Library"));
        fixture.cf_files.insert(
            9,
            vec![cf_file(9, 90, "private.jar", &["1.20.1", "Forge"], false)],
        );
        let catalogue = Arc::new(fixture);
        let parent = cf_parent("parent.jar", b"parent", 9, &catalogue);
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, false).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(
            resolution.skipped[0].name.as_deref(),
            Some("Private Library")
        );
        assert_eq!(resolution.skipped[0].project_id, "9");
        assert_eq!(
            resolution.skipped[0].reason,
            SkippedDependencyReason::NotDownloadable
        );
    }

    fn sodium_fixture(versions: &[(&str, &[&str])]) -> Fixture {
        let mut fixture = Fixture::default();
        let mut project = mr_project(SODIUM, "Sodium", "mod");
        project.slug = "sodium".into();
        fixture.mr_projects.insert(SODIUM.into(), project);
        for (version, games) in versions {
            fixture.mr_versions.insert(
                (*version).into(),
                mr_version(
                    SODIUM,
                    version,
                    &format!("sodium-{version}.jar"),
                    version.as_bytes(),
                    games,
                    &["fabric"],
                    serde_json::json!([]),
                ),
            );
        }
        fixture
    }

    async fn root_children(resolution: &planning::Resolution, root_id: &str) -> Vec<String> {
        planned_ids(resolution)
            .await
            .into_iter()
            .find(|(parent, _)| parent == root_id)
            .map(|(_, children)| children)
            .unwrap()
    }

    async fn child_is_exact(resolution: &planning::Resolution, root_id: &str) -> bool {
        resolution.plan[root_id][0].lock().await.exact_dependency()
    }

    #[tokio::test]
    async fn pin_for_another_release_falls_back_to_a_compatible_version() {
        // Indium 1.0.36+mc1.20.1 is listed for 1.20.6 but pins Sodium
        // mc1.20.1-0.5.11 (ygf8cVZg, 1.20.1 only); mc1.20.6-0.5.11 exists.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "pin-release", "1.20.6", Some(ModLoaderType::Fabric)).await;
        let catalogue = Arc::new(sodium_fixture(&[
            ("ygf8cVZg", &["1.20.1"]),
            ("OwLQelEI", &["1.20.6"]),
        ]));
        let indium = mr_parent(
            "mod",
            "indium.jar",
            b"indium",
            modrinth_requires(Some(SODIUM), Some("ygf8cVZg")),
            &catalogue,
        );
        let root_id = indium.id();
        let resolution = plan(&app, id, indium, false).await.unwrap();
        assert_eq!(
            root_children(&resolution, &root_id).await,
            vec![format!("modrinth:{SODIUM}:OwLQelEI")]
        );
        assert!(resolution.skipped.is_empty());
        // The replacement is not the declared pin, so it binds no other dependant.
        assert!(!child_is_exact(&resolution, &root_id).await);
        // Only if nothing fits is the dependency skipped, and then truly has no
        // version for this game and loader.
        let catalogue = Arc::new(sodium_fixture(&[("ygf8cVZg", &["1.20.1"])]));
        let indium = mr_parent(
            "mod",
            "indium.jar",
            b"indium",
            modrinth_requires(Some(SODIUM), Some("ygf8cVZg")),
            &catalogue,
        );
        let resolution = plan(&app, id, indium, false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(resolution.skipped[0].name.as_deref(), Some("Sodium"));
        assert_eq!(
            resolution.skipped[0].reason,
            SkippedDependencyReason::NoCompatibleVersion
        );
    }

    #[tokio::test]
    async fn removed_pin_falls_back_and_skips_are_named_by_title() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "lookup", "1.20.1", Some(ModLoaderType::Fabric)).await;
        // 404 on the pin: the newest compatible version is used instead.
        let catalogue = Arc::new(sodium_fixture(&[("OihdIimA", &["1.20.1"])]));
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("gone")),
            &catalogue,
        );
        let root_id = iris.id();
        let resolution = plan(&app, id, iris, false).await.unwrap();
        assert_eq!(
            root_children(&resolution, &root_id).await,
            vec![format!("modrinth:{SODIUM}:OihdIimA")]
        );
        assert!(resolution.skipped.is_empty());
        // Pin gone and nothing compatible: skipped under its title.
        let catalogue = Arc::new(sodium_fixture(&[]));
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("gone")),
            &catalogue,
        );
        let resolution = plan(&app, id, iris, false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(resolution.skipped[0].name.as_deref(), Some("Sodium"));
        assert_eq!(resolution.skipped[0].project_id, SODIUM);
        // The project itself is gone: no name to give, the UI names the platform.
        let catalogue = Arc::new(Fixture::default());
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("gone")),
            &catalogue,
        );
        let resolution = plan(&app, id, iris, false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(resolution.skipped[0].name, None);
        assert_eq!(resolution.skipped[0].platform, DependencyPlatform::Modrinth);
        assert_eq!(resolution.skipped[0].project_id, SODIUM);
        assert_eq!(
            resolution.skipped[0].reason,
            SkippedDependencyReason::Unavailable
        );
    }

    #[tokio::test]
    async fn lookup_outage_stays_fatal_and_names_the_dependency() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "outage", "1.20.1", Some(ModLoaderType::Fabric)).await;
        // 503 on the pin: never install the addon without its dependency over
        // a transient error, and say which dependency could not be looked up.
        let mut fixture = sodium_fixture(&[("OihdIimA", &["1.20.1"])]);
        fixture.status = Some(503);
        fixture.down = Some("mr_version:");
        let catalogue = Arc::new(fixture);
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("ryOMVRuG")),
            &catalogue,
        );
        let error = plan(&app, id, iris, false).await.err().unwrap();
        assert!(
            format!("{error:#}").contains(&format!(
                "Could not look up required dependency Sodium ({SODIUM})"
            )),
            "{error:#}"
        );
        // Everything down: only the id is known.
        let catalogue = Arc::new(Fixture {
            status: Some(503),
            ..Default::default()
        });
        let parent = cf_parent("parent.jar", b"parent", 12345, &catalogue);
        let error = plan(&app, id, parent, false).await.err().unwrap();
        assert!(
            format!("{error:#}")
                .contains("Could not look up required dependency CurseForge project 12345"),
            "{error:#}"
        );
    }

    #[tokio::test]
    async fn modrinth_query_asks_for_the_loaders_the_instance_runs() {
        for (loader, release, asked) in [
            (
                ModLoaderType::Quilt,
                "1.20.1",
                r#"Some(["fabric", "quilt"])"#,
            ),
            (
                ModLoaderType::Neoforge,
                "1.20.1",
                r#"Some(["forge", "neoforge"])"#,
            ),
            (ModLoaderType::Neoforge, "1.21.1", r#"Some(["neoforge"])"#),
        ] {
            let app = crate::setup_managers_for_test().await;
            let id = instance(&app, "loaders", release, Some(loader)).await;
            let mut fixture = Fixture::default();
            fixture.mr_projects.insert(
                "P7dR8mSH".into(),
                mr_project("P7dR8mSH", "Fabric API", "mod"),
            );
            fixture.mr_versions.insert(
                "F1".into(),
                mr_version(
                    "P7dR8mSH",
                    "F1",
                    "fabric-api.jar",
                    b"f",
                    &[release],
                    &["fabric", "forge"],
                    serde_json::json!([]),
                ),
            );
            let catalogue = Arc::new(fixture);
            let parent = mr_parent(
                "mod",
                "parent.jar",
                b"parent",
                modrinth_requires(Some("P7dR8mSH"), None),
                &catalogue,
            );
            plan(&app, id, parent, false).await.unwrap();
            let expected = format!("mr_versions:P7dR8mSH:{asked}");
            assert!(
                catalogue.calls().contains(&expected),
                "{loader:?} {release}: {:?}",
                catalogue.calls()
            );
        }
    }

    #[tokio::test]
    async fn file_name_only_dependency_is_satisfied_by_that_file() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "file-only", "1.20.1", Some(ModLoaderType::Fabric)).await;
        let catalogue = Arc::new(Fixture::default());
        let requires = serde_json::json!([{"project_id": null, "version_id": null, "file_name": "library-1.0.jar", "dependency_type": "required"}]);
        let parent = || mr_parent("mod", "parent.jar", b"parent", requires.clone(), &catalogue);
        // Missing: named once, by its file name.
        let resolution = plan(&app, id, parent(), false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(
            resolution.skipped[0].name.as_deref(),
            Some("library-1.0.jar")
        );
        assert_eq!(resolution.skipped[0].project_id, "library-1.0.jar");
        // A jar the scanner has not seen yet.
        let mods = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_mods_path();
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("library-1.0.jar"), b"library").unwrap();
        assert!(first_getter(&app, id, &parent()).await.is_none());
        std::fs::remove_file(mods.join("library-1.0.jar")).unwrap();
        // A scanned copy.
        unlinked(&app, id, "library-1.0.jar", None).await;
        assert!(first_getter(&app, id, &parent()).await.is_none());
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());
    }

    #[tokio::test]
    async fn unlinked_jar_with_the_dependency_mod_id_is_kept() {
        // A Sodium CI build the scan cannot link to the project: Fabric refuses
        // to start with two jars of mod id "sodium".
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "unlinked", "1.20.1", Some(ModLoaderType::Fabric)).await;
        unlinked(&app, id, "sodium-ci-build.jar", Some("sodium")).await;
        let catalogue = Arc::new(sodium_fixture(&[("OihdIimA", &["1.20.1"])]));
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("OihdIimA")),
            &catalogue,
        );
        let root_id = iris.id();
        let resolution = plan(&app, id, iris, false).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert!(resolution.skipped.is_empty());
        assert_eq!(catalogue.calls(), vec![format!("mr_project:{SODIUM}")]);
        // Same on CurseForge, where Forge mod ids use '_' for the slug's '-'.
        let id = instance(&app, "unlinked-cf", "1.20.1", Some(ModLoaderType::Forge)).await;
        unlinked(&app, id, "cloth-config-dev.jar", Some("cloth_config")).await;
        let mut fixture = Fixture::default();
        fixture
            .cf_mods
            .insert(348521, cf_mod(348521, "cloth-config"));
        let catalogue = Arc::new(fixture);
        let parent = cf_parent("parent.jar", b"parent", 348521, &catalogue);
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert_eq!(catalogue.calls(), vec!["cf_mod:348521".to_owned()]);
    }

    #[tokio::test]
    async fn enabled_jar_of_a_disabled_copys_mod_id_satisfies_a_fresh_install() {
        // The Duplicate mods resolver kept a Sodium build from GitHub and
        // disabled the Modrinth copy. Iris must install: enabling the disabled
        // copy, as the refusal would advise, starts the game with two "sodium".
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "github-sodium", "1.20.1", Some(ModLoaderType::Fabric)).await;
        unlinked(&app, id, "sodium-github-build.jar", Some("sodium")).await;
        installed_copy(
            &app,
            id,
            "sodium-fabric-0.5.13+mc1.20.1.jar",
            false,
            None,
            Some((SODIUM, "OihdIimA", "Sodium")),
            Some("sodium"),
        )
        .await;
        let catalogue = Arc::new(sodium_fixture(&[("OihdIimA", &["1.20.1"])]));
        let iris = || {
            mr_parent(
                "mod",
                "iris.jar",
                b"iris",
                modrinth_requires(Some(SODIUM), Some("OihdIimA")),
                &catalogue,
            )
        };
        let root_id = iris().id();
        let resolution = plan(&app, id, iris(), false).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert!(resolution.skipped.is_empty());
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());

        // The enabled copy is linked only to the other platform.
        let id = instance(&app, "cf-sodium", "1.20.1", Some(ModLoaderType::Fabric)).await;
        installed_copy(
            &app,
            id,
            "sodium-curseforge.jar",
            true,
            Some((394468, 1, "Sodium")),
            None,
            Some("sodium"),
        )
        .await;
        installed_copy(
            &app,
            id,
            "sodium-modrinth.jar",
            false,
            None,
            Some((SODIUM, "OihdIimA", "Sodium")),
            Some("sodium"),
        )
        .await;
        assert!(first_getter(&app, id, &iris()).await.is_none());

        // The same through a CurseForge requirement, without any lookup.
        let id = instance(
            &app,
            "cf-github-sodium",
            "1.20.1",
            Some(ModLoaderType::Fabric),
        )
        .await;
        unlinked(&app, id, "sodium-github-build.jar", Some("sodium")).await;
        installed_copy(
            &app,
            id,
            "sodium-curseforge.jar",
            false,
            // One CurseForge file links to one copy; the first case owns file 1.
            Some((394468, 2, "Sodium")),
            None,
            Some("sodium"),
        )
        .await;
        let parent = cf_parent("iris.jar", b"iris", 394468, &catalogue);
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());

        // An enabled jar of another mod id does not provide it.
        let id = instance(&app, "other-mod", "1.20.1", Some(ModLoaderType::Fabric)).await;
        unlinked(&app, id, "lithium.jar", Some("lithium")).await;
        installed_copy(
            &app,
            id,
            "sodium-modrinth.jar",
            false,
            None,
            Some((SODIUM, "OihdIimA", "Sodium")),
            Some("sodium"),
        )
        .await;
        let error = first_getter(&app, id, &iris())
            .await
            .unwrap()
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("installed but disabled"), "{error}");
    }

    fn sodium_releases(versions: &[(&str, &str, &[&str])]) -> Fixture {
        let mut fixture = sodium_fixture(&[]);
        for (id, number, games) in versions {
            let mut version = mr_version(
                SODIUM,
                id,
                &format!("sodium-{number}.jar"),
                id.as_bytes(),
                games,
                &["fabric"],
                serde_json::json!([]),
            );
            version.version_number = (*number).into();
            fixture.mr_versions.insert((*id).into(), version);
        }
        fixture
    }

    #[tokio::test]
    async fn pin_for_another_release_prefers_the_same_release_and_says_nothing() {
        // Indium 1.0.36+mc1.20.1 is listed for 1.20.6 but pins Sodium
        // mc1.20.1-0.5.11; the same release is built for 1.20.6.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "same-release", "1.20.6", Some(ModLoaderType::Fabric)).await;
        let mut fixture = sodium_releases(&[
            ("ygf8cVZg", "mc1.20.1-0.5.11", &["1.20.1"]),
            ("OwLQelEI", "mc1.20.6-0.5.11", &["1.20.6"]),
            ("NewerOne", "mc1.20.6-0.5.12", &["1.20.6"]),
        ]);
        // The newest compatible version is 0.5.12.
        fixture
            .mr_versions
            .get_mut("NewerOne")
            .unwrap()
            .date_published = "2026-02-01T00:00:00Z".parse().unwrap();
        let catalogue = Arc::new(fixture);
        let indium = mr_parent(
            "mod",
            "indium.jar",
            b"indium",
            modrinth_requires(Some(SODIUM), Some("ygf8cVZg")),
            &catalogue,
        );
        let root_id = indium.id();
        let resolution = plan(&app, id, indium, false).await.unwrap();
        assert_eq!(
            root_children(&resolution, &root_id).await,
            vec![format!("modrinth:{SODIUM}:OwLQelEI")]
        );
        assert!(resolution.skipped.is_empty());
        assert!(resolution.replaced.is_empty(), "{:?}", resolution.replaced);
    }

    #[tokio::test]
    async fn pin_replaced_by_another_release_is_reported_with_both_versions() {
        // Iris 1.11.4+26.1 is listed for 26.1.1 but pins Sodium mc26.1.2-0.9.2
        // (26.1.2 only); 26.1.1 only has 0.8.9, which Iris may refuse.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "other-release", "26.1.1", Some(ModLoaderType::Fabric)).await;
        let catalogue = Arc::new(sodium_releases(&[
            ("tZQ3jqnf", "mc26.1.2-0.9.2", &["26.1.2"]),
            ("uGvVQBnw", "mc26.1.1-0.8.9-fabric", &["26.1.1"]),
        ]));
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("tZQ3jqnf")),
            &catalogue,
        );
        let root_id = iris.id();
        let resolution = plan(&app, id, iris, false).await.unwrap();
        assert_eq!(
            root_children(&resolution, &root_id).await,
            vec![format!("modrinth:{SODIUM}:uGvVQBnw")]
        );
        assert!(resolution.skipped.is_empty());
        assert_eq!(resolution.replaced.len(), 1, "{:?}", resolution.replaced);
        assert_eq!(
            TaskWarning::from(resolution.replaced[0].clone()),
            TaskWarning::ReplacedPinnedVersion {
                name: "Sodium".into(),
                project_id: SODIUM.into(),
                pinned: "mc26.1.2-0.9.2".into(),
                installed: "mc26.1.1-0.8.9-fabric".into(),
            }
        );
        // A pin that fits, or that is gone, is not reported.
        let id = instance(&app, "fitting-pin", "26.1.2", Some(ModLoaderType::Fabric)).await;
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("tZQ3jqnf")),
            &catalogue,
        );
        assert!(
            plan(&app, id, iris, false)
                .await
                .unwrap()
                .replaced
                .is_empty()
        );
        let id = instance(&app, "gone-pin", "26.1.1", Some(ModLoaderType::Fabric)).await;
        let iris = mr_parent(
            "mod",
            "iris.jar",
            b"iris",
            modrinth_requires(Some(SODIUM), Some("gone")),
            &catalogue,
        );
        assert!(
            plan(&app, id, iris, false)
                .await
                .unwrap()
                .replaced
                .is_empty()
        );
    }

    #[tokio::test]
    async fn fabric_api_pin_for_another_release_is_replaced_quietly() {
        // A mod listed for 1.20.4 pins Fabric API 0.86.1+1.20.1 but runs with
        // any build. Every build names its own Minecraft version, so the one
        // for 1.20.4 is always another release; that is not worth a warning.
        let app = crate::setup_managers_for_test().await;
        for (api, loader, kind, pinned, built) in [
            (
                "P7dR8mSH",
                "fabric",
                ModLoaderType::Fabric,
                "0.86.1+1.20.1",
                "0.97.0+1.20.4",
            ),
            (
                "qvIfYCYJ",
                "quilt",
                ModLoaderType::Quilt,
                "7.0.6+0.86.1-1.20.1",
                "9.0.0+0.97.0-1.20.4",
            ),
        ] {
            let mut fixture = Fixture::default();
            fixture
                .mr_projects
                .insert(api.into(), mr_project(api, "API", "mod"));
            for (version_id, number, game) in
                [("Pinned", pinned, "1.20.1"), ("Built", built, "1.20.4")]
            {
                let mut version = mr_version(
                    api,
                    version_id,
                    &format!("{version_id}.jar"),
                    number.as_bytes(),
                    &[game],
                    &[loader],
                    serde_json::json!([]),
                );
                version.version_number = number.into();
                fixture.mr_versions.insert(version_id.into(), version);
            }
            let catalogue = Arc::new(fixture);
            let id = instance(&app, &format!("{loader}-api-pin"), "1.20.4", Some(kind)).await;
            let parent = mr_parent(
                "mod",
                "parent.jar",
                b"parent",
                modrinth_requires(Some(api), Some("Pinned")),
                &catalogue,
            );
            let root_id = parent.id();
            let resolution = plan(&app, id, parent, false).await.unwrap();
            assert_eq!(
                root_children(&resolution, &root_id).await,
                vec![format!("modrinth:{api}:Built")]
            );
            assert!(resolution.skipped.is_empty(), "{:?}", resolution.skipped);
            assert!(resolution.replaced.is_empty(), "{:?}", resolution.replaced);
        }
    }

    fn fabric_api_fixture(project_loaders: &[&str], version_loaders: &[&str]) -> Fixture {
        let mut fixture = Fixture::default();
        let mut project = mr_project("P7dR8mSH", "Fabric API", "mod");
        project.loaders = project_loaders.iter().map(|l| (*l).to_owned()).collect();
        fixture.mr_projects.insert("P7dR8mSH".into(), project);
        fixture.mr_versions.insert(
            "F1".into(),
            mr_version(
                "P7dR8mSH",
                "F1",
                "fabric-api.jar",
                b"f",
                &["1.21.1"],
                version_loaders,
                serde_json::json!([]),
            ),
        );
        fixture.cf_mods.insert(306612, cf_mod(306612, "Fabric API"));
        fixture.cf_files.insert(
            306612,
            vec![cf_file(
                306612,
                61,
                "fabric-api.jar",
                &["1.21.1", "Fabric"],
                true,
            )],
        );
        fixture
    }

    fn multi_loader_parent(loaders: &[&str], catalogue: &Arc<Fixture>) -> ModrinthModInstaller {
        let mut parent = mr_parent(
            "mod",
            "lambdynamiclights.jar",
            b"ldl",
            modrinth_requires(Some("P7dR8mSH"), None),
            catalogue,
        );
        parent.version.game_versions = vec!["1.21.1".into()];
        parent.version.loaders = loaders.iter().map(|l| (*l).to_owned()).collect();
        parent
    }

    #[tokio::test]
    async fn requirement_of_another_loaders_build_is_left_out_quietly() {
        // LambDynamicLights 4.8.11+1.21.1 is one upload for Fabric, NeoForge and
        // Quilt and requires Fabric API, which NeoForge neither has nor needs.
        let app = crate::setup_managers_for_test().await;
        let id = instance(
            &app,
            "multi-loader",
            "1.21.1",
            Some(ModLoaderType::Neoforge),
        )
        .await;
        let catalogue = Arc::new(fabric_api_fixture(&["fabric"], &["fabric"]));
        let parent = multi_loader_parent(&["fabric", "neoforge", "quilt"], &catalogue);
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, false).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert!(resolution.skipped.is_empty(), "{:?}", resolution.skipped);

        // Same for a CurseForge file tagged for both loaders.
        let mut parent = cf_parent("parent.jar", b"parent", 306612, &catalogue);
        parent.file.game_versions = vec!["1.21.1".into(), "Fabric".into(), "NeoForge".into()];
        assert!(first_getter(&app, id, &parent).await.is_none());

        // A NeoForge-only parent really asks for it: reported.
        let parent = multi_loader_parent(&["neoforge"], &catalogue);
        let resolution = plan(&app, id, parent, false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(resolution.skipped[0].name.as_deref(), Some("Fabric API"));
        let mut parent = cf_parent("parent.jar", b"parent", 306612, &catalogue);
        parent.file.game_versions = vec!["1.21.1".into(), "NeoForge".into()];
        let resolution = plan(&app, id, parent, false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);

        // A dependency that has NeoForge builds, just none for this release,
        // is still reported for a multi-loader parent.
        let catalogue = Arc::new(fabric_api_fixture(&["fabric", "neoforge"], &["fabric"]));
        let parent = multi_loader_parent(&["fabric", "neoforge"], &catalogue);
        let resolution = plan(&app, id, parent, false).await.unwrap();
        assert_eq!(resolution.skipped.len(), 1);
        assert_eq!(
            resolution.skipped[0].reason,
            SkippedDependencyReason::NoCompatibleVersion
        );
    }

    #[test]
    fn release_of_ignores_minecraft_versions_and_loaders() {
        let release = |number: &str, games: &[&str]| {
            let mut version = mr_version(
                SODIUM,
                "id",
                "sodium.jar",
                b"s",
                games,
                &["fabric"],
                serde_json::json!([]),
            );
            version.version_number = number.into();
            dependency::release_of(&version)
        };
        assert_eq!(
            release("mc1.20.1-0.5.11", &["1.20.1"]),
            release("mc1.20.6-0.5.11", &["1.20.6"])
        );
        assert_eq!(
            release("0.92.2+1.20.1", &["1.20.1"]),
            release("0.92.2+1.20.2", &["1.20.2"])
        );
        assert_eq!(
            release("v3.11.10-fabric", &["1.21"]),
            release("3.11.10+neoforge", &["1.21.1"])
        );
        assert_ne!(
            release("mc26.1.2-0.9.2", &["26.1.2"]),
            release("mc26.1.1-0.8.9-fabric", &["26.1.1"])
        );
    }

    #[tokio::test]
    async fn every_disabled_copy_is_named_when_there_are_several() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "two-disabled", "1.20.1", Some(ModLoaderType::Fabric)).await;
        for (filename, file) in [("fabric-api-0.92.jar", 2), ("fabric-api-0.90.jar", 1)] {
            installed(
                &app,
                id,
                filename,
                false,
                Some((306612, file, "Fabric API")),
                None,
            )
            .await;
        }
        let catalogue = Arc::new(Fixture::default());
        let parent = cf_parent("parent.jar", b"parent", 306612, &catalogue);
        let error = first_getter(&app, id, &parent)
            .await
            .unwrap()
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains(
                "Fabric API is installed but disabled (fabric-api-0.90.jar, fabric-api-0.92.jar)"
            ),
            "{error}"
        );
        assert!(error.contains("Enable one of them in Addons"), "{error}");
    }

    // --- Updates only add what the new version newly requires -----------------

    fn mr_update(
        new_requires: serde_json::Value,
        catalogue: &Arc<Fixture>,
    ) -> ModrinthModInstaller {
        let mut parent = mr_parent("mod", "parent.jar", b"new parent", new_requires, catalogue);
        parent.replaces(&dependency::ReplacedIds {
            curseforge: None,
            modrinth_version: Some("parent-1".into()),
        });
        parent
    }

    fn old_parent(fixture: &mut Fixture, old_requires: serde_json::Value) {
        fixture.mr_versions.insert(
            "parent-1".into(),
            mr_version(
                "parent",
                "parent-1",
                "parent.jar",
                b"parent",
                &["1.20.1"],
                &["fabric"],
                old_requires,
            ),
        );
    }

    #[tokio::test]
    async fn update_only_resolves_requirements_the_new_version_introduces() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "update-new", "1.20.1", Some(ModLoaderType::Fabric)).await;
        // The old version already required Sodium: left to the user, as 2.0.40.
        let mut fixture = sodium_fixture(&[("OihdIimA", &["1.20.1"])]);
        old_parent(&mut fixture, modrinth_requires(Some(SODIUM), None));
        let catalogue = Arc::new(fixture);
        let parent = mr_update(
            modrinth_requires(Some(SODIUM), Some("OihdIimA")),
            &catalogue,
        );
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, true).await.unwrap();
        assert_eq!(planned_ids(&resolution).await, vec![(root_id, vec![])]);
        assert!(resolution.skipped.is_empty());
        assert_eq!(catalogue.calls(), vec!["mr_version:parent-1".to_owned()]);
        // A requirement new in this version is still added (GPT-0022).
        let mut fixture = sodium_fixture(&[("OihdIimA", &["1.20.1"])]);
        old_parent(&mut fixture, serde_json::json!([]));
        let catalogue = Arc::new(fixture);
        let parent = mr_update(
            modrinth_requires(Some(SODIUM), Some("OihdIimA")),
            &catalogue,
        );
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, true).await.unwrap();
        assert_eq!(
            root_children(&resolution, &root_id).await,
            vec![format!("modrinth:{SODIUM}:OihdIimA")]
        );
        // The old version is gone from Modrinth: every missing requirement is new.
        let catalogue = Arc::new(sodium_fixture(&[("OihdIimA", &["1.20.1"])]));
        let parent = mr_update(modrinth_requires(Some(SODIUM), None), &catalogue);
        let root_id = parent.id();
        let resolution = plan(&app, id, parent, true).await.unwrap();
        assert_eq!(
            root_children(&resolution, &root_id).await,
            vec![format!("modrinth:{SODIUM}:OihdIimA")]
        );
        // The old version cannot be looked up right now: retry, never guess.
        let mut fixture = sodium_fixture(&[("OihdIimA", &["1.20.1"])]);
        old_parent(&mut fixture, serde_json::json!([]));
        fixture.status = Some(503);
        fixture.down = Some("mr_version:parent-1");
        let catalogue = Arc::new(fixture);
        let parent = mr_update(modrinth_requires(Some(SODIUM), None), &catalogue);
        let error = plan(&app, id, parent, true).await.err().unwrap();
        assert!(
            format!("{error:#}").contains("Could not look up the version this update replaces"),
            "{error:#}"
        );
    }

    #[tokio::test]
    async fn curseforge_update_keeps_a_library_the_old_file_already_required() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "update-cf", "1.12.2", Some(ModLoaderType::Forge)).await;
        let mut fixture = Fixture::default();
        let mut old = cf_file(10, 1, "parent-old.jar", &["1.12.2"], true);
        old.dependencies =
            serde_json::from_value(serde_json::json!([{"modId": 227083, "relationType": 3}]))
                .unwrap();
        fixture.cf_files.insert(10, vec![old]);
        let catalogue = Arc::new(fixture);
        let mut parent = cf_parent("parent.jar", b"new parent", 227083, &catalogue);
        parent.replaces(&dependency::ReplacedIds {
            curseforge: Some((10, 1)),
            modrinth_version: None,
        });
        assert!(first_getter(&app, id, &parent).await.is_none());
        assert_eq!(catalogue.calls(), vec!["cf_file:10:1".to_owned()]);
    }

    #[tokio::test]
    async fn update_never_downloads_a_second_copy_of_an_unlinked_library() {
        // Sodium was added by hand (a CI build), so the scan has no platform id
        // for it. Updating Iris, whose old version already required Sodium,
        // must not put the platform's Sodium next to it.
        let app = crate::setup_managers_for_test().await;
        let id = instance(
            &app,
            "update-unlinked",
            "1.20.1",
            Some(ModLoaderType::Fabric),
        )
        .await;
        let parent = installed(
            &app,
            id,
            "parent.jar",
            true,
            None,
            Some(("parent", "parent-1", "Parent")),
        )
        .await;
        unlinked(&app, id, "sodium-ci-build.jar", None).await;
        let mut fixture = sodium_fixture(&[("ygf8cVZg", &["1.20.1"])]);
        old_parent(
            &mut fixture,
            modrinth_requires(Some(SODIUM), Some("ygf8cVZg")),
        );
        let catalogue = Arc::new(fixture);
        let mut installer = Installer::new(Box::new(mr_parent(
            "mod",
            "parent.jar",
            b"new parent",
            modrinth_requires(Some(SODIUM), Some("ygf8cVZg")),
            &catalogue,
        )));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"new parent").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer
            .install(&app, id, false, Some(parent.id))
            .await
            .unwrap();
        settle(&installer, &app, task).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        let mods = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_mods_path();
        let mut files: Vec<_> = std::fs::read_dir(&mods)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        assert_eq!(files, vec!["parent.jar", "sodium-ci-build.jar"]);
        assert_eq!(catalogue.calls(), vec!["mr_version:parent-1".to_owned()]);
        assert!(app.task_manager().get_task_warnings().await.is_empty());
    }

    fn identity_of(row: &mfcdb::ModFileCacheRow) -> (String, String, bool, String) {
        (
            row.id.clone(),
            row.filename.clone(),
            row.enabled,
            row.addon_type.clone(),
        )
    }

    async fn identity(app: &Arc<AppInner>, id: &str) -> (String, String, bool, String) {
        identity_of(
            &mfcdb::get_mod_file_cache_by_id(&app.db, id)
                .await
                .unwrap()
                .expect("dependency row retired"),
        )
    }

    async fn settle(installer: &Installer, app: &Arc<AppInner>, task: VisualTaskId) {
        tokio::time::timeout(
            Duration::from_secs(20),
            wait_install_completion(installer.abort_handle.lock().await.completion.clone()),
        )
        .await
        .unwrap()
        .unwrap();
        for _ in 0..100 {
            if !matches!(
                app.task_manager().get_task_outcome(task).await,
                crate::managers::vtask::TaskOutcome::Running(_)
            ) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn update_keeps_an_installed_dependency_pinned_at_another_version() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "update-pin", "1.20.1", Some(ModLoaderType::Fabric)).await;
        let parent = installed(
            &app,
            id,
            "parent.jar",
            true,
            None,
            Some(("parent", "parent-1", "Parent")),
        )
        .await;
        let sodium = installed(
            &app,
            id,
            "sodium-0.5.13.jar",
            true,
            None,
            Some((SODIUM, "OihdIimA", "Sodium")),
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let sodium_path = path.get_mods_path().join("sodium-0.5.13.jar");
        let sodium_bytes = std::fs::read(&sodium_path).unwrap();
        let catalogue = Arc::new(Fixture::default());
        let mut installer = Installer::new(Box::new(mr_parent(
            "mod",
            "parent.jar",
            b"new parent",
            modrinth_requires(Some(SODIUM), Some("ygf8cVZg")),
            &catalogue,
        )));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"new parent").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer
            .install(&app, id, false, Some(parent.id))
            .await
            .unwrap();
        settle(&installer, &app, task).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        assert_eq!(
            std::fs::read(path.get_mods_path().join("parent.jar")).unwrap(),
            b"new parent"
        );
        assert_eq!(std::fs::read(&sodium_path).unwrap(), sodium_bytes);
        // The post-install scan refreshes the fixture's synthetic metadata; the
        // dependency's own row is kept, never replaced or retired.
        assert_eq!(identity(&app, &sodium.id).await, identity_of(&sodium));
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());
    }

    #[tokio::test]
    async fn update_all_over_a_disabled_mod_with_a_disabled_library_completes() {
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "disabled-pair", "1.20.1", Some(ModLoaderType::Fabric)).await;
        let parent = installed(&app, id, "parent.jar", false, Some((10, 1, "Parent")), None).await;
        let library = installed(
            &app,
            id,
            "library.jar",
            false,
            Some((42, 7, "Library")),
            None,
        )
        .await;
        let catalogue = Arc::new(Fixture::default());
        let mut installer = Installer::new(Box::new(cf_parent(
            "parent.jar",
            b"new parent",
            42,
            &catalogue,
        )));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"new parent").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer
            .install(&app, id, false, Some(parent.id))
            .await
            .unwrap();
        settle(&installer, &app, task).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        let mods = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_mods_path();
        assert_eq!(
            std::fs::read(mods.join("parent.jar.disabled")).unwrap(),
            b"new parent"
        );
        assert!(!mods.join("parent.jar").exists());
        assert_eq!(
            std::fs::read(mods.join("library.jar.disabled")).unwrap(),
            b"library.jar"
        );
        assert_eq!(identity(&app, &library.id).await, identity_of(&library));
        assert!(catalogue.calls().is_empty());
    }

    /// Updates `parent.jar`, whose new version newly requires Sodium, where
    /// its row records `row_enabled` and its file on disk is `file_enabled`
    /// (a rename the scan has not recorded yet when they differ). The update
    /// follows the file: Sodium is added only next to an enabled copy.
    async fn update_to_a_new_requirement(
        app: &Arc<AppInner>,
        base: &str,
        row_enabled: bool,
        file_enabled: bool,
    ) {
        let label = format!("row {row_enabled}, file {file_enabled}");
        let id = instance(
            app,
            &format!("new-requirement-{row_enabled}-{file_enabled}"),
            "1.20.1",
            Some(ModLoaderType::Fabric),
        )
        .await;
        let parent = installed(
            app,
            id,
            "parent.jar",
            row_enabled,
            None,
            Some(("parent", "parent-1", "Parent")),
        )
        .await;
        // The fixture seeds synthetic metadata; settle the parent's real
        // bytes first, so the scan that follows the library's publication
        // does not look like the replaced copy changing under the update.
        let manager = app.instance_manager();
        let mods = publication::current_path(app, id)
            .await
            .unwrap()
            .get_mods_path();
        let operation = manager.lock_instance(id).await;
        app.meta_cache_manager()
            .refresh_published_addon(
                &operation,
                &mods,
                parent.filename.clone(),
                row_enabled,
                parent.addon_type.clone(),
            )
            .await
            .unwrap();
        drop(operation);
        let parent = manager.get_owned_mod_file(id, &parent.id).await.unwrap();
        assert_eq!(parent.enabled, row_enabled, "{label}");
        if file_enabled != row_enabled {
            let spelling = |enabled: bool| {
                mods.join(if enabled {
                    "parent.jar"
                } else {
                    "parent.jar.disabled"
                })
            };
            std::fs::rename(spelling(row_enabled), spelling(file_enabled)).unwrap();
        }
        let mut fixture = sodium_fixture(&[]);
        // The served artifact is `test`, so the version's hashes are of it.
        let mut sodium = mr_version(
            SODIUM,
            "OihdIimA",
            "sodium.jar",
            b"test",
            &["1.20.1"],
            &["fabric"],
            serde_json::json!([]),
        );
        sodium.files[0].url = format!("{base}/sodium.jar");
        fixture.mr_versions.insert("OihdIimA".into(), sodium);
        old_parent(&mut fixture, serde_json::json!([]));
        let catalogue = Arc::new(fixture);
        let mut installer = Installer::new(Box::new(mr_parent(
            "mod",
            "parent.jar",
            b"new parent",
            modrinth_requires(Some(SODIUM), Some("OihdIimA")),
            &catalogue,
        )));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"new parent").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer
            .install(app, id, false, Some(parent.id))
            .await
            .unwrap();
        settle(&installer, app, task).await;
        let outcome = match app.task_manager().get_task_outcome(task).await {
            crate::managers::vtask::TaskOutcome::Succeeded => "succeeded".to_owned(),
            crate::managers::vtask::TaskOutcome::Running(_) => "still running".to_owned(),
            crate::managers::vtask::TaskOutcome::Failed(task) => {
                format!("failed: {:?}", task.progress)
            }
            _ => "dismissed or unavailable".to_owned(),
        };
        assert_eq!(outcome, "succeeded", "{label}");
        let mut files: Vec<_> = std::fs::read_dir(&mods)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        if file_enabled {
            assert_eq!(files, vec!["parent.jar", "sodium.jar"], "{label}");
            assert_eq!(
                std::fs::read(mods.join("parent.jar")).unwrap(),
                b"new parent"
            );
            assert_eq!(std::fs::read(mods.join("sodium.jar")).unwrap(), b"test");
        } else {
            assert_eq!(files, vec!["parent.jar.disabled"], "{label}");
            assert_eq!(
                std::fs::read(mods.join("parent.jar.disabled")).unwrap(),
                b"new parent"
            );
            assert!(
                catalogue.calls().is_empty(),
                "{label}: {:?}",
                catalogue.calls()
            );
        }
        assert!(app.task_manager().get_task_warnings().await.is_empty());
    }

    #[tokio::test]
    async fn updating_a_disabled_mod_adds_no_library_its_new_version_requires() {
        // Iris 1.7.1 required nothing and 1.7.6 requires Sodium. An enabled
        // Iris gets Sodium with the update; a disabled one stays as it was, so
        // the game is not handed a library that may break what runs instead.
        let app = crate::setup_managers_for_test().await;
        let base = super::round1_graph::serve_artifacts().await;
        for enabled in [false, true] {
            update_to_a_new_requirement(&app, &base, enabled, enabled).await;
        }

        // A disabled CurseForge mod whose new file requires a library that
        // cannot be resolved updates without the sticky warning about it.
        let id = instance(
            &app,
            "cf-new-requirement",
            "1.20.1",
            Some(ModLoaderType::Fabric),
        )
        .await;
        let parent = installed(&app, id, "parent.jar", false, Some((10, 1, "Parent")), None).await;
        let catalogue = Arc::new(Fixture::default());
        let mut installer = Installer::new(Box::new(cf_parent(
            "parent.jar",
            b"new parent",
            42,
            &catalogue,
        )));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"new parent").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer
            .install(&app, id, false, Some(parent.id))
            .await
            .unwrap();
        settle(&installer, &app, task).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        let mods = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_mods_path();
        let files: Vec<_> = std::fs::read_dir(&mods)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(files, vec!["parent.jar.disabled"]);
        assert!(catalogue.calls().is_empty(), "{:?}", catalogue.calls());
        assert!(app.task_manager().get_task_warnings().await.is_empty());
    }

    #[tokio::test]
    async fn an_update_adds_libraries_by_the_file_on_disk_not_a_stale_row() {
        // The user renamed the jar in the file manager and updated it before
        // the scan recorded the rename. The update is published as the file
        // is, so its libraries follow the file too: Sodium next to an enabled
        // Iris whose row still says disabled, none next to a disabled one
        // whose row still says enabled.
        let app = crate::setup_managers_for_test().await;
        let base = super::round1_graph::serve_artifacts().await;
        for file_enabled in [true, false] {
            update_to_a_new_requirement(&app, &base, !file_enabled, file_enabled).await;
        }
    }

    #[tokio::test]
    async fn resource_pack_installs_without_a_dependency_vanilla_cannot_load() {
        // 'Translations for Sodium' on a vanilla instance requires Sodium.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "vanilla-pack", "1.20.1", None).await;
        let mut fixture = Fixture::default();
        fixture
            .mr_projects
            .insert(SODIUM.into(), mr_project(SODIUM, "Sodium", "mod"));
        fixture.mr_versions.insert(
            "OihdIimA".into(),
            mr_version(
                SODIUM,
                "OihdIimA",
                "sodium.jar",
                b"s",
                &["1.20.1"],
                &["fabric"],
                serde_json::json!([]),
            ),
        );
        let catalogue = Arc::new(fixture);
        let mut pack = mr_parent(
            "resourcepack",
            "translations.zip",
            b"pack",
            modrinth_requires(Some(SODIUM), None),
            &catalogue,
        );
        // Modrinth version names are often a bare number.
        pack.version.name = "4.0".into();
        pack.mod_info.title = "Translations for Sodium".into();
        let mut installer = Installer::new(Box::new(pack));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"pack").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer.install(&app, id, true, None).await.unwrap();
        settle(&installer, &app, task).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        let path = publication::current_path(&app, id).await.unwrap();
        assert_eq!(
            std::fs::read(path.get_resourcepacks_path().join("translations.zip")).unwrap(),
            b"pack"
        );
        assert!(
            std::fs::read_dir(path.get_mods_path())
                .map(|entries| entries.count() == 0)
                .unwrap_or(true),
            "no incompatible Sodium may be downloaded"
        );
        let mut pending = Vec::new();
        for _ in 0..100 {
            pending = app.task_manager().get_task_warnings().await;
            if !pending.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, task);
        // The toast names the project, not the version "4.0".
        assert_eq!(
            pending[0].subject.as_deref(),
            Some("Translations for Sodium")
        );
        assert_eq!(
            pending[0].warnings,
            vec![TaskWarning::SkippedDependency {
                name: Some("Sodium".into()),
                platform: DependencyPlatform::Modrinth,
                project_id: SODIUM.into(),
                reason: SkippedDependencyReason::NoCompatibleVersion,
            }]
        );
        app.task_manager().acknowledge_task_warnings(task).await;
        assert!(app.task_manager().get_task_warnings().await.is_empty());
    }

    #[tokio::test]
    async fn failure_of_the_selected_addon_still_errors() {
        // Only dependencies are skipped: a failure of the addon the user picked
        // (here its own download) still fails the task and keeps the old file.
        let app = crate::setup_managers_for_test().await;
        let id = instance(&app, "root-failure", "1.20.1", Some(ModLoaderType::Fabric)).await;
        let parent = installed(&app, id, "parent.jar", true, Some((10, 1, "Parent")), None).await;
        let catalogue = Arc::new(Fixture::default());
        let mut installer = Installer::new(Box::new(cf_parent(
            "parent.jar",
            b"new parent",
            9,
            &catalogue,
        )));
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"corrupted download").unwrap();
        installer.download_fixture = Some(source.path().into());
        let task = installer
            .install(&app, id, false, Some(parent.id))
            .await
            .unwrap();
        settle(&installer, &app, task).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Failed(_)
        ));
        let mods = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_mods_path();
        assert_eq!(
            std::fs::read(mods.join("parent.jar")).unwrap(),
            b"parent.jar"
        );
        assert!(app.task_manager().get_task_warnings().await.is_empty());
    }
}
