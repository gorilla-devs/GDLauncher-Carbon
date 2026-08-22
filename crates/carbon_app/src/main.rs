// allow dead code during development to keep warning outputs meaningful
#![allow(warnings)]
#![allow(dead_code)]

use crate::managers::{
    App, AppInner,
    java::{
        discovery::{Discovery, RealDiscovery},
        java_checker::RealJavaChecker,
    },
};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{debug, info};

pub mod api;
mod app_version;
pub mod cache_middleware;
pub mod domain;
mod error;
pub mod iridium_client;
mod livenesstracker;
pub mod managers;
mod platform;
// mod pprocess_keepalive;
mod base_api_override;
mod logger;
mod once_send;
mod runtime_path_override;
mod util;

pub fn main() {
    // pprocess_keepalive::init();
    #[cfg(debug_assertions)]
    {
        let mut args = std::env::args();
        if args.any(|arg| arg == "--generate-ts-bindings") {
            crate::api::build_rspc_router(String::new())
                .config(
                    rspc::Config::new().export_ts_bindings(
                        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .parent()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join("packages")
                            .join("core_module")
                            .join("bindings.d.ts"),
                    ),
                )
                .build();

            // exit process with ok status
            std::process::exit(0);
        }
    }

    #[cfg(feature = "production")]
    #[cfg(not(test))]
    let sentry_session_id = &uuid::Uuid::new_v4().to_string();

    #[cfg(feature = "production")]
    #[cfg(not(test))]
    let _guard = {
        let s = sentry::init((
            env!("CORE_MODULE_DSN"),
            sentry::ClientOptions {
                release: Some(app_version::APP_VERSION.into()),
                ..Default::default()
            },
        ));

        sentry::configure_scope(|scope| {
            scope.set_tag("gdl_session_id", &sentry_session_id);
        });

        s
    };

    let x = 1;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(256)
        .build()
        .unwrap()
        .block_on(async {
            daedalus::Branding::set_branding(daedalus::Branding::new(
                "gdlauncher".to_string(),
                "".to_string(),
            ))
            .expect("Branding not to fail");

            #[cfg(feature = "production")]
            iridium::startup_check();

            info!("Initializing runtime path");
            let runtime_path = runtime_path_override::get_runtime_path_override().await;
            let base_api_override = base_api_override::get_base_api_override();

            logger::setup_logger(&runtime_path).await;
            // After the logger so a panic anywhere below this point flushes
            // the release-build file log before unwinding past it — see
            // `logger::install_panic_hook`'s own doc comment for why a
            // static `WorkerGuard` needs this at all.
            logger::install_panic_hook();

            // After the logger so its `E2E MODE` warnings land somewhere: with
            // no subscriber installed yet, `tracing::warn!` is a silent no-op.
            managers::account::endpoints::init_from_args();

            // Clean up leftover temp files/folders from previous sessions
            let temp_path = carbon_rt_path::TempPath::new(runtime_path.join("temp"));
            temp_path.cleanup_all().await;

            info!("Starting Carbon App v{}", app_version::APP_VERSION);

            #[cfg(feature = "production")]
            #[cfg(not(test))]
            info!("Sentry Session Id: {}", sentry_session_id);

            info!("Runtime path: {}", runtime_path.display());

            info!("Scanning ports");

            let init_time = std::time::Instant::now();

            let listener = if cfg!(debug_assertions) {
                TcpListener::bind("127.0.0.1:4650").await.unwrap()
            } else {
                get_available_port().await
            };

            info!(
                "Found port: {:?} in {:?}",
                listener.local_addr(),
                init_time.elapsed()
            );

            start_router(runtime_path, base_api_override, listener).await;
        });
}

/// Waits for a request to terminate this process: SIGTERM or SIGINT on
/// unix (an external `kill`, Ctrl+C, or a service manager's stop signal),
/// or Ctrl+C everywhere else.
///
/// This does NOT observe Windows' `TerminateProcess` — that is how
/// Electron actually kills the core on Windows, and it is not a catchable
/// signal on that platform. Orphaned server JVMs from that path are instead
/// cleaned up on the next launch by the pidfile check in
/// `ServerManager::load_servers`.
#[cfg(unix)]
async fn wait_for_termination_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install a SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install a SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_termination_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn get_available_port() -> TcpListener {
    info!("Scanning for available port");
    for port in 1025..65535 {
        let conn = TcpListener::bind(format!("127.0.0.1:{port}")).await;
        match conn {
            Ok(listener) => return listener,
            Err(_) => continue,
        }
    }

    tracing::error!(
        "No available port found in range 1025-65535. Please close some applications and try again."
    );

    panic!(
        "No available port found in range 1025-65535. All ports appear to be in use. Please close some applications that may be using network ports and try again."
    );
}

/// In dev builds the token must be a fixed value the Electron renderer can
/// hardcode — production builds rotate per launch. This string is intentionally
/// recognizable so it's obvious if it ever leaks into a non-debug build.
const DEV_API_TOKEN: &str = "dev-mode-only-do-not-use-in-production";

fn generate_api_token() -> String {
    if cfg!(debug_assertions) {
        return DEV_API_TOKEN.to_string();
    }
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

async fn start_router(runtime_path: PathBuf, base_api_override: String, listener: TcpListener) {
    info!("Starting router");
    let startup_total = std::time::Instant::now();
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(1000);

    // Per-process API token: required on every Axum/rspc request from the
    // renderer. Rotated on each launch in release builds; fixed in dev so the
    // Electron renderer can hardcode the same value.
    let api_token = generate_api_token();
    crate::api::auth::set_expected_token(api_token.clone());

    let router: Arc<rspc::Router<App>> = crate::api::build_rspc_router(base_api_override.clone())
        .build()
        .arced();

    // CORS: allow renderer origins only. Electron's file:// origin appears as
    // "null" in browsers; dev mode runs on http://localhost:* / 127.0.0.1:*.
    let cors = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_origin(AllowOrigin::predicate(|origin, _req| {
            let Ok(origin_str) = origin.to_str() else {
                return false;
            };
            origin_str == "null"
                || origin_str.starts_with("http://localhost:")
                || origin_str.starts_with("http://127.0.0.1:")
                || origin_str == "http://localhost"
                || origin_str == "http://127.0.0.1"
        }));

    let t = std::time::Instant::now();
    let app = AppInner::new(invalidation_sender, runtime_path, base_api_override).await;
    debug!(
        "[startup-timing] AppInner::new completed in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    // Detached: a full instance scan must not hold up the rest of startup.
    tokio::spawn({
        let app = app.clone();
        async move { app.start_background_tasks().await }
    });

    // Re-exchange GDL tokens on every startup to ensure they're valid
    // (handles backend target changes where JWT signing keys differ)
    //
    // Bounded so a stalled connection can't block `axum::serve` forever: generous
    // enough for the HTTP client's own timeout/retries to get through a handful of
    // accounts, but finite. Any account left un-refreshed when this times out is
    // simply refreshed later, on demand, by `ensure_gdl_auth_token`, so a timeout
    // here is handled exactly like the existing error path: log and move on.
    const REFRESH_GDL_TOKENS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

    let t = std::time::Instant::now();
    match tokio::time::timeout(
        REFRESH_GDL_TOKENS_TIMEOUT,
        app.account_manager().refresh_all_gdl_tokens(),
    )
    .await
    {
        Ok(Err(e)) => tracing::warn!("Failed to refresh GDL tokens on startup: {}", e),
        Err(_) => tracing::warn!(
            "Timed out after {:?} refreshing GDL tokens on startup",
            REFRESH_GDL_TOKENS_TIMEOUT
        ),
        Ok(Ok(())) => {}
    }
    debug!(
        "[startup-timing] refresh_all_gdl_tokens completed in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    let t = std::time::Instant::now();
    let auto_manage_java_system_profiles = app
        .settings_manager()
        .get_settings()
        .await
        .unwrap()
        .auto_manage_java_system_profiles;
    debug!(
        "[startup-timing] settings.get_settings completed in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    let t = std::time::Instant::now();
    crate::managers::java::JavaManager::scan_and_sync(
        auto_manage_java_system_profiles,
        &app.db,
        &RealDiscovery::new(app.settings_manager().runtime_path.clone()),
        &RealJavaChecker,
    )
    .await
    .expect("Failed to scan and sync java system profiles");
    debug!(
        "[startup-timing] JavaManager::scan_and_sync completed in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    let app1 = app.clone();
    let app2 = app.clone();
    let rspc_axum_router: axum::Router<Arc<AppInner>> =
        rspc_axum::endpoint(router, move || app.clone());

    let app = axum::Router::new()
        .nest("/", crate::api::build_axum_vanilla_router())
        .nest("/rspc", rspc_axum_router)
        .layer(axum::middleware::from_fn(crate::api::auth::require_token))
        .layer(cors)
        .with_state(app1);

    let port = listener
        .local_addr()
        .expect("Failed to get local address from TCP listener")
        .port();

    debug!(
        "[startup-timing] reached axum::serve in {:.2}s total",
        startup_total.elapsed().as_secs_f64()
    );

    // Graceful shutdown on external termination: without this, a running
    // server's JVM is orphaned whenever this core process is killed instead
    // of exiting through its own request handling (an external `kill`,
    // Ctrl+C while running `pnpm watch:core`, or Electron's normal-quit
    // path).
    //
    // Servers only, deliberately. A local server is infrastructure the
    // launcher hosts, so it stops with the launcher; a *game* is the user's
    // session, and closing the launcher mid-game does not end it. A game
    // that outlives this process is recorded in its instance's pidfile and
    // picked back up by the next startup's reconciliation
    // (`InstanceManager::scan_instances`).
    //
    // `shutdown_running` bounds itself to ~3s, so this task always resolves
    // and exits well inside the ~5s Electron waits before force killing the
    // core.
    tokio::spawn(async move {
        wait_for_termination_signal().await;
        info!("Termination signal received, shutting down running servers before exit");
        app2.server_manager().shutdown_running().await;
        // Through `flush_and_exit` rather than a bare `std::process::exit`
        // so the `info!` line just above reaches disk before the process
        // dies, the same reasoning as the fatal-DB-error exit in
        // `managers/mod.rs`.
        logger::flush_and_exit(0);
    });

    // As soon as the server is ready, notify via stdout
    tokio::spawn(async move {
        let mut counter = 0;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
        let reqwest_client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build health check HTTP client");
        let health_check_start = std::time::Instant::now();
        loop {
            counter += 1;
            // If we've waited for 40 seconds, give up
            if counter > 200 {
                let error = "Server failed to start within 40 seconds. This may indicate a system issue preventing local connections to localhost (maybe a proxy?).";

                tracing::error!(error);
                panic!("{}", error);
            }

            interval.tick().await;
            let res = reqwest_client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await;

            if res.is_ok() {
                debug!(
                    "[startup-timing] health check responded after {:.2}s ({} polls)",
                    health_check_start.elapsed().as_secs_f64(),
                    counter
                );
                debug!(
                    "[startup-timing] READY emitted at {:.2}s after start_router",
                    startup_total.elapsed().as_secs_f64()
                );
                info!("_STATUS_:READY|{port}|<token redacted>");
                println!("_STATUS_:READY|{port}|{api_token}");
                break;
            }
        }
    });

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
struct TestEnv {
    tmpdir: PathBuf,
    //log_guard: tracing_appender::non_blocking::WorkerGuard,
    app: App,
    invalidation_recv: tokio::sync::broadcast::Receiver<api::InvalidationEvent>,
}

#[cfg(test)]
/// Every test waits for startup to settle, so a hang in there stops the whole
/// suite rather than one test. Failing fast turns that into something with a
/// message on it. Deliberately test-only: the app spawns the same work
/// detached, where a slow scan on a large install is not a fault and
/// cancelling it half-way would leave the instance map worse than late.
const STARTUP_TASKS_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(test)]
async fn start_background_tasks_or_panic(app: &App) {
    if tokio::time::timeout(STARTUP_TASKS_TEST_TIMEOUT, app.start_background_tasks())
        .await
        .is_err()
    {
        panic!(
            "startup background tasks did not finish within {:?} — instance \
             scanning, server scanning or the metadata cache loops are stuck",
            STARTUP_TASKS_TEST_TIMEOUT
        );
    }
}

#[cfg(test)]
impl TestEnv {
    async fn restart_in_place(&mut self) {
        let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);
        self.app = AppInner::new(
            invalidation_sender,
            self.tmpdir.clone(),
            crate::util::base_api::get_base_api_env!(),
        )
        .await;
        start_background_tasks_or_panic(&self.app).await;
    }
}

#[cfg(test)]
impl std::ops::Deref for TestEnv {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

// #[cfg(test)]
// impl Drop for TestEnv {
//     fn drop(&mut self) {
//         let _ = std::fs::remove_dir_all(&self.tmpdir);
//     }
// }

#[cfg(test)]
async fn setup_managers_for_test() -> TestEnv {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = dunce::canonicalize(temp_dir.into_path()).unwrap();
    //let log_guard = logger::setup_logger(&temp_path).await;
    println!("Test RTP: {}", temp_path.to_str().unwrap());
    let (invalidation_sender, invalidation_recv) = tokio::sync::broadcast::channel(200);

    TestEnv {
        tmpdir: temp_path.clone(),
        // log_guard,
        invalidation_recv,
        app: {
            let app = AppInner::new(
                invalidation_sender,
                temp_path,
                crate::util::base_api::get_base_api_env!(),
            )
            .await;

            // Awaited rather than spawned: the scan reconciles on-disk
            // pidfiles and removes the stale ones, so a test that writes one
            // would otherwise be racing it. Finishing here means every test
            // starts from an app whose startup has already settled.
            start_background_tasks_or_panic(&app).await;

            app
        },
    }
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_eq_display {
    ($a:expr, $b:expr) => {
        if $a != $b {
            panic!(
                "Assertion failed: left == right\nleft:\n{a_val}\nright:\n{b_val}",
                a_val = $a,
                b_val = $b,
            );
        }
    };
}

#[macro_export]
macro_rules! mirror_into {
    ($a:path, $b:path, |$value:ident| $expr:expr) => {
        impl From<$a> for $b {
            fn from($value: $a) -> Self {
                use $a as Other;

                $expr
            }
        }

        impl From<$b> for $a {
            fn from($value: $b) -> Self {
                use $b as Other;

                $expr
            }
        }
    };
}

#[cfg(test)]
mod test {
    use crate::get_available_port;

    #[tokio::test]
    async fn test_router() {
        let tcp_listener = get_available_port().await;
        let port = &tcp_listener.local_addr().unwrap().port();
        let temp_dir = tempfile::tempdir().unwrap();
        let server = tokio::spawn(async move {
            super::start_router(
                temp_dir.into_path(),
                crate::util::base_api::get_base_api_env!(),
                tcp_listener,
            )
            .await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let resp = client
            .get(format!("http://127.0.0.1:{port}",))
            .send()
            .await
            .unwrap();
        let resp_code = resp.status();
        let resp_body = resp.text().await.unwrap();

        assert_eq!(resp_code, 200);
        assert_eq!(resp_body, "Hello 'rspc'!");

        server.abort();
    }
}
