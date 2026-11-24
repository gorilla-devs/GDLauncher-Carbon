//! The actions of Settings › Java: choosing the Java for a Java version,
//! unpinning, adding a Java from a file, removing unused Javas, and moving a
//! Java's users to a fixed Azul Zulu build.

use super::{
    build_version::JavaBuildVersion,
    instance_java::running_javas,
    invalidate_java_lists,
    java_checker::{JavaChecker, RealJavaChecker},
    java_display::java_binary_from_selection,
    major_defaults::{get_major_defaults, set_major_default},
    managed::mojang::managed_vendor_for_path,
    minecraft_majors::{MinecraftVersionProfiles, minecraft_version_profiles},
    overview::{JavaEntry, get_java_overview, java_entry_for, javas_with_users},
    resolve::{
        AutomaticInputs, AutomaticPick, pick_automatic, relink_system_profiles, requirement_for,
    },
    scan_and_sync::upsert_java_component_to_db,
    updates::{get_pinned_managed_javas, set_pinned_managed_javas},
};
use crate::{
    App,
    domain::java::{JavaComponentType, JavaOs, JavaVendor, SystemJavaProfileName},
};
use anyhow::{anyhow, bail};
use carbon_repos::repos::java as java_repo;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use strum::IntoEnumIterator;

/// Makes `java_id` the Java Automatic uses for `major`, or with `None` lets
/// the major choose automatically again, then relinks that major's system
/// profiles.
pub async fn set_java_major_default(
    app: &App,
    major: u8,
    java_id: Option<String>,
) -> anyhow::Result<()> {
    if let Some(java_id) = &java_id {
        let java = java_repo::get_java_by_id(&app.db, java_id)
            .await?
            .ok_or_else(|| anyhow!("Java with id {java_id} not found"))?;
        if java.major != i32::from(major) {
            bail!("Java with id {java_id} is not a Java {major}");
        }
        // Automatic would skip it anyway, so the choice would silently not
        // apply.
        if !java.is_valid {
            bail!("Java with id {java_id} doesn't start");
        }
    }

    let previous = get_major_defaults(&app.db).await?.remove(&major);
    set_major_default(&app.db, major, java_id).await?;
    relink_major(app, major, previous.as_deref()).await?;
    invalidate_java_lists(app);

    Ok(())
}

/// Links each of `major`'s system profiles to its automatic choice. Where
/// that is a download, a profile still linked to `previous`, the choice just
/// replaced, is unlinked, so nothing keeps pointing at a Java the user moved
/// away from.
async fn relink_major(app: &App, major: u8, previous: Option<&str>) -> anyhow::Result<()> {
    let inputs = AutomaticInputs::load(app).await?;
    let profiles = java_repo::get_all_profiles(&app.db).await?;

    let mut links = Vec::new();
    for profile in SystemJavaProfileName::iter().filter(|profile| profile.major() == major) {
        let name = profile.to_string();
        let linked = profiles
            .iter()
            .find(|row| row.name == name)
            .and_then(|row| row.java_id.clone());
        let pick = pick_automatic(
            &requirement_for(profile),
            &inputs.javas,
            &inputs.defaults,
            inputs.host_arch,
            &inputs.flagged,
        );

        let target = match pick {
            AutomaticPick::Java { id, .. } => Some(id),
            AutomaticPick::Download { .. }
                if previous.is_some() && linked.as_deref() == previous =>
            {
                None
            }
            AutomaticPick::Download { .. } => continue,
        };
        if linked != target {
            links.push((name, target));
        }
    }

    if links.is_empty() {
        return Ok(());
    }

    // One transaction, so the profiles of a major move together.
    app.db
        .write(move |mut conn| {
            let tx = conn.transaction()?;
            for (name, java_id) in links {
                java_repo::set_profile_java_conn(&tx, &name, java_id.as_deref())?;
            }
            tx.commit()?;
            Ok(())
        })
        .await?;

    Ok(())
}

/// Lets auto-update move the managed Java `java_id` along again.
pub async fn unpin_managed_java(app: &App, java_id: &str) -> anyhow::Result<()> {
    let mut pinned = get_pinned_managed_javas(&app.db).await?;

    if pinned.remove(java_id) {
        set_pinned_managed_javas(&app.db, &pinned).await?;
    }
    invalidate_java_lists(app);

    Ok(())
}

/// Adds the Java the user picked in a file dialog (see
/// [`java_binary_from_selection`]) as a user-added Java. One already listed
/// is returned as it is.
pub async fn add_java_from_file(app: &App, selected: &str) -> anyhow::Result<JavaEntry> {
    let binary = java_binary_from_selection(Path::new(selected), JavaOs::get_current_os()?);
    let not_java = || anyhow!("This file isn't a Java we can run");

    if !binary.is_file() {
        return Err(not_java());
    }
    let component = RealJavaChecker::new(app.runtime_lifecycle.clone())
        .get_bin_info(&binary, JavaComponentType::Custom)
        .await
        .map_err(|e| {
            tracing::info!("{} is not a Java we can run: {e:#}", binary.display());
            not_java()
        })?;

    let id = upsert_java_component_to_db(&app.db, component).await?;
    invalidate_java_lists(app);

    java_entry_for(app, &id).await
}

/// What [`remove_unused_javas`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedUnusedJavas {
    /// Deleted, or waiting to be.
    pub count: u32,
    /// A running game or server may run on some of them, so those are
    /// deleted once none is.
    pub deferred: bool,
}

/// Deletes the Java `java_id` from Settings › Java: a GDLauncher-installed
/// one with its files, a user-added one from the list only. While a game or
/// server that may run on it is starting or running (see [`running_javas`]),
/// it waits until none is (see `process_pending_removals`), so no running
/// Java loses its files.
pub async fn delete_java(app: &App, java_id: &str) -> anyhow::Result<()> {
    let manager = app.java_manager();
    // Read before taking removal_lock, since it may wait on the network. Only
    // telling what a running game runs on needs it.
    let versions = match manager.java_in_use_by_running().await {
        true => minecraft_version_profiles(app).await,
        false => None,
    };
    // So this never races a deferred-delete pass or Remove unused.
    let _guard = manager.removal_lock.lock().await;

    let java = java_repo::get_java_by_id(&app.db, java_id)
        .await?
        .ok_or_else(|| anyhow!("Java with id {java_id} not found"))?;
    if JavaComponentType::try_from(&*java.r#type)? == JavaComponentType::Local {
        bail!("Java with id {java_id} is local. Cannot delete.");
    }

    if !running_javas(app, versions.as_deref())
        .await?
        .uses(&java.id, &java.path)
    {
        return manager.delete_java_version(java_id.to_string()).await;
    }

    manager
        .requested_removals
        .lock()
        .await
        .insert(java_id.to_string());
    invalidate_java_lists(app);

    Ok(())
}

/// Deletes every Java in the overview's `unused_java_ids`. The ones a game or
/// server that is starting or running may run on are deleted once none is.
pub async fn remove_unused_javas(app: &App) -> anyhow::Result<RemovedUnusedJavas> {
    let manager = app.java_manager();
    // Read before taking removal_lock: the version data may wait on the
    // network, and a row's Delete or the deferred-delete pass must not wait
    // behind that.
    let unused = get_java_overview(app).await?.unused_java_ids;
    let versions = minecraft_version_profiles(app).await;
    let _guard = manager.removal_lock.lock().await;
    let unused = still_unused(app, unused, versions.as_deref()).await?;
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);

    let running = running_javas(app, versions.as_deref()).await?;
    let paths: HashMap<String, String> = java_repo::get_all_java(&app.db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.path))
        .collect();
    let (waiting, unused): (Vec<String>, Vec<String>) = unused
        .into_iter()
        .partition(|id| paths.get(id).is_some_and(|path| running.uses(id, path)));
    let deferred = count(waiting.len());
    if !waiting.is_empty() {
        manager.pending_removals.lock().await.extend(waiting);
    }

    let mut removed = 0;
    let mut failure = None;
    for id in unused {
        match manager.delete_java_version(id.clone()).await {
            Ok(()) => removed += 1,
            Err(e) => {
                tracing::warn!("could not delete unused java {id}: {e:#}");
                failure.get_or_insert(e);
            }
        }
    }
    invalidate_java_lists(app);

    match failure {
        Some(e) => Err(e.context("Could not remove every unused Java")),
        None => Ok(RemovedUnusedJavas {
            count: count(removed).saturating_add(deferred),
            deferred: deferred > 0,
        }),
    }
}

/// Of `unused`, read before the caller took `removal_lock`, the Javas still
/// there that nothing has started using since. With what some instance uses
/// unknown, none.
async fn still_unused(
    app: &App,
    unused: Vec<String>,
    versions: Option<&MinecraftVersionProfiles>,
) -> anyhow::Result<Vec<String>> {
    let Some(users) = javas_with_users(app, versions).await? else {
        return Ok(Vec::new());
    };
    let present: HashSet<String> = java_repo::get_all_java(&app.db)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect();

    Ok(unused
        .into_iter()
        .filter(|id| present.contains(id) && !users.contains(id))
        .collect())
}

/// Moves what uses `java_id` (profiles, instance path overrides and per-major
/// defaults) onto the Azul Zulu build `build_id`, which is installed unless a
/// GDLauncher-installed Azul Java of that version exists. The old Java stays,
/// and so does what can't use the new build (see `move_java_users`), such as
/// a Forge 1.16.5 instance that names it. Returns the id of the Java it moved
/// to.
pub async fn switch_java_to_fixed(
    app: &App,
    java_id: &str,
    build_id: &str,
) -> anyhow::Result<String> {
    let manager = app.java_manager();
    let old = java_repo::get_java_by_id(&app.db, java_id)
        .await?
        .ok_or_else(|| anyhow!("Java with id {java_id} not found"))?;
    let catalog = manager
        .managed_service
        .get_catalog(JavaVendor::Azul)
        .await?;
    let build = catalog
        .find(build_id)
        .ok_or_else(|| anyhow!("No Azul Zulu build {build_id}"))?;
    if i32::from(build.major) != old.major {
        bail!(
            "Azul Zulu {} can't replace a Java {}",
            build.version,
            old.major
        );
    }

    let managed_root = app
        .settings_manager()
        .runtime_path
        .get_managed_javas()
        .to_path();
    let installed = java_repo::get_all_java(&app.db)
        .await?
        .into_iter()
        .find(|row| {
            row.r#type == "managed"
                && row.is_valid
                && managed_vendor_for_path(&managed_root, &row.path) == Some(JavaVendor::Azul)
                && JavaBuildVersion::parse(&row.full_version).as_ref() == Some(&build.version)
        });

    let new = match installed {
        Some(row) => row,
        None => {
            let new_id = manager
                .managed_service
                .setup_managed(JavaVendor::Azul, build.id.clone(), app.clone(), None)
                .await?;

            java_repo::get_java_by_id(&app.db, &new_id)
                .await?
                .ok_or_else(|| anyhow!("The installed Java is missing"))?
        }
    };
    if new.id == old.id {
        bail!("Java {} is already a fixed build", old.full_version);
    }

    manager.move_java_users(&old, &new).await?;
    // The moved default, or the new build itself, may now be the automatic
    // choice.
    relink_system_profiles(app).await?;
    invalidate_java_lists(app);

    Ok(new.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::java::{
        managed::BuildSource, overview::JavaGroup, test_support::*, updates::tests::override_of,
    };
    use std::collections::{HashMap, HashSet};

    #[tokio::test]
    async fn set_major_default_relinks_and_clears() {
        let app = test_app(vec![]).await;
        add_managed(&app.app, "m21", "zulu21.0.13.1-a", "21.0.13.1", 1).await;
        add_other(&app.app, "c21", "custom", "/opt/jdk21/bin/java", "21.0.5").await;
        add_other(&app.app, "c17", "custom", "/opt/jdk17/bin/java", "17.0.12").await;

        set_java_major_default(&app.app, 21, Some("c21".into()))
            .await
            .unwrap();
        assert_eq!(
            get_major_defaults(&app.db).await.unwrap(),
            HashMap::from([(21, "c21".to_string())])
        );
        assert_eq!(
            linked(&app.app, SystemJavaProfileName::Delta)
                .await
                .as_deref(),
            Some("c21")
        );
        let overview = get_java_overview(&app.app).await.unwrap();
        let java21 = group(&overview, 21);
        assert_eq!(
            (java21.in_use_java_id.as_deref(), java21.user_chosen),
            (Some("c21"), true)
        );
        // Its row stays where its version puts it, under the newer build.
        assert_eq!(
            java21
                .javas
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["m21", "c21"]
        );

        // Back to automatic: GDLauncher's own Java again.
        set_java_major_default(&app.app, 21, None).await.unwrap();
        assert!(get_major_defaults(&app.db).await.unwrap().is_empty());
        assert_eq!(
            linked(&app.app, SystemJavaProfileName::Delta)
                .await
                .as_deref(),
            Some("m21")
        );
        let overview = get_java_overview(&app.app).await.unwrap();
        let java21 = group(&overview, 21);
        assert_eq!(
            (java21.in_use_java_id.as_deref(), java21.user_chosen),
            (Some("m21"), false)
        );

        // A default whose Java no longer runs counts as unset.
        set_java_major_default(&app.app, 21, Some("c21".into()))
            .await
            .unwrap();
        java_repo::set_java_validity(&app.db, "c21", false)
            .await
            .unwrap();
        let overview = get_java_overview(&app.app).await.unwrap();
        let java21 = group(&overview, 21);
        assert_eq!(
            (java21.in_use_java_id.as_deref(), java21.user_chosen),
            (Some("m21"), false)
        );

        // Clearing a major with nothing to fall back on unlinks its profiles,
        // so nothing keeps pointing at the Java the user moved away from.
        set_java_major_default(&app.app, 17, Some("c17".into()))
            .await
            .unwrap();
        for profile in [
            SystemJavaProfileName::Beta,
            SystemJavaProfileName::Gamma,
            SystemJavaProfileName::GammaSnapshot,
        ] {
            assert_eq!(linked(&app.app, profile).await.as_deref(), Some("c17"));
        }
        set_java_major_default(&app.app, 17, None).await.unwrap();
        assert_eq!(linked(&app.app, SystemJavaProfileName::Gamma).await, None);
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(group(&overview, 17).downloads_when_needed);

        // A Java of another major can't be a major's default.
        assert!(
            set_java_major_default(&app.app, 17, Some("m21".into()))
                .await
                .is_err()
        );
        assert!(
            set_java_major_default(&app.app, 17, Some("gone".into()))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn set_major_default_refuses_a_java_that_does_not_start() {
        let app = test_app(vec![]).await;
        add_other(&app.app, "c21", "custom", "/opt/jdk21/bin/java", "21.0.5").await;
        java_repo::set_java_validity(&app.db, "c21", false)
            .await
            .unwrap();

        let error = set_java_major_default(&app.app, 21, Some("c21".into()))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("doesn't start"), "{error:#}");
        assert!(get_major_defaults(&app.db).await.unwrap().is_empty());

        // Clearing the major's default needs no Java, so it still works.
        set_java_major_default(&app.app, 21, None).await.unwrap();
    }

    /// A game on another Java doesn't hold up Remove unused; a server on
    /// one of them does, until it stops.
    #[tokio::test]
    async fn remove_unused_javas_waits_only_for_what_runs_on_them() {
        let Fixture { app, auto, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;

        set_running_server(&app.app, Some(&old_path)).await;
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                crate::managers::instance::run::LaunchState::Preparing(
                    crate::domain::vtask::VisualTaskId(1),
                ),
            )
            .await;
        assert_eq!(
            remove_unused_javas(&app.app).await.unwrap(),
            RemovedUnusedJavas {
                count: 1,
                deferred: true
            }
        );
        assert!(Path::new(&old_path).exists());
        assert!(
            get_java_overview(&app.app)
                .await
                .unwrap()
                .unused_java_ids
                .is_empty()
        );
        // Nothing is left to remove, so nothing waits.
        assert_eq!(
            remove_unused_javas(&app.app).await.unwrap(),
            RemovedUnusedJavas {
                count: 0,
                deferred: false
            }
        );

        stop_servers(&app.app).await;
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                crate::managers::instance::run::LaunchState::Inactive { failed_task: None },
            )
            .await;
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!Path::new(&old_path).exists());
        for id in ["m21-new", "l21", "l22"] {
            assert!(
                java_repo::get_java_by_id(&app.db, id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }

    /// The game runs on m21-new, so m21-old goes at once.
    #[tokio::test]
    async fn remove_unused_javas_does_not_wait_for_a_game_on_another_java() {
        let Fixture { app, auto, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                crate::managers::instance::run::LaunchState::Preparing(
                    crate::domain::vtask::VisualTaskId(1),
                ),
            )
            .await;

        assert_eq!(
            remove_unused_javas(&app.app).await.unwrap(),
            RemovedUnusedJavas {
                count: 1,
                deferred: false
            }
        );
        assert!(!Path::new(&old_path).exists());
    }

    #[tokio::test]
    async fn remove_unused_javas_deletes_at_once_when_nothing_runs() {
        let Fixture { app, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;

        assert_eq!(
            remove_unused_javas(&app.app).await.unwrap(),
            RemovedUnusedJavas {
                count: 1,
                deferred: false
            }
        );

        assert!(!Path::new(&old_path).exists());
        assert!(
            java_repo::get_java_by_id(&app.db, "m21-old")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn delete_java_waits_only_for_a_game_that_uses_it() {
        let Fixture { app, auto, .. } = fixture().await;
        let used_path = entry_path(&app.app, "m21-new").await;
        let old_path = entry_path(&app.app, "m21-old").await;
        let set_state = |state| {
            let app = app.app.clone();
            async move {
                app.instance_manager()
                    .set_launch_state_for_test(auto, state)
                    .await
            }
        };

        // A game on Automatic, so on m21-new, is starting: its Java waits,
        // the other goes at once.
        set_state(crate::managers::instance::run::LaunchState::Preparing(
            crate::domain::vtask::VisualTaskId(1),
        ))
        .await;
        delete_java(&app.app, "m21-new").await.unwrap();
        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(Path::new(&used_path).exists());
        assert!(!Path::new(&old_path).exists());
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(entry(&overview, "m21-new").pending_removal);
        // Remove unused leaves what already waits alone.
        assert_eq!(
            remove_unused_javas(&app.app).await.unwrap(),
            RemovedUnusedJavas {
                count: 0,
                deferred: false
            }
        );
        // A Java found on the computer can't be deleted, game or not.
        assert!(delete_java(&app.app, "l21").await.is_err());
        assert!(!entry(&get_java_overview(&app.app).await.unwrap(), "l21").pending_removal);

        // Once the game is over it goes, although "Auto" still uses it: the
        // user asked, and it moves to Automatic.
        set_state(crate::managers::instance::run::LaunchState::Inactive { failed_task: None })
            .await;
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!Path::new(&used_path).exists());
        for id in ["m21-new", "m21-old"] {
            assert!(
                java_repo::get_java_by_id(&app.db, id)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            app.java_manager()
                .requested_removals
                .lock()
                .await
                .is_empty()
        );
    }

    /// The game started on m21-old, and its instance moved to another Java
    /// while it runs.
    #[tokio::test]
    async fn delete_java_waits_for_the_java_a_running_game_started_on() {
        let Fixture { app, auto, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;
        app.java_manager().note_launch_java(auto, &old_path);
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                crate::managers::instance::run::LaunchState::Preparing(
                    crate::domain::vtask::VisualTaskId(1),
                ),
            )
            .await;

        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(Path::new(&old_path).exists());

        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                crate::managers::instance::run::LaunchState::Inactive { failed_task: None },
            )
            .await;
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!Path::new(&old_path).exists());
    }

    /// A game the last session left running, adopted at startup. Its
    /// instance may have moved to another Java since it started (an update
    /// moved it to m21-new), so without the Java its process runs on every
    /// Java waits.
    #[tokio::test]
    async fn delete_java_waits_for_an_adopted_game_whose_java_is_unknown() {
        use crate::managers::instance::run::{LaunchState, RunningInstance};

        let Fixture { app, auto, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;
        let new_path = entry_path(&app.app, "m21-new").await;
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                LaunchState::Running(RunningInstance::adopted(
                    4242,
                    1,
                    chrono::Utc::now(),
                    chrono::Utc::now(),
                )),
            )
            .await;

        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(
            Path::new(&old_path).exists(),
            "deleted under an adopted game"
        );

        // Adoption read the Java its process runs: m21-new. The other goes.
        app.java_manager().requested_removals.lock().await.clear();
        app.java_manager().note_launch_java(auto, &new_path);
        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(!Path::new(&old_path).exists());
    }

    #[tokio::test]
    async fn delete_java_waits_for_a_server_that_may_run_on_it() {
        let Fixture { app, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;
        let new_path = entry_path(&app.app, "m21-new").await;

        // A server on another Java: m21-old goes at once.
        set_running_server(&app.app, Some(&new_path)).await;
        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(!Path::new(&old_path).exists());

        // On m21-new, it waits until the server stops.
        delete_java(&app.app, "m21-new").await.unwrap();
        assert!(Path::new(&new_path).exists());
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );
        stop_servers(&app.app).await;
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!Path::new(&new_path).exists());
    }

    #[tokio::test]
    async fn delete_java_waits_for_a_server_whose_java_is_unknown() {
        let Fixture { app, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;

        set_running_server(&app.app, None).await;
        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(Path::new(&old_path).exists());
    }

    /// Until the startup scan has adopted the games the last session left
    /// running, nothing knows what they run on.
    #[tokio::test]
    async fn delete_java_waits_for_the_instance_scan() {
        let Fixture { app, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;
        app.instance_manager().instances_scanned.send_replace(false);

        delete_java(&app.app, "m21-old").await.unwrap();
        assert!(Path::new(&old_path).exists());
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );

        app.instance_manager().instances_scanned.send_replace(true);
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!Path::new(&old_path).exists());
    }

    /// A server running on `java_path`, or on a Java not known yet.
    async fn set_running_server(app: &crate::App, java_path: Option<&str>) {
        use crate::{
            domain::server::{ServerId, ServerLogId, ServerState},
            managers::server::ServerData,
        };

        app.server_manager().servers.write().await.insert(
            ServerId(1),
            ServerData {
                shortpath: "srv".into(),
                state: ServerState::Running {
                    start_time: chrono::Utc::now(),
                    log_id: ServerLogId(1),
                    process_id: 4242,
                },
                handle: None,
                last_log_id: None,
                java_path: java_path.map(std::path::PathBuf::from),
            },
        );
    }

    async fn stop_servers(app: &crate::App) {
        for server in app.server_manager().servers.write().await.values_mut() {
            server.state = crate::domain::server::ServerState::Stopped { failed_task: None };
        }
    }

    #[tokio::test]
    async fn delete_java_deletes_at_once_when_nothing_runs() {
        let Fixture { app, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;

        delete_java(&app.app, "m21-old").await.unwrap();

        assert!(!Path::new(&old_path).exists());
        assert!(
            java_repo::get_java_by_id(&app.db, "m21-old")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            app.java_manager()
                .requested_removals
                .lock()
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn unpin_managed_java_removes_only_that_pin() {
        let app = test_app(vec![]).await;
        set_pinned_managed_javas(&app.db, &HashSet::from(["a".to_string(), "b".to_string()]))
            .await
            .unwrap();

        unpin_managed_java(&app.app, "a").await.unwrap();

        assert_eq!(
            get_pinned_managed_javas(&app.db).await.unwrap(),
            HashSet::from(["b".to_string()])
        );
    }

    #[cfg(unix)]
    mod unix {
        use super::*;

        /// A `java` the real checker accepts: it prints what JavaCheck would.
        fn fake_java_script(version: &str) -> String {
            format!(
                "#!/bin/sh\necho java.version={version}\necho os.arch={}\necho java.vendor=Azul Systems, Inc.\n",
                std::env::consts::ARCH
            )
        }

        fn fake_java(bin: &Path, version: &str) {
            use std::os::unix::fs::PermissionsExt;

            std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
            std::fs::write(bin, fake_java_script(version)).unwrap();
            std::fs::set_permissions(bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        #[tokio::test]
        async fn add_java_from_file_normalises_and_rejects_non_java() {
            let app = test_app(vec![]).await;
            let home = app.tmpdir.join("jdk-21");
            fake_java(&home.join("bin/java"), "21.0.5");

            let added = add_java_from_file(&app.app, &home.to_string_lossy())
                .await
                .unwrap();

            assert_eq!(added.path, home.join("bin/java").to_string_lossy());
            assert_eq!(added.source, JavaComponentType::Custom);
            assert_eq!(added.major, 21);
            assert_eq!(added.vendor_name, "Azul Zulu");
            // Adding it again gives the same Java, also from its bin folder.
            let again = add_java_from_file(&app.app, &added.path).await.unwrap();
            assert_eq!(again.id, added.id);
            let from_bin = add_java_from_file(&app.app, &home.join("bin").to_string_lossy())
                .await
                .unwrap();
            assert_eq!(from_bin.id, added.id);

            let not_java = app.tmpdir.join("notes.txt");
            std::fs::write(&not_java, "hello").unwrap();
            for path in [not_java, app.tmpdir.join("missing")] {
                let err = add_java_from_file(&app.app, &path.to_string_lossy())
                    .await
                    .unwrap_err();
                assert_eq!(err.to_string(), "This file isn't a Java we can run");
            }
        }

        /// An Azul zip with one root folder holding a fake `bin/java`.
        fn azul_zip(root: &str, version: &str) -> Vec<u8> {
            use std::io::Write;
            use zip::write::SimpleFileOptions;

            let mut bytes = Vec::new();
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            zip.add_directory(format!("{root}/"), SimpleFileOptions::default())
                .unwrap();
            zip.add_directory(format!("{root}/bin/"), SimpleFileOptions::default())
                .unwrap();
            zip.start_file(
                format!("{root}/bin/java"),
                SimpleFileOptions::default().unix_permissions(0o755),
            )
            .unwrap();
            zip.write_all(fake_java_script(version).as_bytes()).unwrap();
            zip.finish().unwrap();
            bytes
        }

        #[tokio::test]
        async fn switch_java_to_fixed_moves_users_but_keeps_old() {
            use sha2::Digest;

            let zip = azul_zip("zulu21.0.13.1-fixed", "21.0.13.1");
            let sha256 = hex::encode(sha2::Sha256::digest(&zip));
            let mut server = mockito::Server::new_async().await;
            // One probe for the size, one download: installed once.
            let download = server
                .mock("GET", "/zulu21-fixed.zip")
                .with_body(zip)
                .expect(2)
                .create_async()
                .await;
            let mut fixed = build("b21-fixed", &[21, 0, 13, 1]);
            fixed.source = BuildSource::AzulZip {
                download_url: format!("{}/zulu21-fixed.zip", server.url()),
                sha256: Some(sha256),
            };
            let app = test_app(vec![fixed]).await;

            let old = add_other(
                &app.app,
                "l21",
                "local",
                "/usr/lib/jvm/java-21/bin/java",
                "21.0.8",
            )
            .await;
            let other = add_other(&app.app, "c21", "custom", "/opt/jdk21/bin/java", "21.0.5").await;
            let pathed = add_instance(
                &app.app,
                "Pathed",
                "1.21.4",
                false,
                Some(JavaOverride::Path(Some(old.path.clone()))),
            )
            .await;
            app.java_manager()
                .create_java_profile("Mine".into(), Some(old.id.clone()))
                .await
                .unwrap();
            set_java_major_default(&app.app, 21, Some(old.id.clone()))
                .await
                .unwrap();

            let new_id = switch_java_to_fixed(&app.app, &old.id, "b21-fixed")
                .await
                .unwrap();

            let new = java_repo::get_java_by_id(&app.db, &new_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(new.r#type, "managed");
            assert_eq!(new.full_version, "21.0.13.1");
            assert!(matches!(
                crate::managers::java::updates::tests::override_of(&app.app, pathed).await,
                Some(JavaOverride::Path(Some(p))) if p == new.path
            ));
            assert_eq!(
                java_repo::get_profile(&app.db, "Mine")
                    .await
                    .unwrap()
                    .unwrap()
                    .java_id,
                Some(new_id.clone())
            );
            assert_eq!(
                get_major_defaults(&app.db).await.unwrap(),
                HashMap::from([(21, new_id.clone())])
            );
            assert_eq!(
                linked(&app.app, SystemJavaProfileName::Delta).await,
                Some(new_id.clone())
            );
            // The old Java stays.
            assert!(
                java_repo::get_java_by_id(&app.db, &old.id)
                    .await
                    .unwrap()
                    .is_some()
            );

            // Another affected Java reuses the installed build.
            let again = switch_java_to_fixed(&app.app, &other.id, "b21-fixed")
                .await
                .unwrap();
            assert_eq!(again, new_id);
            download.assert_async().await;

            // A build of another major is refused.
            add_other(&app.app, "c17", "custom", "/opt/jdk17/bin/java", "17.0.12").await;
            assert!(
                switch_java_to_fixed(&app.app, "c17", "b21-fixed")
                    .await
                    .is_err()
            );
        }

        #[tokio::test]
        async fn add_java_from_file_works_without_version_data() {
            let app = test_app(vec![]).await;
            manifest_unreachable(&app.app);
            let home = app.tmpdir.join("jdk-21");
            fake_java(&home.join("bin/java"), "21.0.5");
            let bin = home.join("bin/java").to_string_lossy().into_owned();
            let pathed = add_instance(
                &app.app,
                "Pathed",
                "1.21.4",
                false,
                Some(JavaOverride::Path(Some(bin.clone()))),
            )
            .await;

            let added = add_java_from_file(&app.app, &bin).await.unwrap();

            assert_eq!(added.path, bin);
            assert_eq!(added.major, 21);
            assert_eq!(added.used_by, vec![usage("Pathed", pathed, true)]);
            assert!(
                java_repo::get_java_by_id(&app.db, &added.id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[tokio::test]
    async fn a_deferred_removal_that_gains_a_user_is_kept() {
        let Fixture { app, .. } = fixture().await;
        let old_path = entry_path(&app.app, "m21-old").await;

        // A server runs on it.
        set_running_server(&app.app, Some(&old_path)).await;
        assert_eq!(
            remove_unused_javas(&app.app).await.unwrap(),
            RemovedUnusedJavas {
                count: 1,
                deferred: true
            }
        );
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(entry(&overview, "m21-old").pending_removal);
        assert!(!entry(&overview, "m21-new").pending_removal);

        // While the server runs, the user picks it for Java 21.
        set_java_major_default(&app.app, 21, Some("m21-old".into()))
            .await
            .unwrap();
        stop_servers(&app.app).await;

        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );
        assert!(Path::new(&old_path).exists());
        assert!(
            java_repo::get_java_by_id(&app.db, "m21-old")
                .await
                .unwrap()
                .is_some()
        );
        assert!(app.java_manager().pending_removals.lock().await.is_empty());
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(!entry(&overview, "m21-old").pending_removal);
    }

    #[tokio::test]
    async fn switch_keeps_forge_1_16_5_on_the_build_it_needs() {
        let app = test_app(vec![build("b8", &[8, 0, 492])]).await;
        // The fixed build is installed already, so nothing downloads.
        let fixed = add_managed(&app.app, "m8-fixed", "zulu8.0.492-a", "8.0.492", 1).await;
        let old = add_other(&app.app, "c8", "custom", "/opt/jdk8/bin/java", "8.0.302").await;
        let path = |p: &str| Some(JavaOverride::Path(Some(p.to_string())));
        let forge = add_instance(&app.app, "Forge", "1.16.5", true, path(&old.path)).await;
        let vanilla = add_instance(&app.app, "Vanilla", "1.16.5", false, path(&old.path)).await;
        app.java_manager()
            .create_java_profile("ForgeJava".into(), Some(old.id.clone()))
            .await
            .unwrap();
        add_instance(
            &app.app,
            "ForgeProfiled",
            "1.16.5",
            true,
            Some(JavaOverride::Profile(Some("ForgeJava".into()))),
        )
        .await;

        let new_id = switch_java_to_fixed(&app.app, &old.id, "b8").await.unwrap();

        assert_eq!(new_id, "m8-fixed");
        assert!(matches!(
            override_of(&app.app, forge).await,
            Some(JavaOverride::Path(Some(p))) if p == old.path
        ));
        assert!(matches!(
            override_of(&app.app, vanilla).await,
            Some(JavaOverride::Path(Some(p))) if p == fixed.path
        ));
        assert_eq!(
            java_repo::get_profile(&app.db, "ForgeJava")
                .await
                .unwrap()
                .unwrap()
                .java_id
                .as_deref(),
            Some("c8")
        );
        assert!(
            java_repo::get_java_by_id(&app.db, &old.id)
                .await
                .unwrap()
                .is_some()
        );
    }
}
