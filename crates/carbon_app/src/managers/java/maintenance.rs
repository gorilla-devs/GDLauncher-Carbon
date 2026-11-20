//! Background upkeep of managed Javas: updates, deferred deletes and the
//! advisory refresh.

use super::{
    managed::{ManagedJavaCatalog, ManagedService},
    refresh_catalogs,
    updates::{get_pinned_managed_javas, updates_to_apply},
};
use crate::{App, domain::java::JavaVendor};
use std::collections::HashMap;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;

pub const JAVA_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

/// How often a waiting update re-checks for running games and servers.
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// One maintenance pass. Two independent halves run together:
/// - the updates: fetch every distribution's catalog (one that fails is
///   skipped), wait until no
///   game or server runs, then update every Java auto-update covers;
/// - the housekeeping: delete the Javas whose removal was deferred, then
///   refresh the advisories. This never waits behind a running game.
pub async fn run_java_maintenance_pass(app: &App) -> anyhow::Result<()> {
    let java_manager = app.java_manager();

    run_pass(app, &java_manager.managed_service, refresh_advisories(app)).await
}

async fn run_pass(
    app: &App,
    service: &ManagedService,
    advisories: impl Future<Output = ()>,
) -> anyhow::Result<()> {
    tokio::join!(run_updates(app, service), run_housekeeping(app, advisories));

    Ok(())
}

async fn run_updates(app: &App, service: &ManagedService) {
    let catalogs = refresh_catalogs(app, service).await;

    if catalogs.is_empty() {
        tracing::warn!("could not fetch any java catalog, skipping updates");
        return;
    }

    if let Err(e) = update_managed_javas(app, &catalogs).await {
        tracing::warn!("could not update managed javas: {e:#}");
    }
}

/// The deferred deletes wait for the instance scan (see
/// [`maintenance_loop`]); the advisories touch no Java, so they don't.
async fn run_housekeeping(app: &App, advisories: impl Future<Output = ()>) {
    let removals = async {
        app.instance_manager().wait_for_instance_scan().await;
        if let Err(e) = app.java_manager().process_pending_removals().await {
            tracing::warn!("could not delete replaced javas: {e:#}");
        }
    };

    tokio::join!(removals, advisories);
}

/// Ids of the Javas auto-update covers right now: nothing when it is off, and
/// never a pinned or looping one.
async fn updates_now(
    app: &App,
    catalogs: &HashMap<JavaVendor, ManagedJavaCatalog>,
) -> anyhow::Result<Vec<String>> {
    let auto_update = app
        .settings_manager()
        .get_settings()
        .await?
        .auto_update_managed_java;

    if !auto_update {
        return Ok(Vec::new());
    }

    let updates = app.java_manager().managed_updates(catalogs).await?;
    let pinned = get_pinned_managed_javas(&app.db).await?;

    Ok(updates_to_apply(&updates, auto_update, &pinned)
        .into_iter()
        .map(|update| update.java_id.clone())
        .collect())
}

/// Blocks until no game or server is starting or running.
async fn wait_until_java_idle(app: &App) {
    let mut running = app.instance_manager().any_instance_running.subscribe();

    while app.java_manager().java_in_use_by_running().await {
        tokio::select! {
            _ = running.changed() => {}
            _ = tokio::time::sleep(IDLE_POLL_INTERVAL) => {}
        }
    }
}

/// Updates what auto-update covers and returns the ids it tried. The wait can
/// last hours, so the setting and the pins are read again after it.
async fn update_managed_javas(
    app: &App,
    catalogs: &HashMap<JavaVendor, ManagedJavaCatalog>,
) -> anyhow::Result<Vec<String>> {
    if updates_now(app, catalogs).await?.is_empty() {
        return Ok(Vec::new());
    }

    // An update can delete the old Java, so it waits for games and servers.
    wait_until_java_idle(app).await;

    let ids = updates_now(app, catalogs).await?;

    for id in &ids {
        if let Err(e) = app.java_manager().update_managed_java(id).await {
            tracing::warn!("could not update managed java {id}: {e:#}");
        }
    }

    Ok(ids)
}

async fn refresh_advisories(app: &App) {
    app.java_manager().refresh_advisories(app).await;
}

/// Runs a pass at startup and then every [`JAVA_MAINTENANCE_INTERVAL`] (see
/// [`maintenance_loop`]). Also deletes deferred Javas as soon as a game or
/// server that held them stops.
pub fn spawn_java_maintenance(app: App) {
    tokio::spawn(delete_removals_on_exit(app.clone()));

    tokio::spawn(maintenance_loop(app, JAVA_MAINTENANCE_INTERVAL, |app| {
        let app = app.clone();
        async move { refresh_advisories(&app).await }
    }));
}

/// Deletes the deferred Javas each time a game or server stops: a Java the
/// user deleted goes once nothing running uses it, even while another game
/// runs (see `process_pending_removals`). Again when the last game's
/// liveness marker goes, which can be after its state turned inactive (the
/// adopted games' poller drops its marker on its next tick): the replaced
/// Javas wait for that.
async fn delete_removals_on_exit(app: App) {
    let mut running = app.instance_manager().any_instance_running.subscribe();
    let java_manager = app.java_manager();

    loop {
        tokio::select! {
            _ = java_manager.runner_ended.notified() => {}
            changed = running.changed() => {
                if changed.is_err() {
                    return;
                }
                if *running.borrow_and_update() {
                    continue;
                }
            }
        }

        if let Err(e) = java_manager.process_pending_removals().await {
            tracing::warn!("could not delete replaced javas: {e:#}");
        }
    }
}

/// A pass at once, then one every `interval`. Its updates and deferred
/// deletes wait for the instance scan: until it has adopted the games left
/// running by the last session, nothing knows they run, so they could delete
/// a Java one runs on.
async fn maintenance_loop<F, A>(app: App, interval: Duration, advisories: F)
where
    F: Fn(&App) -> A,
    A: Future<Output = ()>,
{
    let mut interval = tokio::time::interval(interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // The update half can sit waiting for a game for hours; it runs on its
    // own so the housekeeping still happens every tick.
    let updating = Arc::new(tokio::sync::Mutex::new(()));

    loop {
        interval.tick().await;

        if let Ok(guard) = updating.clone().try_lock_owned() {
            let update_app = app.clone();
            tokio::spawn(async move {
                update_app.instance_manager().wait_for_instance_scan().await;
                let java_manager = update_app.java_manager();
                run_updates(&update_app, &java_manager.managed_service).await;
                drop(guard);
            });
        }

        run_housekeeping(&app, advisories(&app)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        managers::java::{
            build_version::JavaBuildVersion,
            managed::{
                BuildSource, ManagedJavaBuild, ManagedService, azul_zulu::AzulZulu,
                mojang::MojangRuntime,
            },
        },
        setup_managers_for_test,
    };
    use carbon_repos::repos::java::{self as java_repo, JavaRow};
    use std::sync::atomic::{AtomicBool, Ordering};

    fn java_row(id: &str, path: &str, version: &str) -> JavaRow {
        JavaRow {
            id: id.into(),
            path: path.into(),
            major: 17,
            full_version: version.into(),
            r#type: "managed".into(),
            os: "linux".into(),
            arch: "amd64".into(),
            vendor: "azul".into(),
            is_valid: true,
        }
    }

    /// A managed Azul Java's path under this app's managed javas folder.
    fn managed_bin(app: &crate::App) -> String {
        app.settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path()
            .join("zulu17.0.19")
            .join("bin/java")
            .to_string_lossy()
            .into_owned()
    }

    fn catalog_with_newer_17() -> HashMap<JavaVendor, ManagedJavaCatalog> {
        HashMap::from([(JavaVendor::Azul, azul_catalog_with_newer_17())])
    }

    fn service_for(server: &mockito::ServerGuard) -> ManagedService {
        ManagedService::with_runtimes(
            AzulZulu::with_base_url(format!("{}/", server.url())),
            MojangRuntime::with_base_url(format!("{}/", server.url())),
        )
    }

    fn azul_catalog_with_newer_17() -> ManagedJavaCatalog {
        ManagedJavaCatalog::from_builds(vec![ManagedJavaBuild {
            id: "b17-new".into(),
            name: "zulu17.0.21.1".into(),
            major: 17,
            version: JavaBuildVersion::parse("17.0.21.1").unwrap(),
            source: BuildSource::AzulZip {
                download_url: String::new(),
                sha256: None,
            },
            size_bytes: None,
            release_date: None,
            lts: false,
        }])
    }

    #[tokio::test]
    async fn maintenance_pass_survives_catalog_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .expect_at_least(1)
            .create_async()
            .await;
        let service = service_for(&server);
        let app = setup_managers_for_test().await;

        run_pass(&app.app, &service, async {}).await.unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn catalog_error_and_failed_delete_still_refresh_advisories() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .create_async()
            .await;
        let service = service_for(&server);
        let app = setup_managers_for_test().await;

        // Outside the managed folder, so deleting it fails. Its replacement is
        // what Java 17 uses now.
        let stuck = java_row("stuck", "/nowhere/zulu17/bin/java", "17.0.19");
        java_repo::insert_java(&app.db, stuck).await.unwrap();
        let replacement = java_row("replacement", "/nowhere/zulu17-new/bin/java", "17.0.21.1");
        java_repo::insert_java(&app.db, replacement).await.unwrap();
        app.java_manager()
            .pending_removals
            .lock()
            .await
            .insert("stuck".into());

        let refreshed = AtomicBool::new(false);
        run_pass(&app.app, &service, async {
            refreshed.store(true, Ordering::SeqCst);
        })
        .await
        .unwrap();

        assert!(refreshed.load(Ordering::SeqCst));
        assert!(
            app.java_manager()
                .pending_removals
                .lock()
                .await
                .contains("stuck")
        );
    }

    /// A game left running when the launcher last quit is adopted by the
    /// instance scan. Until then nothing knows it runs, so maintenance must
    /// not delete any Java yet.
    #[tokio::test]
    async fn the_first_pass_waits_for_the_instance_scan() {
        use crate::managers::java::test_support::{add_managed, test_app};

        let app = test_app(vec![]).await;
        app.instance_manager().instances_scanned.send_replace(false);
        let replaced = add_managed(&app.app, "old", "zulu17.0.19-old", "17.0.19", 1).await;
        // What replaced it, which Java 17's Automatic now uses.
        add_managed(&app.app, "new", "zulu17.0.21.1-new", "17.0.21.1", 1).await;
        let folder = std::path::Path::new(&replaced.path)
            .parent()
            .and_then(std::path::Path::parent)
            .unwrap()
            .to_path_buf();
        app.java_manager()
            .pending_removals
            .lock()
            .await
            .insert("old".into());

        let maintenance = tokio::spawn(maintenance_loop(
            app.app.clone(),
            Duration::from_secs(3600),
            |_| async {},
        ));
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(folder.exists(), "deleted before the instance scan");

        app.instance_manager().instances_scanned.send_replace(true);
        tokio::time::timeout(Duration::from_secs(5), async {
            while folder.exists() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the pass should run once the instances are scanned");
        maintenance.abort();
    }

    /// Advisories touch no Java, so a big library's slow scan doesn't hold
    /// up the warnings.
    #[tokio::test]
    async fn advisories_refresh_before_the_instance_scan() {
        let app = setup_managers_for_test().await;
        app.instance_manager().instances_scanned.send_replace(false);

        let refreshed = Arc::new(AtomicBool::new(false));
        let maintenance = tokio::spawn(maintenance_loop(
            app.app.clone(),
            Duration::from_secs(3600),
            {
                let refreshed = Arc::clone(&refreshed);
                move |_| {
                    let refreshed = Arc::clone(&refreshed);
                    async move { refreshed.store(true, Ordering::SeqCst) }
                }
            },
        ));

        let result = tokio::time::timeout(Duration::from_secs(5), async {
            while !refreshed.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        maintenance.abort();
        assert!(
            result.is_ok(),
            "the advisories waited for the instance scan"
        );
    }

    /// A Java whose only game closed goes then, while another game still
    /// runs.
    #[tokio::test]
    async fn a_deferred_delete_goes_when_its_game_exits_while_another_runs() {
        use crate::managers::{
            instance::run::LaunchState,
            java::{
                overview_actions::delete_java,
                test_support::{entry_path, fixture},
            },
        };

        let fixture = fixture().await;
        let app = &fixture.app;
        let path = entry_path(&app.app, "m21-new").await;
        let preparing = || LaunchState::Preparing(crate::domain::vtask::VisualTaskId(1));
        // "Auto" runs on m21-new, "Pathed" on the found Java 21.
        for id in [fixture.auto, fixture.pathed] {
            app.instance_manager()
                .set_launch_state_for_test(id, preparing())
                .await;
        }
        let watcher = tokio::spawn(delete_removals_on_exit(app.app.clone()));

        delete_java(&app.app, "m21-new").await.unwrap();
        assert!(std::path::Path::new(&path).exists());

        app.instance_manager()
            .change_launch_state(fixture.auto, LaunchState::Inactive { failed_task: None })
            .await
            .unwrap();
        let gone = tokio::time::timeout(Duration::from_secs(5), async {
            while std::path::Path::new(&path).exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        watcher.abort();
        assert!(gone.is_ok(), "the Java waited for the other game");
    }

    /// A game's state turns inactive before its liveness marker goes (the
    /// adopted poller drops its own on the next tick), so the replaced Javas
    /// go when the last marker does.
    #[tokio::test]
    async fn replaced_javas_go_once_the_last_game_is_gone() {
        use crate::managers::java::test_support::{add_managed, test_app};

        let app = test_app(vec![]).await;
        let old = add_managed(&app.app, "old", "zulu17.0.19-old", "17.0.19", 1).await;
        add_managed(&app.app, "new", "zulu17.0.21.1-new", "17.0.21.1", 1).await;
        app.java_manager()
            .pending_removals
            .lock()
            .await
            .insert("old".into());
        app.instance_manager()
            .any_instance_running
            .send_replace(true);
        let watcher = tokio::spawn(delete_removals_on_exit(app.app.clone()));
        tokio::time::sleep(Duration::from_millis(100)).await;

        // The game's state is inactive, but its marker still counts it.
        app.java_manager().note_runner_ended();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(std::path::Path::new(&old.path).exists());

        app.instance_manager()
            .any_instance_running
            .send_replace(false);
        let gone = tokio::time::timeout(Duration::from_secs(5), async {
            while std::path::Path::new(&old.path).exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        watcher.abort();
        assert!(gone.is_ok(), "the replaced Java waited for the next pass");
    }

    #[tokio::test]
    async fn housekeeping_does_not_wait_for_a_running_game() {
        let app = setup_managers_for_test().await;
        app.instance_manager()
            .any_instance_running
            .send_replace(true);

        let refreshed = AtomicBool::new(false);
        tokio::time::timeout(
            Duration::from_secs(5),
            run_housekeeping(&app.app, async {
                refreshed.store(true, Ordering::SeqCst);
            }),
        )
        .await
        .unwrap();

        assert!(refreshed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn update_is_dropped_when_auto_update_is_turned_off_while_waiting() {
        let app = setup_managers_for_test().await;
        let old = java_row("old", &managed_bin(&app.app), "17.0.19");
        java_repo::insert_java(&app.db, old).await.unwrap();
        app.instance_manager()
            .any_instance_running
            .send_replace(true);

        let task_app = app.app.clone();
        let task = tokio::spawn(async move {
            update_managed_javas(&task_app, &catalog_with_newer_17())
                .await
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!task.is_finished(), "should still be waiting for the game");

        app.db
            .write(|conn| {
                let patch = carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    auto_update_managed_java: Some(false),
                    ..Default::default()
                };
                Ok(patch.build().map(|q| q.execute(&conn)).transpose()?)
            })
            .await
            .unwrap();
        app.instance_manager()
            .any_instance_running
            .send_replace(false);

        let attempted = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert!(attempted.is_empty(), "{attempted:?}");
    }

    #[tokio::test]
    async fn pin_added_while_waiting_stops_the_update() {
        let app = setup_managers_for_test().await;
        let old = java_row("old", &managed_bin(&app.app), "17.0.19");
        java_repo::insert_java(&app.db, old).await.unwrap();
        app.instance_manager()
            .any_instance_running
            .send_replace(true);

        let task_app = app.app.clone();
        let task = tokio::spawn(async move {
            update_managed_javas(&task_app, &catalog_with_newer_17())
                .await
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        crate::managers::java::updates::set_pinned_managed_javas(
            &app.db,
            &std::collections::HashSet::from(["old".to_string()]),
        )
        .await
        .unwrap();
        app.instance_manager()
            .any_instance_running
            .send_replace(false);

        let attempted = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert!(attempted.is_empty(), "{attempted:?}");
    }
}
