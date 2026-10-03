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
    assert!(app.runtime_lifecycle.begin_relocation("move").is_err());
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

#[tokio::test]
async fn task04_runtime_reconciliation_survives_legacy_pid_cleanup_and_restart()
-> anyhow::Result<()> {
    let app = crate::setup_managers_for_test().await;
    let (_, root) = fixture(&app, "legacy-owner").await?;
    // The test runner is an existing benign non-Java process; never signal it.
    tokio::fs::write(root.join(PID_FILE_NAME), std::process::id().to_string()).await?;
    app.instance_manager().scan_instances().await?;
    assert!(app.runtime_lifecycle.begin_relocation("one").is_err());
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
    assert!(
        restarted.begin_relocation("two").is_err(),
        "best-effort pid cleanup must not erase a present relocation owner across restart"
    );
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
