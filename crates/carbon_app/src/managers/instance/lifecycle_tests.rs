//! Deterministic ownership/configuration regressions. Hooks only suspend real
//! operations; all publication uses the production filesystem and database.
use super::*;
use std::sync::Mutex as StdMutex;
use tokio::sync::Notify;

#[derive(Debug, Default)]
pub(super) struct Hooks {
    pauses: StdMutex<HashMap<(&'static str, InstanceId), Arc<Pause>>>,
    path_pauses: StdMutex<HashMap<(&'static str, String), Arc<Pause>>>,
    drop_barriers: StdMutex<HashMap<InstanceId, Arc<std::sync::Barrier>>>,
}

#[derive(Debug, Default)]
pub(super) struct Pause {
    pub(super) entered: Notify,
    pub(super) release: Notify,
}

impl Hooks {
    fn arm_path(&self, name: &'static str, path: &str) -> Arc<Pause> {
        let pause = Arc::new(Pause::default());
        self.path_pauses
            .lock()
            .unwrap()
            .insert((name, path.into()), pause.clone());
        pause
    }

    pub(super) async fn pause_path(&self, name: &'static str, path: &str) {
        let pause = self
            .path_pauses
            .lock()
            .unwrap()
            .remove(&(name, path.into()));
        if let Some(pause) = pause {
            pause.entered.notify_one();
            pause.release.notified().await;
        }
    }

    pub(super) fn drop_barrier(&self, id: InstanceId) -> Option<Arc<std::sync::Barrier>> {
        self.drop_barriers.lock().unwrap().get(&id).cloned()
    }

    pub(super) fn arm(&self, name: &'static str, id: InstanceId) -> Arc<Pause> {
        let pause = Arc::new(Pause::default());
        self.pauses
            .lock()
            .unwrap()
            .insert((name, id), pause.clone());
        pause
    }

    pub(super) async fn pause(&self, name: &'static str, id: InstanceId) {
        let pause = self.pauses.lock().unwrap().remove(&(name, id));
        if let Some(pause) = pause {
            pause.entered.notify_one();
            pause.release.notified().await;
        }
    }
}

async fn fixture(app: &crate::App, name: &str) -> anyhow::Result<(InstanceId, PathBuf)> {
    let group = app.instance_manager().get_default_group().await?;
    let id = app
        .instance_manager()
        .create_instance(
            group,
            name.into(),
            false,
            InstanceVersionSource::Version(GameVersion::Standard(info::StandardVersion {
                release: "1.7.10".into(),
                modloaders: HashSet::new(),
            })),
            String::new(),
        )
        .await?;
    let row = instance_repo::get_instance(&app.db, id.0).await?.unwrap();
    let root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path()
        .join(row.shortpath);
    Ok((id, root))
}

fn update(id: InstanceId) -> domain::InstanceSettingsUpdate {
    domain::InstanceSettingsUpdate {
        instance_id: id,
        name: None,
        use_loaded_icon: None,
        notes: None,
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

async fn disk_config(root: &Path) -> anyhow::Result<info::Instance> {
    Ok(schema::parse_instance_config(
        &tokio::fs::read_to_string(root.join("instance.json")).await?,
    )?)
}

#[tokio::test]
async fn lifecycle_gpt0001_delete_excludes_rename_and_preserves_replacement() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    app.settings_manager()
        .set(
            carbon_repos::repos::app_configuration::AppConfigurationPatch {
                deletion_through_recycle_bin: Some(false),
                ..Default::default()
            },
        )
        .await?;
    let (id, root) = fixture(&app, "selected").await?;
    tokio::fs::write(root.join("save"), b"original").await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("delete", id);
    let delete = manager._delete_instance(id);
    tokio::pin!(delete);
    tokio::select! {
        _ = pause.entered.notified() => {},
        result = &mut delete => panic!("deletion did not pause: {result:?}"),
    }
    let mut edit = update(id);
    edit.name = Some("renamed".into());
    let rename = manager.update_instance(edit);
    tokio::pin!(rename);
    // A pending rename is queued behind deletion; an immediate rejection is
    // also safe. Success here would release the selected path for reuse.
    let early = tokio::time::timeout(Duration::from_millis(150), &mut rename).await;
    assert!(
        !matches!(early, Ok(Ok(_))),
        "rename succeeded while deletion owns selected root"
    );
    pause.release.notify_one();
    delete.await?;
    if early.is_err() {
        assert!(rename.await.is_err());
    }
    assert!(!root.exists());
    assert!(instance_repo::get_instance(&app.db, id.0).await?.is_none());
    assert!(!manager.instances.read().await.contains_key(&id));
    let (replacement, replacement_root) = fixture(&app, "selected").await?;
    tokio::fs::write(replacement_root.join("save"), b"replacement").await?;
    manager._delete_instance(id).await?;
    assert_eq!(
        tokio::fs::read(replacement_root.join("save")).await?,
        b"replacement"
    );
    assert!(
        instance_repo::get_instance(&app.db, replacement.0)
            .await?
            .is_some()
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0015_cached_invalid_instances_remain_visible() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (missing, missing_root) = fixture(&app, "missing").await?;
    let (broken, broken_root) = fixture(&app, "broken").await?;
    let (valid, _) = fixture(&app, "valid").await?;
    let group = instance_repo::get_instance(&app.db, missing.0)
        .await?
        .unwrap()
        .group_id;
    tokio::fs::remove_file(missing_root.join("instance.json")).await?;
    tokio::fs::write(broken_root.join("instance.json"), "{").await?;
    let parent = missing_root.parent().unwrap();
    tokio::fs::create_dir(parent.join("uncached-missing")).await?;
    tokio::fs::create_dir(parent.join("uncached-broken")).await?;
    tokio::fs::write(parent.join("uncached-broken/instance.json"), "{").await?;
    app.instance_manager().instances.write().await.clear();
    app.instance_manager().scan_instances().await?;
    let groups = app.instance_manager().list_groups().await?;
    let group = groups.iter().find(|g| g.id.0 == group).unwrap();
    let missing = group
        .instances
        .iter()
        .find(|i| i.id == missing)
        .expect("cached missing tile disappeared");
    assert_eq!(missing.name, "missing");
    assert!(matches!(
        missing.status,
        ListInstanceStatus::Invalid(InvalidListInstance::JsonMissing)
    ));
    let broken = group
        .instances
        .iter()
        .find(|i| i.id == broken)
        .expect("cached invalid tile disappeared");
    assert_eq!(broken.name, "broken");
    assert!(matches!(
        broken.status,
        ListInstanceStatus::Invalid(InvalidListInstance::JsonError(_))
    ));
    assert!(group.instances.iter().any(|i| i.id == valid));
    assert_eq!(instance_repo::get_all_instances(&app.db).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0016_details_releases_map_before_metadata() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, _) = fixture(&app, "details").await?;
    let (other, _) = fixture(&app, "other").await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("details", id);
    let details = manager.instance_details(id);
    tokio::pin!(details);
    tokio::select! {
        _ = pause.entered.notified() => {},
        _ = &mut details => panic!("details did not pause"),
    }
    tokio::time::timeout(Duration::from_secs(1), manager.set_favorite(other, true))
        .await
        .expect("metadata wait blocked another instance's writer")?;
    assert!(
        tokio::time::timeout(Duration::from_secs(1), manager.instances.read())
            .await
            .is_ok()
    );
    pause.release.notify_one();
    assert_eq!(details.await?.name, "details");
    manager
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .config
        .game_configuration
        .version = None;
    let unversioned = manager.instance_details(id).await?;
    assert!(unversioned.version.is_none());
    assert!(unversioned.required_java_profile.is_none());
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0055_settings_preserves_overlapping_playtime() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "playtime").await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("settings", id);
    let mut edit = update(id);
    edit.notes = Some("edited note".into());
    let settings = manager.update_instance(edit);
    tokio::pin!(settings);
    tokio::select! {
        _ = pause.entered.notified() => {},
        _ = &mut settings => panic!("settings did not pause"),
    }
    let bank = manager.update_playtime(id, 60);
    tokio::pin!(bank);
    let early = tokio::time::timeout(Duration::from_millis(150), &mut bank).await;
    if let Ok(result) = &early {
        assert!(result.is_ok());
    }
    pause.release.notify_one();
    settings.await?;
    if early.is_err() {
        bank.await?;
    }
    let persisted = disk_config(&root).await?;
    assert_eq!(
        persisted.seconds_played, 60,
        "settings overwrote banked playtime"
    );
    assert_eq!(persisted.notes, "edited note");
    assert!(persisted.last_played.is_some());
    let instances = manager.instances.read().await;
    let config = &instances[&id].data()?.config;
    assert_eq!(config.seconds_played, persisted.seconds_played);
    assert_eq!(config.last_played, persisted.last_played);
    drop(instances);
    manager.update_instance(update(id)).await?;
    assert_eq!(disk_config(&root).await?.seconds_played, 60);
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0056_arrange_then_move_to_end_with_folders() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (a, _) = fixture(&app, "A").await?;
    let (b, _) = fixture(&app, "B").await?;
    let manager = app.instance_manager();
    manager.create_group("folder 1".into()).await?;
    manager.create_group("folder 2".into()).await?;
    manager.arrange_library(LibrarySortCriteria::Name).await?;
    let group = manager.get_default_group().await?;
    manager
        .move_instance(a, InstanceMoveTarget::EndOfGroup(group))
        .await?;
    let mut rows = instance_repo::get_instances_by_group(&app.db, group.0).await?;
    rows.sort_by_key(|r| r.library_position);
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![b.0, a.0]
    );
    // A legacy library may already contain sparse folder-offset indices.
    instance_repo::set_instance_index_and_library_position(&app.db, a.0, 2, Some(2)).await?;
    instance_repo::set_instance_index_and_library_position(&app.db, b.0, 3, Some(3)).await?;
    manager
        .move_instance(a, InstanceMoveTarget::EndOfGroup(group))
        .await?;
    let rows = instance_repo::get_instances_by_group_ordered_by_index(&app.db, group.0).await?;
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![b.0, a.0]
    );
    Ok(())
}

#[test]
fn lifecycle_gpt0057_long_components_publish_with_graphemes_and_suffix_room() -> anyhow::Result<()>
{
    let parent = tempfile::tempdir()?;
    for name in [
        "a".repeat(300),
        "界".repeat(100),
        "e\u{301}".repeat(100),
        "Normal Name".into(),
    ] {
        let component = truncate_name(&name, parent.path());
        assert!(!component.is_empty());
        assert!(name.starts_with(&component));
        let suffixed = format!("{component}999");
        std::fs::create_dir(parent.path().join(&suffixed))?;
        assert!(path_length(Path::new(&suffixed)) <= 255);
        assert!(
            name.grapheme_indices(true)
                .any(|(i, _)| i == component.len())
                || component == name
        );
    }
    assert!(truncate_name("x", Path::new(&"p".repeat(MAX_PATH + 1))).is_empty());
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0154_display_rename_to_current_component_updates_library()
-> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, _) = fixture(&app, "benign-source").await?;
    let manager = app.instance_manager();
    let copy = manager.duplicate_instance(id, "benign-copy".into()).await?;
    let row = instance_repo::get_instance(&app.db, copy.0).await?.unwrap();
    let mut edit = update(copy);
    edit.name = Some(row.shortpath.clone());
    manager.update_instance(edit).await?;
    let updated = instance_repo::get_instance(&app.db, copy.0).await?.unwrap();
    assert_eq!(updated.name, row.shortpath);
    assert_eq!(updated.shortpath, row.shortpath);
    let root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path()
        .join(&row.shortpath);
    assert_eq!(disk_config(&root).await?.name, row.shortpath);
    assert_eq!(
        manager.instances.read().await[&copy].data()?.config.name,
        row.shortpath
    );
    let mut edit = update(copy);
    edit.name = Some("distinct".into());
    manager.update_instance(edit).await?;
    assert_eq!(
        instance_repo::get_instance(&app.db, copy.0)
            .await?
            .unwrap()
            .name,
        "distinct"
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0222_duplicate_aborts_before_publishing_unremovable_marker()
-> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "source").await?;
    // A directory is a deterministic unlink error, including when tests run as root.
    tokio::fs::create_dir(root.join(PID_FILE_NAME)).await?;
    tokio::fs::write(root.join(PID_FILE_NAME).join("sentinel"), b"source").await?;
    let result = app
        .instance_manager()
        .duplicate_instance(id, "copy".into())
        .await;
    assert!(
        result.is_err(),
        "duplicate published a root whose PID marker could not be removed"
    );
    assert!(!root.with_file_name("source1").exists());
    assert_eq!(instance_repo::get_all_instances(&app.db).await?.len(), 1);
    assert_eq!(
        tokio::fs::read(root.join(PID_FILE_NAME).join("sentinel")).await?,
        b"source"
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0055_cached_scan_cannot_overwrite_bank() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "scan-bank").await?;
    // Force canonicalization so this scan is both a disk and a map writer.
    let json = tokio::fs::read_to_string(root.join("instance.json")).await?;
    tokio::fs::write(root.join("instance.json"), format!("{json}\n")).await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("scan", id);
    let scan = manager.scan_instances();
    tokio::pin!(scan);
    tokio::select! {
        _ = pause.entered.notified() => {},
        _ = &mut scan => panic!("scan did not pause"),
    }
    let bank = manager.update_playtime(id, 60);
    tokio::pin!(bank);
    let early = tokio::time::timeout(Duration::from_millis(150), &mut bank).await;
    if let Ok(result) = &early {
        assert!(result.is_ok());
    }
    pause.release.notify_one();
    scan.await?;
    if early.is_err() {
        bank.await?;
    }
    assert_eq!(disk_config(&root).await?.seconds_played, 60);
    assert_eq!(
        manager.instances.read().await[&id]
            .data()?
            .config
            .seconds_played,
        60
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0055_failed_playtime_write_does_not_publish_memory() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "failed-bank").await?;
    tokio::fs::remove_file(root.join("instance.json")).await?;
    tokio::fs::create_dir(root.join("instance.json")).await?;
    assert!(
        app.instance_manager()
            .update_playtime(id, 60)
            .await
            .is_err()
    );
    let manager = app.instance_manager();
    let instances = manager.instances.read().await;
    assert_eq!(instances[&id].data()?.config.seconds_played, 0);
    assert!(instances[&id].data()?.config.last_played.is_none());
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0055_modpack_publication_preserves_current_edits() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "pack-publication").await?;
    let manager = app.instance_manager();
    let mut prepared = disk_config(&root).await?;
    let original_modpack = prepared.modpack.clone();
    prepared.game_configuration.version = Some(GameVersion::Standard(info::StandardVersion {
        release: "1.20.1".into(),
        modloaders: HashSet::new(),
    }));
    manager.update_playtime(id, 60).await?;
    let mut edit = update(id);
    edit.notes = Some("keep my notes".into());
    manager.update_instance(edit).await?;
    manager
        .publish_modpack_config(id, prepared, original_modpack)
        .await?;
    let result = disk_config(&root).await?;
    assert_eq!(
        result.seconds_played, 60,
        "pack publication overwrote banked playtime"
    );
    assert_eq!(result.notes, "keep my notes");
    assert!(result.last_played.is_some());
    assert!(
        matches!(result.game_configuration.version, Some(GameVersion::Standard(v)) if v.release == "1.20.1")
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_gate_preserves_waiters_and_retires_idle_entries() {
    let manager = InstanceManager::new();
    let id = InstanceId(123);
    let owner = manager.lock_instance(id).await;
    let mut waiter = Box::pin(manager.lock_instance(id));
    assert!(futures::poll!(&mut waiter).is_pending());
    let mut later = Box::pin(manager.lock_instance(id));
    assert!(futures::poll!(&mut later).is_pending());
    drop(owner);
    let waiter = waiter.await;
    assert!(
        futures::poll!(&mut later).is_pending(),
        "gate split while a waiter owned it"
    );
    drop(waiter);
    drop(later.await);
    assert!(manager.instance_op_locks.is_empty());
    let owner = manager.lock_instance(id).await;
    let mut cancelled = Box::pin(manager.lock_instance(id));
    assert!(futures::poll!(&mut cancelled).is_pending());
    drop(cancelled);
    drop(owner);
    assert!(
        manager.instance_op_locks.is_empty(),
        "cancelled waiter leaked a gate"
    );
}

#[tokio::test]
async fn lifecycle_controls_in_use_and_failed_delete_keep_identity() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    app.settings_manager()
        .set(
            carbon_repos::repos::app_configuration::AppConfigurationPatch {
                deletion_through_recycle_bin: Some(false),
                ..Default::default()
            },
        )
        .await?;
    let (id, root) = fixture(&app, "delete-controls").await?;
    let manager = app.instance_manager();
    for state in [
        LaunchState::Queued(VisualTaskId(1)),
        LaunchState::Preparing(VisualTaskId(2)),
        LaunchState::Running(RunningInstance::adopted(
            u32::MAX,
            1,
            Utc::now(),
            Utc::now(),
        )),
    ] {
        manager
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()?
            .state = state;
        assert!(manager._delete_instance(id).await.is_err());
        assert!(root.exists());
        assert!(instance_repo::get_instance(&app.db, id.0).await?.is_some());
    }
    manager
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Deleting;
    assert!(manager.update_instance(update(id)).await.is_err());
    assert!(manager.update_playtime(id, 60).await.is_err());
    manager._delete_instance(id).await?;
    assert!(root.exists());
    manager
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Inactive { failed_task: None };
    let saved = root.with_extension("saved");
    tokio::fs::rename(&root, &saved).await?;
    assert!(manager._delete_instance(id).await.is_err());
    assert!(instance_repo::get_instance(&app.db, id.0).await?.is_some());
    assert!(manager.instance_op_locks.is_empty());
    tokio::fs::rename(&saved, &root).await?;
    // Mirrors the existing caller's failure recovery before a retry.
    manager
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Inactive { failed_task: None };
    manager._delete_instance(id).await?;
    assert!(!root.exists());
    Ok(())
}

#[tokio::test]
async fn lifecycle_controls_modpack_preserves_lock_detach_and_renamed_root() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "pack-controls").await?;
    let manager = app.instance_manager();
    let mut config = disk_config(&root).await?;
    config.modpack = Some(info::ModpackInfo {
        modpack: Modpack::Curseforge(info::CurseforgeModpack {
            project_id: 1,
            file_id: 2,
        }),
        locked: true,
    });
    {
        let _gate = manager.lock_instance(id).await;
        manager
            .persist_config(id, "pack-controls", config.clone())
            .await?;
    }
    let original = config.modpack.clone();
    config.modpack.as_mut().unwrap().modpack = Modpack::Curseforge(info::CurseforgeModpack {
        project_id: 1,
        file_id: 3,
    });
    config.game_configuration.version = Some(GameVersion::Standard(info::StandardVersion {
        release: "1.20.1".into(),
        modloaders: HashSet::new(),
    }));
    let mut edit = update(id);
    edit.modpack_locked = Some(Some(false));
    edit.name = Some("pack-renamed".into());
    manager.update_instance(edit).await?;
    manager
        .publish_modpack_config(id, config.clone(), original)
        .await?;
    assert!(!root.exists());
    let renamed = root.with_file_name("pack-renamed");
    let result = disk_config(&renamed).await?;
    assert!(!result.modpack.as_ref().unwrap().locked);
    assert_eq!(
        result.modpack.as_ref().unwrap().modpack,
        config.modpack.as_ref().unwrap().modpack
    );
    assert_eq!(result.name, "pack-renamed");
    let original = result.modpack.clone();
    let mut edit = update(id);
    edit.modpack_locked = Some(None);
    manager.update_instance(edit).await?;
    manager.publish_modpack_config(id, config, original).await?;
    assert!(
        disk_config(&renamed).await?.modpack.is_none(),
        "preparation reattached a user-detached pack"
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_controls_sparse_negative_end_and_same_name_edit() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (a, root) = fixture(&app, "A").await?;
    let (b, _) = fixture(&app, "B").await?;
    let manager = app.instance_manager();
    let group = manager.get_default_group().await?;
    for (a_index, b_index) in [(-10, -2), (100, 500)] {
        instance_repo::set_instance_index_and_library_position(&app.db, a.0, a_index, Some(0))
            .await?;
        instance_repo::set_instance_index_and_library_position(&app.db, b.0, b_index, Some(1))
            .await?;
        manager
            .move_instance(a, InstanceMoveTarget::EndOfGroup(group))
            .await?;
        let rows = instance_repo::get_instances_by_group_ordered_by_index(&app.db, group.0).await?;
        assert_eq!(
            rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![b.0, a.0]
        );
    }
    let mut edit = update(a);
    edit.name = Some("A".into());
    manager.update_instance(edit).await?;
    assert!(root.exists());
    assert!(!root.with_file_name("A1").exists());
    Ok(())
}

#[tokio::test]
async fn lifecycle_controls_next_folder_handles_missing_parent_and_collisions() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let parent = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path();
    if parent.exists() {
        tokio::fs::remove_dir(&parent).await?;
    }
    let name = "界".repeat(100);
    let (component, path) = manager.next_folder(&name)?;
    assert!(path_length(Path::new(&component)) <= 252);
    tokio::fs::create_dir_all(&path).await?;
    let (second, second_path) = manager.next_folder(&name)?;
    assert_eq!(second, format!("{component}1"));
    tokio::fs::create_dir(&second_path).await?;
    assert!(manager.next_folder("   ").is_err());
    Ok(())
}

#[tokio::test]
async fn lifecycle_controls_duplicate_marker_absent_and_removed() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "pid-controls").await?;
    let manager = app.instance_manager();
    for marker in [None, Some((4242, 1700000000))] {
        if let Some((pid, started)) = marker {
            orphan_pid::write_pid_file(&root, PID_FILE_NAME, pid, started).await;
        }
        let duplicate = manager.duplicate_instance(id, "copy".into()).await?;
        let row = instance_repo::get_instance(&app.db, duplicate.0)
            .await?
            .unwrap();
        let copy_root = root.with_file_name(row.shortpath);
        assert!(!copy_root.join(PID_FILE_NAME).exists());
    }
    assert_eq!(
        orphan_pid::read_pid_file(&root, PID_FILE_NAME).await?,
        Some((4242, Some(1700000000)))
    );
    Ok(())
}

#[tokio::test]
async fn lifecycle_gpt0015_invalid_rescan_preserves_active_launch_ownership() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "active-rescan").await?;
    let manager = app.instance_manager();
    manager
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Preparing(VisualTaskId(17));
    tokio::fs::remove_file(root.join("instance.json")).await?;
    manager.scan_instances().await?;
    assert!(
        matches!(
            manager.instances.read().await[&id].data().map(|d| &d.state),
            Ok(LaunchState::Preparing(VisualTaskId(17)))
        ),
        "invalid rescan discarded live launch ownership"
    );
    // This is state only: no PID marker, process lookup, watcher, or signal.
    manager
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Running(RunningInstance::adopted(
        u32::MAX,
        1,
        Utc::now(),
        Utc::now(),
    ));
    manager.scan_instances().await?;
    assert!(
        matches!(manager.instances.read().await[&id].data().map(|d| &d.state),
        Ok(LaunchState::Running(running)) if running.is_adopted())
    );
    Ok(())
}

async fn round1_scan_identity_after_initial_snapshot(rename: bool) -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let parent = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path();
    tokio::fs::create_dir_all(&parent).await?;
    let original = if rename {
        Some(fixture(&app, "before-rename").await?.0)
    } else {
        None
    };
    let admission = manager.test_hooks.arm("scan_admission", InstanceId(0));
    let scan = manager.scan_instances();
    tokio::pin!(scan);
    tokio::select! {
        _ = admission.entered.notified() => {},
        result = &mut scan => panic!("scan did not pause before directory admission: {result:?}"),
    }
    let id = if let Some(id) = original {
        let mut edit = update(id);
        edit.name = Some("round1-current".into());
        manager.update_instance(edit).await?;
        id
    } else {
        fixture(&app, "round1-current").await?.0
    };
    let root = parent.join("round1-current");
    let json = tokio::fs::read_to_string(root.join("instance.json")).await?;
    tokio::fs::write(root.join("instance.json"), format!("{json}\n")).await?;
    let read = manager.test_hooks.arm_path("scan_read", "round1-current");
    admission.release.notify_one();
    tokio::select! {
        _ = read.entered.notified() => {},
        result = &mut scan => panic!("scan did not pause after config read: {result:?}"),
    }
    let writes = async {
        manager.update_playtime(id, 60).await?;
        let mut edit = update(id);
        edit.notes = Some("current settings".into());
        manager.update_instance(edit).await?;
        anyhow::Ok(())
    };
    tokio::pin!(writes);
    let early = tokio::time::timeout(Duration::from_millis(150), &mut writes).await;
    if let Ok(result) = &early {
        assert!(result.is_ok());
    }
    read.release.notify_one();
    scan.await?;
    if early.is_err() {
        writes.await?;
    }
    let rows = instance_repo::get_all_instances(&app.db).await?;
    let config = disk_config(&root).await?;
    let instances = manager.instances.read().await;
    let mut mapped = instances.keys().map(|id| id.0).collect::<Vec<_>>();
    mapped.sort();
    assert_eq!(
        (
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            mapped,
            config.seconds_played,
            config.notes.as_str()
        ),
        (vec![id.0], vec![id.0], 60, "current settings"),
        "scan replaced a newly admitted identity or overwrote its completed writers",
    );
    assert_eq!(instances[&id].data()?.config.seconds_played, 60);
    assert_eq!(instances[&id].data()?.config.notes, "current settings");
    assert_eq!(
        instances[&id].data()?.config.last_played,
        config.last_played
    );
    Ok(())
}

#[tokio::test]
async fn round1_r1_scan_admits_created_identity_after_initial_snapshot() -> anyhow::Result<()> {
    round1_scan_identity_after_initial_snapshot(false).await
}

#[tokio::test]
async fn round1_r1_scan_admits_renamed_identity_after_initial_snapshot() -> anyhow::Result<()> {
    round1_scan_identity_after_initial_snapshot(true).await
}

#[tokio::test]
async fn round1_r1_scan_waits_for_creation_registration() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let publication = manager.test_hooks.arm("create_publish", InstanceId(0));
    let create = fixture(&app, "publication");
    tokio::pin!(create);
    tokio::select! {
        _ = publication.entered.notified() => {},
        result = &mut create => panic!("create did not pause before registration: {result:?}"),
    }
    let scan = manager.scan_instances();
    tokio::pin!(scan);
    let early = tokio::time::timeout(Duration::from_millis(150), &mut scan).await;
    if let Ok(result) = &early {
        assert!(result.is_ok());
    }
    publication.release.notify_one();
    let (id, root) = create.await?;
    if early.is_err() {
        scan.await?;
    }
    let row = instance_repo::get_instance_by_shortpath(&app.db, "publication")
        .await?
        .unwrap();
    assert_eq!(row.id, id.0);
    let instances = manager.instances.read().await;
    assert_eq!(
        instances.len(),
        1,
        "scanner admitted a second ID before creation registered its root"
    );
    assert!(instances.contains_key(&id));
    assert_eq!(disk_config(&root).await?.name, "publication");
    Ok(())
}

#[tokio::test]
async fn round1_r2_concurrent_owner_and_cancelled_waiter_retire_gate() {
    let manager = InstanceManager::new();
    let id = InstanceId(321);
    // Both destructors must finish their retirement decision before either
    // wrapper's fields are automatically destroyed. This forces the reviewed
    // last-departure interleaving without relying on thread scheduling luck.
    manager
        .test_hooks
        .drop_barriers
        .lock()
        .unwrap()
        .insert(id, Arc::new(std::sync::Barrier::new(2)));
    let owner = manager.lock_instance(id).await;
    let mut cancelled = Box::pin(manager.lock_instance(id));
    assert!(futures::poll!(&mut cancelled).is_pending());
    std::thread::scope(|threads| {
        threads.spawn(move || drop(owner));
        threads.spawn(move || drop(cancelled));
    });
    assert!(
        manager.instance_op_locks.is_empty(),
        "concurrent last departures left an idle gate registered"
    );
}

#[tokio::test]
async fn round1_r1_create_and_duplicate_preserve_db_only_path_reservations() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let group = manager.get_default_group().await?;
    let reservation = manager
        .add_instance("recoverable".into(), "reserved".into(), group)
        .await?;
    let (created, root) = fixture(&app, "reserved").await?;
    assert_eq!(root.file_name().unwrap(), "reserved1");
    assert_eq!(
        instance_repo::get_instance(&app.db, reservation.0)
            .await?
            .unwrap()
            .name,
        "recoverable"
    );
    let copy_reservation = manager
        .add_instance("recoverable-copy".into(), "reserved11".into(), group)
        .await?;
    let copy = manager.duplicate_instance(created, "copy".into()).await?;
    assert_eq!(
        instance_repo::get_instance(&app.db, copy.0)
            .await?
            .unwrap()
            .shortpath,
        "reserved12"
    );
    assert_eq!(
        instance_repo::get_instance(&app.db, copy_reservation.0)
            .await?
            .unwrap()
            .name,
        "recoverable-copy"
    );
    assert!(!root.with_file_name("reserved").exists());
    assert!(!root.with_file_name("reserved11").exists());
    Ok(())
}

#[tokio::test]
async fn round1_r1_genuinely_uncached_root_registers_once() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (_, source) = fixture(&app, "seed").await?;
    let mut config = disk_config(&source).await?;
    config.name = "imported".into();
    let imported = source.with_file_name("imported");
    tokio::fs::create_dir(&imported).await?;
    tokio::fs::write(
        imported.join("instance.json"),
        schema::make_instance_config(config)?,
    )
    .await?;
    let manager = app.instance_manager();
    let (one, two) = tokio::join!(manager.scan_instances(), manager.scan_instances());
    one?;
    two?;
    let row = instance_repo::get_instance_by_shortpath(&app.db, "imported")
        .await?
        .unwrap();
    let instances = manager.instances.read().await;
    assert_eq!(instances.len(), 2);
    assert_eq!(
        instances[&InstanceId(row.id)].data()?.config.name,
        "imported"
    );
    assert_eq!(instance_repo::get_all_instances(&app.db).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn task04_runtime_prepare_admission_serializes_with_relocation() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let id = InstanceId(-404);
    let pause = app.instance_manager.test_hooks.arm("runtime_admitted", id);
    let admitted = app.instance_manager().prepare_game(id, None, None, false);
    tokio::pin!(admitted);
    tokio::select! {
        _ = &mut admitted => panic!("prepare should be paused before taking instance locks"),
        _ = pause.entered.notified() => {},
    }
    assert!(app.runtime_lifecycle.begin_relocation("busy").is_err());
    pause.release.notify_one();
    assert!(admitted.await.is_err()); // absent synthetic instance, no child or download
    app.runtime_lifecycle.begin_relocation("move")?;
    let error = app
        .instance_manager()
        .prepare_game(id, None, None, false)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("relocation"));
    app.runtime_lifecycle.cancel_relocation("move")?;
    Ok(())
}

#[tokio::test]
async fn task04_runtime_server_entry_points_obey_same_freeze() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    app.runtime_lifecycle.begin_relocation("move")?;
    let manager = app.server_manager();
    let id = crate::domain::server::ServerId(-404);
    let error = manager.start_server(id).await.unwrap_err();
    assert!(error.to_string().contains("relocation"));
    let error = manager.reinstall_server_from_modpack(id).await.unwrap_err();
    assert!(error.to_string().contains("relocation"));
    let error = manager
        .create_server(
            crate::domain::server::ServerGroupId(-404),
            "fixture".into(),
            "1.20".into(),
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("relocation"));
    app.runtime_lifecycle.cancel_relocation("move")?;
    Ok(())
}

/// A legacy marker whose PID a process other than Java holds now names a
/// reused PID, as the instance scan and 2.0.40 both concluded: it is not a
/// relocation owner in this session or, through a kept copy, after a
/// restart. It used to refuse every move for as long as that unrelated
/// process ran.
#[tokio::test]
async fn task04_runtime_reconciliation_retires_a_legacy_marker_on_a_reused_pid_across_restart()
-> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (_, root) = fixture(&app, "legacy-owner").await?;
    // The test runner is an existing benign non-Java process; never signal it.
    let pid = std::process::id();
    tokio::fs::write(root.join(PID_FILE_NAME), pid.to_string()).await?;
    app.instance_manager().scan_instances().await?;
    assert!(
        !app.runtime_lifecycle
            .relocation_blockers("one")
            .iter()
            .any(|blocker| blocker.pid == Some(pid)),
        "{:?}",
        app.runtime_lifecycle.relocation_blockers("one")
    );
    let restarted = crate::managers::runtime_lifecycle::RuntimeLifecycle::with_runtime_path(
        &app.settings_manager().runtime_path.to_path_buf(),
    );
    restarted.reconcile_children().await?;
    restarted
        .reconcile_root(
            &app.settings_manager()
                .runtime_path
                .get_instances()
                .to_path(),
            PID_FILE_NAME,
        )
        .await?;
    restarted.reconciliation_finished(Ok(()));
    restarted
        .begin_relocation("two")
        .map_err(|refused| anyhow::anyhow!("{refused}"))?;
    restarted.cancel_relocation("two")?;
    Ok(())
}

#[tokio::test]
async fn task04_runtime_reconciliation_preserves_corrupt_pid_evidence() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (_, root) = fixture(&app, "corrupt-owner").await?;
    tokio::fs::write(root.join(PID_FILE_NAME), "malformed ownership").await?;
    app.instance_manager().scan_instances().await?;
    assert!(app.runtime_lifecycle.begin_relocation("one").is_err());
    assert_eq!(
        tokio::fs::read_to_string(root.join(PID_FILE_NAME)).await?,
        "malformed ownership",
        "a best-effort UI scan must preserve strict reconciliation failure evidence for restart"
    );
    Ok(())
}

fn task08_archive(root: &Path, files: &[(&str, &[u8])]) -> anyhow::Result<()> {
    use std::io::Write;
    // Explicit older producer fixture: no current authoritative session.
    let _ = std::fs::remove_file(root.join(".setup/setup-session.json"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join(".setup/modrinth"))?);
    zip.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    zip.write_all(br#"{"formatVersion":1,"game":"minecraft","versionId":"synthetic-B","name":"fixture","files":[],"dependencies":{"minecraft":"1.20.1"}}"#)?;
    for (path, bytes) in files {
        zip.start_file(
            format!("overrides/{path}"),
            zip::write::SimpleFileOptions::default(),
        )?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;
    Ok(())
}

#[tokio::test]
async fn task08_legacy_archive_restores_consumed_complete_membership() -> anyhow::Result<()> {
    use super::modpack::packinfo;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "legacy-consumed").await?;
    task08_archive(&root, &[("unchanged.txt", b"same"), ("new.txt", b"new")])?;
    tokio::fs::write(root.join("instance/unchanged.txt"), b"same").await?;
    let old = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(old)?).await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance")).await?;
    tokio::fs::write(root.join(".setup/staging/instance/new.txt"), b"new").await?;
    tokio::fs::create_dir(root.join(".setup/modpack-skip-overrides")).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let shortpath = root.file_name().unwrap().to_str().unwrap().to_owned();
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    assert_eq!(
        tokio::fs::read(root.join("instance/unchanged.txt")).await?,
        b"same"
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/new.txt")).await?,
        b"new"
    );
    let target =
        packinfo::parse_packinfo(&tokio::fs::read_to_string(root.join("packinfo.json")).await?)?;
    assert_eq!(target.files.len(), 2);
    Ok(())
}

#[tokio::test]
async fn task08_preparation_preserves_installed_version_until_apply() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "pending-metadata").await?;
    task08_archive(&root, &[("new.txt", b"new")])?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let shortpath = root.file_name().unwrap().to_str().unwrap().to_owned();
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (_, version, _, _) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut owner,
    )
    .await?;
    assert_eq!(version.unwrap().release, "1.20.1");
    assert!(
        matches!(disk_config(&root).await?.game_configuration.version, Some(GameVersion::Standard(v)) if v.release == "1.7.10")
    );
    assert!(!root.join("instance/new.txt").exists());
    Ok(())
}

#[tokio::test]
async fn task08_refused_repair_preserves_accepted_setup() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "overlap").await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    let (_, mut config) = app.instance_manager().current_config(id).await?;
    config.modpack = Some(info::ModpackInfo {
        modpack: info::Modpack::Modrinth(info::ModrinthModpack {
            project_id: "synthetic".into(),
            version_id: "A".into(),
        }),
        locked: true,
    });
    app.instance_manager()
        .persist_config(id, root.file_name().unwrap().to_str().unwrap(), config)
        .await?;
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let pause = manager.test_hooks.arm("repair_prechecked", id);
    let repair_app = app.clone();
    let repair = tokio::spawn(async move {
        repair_app
            .instance_manager()
            .repair_modpack(id, super::modpack::RepairMarkerFile::default(), None)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered.notified()).await?;
    let change_app = app.clone();
    let mut change = tokio::spawn(async move {
        change_app
            .instance_manager()
            .change_modpack(
                id,
                info::Modpack::Modrinth(info::ModrinthModpack {
                    project_id: "synthetic".into(),
                    version_id: "B".into(),
                }),
            )
            .await
    });
    let accepted = tokio::time::timeout(Duration::from_secs(2), &mut change).await;
    pause.release.notify_one();
    if let Ok(result) = accepted {
        result??;
        let marker = tokio::fs::read(root.join(".setup/change-pack-version.json")).await?;
        assert!(
            tokio::time::timeout(Duration::from_secs(5), repair)
                .await??
                .is_err()
        );
        assert_eq!(
            tokio::fs::read(root.join(".setup/change-pack-version.json")).await?,
            marker
        );
    } else {
        tokio::time::timeout(Duration::from_secs(5), repair).await???;
        assert!(
            tokio::time::timeout(Duration::from_secs(5), change)
                .await??
                .is_err()
        );
        assert!(root.join(".setup/repair").exists());
    }
    Ok(())
}

#[tokio::test]
async fn task08_queued_rename_refuses_before_config_write() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "queued-name").await?;
    app.instance_manager()
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = run::LaunchState::Queued(crate::domain::vtask::VisualTaskId(777));
    let before = tokio::fs::read(root.join("instance.json")).await?;
    let mut edit = update(id);
    edit.name = Some("moved-name".into());
    assert!(app.instance_manager().update_instance(edit).await.is_err());
    assert_eq!(tokio::fs::read(root.join("instance.json")).await?, before);
    Ok(())
}

#[tokio::test]
async fn task08_cancel_pending_offline_preserves_current_independent_edits() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "cancel-local").await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    tokio::fs::create_dir(root.join(".setup")).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let mut session = Session::new(&config, Producer::Change, Kind::Pack, None)?;
    session.instance_id = Some(id.0);
    session.create(&root).await?;
    tokio::fs::write(root.join(".setup/change-pack-version.json"), b"pending B").await?;
    tokio::fs::write(root.join("instance/kept.txt"), b"installed").await?;
    let mut edit = update(id);
    edit.notes = Some("current notes".into());
    app.instance_manager().update_instance(edit).await?;
    app.instance_manager()
        .cancel_pending_setup(id, session.operation)
        .await?;
    assert!(!root.join(".setup").exists());
    assert_eq!(disk_config(&root).await?.notes, "current notes");
    assert_eq!(
        tokio::fs::read(root.join("instance/kept.txt")).await?,
        b"installed"
    );
    assert!(
        matches!(disk_config(&root).await?.game_configuration.version, Some(GameVersion::Standard(v)) if v.release == "1.7.10")
    );
    Ok(())
}

#[tokio::test]
async fn task08_legacy_disabled_archive_recovers_with_owned_retained_original() -> anyhow::Result<()>
{
    use super::modpack::{packinfo, session::Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "legacy-disabled").await?;
    task08_archive(&root, &[("mods/a.jar.disabled", b"target")])?;
    tokio::fs::create_dir(root.join("instance/mods")).await?;
    tokio::fs::write(root.join("instance/mods/a.jar"), b"old").await?;
    let old = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(old)?).await?;
    tokio::fs::write(root.join("instance/mods/a.jar.disabled"), b"target").await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance")).await?;
    tokio::fs::create_dir(root.join(".setup/modpack-skip-overrides")).await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    assert!(!root.join("instance/mods/a.jar").exists());
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/a.jar.disabled")).await?,
        b"target"
    );
    let session = Session::load(&root).await?.unwrap();
    let mut records = tokio::fs::read_dir(root.join(".setup/representation-transitions")).await?;
    let record: serde_json::Value = serde_json::from_slice(
        &tokio::fs::read(records.next_entry().await?.unwrap().path()).await?,
    )?;
    assert_eq!(record["schema_version"], 2);
    assert_eq!(record["owner"]["operation"], session.operation);
    let batch = root
        .join(".repair-recovery")
        .join(record["recovery_batch"].as_str().unwrap());
    let mut entries = tokio::fs::read_dir(batch).await?;
    let entry = entries.next_entry().await?.unwrap().path();
    assert_eq!(tokio::fs::read(entry.join("payload")).await?, b"old");
    let manifest: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(entry.join("entry.json")).await?)?;
    assert_eq!(manifest["owner"], record["owner"]);
    Ok(())
}

/// A long instance folder name ('Vault Hunters 3rd Edition' under a short
/// Windows user name is already 89 characters) puts the representation
/// record, its recovery manifest and deep pack files past 260 characters.
/// Windows refuses those through plain Win32 paths, which left the setup
/// stuck in Applying; a full owned update must reach Applied and commit.
#[tokio::test]
async fn long_instance_root_owned_update_with_disabled_replace_commits() -> anyhow::Result<()> {
    use super::modpack::{
        packinfo,
        session::{Phase, Session},
    };
    let app = crate::setup_managers_for_test().await;
    let name = format!("Vault Hunters 3rd Edition {}", "x".repeat(144));
    let (id, root) = fixture(&app, &name).await?;
    assert!(root.as_os_str().len() >= 190, "{}", root.display());
    let deep = format!(
        "kubejs/{}/{}/biome_tags.json",
        "a".repeat(40),
        "b".repeat(40)
    );
    task08_archive(
        &root,
        &[("mods/a.jar.disabled", b"target"), (deep.as_str(), b"deep")],
    )?;
    tokio::fs::create_dir(root.join("instance/mods")).await?;
    tokio::fs::write(root.join("instance/mods/a.jar"), b"old").await?;
    let old = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(old)?).await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Committed);
    assert!(!root.join("instance/mods/a.jar").exists());
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/a.jar.disabled")).await?,
        b"target"
    );
    let deep = root.join("instance").join(&deep);
    assert!(deep.as_os_str().len() >= 270);
    assert_eq!(tokio::fs::read(&deep).await?, b"deep");
    let mut records = tokio::fs::read_dir(root.join(".setup/representation-transitions")).await?;
    let record_path = records.next_entry().await?.unwrap().path();
    let record: serde_json::Value = serde_json::from_slice(&tokio::fs::read(&record_path).await?)?;
    assert_eq!(record["completed"], true);
    let batch = root
        .join(".repair-recovery")
        .join(record["recovery_batch"].as_str().unwrap());
    let entry = tokio::fs::read_dir(batch)
        .await?
        .next_entry()
        .await?
        .unwrap()
        .path();
    assert!(entry.join("entry.json").as_os_str().len() >= 260);
    assert_eq!(tokio::fs::read(entry.join("payload")).await?, b"old");
    Ok(())
}

#[tokio::test]
async fn task08_promoted_restart_merges_current_edits_and_rejects_old_record() -> anyhow::Result<()>
{
    use super::modpack::session::{Phase, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "promotion-gap").await?;
    task08_archive(&root, &[("target.txt", b"target")])?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    let pause = app.instance_manager.test_hooks.arm("pack_promoted", id);
    {
        let apply = run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath.clone(),
            &subtasks,
            options.clone(),
            oracle,
            owner.as_ref(),
        );
        tokio::pin!(apply);
        tokio::select! { _ = pause.entered.notified() => {}, result = &mut apply => panic!("apply failed before promotion: {result:?}") }
    } // a interrupted worker releases its gate; durable Applied survives
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Applied);
    assert!(
        matches!(disk_config(&root).await?.game_configuration.version, Some(GameVersion::Standard(v)) if v.release == "1.7.10")
    );
    assert!(!app.instance_manager().pending_setup(id).await?.can_cancel);
    assert!(app.instance_manager().repair_preview(id).await.is_err());
    let promoted = tokio::fs::read(root.join("packinfo.json")).await?;
    tokio::fs::write(
        root.join("packinfo.json"),
        super::modpack::packinfo::make_packinfo(super::modpack::packinfo::PackInfo {
            files: HashMap::new(),
        })?,
    )
    .await?;
    assert!(
        run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath.clone(),
            &subtasks,
            options.clone(),
            None,
            owner.as_ref()
        )
        .await
        .is_err()
    );
    tokio::fs::write(root.join("packinfo.json"), promoted).await?;
    let mut edit = update(id);
    edit.notes = Some("changed during recovery".into());
    app.instance_manager().update_instance(edit).await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        None,
        owner.as_ref(),
    )
    .await?;
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Committed);
    assert_eq!(disk_config(&root).await?.notes, "changed during recovery");
    assert!(
        matches!(disk_config(&root).await?.game_configuration.version, Some(GameVersion::Standard(v)) if v.release == "1.20.1")
    );
    Ok(())
}

#[tokio::test]
async fn task08_cancel_refuses_stale_foreign_partial_and_retains_evidence() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "cancel-refusals").await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    tokio::fs::create_dir(root.join(".setup")).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let mut session = Session::new(&config, Producer::Change, Kind::Pack, None)?;
    session.instance_id = Some(id.0);
    session.create(&root).await?;
    let owner = session.owner()?;
    tokio::fs::write(root.join(".setup/evidence"), b"preserve").await?;
    assert!(
        app.instance_manager()
            .cancel_pending_setup(id, uuid::Uuid::new_v4().to_string())
            .await
            .is_err()
    );
    session.phase = Phase::Applying;
    session.save(&root, &owner).await?;
    assert!(
        app.instance_manager()
            .cancel_pending_setup(id, owner.operation.clone())
            .await
            .is_err()
    );
    session.instance_id = Some(id.0 + 100);
    super::modpack::session::atomic_json(&root.join(".setup/setup-session.json"), &session, false)
        .await?;
    assert!(
        app.instance_manager()
            .cancel_pending_setup(id, owner.operation)
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read(root.join(".setup/evidence")).await?,
        b"preserve"
    );
    Ok(())
}

#[tokio::test]
async fn task08_current_missing_source_refuses_without_deleting_live_twins() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "vanished-source").await?;
    task08_archive(&root, &[("a.disabled", b"target")])?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    tokio::fs::remove_file(root.join(".setup/staging/instance/a.disabled")).await?;
    tokio::fs::write(root.join("instance/a"), b"old").await?;
    tokio::fs::write(root.join("instance/a.disabled"), b"target").await?;
    assert!(
        run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath,
            &subtasks,
            options,
            oracle,
            owner.as_ref()
        )
        .await
        .is_err()
    );
    assert_eq!(tokio::fs::read(root.join("instance/a")).await?, b"old");
    assert_eq!(
        tokio::fs::read(root.join("instance/a.disabled")).await?,
        b"target"
    );
    Ok(())
}

#[tokio::test]
async fn task08_copied_pending_archive_is_reconstructed_for_new_identity() -> anyhow::Result<()> {
    use super::modpack::session::Session;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "copy-pending").await?;
    task08_archive(&root, &[("target.txt", b"target")])?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut original_owner = None;
    run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut original_owner,
    )
    .await?;
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "copy destination".into())
        .await?;
    let (shortpath, config) = app.instance_manager().current_config(copy).await?;
    let copy_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    assert_eq!(
        Session::load(&copy_root).await?.unwrap().owner()?,
        original_owner.clone().unwrap()
    );
    let mut copy_owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        copy,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut copy_owner,
    )
    .await?;
    assert_eq!(copy_owner.as_ref().unwrap().instance_id, copy.0);
    assert_ne!(copy_owner, original_owner);
    run::modpack::process_modpack_staging(
        app.clone(),
        copy,
        shortpath,
        &subtasks,
        options,
        oracle,
        copy_owner.as_ref(),
    )
    .await?;
    assert_eq!(
        tokio::fs::read(copy_root.join("instance/target.txt")).await?,
        b"target"
    );
    assert!(!root.join("instance/target.txt").exists());
    assert_eq!(
        Session::load(&root).await?.unwrap().owner()?,
        original_owner.unwrap()
    );
    Ok(())
}

#[tokio::test]
async fn task08_markerless_tmp_only_remains_recoverable() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "ambiguous-tmp").await?;
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    tokio::fs::write(root.join("tmp-packinfo.json"), b"legacy evidence").await?;
    tokio::fs::write(root.join("instance/keep.txt"), b"keep").await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    assert!(
        run::modpack::process_modpack(
            app.clone(),
            id,
            true,
            config,
            shortpath,
            &task,
            false,
            &mut None
        )
        .await
        .is_err()
    );
    assert_eq!(
        tokio::fs::read(root.join("tmp-packinfo.json")).await?,
        b"legacy evidence"
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/keep.txt")).await?,
        b"keep"
    );
    assert!(!app.instance_manager().pending_setup(id).await?.can_cancel);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn task08_repair_pipeline_filters_unrelated_but_refuses_required_fault() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    for required in [false, true] {
        let (id, root) = fixture(
            &app,
            if required {
                "required-fault"
            } else {
                "unrelated-fault"
            },
        )
        .await?;
        task08_archive(&root, &[("mods/required.jar", b"target")])?;
        tokio::fs::write(
            root.join(".setup/repair"),
            serde_json::to_vec(&super::modpack::RepairMarkerFile::default())?,
        )
        .await?;
        tokio::fs::create_dir(root.join("instance/mods")).await?;
        tokio::fs::create_dir(root.join("instance/logs")).await?;
        let broken = root.join(if required {
            "instance/mods/required.jar"
        } else {
            "instance/logs/unrelated"
        });
        std::os::unix::fs::symlink("absent", &broken)?;
        let (shortpath, config) = app.instance_manager().current_config(id).await?;
        let task = crate::managers::vtask::VisualTask::new(
            crate::api::translation::Translation::InstanceTaskPrepare {
                name: "fixture".into(),
            },
        );
        let mut owner = None;
        let result = run::modpack::process_modpack(
            app.clone(),
            id,
            true,
            config,
            shortpath.clone(),
            &task,
            false,
            &mut owner,
        )
        .await;
        if required {
            // Names the link and the way out.
            let message = result.err().expect("required scan must fail").to_string();
            assert!(
                message.contains("Could not read /mods/required.jar in the instance folder"),
                "{message}"
            );
            assert!(message.contains("Reconnect the drive"), "{message}");
        } else {
            let (subtasks, _, options, oracle) = result?;
            run::modpack::process_modpack_staging(
                app.clone(),
                id,
                shortpath,
                &subtasks,
                options,
                oracle,
                owner.as_ref(),
            )
            .await?;
            assert_eq!(
                tokio::fs::read(root.join("instance/mods/required.jar")).await?,
                b"target"
            );
        }
        assert!(std::fs::symlink_metadata(broken)?.file_type().is_symlink());
    }
    Ok(())
}

#[tokio::test]
async fn task08_all_setup_producers_obey_runtime_freeze_before_disk_write() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "frozen-producers").await?;
    let before = tokio::fs::read(root.join("instance.json")).await?;
    let session_before = tokio::fs::read(root.join(".setup/setup-session.json")).await?;
    app.runtime_lifecycle.begin_relocation("test-freeze")?;
    let pack = info::Modpack::Modrinth(info::ModrinthModpack {
        project_id: "synthetic".into(),
        version_id: "B".into(),
    });
    assert!(
        app.instance_manager()
            .change_modpack(id, pack)
            .await
            .unwrap_err()
            .to_string()
            .contains("relocation")
    );
    assert!(
        app.instance_manager()
            .repair_modpack(id, super::modpack::RepairMarkerFile::default(), None)
            .await
            .unwrap_err()
            .to_string()
            .contains("relocation")
    );
    let mut edit = update(id);
    edit.version = Some("1.20.1".into());
    assert!(
        app.instance_manager()
            .update_instance(edit)
            .await
            .unwrap_err()
            .to_string()
            .contains("relocation")
    );
    assert!(
        app.instance_manager()
            .duplicate_instance(id, "frozen-copy".into())
            .await
            .unwrap_err()
            .to_string()
            .contains("relocation")
    );
    assert_eq!(tokio::fs::read(root.join("instance.json")).await?, before);
    assert_eq!(
        tokio::fs::read(root.join(".setup/setup-session.json")).await?,
        session_before
    );
    app.runtime_lifecycle.cancel_relocation("test-freeze")?;
    Ok(())
}

#[tokio::test]
async fn task08_private_and_imported_complete_shapes_qualify_without_pack_download()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Session};
    let app = crate::setup_managers_for_test().await;
    for shape in [
        "private-version",
        "legacy-complete-file",
        "legacy-complete-dir",
    ] {
        let (id, root) = fixture(&app, shape).await?;
        if shape == "private-version" {
            let mut session = Session::load(&root).await?.unwrap();
            session.instance_id = None;
            super::modpack::session::atomic_json(
                &root.join(".setup/setup-session.json"),
                &session,
                false,
            )
            .await?;
        } else {
            tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
            let pack = info::Modpack::Modrinth(info::ModrinthModpack {
                project_id: "imported".into(),
                version_id: "installed".into(),
            });
            tokio::fs::write(
                root.join(".setup/change-pack-version.json"),
                serde_json::to_vec(&super::modpack::PackVersionFile::from(pack))?,
            )
            .await?;
            if shape.ends_with("file") {
                tokio::fs::write(root.join(".setup/modpack-complete"), b"").await?;
            } else {
                tokio::fs::create_dir(root.join(".setup/modpack-complete")).await?;
            }
        }
        let (shortpath, config) = app.instance_manager().current_config(id).await?;
        let task = crate::managers::vtask::VisualTask::new(
            crate::api::translation::Translation::InstanceTaskPrepare {
                name: "fixture".into(),
            },
        );
        let mut owner = None;
        let (subtasks, _, options, oracle) = run::modpack::process_modpack(
            app.clone(),
            id,
            true,
            config,
            shortpath.clone(),
            &task,
            false,
            &mut owner,
        )
        .await?;
        run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath,
            &subtasks,
            options,
            oracle,
            owner.as_ref(),
        )
        .await?;
        let session = Session::load(&root).await?.unwrap();
        assert_eq!(session.owner()?.instance_id, id.0);
        assert_ne!(session.kind, Kind::Pack);
        assert_eq!(session.phase, Phase::Committed);
        assert!(!root.join("tmp-packinfo.json").exists());
    }
    Ok(())
}

#[tokio::test]
async fn task08_normal_completed_duplication_remains_launchable_without_setup() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "completed-copy").await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    tokio::fs::write(root.join("instance/kept.txt"), b"installed").await?;
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "completed duplicate".into())
        .await?;
    let (shortpath, config) = app.instance_manager().current_config(copy).await?;
    let copy_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    assert!(!copy_root.join(".setup").exists());
    assert_eq!(
        tokio::fs::read(copy_root.join("instance/kept.txt")).await?,
        b"installed"
    );
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    run::modpack::process_modpack(
        app.clone(),
        copy,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut owner,
    )
    .await?;
    assert!(owner.is_none());
    Ok(())
}

#[tokio::test]
async fn task08_legacy_transition_adopts_both_records_and_preserves_payload() -> anyhow::Result<()>
{
    use super::modpack::{packinfo, session::Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "legacy-transition").await?;
    task08_archive(&root, &[("a.disabled", b"target")])?;
    tokio::fs::write(root.join("instance/a"), b"old").await?;
    let old = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(old)?).await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath.clone(),
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    let mut records = tokio::fs::read_dir(root.join(".setup/representation-transitions")).await?;
    let record_path = records.next_entry().await?.unwrap().path();
    let mut record: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(&record_path).await?)?;
    let mut entries = tokio::fs::read_dir(
        root.join(".repair-recovery")
            .join(record["recovery_batch"].as_str().unwrap()),
    )
    .await?;
    let recovery = entries.next_entry().await?.unwrap().path();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(recovery.join("entry.json")).await?)?;
    for value in [&mut record, &mut manifest] {
        value["schema_version"] = 1.into();
        value.as_object_mut().unwrap().remove("owner");
    }
    tokio::fs::write(&record_path, serde_json::to_vec(&record)?).await?;
    tokio::fs::write(recovery.join("entry.json"), serde_json::to_vec(&manifest)?).await?;
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let mut new_owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut new_owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        new_owner.as_ref(),
    )
    .await?;
    let session = Session::load(&root).await?.unwrap();
    let record: serde_json::Value = serde_json::from_slice(&tokio::fs::read(record_path).await?)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(recovery.join("entry.json")).await?)?;
    assert_eq!(record["schema_version"], 2);
    assert_eq!(manifest["schema_version"], 2);
    assert_eq!(record["owner"], manifest["owner"]);
    assert_eq!(record["owner"]["operation"], session.operation);
    assert_eq!(tokio::fs::read(recovery.join("payload")).await?, b"old");
    assert_eq!(
        tokio::fs::read(root.join("instance/a.disabled")).await?,
        b"target"
    );
    Ok(())
}

#[tokio::test]
async fn task08_reconstruction_preserves_prior_target_and_list() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "legacy-prior-records").await?;
    task08_archive(&root, &[("new.txt", b"new")])?;
    tokio::fs::write(root.join("tmp-packinfo.json"), b"legacy target evidence").await?;
    tokio::fs::write(
        root.join(".setup/staging-packinfo.json"),
        b"legacy list evidence",
    )
    .await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let shortpath = root.file_name().unwrap().to_str().unwrap().to_owned();
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut owner,
    )
    .await?;
    let mut retained = Vec::new();
    for entry in std::fs::read_dir(root.join(".setup"))? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("prior-record-")
        {
            retained.push(std::fs::read(entry.path())?);
        }
    }
    assert!(
        retained
            .iter()
            .any(|bytes| bytes == b"legacy target evidence")
    );
    assert!(
        retained
            .iter()
            .any(|bytes| bytes == b"legacy list evidence")
    );
    Ok(())
}

// Round 1 fixtures exercise the real archive decoder, skip selection, record
// publication and re-entry. Local original bytes only satisfy an already
// selected Downloadable; the normal integrity check still runs offline.
fn task08_round1_archive(root: &Path, downloadable: bool) -> anyhow::Result<()> {
    use sha2::Digest;
    use std::io::Write;
    let bytes = b"original downloadable";
    let files = if downloadable {
        serde_json::json!([{
            "path":"mods/a.jar", "hashes":{"sha1":"", "sha512":hex::encode(sha2::Sha512::digest(bytes))},
            "env":{"client":"required","server":"required"},
            "downloads":["https://task08.invalid/original.jar"],"fileSize":bytes.len()
        }])
    } else {
        serde_json::json!([])
    };
    let index = serde_json::json!({"formatVersion":1,"game":"minecraft","versionId":"synthetic-B","name":"fixture","files":files,"dependencies":{"minecraft":"1.20.1"}});
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join(".setup/modrinth"))?);
    zip.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    zip.write_all(&serde_json::to_vec(&index)?)?;
    if !downloadable {
        zip.start_file(
            "overrides/target.txt",
            zip::write::SimpleFileOptions::default(),
        )?;
        zip.write_all(b"target")?;
    }
    zip.finish()?;
    Ok(())
}

async fn task08_round1_publication(
    producer: super::modpack::session::Producer,
) -> anyhow::Result<()> {
    use super::modpack::session::{Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    for (index, boundary) in [
        "setup_staging_saved",
        "setup_session_saved",
        "setup_inventory_saved",
        "setup_target_saved",
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("round1-{producer:?}-{index}");
        let (id, root) = if producer == Producer::Reconstructed {
            let (id, root) = fixture(&app, &name).await?;
            tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
            task08_round1_archive(&root, false)?;
            (id, root)
        } else {
            let group = app.instance_manager().get_default_group().await?;
            let pack = info::Modpack::Modrinth(info::ModrinthModpack {
                project_id: "synthetic".into(),
                version_id: "B".into(),
            });
            let id = app
                .instance_manager()
                .create_instance_ext(
                    group,
                    name,
                    None,
                    None,
                    None,
                    InstanceVersionSource::Modpack(pack, true),
                    String::new(),
                    move |root| async move {
                        if producer == Producer::Import {
                            task08_round1_archive(&root, false)?;
                        }
                        Ok(())
                    },
                )
                .await?;
            let (shortpath, _) = app.instance_manager().current_config(id).await?;
            let root = app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&shortpath)
                .get_root();
            if producer == Producer::Create {
                task08_round1_archive(&root, false)?;
            }
            assert_eq!(Session::load(&root).await?.unwrap().producer, producer);
            (id, root)
        };
        let old_tmp = b"old truncated target evidence";
        let old_list = b"old inventory evidence";
        // Missing inventory after the first boundary; an existing malformed
        // target after the second. Both have complete verified Session input.
        tokio::fs::write(root.join("tmp-packinfo.json"), old_tmp).await?;
        if index >= 2 {
            tokio::fs::write(root.join(".setup/staging-packinfo.json"), old_list).await?;
        }
        let (shortpath, config) = app.instance_manager().current_config(id).await?;
        let task = crate::managers::vtask::VisualTask::new(
            crate::api::translation::Translation::InstanceTaskPrepare {
                name: "fixture".into(),
            },
        );
        let pause = app.instance_manager.test_hooks.arm(boundary, id);
        let mut owner = None;
        {
            let prepare = run::modpack::process_modpack(
                app.clone(),
                id,
                true,
                config,
                shortpath.clone(),
                &task,
                true,
                &mut owner,
            );
            tokio::pin!(prepare);
            tokio::select! {
                _ = pause.entered.notified() => {},
                result = &mut prepare => panic!("publication never paused: {:?}", result.err()),
                _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("bounded publication pause timed out"),
            }
        }
        if boundary != "setup_staging_saved" {
            assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Resolved);
        }
        assert!(!root.join("instance/target.txt").exists());
        let (_, config) = app.instance_manager().current_config(id).await?;
        let (subtasks, _, options, oracle) = run::modpack::process_modpack(
            app.clone(),
            id,
            true,
            config,
            shortpath.clone(),
            &task,
            true,
            &mut owner,
        )
        .await?;
        run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath,
            &subtasks,
            options,
            oracle,
            owner.as_ref(),
        )
        .await?;
        assert_eq!(
            tokio::fs::read(root.join("instance/target.txt")).await?,
            b"target"
        );
        assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Committed);
        let mut originals = Vec::new();
        for entry in std::fs::read_dir(root.join(".setup"))? {
            let entry = entry?;
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with("prior-record-")
            {
                originals.push(std::fs::read(entry.path())?);
            }
        }
        assert!(originals.iter().any(|b| b == old_tmp));
        if index >= 2 {
            assert!(originals.iter().any(|b| b == old_list));
        }
    }
    Ok(())
}
#[tokio::test]
async fn task08_round1_publication_create() -> anyhow::Result<()> {
    task08_round1_publication(super::modpack::session::Producer::Create).await
}
#[tokio::test]
async fn task08_round1_publication_import() -> anyhow::Result<()> {
    task08_round1_publication(super::modpack::session::Producer::Import).await
}
#[tokio::test]
async fn task08_round1_publication_reconstructed() -> anyhow::Result<()> {
    task08_round1_publication(super::modpack::session::Producer::Reconstructed).await
}

async fn task08_round1_reenable(published: bool, mode: &str, cf: bool) -> anyhow::Result<()> {
    use super::modpack::{RepairMarkerFile, packinfo, session::Session};
    let app = crate::setup_managers_for_test().await;
    let (mut id, mut root) = fixture(&app, "round1-download-reenable").await?;
    let copied = mode.starts_with("copy");
    let half = mode.ends_with("half");
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    let mut metadata_guards = Vec::new();
    if cf {
        task08_round2_cf_archive(&root)?;
        metadata_guards.push(
            crate::managers::minecraft::curseforge::preparation_fixture::set(
                root.join(".setup/curseforge"),
                task08_round2_cf_metadata(),
            ),
        );
    } else {
        task08_round1_archive(&root, true)?;
    }
    tokio::fs::create_dir_all(root.join("instance/mods")).await?;
    if cf {
        tokio::fs::write(root.join("instance/mods/b.jar"), b"original downloadable").await?;
    }
    tokio::fs::write(
        root.join("instance/mods/a.jar.disabled"),
        b"original downloadable",
    )
    .await?;
    let old = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(old)?).await?;
    let repair = RepairMarkerFile {
        re_enable_disabled: true,
        ..Default::default()
    };
    tokio::fs::write(root.join(".setup/repair"), serde_json::to_vec(&repair)?).await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        true,
        &mut owner,
    )
    .await?;
    let initial = Session::load(&root).await?.unwrap();
    assert!(
        initial.target.as_ref().unwrap().sources.is_empty(),
        "ordinary no-transition skip remains effective"
    );
    assert_eq!(
        initial.target.as_ref().unwrap().skipped,
        if cf {
            vec!["/mods/a.jar", "/mods/b.jar"]
        } else {
            vec!["/mods/a.jar"]
        }
    );
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath.clone(),
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    let mut record_path = std::fs::read_dir(root.join(".setup/representation-transitions"))?
        .next()
        .unwrap()?
        .path();
    let mut record: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(&record_path).await?)?;
    assert_eq!(record["entry"]["action"], "ReEnable");
    let mut recovery = std::fs::read_dir(
        root.join(".repair-recovery")
            .join(record["recovery_batch"].as_str().unwrap()),
    )?
    .next()
    .unwrap()?
    .path();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(recovery.join("entry.json")).await?)?;
    if !copied && mode != "foreign" {
        for value in [&mut record, &mut manifest] {
            value["schema_version"] = 1.into();
            value.as_object_mut().unwrap().remove("owner");
        }
    }
    record["completed"] = false.into();
    record["published"] = published.into();
    if !published {
        tokio::fs::remove_file(root.join("instance/mods/a.jar")).await?;
        tokio::fs::rename(
            recovery.join("payload"),
            root.join("instance/mods/a.jar.disabled"),
        )
        .await?;
    }
    tokio::fs::write(&record_path, serde_json::to_vec(&record)?).await?;
    tokio::fs::write(recovery.join("entry.json"), serde_json::to_vec(&manifest)?).await?;
    let original_root = root.clone();
    let original_record = tokio::fs::read(&record_path).await?;
    if copied {
        let mut saved = Session::load(&root).await?.unwrap();
        saved.phase = super::modpack::session::Phase::Applying;
        saved.save(&root, &saved.owner()?).await?;
        id = app
            .instance_manager()
            .duplicate_instance(id, "round1-copy".into())
            .await?;
        let (path, _) = app.instance_manager().current_config(id).await?;
        root = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&path)
            .get_root();
        record_path = root.join(record_path.strip_prefix(&original_root)?);
        recovery = root.join(recovery.strip_prefix(&original_root)?);
    } else {
        tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
        if mode == "old-resolved" {
            let mut saved = initial.clone();
            saved.operation = uuid::Uuid::new_v4().to_string();
            saved.create(&root).await?;
        }
    }
    if cf && copied {
        metadata_guards.push(
            crate::managers::minecraft::curseforge::preparation_fixture::set(
                root.join(".setup/curseforge"),
                task08_round2_cf_metadata(),
            ),
        );
    }
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let mut owner = None;
    let pause = app
        .instance_manager
        .test_hooks
        .arm("setup_original_acquisition", id);
    let adoption = if half {
        Some(
            app.instance_manager
                .test_hooks
                .arm("setup_transition_manifest_saved", id),
        )
    } else {
        None
    };
    let mut acquired = false;
    let prepared = {
        let prepare = run::modpack::process_modpack(
            app.clone(),
            id,
            true,
            config,
            shortpath.clone(),
            &task,
            true,
            &mut owner,
        );
        tokio::pin!(prepare);
        let deadline = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _ = pause.entered.notified(), if !acquired => {
                    let staging = std::fs::read_dir(root.join(".setup"))?.filter_map(Result::ok).find(|e| e.file_name().to_string_lossy().starts_with("preparing-")).unwrap().path();
                    tokio::fs::create_dir_all(staging.join("instance/mods")).await?;
                    tokio::fs::write(staging.join("instance/mods/a.jar"), b"original downloadable").await?;
                    acquired = true; pause.release.notify_one();
                },
                _ = async { adoption.as_ref().unwrap().entered.notified().await }, if half => { break None; },
                result = &mut prepare => break Some(result),
                _ = &mut deadline => panic!("bounded acquisition/adoption timed out"),
            }
        }
    };
    if mode == "foreign" {
        assert!(!acquired);
        assert!(
            prepared
                .unwrap()
                .err()
                .unwrap()
                .to_string()
                .contains("foreign ownership")
        );
        assert_eq!(tokio::fs::read(&record_path).await?, original_record);
        assert_eq!(
            tokio::fs::read(recovery.join("payload")).await?,
            b"original downloadable"
        );
        return Ok(());
    }
    assert!(
        acquired,
        "transition source must receive an independently verified original"
    );
    let (subtasks, _, options, oracle) = match prepared {
        Some(result) => result?,
        None => {
            let before: serde_json::Value =
                serde_json::from_slice(&tokio::fs::read(&record_path).await?)?;
            let adopted: serde_json::Value =
                serde_json::from_slice(&tokio::fs::read(recovery.join("entry.json")).await?)?;
            assert_ne!(
                before["owner"], adopted["owner"],
                "fixture stopped between the two actual adoption writes"
            );
            let (_, config) = app.instance_manager().current_config(id).await?;
            run::modpack::process_modpack(
                app.clone(),
                id,
                true,
                config,
                shortpath.clone(),
                &task,
                true,
                &mut owner,
            )
            .await?
        }
    };
    assert!(
        Session::load(&root)
            .await?
            .unwrap()
            .target
            .unwrap()
            .sources
            .contains_key("/mods/a.jar")
    );
    if cf {
        let target = Session::load(&root).await?.unwrap().target.unwrap();
        assert!(
            !target.sources.contains_key("/mods/b.jar"),
            "unrelated matching descriptor must still skip"
        );
        assert_eq!(target.skipped, vec!["/mods/b.jar"]);
    }
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    let record: serde_json::Value = serde_json::from_slice(&tokio::fs::read(&record_path).await?)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(recovery.join("entry.json")).await?)?;
    assert_eq!(record["owner"], manifest["owner"]);
    assert_eq!(record["owner"]["instance_id"], id.0);
    if copied {
        let original_path = original_root.join(record_path.strip_prefix(&root)?);
        assert_eq!(
            tokio::fs::read(original_path).await?,
            original_record,
            "copy adoption leaves original operation intact"
        );
    }
    assert_eq!(record["schema_version"], 2);
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/a.jar")).await?,
        b"original downloadable"
    );
    assert_eq!(
        tokio::fs::read(recovery.join("payload")).await?,
        b"original downloadable"
    );
    assert!(!root.join("instance/mods/a.jar.disabled").exists());
    Ok(())
}
#[tokio::test]
async fn task08_round1_download_reenable_before() -> anyhow::Result<()> {
    task08_round1_reenable(false, "legacy", false).await
}
#[tokio::test]
async fn task08_round1_download_reenable_after() -> anyhow::Result<()> {
    task08_round1_reenable(true, "legacy", false).await
}

#[tokio::test]
async fn task08_round1_download_copy_before() -> anyhow::Result<()> {
    task08_round1_reenable(false, "copy", false).await
}
#[tokio::test]
async fn task08_round1_download_copy_after() -> anyhow::Result<()> {
    task08_round1_reenable(true, "copy", false).await
}
#[tokio::test]
async fn task08_round1_download_legacy_half_adoption() -> anyhow::Result<()> {
    task08_round1_reenable(true, "legacy-half", false).await
}
#[tokio::test]
async fn task08_round1_download_copy_half_adoption() -> anyhow::Result<()> {
    task08_round1_reenable(false, "copy-half", false).await
}
#[tokio::test]
async fn task08_round1_download_saved_resolved_retry() -> anyhow::Result<()> {
    task08_round1_reenable(true, "old-resolved", false).await
}
#[tokio::test]
async fn task08_round1_download_foreign_transition_refused() -> anyhow::Result<()> {
    task08_round1_reenable(true, "foreign", false).await
}

#[tokio::test]
async fn task08_round1_publication_requires_original_owner_and_sources() -> anyhow::Result<()> {
    use super::modpack::session::{self, Phase, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "round1-publication-authority").await?;
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    task08_round1_archive(&root, false)?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let mut owner = None;
    let (subtasks, _, options, _) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        true,
        &mut owner,
    )
    .await?;
    let original = Session::load(&root).await?.unwrap();
    let unrelated = b"unrelated temporary record is evidence, not authority";
    tokio::fs::write(root.join("tmp-packinfo.json"), unrelated).await?;
    tokio::fs::remove_file(root.join(".setup/staging-packinfo.json")).await?;
    let mut foreign = original.clone();
    foreign.operation = uuid::Uuid::new_v4().to_string();
    session::atomic_json(&root.join(".setup/setup-session.json"), &foreign, false).await?;
    assert!(
        run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath.clone(),
            &subtasks,
            options.clone(),
            None,
            owner.as_ref()
        )
        .await
        .is_err()
    );
    assert_eq!(
        tokio::fs::read(root.join("tmp-packinfo.json")).await?,
        unrelated
    );
    assert!(!root.join(".setup/staging-packinfo.json").exists());
    session::atomic_json(&root.join(".setup/setup-session.json"), &original, false).await?;
    tokio::fs::write(
        root.join(".setup/staging/instance/target.txt"),
        b"changed original",
    )
    .await?;
    assert!(
        run::modpack::process_modpack_staging(
            app.clone(),
            id,
            shortpath.clone(),
            &subtasks,
            options.clone(),
            None,
            owner.as_ref()
        )
        .await
        .is_err()
    );
    assert_eq!(
        tokio::fs::read(root.join("tmp-packinfo.json")).await?,
        unrelated
    );
    assert!(!root.join("instance/target.txt").exists());
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Resolved);
    tokio::fs::write(root.join(".setup/staging/instance/target.txt"), b"target").await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        None,
        owner.as_ref(),
    )
    .await?;
    assert_eq!(
        tokio::fs::read(root.join("instance/target.txt")).await?,
        b"target"
    );
    let mut retained = false;
    for entry in std::fs::read_dir(root.join(".setup"))? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("prior-record-")
            && std::fs::read(entry.path())? == unrelated
        {
            retained = true;
        }
    }
    assert!(retained);
    Ok(())
}

/// A kitchen-sink sized pack (3,000 override files) prepares and applies
/// with a session load count that does not grow with the pack. Loading the
/// session once per file made "Applying staged patches" quadratic.
#[tokio::test]
async fn large_pack_applies_with_a_bounded_number_of_session_loads() -> anyhow::Result<()> {
    use super::modpack::{packinfo, session::load_count};
    const FILES: usize = 3000;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "large-pack").await?;
    let files: Vec<_> = (0..FILES)
        .map(|i| {
            (
                format!("config/pack/{:02}/f{i:04}.json", i % 30),
                format!("{{\"file\":{i}}}").into_bytes(),
            )
        })
        .collect();
    task08_archive(
        &root,
        &files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    )?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    );
    let before = load_count::of(&root);
    let mut owner = None;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        true,
        &mut owner,
    )
    .await?;
    let prepared = load_count::of(&root);
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    let applied = load_count::of(&root) - prepared;
    let prepared = prepared - before;

    for (path, bytes) in &files {
        assert_eq!(
            &tokio::fs::read(root.join("instance").join(path)).await?,
            bytes
        );
    }
    let installed =
        packinfo::parse_packinfo(&tokio::fs::read_to_string(root.join("packinfo.json")).await?)?;
    assert_eq!(installed.files.len(), FILES);
    assert!(prepared > 0 && applied > 0, "loads were not counted");
    // A constant handful of phase checks and saves, never one per file.
    assert!(prepared + applied < 40, "{prepared} + {applied} loads");
    Ok(())
}

/// `count` override files in one config folder.
fn config_files(count: usize) -> Vec<(String, Vec<u8>)> {
    (0..count)
        .map(|i| {
            (
                format!("config/pack/f{i:02}.json"),
                format!("{{\"file\":{i}}}").into_bytes(),
            )
        })
        .collect()
}

/// A synthetic override-only pack of `files`, prepared up to the apply.
async fn prepared_pack(
    app: &crate::App,
    name: &str,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    let (id, root) = fixture(app, name).await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
    config.modpack = Some(info::ModpackInfo {
        modpack: task08_round2_pack("A"),
        locked: true,
    });
    app.instance_manager()
        .persist_config(id, &shortpath, config)
        .await?;
    Ok((id, root))
}
async fn task08_round2_wait_failed(app: &crate::App, id: InstanceId) -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(
                app.instance_manager.instances.read().await[&id]
                    .data()?
                    .state,
                run::LaunchState::Inactive {
                    failed_task: Some(_)
                }
            ) {
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    Ok(())
}

/// An apply stopped halfway leaves its published files as second names of
/// the staged originals, so a config edited in place before the retry is
/// edited in staging too. The retry must not refuse that as a changed
/// original, and it keeps the edit.
#[tokio::test]
async fn resumed_apply_keeps_an_in_place_edit_of_a_published_file() -> anyhow::Result<()> {
    use run::modpack::staging_test::fail_io;
    use std::io::Write;
    const EDITED: &[u8] = b"{\"edited\":true}";
    let app = crate::setup_managers_for_test().await;
    let (id, root) = task08_round2_change_fixture(&app, "producer-drop").await?;
    let permit = app
        .instance_manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let pause = session::publication_fixture::arm(root.join(".setup/setup-session.json"));
    let before_handoff = app
        .instance_manager
        .test_hooks
        .arm("producer_before_handoff", id);
    let handoff = app
        .instance_manager
        .test_hooks
        .arm("producer_handed_off", id);
    {
        let request = app
            .instance_manager()
            .change_modpack(id, task08_round2_pack("B"));
        tokio::pin!(request);
        tokio::select! {
            _=pause.entered.notified()=>{},
            result=&mut request=>panic!("request did not reach real blocking publication: {result:?}"),
            _=tokio::time::sleep(Duration::from_secs(5))=>panic!("publication wait timed out"),
        }
        assert!(matches!(
            app.instance_manager.instances.read().await[&id]
                .data()?
                .state,
            run::LaunchState::Queued(_)
        ));
    } // Actual request-side future Drop while the blocking writer is outstanding.
    let busy = app
        .runtime_lifecycle
        .begin_relocation("request-drop-probe")
        .is_err();
    if !busy {
        app.runtime_lifecycle
            .cancel_relocation("request-drop-probe")?;
    }
    let gate_held = tokio::time::timeout(
        Duration::from_millis(100),
        app.instance_manager().lock_instance(id),
    )
    .await
    .is_err();
    pause.release();
    tokio::time::timeout(Duration::from_secs(5), pause.finished.notified()).await?;
    assert!(
        busy,
        "request Drop released lifecycle ownership before real session publication settled"
    );
    assert!(
        gate_held,
        "request Drop released the stable instance gate while its writer was pending"
    );
    tokio::time::timeout(Duration::from_secs(5), before_handoff.entered.notified()).await?;
    assert!(
        app.runtime_lifecycle
            .begin_relocation("before-handoff-probe")
            .is_err()
    );
    assert!(
        tokio::time::timeout(
            Duration::from_millis(100),
            app.instance_manager().lock_instance(id)
        )
        .await
        .is_err(),
        "gate must remain owned until the worker is spawned"
    );
    before_handoff.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), handoff.entered.notified()).await?;
    let saved = Session::load(&root).await?.unwrap();
    assert_eq!(saved.instance_id, Some(id.0));
    assert_eq!(
        tokio::fs::read_to_string(root.join(".setup/change-pack-version.json")).await?,
        saved.requested_pack.unwrap()
    );
    assert!(
        app.runtime_lifecycle
            .begin_relocation("handoff-probe")
            .is_err()
    );
    let gate = tokio::time::timeout(
        Duration::from_secs(1),
        app.instance_manager().lock_instance(id),
    )
    .await?;
    drop(gate);
    // A controlled owned failure after handoff avoids provider/game work.
    tokio::fs::write(
        root.join(".setup/setup-session.json"),
        b"controlled invalid session",
    )
    .await?;
    handoff.release.notify_one();
    drop(permit);
    task08_round2_wait_failed(&app, id).await?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if app.runtime_lifecycle.begin_relocation("settled").is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    app.runtime_lifecycle.cancel_relocation(&settled)?;
    Ok(())
}

/// The baseline record a version change of a linked pack without
/// `packinfo.json` writes first, from the verified inventory (see
/// `imported_baseline::ensure`). It stays whatever the request does next.
async fn task08_round2_baseline_recorded(root: &Path) -> anyhow::Result<()> {
    let record: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(root.join("packinfo.json")).await?)?;
    assert_eq!(
        record["verified_import_inventory"],
        serde_json::json!({"schema": 1, "declared_files": 0})
    );
    Ok(())
}

async fn task08_round2_settled(app: &crate::App, id: InstanceId) -> anyhow::Result<()> {
    task08_round2_wait_failed(app, id).await?;
    let gate = tokio::time::timeout(
        Duration::from_secs(1),
        app.instance_manager().lock_instance(id),
    )
    .await?;
    drop(gate);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if app
                .runtime_lifecycle
                .begin_relocation("producer-settled")
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    app.runtime_lifecycle
        .cancel_relocation("producer-settled")?;
    Ok(())
}

#[tokio::test]
async fn task08_round2_normal_producer_returns_and_hands_off() -> anyhow::Result<()> {
    use super::modpack::session::Session;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = task08_round2_change_fixture(&app, "producer-normal").await?;
    let permit = app
        .instance_manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let task = tokio::time::timeout(
        Duration::from_secs(5),
        app.instance_manager()
            .change_modpack(id, task08_round2_pack("B")),
    )
    .await??;
    assert!(
        matches!(app.instance_manager.instances.read().await[&id].data()?.state, run::LaunchState::Queued(current) if current == task)
    );
    let saved = Session::load(&root).await?.unwrap();
    assert_eq!(saved.instance_id, Some(id.0));
    assert_eq!(
        tokio::fs::read_to_string(root.join(".setup/change-pack-version.json")).await?,
        saved.requested_pack.unwrap()
    );
    task08_round2_baseline_recorded(&root).await?;
    assert!(
        app.runtime_lifecycle
            .begin_relocation("normal-handoff")
            .is_err()
    );
    let gate = tokio::time::timeout(
        Duration::from_secs(1),
        app.instance_manager().lock_instance(id),
    )
    .await?;
    drop(gate);
    tokio::fs::write(
        root.join(".setup/setup-session.json"),
        b"controlled invalid session",
    )
    .await?;
    drop(permit);
    task08_round2_settled(&app, id).await
}

async fn task08_round2_producer_io_error(drop_request: bool) -> anyhow::Result<()> {
    use super::modpack::session;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = task08_round2_change_fixture(&app, "producer-io-error").await?;
    let pause = session::publication_fixture::arm(root.join(".setup/setup-session.json"));
    let request = app
        .instance_manager()
        .change_modpack(id, task08_round2_pack("B"));
    let mut request = Box::pin(request);
    tokio::select! {
        _ = pause.entered.notified() => {},
        result = &mut request => panic!("request missed publication: {result:?}"),
        _ = tokio::time::sleep(Duration::from_secs(5)) => panic!("publication wait timed out"),
    }
    assert!(matches!(
        app.instance_manager.instances.read().await[&id]
            .data()?
            .state,
        run::LaunchState::Queued(_)
    ));
    // Real returned publication I/O failure, retaining the original temp evidence.
    tokio::fs::rename(root.join(".setup"), root.join(".setup-injected-failure")).await?;
    tokio::fs::write(root.join(".setup"), b"controlled non-directory").await?;
    if drop_request {
        drop(request);
        assert!(
            app.runtime_lifecycle
                .begin_relocation("io-error-pending")
                .is_err()
        );
        pause.release();
    } else {
        pause.release();
        assert!(
            tokio::time::timeout(Duration::from_secs(5), request)
                .await?
                .is_err()
        );
    }
    task08_round2_settled(&app, id).await?;
    assert_eq!(
        tokio::fs::read(root.join(".setup")).await?,
        b"controlled non-directory"
    );
    assert!(
        !root
            .join(".setup-injected-failure/change-pack-version.json")
            .exists()
    );
    Ok(())
}
#[tokio::test]
async fn task08_round2_returned_io_error_releases_owned_producer() -> anyhow::Result<()> {
    task08_round2_producer_io_error(false).await
}
#[tokio::test]
async fn task08_round2_dropped_request_io_error_settles_owned_producer() -> anyhow::Result<()> {
    task08_round2_producer_io_error(true).await
}

// 2.0.40 leaves `.setup` without a session record: empty after an unfinished
// create or version/loader change, or holding change-pack-version.json (plus
// `repair`) after a pack install, update or repair that stopped before its
// archive finished downloading. None of those touched the instance.

fn legacy_tree(root: &Path) -> std::collections::BTreeMap<String, Option<Vec<u8>>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(|entry| entry.unwrap())
        .map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let bytes = entry
                .file_type()
                .is_file()
                .then(|| std::fs::read(entry.path()).unwrap());
            (relative, bytes)
        })
        .collect()
}

fn legacy_task() -> crate::managers::vtask::VisualTask {
    crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::InstanceTaskPrepare {
            name: "fixture".into(),
        },
    )
}

fn legacy_pack(version: &str) -> info::Modpack {
    info::Modpack::Modrinth(info::ModrinthModpack {
        project_id: "synthetic".into(),
        version_id: version.into(),
    })
}

/// An instance whose `.setup` is exactly what 2.0.40 leaves after an
/// unfinished create or version/loader change: empty, no session record.
async fn legacy_empty_fixture(
    app: &crate::App,
    name: &str,
    fabric: bool,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    let (id, root) = fixture(app, name).await?;
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    if fabric {
        let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
        config.game_configuration.version = Some(GameVersion::Standard(info::StandardVersion {
            release: "1.20.1".into(),
            modloaders: HashSet::from([info::ModLoader {
                type_: info::ModLoaderType::Fabric,
                version: "0.15.11".into(),
            }]),
        }));
        app.instance_manager()
            .persist_config(id, &shortpath, config)
            .await?;
    }
    tokio::fs::create_dir_all(root.join("instance/saves/world")).await?;
    tokio::fs::write(root.join("instance/saves/world/level.dat"), b"world").await?;
    Ok((id, root))
}

/// A pack instance whose 2.0.40 update (or first install, or repair) stopped
/// before the pack archive finished downloading.
async fn legacy_pack_fixture(
    app: &crate::App,
    name: &str,
    installed: bool,
    repair: Option<&super::modpack::RepairMarkerFile>,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    use super::modpack::packinfo;
    let (id, root) = fixture(app, name).await?;
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
    if installed {
        config.modpack = Some(info::ModpackInfo {
            modpack: legacy_pack("A"),
            locked: false,
        });
        config.game_configuration.version = Some(GameVersion::Standard(info::StandardVersion {
            release: "1.20.1".into(),
            modloaders: HashSet::new(),
        }));
        tokio::fs::create_dir_all(root.join("instance/mods")).await?;
        tokio::fs::write(root.join("instance/mods/pack.jar"), b"installed A").await?;
        let record = packinfo::scan_dir(&root.join("instance"), None).await?;
        tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(record)?).await?;
        tokio::fs::write(root.join("instance/mods/user.jar"), b"user").await?;
    } else {
        // A first install from Browse: the pack is configured, the game
        // version is only known once the archive is read.
        config.modpack = Some(info::ModpackInfo {
            modpack: legacy_pack("B"),
            locked: false,
        });
        config.game_configuration.version = None;
    }
    app.instance_manager()
        .persist_config(id, &shortpath, config)
        .await?;
    // 2.0.40's Repair asks for the configured pack again.
    let requested = if repair.is_some() && installed {
        "A"
    } else {
        "B"
    };
    tokio::fs::write(
        root.join(".setup/change-pack-version.json"),
        serde_json::to_vec(&super::modpack::PackVersionFile::from(legacy_pack(
            requested,
        )))?,
    )
    .await?;
    if let Some(options) = repair {
        tokio::fs::write(root.join(".setup/repair"), serde_json::to_vec(options)?).await?;
    }
    tokio::fs::create_dir_all(root.join("instance/saves/world")).await?;
    tokio::fs::write(root.join("instance/saves/world/level.dat"), b"world").await?;
    Ok((id, root))
}

/// What the archive download leaves in `.setup` once it completes.
fn legacy_downloaded_archive(root: &Path, files: &[(&str, &[u8])]) -> anyhow::Result<()> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join(".setup/modrinth"))?);
    zip.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    zip.write_all(br#"{"formatVersion":1,"game":"minecraft","versionId":"synthetic-B","name":"fixture","files":[],"dependencies":{"minecraft":"1.20.1"}}"#)?;
    for (path, bytes) in files {
        zip.start_file(
            format!("overrides/{path}"),
            zip::write::SimpleFileOptions::default(),
        )?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;
    Ok(())
}

/// What Play hands its worker: the owned record, after adopting an untouched
/// 2.0.40 setup with the same Session::adopt_legacy that prepare_game runs
/// under the instance gate. A copied record stays unowned until
/// process_modpack qualifies it.
async fn legacy_play_owner(
    app: &crate::App,
    id: InstanceId,
) -> anyhow::Result<Option<super::modpack::session::Owner>> {
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    let Some(session) =
        super::modpack::session::Session::load_or_adopt_legacy(&root, id, &config).await?
    else {
        return Ok(None);
    };
    session.owned_by(id).then(|| session.owner()).transpose()
}

async fn legacy_prepare(
    app: &crate::App,
    id: InstanceId,
) -> anyhow::Result<Option<super::modpack::session::Owner>> {
    let mut owner = legacy_play_owner(app, id).await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = legacy_task();
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    Ok(owner)
}

/// Runs preparation until it reaches the pack request, then stops it there:
/// proof that qualification let it through, without touching the network.
async fn legacy_reaches_pack_request(app: &crate::App, id: InstanceId) -> anyhow::Result<()> {
    let pause = app
        .instance_manager()
        .test_hooks
        .arm("setup_pack_request", id);
    let mut owner = legacy_play_owner(app, id).await?;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let prepare_app = app.clone();
    let mut preparation = tokio::spawn(async move {
        let task = legacy_task();
        run::modpack::process_modpack(
            prepare_app,
            id,
            true,
            config,
            shortpath,
            &task,
            false,
            &mut owner,
        )
        .await
        .map(|_| ())
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        tokio::select! {
            _ = pause.entered.notified() => {
                preparation.abort();
                Ok(())
            }
            result = &mut preparation => {
                anyhow::bail!("preparation ended before the pack request: {result:?}")
            }
        }
    })
    .await?
}

/// Why an addon edit is refused while the setup waits.
async fn legacy_addon_refusal(app: &crate::App, id: InstanceId) -> String {
    app.instance_manager()
        .ensure_modpack_not_locked(id)
        .await
        .expect_err("addon edits wait for the pending setup")
        .to_string()
}

fn legacy_retired(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut retired = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".setup-cancelled-"))
        {
            retired.push(path);
        }
    }
    Ok(retired)
}

fn legacy_retired_session(retired: &Path) -> anyhow::Result<super::modpack::session::Session> {
    Ok(serde_json::from_slice(&std::fs::read(
        retired.join(super::modpack::session::RECORD),
    )?)?)
}

#[tokio::test]
async fn legacy_empty_setup_installs_as_version_only_with_addons_allowed() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    for fabric in [false, true] {
        let (id, root) = legacy_empty_fixture(
            &app,
            if fabric {
                "legacy-empty-fabric"
            } else {
                "legacy-empty-vanilla"
            },
            fabric,
        )
        .await?;
        tokio::fs::write(root.join("instance/keep.txt"), b"keep").await?;
        let before = tokio::fs::read(root.join("instance.json")).await?;
        // Addon edits before Play are accepted, as in 2.0.40, and the check
        // does not write.
        app.instance_manager().ensure_modpack_not_locked(id).await?;
        assert!(!root.join(".setup/setup-session.json").exists());

        let owner = legacy_prepare(&app, id).await?;
        let session = Session::load(&root).await?.expect("adopted session");
        assert_eq!(session.owner()?, owner.expect("prepare owns the setup"));
        assert_eq!(
            (session.kind, session.producer, session.phase),
            (Kind::VersionOnly, Producer::Create, Phase::Committed)
        );
        assert!(session.requested_pack.is_none());
        assert_eq!(tokio::fs::read(root.join("instance.json")).await?, before);
        assert_eq!(
            tokio::fs::read(root.join("instance/keep.txt")).await?,
            b"keep"
        );
        assert_eq!(
            tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
            b"world"
        );
        assert!(!root.join("packinfo.json").exists());
    }
    Ok(())
}

#[tokio::test]
async fn legacy_empty_setup_version_change_supersedes_it() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_empty_fixture(&app, "legacy-empty-version-change", false).await?;
    // Keep the new install queued: this test stops before any download.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let mut edit = update(id);
    edit.version = Some("1.20.1".into());
    assert!(
        app.instance_manager()
            .update_instance(edit)
            .await?
            .is_some()
    );
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    let old = legacy_retired_session(&retired[0])?;
    assert_eq!(
        (old.kind, old.producer),
        (Kind::VersionOnly, Producer::Create)
    );
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", old.operation))
    );
    let new = Session::load(&root).await?.expect("new settings setup");
    assert_eq!(
        (new.kind, new.producer),
        (Kind::VersionOnly, Producer::Settings)
    );
    assert_eq!(new.owner()?.instance_id, id.0);
    assert_ne!(new.operation, old.operation);
    assert!(
        matches!(disk_config(&root).await?.game_configuration.version, Some(GameVersion::Standard(v)) if v.release == "1.20.1")
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
        b"world"
    );
    Ok(())
}

#[tokio::test]
async fn legacy_play_adopts_under_the_instance_gate() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (empty, empty_root) = legacy_empty_fixture(&app, "legacy-play-empty", true).await?;
    let (pack, pack_root) = legacy_pack_fixture(&app, "legacy-play-pack", true, None).await?;
    // Keep both preparations queued: only qualification is under test.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    for (id, root, expected) in [
        (empty, &empty_root, (Kind::VersionOnly, Producer::Create)),
        (pack, &pack_root, (Kind::Pack, Producer::Change)),
    ] {
        app.instance_manager()
            .prepare_game(id, None, None, false)
            .await?;
        let session = Session::load(root).await?.expect("Play adopts the setup");
        assert_eq!((session.kind, session.producer), expected);
        assert_eq!(session.phase, Phase::Reserved);
        assert_eq!(session.owner()?.instance_id, id.0);
    }
    Ok(())
}

#[tokio::test]
async fn legacy_pending_update_cancels_back_to_installed_version() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_pack_fixture(&app, "legacy-update-cancel", true, None).await?;
    let request = tokio::fs::read(root.join(".setup/change-pack-version.json")).await?;
    let instance_before = legacy_tree(&root.join("instance"));
    let config_before = tokio::fs::read(root.join("instance.json")).await?;
    let record_before = tokio::fs::read(root.join("packinfo.json")).await?;
    // Addons wait for the update, with the same ways out before and after the
    // dialog adopts it; the check itself writes nothing.
    let refusal = legacy_addon_refusal(&app, id).await;
    assert!(refusal.contains("cancel it from Repair"), "{refusal}");
    assert!(!root.join(".setup/setup-session.json").exists());

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.pending);
    assert!(status.can_cancel, "{:?}", status.reason);
    assert_eq!(legacy_addon_refusal(&app, id).await, refusal);
    assert_eq!(status.installed_version.as_deref(), Some("A"));
    assert_eq!(status.pending_version.as_deref(), Some("B"));
    let operation = status.operation.expect("adopted operation");
    app.instance_manager()
        .cancel_pending_setup(id, operation.clone())
        .await?;
    assert!(!root.join(".setup").exists());
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{operation}-"))
    );
    assert_eq!(
        tokio::fs::read(retired[0].join("change-pack-version.json")).await?,
        request
    );

    // The next prepare is the installed version: no setup, no pack request.
    let owner = legacy_prepare(&app, id).await?;
    assert!(owner.is_none());
    assert!(!root.join(".setup").exists());
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    assert_eq!(
        tokio::fs::read(root.join("instance.json")).await?,
        config_before
    );
    assert_eq!(
        tokio::fs::read(root.join("packinfo.json")).await?,
        record_before
    );
    Ok(())
}

/// After Unpair, Repair (and so its Cancel) can't be opened for the
/// instance, so the addon refusal names only Play, before and after Play
/// adopts the update.
#[tokio::test]
async fn legacy_pending_update_of_an_unpaired_pack_points_at_play_not_repair() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_pack_fixture(&app, "legacy-update-unpaired", true, None).await?;
    let mut unpair = update(id);
    unpair.modpack_locked = Some(None);
    app.instance_manager().update_instance(unpair).await?;
    assert!(disk_config(&root).await?.modpack.is_none());
    let setup = legacy_tree(&root.join(".setup"));
    let refusal = legacy_addon_refusal(&app, id).await;
    assert!(!refusal.contains("Repair"), "{refusal}");
    assert!(refusal.contains("Press Play"), "{refusal}");
    assert_eq!(legacy_tree(&root.join(".setup")), setup);
    // Play adopts it as its own record; the way out stays the same.
    assert!(legacy_play_owner(&app, id).await?.is_some());
    assert!(root.join(".setup/setup-session.json").exists());
    let adopted = legacy_addon_refusal(&app, id).await;
    assert!(!adopted.contains("Repair"), "{adopted}");
    assert!(adopted.contains("Press Play"), "{adopted}");
    Ok(())
}

#[tokio::test]
async fn legacy_pending_update_is_reset_by_repair() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_pack_fixture(&app, "legacy-update-repair", true, None).await?;
    let request = tokio::fs::read(root.join(".setup/change-pack-version.json")).await?;
    let instance_before = legacy_tree(&root.join("instance"));
    // Keep the repair queued: this test stops before any download.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    // What the Repair dialog sends for the setup it shows.
    let token = app
        .instance_manager()
        .pending_setup(id)
        .await?
        .discard_token
        .clone();
    app.instance_manager()
        .repair_modpack(id, super::modpack::RepairMarkerFile::default(), token)
        .await?;
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    let old = legacy_retired_session(&retired[0])?;
    assert_eq!((old.kind, old.producer), (Kind::Pack, Producer::Change));
    assert_eq!(
        tokio::fs::read(retired[0].join("change-pack-version.json")).await?,
        request
    );
    let repair = Session::load(&root).await?.expect("repair setup");
    assert_eq!(
        (repair.kind, repair.producer, repair.phase),
        (Kind::Pack, Producer::Repair, Phase::Reserved)
    );
    assert_eq!(repair.owner()?.instance_id, id.0);
    // Repair targets the configured (installed) pack, not the abandoned update.
    let requested: super::modpack::PackVersionFile =
        serde_json::from_str(repair.requested_pack.as_deref().unwrap())?;
    assert_eq!(
        super::modpack::PackProofContext::from(&info::Modpack::from(requested)).version_id,
        "A"
    );
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    Ok(())
}

#[tokio::test]
async fn legacy_pending_repair_keeps_its_options_and_preserves_legacy_cleanup() -> anyhow::Result<()>
{
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let legacy = super::modpack::RepairMarkerFile {
        re_enable_disabled: false,
        cleanup_paths: vec!["/mods/user.jar".into()],
        ..Default::default()
    };
    let (id, root) = legacy_pack_fixture(&app, "legacy-repair", true, Some(&legacy)).await?;
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.can_cancel, "{:?}", status.reason);
    let adopted = Session::load(&root).await?.expect("adopted repair");
    assert_eq!(
        (adopted.kind, adopted.producer, adopted.phase),
        (Kind::Pack, Producer::Repair, Phase::Reserved)
    );
    assert_eq!(adopted.original_options.as_ref(), Some(&legacy));

    // Once the archive download completes, the adopted repair applies.
    legacy_downloaded_archive(&root, &[("mods/pack.jar", b"repaired A")])?;
    let owner = legacy_prepare(&app, id).await?;
    assert_eq!(owner.as_ref(), Some(&adopted.owner()?));
    let session = Session::load(&root).await?.unwrap();
    assert_eq!(session.phase, Phase::Committed);
    assert_eq!(session.original_options.as_ref(), Some(&legacy));
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/pack.jar")).await?,
        b"repaired A"
    );
    // Legacy path-only cleanup has no content proof: kept, not executed.
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/user.jar")).await?,
        b"user"
    );
    assert!(
        tokio::fs::read_to_string(root.join(".install_audit/audit.txt"))
            .await?
            .contains("Legacy cleanup paths preserved")
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
        b"world"
    );
    Ok(())
}

#[tokio::test]
async fn legacy_first_install_cannot_cancel_but_play_continues_it() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    // Also when the user pressed Repair on 2.0.40 after that first install
    // failed, and the repair stopped before its archive arrived too.
    let restarted = super::modpack::RepairMarkerFile::default();
    for (name, repair) in [
        ("legacy-first-install", None),
        ("legacy-first-install-repaired", Some(&restarted)),
    ] {
        let (id, root) = legacy_pack_fixture(&app, name, false, repair).await?;
        // Addons wait for Play, before and after the dialog adopts it, and
        // never mention a Cancel the dialog cannot offer.
        let refusal = legacy_addon_refusal(&app, id).await;
        assert!(refusal.contains("Press Play"), "{name}: {refusal}");
        assert!(!refusal.contains("cancel"), "{name}: {refusal}");
        let status = app.instance_manager().pending_setup(id).await?;
        assert_eq!(legacy_addon_refusal(&app, id).await, refusal, "{name}");
        assert!(!status.can_cancel, "{name}");
        let reason = status.reason.unwrap();
        assert!(reason.contains("first install"), "{name}: {reason}");
        assert!(reason.contains("Press Play"), "{name}: {reason}");
        let operation = status.operation.expect("adopted operation");
        let setup_before = legacy_tree(&root.join(".setup"));
        let refused = app
            .instance_manager()
            .cancel_pending_setup(id, operation)
            .await
            .expect_err("a first install is never cancelled");
        assert!(format!("{refused:#}").contains("Press Play"), "{name}");
        assert_eq!(legacy_tree(&root.join(".setup")), setup_before, "{name}");
        assert!(
            disk_config(&root)
                .await?
                .game_configuration
                .version
                .is_none(),
            "{name}"
        );

        legacy_reaches_pack_request(&app, id).await?;
        let session = Session::load(&root).await?.unwrap();
        assert_eq!(
            (session.kind, session.producer, session.phase),
            (Kind::Pack, Producer::Create, Phase::Reserved),
            "{name}"
        );
        assert_eq!(session.owner()?.instance_id, id.0, "{name}");
        // Play continues the repair 2.0.40 started, with its options.
        assert_eq!(session.original_options.as_ref(), repair, "{name}");
    }
    Ok(())
}

#[tokio::test]
async fn legacy_duplicates_qualify_for_their_own_identity() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let instance_root = |shortpath: &str| {
        app.settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(shortpath)
            .get_root()
    };

    // A duplicate of an empty legacy setup installs like its source.
    let (id, root) = legacy_empty_fixture(&app, "legacy-copy-source", false).await?;
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "legacy copy".into())
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(copy).await?;
    let copy_root = instance_root(&shortpath);
    assert!(Session::load(&copy_root).await?.is_none());
    let owner = legacy_prepare(&app, copy).await?;
    let session = Session::load(&copy_root).await?.unwrap();
    assert_eq!(session.owner()?, owner.unwrap());
    assert_eq!(session.owner()?.instance_id, copy.0);
    assert_eq!(
        (session.kind, session.producer, session.phase),
        (Kind::VersionOnly, Producer::Create, Phase::Committed)
    );
    assert!(Session::load(&root).await?.is_none());

    // A duplicate made after the source's legacy update was adopted asks
    // for the same pack under its own operation.
    let (id, root) = legacy_pack_fixture(&app, "legacy-copy-pack", true, None).await?;
    app.instance_manager().pending_setup(id).await?;
    let source = Session::load(&root).await?.expect("adopted source");
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "legacy pack copy".into())
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(copy).await?;
    let copy_root = instance_root(&shortpath);
    legacy_reaches_pack_request(&app, copy).await?;
    let qualified = Session::load(&copy_root).await?.unwrap();
    assert_eq!(qualified.owner()?.instance_id, copy.0);
    assert_ne!(qualified.operation, source.operation);
    assert_eq!(qualified.adopted_from, Some(source.owner()?));
    assert_eq!(qualified.copied_to, None);
    assert_eq!(qualified.requested_pack, source.requested_pack);
    assert_eq!(
        Session::load(&root).await?.unwrap().operation,
        source.operation
    );

    // So does a duplicate made after a legacy repair was adopted: its legacy
    // options hold no storage bound to the source.
    let legacy = super::modpack::RepairMarkerFile {
        cleanup_paths: vec!["/mods/user.jar".into()],
        ..Default::default()
    };
    let (id, root) = legacy_pack_fixture(&app, "legacy-copy-repair", true, Some(&legacy)).await?;
    app.instance_manager().pending_setup(id).await?;
    let source = Session::load(&root).await?.expect("adopted source repair");
    assert_eq!(source.producer, Producer::Repair);
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "legacy repair copy".into())
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(copy).await?;
    let copy_root = instance_root(&shortpath);
    // Opening the dialog makes the copied repair the copy's own, so the copy
    // can cancel it like its source can.
    let status = app.instance_manager().pending_setup(copy).await?;
    assert!(status.can_cancel, "{:?}", status.reason);
    let operation = status.operation.expect("the copy owns the repair");
    legacy_reaches_pack_request(&app, copy).await?;
    let qualified = Session::load(&copy_root).await?.unwrap();
    assert_eq!(qualified.operation, operation);
    assert_eq!(qualified.owner()?.instance_id, copy.0);
    assert_ne!(qualified.operation, source.operation);
    assert_eq!(qualified.adopted_from, Some(source.owner()?));
    assert_eq!(
        (qualified.kind, qualified.producer, qualified.phase),
        (Kind::Pack, Producer::Repair, Phase::Reserved)
    );
    assert_eq!(qualified.original_options.as_ref(), Some(&legacy));
    assert_eq!(
        Session::load(&root).await?.unwrap().operation,
        source.operation
    );

    // A copied repair whose options are bound to the source's storage is not
    // continued, and Play says the copy is the reason, not an interrupted apply.
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "bound repair copy".into())
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(copy).await?;
    let copy_root = instance_root(&shortpath);
    let mut bound = Session::load(&copy_root).await?.unwrap();
    bound.original_options.as_mut().unwrap().cleanup_batch = Some("batch".into());
    super::modpack::session::atomic_json(
        &copy_root
            .join(".setup")
            .join(super::modpack::session::RECORD),
        &bound,
        false,
    )
    .await?;
    let before = legacy_tree(&copy_root);
    let error = legacy_prepare(&app, copy)
        .await
        .err()
        .expect("a bound copied repair is not continued");
    let message = format!("{error:#}");
    assert!(message.contains("copied"), "{message}");
    assert!(!message.contains("applying"), "{message}");
    let status = app.instance_manager().pending_setup(copy).await?;
    assert!(!status.can_cancel);
    let reason = status.reason.unwrap();
    assert!(reason.contains("copied"), "{reason}");
    let refusal = legacy_addon_refusal(&app, copy).await;
    assert!(refusal.contains("copied"), "{refusal}");
    assert!(!refusal.contains("Press Play"), "{refusal}");
    assert_eq!(legacy_tree(&copy_root), before);
    Ok(())
}

#[tokio::test]
async fn legacy_copied_pending_update_cancels_back_to_the_copy_installed_version()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    // A 2.0.40 update A -> B that stopped before its archive arrived, adopted
    // by the source's dialog, then duplicated as a backup.
    let (id, root) = legacy_pack_fixture(&app, "legacy-copy-cancel", true, None).await?;
    app.instance_manager().pending_setup(id).await?;
    let source = Session::load(&root).await?.expect("adopted source");
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "legacy update copy".into())
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(copy).await?;
    let copy_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    let instance_before = legacy_tree(&copy_root.join("instance"));
    let config_before = tokio::fs::read(copy_root.join("instance.json")).await?;
    // Addons wait for the update, and name both ways out even before the copy
    // owns it. The check does not write.
    let record = tokio::fs::read(copy_root.join(".setup/setup-session.json")).await?;
    let refusal = legacy_addon_refusal(&app, copy).await;
    assert!(refusal.contains("cancel it from Repair"), "{refusal}");
    assert_eq!(
        tokio::fs::read(copy_root.join(".setup/setup-session.json")).await?,
        record
    );

    let status = app.instance_manager().pending_setup(copy).await?;
    assert!(status.can_cancel, "{:?}", status.reason);
    assert_eq!(status.installed_version.as_deref(), Some("A"));
    assert_eq!(status.pending_version.as_deref(), Some("B"));
    let operation = status.operation.expect("the copy owns the update");
    assert_ne!(operation, source.operation);
    let owned = Session::load(&copy_root).await?.unwrap();
    assert_eq!(
        (owned.kind, owned.producer, owned.owner()?.instance_id),
        (Kind::Pack, Producer::Change, copy.0)
    );
    assert_eq!(owned.adopted_from, Some(source.owner()?));
    assert_eq!(owned.copied_to, None);
    let refusal = legacy_addon_refusal(&app, copy).await;
    assert!(refusal.contains("cancel it from Repair"), "{refusal}");

    app.instance_manager()
        .cancel_pending_setup(copy, operation.clone())
        .await?;
    assert!(!copy_root.join(".setup").exists());
    let retired = legacy_retired(&copy_root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{operation}-"))
    );
    // The copy stays on its installed A with no download; the source keeps
    // its own pending update.
    assert!(legacy_prepare(&app, copy).await?.is_none());
    assert!(!copy_root.join(".setup").exists());
    assert_eq!(legacy_tree(&copy_root.join("instance")), instance_before);
    assert_eq!(
        tokio::fs::read(copy_root.join("instance.json")).await?,
        config_before
    );
    assert_eq!(
        Session::load(&root).await?.unwrap().operation,
        source.operation
    );
    Ok(())
}

#[tokio::test]
async fn legacy_partial_apply_still_refuses_play_and_names_repair() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_pack_fixture(&app, "legacy-partial-apply", true, None).await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
    tokio::fs::write(root.join(".setup/staging/instance/mods/pack.jar"), b"B").await?;
    tokio::fs::create_dir(root.join(".setup/modpack-skip-overrides")).await?;
    let before = legacy_tree(&root);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = legacy_task();
    let error = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut None,
    )
    .await
    .err()
    .expect("partial apply evidence is never resumed by Play");
    let message = format!("{error:#}");
    assert!(message.contains("Repair"), "{message}");
    assert!(!message.contains("archive"), "{message}");
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(!status.can_cancel);
    assert!(status.operation.is_none());
    let reason = status.reason.unwrap();
    assert!(reason.contains("interrupted while applying"), "{reason}");
    // Addon edits name the same way out as Play, never a Cancel that the
    // dialog cannot offer.
    let refusal = legacy_addon_refusal(&app, id).await;
    assert!(refusal.contains("Use Repair"), "{refusal}");
    assert!(!refusal.contains("cancel"), "{refusal}");
    assert_eq!(legacy_tree(&root), before);

    // Without a modpack there is no Repair: the refusal says what to delete.
    let (id, root) = legacy_empty_fixture(&app, "legacy-partial-apply-vanilla", false).await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
    tokio::fs::write(root.join(".setup/staging/instance/mods/a.jar"), b"a").await?;
    let before = legacy_tree(&root);
    let refusal = legacy_addon_refusal(&app, id).await;
    assert!(
        refusal.contains("delete the hidden .setup folder"),
        "{refusal}"
    );
    assert!(refusal.contains("press Play"), "{refusal}");
    assert!(!refusal.contains("cancel"), "{refusal}");
    assert_eq!(legacy_tree(&root), before);
    Ok(())
}

/// 2.0.40 writes `.setup/modpack-complete` for a version-only prepare once
/// its (empty) pack step is done, before the Java, Minecraft and loader
/// steps, and importers create it as a directory. Either way only the game
/// version install remains.
async fn legacy_complete_fixture(
    app: &crate::App,
    name: &str,
    import: bool,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    let (id, root) = legacy_empty_fixture(app, name, true).await?;
    if import {
        tokio::fs::create_dir(root.join(".setup/modpack-complete")).await?;
    } else {
        tokio::fs::write(root.join(".setup/modpack-complete"), b"").await?;
    }
    Ok((id, root))
}

#[tokio::test]
async fn legacy_complete_setup_accepts_version_and_addon_changes_before_and_after_play()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (before_play, before_root) =
        legacy_complete_fixture(&app, "legacy-complete-before-play", false).await?;
    let (after_play, after_root) =
        legacy_complete_fixture(&app, "legacy-complete-after-play", false).await?;
    // One Play that got past its pack step and then failed again, at Java or
    // the loader install: the record it leaves is the adopted one.
    let owner = legacy_prepare(&app, after_play).await?;
    let adopted = Session::load(&after_root).await?.expect("Play adopts it");
    assert_eq!(adopted.owner()?, owner.expect("Play owns it"));
    assert_eq!(
        (adopted.kind, adopted.producer, adopted.phase),
        (
            Kind::ImportedComplete,
            Producer::Reconstructed,
            Phase::Committed
        )
    );
    assert!(after_root.join(".setup").exists());

    // Keep the replacing installs queued: this test stops before any download.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    for (id, root) in [(before_play, &before_root), (after_play, &after_root)] {
        // Addon edits are accepted, and the check never writes a record.
        let record = tokio::fs::read(root.join(".setup/setup-session.json"))
            .await
            .ok();
        app.instance_manager().ensure_modpack_not_locked(id).await?;
        assert_eq!(
            tokio::fs::read(root.join(".setup/setup-session.json"))
                .await
                .ok(),
            record
        );
        // Another version replaces the leftover instead of being refused.
        let mut edit = update(id);
        edit.version = Some("1.20.1".into());
        assert!(
            app.instance_manager()
                .update_instance(edit)
                .await?
                .is_some()
        );
        let retired = legacy_retired(root)?;
        assert_eq!(retired.len(), 1);
        let old = legacy_retired_session(&retired[0])?;
        assert_eq!(
            (old.kind, old.producer, old.phase),
            (
                Kind::ImportedComplete,
                Producer::Reconstructed,
                Phase::Committed
            )
        );
        assert!(
            retired[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(".setup-cancelled-{}-", old.operation))
        );
        assert!(retired[0].join("modpack-complete").exists());
        let new = Session::load(root).await?.expect("new settings setup");
        assert_eq!(
            (new.kind, new.producer),
            (Kind::VersionOnly, Producer::Settings)
        );
        assert_eq!(new.owner()?.instance_id, id.0);
        assert_eq!(
            tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
            b"world"
        );
    }
    Ok(())
}

#[tokio::test]
async fn legacy_never_launched_imports_allow_addon_edits_unless_a_pack_waits() -> anyhow::Result<()>
{
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_complete_fixture(&app, "legacy-import", true).await?;
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    assert!(!root.join(".setup/setup-session.json").exists());
    // An import that keeps its platform link writes the link as a pack
    // request beside the marker. 2.0.40 skipped the pack step beside it, and
    // so does Play: addon edits stay allowed for an unlocked pack, before
    // Play and after a Play that failed again at the game install.
    let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
    config.modpack = Some(info::ModpackInfo {
        modpack: legacy_pack("A"),
        locked: false,
    });
    app.instance_manager()
        .persist_config(id, &shortpath, config)
        .await?;
    tokio::fs::write(
        root.join(".setup/change-pack-version.json"),
        serde_json::to_vec(&super::modpack::PackVersionFile::from(legacy_pack("A")))?,
    )
    .await?;
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    assert!(!root.join(".setup/setup-session.json").exists());
    legacy_prepare(&app, id).await?;
    let adopted = Session::load(&root).await?.expect("Play adopts it");
    assert_eq!(
        (adopted.producer, adopted.kind, adopted.phase),
        (
            Producer::Reconstructed,
            Kind::ImportedComplete,
            Phase::Committed
        )
    );
    assert!(adopted.requested_pack.is_some());
    app.instance_manager().ensure_modpack_not_locked(id).await?;

    // A pack request without the marker is a pack that has not arrived yet:
    // its addons wait for it, and the refusal says so.
    let (waiting, waiting_root) =
        legacy_pack_fixture(&app, "legacy-import-waiting", false, None).await?;
    let message = legacy_addon_refusal(&app, waiting).await;
    assert!(message.contains("Press Play"), "{message}");
    assert!(!waiting_root.join(".setup/setup-session.json").exists());
    Ok(())
}

#[tokio::test]
async fn legacy_pending_update_of_an_imported_pack_cancels_back_to_installed_version()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    // 2.0.40 importers know the game version but never write packinfo.json.
    let (id, root) = legacy_pack_fixture(&app, "legacy-imported-update", true, None).await?;
    tokio::fs::remove_file(root.join("packinfo.json")).await?;
    let instance_before = legacy_tree(&root.join("instance"));
    let config_before = tokio::fs::read(root.join("instance.json")).await?;

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.can_cancel, "{:?}", status.reason);
    assert_eq!(status.installed_version.as_deref(), Some("A"));
    assert_eq!(status.pending_version.as_deref(), Some("B"));
    let adopted = Session::load(&root).await?.expect("adopted update");
    assert_eq!(
        (adopted.kind, adopted.producer),
        (Kind::Pack, Producer::Change)
    );
    app.instance_manager()
        .cancel_pending_setup(id, status.operation.expect("adopted operation"))
        .await?;
    assert!(!root.join(".setup").exists());
    assert_eq!(legacy_retired(&root)?.len(), 1);

    // The next prepare is the installed version A: no setup, no download.
    assert!(legacy_prepare(&app, id).await?.is_none());
    assert!(!root.join(".setup").exists());
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    assert_eq!(
        tokio::fs::read(root.join("instance.json")).await?,
        config_before
    );
    assert!(!root.join("packinfo.json").exists());
    Ok(())
}

#[tokio::test]
async fn legacy_pending_reasons_point_to_play_for_shapes_play_resumes() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    // Adopted shapes: an unfinished version install of either kind.
    for (name, complete) in [
        ("legacy-reason-empty", false),
        ("legacy-reason-complete", true),
    ] {
        let (id, _) = if complete {
            legacy_complete_fixture(&app, name, false).await?
        } else {
            legacy_empty_fixture(&app, name, false).await?
        };
        let status = app.instance_manager().pending_setup(id).await?;
        assert!(status.operation.is_some(), "{name}");
        assert!(!status.can_cancel, "{name}");
        let reason = status.reason.unwrap();
        assert!(reason.contains("Press Play"), "{name}: {reason}");
        assert!(!reason.contains("first install"), "{name}: {reason}");
    }
    // A pack update whose archive arrived before mod downloads, Java or the
    // loader failed: Play reconstructs it from the archive.
    let (id, root) = legacy_pack_fixture(&app, "legacy-reason-archive", true, None).await?;
    legacy_downloaded_archive(&root, &[])?;
    let before = legacy_tree(&root);
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.operation.is_none());
    assert!(!status.can_cancel);
    let reason = status.reason.unwrap();
    assert!(reason.contains("Press Play"), "{reason}");
    assert!(!reason.contains("interrupted"), "{reason}");
    assert_eq!(legacy_tree(&root), before);
    Ok(())
}

/// Play through the real preparation worker: prepare_game adopts the
/// leftover under the instance gate and the worker settles it, stopping
/// before the version download.
async fn legacy_play_settles_through_the_worker(complete: bool) -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = if complete {
        legacy_complete_fixture(&app, "legacy-worker-complete", false).await?
    } else {
        legacy_empty_fixture(&app, "legacy-worker-empty", true).await?
    };
    let staged = app.instance_manager().test_hooks.arm("setup_staged", id);
    app.instance_manager()
        .prepare_game(id, None, None, false)
        .await?;
    tokio::time::timeout(Duration::from_secs(30), staged.entered.notified()).await?;
    let session = Session::load(&root).await?.expect("Play adopted the setup");
    assert_eq!(session.owner()?.instance_id, id.0);
    assert_eq!(
        (session.kind, session.producer, session.phase),
        if complete {
            (
                Kind::ImportedComplete,
                Producer::Reconstructed,
                Phase::Committed,
            )
        } else {
            (Kind::VersionOnly, Producer::Create, Phase::Committed)
        }
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
        b"world"
    );
    Ok(())
}

#[tokio::test]
async fn legacy_empty_setup_settles_through_the_preparation_worker() -> anyhow::Result<()> {
    legacy_play_settles_through_the_worker(false).await
}

#[tokio::test]
async fn legacy_complete_setup_settles_through_the_preparation_worker() -> anyhow::Result<()> {
    legacy_play_settles_through_the_worker(true).await
}

// A failed or cancelled version-only install (a new instance's first install,
// a version or loader change in Edit, or an imported instance's first Play)
// keeps its setup until that exact version installs. The setup never holds
// pack content, so choosing another version replaces it, as in 2.0.40, and
// addon edits stay allowed beside it. Pack setups and anything that cannot
// be verified still wait, and say how to finish them.

fn versiononly_loader(type_: info::ModLoaderType, version: &str) -> info::ModLoader {
    info::ModLoader {
        type_,
        version: version.into(),
    }
}

/// A new 1.20.1 instance (with `loader`, if any) as create_instance leaves
/// it before its first prepare, with a world to keep.
async fn versiononly_new(
    app: &crate::App,
    name: &str,
    loader: Option<info::ModLoader>,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    let group = app.instance_manager().get_default_group().await?;
    let id = app
        .instance_manager()
        .create_instance(
            group,
            name.into(),
            false,
            InstanceVersionSource::Version(GameVersion::Standard(info::StandardVersion {
                release: "1.20.1".into(),
                modloaders: loader.into_iter().collect(),
            })),
            String::new(),
        )
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(id).await?;
    let root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    tokio::fs::create_dir_all(root.join("instance/saves/world")).await?;
    tokio::fs::write(root.join("instance/saves/world/level.dat"), b"world").await?;
    Ok((id, root))
}

/// An installed instance: its first install finished and removed `.setup`.
async fn versiononly_installed(
    app: &crate::App,
    name: &str,
    loader: Option<info::ModLoader>,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    let (id, root) = versiononly_new(app, name, loader).await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    Ok((id, root))
}

/// Holds the one-slot install queue, so an admitted preparation waits there.
async fn versiononly_hold_queue(
    app: &crate::App,
) -> anyhow::Result<tokio::sync::SemaphorePermit<'_>> {
    Ok(app
        .instance_manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?)
}

fn versiononly_edit(
    id: InstanceId,
    version: Option<&str>,
    loader: Option<Option<info::ModLoader>>,
) -> domain::InstanceSettingsUpdate {
    let mut edit = update(id);
    edit.version = version.map(Into::into);
    edit.modloader = loader;
    edit
}

/// Fails the preparation waiting in the install queue the way a dropped
/// connection or a refused download fails it, then takes the injected cause
/// away again. With `after_staging`, preparation first gets past staging, as
/// it does before a Java, Minecraft or loader install fails. What is left is
/// exactly the setup such a failure leaves.
async fn versiononly_fail(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
    queue: tokio::sync::SemaphorePermit<'_>,
    after_staging: bool,
) -> anyhow::Result<super::modpack::session::Session> {
    use super::modpack::session::{Phase, Session};
    let injected = root.join(".setup/change-pack-version.json");
    // A linked import's own pack request comes back afterwards.
    let request = match tokio::fs::read(&injected).await {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    tokio::fs::write(&injected, b"injected failure").await?;
    drop(queue);
    task08_round2_settled(app, id).await?;
    match request {
        Some(bytes) => tokio::fs::write(&injected, bytes).await?,
        None => tokio::fs::remove_file(&injected).await?,
    }
    if after_staging {
        legacy_prepare(app, id).await?;
    }
    let failed = Session::load(root)
        .await?
        .expect("a failed install keeps its setup");
    assert_eq!(failed.owner()?.instance_id, id.0);
    if after_staging {
        assert_eq!(failed.phase, Phase::Committed);
    }
    Ok(failed)
}

/// Changes the version or loader of `id` while `old` is pending, and checks
/// the change replaces it: the old setup is kept as
/// `.setup-cancelled-<operation>-*`, a fresh Settings setup is admitted under
/// the same gate (so the next prepare is a first install again), and that
/// prepare gets past staging to the phase `.setup` is removed from once the
/// game version installs.
async fn versiononly_supersede(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
    old: &super::modpack::session::Session,
    edit: domain::InstanceSettingsUpdate,
) -> anyhow::Result<info::Instance> {
    use super::modpack::session::{Kind, Phase, Producer, RECORD, Session};
    let queue = versiononly_hold_queue(app).await?;
    let task = app
        .instance_manager()
        .update_instance(edit)
        .await?
        .expect("a version change prepares the instance");
    let retired = legacy_retired(root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", old.operation))
    );
    assert_eq!(
        serde_json::to_value(legacy_retired_session(&retired[0])?)?,
        serde_json::to_value(old)?
    );
    let new = Session::load(root).await?.expect("the replacing setup");
    assert_eq!(
        (new.producer, new.kind, new.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    assert_eq!(new.owner()?.instance_id, id.0);
    assert_ne!(new.operation, old.operation);
    assert_eq!(
        new.accepted_preparation.as_ref().map(|bound| bound.task_id),
        Some(task.0)
    );
    // Nothing of the old setup is carried over.
    let entries = std::fs::read_dir(root.join(".setup"))?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<anyhow::Result<Vec<_>>>()?;
    assert_eq!(entries, [RECORD]);
    assert_eq!(
        tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
        b"world"
    );
    let config = disk_config(root).await?;

    let staged = app.instance_manager().test_hooks.arm("setup_staged", id);
    drop(queue);
    tokio::time::timeout(Duration::from_secs(30), staged.entered.notified()).await?;
    let prepared = Session::load(root).await?.expect("the replacing setup");
    assert_eq!(prepared.operation, new.operation);
    assert_eq!(prepared.phase, Phase::Committed);
    Ok(config)
}

/// A version or loader change that the pending setup holds back: refused
/// before anything is written, with a reason.
async fn versiononly_refused(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
    mut edit: domain::InstanceSettingsUpdate,
) -> anyhow::Result<String> {
    // A rename sent with the version change waits too.
    edit.name = Some(format!("renamed {}", *id));
    let config = tokio::fs::read(root.join("instance.json")).await?;
    let setup = legacy_tree(&root.join(".setup"));
    let error = app
        .instance_manager()
        .update_instance(edit)
        .await
        .err()
        .expect("the version change waits for the pending setup");
    assert_eq!(tokio::fs::read(root.join("instance.json")).await?, config);
    assert_eq!(legacy_tree(&root.join(".setup")), setup);
    assert!(legacy_retired(root)?.is_empty());
    let message = error.to_string();
    assert!(
        message.contains("changing the game version or loader")
            || message.contains("the game version or loader can't be changed"),
        "{message}"
    );
    Ok(message)
}

async fn versiononly_failed_loader_change_is_replaced(after_staging: bool) -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_installed(&app, "versiononly-settings", None).await?;
    let forge = versiononly_loader(info::ModLoaderType::Forge, "47.2.0");
    let queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(forge))))
        .await?
        .expect("the loader change prepares the instance");
    let old = versiononly_fail(&app, id, &root, queue, after_staging).await?;
    assert_eq!(
        (old.producer, old.kind),
        (Producer::Settings, Kind::VersionOnly)
    );

    // Addon edits stay allowed while the change waits.
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    // It still cannot be cancelled on its own: nothing earlier to go back to.
    let status = app.instance_manager().pending_setup(id).await?;
    assert_eq!(status.operation.as_deref(), Some(old.operation.as_str()));
    assert!(!status.can_cancel);
    let reason = status.reason.unwrap();
    assert!(reason.contains("Press Play"), "{reason}");
    let setup = legacy_tree(&root.join(".setup"));
    app.instance_manager()
        .cancel_pending_setup(id, old.operation.clone())
        .await
        .expect_err("a version-only setup is not cancelled");
    assert_eq!(legacy_tree(&root.join(".setup")), setup);

    // Going back to vanilla replaces it.
    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, None, Some(None)),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version))
            if version.release == "1.20.1" && version.modloaders.is_empty()
    ));
    Ok(())
}

#[tokio::test]
async fn versiononly_loader_change_that_failed_before_staging_is_replaced() -> anyhow::Result<()> {
    versiononly_failed_loader_change_is_replaced(false).await
}

#[tokio::test]
async fn versiononly_loader_change_that_failed_after_staging_is_replaced() -> anyhow::Result<()> {
    versiononly_failed_loader_change_is_replaced(true).await
}

#[tokio::test]
async fn versiononly_cancelled_loader_change_is_replaceable() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_installed(&app, "versiononly-cancelled", None).await?;
    let queue = versiononly_hold_queue(&app).await?;
    let forge = versiononly_loader(info::ModLoaderType::Forge, "47.2.0");
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(forge))))
        .await?
        .expect("the loader change prepares the instance");
    // Quitting cancels the queued install; nothing else can cancel it.
    app.runtime_lifecycle.begin_shutdown(
        "versiononly-cancel",
        crate::managers::runtime_lifecycle::ShutdownPurpose::Quit,
    )?;
    task08_round2_wait_failed(&app, id).await?;
    drop(queue);
    let cancelled = Session::load(&root)
        .await?
        .expect("a cancelled install keeps its setup");
    assert_eq!(
        (cancelled.producer, cancelled.kind, cancelled.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    // After the restart, another version may replace it and addons can change.
    assert!(cancelled.supersedable(&root, id).await?);
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    Ok(())
}

#[tokio::test]
async fn versiononly_failed_first_install_is_replaced_by_another_loader_version()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer};
    let app = crate::setup_managers_for_test().await;
    let forge = versiononly_loader(info::ModLoaderType::Forge, "47.2.0");
    let (id, root) = versiononly_new(&app, "versiononly-create", Some(forge)).await?;
    let queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .prepare_game(id, None, None, false)
        .await?;
    let old = versiononly_fail(&app, id, &root, queue, true).await?;
    assert_eq!(
        (old.producer, old.kind),
        (Producer::Create, Kind::VersionOnly)
    );
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(!status.can_cancel);
    let reason = status.reason.unwrap();
    assert!(reason.contains("Press Play"), "{reason}");
    assert!(reason.contains("Edit"), "{reason}");
    let setup = legacy_tree(&root.join(".setup"));
    app.instance_manager()
        .cancel_pending_setup(id, old.operation.clone())
        .await
        .expect_err("a first install is not cancelled");
    assert_eq!(legacy_tree(&root.join(".setup")), setup);

    let other = versiononly_loader(info::ModLoaderType::Forge, "47.3.0");
    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, None, Some(Some(other.clone()))),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.modloaders == HashSet::from([other])
    ));
    Ok(())
}

#[tokio::test]
async fn versiononly_failed_import_install_is_replaced_by_another_version() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer};
    let app = crate::setup_managers_for_test().await;
    let group = app.instance_manager().get_default_group().await?;
    let fabric = versiononly_loader(info::ModLoaderType::Fabric, "0.15.11");
    // What every launcher importer leaves: the files copied in and only the
    // game version install to go.
    let id = app
        .instance_manager()
        .create_instance_ext(
            group,
            "versiononly-import".into(),
            None,
            None,
            None,
            InstanceVersionSource::Version(GameVersion::Standard(info::StandardVersion {
                release: "1.20.1".into(),
                modloaders: HashSet::from([fabric]),
            })),
            String::new(),
            |path| async move {
                tokio::fs::create_dir_all(path.join(".setup/modpack-complete")).await?;
                tokio::fs::create_dir_all(path.join("instance/saves/world")).await?;
                tokio::fs::write(path.join("instance/saves/world/level.dat"), b"world").await?;
                tokio::fs::create_dir_all(path.join("instance/config")).await?;
                tokio::fs::write(path.join("instance/config/imported.toml"), b"imported").await?;
                Ok(())
            },
        )
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(id).await?;
    let root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    let queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .prepare_game(id, None, None, false)
        .await?;
    let old = versiononly_fail(&app, id, &root, queue, false).await?;
    assert_eq!(
        (old.producer, old.kind, old.phase),
        (Producer::Import, Kind::ImportedComplete, Phase::Committed)
    );
    app.instance_manager().ensure_modpack_not_locked(id).await?;

    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, Some("1.20.4"), None),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.release == "1.20.4"
    ));
    assert!(legacy_retired(&root)?[0].join("modpack-complete").is_dir());
    assert_eq!(
        tokio::fs::read(root.join("instance/config/imported.toml")).await?,
        b"imported"
    );
    Ok(())
}

#[tokio::test]
async fn versiononly_adopted_2040_leftover_that_failed_again_is_replaced() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_complete_fixture(&app, "versiononly-leftover", false).await?;
    // Play adopts it and fails again at the same install.
    let queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .prepare_game(id, None, None, false)
        .await?;
    let old = versiononly_fail(&app, id, &root, queue, false).await?;
    assert_eq!(
        (old.producer, old.kind, old.phase),
        (
            Producer::Reconstructed,
            Kind::ImportedComplete,
            Phase::Committed
        )
    );
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, Some("1.20.4"), None),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.release == "1.20.4"
    ));
    Ok(())
}

/// After a replace, `.setup` is a fresh directory, so the next prepare takes
/// the first-install path: loader processors run even when their shared
/// outputs already exist (process_minecraft reads `is_setup` the same way).
#[tokio::test]
async fn versiononly_prepare_after_a_replace_is_a_first_install() -> anyhow::Result<()> {
    use super::modpack::session::Session;
    let app = crate::setup_managers_for_test().await;
    let neoforge = versiononly_loader(info::ModLoaderType::Neoforge, "47.1.106");
    let (id, root) = versiononly_installed(&app, "versiononly-processors", Some(neoforge)).await?;
    let queue = versiononly_hold_queue(&app).await?;
    let forge = versiononly_loader(info::ModLoaderType::Forge, "47.2.0");
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(forge))))
        .await?
        .expect("the loader change prepares the instance");
    versiononly_fail(&app, id, &root, queue, true).await?;

    // Back to vanilla: without the setup, neither processor step would be
    // created for a vanilla target.
    let _queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(None)))
        .await?
        .expect("the loader change prepares the instance");
    let new = Session::load(&root).await?.expect("the replacing setup");
    let mut owner = Some(new.owner()?);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = legacy_task();
    let (subtasks, _, _, _) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut owner,
    )
    .await?;
    assert!(root.join(".setup").is_dir());
    assert!(subtasks.t_forge_processors.is_some());
    assert!(subtasks.t_neoforge_processors.is_some());
    Ok(())
}

/// A shader-loader retry captured for the failed setup cannot adopt the one
/// that replaced it.
#[tokio::test]
async fn versiononly_shader_retry_of_a_replaced_setup_fails_cleanly() -> anyhow::Result<()> {
    use super::modpack::session::RECORD;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_installed(&app, "versiononly-shader", None).await?;
    let queue = versiononly_hold_queue(&app).await?;
    let fabric = versiononly_loader(info::ModLoaderType::Fabric, "0.15.11");
    let first = app
        .instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(fabric))))
        .await?
        .expect("the loader change prepares the instance");
    versiononly_fail(&app, id, &root, queue, false).await?;

    let queue = versiononly_hold_queue(&app).await?;
    let other = versiononly_loader(info::ModLoaderType::Fabric, "0.16.0");
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(other))))
        .await?
        .expect("the loader change prepares the instance");
    versiononly_fail(&app, id, &root, queue, false).await?;
    let record = tokio::fs::read(root.join(".setup").join(RECORD)).await?;
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    let retired_record = tokio::fs::read(retired[0].join(RECORD)).await?;

    app.instance_manager()
        .resume_shader_modloader(id, first, "1.20.1".into())
        .await
        .err()
        .expect("the retry belongs to the replaced setup");
    assert_eq!(
        tokio::fs::read(root.join(".setup").join(RECORD)).await?,
        record
    );
    assert_eq!(legacy_retired(&root)?, retired);
    assert_eq!(
        tokio::fs::read(retired[0].join(RECORD)).await?,
        retired_record
    );
    assert!(matches!(
        app.instance_manager.instances.read().await[&id]
            .data()?
            .state,
        run::LaunchState::Inactive {
            failed_task: Some(_)
        }
    ));
    Ok(())
}

#[tokio::test]
async fn versiononly_version_change_still_waits_for_pack_and_unverifiable_setups()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let edit = |id| versiononly_edit(id, Some("1.20.4"), None);
    let record = |root: &Path,
                  config: &info::Instance,
                  producer: Producer,
                  kind: Kind,
                  phase: Phase,
                  owner: Option<i32>|
     -> anyhow::Result<Session> {
        let mut session = Session::new(config, producer, kind, None)?;
        session.instance_id = owner;
        session.phase = phase;
        std::fs::create_dir_all(root.join(".setup"))?;
        std::fs::write(
            root.join(".setup").join(super::modpack::session::RECORD),
            serde_json::to_vec(&session)?,
        )?;
        Ok(session)
    };

    // A pack update that has not started applying names Cancel; one that may
    // have started names only Play. So does one whose pack the user unpaired,
    // since Repair can't be opened without a pack.
    for (phase, linked, cancellable) in [
        (Phase::Resolved, true, true),
        (Phase::Applying, true, false),
        (Phase::Resolved, false, false),
    ] {
        let (id, root) =
            versiononly_installed(&app, &format!("versiononly-pack-{phase:?}-{linked}"), None)
                .await?;
        let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
        if linked {
            config.modpack = Some(info::ModpackInfo {
                modpack: legacy_pack("A"),
                locked: false,
            });
            app.instance_manager()
                .persist_config(id, &shortpath, config.clone())
                .await?;
        }
        record(
            &root,
            &config,
            Producer::Change,
            Kind::Pack,
            phase,
            Some(id.0),
        )?;
        let message = versiononly_refused(&app, id, &root, edit(id)).await?;
        assert_eq!(
            message.contains("cancel it from Repair"),
            cancellable,
            "{phase:?}, linked {linked}: {message}"
        );
        assert!(
            message.contains("Press Play"),
            "{phase:?}, linked {linked}: {message}"
        );
    }

    // A version-only setup beside pack evidence is not replaced.
    for evidence in ["apply-started", "staging-packinfo.json", "modrinth"] {
        let (id, root) =
            versiononly_installed(&app, &format!("versiononly-evidence-{evidence}"), None).await?;
        let (_, config) = app.instance_manager().current_config(id).await?;
        record(
            &root,
            &config,
            Producer::Settings,
            Kind::VersionOnly,
            Phase::Committed,
            Some(id.0),
        )?;
        tokio::fs::write(root.join(".setup").join(evidence), b"").await?;
        let message = versiononly_refused(&app, id, &root, edit(id)).await?;
        assert!(message.contains("Press Play"), "{evidence}: {message}");
    }

    // A pack whose installed record is no longer the target it promoted.
    let (id, root) = versiononly_installed(&app, "versiononly-pack-record-changed", None).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let mut changed = record(
        &root,
        &config,
        Producer::Import,
        Kind::Pack,
        Phase::Committed,
        Some(id.0),
    )?;
    let target = super::modpack::packinfo::PackInfo {
        files: HashMap::new(),
    };
    let (digest, options_digest) = super::modpack::session::binding(&target, None)?;
    changed.pending_config = Some(changed.installed_config.clone());
    changed.target = Some(super::modpack::session::Target {
        packinfo: super::modpack::packinfo::make_packinfo(target)?,
        digest,
        options_digest,
        sources: Default::default(),
        skipped: Vec::new(),
    });
    std::fs::write(
        root.join(".setup").join(super::modpack::session::RECORD),
        serde_json::to_vec(&changed)?,
    )?;
    let installed = super::modpack::packinfo::PackInfo {
        files: HashMap::from([(
            "/mods/other.jar".to_string(),
            super::modpack::packinfo::FileHashes {
                sha512: [3; 64],
                md5: [3; 16],
            },
        )]),
    };
    tokio::fs::write(
        root.join("packinfo.json"),
        super::modpack::packinfo::make_packinfo(installed)?,
    )
    .await?;
    let message = versiononly_refused(&app, id, &root, edit(id)).await?;
    assert!(message.contains("Press Play"), "{message}");

    // A copy whose setup may have started applying its files on the original
    // is not continued, so the copy cannot replace it either.
    let (id, root) = versiononly_installed(&app, "versiononly-copied-apply", None).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    record(
        &root,
        &config,
        Producer::Settings,
        Kind::VersionOnly,
        Phase::Committed,
        Some(id.0),
    )?;
    tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "versiononly copy apply".into())
        .await?;
    let (shortpath, _) = app.instance_manager().current_config(copy).await?;
    let copy_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root();
    let config = tokio::fs::read(copy_root.join("instance.json")).await?;
    let setup = legacy_tree(&copy_root.join(".setup"));
    let message = app
        .instance_manager()
        .update_instance(edit(copy))
        .await
        .err()
        .expect("the copy cannot replace a setup it cannot continue")
        .to_string();
    assert!(message.contains("copied"), "{message}");
    assert_eq!(
        tokio::fs::read(copy_root.join("instance.json")).await?,
        config
    );
    assert_eq!(legacy_tree(&copy_root.join(".setup")), setup);
    assert!(legacy_retired(&copy_root)?.is_empty());

    // 2.0.40 leftovers that may have applied part of their files.
    let (id, root) = legacy_empty_fixture(&app, "versiononly-legacy-apply", false).await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
    tokio::fs::write(root.join(".setup/staging/instance/mods/a.jar"), b"a").await?;
    let message = versiononly_refused(&app, id, &root, edit(id)).await?;
    assert!(
        message.contains("delete the hidden .setup folder"),
        "{message}"
    );
    let (id, root) = legacy_pack_fixture(&app, "versiononly-legacy-pack", true, None).await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
    tokio::fs::write(root.join(".setup/staging/instance/mods/pack.jar"), b"B").await?;
    let message = versiononly_refused(&app, id, &root, edit(id)).await?;
    assert!(message.contains("Use Repair"), "{message}");
    assert!(!message.contains("cancel"), "{message}");
    Ok(())
}

/// The root of `id`'s instance folder.
async fn versiononly_root(app: &crate::App, id: InstanceId) -> anyhow::Result<PathBuf> {
    let (shortpath, _) = app.instance_manager().current_config(id).await?;
    Ok(app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&shortpath)
        .get_root())
}

/// A failed version-only install that a duplicate copied is continued as the
/// copy's own, as Play would continue it, so another version replaces it
/// directly: the copy does not have to repeat the install that failed on the
/// original first. The original keeps its own setup.
#[tokio::test]
async fn versiononly_copied_failed_install_is_continued_and_replaced() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_installed(&app, "versiononly-copy-source", None).await?;
    let queue = versiononly_hold_queue(&app).await?;
    let forge = versiononly_loader(info::ModLoaderType::Forge, "47.2.0");
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(forge))))
        .await?
        .expect("the loader change prepares the instance");
    let failed = versiononly_fail(&app, id, &root, queue, true).await?;
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "versiononly copy".into())
        .await?;
    let copy_root = versiononly_root(&app, copy).await?;
    let copied = Session::load(&copy_root).await?.expect("the copied setup");
    assert_eq!(copied.owner()?, failed.owner()?);
    assert_eq!(copied.copied_to, Some(copy.0));
    let source = legacy_tree(&root.join(".setup"));
    // Addon edits stay allowed on the copy, as on the original.
    app.instance_manager()
        .ensure_modpack_not_locked(copy)
        .await?;

    let _queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .update_instance(versiononly_edit(copy, None, Some(None)))
        .await?
        .expect("the loader change prepares the copy");
    let retired = legacy_retired(&copy_root)?;
    assert_eq!(retired.len(), 1);
    let continued = legacy_retired_session(&retired[0])?;
    assert_eq!(continued.instance_id, Some(copy.0));
    assert_eq!(continued.copied_to, None);
    assert_eq!(continued.adopted_from, Some(failed.owner()?));
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", continued.operation))
    );
    let new = Session::load(&copy_root)
        .await?
        .expect("the replacing setup");
    assert_eq!(
        (new.producer, new.kind, new.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    assert_eq!(new.owner()?.instance_id, copy.0);
    assert!(matches!(
        disk_config(&copy_root).await?.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.modloaders.is_empty()
    ));
    assert_eq!(legacy_tree(&root.join(".setup")), source);
    assert!(legacy_retired(&root)?.is_empty());
    Ok(())
}

/// The same for a record written before the instance had an id: it becomes
/// the instance's own, under the operation it already had, then is replaced.
#[tokio::test]
async fn versiononly_unbound_failed_install_is_continued_and_replaced() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, RECORD, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_new(&app, "versiononly-unbound", None).await?;
    let mut unbound = Session::load(&root).await?.expect("the create setup");
    unbound.instance_id = None;
    std::fs::write(
        root.join(".setup").join(RECORD),
        serde_json::to_vec(&unbound)?,
    )?;
    app.instance_manager().ensure_modpack_not_locked(id).await?;

    let _queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .update_instance(versiononly_edit(id, Some("1.20.4"), None))
        .await?
        .expect("the version change prepares the instance");
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", unbound.operation))
    );
    let continued = legacy_retired_session(&retired[0])?;
    assert_eq!(continued.operation, unbound.operation);
    assert_eq!(continued.instance_id, Some(id.0));
    let new = Session::load(&root).await?.expect("the replacing setup");
    assert_eq!(
        (new.producer, new.kind, new.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    assert_eq!(new.owner()?.instance_id, id.0);
    Ok(())
}

/// A new instance's folder copied outside the launcher is registered by the
/// next scan under a new id, and its unfinished first install still names the
/// id it was created under, as after a database reset. Play, addon edits and
/// the Repair dialog continue it as the new id's own, as they do for a
/// duplicate's. Play used to refuse it with "foreign pending setup has no copy
/// provenance; original inputs retained", and edits told the user to finish
/// it on the original instance.
#[tokio::test]
async fn versiononly_setup_of_a_folder_copied_outside_is_played_as_its_own() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_new(&app, "versiononly-outside-source", None).await?;
    let original = Session::load(&root).await?.expect("the create setup");
    assert_eq!(
        (original.producer, original.kind),
        (Producer::Create, Kind::VersionOnly)
    );
    let copy_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path()
        .join("versiononly-outside-copy");
    copy_tree(&root, &copy_root)?;
    app.instance_manager().scan_instances().await?;
    let copy = InstanceId(
        instance_repo::get_instance_by_shortpath(&app.db, "versiononly-outside-copy")
            .await?
            .context("the scan registers the copied folder")?
            .id,
    );
    assert_ne!(copy, id);
    let copied = Session::load(&copy_root).await?.expect("the copied setup");
    assert_eq!(copied.owner()?, original.owner()?);
    assert_eq!(copied.copied_to, None);
    let source = legacy_tree(&root.join(".setup"));

    app.instance_manager()
        .ensure_modpack_not_locked(copy)
        .await?;
    let status = app.instance_manager().pending_setup(copy).await?;
    assert!(status.can_resume);
    let reason = status.reason.unwrap();
    assert!(reason.contains("Press Play"), "{reason}");

    let queue = versiononly_hold_queue(&app).await?;
    let staged = app.instance_manager().test_hooks.arm("setup_staged", copy);
    app.instance_manager()
        .prepare_game(copy, None, None, false)
        .await?;
    drop(queue);
    tokio::time::timeout(Duration::from_secs(30), staged.entered.notified()).await?;
    let continued = Session::load(&copy_root)
        .await?
        .expect("the continued setup");
    assert!(continued.owned_by(copy));
    assert_eq!(continued.adopted_from, Some(original.owner()?));
    assert_eq!(continued.phase, Phase::Committed);
    // The original keeps its own setup.
    assert_eq!(legacy_tree(&root.join(".setup")), source);
    Ok(())
}

/// A database reset gives an instance's folder a new id. An unfinished
/// version-only install that names the old one becomes the new id's own, and
/// another version replaces it. The change used to be refused with "This
/// instance was copied ... Finish the setup on the original instance".
#[tokio::test]
async fn versiononly_renumbered_failed_install_is_continued_and_replaced() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, RECORD, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_new(&app, "versiononly-renumbered", None).await?;
    let mut renumbered = Session::load(&root).await?.expect("the create setup");
    renumbered.instance_id = Some(id.0 + 1000);
    std::fs::write(
        root.join(".setup").join(RECORD),
        serde_json::to_vec(&renumbered)?,
    )?;
    app.instance_manager().ensure_modpack_not_locked(id).await?;

    let _queue = versiononly_hold_queue(&app).await?;
    app.instance_manager()
        .update_instance(versiononly_edit(id, Some("1.20.4"), None))
        .await?
        .expect("the version change prepares the instance");
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    let continued = legacy_retired_session(&retired[0])?;
    assert!(continued.owned_by(id));
    assert_eq!(continued.adopted_from, Some(renumbered.owner()?));
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", continued.operation))
    );
    let new = Session::load(&root).await?.expect("the replacing setup");
    assert_eq!(
        (new.producer, new.kind, new.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    assert_eq!(new.owner()?.instance_id, id.0);
    assert!(matches!(
        disk_config(&root).await?.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.release == "1.20.4"
    ));
    Ok(())
}

/// A record from an earlier registration of the folder that Play can't
/// continue (apply evidence beside it, and no archive to rebuild it from) is
/// refused by Play, Edit and addon changes alike, naming the way out the
/// instance has and without claiming it was copied. Nothing is written. Play
/// used to fail with "foreign pending setup has no copy provenance; original
/// inputs retained", and Edit and addon changes pointed at an original
/// instance that a database reset leaves none of.
#[tokio::test]
async fn versiononly_renumbered_setup_with_apply_evidence_names_a_way_out() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, RECORD, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = versiononly_installed(&app, "versiononly-renumbered-apply", None).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let mut session = Session::new(&config, Producer::Settings, Kind::VersionOnly, None)?;
    session.instance_id = Some(id.0 + 1000);
    session.phase = Phase::Committed;
    tokio::fs::create_dir_all(root.join(".setup")).await?;
    std::fs::write(
        root.join(".setup").join(RECORD),
        serde_json::to_vec(&session)?,
    )?;
    tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
    let setup = legacy_tree(&root.join(".setup"));
    let way_out = "delete the hidden .setup folder";

    let message =
        versiononly_refused(&app, id, &root, versiononly_edit(id, Some("1.20.4"), None)).await?;
    assert!(message.contains(way_out), "{message}");
    assert!(message.contains("database was reset"), "{message}");
    assert!(!message.contains("copied while"), "{message}");
    let message = legacy_addon_refusal(&app, id).await;
    assert!(message.contains(way_out), "{message}");
    assert!(!message.contains("copied while"), "{message}");

    let (play, task) = app
        .instance_manager()
        .prepare_game(id, None, None, false)
        .await?;
    tokio::time::timeout(Duration::from_secs(30), play).await??;
    let crate::managers::vtask::TaskOutcome::Failed(failed) =
        app.task_manager().get_task_outcome(task).await
    else {
        panic!("Play can't continue the setup");
    };
    let crate::domain::vtask::Progress::Failed(error) = &failed.progress else {
        unreachable!()
    };
    let message = format!("{error:#}");
    assert!(message.contains(way_out), "{message}");
    assert!(!message.contains("foreign"), "{message}");
    assert_eq!(legacy_tree(&root.join(".setup")), setup);
    Ok(())
}

/// Why Play, Edit and addon changes refuse a setup that is not the instance's
/// own and can't be continued: the way out the instance itself has, Repair for
/// a modpack and otherwise deleting the hidden folder, and a claim that it was
/// copied only for a duplicate's.
#[test]
fn an_uncontinued_setup_names_the_instances_own_way_out() {
    use super::mods::{PendingChange, uncontinued_setup_refusal};
    let plain = schema::parse_instance_config(
        r#"{"_version":"1","name":"Plain","game_configuration":{"version":{"release":"1.20.1","modloaders":[]}}}"#,
    )
    .unwrap();
    let mut linked = plain.clone();
    linked.modpack = Some(info::ModpackInfo {
        modpack: legacy_pack("A"),
        locked: false,
    });
    for change in [
        PendingChange::Play,
        PendingChange::GameVersion,
        PendingChange::Addons,
    ] {
        for copied in [false, true] {
            let modpack = uncontinued_setup_refusal(&linked, change, copied);
            assert!(modpack.contains("Use Repair"), "{modpack}");
            assert!(!modpack.contains(".setup"), "{modpack}");
            let other = uncontinued_setup_refusal(&plain, change, copied);
            assert!(other.contains("delete the hidden .setup folder"), "{other}");
            assert!(!other.contains("Repair"), "{other}");
            for message in [&modpack, &other] {
                assert_eq!(message.contains("copied while"), copied, "{message}");
                assert_eq!(
                    message.contains("finish the setup on the original instance"),
                    copied,
                    "{message}"
                );
            }
        }
    }
}

/// A new pack instance (a platform install when `linked`, otherwise a local
/// .mrpack import without a platform match) whose first Play put the pack in
/// place and whose game version install then failed, with that setup.
async fn versiononly_installed_pack(
    app: &crate::App,
    linked: bool,
) -> anyhow::Result<(InstanceId, PathBuf, super::modpack::session::Session)> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let group = app.instance_manager().get_default_group().await?;
    let source = if linked {
        InstanceVersionSource::Modpack(legacy_pack("B"), false)
    } else {
        InstanceVersionSource::Version(GameVersion::Standard(info::StandardVersion {
            release: "1.20.1".into(),
            modloaders: HashSet::from([versiononly_loader(info::ModLoaderType::Forge, "47.2.0")]),
        }))
    };
    let id = app
        .instance_manager()
        .create_instance_ext(
            group,
            format!("versiononly-pack-{linked}"),
            None,
            None,
            None,
            source,
            String::new(),
            move |path| async move {
                // A local .mrpack import copies the archive in; a platform
                // install downloads it on its first Play.
                if !linked {
                    task08_round1_archive(&path, false)?;
                }
                tokio::fs::create_dir_all(path.join("instance/saves/world")).await?;
                tokio::fs::write(path.join("instance/saves/world/level.dat"), b"world").await?;
                Ok(())
            },
        )
        .await?;
    let root = versiononly_root(app, id).await?;
    if linked {
        task08_round1_archive(&root, false)?;
    }
    // Play puts the pack in place; its game version install then fails.
    legacy_prepare(app, id).await?;
    let old = Session::load(&root).await?.expect("the pack setup");
    assert_eq!(
        (old.producer, old.kind, old.phase),
        (
            if linked {
                Producer::Create
            } else {
                Producer::Import
            },
            Kind::Pack,
            Phase::Committed
        )
    );
    Ok((id, root, old))
}

/// A pack install whose files and configuration were published, but whose
/// game version or loader install then failed, only waits for that install,
/// like a completed import: another version or loader replaces it, as in
/// 2.0.40. This is the only way out for an archive import without a
/// platform match, which has no Repair. The pack's files and installed
/// record stay; the finished install's archive is not kept.
async fn versiononly_installed_pack_with_failed_game_install_is_replaced(
    linked: bool,
) -> anyhow::Result<()> {
    use super::modpack::session::RECORD;
    let app = crate::setup_managers_for_test().await;
    let (id, root, old) = versiononly_installed_pack(&app, linked).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    assert_eq!(config.modpack.is_some(), linked);
    assert!(root.join(".setup/modrinth").is_file());
    let installed = tokio::fs::read(root.join("packinfo.json")).await?;
    assert_eq!(
        tokio::fs::read(root.join("instance/target.txt")).await?,
        b"target"
    );
    // Addon edits wait for it. Without a modpack the refusal names Edit; a
    // modpack's files target the pack's own version, so it is not advertised.
    let refusal = legacy_addon_refusal(&app, id).await;
    assert_eq!(
        refusal.contains("choose another version with Edit"),
        !linked,
        "{refusal}"
    );

    let other = versiononly_loader(info::ModLoaderType::Forge, "47.3.0");
    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, None, Some(Some(other.clone()))),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version))
            if version.release == "1.20.1" && version.modloaders == HashSet::from([other])
    ));
    assert_eq!(
        config.modpack.map(|pack| pack.modpack),
        linked.then(|| legacy_pack("B"))
    );
    let retired = legacy_retired(&root)?;
    assert!(!retired[0].join("modrinth").exists());
    assert!(retired[0].join(RECORD).is_file());
    assert_eq!(
        tokio::fs::read(root.join("packinfo.json")).await?,
        installed
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/target.txt")).await?,
        b"target"
    );
    Ok(())
}

#[tokio::test]
async fn versiononly_imported_pack_with_failed_game_install_is_replaced() -> anyhow::Result<()> {
    versiononly_installed_pack_with_failed_game_install_is_replaced(false).await
}

#[tokio::test]
async fn versiononly_platform_pack_with_failed_game_install_is_replaced() -> anyhow::Result<()> {
    versiononly_installed_pack_with_failed_game_install_is_replaced(true).await
}

/// The same on a duplicate taken while that install waits: the copy's own
/// packinfo.json is still the target the copied record promoted, so another
/// version replaces the copied setup as it is. The copy does not have to
/// rebuild the pack from the copied archive and fail at the same loader
/// first, and nothing on the original changes. A copy whose installed record
/// changed since still waits for Play.
#[tokio::test]
async fn versiononly_copied_installed_pack_with_failed_game_install_is_replaced()
-> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root, old) = versiononly_installed_pack(&app, false).await?;
    let copy = app
        .instance_manager()
        .duplicate_instance(id, "versiononly pack copy".into())
        .await?;
    let copy_root = versiononly_root(&app, copy).await?;
    let copied = Session::load(&copy_root).await?.expect("the copied setup");
    assert_eq!(copied.owner()?, old.owner()?);
    assert_eq!(copied.copied_to, Some(copy.0));
    let source = legacy_tree(&root.join(".setup"));
    let installed = tokio::fs::read(copy_root.join("packinfo.json")).await?;
    // Addon edits on the copy wait for it as on the original, and the refusal
    // names Edit.
    let refusal = legacy_addon_refusal(&app, copy).await;
    assert!(
        refusal.contains("choose another version with Edit"),
        "{refusal}"
    );

    // A second copy whose installed record changed is not replaced.
    let changed = app
        .instance_manager()
        .duplicate_instance(id, "versiononly pack copy changed".into())
        .await?;
    let changed_root = versiononly_root(&app, changed).await?;
    tokio::fs::write(
        changed_root.join("packinfo.json"),
        super::modpack::packinfo::make_packinfo(super::modpack::packinfo::PackInfo {
            files: HashMap::new(),
        })?,
    )
    .await?;
    let message = versiononly_refused(
        &app,
        changed,
        &changed_root,
        versiononly_edit(changed, Some("1.20.4"), None),
    )
    .await?;
    assert!(message.contains("Press Play"), "{message}");

    let _queue = versiononly_hold_queue(&app).await?;
    let other = versiononly_loader(info::ModLoaderType::Forge, "47.3.0");
    app.instance_manager()
        .update_instance(versiononly_edit(copy, None, Some(Some(other.clone()))))
        .await?
        .expect("the loader change prepares the copy");
    let retired = legacy_retired(&copy_root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", copied.operation))
    );
    // The copied record is kept as it was; only the finished install's
    // archive goes.
    assert_eq!(
        serde_json::to_value(legacy_retired_session(&retired[0])?)?,
        serde_json::to_value(&copied)?
    );
    assert!(!retired[0].join("modrinth").exists());
    let new = Session::load(&copy_root)
        .await?
        .expect("the replacing setup");
    assert_eq!(
        (new.producer, new.kind, new.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    assert_eq!(new.owner()?.instance_id, copy.0);
    assert!(matches!(
        disk_config(&copy_root).await?.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.modloaders == HashSet::from([other])
    ));
    assert_eq!(
        tokio::fs::read(copy_root.join("packinfo.json")).await?,
        installed
    );
    assert_eq!(
        tokio::fs::read(copy_root.join("instance/target.txt")).await?,
        b"target"
    );
    assert_eq!(legacy_tree(&root.join(".setup")), source);
    assert!(legacy_retired(&root)?.is_empty());
    Ok(())
}

/// What 2.0.40 leaves once a pack install, update or repair applied its files
/// and its game version or loader install then failed: the archive, the
/// request and the apply leftovers beside modpack-complete, the record apply
/// promoted, and no session record.
fn versiononly_legacy_applied_pack(root: &Path) -> anyhow::Result<()> {
    legacy_downloaded_archive(root, &[("config/pack.toml", b"B")])?;
    std::fs::write(root.join(".setup/staging-packinfo.json"), b"[]")?;
    std::fs::write(root.join(".setup/apply-started"), b"")?;
    std::fs::create_dir(root.join(".setup/modpack-skip-overrides"))?;
    std::fs::write(root.join(".setup/modpack-complete"), b"")?;
    Ok(())
}

/// 2.0.40 skipped the pack step beside that marker, so choosing another
/// loader in Edit installed it straight away. It still does: the leftover is
/// retired without the finished install's archive, and the new choice is
/// prepared as a version-only install, with no pack rebuild and no failing
/// Play first. Play alone still rebuilds the pack from the archive, so addon
/// edits wait for it. A leftover whose apply may not have finished still
/// waits for Play.
#[tokio::test]
async fn versiononly_2040_applied_pack_with_failed_game_install_is_replaced() -> anyhow::Result<()>
{
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = legacy_pack_fixture(&app, "versiononly-legacy-applied", true, None).await?;
    versiononly_legacy_applied_pack(&root)?;
    let refusal = legacy_addon_refusal(&app, id).await;
    assert!(refusal.contains("Press Play"), "{refusal}");
    assert!(!root.join(".setup/setup-session.json").exists());

    // Without a pack link (a local .mrpack with no platform match), Edit is
    // named too, and the check changes nothing.
    let (unlinked, unlinked_root) =
        legacy_pack_fixture(&app, "versiononly-legacy-applied-unlinked", true, None).await?;
    let (shortpath, mut config) = app.instance_manager().current_config(unlinked).await?;
    config.modpack = None;
    app.instance_manager()
        .persist_config(unlinked, &shortpath, config)
        .await?;
    versiononly_legacy_applied_pack(&unlinked_root)?;
    let unlinked_setup = legacy_tree(&unlinked_root.join(".setup"));
    let refusal = legacy_addon_refusal(&app, unlinked).await;
    assert!(
        refusal.contains("choose another version with Edit"),
        "{refusal}"
    );
    assert!(!refusal.contains("Repair"), "{refusal}");
    assert_eq!(legacy_tree(&unlinked_root.join(".setup")), unlinked_setup);

    // A promotion still waiting.
    let (pending, pending_root) =
        legacy_pack_fixture(&app, "versiononly-legacy-promoting", true, None).await?;
    versiononly_legacy_applied_pack(&pending_root)?;
    tokio::fs::write(pending_root.join("tmp-packinfo.json"), b"{}").await?;
    let message = versiononly_refused(
        &app,
        pending,
        &pending_root,
        versiononly_edit(pending, Some("1.20.4"), None),
    )
    .await?;
    assert!(message.contains("Press Play"), "{message}");

    let setup = legacy_tree(&root.join(".setup"));
    let instance = legacy_tree(&root.join("instance"));
    let installed = tokio::fs::read(root.join("packinfo.json")).await?;
    let queue = versiononly_hold_queue(&app).await?;
    let forge = versiononly_loader(info::ModLoaderType::Forge, "47.3.0");
    app.instance_manager()
        .update_instance(versiononly_edit(id, None, Some(Some(forge.clone()))))
        .await?
        .expect("the loader change prepares the instance");
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".setup-cancelled-legacy-")
    );
    let mut kept = setup;
    kept.remove("modrinth");
    assert_eq!(legacy_tree(&retired[0]), kept);
    let new = Session::load(&root).await?.expect("the replacing setup");
    assert_eq!(
        (new.producer, new.kind, new.phase),
        (Producer::Settings, Kind::VersionOnly, Phase::Reserved)
    );
    assert_eq!(new.owner()?.instance_id, id.0);
    let config = disk_config(&root).await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.modloaders == HashSet::from([forge])
    ));
    assert_eq!(
        config.modpack.map(|pack| pack.modpack),
        Some(legacy_pack("A"))
    );

    // Its prepare goes straight to the game version install.
    let staged = app.instance_manager().test_hooks.arm("setup_staged", id);
    drop(queue);
    tokio::time::timeout(Duration::from_secs(30), staged.entered.notified()).await?;
    let prepared = Session::load(&root).await?.expect("the replacing setup");
    assert_eq!(prepared.operation, new.operation);
    assert_eq!(prepared.phase, Phase::Committed);
    assert_eq!(legacy_tree(&root.join("instance")), instance);
    assert_eq!(
        tokio::fs::read(root.join("packinfo.json")).await?,
        installed
    );
    Ok(())
}

/// An import that keeps its platform link (as the CurseForge app, Prism,
/// Modrinth App, ATLauncher and FTB importers leave a managed pack) whose
/// first install failed, still locked as it was imported.
async fn versiononly_failed_linked_import(
    app: &crate::App,
    name: &str,
) -> anyhow::Result<(InstanceId, PathBuf, super::modpack::session::Session)> {
    use super::modpack::session::{Kind, Phase, Producer};
    let group = app.instance_manager().get_default_group().await?;
    let fabric = versiononly_loader(info::ModLoaderType::Fabric, "0.15.11");
    let id = app
        .instance_manager()
        .create_instance_ext(
            group,
            name.into(),
            None,
            None,
            None,
            InstanceVersionSource::ModpackWithKnownVersion(
                GameVersion::Standard(info::StandardVersion {
                    release: "1.20.1".into(),
                    modloaders: HashSet::from([fabric]),
                }),
                legacy_pack("A"),
                true,
            ),
            String::new(),
            |path| async move {
                tokio::fs::create_dir_all(path.join(".setup/modpack-complete")).await?;
                tokio::fs::create_dir_all(path.join("instance/saves/world")).await?;
                tokio::fs::write(path.join("instance/saves/world/level.dat"), b"world").await?;
                Ok(())
            },
        )
        .await?;
    let root = versiononly_root(app, id).await?;
    let queue = versiononly_hold_queue(app).await?;
    app.instance_manager()
        .prepare_game(id, None, None, false)
        .await?;
    let old = versiononly_fail(app, id, &root, queue, false).await?;
    assert_eq!(
        (old.producer, old.kind, old.phase),
        (Producer::Import, Kind::ImportedComplete, Phase::Committed)
    );
    assert!(old.requested_pack.is_some());
    assert!(root.join(".setup/change-pack-version.json").is_file());
    let locked = app
        .instance_manager()
        .ensure_modpack_not_locked(id)
        .await
        .expect_err("a locked pack");
    assert_eq!(locked.to_string(), "Modpack is locked");
    Ok((id, root, old))
}

/// Its pack request is only the link, which Play never downloads or applies.
/// Once the pack is unlocked, addon edits are allowed beside it, as in
/// 2.0.40, and another version replaces it.
#[tokio::test]
async fn versiononly_failed_linked_import_keeps_addon_edits_and_is_replaced() -> anyhow::Result<()>
{
    let app = crate::setup_managers_for_test().await;
    let (id, root, old) =
        versiononly_failed_linked_import(&app, "versiononly-linked-import").await?;
    let mut unlock = update(id);
    unlock.modpack_locked = Some(Some(false));
    assert!(
        app.instance_manager()
            .update_instance(unlock)
            .await?
            .is_none()
    );
    app.instance_manager().ensure_modpack_not_locked(id).await?;

    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, Some("1.20.4"), None),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.release == "1.20.4"
    ));
    assert_eq!(
        config.modpack.map(|pack| (pack.modpack, pack.locked)),
        Some((legacy_pack("A"), false))
    );
    assert!(
        legacy_retired(&root)?[0]
            .join("change-pack-version.json")
            .is_file()
    );
    Ok(())
}

/// Unpair instead of Unlock allows the same addon edits: Play neither
/// applies the link nor attaches it again, so nothing the edits touch is
/// waiting. Another version still replaces the setup, and the pack stays
/// unpaired.
#[tokio::test]
async fn versiononly_failed_linked_import_unpaired_keeps_addon_edits() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root, old) =
        versiononly_failed_linked_import(&app, "versiononly-linked-import-unpaired").await?;
    let mut unpair = update(id);
    unpair.modpack_locked = Some(None);
    assert!(
        app.instance_manager()
            .update_instance(unpair)
            .await?
            .is_none()
    );
    let setup = legacy_tree(&root.join(".setup"));
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    assert_eq!(legacy_tree(&root.join(".setup")), setup);

    let config = versiononly_supersede(
        &app,
        id,
        &root,
        &old,
        versiononly_edit(id, Some("1.20.4"), None),
    )
    .await?;
    assert!(matches!(
        config.game_configuration.version,
        Some(GameVersion::Standard(version)) if version.release == "1.20.4"
    ));
    assert!(config.modpack.is_none());
    Ok(())
}

/// A setup record this build cannot read (damaged, or written by another
/// build) still holds version changes, addon edits and Play, and each
/// refusal says how to get out: Repair for a modpack, otherwise removing the
/// hidden folder. Nothing is changed.
#[tokio::test]
async fn versiononly_unreadable_setup_names_the_way_out() -> anyhow::Result<()> {
    use super::modpack::session::RECORD;
    let app = crate::setup_managers_for_test().await;
    for linked in [false, true] {
        let (id, root) =
            versiononly_installed(&app, &format!("versiononly-unreadable-{linked}"), None).await?;
        if linked {
            let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
            config.modpack = Some(info::ModpackInfo {
                modpack: legacy_pack("A"),
                locked: false,
            });
            app.instance_manager()
                .persist_config(id, &shortpath, config)
                .await?;
        }
        tokio::fs::create_dir(root.join(".setup")).await?;
        tokio::fs::write(root.join(".setup").join(RECORD), br#"{"schema_version":2}"#).await?;
        let way_out = if linked {
            "Use Repair on this instance"
        } else {
            "delete the hidden .setup folder"
        };
        let before = legacy_tree(&root);

        let message =
            versiononly_refused(&app, id, &root, versiononly_edit(id, Some("1.20.4"), None))
                .await?;
        assert!(message.contains("can't be checked"), "{message}");
        assert!(message.contains(way_out), "{message}");
        assert_eq!(
            message.contains("choose the version again"),
            !linked,
            "{message}"
        );

        let refusal = legacy_addon_refusal(&app, id).await;
        assert!(refusal.contains("can't be checked"), "{refusal}");
        assert!(refusal.contains(way_out), "{refusal}");

        let error = app
            .instance_manager()
            .prepare_game(id, None, None, false)
            .await
            .err()
            .expect("Play cannot finish a setup it cannot read");
        let message = format!("{error:#}");
        assert!(message.contains("can't be checked"), "{message}");
        assert!(message.contains(way_out), "{message}");
        assert_eq!(legacy_tree(&root), before);
    }
    Ok(())
}

// Repair is the way out of every pending setup: it retires whatever is pending
// (keeping it as `.setup-cancelled-<operation|legacy>-<uuid>`) and reinstalls
// the configured pack, or the pack an interrupted apply was installing, from
// a fresh download, planned against the root packinfo.json. It only discards
// the setup the dialog showed.

fn repairblocked_request(version: &str) -> anyhow::Result<String> {
    Ok(serde_json::to_string(
        &super::modpack::PackVersionFile::from(legacy_pack(version)),
    )?)
}

/// The version a setup's pack request (or a repair's) installs.
fn repairblocked_requested(session: &super::modpack::session::Session) -> anyhow::Result<String> {
    let requested: super::modpack::PackVersionFile = serde_json::from_str(
        session
            .requested_pack
            .as_deref()
            .context("the setup requests a pack")?,
    )?;
    Ok(super::modpack::PackProofContext::from(&info::Modpack::from(requested)).version_id)
}

/// An instance with pack A installed: `mods/pack.jar` and `mods/old.jar`
/// (which only A ships) recorded in the root packinfo.json, plus the user's
/// own mod and world. `configured` is the pack instance.json names.
async fn repairblocked_installed(
    app: &crate::App,
    name: &str,
    configured: &str,
) -> anyhow::Result<(InstanceId, PathBuf)> {
    use super::modpack::packinfo;
    let (id, root) = fixture(app, name).await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    let (shortpath, mut config) = app.instance_manager().current_config(id).await?;
    config.modpack = Some(info::ModpackInfo {
        modpack: legacy_pack(configured),
        locked: false,
    });
    config.game_configuration.version = Some(GameVersion::Standard(info::StandardVersion {
        release: "1.20.1".into(),
        modloaders: HashSet::new(),
    }));
    app.instance_manager()
        .persist_config(id, &shortpath, config)
        .await?;
    tokio::fs::create_dir_all(root.join("instance/mods")).await?;
    tokio::fs::write(root.join("instance/mods/pack.jar"), b"A").await?;
    tokio::fs::write(root.join("instance/mods/old.jar"), b"only A").await?;
    let record = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(record)?).await?;
    tokio::fs::write(root.join("instance/mods/user.jar"), b"user").await?;
    tokio::fs::create_dir_all(root.join("instance/saves/world")).await?;
    tokio::fs::write(root.join("instance/saves/world/level.dat"), b"world").await?;
    Ok((id, root))
}

/// Pack B's archive as a download leaves it in `.setup`.
fn repairblocked_archive_b(root: &Path) -> anyhow::Result<()> {
    legacy_downloaded_archive(
        root,
        &[("mods/pack.jar", b"B"), ("mods/new.jar", b"new in B")],
    )
}

/// Pack A installed, with this build's own update to B resolved: its archive
/// downloaded and staged, its target recorded in the root tmp-packinfo.json,
/// nothing applied yet.
async fn repairblocked_resolved_change(
    app: &crate::App,
    name: &str,
) -> anyhow::Result<(InstanceId, PathBuf, super::modpack::session::Owner)> {
    use super::modpack::session::{Kind, Phase, Producer, Session};
    let (id, root) = repairblocked_installed(app, name, "A").await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    tokio::fs::create_dir(root.join(".setup")).await?;
    let mut session = Session::new(&config, Producer::Change, Kind::Pack, None)?;
    session.instance_id = Some(id.0);
    session.requested_pack = Some(repairblocked_request("B")?);
    session.create(&root).await?;
    tokio::fs::write(
        root.join(".setup/change-pack-version.json"),
        repairblocked_request("B")?,
    )
    .await?;
    repairblocked_archive_b(&root)?;
    let mut owner = Some(session.owner()?);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &legacy_task(),
        false,
        &mut owner,
    )
    .await?;
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Resolved);
    assert!(root.join("tmp-packinfo.json").is_file());
    Ok((id, root, owner.unwrap()))
}

/// Runs the repair that `owner` admitted once its fresh archive of B has been
/// downloaded, through apply, as Play's worker does.
async fn repairblocked_finish_repair(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
    owner: super::modpack::session::Owner,
) -> anyhow::Result<()> {
    repairblocked_archive_b(root)?;
    let mut owner = Some(owner);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = legacy_task();
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await
}

/// The pack's files after a repair that installed B over A's record: what
/// only A owned is gone, the user's files and world stay.
async fn repairblocked_assert_b_installed(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
) -> anyhow::Result<()> {
    use super::modpack::{
        packinfo,
        session::{Phase, Session},
    };
    assert_eq!(Session::load(root).await?.unwrap().phase, Phase::Committed);
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/pack.jar")).await?,
        b"B"
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/new.jar")).await?,
        b"new in B"
    );
    assert!(!root.join("instance/mods/old.jar").exists());
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/user.jar")).await?,
        b"user"
    );
    assert_eq!(
        tokio::fs::read(root.join("instance/saves/world/level.dat")).await?,
        b"world"
    );
    let record =
        packinfo::parse_packinfo(&tokio::fs::read_to_string(root.join("packinfo.json")).await?)?;
    assert!(record.files.contains_key("/mods/new.jar"));
    assert!(!record.files.contains_key("/mods/old.jar"));
    let (_, config) = app.instance_manager().current_config(id).await?;
    assert_eq!(
        config.modpack.map(|pack| pack.modpack),
        Some(legacy_pack("B"))
    );
    Ok(())
}

/// A first install whose staged jar disappeared (antivirus, a cleanup tool)
/// fails the same way on every Play and can't be cancelled, as there is no
/// earlier version. Repair retires it and installs the configured pack
/// again; nothing in the instance changes before the new download is staged.
#[tokio::test]
async fn repairblocked_failed_first_install_is_replaced_by_repair() -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Phase, Producer, RECORD, Session};
    let app = crate::setup_managers_for_test().await;
    let group = app.instance_manager().get_default_group().await?;
    let id = app
        .instance_manager()
        .create_instance(
            group,
            "repairblocked-first-install".into(),
            false,
            InstanceVersionSource::Modpack(legacy_pack("B"), false),
            String::new(),
        )
        .await?;
    let root = versiononly_root(&app, id).await?;
    let created = Session::load(&root).await?.expect("the install's setup");
    assert_eq!(
        (created.producer, created.kind),
        (Producer::Create, Kind::Pack)
    );
    repairblocked_archive_b(&root)?;
    let mut owner = Some(created.owner()?);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let task = legacy_task();
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &task,
        false,
        &mut owner,
    )
    .await?;
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Resolved);
    tokio::fs::remove_file(root.join(".setup/staging/instance/mods/new.jar")).await?;
    let error = run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await
    .expect_err("Play can't finish an install whose staged file disappeared");
    // In plain words, with the way out.
    let message = format!("{error:#}");
    assert!(message.contains("mods/new.jar"), "{message}");
    assert!(message.contains("antivirus"), "{message}");
    assert!(message.contains("Use Repair on this instance"), "{message}");

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.pending);
    assert!(!status.can_cancel);
    assert!(status.can_resume);
    assert!(!status.preview_withheld);
    assert_eq!(status.repair_version.as_deref(), Some("B"));
    // Nothing was installed yet: the preview shows a pack with no record.
    assert!(
        !app.instance_manager()
            .repair_preview(id)
            .await?
            .has_packinfo
    );
    let token = status.discard_token.clone();
    assert_eq!(token.as_deref(), Some(created.operation.as_str()));
    let instance_before = legacy_tree(&root.join("instance"));
    let config_before = tokio::fs::read(root.join("instance.json")).await?;
    let target_before = tokio::fs::read(root.join("tmp-packinfo.json")).await?;

    // Keep the repair queued: this test stops before any download.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    app.instance_manager()
        .repair_modpack(id, super::modpack::RepairMarkerFile::default(), token)
        .await?;

    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", created.operation))
    );
    let old = legacy_retired_session(&retired[0])?;
    assert_eq!(
        (old.producer, old.phase),
        (Producer::Create, Phase::Resolved)
    );
    // Its records stay; the downloads it never applied don't.
    assert!(retired[0].join(RECORD).is_file());
    assert!(retired[0].join("change-pack-version.json").is_file());
    assert_eq!(
        tokio::fs::read(retired[0].join("tmp-packinfo.json")).await?,
        target_before
    );
    assert!(!retired[0].join("modrinth").exists());
    assert!(!retired[0].join("staging").exists());
    assert!(!root.join("tmp-packinfo.json").exists());

    let repair = Session::load(&root).await?.expect("the repair's setup");
    assert_eq!(
        (repair.producer, repair.kind, repair.phase),
        (Producer::Repair, Kind::Pack, Phase::Reserved)
    );
    assert_eq!(repair.owner()?.instance_id, id.0);
    assert_eq!(repairblocked_requested(&repair)?, "B");
    assert!(repair.replaces_interrupted.is_none());
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    assert_eq!(
        tokio::fs::read(root.join("instance.json")).await?,
        config_before
    );
    Ok(())
}

/// An update whose files and configuration were published, but whose game
/// install then failed (a loader processor, Java, assets), only waits for
/// that install. Repair previews B over B's record and reinstalls B.
#[tokio::test]
async fn repairblocked_committed_update_is_previewed_and_repaired() -> anyhow::Result<()> {
    use super::modpack::session::{Phase, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root, owner) = repairblocked_resolved_change(&app, "repairblocked-committed").await?;
    let (shortpath, _) = app.instance_manager().current_config(id).await?;
    let subtasks_task = legacy_task();
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        app.instance_manager().current_config(id).await?.1,
        shortpath.clone(),
        &subtasks_task,
        false,
        &mut Some(owner.clone()),
    )
    .await?;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        Some(&owner),
    )
    .await?;
    repairblocked_assert_b_installed(&app, id, &root).await?;

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.pending);
    assert!(!status.can_cancel);
    assert!(!status.preview_withheld);
    assert_eq!(status.repair_version.as_deref(), Some("B"));
    let preview = app.instance_manager().repair_preview(id).await?;
    assert!(preview.has_packinfo);
    assert_eq!(preview.without_re_enable.counts.restore_modified, 0);

    let instance_before = legacy_tree(&root.join("instance"));
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    app.instance_manager()
        .repair_modpack(
            id,
            super::modpack::RepairMarkerFile::default(),
            status.discard_token.clone(),
        )
        .await?;
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", owner.operation))
    );
    assert_eq!(legacy_retired_session(&retired[0])?.phase, Phase::Committed);
    let repair = Session::load(&root).await?.expect("the repair's setup");
    assert_eq!(repair.producer, Producer::Repair);
    assert_eq!(repairblocked_requested(&repair)?, "B");
    assert!(repair.replaces_interrupted.is_none());
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    Ok(())
}

/// An instance folder moved to another drive and linked back (a symlink, or a
/// junction on Windows) updates and repairs as it did in 2.0.40: the update's
/// target record in the instance folder is written through the link, and
/// both setups apply and commit.
async fn linked_root_updates_and_repairs(link: fn(&Path, &Path)) -> anyhow::Result<()> {
    use super::modpack::session::{Kind, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root) = repairblocked_installed(&app, "linked-root", "A").await?;
    let drive = root.parent().unwrap().parent().unwrap().join("other-drive");
    tokio::fs::create_dir(&drive).await?;
    let moved = drive.join("linked-root");
    tokio::fs::rename(&root, &moved).await?;
    link(&root, &moved);
    assert!(std::fs::symlink_metadata(&root)?.file_type().is_symlink());

    // The user updates the pack to B: download, staging, apply.
    let (_, config) = app.instance_manager().current_config(id).await?;
    tokio::fs::create_dir(root.join(".setup")).await?;
    let mut session = Session::new(&config, Producer::Change, Kind::Pack, None)?;
    session.instance_id = Some(id.0);
    session.requested_pack = Some(repairblocked_request("B")?);
    session.create(&root).await?;
    tokio::fs::write(
        root.join(".setup/change-pack-version.json"),
        repairblocked_request("B")?,
    )
    .await?;
    // Runs any admitted setup of B through apply, as Play's worker does.
    repairblocked_finish_repair(&app, id, &root, session.owner()?).await?;
    repairblocked_assert_b_installed(&app, id, &root).await?;

    // Then repairs a pack file that got damaged.
    tokio::fs::write(root.join("instance/mods/pack.jar"), b"damaged").await?;
    let status = app.instance_manager().pending_setup(id).await?;
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    manager
        .repair_modpack(
            id,
            super::modpack::RepairMarkerFile::default(),
            status.discard_token.clone(),
        )
        .await?;

    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", owner.operation))
    );
    assert_eq!(legacy_retired_session(&retired[0])?.phase, Phase::Applying);
    // Everything it held stays until a later setup finishes.
    assert!(retired[0].join("apply-started").is_file());
    assert_eq!(
        tokio::fs::read(retired[0].join("representation-transitions/transition.json")).await?,
        b"{\"evidence\":true}"
    );
    assert!(retired[0].join("staging/instance/mods/new.jar").is_file());
    assert!(retired[0].join("modrinth").is_file());
    assert_eq!(
        tokio::fs::read(retired[0].join("tmp-packinfo.json")).await?,
        target_before
    );
    assert!(!root.join("tmp-packinfo.json").exists());
    assert_eq!(
        tokio::fs::read(root.join("packinfo.json")).await?,
        record_before
    );
    assert_eq!(legacy_tree(&root.join(".repair-recovery")), recovery_before);
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);

    let repair = Session::load(&root).await?.expect("the repair's setup");
    assert_eq!(
        (repair.producer, repair.phase),
        (Producer::Repair, Phase::Reserved)
    );
    assert_eq!(repairblocked_requested(&repair)?, "B");
    assert_eq!(
        repair.replaces_interrupted.as_deref(),
        Some(owner.operation.as_str())
    );
    // Still no installed version to go back to, and still no preview, until
    // the repair finishes.
    let status = app.instance_manager().pending_setup(id).await?;
    assert_eq!(status.operation.as_deref(), Some(repair.operation.as_str()));
    assert!(!status.can_cancel);
    assert!(status.preview_withheld);
    assert_eq!(status.repair_version.as_deref(), Some("B"));
    assert!(app.instance_manager().repair_preview(id).await.is_err());
    assert!(
        app.instance_manager()
            .cancel_pending_setup(id, repair.operation.clone())
            .await
            .is_err()
    );

    // The repair fails too, after downloading and staging B (offline half
    // way, or antivirus removing a staged jar), and the user repairs again.
    // That repair never changed a file itself, so only its records stay; the
    // interrupted update's evidence stays until a repair finishes.
    repairblocked_archive_b(&root)?;
    let mut first = Some(repair.owner()?);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &legacy_task(),
        false,
        &mut first,
    )
    .await?;
    assert_eq!(Session::load(&root).await?.unwrap().phase, Phase::Resolved);
    tokio::fs::create_dir_all(root.join(".setup/preparing-earlier/instance/mods")).await?;
    tokio::fs::write(
        root.join(".setup/preparing-earlier/instance/mods/new.jar"),
        b"part",
    )
    .await?;
    // Its worker gave up (here it waits for the download permit instead).
    app.instance_manager()
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Inactive { failed_task: None };
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(!status.busy);
    assert!(!status.can_cancel);
    assert!(status.preview_withheld);
    assert_eq!(status.repair_version.as_deref(), Some("B"));
    let instance_before = legacy_tree(&root.join("instance"));
    app.instance_manager()
        .repair_modpack(
            id,
            super::modpack::RepairMarkerFile::default(),
            status.discard_token.clone(),
        )
        .await?;
    let failed = legacy_retired(&root)?
        .into_iter()
        .find(|retired| {
            retired
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(".setup-cancelled-{}-", repair.operation))
        })
        .expect("the failed repair is retired");
    assert_eq!(legacy_retired(&root)?.len(), 2);
    let mut kept: Vec<_> = std::fs::read_dir(&failed)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<anyhow::Result<_>>()?;
    kept.sort();
    assert_eq!(
        kept,
        [
            "change-pack-version.json",
            // An empty marker.
            "modpack-skip-overrides",
            "repair",
            super::modpack::session::RECORD,
            "staging-packinfo.json",
            "tmp-packinfo.json",
        ]
    );
    // The interrupted update's payload is still there.
    assert!(retired[0].join("staging/instance/mods/new.jar").is_file());
    assert!(retired[0].join("modrinth").is_file());
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    let second = Session::load(&root)
        .await?
        .expect("the second repair's setup");
    assert_eq!(second.producer, Producer::Repair);
    assert_eq!(repairblocked_requested(&second)?, "B");
    assert_eq!(
        second.replaces_interrupted.as_deref(),
        Some(repair.operation.as_str())
    );

    // Its fresh download applies over A's record.
    repairblocked_finish_repair(&app, id, &root, second.owner()?).await?;
    repairblocked_assert_b_installed(&app, id, &root).await?;
    assert!(retired[0].join("staging/instance/mods/new.jar").is_file());
    // Once the game install finishes too, the setup goes and every setup it
    // replaced keeps only its records.
    super::modpack::session::dispose(&root, &second.owner()?).await?;
    assert!(!root.join(".setup").exists());
    assert!(!retired[0].join("staging").exists());
    assert!(!retired[0].join("modrinth").exists());
    assert!(retired[0].join(super::modpack::session::RECORD).is_file());
    assert!(
        retired[0]
            .join("representation-transitions/transition.json")
            .is_file()
    );
    assert!(failed.join(super::modpack::session::RECORD).is_file());
    Ok(())
}

/// A cancelled update never changed the instance, so only its records stay:
/// the archive and every download it staged or was preparing go.
#[tokio::test]
async fn repairblocked_cancel_keeps_only_the_records() -> anyhow::Result<()> {
    use super::modpack::session::RECORD;
    let app = crate::setup_managers_for_test().await;
    let (id, root, owner) = repairblocked_resolved_change(&app, "repairblocked-cancel").await?;
    tokio::fs::create_dir_all(root.join(".setup/preparing-earlier/instance/mods")).await?;
    tokio::fs::write(
        root.join(".setup/preparing-earlier/instance/mods/new.jar"),
        b"part",
    )
    .await?;
    assert!(root.join(".setup/modrinth").is_file());
    assert!(root.join(".setup/staging").is_dir());
    let instance_before = legacy_tree(&root.join("instance"));
    app.instance_manager()
        .cancel_pending_setup(id, owner.operation.clone())
        .await?;
    assert!(!root.join(".setup").exists());
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    let mut kept: Vec<_> = std::fs::read_dir(&retired[0])?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<anyhow::Result<_>>()?;
    kept.sort();
    assert_eq!(
        kept,
        [
            "cancelled-target.json",
            "change-pack-version.json",
            // An empty marker.
            "modpack-skip-overrides",
            RECORD,
            "staging-packinfo.json",
        ]
    );
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    Ok(())
}

/// When the replacement can't take the pending setup's place after the
/// setup was retired, the setup comes back exactly as it was, with its
/// temporary record, and nothing of the replacement stays.
#[tokio::test]
async fn repairblocked_unpublished_replacement_puts_the_setup_back() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root, owner) =
        repairblocked_resolved_change(&app, "repairblocked-unpublished").await?;
    tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.preview_withheld);
    let before = legacy_tree(&root);
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("setup_replacement_retired", id);
    let repair = manager.repair_modpack(
        id,
        super::modpack::RepairMarkerFile::default(),
        status.discard_token.clone(),
    );
    tokio::pin!(repair);
    tokio::select! {
        _ = pause.entered.notified() => {},
        result = &mut repair => panic!("the repair did not reach publication: {result:?}"),
    }
    // Retired, with its replacement written beside it.
    assert!(!root.join(".setup").exists());
    assert!(!root.join("tmp-packinfo.json").exists());
    assert_eq!(legacy_retired(&root)?.len(), 1);
    let replacement: Vec<_> = std::fs::read_dir(&root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".setup-next-")
        })
        .collect();
    assert_eq!(replacement.len(), 1);
    assert!(
        replacement[0]
            .join(super::modpack::session::RECORD)
            .is_file()
    );
    // Its publication fails.
    std::fs::remove_dir_all(&replacement[0])?;
    pause.release.notify_one();
    assert!(repair.await.is_err());
    assert_eq!(legacy_tree(&root), before);
    let status = app.instance_manager().pending_setup(id).await?;
    assert_eq!(status.operation.as_deref(), Some(owner.operation.as_str()));
    Ok(())
}

/// A crash after Repair retired an interrupted setup and before its
/// replacement took `.setup`'s place never leaves the instance looking like
/// nothing is pending: the next start (or the Repair dialog) publishes the
/// replacement, which Play then continues.
#[tokio::test]
async fn repairblocked_crash_before_publication_keeps_the_replacement() -> anyhow::Result<()> {
    use super::modpack::session::{self, Kind, Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root, owner) = repairblocked_resolved_change(&app, "repairblocked-crash").await?;
    tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
    // What a Repair leaves when the process dies between the two renames.
    let crash = |root: PathBuf, label: String| {
        let app = app.clone();
        async move {
            session::retire_replaced(&root, &label).await?;
            let next = session::begin_replacement(&root).await?;
            let (_, config) = app.instance_manager().current_config(id).await?;
            let mut repair = Session::new(&config, Producer::Repair, Kind::Pack, None)?;
            repair.instance_id = Some(id.0);
            repair.requested_pack = Some(repairblocked_request("A")?);
            session::atomic_json(&next.join(session::RECORD), &repair, true).await?;
            assert!(!root.join(".setup").exists());
            anyhow::Ok(repair)
        }
    };
    let replacements = |root: &Path| -> anyhow::Result<usize> {
        Ok(std::fs::read_dir(root)?
            .filter(|entry| {
                entry.as_ref().is_ok_and(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".setup-next-")
                })
            })
            .count())
    };

    let first = crash(root.clone(), owner.operation.clone()).await?;
    app.instance_manager().scan_instances().await?;
    assert_eq!(replacements(&root)?, 0);
    assert_eq!(
        Session::load(&root).await?.map(|saved| saved.operation),
        Some(first.operation.clone())
    );

    let second = crash(root.clone(), first.operation.clone()).await?;
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.pending);
    assert_eq!(status.operation, Some(second.operation.clone()));
    assert_eq!(replacements(&root)?, 0);
    assert_eq!(legacy_retired(&root)?.len(), 2);
    Ok(())
}

/// Every 2.0.40 leftover is replaced by Repair: the ones Play refuses to
/// resume (and a record this build can't read) are retired as `legacy`, the
/// untouched ones once the dialog adopted them. Pack B is what instance.json
/// names (2.0.40 wrote it there before applying an update) while
/// packinfo.json still records A's files: Repair installs B from a fresh
/// archive, removes what only A owned, and never reads the retired staging.
#[tokio::test]
async fn repairblocked_legacy_leftovers_are_replaced_by_the_configured_pack() -> anyhow::Result<()>
{
    use super::modpack::session::{Producer, RECORD, Session};
    let app = crate::setup_managers_for_test().await;
    // Keep each repair's worker queued: this test runs the repair itself.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    for shape in ["empty", "request", "staging", "apply-started", "unreadable"] {
        let (id, root) =
            repairblocked_installed(&app, &format!("repairblocked-legacy-{shape}"), "B").await?;
        tokio::fs::create_dir(root.join(".setup")).await?;
        if shape != "empty" {
            tokio::fs::write(
                root.join(".setup/change-pack-version.json"),
                repairblocked_request("B")?,
            )
            .await?;
        }
        let staged = matches!(shape, "staging" | "apply-started");
        if staged {
            // Staged bytes the repair must never take.
            tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
            tokio::fs::write(root.join(".setup/staging/instance/mods/new.jar"), b"stale").await?;
        }
        if shape == "apply-started" {
            tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
            tokio::fs::write(root.join("tmp-packinfo.json"), b"2.0.40 target").await?;
            tokio::fs::write(root.join("instance/mods/pack.jar"), b"B").await?;
        }
        if shape == "unreadable" {
            tokio::fs::write(root.join(".setup").join(RECORD), b"{damaged").await?;
        }

        // The dialog adopts the shapes that provably never reached apply, as
        // the earlier-launcher-version handling does; the rest stay unowned.
        let adopted = matches!(shape, "empty" | "request");
        let status = app.instance_manager().pending_setup(id).await?;
        assert!(status.pending, "{shape}");
        assert_eq!(status.operation.is_some(), adopted, "{shape}");
        assert_eq!(status.can_resume, adopted, "{shape}");
        // Only an untouched update of an installed pack can be cancelled.
        assert_eq!(status.can_cancel, shape == "request", "{shape}");
        assert_eq!(status.repair_version.as_deref(), Some("B"), "{shape}");
        assert_eq!(status.preview_withheld, staged, "{shape}");
        let label = status.operation.clone().unwrap_or_else(|| "legacy".into());
        assert_eq!(
            status.discard_token.clone().as_deref(),
            Some(label.as_str()),
            "{shape}"
        );
        let instance_before = legacy_tree(&root.join("instance"));
        let record_before = tokio::fs::read(root.join("packinfo.json")).await?;

        app.instance_manager()
            .repair_modpack(
                id,
                super::modpack::RepairMarkerFile::default(),
                status.discard_token.clone(),
            )
            .await?;
        let repair = Session::load(&root).await?.expect("the repair's setup");
        let retired = legacy_retired(&root)?;
        assert_eq!(retired.len(), 1, "{shape}");
        assert!(
            retired[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(".setup-cancelled-{label}-")),
            "{shape}"
        );
        assert_eq!(
            retired[0].join("change-pack-version.json").is_file(),
            shape != "empty",
            "{shape}"
        );
        if shape == "apply-started" {
            assert_eq!(
                tokio::fs::read(retired[0].join("tmp-packinfo.json")).await?,
                b"2.0.40 target"
            );
        }
        if shape == "unreadable" {
            assert_eq!(tokio::fs::read(retired[0].join(RECORD)).await?, b"{damaged");
        }
        assert_eq!(
            tokio::fs::read(root.join("packinfo.json")).await?,
            record_before
        );
        assert_eq!(
            legacy_tree(&root.join("instance")),
            instance_before,
            "{shape}"
        );
        assert_eq!(repair.producer, Producer::Repair);
        assert_eq!(repairblocked_requested(&repair)?, "B");
        assert_eq!(repair.replaces_interrupted.is_some(), staged, "{shape}");

        repairblocked_finish_repair(&app, id, &root, repair.owner()?).await?;
        repairblocked_assert_b_installed(&app, id, &root).await?;
        if staged {
            assert_eq!(
                tokio::fs::read(retired[0].join("staging/instance/mods/new.jar")).await?,
                b"stale"
            );
        }
    }
    Ok(())
}

/// 2.0.40 leftovers that never changed the installed files, or finished
/// changing them, keep the preview: an update to B that failed while
/// downloading (instance.json and packinfo.json still name A, which is
/// untouched), a repair of A that failed while downloading (2.0.40 repaired
/// the configured pack, so its configuration switch changed nothing), and
/// an install of B whose files applied before its game install failed, also
/// once Play rebuilt it from its archive and stopped before applying it
/// again (2.0.40's apply marker is still beside it). Repair retires them
/// without their downloads, and the repair that replaces them can still be
/// cancelled back to what is installed.
#[tokio::test]
async fn repairblocked_untouched_legacy_leftovers_keep_the_preview() -> anyhow::Result<()> {
    use super::modpack::{
        packinfo,
        session::{Phase, Producer, Session},
    };
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    for shape in [
        "update downloading",
        "repair downloading",
        "applied",
        "applied, then rebuilt by Play",
    ] {
        let applied = shape.starts_with("applied");
        let configured = if applied { "B" } else { "A" };
        let requested = if shape == "update downloading" {
            "B"
        } else {
            configured
        };
        let (id, root) = repairblocked_installed(
            &app,
            &format!("repairblocked-untouched-{}", shape.replace([' ', ','], "-")),
            configured,
        )
        .await?;
        if applied {
            // B's files and their promoted record.
            tokio::fs::write(root.join("instance/mods/pack.jar"), b"B").await?;
            tokio::fs::remove_file(root.join("instance/mods/old.jar")).await?;
            tokio::fs::write(root.join("instance/mods/new.jar"), b"new in B").await?;
            tokio::fs::remove_file(root.join("instance/mods/user.jar")).await?;
            let record = packinfo::scan_dir(&root.join("instance"), None).await?;
            tokio::fs::write(root.join("packinfo.json"), packinfo::make_packinfo(record)?).await?;
            tokio::fs::write(root.join("instance/mods/user.jar"), b"user").await?;
        }
        tokio::fs::create_dir(root.join(".setup")).await?;
        tokio::fs::write(
            root.join(".setup/change-pack-version.json"),
            repairblocked_request(requested)?,
        )
        .await?;
        repairblocked_archive_b(&root)?;
        tokio::fs::create_dir(root.join(".setup/modpack-skip-overrides")).await?;
        if applied {
            tokio::fs::write(root.join(".setup/modpack-complete"), b"").await?;
            tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
            tokio::fs::write(root.join(".setup/staging-packinfo.json"), b"[]").await?;
        } else {
            tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
            tokio::fs::write(root.join(".setup/staging/instance/mods/new.jar"), b"part").await?;
        }
        if shape == "repair downloading" {
            tokio::fs::write(
                root.join(".setup/repair"),
                serde_json::to_vec(&super::modpack::RepairMarkerFile::default())?,
            )
            .await?;
        }
        // Play rebuilds it from its archive and records it, then stops before
        // applying it again (cancelled, closed, or a failed check).
        let rebuilt = if shape == "applied, then rebuilt by Play" {
            let mut owner = legacy_play_owner(&app, id).await?;
            assert!(owner.is_none(), "Play leaves it unowned until rebuilt");
            let (shortpath, config) = app.instance_manager().current_config(id).await?;
            run::modpack::process_modpack(
                app.clone(),
                id,
                true,
                config,
                shortpath,
                &legacy_task(),
                false,
                &mut owner,
            )
            .await?;
            let session = Session::load(&root).await?.expect("Play recorded it");
            assert_eq!(
                (session.producer, session.phase),
                (Producer::Reconstructed, Phase::Resolved)
            );
            assert!(root.join(".setup/apply-started").is_file());
            Some(session.operation)
        } else {
            None
        };

        let status = app.instance_manager().pending_setup(id).await?;
        assert!(status.pending, "{shape}");
        assert_eq!(status.operation, rebuilt, "{shape}");
        assert!(status.can_resume, "{shape}");
        assert!(!status.preview_withheld, "{shape}");
        assert_eq!(
            status.repair_version.as_deref(),
            Some(configured),
            "{shape}"
        );
        assert!(
            app.instance_manager()
                .repair_preview(id)
                .await?
                .has_packinfo,
            "{shape}"
        );
        let instance_before = legacy_tree(&root.join("instance"));
        app.instance_manager()
            .repair_modpack(
                id,
                super::modpack::RepairMarkerFile::default(),
                status.discard_token.clone(),
            )
            .await?;
        let retired = legacy_retired(&root)?;
        assert_eq!(retired.len(), 1, "{shape}");
        let label = rebuilt.clone().unwrap_or_else(|| "legacy".into());
        assert!(
            retired[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(".setup-cancelled-{label}-")),
            "{shape}"
        );
        assert!(retired[0].join("change-pack-version.json").is_file());
        assert!(!retired[0].join("modrinth").exists(), "{shape}");
        assert!(!retired[0].join("staging").exists(), "{shape}");
        assert_eq!(legacy_tree(&root.join("instance")), instance_before);
        let repair = Session::load(&root).await?.expect("the repair's setup");
        assert_eq!(repairblocked_requested(&repair)?, configured, "{shape}");
        assert!(repair.replaces_interrupted.is_none(), "{shape}");
        // Its worker gave up (here it waits for the download permit
        // instead): the repair can be cancelled back to what is installed.
        app.instance_manager()
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()?
            .state = LaunchState::Inactive { failed_task: None };
        let status = app.instance_manager().pending_setup(id).await?;
        assert!(status.can_cancel, "{shape}: {:?}", status.reason);
        assert!(!status.preview_withheld, "{shape}");
        app.instance_manager()
            .cancel_pending_setup(id, repair.operation.clone())
            .await?;
        assert!(!root.join(".setup").exists(), "{shape}");
        assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    }
    Ok(())
}

/// A 2.0.40 first install that failed while downloading never applied
/// anything (2.0.40 hadn't given the instance its game version yet, and
/// nothing records installed files), so the preview shows the pack with no
/// record, as for this build's own first install. Repair retires it without
/// its downloads; there is still no installed version to cancel back to.
#[tokio::test]
async fn repairblocked_legacy_first_install_keeps_the_preview() -> anyhow::Result<()> {
    use super::modpack::session::{Producer, Session};
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let (id, root) =
        legacy_pack_fixture(&app, "repairblocked-legacy-first-install", false, None).await?;
    repairblocked_archive_b(&root)?;
    tokio::fs::create_dir(root.join(".setup/modpack-skip-overrides")).await?;
    tokio::fs::create_dir_all(root.join(".setup/staging/instance/mods")).await?;
    tokio::fs::write(root.join(".setup/staging/instance/mods/new.jar"), b"part").await?;
    assert!(!root.join("packinfo.json").exists());

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.pending);
    assert_eq!(status.operation, None);
    assert!(status.can_resume);
    assert!(!status.can_cancel);
    assert!(!status.preview_withheld);
    assert_eq!(status.repair_version.as_deref(), Some("B"));
    assert!(
        !app.instance_manager()
            .repair_preview(id)
            .await?
            .has_packinfo
    );
    let instance_before = legacy_tree(&root.join("instance"));
    app.instance_manager()
        .repair_modpack(
            id,
            super::modpack::RepairMarkerFile::default(),
            status.discard_token.clone(),
        )
        .await?;
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(
        retired[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".setup-cancelled-legacy-")
    );
    assert!(retired[0].join("change-pack-version.json").is_file());
    assert!(!retired[0].join("modrinth").exists());
    assert!(!retired[0].join("staging").exists());
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    let repair = Session::load(&root).await?.expect("the repair's setup");
    assert_eq!(repair.producer, Producer::Repair);
    assert_eq!(repairblocked_requested(&repair)?, "B");
    assert!(repair.replaces_interrupted.is_none());
    app.instance_manager()
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Inactive { failed_task: None };
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(!status.preview_withheld);
    assert!(!status.can_cancel);
    Ok(())
}

/// A Repair that failed while applying (a mod file locked by a game that
/// crashed, a read-only file) keeps only its records when the next Repair
/// replaces it: its downloads are fresh and nothing reads them again, and
/// what its apply displaced is in `.repair-recovery`. The next Repair still
/// continues it, as its files may be part-way.
#[tokio::test]
async fn repairblocked_repair_that_failed_applying_keeps_only_its_records() -> anyhow::Result<()> {
    use super::modpack::session::{Phase, Producer, RECORD, Session};
    let app = crate::setup_managers_for_test().await;
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let (id, root, _) =
        repairblocked_resolved_change(&app, "repairblocked-repair-apply-failed").await?;
    let status = app.instance_manager().pending_setup(id).await?;
    app.instance_manager()
        .repair_modpack(
            id,
            super::modpack::RepairMarkerFile::default(),
            status.discard_token.clone(),
        )
        .await?;
    let first = Session::load(&root).await?.expect("the first repair");
    assert_eq!(first.producer, Producer::Repair);
    assert!(first.replaces_interrupted.is_none());
    // It downloads and stages A, then its apply fails part-way.
    repairblocked_archive_b(&root)?;
    let mut first_owner = Some(first.owner()?);
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &legacy_task(),
        false,
        &mut first_owner,
    )
    .await?;
    let mut applying = Session::require(&root, first_owner.as_ref().unwrap()).await?;
    applying.phase = Phase::Applying;
    applying.save(&root, first_owner.as_ref().unwrap()).await?;
    tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
    tokio::fs::create_dir(root.join(".setup/representation-transitions")).await?;
    tokio::fs::write(
        root.join(".setup/representation-transitions/transition.json"),
        b"{\"evidence\":true}",
    )
    .await?;
    tokio::fs::create_dir_all(root.join(".setup/preparing-earlier/instance/mods")).await?;
    tokio::fs::write(
        root.join(".setup/preparing-earlier/instance/mods/new.jar"),
        b"part",
    )
    .await?;
    tokio::fs::write(root.join("instance/mods/pack.jar"), b"B").await?;
    app.instance_manager()
        .instances
        .write()
        .await
        .get_mut(&id)
        .unwrap()
        .data_mut()?
        .state = LaunchState::Inactive { failed_task: None };
    let status = app.instance_manager().pending_setup(id).await?;
    assert_eq!(status.operation.as_deref(), Some(first.operation.as_str()));
    assert!(status.preview_withheld);
    let target_before = tokio::fs::read(root.join("tmp-packinfo.json")).await?;
    let instance_before = legacy_tree(&root.join("instance"));

    app.instance_manager()
        .repair_modpack(
            id,
            super::modpack::RepairMarkerFile::default(),
            status.discard_token.clone(),
        )
        .await?;
    let failed = legacy_retired(&root)?
        .into_iter()
        .find(|retired| {
            retired
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(".setup-cancelled-{}-", first.operation))
        })
        .expect("the failed repair is retired");
    let mut kept: Vec<_> = std::fs::read_dir(&failed)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<anyhow::Result<_>>()?;
    kept.sort();
    assert_eq!(
        kept,
        [
            "apply-started",
            "change-pack-version.json",
            // An empty marker.
            "modpack-skip-overrides",
            "repair",
            "representation-transitions",
            RECORD,
            "staging-packinfo.json",
            "tmp-packinfo.json",
        ]
    );
    assert_eq!(
        tokio::fs::read(failed.join("representation-transitions/transition.json")).await?,
        b"{\"evidence\":true}"
    );
    assert_eq!(
        tokio::fs::read(failed.join("tmp-packinfo.json")).await?,
        target_before
    );
    assert_eq!(legacy_retired_session(&failed)?.phase, Phase::Applying);
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    // The cancelled update it replaced never changed anything either.
    assert!(legacy_retired(&root)?.iter().all(|retired| {
        !retired.join("modrinth").exists() && !retired.join("staging").exists()
    }));
    let second = Session::load(&root).await?.expect("the second repair");
    assert_eq!(second.producer, Producer::Repair);
    assert_eq!(repairblocked_requested(&second)?, "A");
    assert_eq!(
        second.replaces_interrupted.as_deref(),
        Some(first.operation.as_str())
    );
    Ok(())
}

/// When neither the replacement can take the pending setup's place nor the
/// retired setup can be put back (two failed renames, say an instance folder
/// that briefly turned read-only), the complete replacement stays: the next
/// look at the instance publishes it, so there is never no setup where one
/// that may have changed files was.
#[cfg(unix)]
#[tokio::test]
async fn repairblocked_unrestorable_setup_keeps_its_replacement() -> anyhow::Result<()> {
    use super::modpack::session::{Producer, Session};
    use std::os::unix::fs::PermissionsExt;
    let app = crate::setup_managers_for_test().await;
    let (id, root, owner) =
        repairblocked_resolved_change(&app, "repairblocked-unrestorable").await?;
    tokio::fs::write(root.join(".setup/apply-started"), b"").await?;
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.preview_withheld);
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("setup_replacement_retired", id);
    let repair = manager.repair_modpack(
        id,
        super::modpack::RepairMarkerFile::default(),
        status.discard_token.clone(),
    );
    tokio::pin!(repair);
    tokio::select! {
        _ = pause.entered.notified() => {},
        result = &mut repair => panic!("the repair did not reach publication: {result:?}"),
    }
    let permissions = std::fs::metadata(&root)?.permissions();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o555))?;
    pause.release.notify_one();
    let result = repair.await;
    std::fs::set_permissions(&root, permissions)?;
    let error = result.expect_err("neither rename can happen");
    assert!(format!("{error:#}").contains("press Play"), "{error:#}");
    assert!(!root.join(".setup").exists());
    assert_eq!(legacy_retired(&root)?.len(), 1);

    // A version or loader change publishes the kept replacement first, then
    // waits for it like any pending setup, without touching instance.json.
    let config = tokio::fs::read(root.join("instance.json")).await?;
    let refusal = app
        .instance_manager()
        .update_instance(versiononly_edit(id, Some("1.20.4"), None))
        .await
        .expect_err("the published repair is pending");
    assert_eq!(
        refusal.to_string(),
        "This instance has an unfinished install or update. Press Play to finish it before changing the game version or loader."
    );
    assert_eq!(tokio::fs::read(root.join("instance.json")).await?, config);
    assert!(root.join(".setup").is_dir());

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.pending);
    let repair = Session::load(&root)
        .await?
        .expect("the replacement took its place");
    assert_eq!(status.operation.as_deref(), Some(repair.operation.as_str()));
    assert_eq!(repair.producer, Producer::Repair);
    assert_eq!(
        repair.replaces_interrupted.as_deref(),
        Some(owner.operation.as_str())
    );
    assert!(status.preview_withheld);
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    assert!(retired[0].join("apply-started").is_file());
    assert!(std::fs::read_dir(&root)?.all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".setup-next-")
    }));
    Ok(())
}

/// An owned update to `version`, accepted after Repair's dialog opened, that
/// stopped before its archive arrived.
async fn repairblocked_pending_change(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
    version: &str,
) -> anyhow::Result<super::modpack::session::Session> {
    use super::modpack::session::{Kind, Producer, Session};
    let (_, config) = app.instance_manager().current_config(id).await?;
    tokio::fs::create_dir(root.join(".setup")).await?;
    let mut session = Session::new(&config, Producer::Change, Kind::Pack, None)?;
    session.instance_id = Some(id.0);
    session.requested_pack = Some(repairblocked_request(version)?);
    session.create(root).await?;
    tokio::fs::write(
        root.join(".setup/change-pack-version.json"),
        repairblocked_request(version)?,
    )
    .await?;
    Ok(session)
}

/// GPT-0214: a Repair confirmed in a dialog that went stale never discards a
/// setup accepted after it opened. Only the dialog that shows a setup
/// discards it, and a refusal changes nothing.
#[tokio::test]
async fn repairblocked_stale_dialog_never_discards_the_pending_setup() -> anyhow::Result<()> {
    use super::modpack::session::Session;
    let app = crate::setup_managers_for_test().await;
    let (id, root) = repairblocked_installed(&app, "repairblocked-stale", "A").await?;
    let opened = app.instance_manager().pending_setup(id).await?;
    assert!(!opened.pending);
    assert_eq!(opened.discard_token.clone(), None);

    let first = repairblocked_pending_change(&app, id, &root, "B").await?;
    let shown = app
        .instance_manager()
        .pending_setup(id)
        .await?
        .discard_token
        .clone();
    assert_eq!(shown.as_deref(), Some(first.operation.as_str()));
    let before = legacy_tree(&root);
    for stale in [
        None,
        Some(uuid::Uuid::new_v4().to_string()),
        Some("legacy".to_owned()),
    ] {
        let error = app
            .instance_manager()
            .repair_modpack(id, super::modpack::RepairMarkerFile::default(), stale)
            .await
            .expect_err("a stale dialog never discards the pending update");
        assert!(
            error
                .to_string()
                .contains("changed after Repair was opened"),
            "{error}"
        );
        assert_eq!(legacy_tree(&root), before);
    }

    // The first update was cancelled and another accepted: the dialog that
    // showed the first never discards the second.
    app.instance_manager()
        .cancel_pending_setup(id, first.operation.clone())
        .await?;
    let second = repairblocked_pending_change(&app, id, &root, "C").await?;
    let before = legacy_tree(&root);
    assert!(
        app.instance_manager()
            .repair_modpack(id, super::modpack::RepairMarkerFile::default(), shown)
            .await
            .is_err()
    );
    assert_eq!(legacy_tree(&root), before);

    // The dialog that shows it discards it, and repairs the installed pack.
    let manager = app.instance_manager();
    let _permit = manager
        .persistence_manager
        .instance_download_lock
        .acquire()
        .await?;
    let token = app
        .instance_manager()
        .pending_setup(id)
        .await?
        .discard_token
        .clone();
    assert_eq!(token.as_deref(), Some(second.operation.as_str()));
    app.instance_manager()
        .repair_modpack(id, super::modpack::RepairMarkerFile::default(), token)
        .await?;
    assert!(legacy_retired(&root)?.iter().any(|retired| {
        retired
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!(".setup-cancelled-{}-", second.operation))
    }));
    let repair = Session::load(&root).await?.expect("the repair's setup");
    assert_eq!(repairblocked_requested(&repair)?, "A");
    Ok(())
}

/// While the instance is queued or running, Repair changes nothing, even
/// for the setup the dialog showed.
#[tokio::test]
async fn repairblocked_busy_instance_keeps_its_setup() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root, _) = repairblocked_resolved_change(&app, "repairblocked-busy").await?;
    let status = app.instance_manager().pending_setup(id).await?;
    assert!(!status.busy);
    let token = status.discard_token.clone();
    assert!(token.is_some());
    for state in [
        LaunchState::Queued(VisualTaskId(1)),
        LaunchState::Running(RunningInstance::adopted(
            u32::MAX,
            1,
            Utc::now(),
            Utc::now(),
        )),
    ] {
        app.instance_manager()
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()?
            .state = state;
        let before = legacy_tree(&root);
        // The dialog says so and offers nothing to click.
        let status = app.instance_manager().pending_setup(id).await?;
        assert!(status.busy);
        assert!(!status.can_cancel);
        assert!(!status.can_resume);
        let error = app
            .instance_manager()
            .repair_modpack(
                id,
                super::modpack::RepairMarkerFile::default(),
                token.clone(),
            )
            .await
            .expect_err("a busy instance is never repaired");
        assert!(error.to_string().contains("Cannot repair while"), "{error}");
        assert_eq!(legacy_tree(&root), before);
        assert!(legacy_retired(&root)?.is_empty());
    }
    Ok(())
}

/// A Modrinth pack whose `downloads` come from `server` and whose
/// `overrides` are in the archive, already downloaded into `.setup`.
fn resume_pack(
    root: &Path,
    server: &str,
    downloads: &[(&str, &[u8])],
    overrides: &[(&str, &[u8])],
) -> anyhow::Result<()> {
    use sha2::Digest;
    use std::io::Write;
    let files: Vec<_> = downloads
        .iter()
        .map(|(path, bytes)| {
            serde_json::json!({
                "path": path,
                "hashes": {
                    "sha1": hex::encode(sha1::Sha1::digest(bytes)),
                    "sha512": hex::encode(sha2::Sha512::digest(bytes)),
                },
                "env": {"client": "required", "server": "required"},
                "downloads": [format!("{server}/{path}")],
                "fileSize": bytes.len(),
            })
        })
        .collect();
    let index = serde_json::json!({"formatVersion":1,"game":"minecraft","versionId":"synthetic-B","name":"fixture","files":files,"dependencies":{"minecraft":"1.20.1"}});
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join(".setup/modrinth"))?);
    zip.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    zip.write_all(&serde_json::to_vec(&index)?)?;
    for (path, bytes) in overrides {
        zip.start_file(
            format!("overrides/{path}"),
            zip::write::SimpleFileOptions::default(),
        )?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;
    Ok(())
}

/// The first install of a pack from Browse, as Create leaves it once the
/// pack archive arrived: a Reserved pack setup with no target.
async fn resume_first_install(
    app: &crate::App,
    name: &str,
    server: &str,
    downloads: &[(&str, &[u8])],
) -> anyhow::Result<(InstanceId, PathBuf)> {
    use super::modpack::session::{Kind, Producer, Session};
    let (id, root) = fixture(app, name).await?;
    tokio::fs::remove_dir_all(root.join(".setup")).await?;
    tokio::fs::create_dir(root.join(".setup")).await?;
    let (_, config) = app.instance_manager().current_config(id).await?;
    let mut session = Session::new(&config, Producer::Create, Kind::Pack, None)?;
    session.instance_id = Some(id.0);
    session.create(&root).await?;
    resume_pack(
        &root,
        server,
        downloads,
        &[("config/pack.json", b"{\"pack\":true}")],
    )?;
    Ok((id, root))
}

fn preparing_dirs(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(root.join(".setup"))? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("preparing-")
        {
            dirs.push(entry.path());
        }
    }
    Ok(dirs)
}

/// One Play of `id` that stops, as a quit or a cancel does, once its pack
/// files are about to download, after `downloaded` already landed in its
/// preparation folder. Returns that folder.
async fn resume_interrupted_attempt(
    app: &crate::App,
    id: InstanceId,
    root: &Path,
    downloaded: &[(&str, &[u8])],
) -> anyhow::Result<PathBuf> {
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let pause = app
        .instance_manager
        .test_hooks
        .arm("setup_original_acquisition", id);
    let task = legacy_task();
    let mut owner = None;
    let prepare = run::modpack::process_modpack_cancellable(
        app.clone(),
        id,
        true,
        config,
        shortpath,
        &task,
        false,
        &mut owner,
        &cancel,
    );
    tokio::pin!(prepare);
    tokio::select! {
        _ = pause.entered.notified() => {}
        result = &mut prepare => {
            anyhow::bail!("preparation settled before its downloads: {:?}", result.err());
        }
    }
    let folders = preparing_dirs(root)?;
    assert_eq!(folders.len(), 1, "{folders:?}");
    for (path, bytes) in downloaded {
        let file = folders[0].join("instance").join(path);
        tokio::fs::create_dir_all(file.parent().unwrap()).await?;
        tokio::fs::write(file, bytes).await?;
    }
    cancel.cancel();
    pause.release.notify_one();
    assert!(prepare.await.is_err(), "the interrupted attempt failed");
    Ok(folders[0].clone())
}

/// A first install that stopped part-way through its downloads (a quit, a
/// cancel, a network error) keeps what it downloaded: the next Play
/// downloads only the rest and leaves no second copy behind.
#[tokio::test]
async fn an_interrupted_first_install_downloads_only_what_it_was_missing() -> anyhow::Result<()> {
    use super::modpack::session::{Phase, Session};
    const A: &[u8] = b"first mod, finished before the quit";
    const B: &[u8] = b"second mod, still to download";
    let app = crate::setup_managers_for_test().await;
    let mut server = mockito::Server::new_async().await;
    let a = server
        .mock("GET", "/mods/a.jar")
        .with_body(A)
        .expect(0)
        .create_async()
        .await;
    let b = server
        .mock("GET", "/mods/b.jar")
        .with_body(B)
        .expect(1)
        .create_async()
        .await;
    let (id, root) = resume_first_install(
        &app,
        "resume-first-install",
        &server.url(),
        &[("mods/a.jar", A), ("mods/b.jar", B)],
    )
    .await?;
    let first = resume_interrupted_attempt(&app, id, &root, &[("mods/a.jar", A)]).await?;
    assert!(first.join("instance/mods/a.jar").is_file());

    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let mut owner = None;
    run::modpack::process_modpack(
        app.clone(),
        id,
        false,
        config,
        shortpath,
        &legacy_task(),
        false,
        &mut owner,
    )
    .await?;

    a.assert_async().await;
    b.assert_async().await;
    assert_eq!(preparing_dirs(&root)?, Vec::<PathBuf>::new());
    let staged = root.join(".setup/staging/instance");
    assert_eq!(tokio::fs::read(staged.join("mods/a.jar")).await?, A);
    assert_eq!(tokio::fs::read(staged.join("mods/b.jar")).await?, B);
    let session = Session::load(&root).await?.unwrap();
    assert_eq!(session.phase, Phase::Resolved);
    let target = session.target.unwrap();
    assert!(target.sources.contains_key("/mods/a.jar"));
    assert!(target.sources.contains_key("/mods/b.jar"));
    Ok(())
}

/// A file an earlier attempt left is installed only when its bytes are the
/// pack's, even on a Play that would otherwise trust a file of the right
/// size: one that changed since is downloaded again.
#[tokio::test]
async fn a_reused_download_whose_bytes_changed_is_downloaded_again() -> anyhow::Result<()> {
    const A: &[u8] = b"the pack's own bytes";
    let app = crate::setup_managers_for_test().await;
    let mut server = mockito::Server::new_async().await;
    let a = server
        .mock("GET", "/mods/a.jar")
        .with_body(A)
        .expect(1)
        .create_async()
        .await;
    let (id, root) = resume_first_install(
        &app,
        "resume-changed-reuse",
        &server.url(),
        &[("mods/a.jar", A)],
    )
    .await?;
    let damaged = vec![b'x'; A.len()];
    resume_interrupted_attempt(&app, id, &root, &[("mods/a.jar", &damaged)]).await?;

    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let mut owner = None;
    run::modpack::process_modpack(
        app.clone(),
        id,
        false,
        config,
        shortpath,
        &legacy_task(),
        false,
        &mut owner,
    )
    .await?;

    a.assert_async().await;
    assert_eq!(
        tokio::fs::read(root.join(".setup/staging/instance/mods/a.jar")).await?,
        A
    );
    assert_eq!(preparing_dirs(&root)?, Vec::<PathBuf>::new());
    Ok(())
}

/// An attempt that fails before its downloads start (here, a pack file
/// without a usable checksum) holds only overrides it extracted, so it
/// leaves no folder: repeated failures, a full disk among them, don't pile
/// up copies.
#[tokio::test]
async fn a_preparation_failing_before_its_downloads_leaves_no_folder() -> anyhow::Result<()> {
    use std::io::Write;
    let app = crate::setup_managers_for_test().await;
    let (id, root) =
        resume_first_install(&app, "resume-early-failure", "http://127.0.0.1:9", &[]).await?;
    let index = serde_json::json!({"formatVersion":1,"game":"minecraft","versionId":"synthetic-B","name":"fixture",
        "files":[{"path":"mods/a.jar","hashes":{"sha1":"","sha512":""},"downloads":["http://127.0.0.1:9/a.jar"],"fileSize":1}],
        "dependencies":{"minecraft":"1.20.1"}});
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join(".setup/modrinth"))?);
    zip.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    zip.write_all(&serde_json::to_vec(&index)?)?;
    zip.start_file(
        "overrides/config/pack.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    zip.write_all(b"{}")?;
    zip.finish()?;

    for _ in 0..2 {
        let (shortpath, config) = app.instance_manager().current_config(id).await?;
        let mut owner = None;
        let error = run::modpack::process_modpack(
            app.clone(),
            id,
            true,
            config,
            shortpath,
            &legacy_task(),
            false,
            &mut owner,
        )
        .await
        .err()
        .expect("a pack file without a checksum is refused");
        assert!(format!("{error:#}").contains("SHA512 or SHA1"), "{error:#}");
        assert_eq!(preparing_dirs(&root)?, Vec::<PathBuf>::new());
    }
    Ok(())
}

/// A Repair skips a mod that is intact when it checks the files. When that
/// mod goes missing before the apply (antivirus, or a Repair interrupted and
/// retried later), the apply refuses it with a reason, changes nothing, and
/// sends the setup back to be prepared again: the next Play downloads the mod
/// and restores it, instead of failing the same way on every Play.
#[tokio::test]
async fn a_repair_whose_skipped_file_vanished_is_prepared_again() -> anyhow::Result<()> {
    use super::modpack::{
        RepairMarkerFile, packinfo,
        session::{Phase, Session},
    };
    const A: &[u8] = b"the pack's mod";
    let app = crate::setup_managers_for_test().await;
    let mut server = mockito::Server::new_async().await;
    let a = server
        .mock("GET", "/mods/a.jar")
        .with_body(A)
        .expect(1)
        .create_async()
        .await;
    let (id, root) = fixture(&app, "repair-skipped-vanished").await?;
    // An older producer's setup: no record, reconstructed from the archive.
    tokio::fs::remove_file(root.join(".setup/setup-session.json")).await?;
    resume_pack(&root, &server.url(), &[("mods/a.jar", A)], &[])?;
    tokio::fs::create_dir_all(root.join("instance/mods")).await?;
    tokio::fs::write(root.join("instance/mods/a.jar"), A).await?;
    let installed = packinfo::scan_dir(&root.join("instance"), None).await?;
    tokio::fs::write(
        root.join("packinfo.json"),
        packinfo::make_packinfo(installed)?,
    )
    .await?;
    tokio::fs::write(
        root.join(".setup/repair"),
        serde_json::to_vec(&RepairMarkerFile::default())?,
    )
    .await?;
    let mut owner = None;
    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &legacy_task(),
        false,
        &mut owner,
    )
    .await?;
    let bound = Session::load(&root).await?.unwrap();
    assert_eq!(bound.target.unwrap().skipped, vec!["/mods/a.jar"]);
    tokio::fs::remove_file(root.join("instance/mods/a.jar")).await?;

    let error = run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath.clone(),
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await
    .err()
    .expect("nothing staged can restore the vanished mod");
    assert!(
        error.to_string().starts_with(
            "mods/a.jar was changed or removed after this modpack's files were checked"
        ),
        "{error:#}"
    );
    let released = Session::load(&root).await?.unwrap();
    assert_eq!(released.phase, Phase::Reserved);
    assert!(released.target.is_none());
    assert!(!root.join("tmp-packinfo.json").exists());
    assert!(!root.join(".setup/staging").exists());
    assert_eq!(preparing_dirs(&root)?.len(), 1);
    assert!(!root.join(".setup/apply-started").exists());

    let (shortpath, config) = app.instance_manager().current_config(id).await?;
    let (subtasks, _, options, oracle) = run::modpack::process_modpack(
        app.clone(),
        id,
        true,
        config,
        shortpath.clone(),
        &legacy_task(),
        false,
        &mut owner,
    )
    .await?;
    a.assert_async().await;
    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        oracle,
        owner.as_ref(),
    )
    .await?;
    assert_eq!(tokio::fs::read(root.join("instance/mods/a.jar")).await?, A);
    assert_eq!(preparing_dirs(&root)?, Vec::<PathBuf>::new());
    Ok(())
}

/// A pending change sent back to be prepared again is still one the user can
/// cancel: the target record it released is kept as evidence, not left
/// where cancelling would refuse it as unbound.
#[tokio::test]
async fn a_released_change_can_still_be_cancelled() -> anyhow::Result<()> {
    use super::modpack::session::{Phase, Session};
    let app = crate::setup_managers_for_test().await;
    let (id, root, owner) = repairblocked_resolved_change(&app, "released-change").await?;
    let mut session = Session::require(&root, &owner).await?;
    session.release_target(&root, &owner).await?;
    assert_eq!(
        Session::require(&root, &owner).await?.phase,
        Phase::Reserved
    );
    assert!(!root.join("tmp-packinfo.json").exists());
    assert!(!root.join(".setup/staging").exists());
    assert_eq!(preparing_dirs(&root)?.len(), 1);
    let instance_before = legacy_tree(&root.join("instance"));

    let status = app.instance_manager().pending_setup(id).await?;
    assert!(status.can_cancel, "{:?}", status.reason);
    app.instance_manager()
        .cancel_pending_setup(id, owner.operation.clone())
        .await?;
    assert!(!root.join(".setup").exists());
    let retired = legacy_retired(&root)?;
    assert_eq!(retired.len(), 1);
    let kept: Vec<_> = std::fs::read_dir(&retired[0])?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<anyhow::Result<_>>()?;
    assert!(
        kept.iter()
            .any(|name| name.starts_with("prior-record-tmp-packinfo.json-")),
        "{kept:?}"
    );
    assert!(
        !kept.iter().any(|name| name.starts_with("preparing-")),
        "{kept:?}"
    );
    assert_eq!(legacy_tree(&root.join("instance")), instance_before);
    Ok(())
}

/// One preparation reads its pack archive in full once: the proof it binds
/// is not hashed again when nothing wrote the archive since.
#[tokio::test]
async fn a_preparation_hashes_its_archive_once() -> anyhow::Result<()> {
    use super::modpack::origin_check::hash_count;
    let app = crate::setup_managers_for_test().await;
    let (_, root, ..) = prepared_pack(&app, "archive-hashed-once", config_files(3)).await?;
    let archive = root.join(".setup/modrinth");
    assert_eq!(
        hash_count::under(&root.join(".setup")).get(&archive),
        Some(&1)
    );
    Ok(())
}

/// A pack that ships a file both enabled and as its `.disabled` twin
/// installs the enabled one, as 2.0.40 did, instead of refusing the pack.
#[tokio::test]
async fn a_pack_shipping_a_file_enabled_and_disabled_installs_the_enabled_one() -> anyhow::Result<()>
{
    use super::modpack::{packinfo, session::Session};
    let app = crate::setup_managers_for_test().await;
    let files = vec![
        ("mods/both.jar".to_string(), b"enabled".to_vec()),
        (
            "mods/both.jar.disabled".to_string(),
            b"disabled copy".to_vec(),
        ),
        ("config/pack.json".to_string(), b"{}".to_vec()),
    ];
    let (id, root, shortpath, _, subtasks, options, owner) =
        prepared_pack(&app, "enabled-and-disabled", files).await?;
    let target = Session::load(&root).await?.unwrap().target.unwrap();
    assert_eq!(
        target.sources.get("/mods/both.jar").map(String::as_str),
        Some("/mods/both.jar")
    );
    let recorded = packinfo::parse_packinfo(&target.packinfo)?;
    let enabled = packinfo::scan_dir(&root.join(".setup/staging/instance"), None).await?;
    assert_eq!(
        recorded.files["/mods/both.jar"].sha512,
        enabled.files["/mods/both.jar"].sha512
    );
    assert!(
        !root
            .join(".setup/staging/instance/mods/both.jar.disabled")
            .exists()
    );

    run::modpack::process_modpack_staging(
        app.clone(),
        id,
        shortpath,
        &subtasks,
        options,
        None,
        owner.as_ref(),
    )
    .await?;
    assert_eq!(
        tokio::fs::read(root.join("instance/mods/both.jar")).await?,
        b"enabled"
    );
    assert!(!root.join("instance/mods/both.jar.disabled").exists());
    Ok(())
}

async fn without_recycle_bin(app: &crate::App) -> anyhow::Result<()> {
    app.settings_manager()
        .set(
            carbon_repos::repos::app_configuration::AppConfigurationPatch {
                deletion_through_recycle_bin: Some(false),
                ..Default::default()
            },
        )
        .await?;
    Ok(())
}

/// Waits until `operation` reaches the armed `pause`.
async fn reach<F: Future>(pause: &Pause, operation: std::pin::Pin<&mut F>)
where
    F::Output: std::fmt::Debug,
{
    tokio::select! {
        _ = pause.entered.notified() => {},
        result = operation => panic!("operation finished before its pause: {result:?}"),
    }
}

const PROMPTLY: Duration = Duration::from_secs(10);

#[tokio::test]
async fn deleting_an_instance_holds_up_no_other_instance() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await?;
    let (deleted, deleted_root) = fixture(&app, "deleted").await?;
    let (other_deleted, other_root) = fixture(&app, "also-deleted").await?;
    let (renamed, _) = fixture(&app, "renamed").await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("delete", deleted);
    let delete = manager._delete_instance(deleted);
    tokio::pin!(delete);
    reach(&pause, delete.as_mut()).await;
    assert!(matches!(
        manager.instances.read().await[&deleted].data()?.state,
        LaunchState::Deleting
    ));

    // While the first folder is still being removed, everything else on other
    // instances goes ahead: creating, renaming, duplicating, and a second
    // delete of the same batch.
    let (created, created_root) = tokio::time::timeout(PROMPTLY, fixture(&app, "deleted"))
        .await
        .expect("creating waited for another instance's delete")?;
    assert_eq!(
        created_root.file_name().unwrap(),
        "deleted1",
        "the folder being deleted still reserves its name"
    );
    let mut edit = update(renamed);
    edit.name = Some("renamed-again".into());
    tokio::time::timeout(PROMPTLY, manager.update_instance(edit))
        .await
        .expect("renaming waited for another instance's delete")?;
    let copy = tokio::time::timeout(PROMPTLY, manager.duplicate_instance(renamed, "copy".into()))
        .await
        .expect("duplicating waited for another instance's delete")?;
    tokio::time::timeout(PROMPTLY, manager._delete_instance(other_deleted))
        .await
        .expect("a batch delete waited for the one before it")?;
    assert!(!other_root.exists());

    pause.release.notify_one();
    delete.await?;
    assert!(!deleted_root.exists());
    assert!(
        instance_repo::get_instance(&app.db, deleted.0)
            .await?
            .is_none()
    );
    assert!(!manager.instances.read().await.contains_key(&deleted));
    for id in [created, renamed, copy] {
        assert!(instance_repo::get_instance(&app.db, id.0).await?.is_some());
        assert!(manager.instances.read().await.contains_key(&id));
    }
    assert!(manager.instance_op_locks.is_empty());
    assert!(manager.instance_root_locks.is_empty());
    Ok(())
}

#[tokio::test]
async fn duplicating_an_instance_holds_up_nothing_on_the_original() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "original").await?;
    tokio::fs::write(root.join("instance/world.dat"), b"world").await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("duplicate_copy", id);
    let duplicate = manager.duplicate_instance(id, "copy".into());
    tokio::pin!(duplicate);
    reach(&pause, duplicate.as_mut()).await;

    // Play, the game's playtime and exit, the Logs tab and settings all take
    // the original's gate: none of them waits for the copy.
    let gate = tokio::time::timeout(PROMPTLY, manager.lock_instance(id))
        .await
        .expect("the copy held the original's gate");
    drop(gate);
    tokio::time::timeout(PROMPTLY, manager.update_playtime(id, 60))
        .await
        .expect("playtime waited for the copy")?;
    tokio::time::timeout(PROMPTLY, manager.get_logs(id))
        .await
        .expect("the Logs tab waited for the copy");
    let mut edit = update(id);
    edit.notes = Some("edited during the copy".into());
    tokio::time::timeout(PROMPTLY, manager.update_instance(edit))
        .await
        .expect("settings waited for the copy")?;

    pause.release.notify_one();
    let copy = duplicate.await?;
    let row = instance_repo::get_instance(&app.db, copy.0).await?.unwrap();
    let copy_root = root.with_file_name(row.shortpath);
    assert_eq!(
        tokio::fs::read(copy_root.join("instance/world.dat")).await?,
        b"world"
    );
    assert_eq!(disk_config(&copy_root).await?.name, "copy");
    assert_eq!(disk_config(&root).await?.seconds_played, 60);
    assert!(manager.instance_root_locks.is_empty());
    Ok(())
}

#[tokio::test]
async fn deleting_or_renaming_an_original_waits_for_its_copy() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await?;
    let (renamed, renamed_root) = fixture(&app, "renamed-source").await?;
    let (deleted, deleted_root) = fixture(&app, "deleted-source").await?;
    tokio::fs::write(deleted_root.join("instance/world.dat"), b"kept").await?;
    let manager = app.instance_manager();

    let pause = manager.test_hooks.arm("duplicate_copy", renamed);
    let duplicate = manager.duplicate_instance(renamed, "renamed copy".into());
    tokio::pin!(duplicate);
    reach(&pause, duplicate.as_mut()).await;
    let mut edit = update(renamed);
    edit.name = Some("renamed-later".into());
    let rename = manager.update_instance(edit);
    tokio::pin!(rename);
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut rename)
            .await
            .is_err(),
        "the folder moved while it was being copied"
    );
    assert!(renamed_root.exists());
    pause.release.notify_one();
    let copy = duplicate.await?;
    rename.await?;
    assert!(!renamed_root.exists());
    assert!(renamed_root.with_file_name("renamed-later").exists());
    assert!(
        instance_repo::get_instance(&app.db, copy.0)
            .await?
            .is_some()
    );

    // The user asked for both: the copy is made, then the original goes.
    let pause = manager.test_hooks.arm("duplicate_copy", deleted);
    let duplicate = manager.duplicate_instance(deleted, "kept copy".into());
    tokio::pin!(duplicate);
    reach(&pause, duplicate.as_mut()).await;
    let delete = manager._delete_instance(deleted);
    tokio::pin!(delete);
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut delete)
            .await
            .is_err(),
        "the folder was deleted while it was being copied"
    );
    assert!(matches!(
        manager.instances.read().await[&deleted].data()?.state,
        LaunchState::Deleting
    ));
    assert!(deleted_root.exists());
    pause.release.notify_one();
    let copy = duplicate.await?;
    delete.await?;
    assert!(!deleted_root.exists());
    let row = instance_repo::get_instance(&app.db, copy.0).await?.unwrap();
    assert_eq!(
        tokio::fs::read(
            deleted_root
                .with_file_name(row.shortpath)
                .join("instance/world.dat")
        )
        .await?,
        b"kept"
    );
    assert!(manager.instance_op_locks.is_empty());
    assert!(manager.instance_root_locks.is_empty());
    Ok(())
}

#[tokio::test]
async fn an_instance_with_a_broken_instance_json_can_be_unfavorited_and_deleted()
-> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await?;
    let manager = app.instance_manager();
    let (broken, broken_root) = fixture(&app, "broken").await?;
    manager.set_favorite(broken, true).await?;
    tokio::fs::write(broken_root.join("instance.json"), "{").await?;
    let (missing, missing_root) = fixture(&app, "missing").await?;
    tokio::fs::remove_file(missing_root.join("instance.json")).await?;
    manager.instances.write().await.clear();
    manager.scan_instances().await?;

    manager.set_favorite(broken, false).await?;
    assert!(
        !instance_repo::get_instance(&app.db, broken.0)
            .await?
            .unwrap()
            .favorite
    );
    manager._delete_instance(broken).await?;
    assert!(!broken_root.exists());
    assert!(
        instance_repo::get_instance(&app.db, broken.0)
            .await?
            .is_none()
    );
    assert!(!manager.instances.read().await.contains_key(&broken));

    // Its folder already went outside the launcher: only the entry is left.
    tokio::fs::remove_dir_all(&missing_root).await?;
    manager._delete_instance(missing).await?;
    assert!(
        instance_repo::get_instance(&app.db, missing.0)
            .await?
            .is_none()
    );
    assert!(!manager.instances.read().await.contains_key(&missing));
    let groups = manager.list_groups().await?;
    assert!(groups.iter().all(|group| group.instances.is_empty()));
    Ok(())
}

#[tokio::test]
async fn deleting_a_folder_with_its_instances_finishes_past_a_broken_one() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await?;
    let manager = app.instance_manager();
    let group = manager.create_group("pack folder".into()).await?;
    let mut members = Vec::new();
    for name in ["first", "broken member", "last"] {
        let id = manager
            .create_instance(
                group,
                name.into(),
                false,
                InstanceVersionSource::Version(GameVersion::Standard(info::StandardVersion {
                    release: "1.7.10".into(),
                    modloaders: HashSet::new(),
                })),
                String::new(),
            )
            .await?;
        let row = instance_repo::get_instance(&app.db, id.0).await?.unwrap();
        members.push((id, row.shortpath));
    }
    let instances_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path();
    tokio::fs::write(
        instances_root.join(&members[1].1).join("instance.json"),
        "not json",
    )
    .await?;
    manager.instances.write().await.clear();
    manager.scan_instances().await?;

    manager._delete_group_with_instances(group).await?;
    for (id, shortpath) in &members {
        assert!(!instances_root.join(shortpath).exists());
        assert!(instance_repo::get_instance(&app.db, id.0).await?.is_none());
    }
    assert!(instance_repo::get_group(&app.db, group.0).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn a_failed_playtime_write_is_saved_by_the_next_one() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "unsaved-playtime").await?;
    let manager = app.instance_manager();
    let json = tokio::fs::read_to_string(root.join("instance.json")).await?;
    tokio::fs::remove_file(root.join("instance.json")).await?;
    tokio::fs::create_dir(root.join("instance.json")).await?;
    assert!(manager.update_playtime(id, 60).await.is_err());
    // The library shows only what instance.json holds.
    assert_eq!(
        manager.instances.read().await[&id]
            .data()?
            .config
            .seconds_played,
        0
    );
    tokio::fs::remove_dir(root.join("instance.json")).await?;
    tokio::fs::write(root.join("instance.json"), json).await?;
    manager.update_playtime(id, 10).await?;
    assert_eq!(disk_config(&root).await?.seconds_played, 70);
    assert_eq!(
        manager.instances.read().await[&id]
            .data()?
            .config
            .seconds_played,
        70
    );
    manager.update_playtime(id, 5).await?;
    assert_eq!(disk_config(&root).await?.seconds_played, 75);
    Ok(())
}

#[tokio::test]
async fn opening_the_logs_tab_holds_up_nothing_while_past_logs_load() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (id, root) = fixture(&app, "logs").await?;
    tokio::fs::create_dir_all(root.join("logs")).await?;
    tokio::fs::write(
        root.join("logs/2025-01-14_12-00-00.log"),
        b"retained history\n",
    )
    .await?;
    let manager = app.instance_manager();
    let pause = manager.test_hooks.arm("log_history", id);
    let logs = manager.get_logs(id);
    tokio::pin!(logs);
    reach(&pause, logs.as_mut()).await;
    tokio::time::timeout(PROMPTLY, manager.update_playtime(id, 60))
        .await
        .expect("playtime waited for past logs to load")?;
    let gate = tokio::time::timeout(PROMPTLY, manager.lock_instance(id))
        .await
        .expect("loading past logs held the instance gate");
    drop(gate);
    pause.release.notify_one();
    let logs = logs.await;
    assert_eq!(logs.len(), 1);
    assert!(!logs[0].active);
    Ok(())
}

#[tokio::test]
async fn past_logs_loaded_while_an_instance_is_deleted_are_not_kept() -> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    without_recycle_bin(&app).await?;
    let (id, root) = fixture(&app, "deleted-logs").await?;
    tokio::fs::create_dir_all(root.join("logs")).await?;
    let manager = app.instance_manager();
    let log = root.join("logs/2025-01-14_12-00-00.log");
    tokio::fs::write(&log, b"retained history\n").await?;
    // Kept readable for the load after the folder is gone.
    let kept = tempfile::tempdir()?;
    tokio::fs::copy(&log, kept.path().join("2025-01-14_12-00-00.log")).await?;
    let history = manager.log_history_state(id);
    let pause = manager.test_hooks.arm("log_history", id);
    let logs = manager.get_logs(id);
    tokio::pin!(logs);
    reach(&pause, logs.as_mut()).await;
    tokio::time::timeout(PROMPTLY, manager._delete_instance(id))
        .await
        .expect("deleting waited for past logs to load")?;
    pause.release.notify_one();
    logs.await;
    // A load that started before the delete finishes after it.
    manager
        .load_log_history_into(id, kept.path(), &history)
        .await?;
    assert!(
        manager
            .game_logs
            .read()
            .await
            .values()
            .all(|(owner, _, _)| *owner != id),
        "a deleted instance's past logs were added back"
    );
    assert!(!manager.log_history.contains_key(&id));
    Ok(())
}
