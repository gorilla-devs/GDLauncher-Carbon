//! A user's Delete of an addon follows "Deletion through Recycle Bin": on,
//! which it is by default, the mod, pack or world is moved there; off, it is
//! erased. A selection is deleted as one step with a result per addon.

use super::mods::tests::addon_fixture;
use crate::domain::instance::{AddonType, InstanceId};
use crate::managers::AppInner;
use crate::util::recycle_bin::bin_under_test;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::mod_file_cache as mfcdb;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::Arc;

async fn without_recycle_bin(app: &Arc<AppInner>) {
    app.settings_manager()
        .set(
            carbon_repos::repos::app_configuration::AppConfigurationPatch {
                deletion_through_recycle_bin: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
}

async fn row_exists(app: &Arc<AppInner>, id: &str) -> bool {
    mfcdb::get_mod_file_cache_by_id(&app.db, id)
        .await
        .unwrap()
        .is_some()
}

/// A second mod in the instance of `first`, beside it.
async fn another_mod(
    app: &Arc<AppInner>,
    instance: InstanceId,
    first: &mfcdb::ModFileCacheRow,
    beside: &Path,
    filename: &str,
) -> (mfcdb::ModFileCacheRow, PathBuf) {
    let path = beside.with_file_name(filename);
    std::fs::write(&path, filename.as_bytes()).unwrap();
    mfcdb::upsert_mod_file_cache(
        &app.db,
        *instance,
        filename.to_owned(),
        filename.len() as i32,
        true,
        "mods".into(),
        first.metadata_id.clone(),
        DbDateTime(Utc::now().fixed_offset()),
    )
    .await
    .unwrap();
    let row = mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *instance, filename)
        .await
        .unwrap()
        .unwrap();
    (row, path)
}

#[tokio::test]
async fn a_deleted_mod_goes_to_the_recycle_bin_by_default() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) =
        addon_fixture(&app, "binned-mod", AddonType::Mods, "own.jar", b"hand made").await;

    let kept = app
        .instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap();

    assert_eq!(kept, None);
    assert!(!path.exists());
    assert_eq!(
        std::fs::read(bin_under_test(&path).join("own.jar")).unwrap(),
        b"hand made"
    );
    assert!(!row_exists(&app, &row.id).await);
}

/// A disabled addon is the `.disabled` file, and that is what is moved.
#[tokio::test]
async fn a_deleted_disabled_pack_goes_to_the_recycle_bin_under_its_own_name() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = addon_fixture(
        &app,
        "binned-disabled",
        AddonType::ResourcePacks,
        "pack.zip",
        b"edited pack",
    )
    .await;
    app.instance_manager()
        .enable_mod(id, row.id.clone(), false)
        .await
        .unwrap();

    app.instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap();

    assert_eq!(
        std::fs::read(bin_under_test(&path).join("pack.zip.disabled")).unwrap(),
        b"edited pack"
    );
    assert!(!row_exists(&app, &row.id).await);
}

#[tokio::test]
async fn a_deleted_world_goes_to_the_recycle_bin_whole() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) =
        addon_fixture(&app, "binned-world", AddonType::Worlds, "My World", b"save").await;
    std::fs::create_dir(path.join("region")).unwrap();
    std::fs::write(path.join("region").join("r.0.0.mca"), b"chunks").unwrap();

    app.instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap();

    assert!(!path.exists());
    let binned = bin_under_test(&path).join("My World");
    assert_eq!(std::fs::read(binned.join("level.dat")).unwrap(), b"save");
    assert_eq!(
        std::fs::read(binned.join("region").join("r.0.0.mca")).unwrap(),
        b"chunks"
    );
    assert!(!row_exists(&app, &row.id).await);
}

#[tokio::test]
async fn with_the_recycle_bin_off_addons_are_erased() {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await;
    let (id, row, path) =
        addon_fixture(&app, "erased-mod", AddonType::Mods, "own.jar", b"jar").await;
    let (world_instance, world_row, world) =
        addon_fixture(&app, "erased-world", AddonType::Worlds, "World", b"save").await;

    app.instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap();
    app.instance_manager()
        .delete_mod(world_instance, world_row.id.clone())
        .await
        .unwrap();

    assert!(!path.exists() && !world.exists());
    assert!(!bin_under_test(&path).exists());
    assert!(!bin_under_test(&world).exists());
    assert!(!row_exists(&app, &row.id).await);
    assert!(!row_exists(&app, &world_row.id).await);
}

/// A Recycle Bin that refuses (a drive without one) deletes nothing: the
/// file and its row stay, the error says what to do, and the same delete
/// goes through once the setting is off.
#[tokio::test]
async fn a_recycle_bin_that_refuses_keeps_the_addon_and_its_row() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) =
        addon_fixture(&app, "refused-world", AddonType::Worlds, "World", b"save").await;
    // A file where the Recycle Bin's folder would be.
    std::fs::write(bin_under_test(&path), b"").unwrap();

    let error = app
        .instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap_err();

    let error = format!("{error:#}");
    assert!(error.contains("Nothing was deleted"), "{error}");
    assert!(error.contains("Recycle Bin"), "{error}");
    assert!(error.contains("turn off"), "{error}");
    assert_eq!(std::fs::read(path.join("level.dat")).unwrap(), b"save");
    assert!(row_exists(&app, &row.id).await);

    without_recycle_bin(&app).await;
    app.instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap();
    assert!(!path.exists());
    assert!(!row_exists(&app, &row.id).await);
}

/// A mod that is a link to a jar kept elsewhere: only the link is removed,
/// directly, and the jar it points to stays where it is.
#[cfg(unix)]
#[tokio::test]
async fn a_linked_mod_is_removed_as_a_link_and_never_through_the_recycle_bin() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) =
        addon_fixture(&app, "linked-mod", AddonType::Mods, "shared.jar", b"x").await;
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("shared.jar");
    std::fs::write(&target, b"shared jar").unwrap();
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();

    app.instance_manager()
        .delete_mod(id, row.id.clone())
        .await
        .unwrap();

    assert!(std::fs::symlink_metadata(&path).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"shared jar");
    assert!(!bin_under_test(&path).exists());
    assert!(!row_exists(&app, &row.id).await);
}

/// The duplicate-mods dialog deletes through the same setting.
#[tokio::test]
async fn a_duplicate_removed_from_its_dialog_goes_to_the_recycle_bin_too() {
    let app = crate::setup_managers_for_test().await;
    let (id, row, path) = addon_fixture(
        &app,
        "binned-duplicate",
        AddonType::Mods,
        "dupe.jar",
        b"dupe",
    )
    .await;

    app.instance_manager()
        .delete_mod_retryable(
            id,
            row.id.clone(),
            Some("7f2c1a52-64f2-4f6a-9a55-0d2f1f6f0c11".into()),
        )
        .await
        .unwrap();

    assert!(!path.exists());
    assert_eq!(
        std::fs::read(bin_under_test(&path).join("dupe.jar")).unwrap(),
        b"dupe"
    );
    assert!(!row_exists(&app, &row.id).await);
}

/// A selection is one step: every addon in it is moved, each has its own
/// result in the order asked, one asked for twice is deleted once, and one
/// that does not exist does not stop the others.
#[tokio::test]
async fn a_selection_is_deleted_in_one_step_with_a_result_for_each_addon() {
    let app = crate::setup_managers_for_test().await;
    let (id, first, first_path) =
        addon_fixture(&app, "selection", AddonType::Mods, "a.jar", b"a").await;
    let (second, second_path) = another_mod(&app, id, &first, &first_path, "b.jar").await;
    let (kept, kept_path) = another_mod(&app, id, &first, &first_path, "kept.jar").await;

    let results = app
        .instance_manager()
        .delete_mods(
            id,
            vec![
                first.id.clone(),
                "no-such-addon".into(),
                second.id.clone(),
                first.id.clone(),
            ],
        )
        .await
        .unwrap();

    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, [&first.id, "no-such-addon", &second.id]);
    assert!(matches!(results[0].1, Ok(None)), "{:?}", results[0].1);
    assert!(results[1].1.is_err());
    assert!(matches!(results[2].1, Ok(None)), "{:?}", results[2].1);
    let bin = bin_under_test(&first_path);
    assert_eq!(std::fs::read(bin.join("a.jar")).unwrap(), b"a");
    assert_eq!(std::fs::read(bin.join("b.jar")).unwrap(), b"b.jar");
    assert!(!first_path.exists() && !second_path.exists());
    assert!(!row_exists(&app, &first.id).await);
    assert!(!row_exists(&app, &second.id).await);
    // What was not selected is untouched.
    assert!(kept_path.exists());
    assert!(row_exists(&app, &kept.id).await);
}

/// When the Recycle Bin refuses a selection, each addon says so and all of
/// them stay, rows included.
#[tokio::test]
async fn a_selection_the_recycle_bin_refuses_stays_whole() {
    let app = crate::setup_managers_for_test().await;
    let (id, first, first_path) =
        addon_fixture(&app, "refused-selection", AddonType::Mods, "a.jar", b"a").await;
    let (second, second_path) = another_mod(&app, id, &first, &first_path, "b.jar").await;
    std::fs::write(bin_under_test(&first_path), b"").unwrap();

    let results = app
        .instance_manager()
        .delete_mods(id, vec![first.id.clone(), second.id.clone()])
        .await
        .unwrap();

    for (name, (_, result)) in ["a.jar", "b.jar"].into_iter().zip(&results) {
        let error = format!("{:#}", result.as_ref().unwrap_err());
        assert!(error.contains("Nothing was deleted"), "{error}");
        assert!(error.contains(name), "{error}");
    }
    assert!(first_path.exists() && second_path.exists());
    assert!(row_exists(&app, &first.id).await);
    assert!(row_exists(&app, &second.id).await);
}

/// With the setting off a selection is erased the same way, one result each.
#[tokio::test]
async fn a_selection_is_erased_with_the_recycle_bin_off() {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await;
    let (id, first, first_path) =
        addon_fixture(&app, "erased-selection", AddonType::Mods, "a.jar", b"a").await;
    let (second, second_path) = another_mod(&app, id, &first, &first_path, "b.jar").await;

    let results = app
        .instance_manager()
        .delete_mods(id, vec![first.id.clone(), second.id.clone()])
        .await
        .unwrap();

    assert!(results.iter().all(|(_, result)| result.is_ok()));
    assert!(!first_path.exists() && !second_path.exists());
    assert!(!bin_under_test(&first_path).exists());
    assert!(!row_exists(&app, &first.id).await);
    assert!(!row_exists(&app, &second.id).await);
}
