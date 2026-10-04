use crate::managers::App;
use crate::managers::runtime_lifecycle::{
    CancelRelocationError, CancelShutdownError, ShutdownPurpose, ShutdownRetired,
};
use axum::{Json, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    request_id: String,
    purpose: Option<ShutdownPurpose>,
}
/// Relocation only names its own lease; it never takes a shutdown purpose.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocationRequest {
    request_id: String,
}
/// Cancelling names only the request; its purpose does not matter.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelShutdown {
    request_id: String,
}
/// A blocker check may come before any lease exists, so the id is optional.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocationCheck {
    #[serde(default)]
    request_id: String,
}
fn conflict(error: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::CONFLICT, error.to_string())
}
type Refusal = (StatusCode, Json<serde_json::Value>);
pub(super) fn mount() -> axum::Router<App> {
    axum::Router::new()
        // A refusal says `cancelled: true` when the id was cancelled or
        // replaced earlier: nothing is frozen for it, so the launcher starts
        // over under a new id (its cancel may have landed unanswered).
        .route("/prepare-shutdown", post(|State(app): State<App>, Json(request): Json<Request>| async move {
            let purpose = request.purpose.unwrap_or(ShutdownPurpose::Quit);
            let owned = app.clone();
            let id = request.request_id.clone();
            let status = app.runtime_lifecycle.prepare_shutdown(&request.request_id, purpose, async move {
                let drain = owned.runtime_lifecycle.shutdown_drain(&id);
                owned.server_manager.shutdown_running_for(&drain).await
            }).map_err(|error| {
                let cancelled = error.is::<ShutdownRetired>();
                (StatusCode::CONFLICT, Json(serde_json::json!({ "error": error.to_string(), "cancelled": cancelled })))
            })?;
            Ok::<_, Refusal>(Json(serde_json::json!({ "requestId": request.request_id, "runtimePath": app.settings_manager().runtime_path.to_path_buf(), "status": status })))
        }))
        .route("/shutdown-status", post(|State(app): State<App>, Json(request): Json<Request>| async move {
            let status = app.runtime_lifecycle.shutdown_status(&request.request_id, request.purpose.unwrap_or(ShutdownPurpose::Quit)).map_err(conflict)?;
            Ok::<_, (StatusCode, String)>(Json(serde_json::json!({ "requestId": request.request_id, "runtimePath": app.settings_manager().runtime_path.to_path_buf(), "status": status })))
        }))
        // The user chose to keep the launcher open after a quit, restart or
        // update could not finish: admission opens again. A committed request
        // is reported so the caller observes the exit, and one that is
        // already finishing so the caller lets it finish.
        .route("/cancel-shutdown", post(|State(app): State<App>, Json(request): Json<CancelShutdown>| async move {
            app.runtime_lifecycle.cancel_shutdown(&request.request_id).map_err(|error| {
                let committed = matches!(error, CancelShutdownError::Committed);
                let finishing = matches!(error, CancelShutdownError::Finishing);
                (StatusCode::CONFLICT, Json(serde_json::json!({ "error": error.to_string(), "committed": committed, "finishing": finishing })))
            })?;
            Ok::<_, Refusal>(Json(serde_json::json!({ "requestId": request.request_id })))
        }))
        // Refuse-first: a 409 lists what is running and changes nothing, so the
        // game, servers and downloads carry on and quit is never pinned.
        .route("/prepare-relocation", post(|State(app): State<App>, Json(request): Json<RelocationRequest>| async move {
            if request.request_id.is_empty() {
                return Err::<Json<serde_json::Value>, Refusal>((StatusCode::CONFLICT, Json(serde_json::json!({ "error": "Missing request id", "blockers": [] }))));
            }
            app.runtime_lifecycle.refresh_adopted();
            app.runtime_lifecycle.begin_relocation(&request.request_id).map_err(|refused| {
                (StatusCode::CONFLICT, Json(serde_json::json!({ "error": refused.to_string(), "blockers": refused.blockers, "cancelled": refused.cancelled })))
            })?;
            Ok(Json(serde_json::json!({ "requestId": request.request_id, "runtimePath": app.settings_manager().runtime_path.to_path_buf() })))
        }))
        // Read-only: lets the launcher refuse a move before scanning the whole
        // data folder. It takes no lease; prepare-relocation checks again.
        .route("/relocation-blockers", post(|State(app): State<App>, Json(request): Json<RelocationCheck>| async move {
            app.runtime_lifecycle.refresh_adopted();
            Json(serde_json::json!({ "blockers": app.runtime_lifecycle.relocation_blockers(&request.request_id) }))
        }))
        // A committed lease answers `committed: true`: the core is already on
        // its way out, so the caller must observe the exit, not report a release.
        .route("/cancel-relocation", post(|State(app): State<App>, Json(request): Json<RelocationRequest>| async move {
            app.runtime_lifecycle.cancel_relocation(&request.request_id).map_err(|error| {
                let committed = matches!(error, CancelRelocationError::Committed);
                (StatusCode::CONFLICT, Json(serde_json::json!({ "error": error.to_string(), "committed": committed })))
            })?;
            Ok::<_, Refusal>(Json(serde_json::json!({ "requestId": request.request_id })))
        }))
        .route("/shutdown", post(|State(app): State<App>, Json(request): Json<Request>| async move {
            // Admission is already frozen and all external owners were absent.
            // No games are killed. Electron must observe actual core exit after this ACK.
            if let Some(purpose) = request.purpose {
                app.runtime_lifecycle.commit_prepared_shutdown(&request.request_id, purpose).map_err(conflict)?;
            } else {
                app.runtime_lifecycle.commit_shutdown(&request.request_id).map_err(conflict)?;
            }
            Ok::<_, (StatusCode, String)>(Json(serde_json::json!({ "requestId": request.request_id })))
        }))
        .merge(e2e::mount())
}

/// Hooks for the end-to-end tests (`--features e2e` builds only): a task that
/// ignores cancellation, as a long copy or a Recycle Bin move does, so a
/// test can make a quit wait on it.
#[cfg(feature = "e2e")]
mod e2e {
    use super::*;
    use crate::managers::runtime_lifecycle::{ActivityPolicy, RuntimeActivity};
    static HELD: std::sync::Mutex<Vec<RuntimeActivity>> = std::sync::Mutex::new(vec![]);
    pub(super) fn mount() -> axum::Router<App> {
        axum::Router::new()
            .route(
                "/e2e-hold-task",
                post(|State(app): State<App>| async move {
                    let activity = app
                        .runtime_lifecycle
                        .admit_owned("e2e held task", ActivityPolicy::CoreWork)
                        .map_err(conflict)?;
                    HELD.lock().unwrap().push(activity);
                    Ok::<_, (StatusCode, String)>(Json(serde_json::json!({})))
                }),
            )
            .route(
                "/e2e-release-tasks",
                post(|| async move {
                    for activity in HELD.lock().unwrap().drain(..) {
                        activity.finish();
                    }
                    Json(serde_json::json!({}))
                }),
            )
    }
}
#[cfg(not(feature = "e2e"))]
mod e2e {
    pub(super) fn mount() -> axum::Router<super::App> {
        axum::Router::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::managers::runtime_lifecycle::ActivityPolicy;

    async fn serve(app: crate::managers::App) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(listener, super::mount().with_state(app))
                .await
                .unwrap();
        });
        base
    }
    async fn post(base: &str, endpoint: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("{base}/{endpoint}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let text = response.text().await.unwrap();
        (
            status,
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)),
        )
    }

    /// A move refused while something runs freezes nothing: the same session
    /// can still quit with a fresh request, and a launch is still admitted.
    #[tokio::test]
    async fn prepare_relocation_refuses_with_blockers_and_leaves_quit_and_launches_open() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let base = serve(app.clone()).await;
        let mut server = app.runtime_lifecycle.admit("server 7 run").unwrap();
        server.external();
        server.spawn(ActivityPolicy::Server, || Ok(())).unwrap();

        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "move-1" }),
        )
        .await;
        assert_eq!(status, 409, "{body}");
        assert!(
            body["blockers"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!({ "kind": "server", "name": "server 7 run" })),
            "{body}"
        );
        assert!(!app.runtime_lifecycle.admission_frozen());
        drop(
            app.runtime_lifecycle
                .admit("launch after refused move")
                .unwrap(),
        );

        // Nothing was leased, so there is nothing to cancel.
        let (status, body) = post(
            &base,
            "cancel-relocation",
            serde_json::json!({ "requestId": "move-1" }),
        )
        .await;
        assert_eq!(
            (status, &body["committed"]),
            (409, &serde_json::json!(false)),
            "{body}"
        );

        // A retry of a request whose first send went unanswered is refused too.
        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "move-2" }),
        )
        .await;
        assert_eq!(
            (status, &body["cancelled"]),
            (409, &serde_json::json!(false))
        );
        server.finish();
        // That first send reaching the core only now, with the server gone,
        // takes no lease: nobody would own it.
        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "move-2" }),
        )
        .await;
        assert_eq!(
            (status, &body["cancelled"]),
            (409, &serde_json::json!(true)),
            "{body}"
        );
        assert!(!app.runtime_lifecycle.admission_frozen());

        let (status, body) = post(
            &base,
            "prepare-shutdown",
            serde_json::json!({ "requestId": "quit-1", "purpose": "quit" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["status"]["purpose"], "quit");
    }

    /// A quit that a long deletion keeps from finishing names it, and the
    /// user's 'Keep GDLauncher open' gives the launcher back: the next Play
    /// is admitted instead of 'Runtime shutdown or relocation is in progress'.
    #[tokio::test]
    async fn an_unresolved_quit_names_its_deletion_and_cancel_reopens_launches() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        app.runtime_lifecycle
            .set_settle_budget(std::time::Duration::from_millis(300));
        let base = serve(app.clone()).await;
        // As instance deletion admits it: a Recycle Bin move that does not
        // stop when the quit cancels it.
        let deletion = app
            .runtime_lifecycle
            .admit_owned("delete instance 12", ActivityPolicy::Deletion)
            .unwrap();
        let blocker = serde_json::json!({ "kind": "deletion", "name": "delete instance 12" });

        let request = serde_json::json!({ "requestId": "quit-1", "purpose": "quit" });
        let (status, body) = post(&base, "prepare-shutdown", request.clone()).await;
        assert_eq!(status, 200, "{body}");
        let status = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let (code, body) = post(&base, "shutdown-status", request.clone()).await;
                assert_eq!(code, 200, "{body}");
                if body["status"]["phase"] != "draining" {
                    break body["status"].clone();
                }
                assert_eq!(body["status"]["blockers"], serde_json::json!([blocker]));
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(status["phase"], "unresolved", "{status}");
        assert_eq!(status["blockers"], serde_json::json!([blocker]));
        assert_eq!(status["serversStopped"], false);
        assert!(app.runtime_lifecycle.admit("launch").is_err());

        let (code, body) = post(
            &base,
            "cancel-shutdown",
            serde_json::json!({ "requestId": "quit-1" }),
        )
        .await;
        assert_eq!(code, 200, "{body}");
        assert!(!app.runtime_lifecycle.admission_frozen());
        // What a launch admits first (instance/run prepare_game).
        let launch = app
            .runtime_lifecycle
            .admit_owned("instance 1 preparation/session", ActivityPolicy::Installer)
            .expect("a launch is admitted again");
        launch.finish();

        // The cancelled id holds nothing and cannot take the core again.
        let (code, body) = post(
            &base,
            "cancel-shutdown",
            serde_json::json!({ "requestId": "quit-1" }),
        )
        .await;
        assert_eq!(
            (code, &body["committed"], &body["finishing"]),
            (409, &serde_json::json!(false), &serde_json::json!(false)),
            "{body}"
        );
        // It says so, so a launcher whose cancel answer was lost starts over
        // under a new id instead of showing the user a refusal.
        let (code, body) = post(&base, "prepare-shutdown", request).await;
        assert_eq!(
            (code, &body["cancelled"]),
            (409, &serde_json::json!(true)),
            "{body}"
        );
        assert!(!app.runtime_lifecycle.admission_frozen());
        deletion.finish();
    }

    /// Relocation can no longer reach the freezing shutdown path, and the
    /// rejected request leaves admission open.
    #[tokio::test]
    async fn prepare_shutdown_rejects_relocation_purpose_without_freezing() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let base = serve(app.clone()).await;
        let (status, body) = post(
            &base,
            "prepare-shutdown",
            serde_json::json!({ "requestId": "move-1", "purpose": "relocation" }),
        )
        .await;
        assert!((400..500).contains(&status), "{status} {body}");
        assert!(!app.runtime_lifecycle.admission_frozen());
        drop(app.runtime_lifecycle.admit("launch").unwrap());
    }

    /// Lease, commit, a retried commit after a lost ACK, and a cancel that
    /// reports the commit instead of pretending nothing happened.
    #[tokio::test]
    async fn relocation_lease_commit_is_retryable_and_reported_on_cancel() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let base = serve(app.clone()).await;
        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "move-2" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert!(body["runtimePath"].is_string());
        for _ in 0..2 {
            let (status, body) = post(
                &base,
                "shutdown",
                serde_json::json!({ "requestId": "move-2" }),
            )
            .await;
            assert_eq!(status, 200, "{body}");
        }
        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "move-2" }),
        )
        .await;
        assert_eq!(
            status, 200,
            "a retry re-joins its own committed lease: {body}"
        );
        let (status, body) = post(
            &base,
            "cancel-relocation",
            serde_json::json!({ "requestId": "move-2" }),
        )
        .await;
        assert_eq!(
            (status, &body["committed"]),
            (409, &serde_json::json!(true)),
            "{body}"
        );
        assert!(app.runtime_lifecycle.shutdown_committed());
    }

    /// The blocker check before the folder scan names what runs, takes no
    /// lease and does not count the caller's own lease against it.
    #[tokio::test]
    async fn relocation_blockers_lists_what_runs_without_taking_a_lease() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let base = serve(app.clone()).await;
        let mut server = app.runtime_lifecycle.admit("server 7 run").unwrap();
        server.external();
        server.spawn(ActivityPolicy::Server, || Ok(())).unwrap();

        let (status, body) = post(&base, "relocation-blockers", serde_json::json!({})).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            body["blockers"],
            serde_json::json!([{ "kind": "server", "name": "server 7 run" }])
        );
        assert!(!app.runtime_lifecycle.admission_frozen());
        server.finish();

        let (status, _) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "move-3" }),
        )
        .await;
        assert_eq!(status, 200);
        let (_, own) = post(
            &base,
            "relocation-blockers",
            serde_json::json!({ "requestId": "move-3" }),
        )
        .await;
        assert_eq!(own["blockers"], serde_json::json!([]), "{own}");
        let (_, other) = post(
            &base,
            "relocation-blockers",
            serde_json::json!({ "requestId": "move-4" }),
        )
        .await;
        assert_eq!(
            other["blockers"],
            serde_json::json!([{ "kind": "relocation", "name": "" }])
        );
    }

    /// A prepare-relocation that only reaches the core after the launcher
    /// gave up on it and cancelled its id is refused instead of leaving a
    /// lease nobody owns, which would refuse every launch until a restart.
    #[tokio::test]
    async fn a_prepare_that_arrives_after_its_cancel_takes_no_lease() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let base = serve(app.clone()).await;
        let (status, body) = post(
            &base,
            "cancel-relocation",
            serde_json::json!({ "requestId": "late" }),
        )
        .await;
        assert_eq!(
            (status, &body["committed"]),
            (409, &serde_json::json!(false))
        );
        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "late" }),
        )
        .await;
        assert_eq!(
            (status, &body["cancelled"]),
            (409, &serde_json::json!(true)),
            "{body}"
        );
        assert!(!app.runtime_lifecycle.admission_frozen());
        drop(app.runtime_lifecycle.admit("launch").unwrap());

        let (status, body) = post(
            &base,
            "prepare-relocation",
            serde_json::json!({ "requestId": "fresh" }),
        )
        .await;
        assert_eq!(status, 200, "a new attempt still gets its lease: {body}");
    }

    /// The requests an update install sends (Electron's
    /// prepareOwnedShutdown("update") through stopCoreForShutdown) after an
    /// upgrade that left an empty instance marker, a writer record whose PID
    /// another process now holds and a stray .DS_Store: the core gets ready
    /// and commits, so electron-updater can install. Only a move names them.
    #[tokio::test]
    async fn an_update_install_commits_despite_leftover_process_records() {
        let env = crate::setup_managers_for_test().await;
        let app = env.app.clone();
        let runtime = app.settings_manager().runtime_path.clone();
        let instance = runtime.get_instances().to_path().join("Crashed");
        std::fs::create_dir_all(&instance).unwrap();
        let marker = instance.join(crate::managers::instance::PID_FILE_NAME);
        std::fs::write(&marker, b"").unwrap();
        let writers = runtime.join("__gdl_runtime_writers");
        std::fs::create_dir_all(&writers).unwrap();
        std::fs::write(writers.join(".DS_Store"), b"\0\0\0\x01Bud1").unwrap();
        // This test process is alive and not Java; the start time is not its.
        let reused = writers.join("3f1c2b7e-7a51-4c55-9a0e-2d4b6f0c8e11.json");
        let pid = std::process::id();
        let record =
            format!(r#"{{"version":1,"process":{{"pid":{pid},"start_time":1,"identity":null}}}}"#);
        std::fs::write(&reused, &record).unwrap();
        // The startup check, as start_background_tasks runs it.
        app.runtime_lifecycle.reconcile_children().await.unwrap();
        app.runtime_lifecycle
            .reconcile_root(
                &runtime.get_instances().to_path(),
                crate::managers::instance::PID_FILE_NAME,
            )
            .await
            .unwrap();
        app.instance_manager().scan_instances().await.unwrap();
        app.runtime_lifecycle.reconciliation_finished(Ok(()));
        let base = serve(app.clone()).await;

        let (_, body) = post(&base, "relocation-blockers", serde_json::json!({})).await;
        let blockers = body["blockers"].as_array().unwrap();
        assert!(
            blockers.contains(&serde_json::json!({
                "kind": "unreadableRecord",
                "name": marker.display().to_string()
            })),
            "{body}"
        );
        // Windows and macOS creation times never move, so there another
        // start time proves the PID was reused and the record is removed.
        // On Linux a clock step could explain it (M23): kept, unconfirmed.
        let reuse_proven = cfg!(not(target_os = "linux"));
        assert_eq!(
            blockers.iter().any(|blocker| blocker["pid"] == pid),
            !reuse_proven,
            "{body}"
        );

        let request = serde_json::json!({ "requestId": "update-1", "purpose": "update" });
        let (status, body) = post(&base, "prepare-shutdown", request.clone()).await;
        assert_eq!(status, 200, "{body}");
        let phase = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let (status, body) = post(&base, "shutdown-status", request.clone()).await;
                assert_eq!(status, 200, "{body}");
                if body["status"]["phase"] != "draining" {
                    break body["status"].clone();
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(phase["phase"], "ready", "{phase}");
        let (status, body) = post(&base, "shutdown", request.clone()).await;
        assert_eq!(status, 200, "{body}");
        assert!(app.runtime_lifecycle.shutdown_committed());
        // Kept where they were for the next start.
        assert_eq!(std::fs::read(&marker).unwrap(), b"");
        if !reuse_proven {
            assert_eq!(std::fs::read_to_string(&reused).unwrap(), record);
        }
    }
}
