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
mod base_api_override;
pub mod cache_middleware;
pub mod domain;
mod error;
pub mod iridium_client;
mod launcher;
mod livenesstracker;
mod logger;
pub mod managers;
mod once_send;
mod platform;
mod runtime_path_override;
mod util;

pub fn main() {
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

            // Until the shutdown loop in `start_router` takes over, nothing
            // of the user's runs here yet, and a launcher that is gone ends
            // this core at once, as SIGTERM does before that loop.
            let launcher = launcher::Watch::from_args();
            tokio::spawn({
                let launcher = launcher.clone();
                async move {
                    launcher.gone().await;
                    if !SHUTDOWN_LOOP_STARTED.load(std::sync::atomic::Ordering::SeqCst) {
                        info!("The launcher is gone; exiting");
                        logger::flush_and_exit(0);
                    }
                }
            });

            // After the logger so its `E2E MODE` warnings land somewhere: with
            // no subscriber installed yet, `tracing::warn!` is a silent no-op.
            managers::account::endpoints::init_from_args();

            // Clean up leftover temp files/folders from previous sessions,
            // wherever they were built (see carbon_rt_path's staging module).
            carbon_rt_path::RuntimePath::new(runtime_path.clone())
                .cleanup_staging()
                .await;
            // And addon install staging an earlier session left in the data
            // folder (quit or crashed mid-install, or kept by an older build).
            managers::instance::installer::publication::sweep_stale_staging(&runtime_path).await;

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

            start_router(runtime_path, base_api_override, listener, launcher).await;
        });
}

/// Set once the loop in `start_router` answers termination requests with an
/// orderly shutdown. Before that a launcher that is gone ends the core at once.
static SHUTDOWN_LOOP_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Waits for the next reason to shut down: a termination `signal`, the end
/// of the launcher, or, once the launcher is gone, `retry` having passed
/// since the last attempt.
async fn next_stop_request(
    signal: impl std::future::Future<Output = ()>,
    launcher: &launcher::Watch,
    launcher_gone: &mut bool,
    retry: std::time::Duration,
) {
    tokio::select! {
        _ = signal => {}
        _ = launcher.gone(), if !*launcher_gone => {
            *launcher_gone = true;
            tracing::warn!("The launcher is gone; shutting down");
        }
        _ = tokio::time::sleep(retry), if *launcher_gone => {}
    }
}

/// How long the core waits before it tries again to shut down after the
/// launcher is gone and an attempt ended unresolved: nobody is left to ask.
const LAUNCHER_GONE_RETRY: std::time::Duration = std::time::Duration::from_secs(30);

/// Waits for a request to terminate this process: SIGTERM or SIGINT on
/// unix (an external `kill`, Ctrl+C, or a service manager's stop signal),
/// or Ctrl+C everywhere else.
///
/// This does NOT observe Windows' `TerminateProcess` — that is how
/// Electron actually kills the core on Windows, and it is not a catchable
/// signal on that platform. The authenticated handshake must settle ownership
/// before Electron terminates the core. After an unclean exit, server pidfile
/// recovery retains live/ambiguous owners without automatically killing them.
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

/// Re-exchanges the GDL tokens on every startup to ensure they're valid
/// (handles backend target changes where JWT signing keys differ).
///
/// In the background, so the launcher does not wait on the API to open: every
/// request that needs a GDL token checks the stored one first and exchanges it
/// on demand (`ensure_gdl_auth_token`), with or without this pass.
fn refresh_gdl_tokens_in_background(app: &App) -> tokio::task::JoinHandle<()> {
    let app = app.clone();
    tokio::spawn(async move {
        let t = std::time::Instant::now();
        if let Err(e) = app.account_manager().refresh_all_gdl_tokens().await {
            tracing::warn!("Failed to refresh GDL tokens on startup: {}", e);
        }
        debug!(
            "[startup-timing] refresh_all_gdl_tokens completed in {:.2}s",
            t.elapsed().as_secs_f64()
        );
    })
}

async fn start_router(
    runtime_path: PathBuf,
    base_api_override: String,
    listener: TcpListener,
    launcher: launcher::Watch,
) {
    info!("Starting router");
    let startup_total = std::time::Instant::now();
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(1000);

    // Per-process API token: required on every Axum/rspc request from the
    // renderer. Rotated on each launch in release builds; fixed in dev so the
    // Electron renderer can hardcode the same value.
    let api_token = generate_api_token();
    crate::api::auth::set_expected_token(api_token.clone());
    let port = listener
        .local_addr()
        .expect("Failed to get local address from TCP listener")
        .port();
    crate::api::auth::set_local_port(port);

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

    // Before the server answers, so a stale seasonal theme never flashes.
    if let Err(e) = app.theme_manager().check_season().await {
        tracing::warn!("the theme season check failed: {e:#}");
    }

    // Detached: a full instance scan must not hold up the rest of startup.
    tokio::spawn({
        let app = app.clone();
        async move { app.start_background_tasks().await }
    });

    refresh_gdl_tokens_in_background(&app);

    // Before the sync below relinks the system profiles, so the Javas picked
    // by hand are read before anything replaces them.
    let t = std::time::Instant::now();
    match app.settings_manager().get_settings().await {
        Ok(settings) => {
            match crate::managers::java::major_defaults::migrate_major_defaults(&app.db, &settings)
                .await
            {
                Ok(true) => tracing::info!("Kept the chosen Javas as per-version defaults"),
                Ok(false) => {}
                Err(e) => tracing::warn!("Could not migrate the chosen Javas to defaults: {e:#}"),
            }
        }
        Err(e) => tracing::warn!("Could not read settings to migrate Java defaults: {e:#}"),
    }
    debug!(
        "[startup-timing] migrate_major_defaults completed in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    // Relinking the profiles before the migration has read them would lose
    // the Javas picked by hand; it runs again at the next start.
    let link_profiles = crate::managers::java::major_defaults::major_defaults_migrated(&app.db)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Could not tell whether the Java defaults were migrated: {e:#}");
            false
        });
    if !link_profiles {
        tracing::warn!("Not linking the Java profiles until the Java defaults are migrated");
    }

    let t = std::time::Instant::now();
    crate::managers::java::JavaManager::scan_and_sync(
        &app.db,
        &RealDiscovery::new(app.settings_manager().runtime_path.clone()),
        &RealJavaChecker::new(app.runtime_lifecycle.clone()),
        link_profiles,
    )
    .await
    .expect("Failed to scan and sync java system profiles");
    debug!(
        "[startup-timing] JavaManager::scan_and_sync completed in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    crate::managers::java::maintenance::spawn_java_maintenance(app.clone());

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
    // The signal path consumes the same request-owned freeze/drain/commit as
    // authenticated shutdown. Unsettled participants retain the core and evidence.
    // Each signal is its own request, so a later signal takes the place of an
    // attempt that ended unresolved, as the launcher's 'Quit now' does
    // before it escalates to SIGKILL.
    //
    // A launcher that crashed or was killed can send none of this: on unix
    // the core would stay behind, unseen, with its servers. Its end counts as
    // a request too, and with nobody left to ask again the core tries again
    // by itself for as long as an attempt ends unresolved.
    SHUTDOWN_LOOP_STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
    tokio::spawn(async move {
        let mut launcher_gone = false;
        loop {
            next_stop_request(
                wait_for_termination_signal(),
                &launcher,
                &mut launcher_gone,
                LAUNCHER_GONE_RETRY,
            )
            .await;
            if app2.runtime_lifecycle.shutdown_committed() {
                logger::flush_and_exit(0);
            }
            let request = format!("core-termination-signal-{}", uuid::Uuid::new_v4());
            let purpose = managers::runtime_lifecycle::ShutdownPurpose::Quit;
            let draining_app = app2.clone();
            let id = request.clone();
            match app2
                .runtime_lifecycle
                .prepare_shutdown_and_commit(&request, purpose, async move {
                    let drain = draining_app.runtime_lifecycle.shutdown_drain(&id);
                    draining_app
                        .server_manager()
                        .shutdown_running_for(&drain)
                        .await
                })
                .await
            {
                Ok(()) => logger::flush_and_exit(0),
                Err(error) => tracing::error!("Shutdown unresolved; core retained: {error:#}"),
            }
        }
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
                launcher::tell(format_args!("_STATUS_:READY|{port}|{api_token}"));
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
mod stop_request_tests {
    use super::{launcher, next_stop_request};
    use std::time::Duration;

    const RETRY: Duration = Duration::from_secs(30);
    async fn within(time: Duration, wait: impl std::future::Future<Output = ()>) -> bool {
        tokio::time::timeout(time, wait).await.is_ok()
    }

    #[tokio::test(start_paused = true)]
    async fn only_a_signal_stops_a_core_whose_launcher_is_there() {
        let (launcher_end, watch) = launcher::Watch::over_pipe();
        let mut gone = false;
        let never = std::future::pending::<()>();
        assert!(
            !within(
                RETRY * 10,
                next_stop_request(never, &watch, &mut gone, RETRY)
            )
            .await
        );
        assert!(!gone);
        assert!(within(RETRY, next_stop_request(async {}, &watch, &mut gone, RETRY)).await);
        assert!(!gone);
        drop(launcher_end);
    }

    #[tokio::test]
    async fn the_end_of_the_launcher_is_one_request_and_then_one_more_at_every_retry() {
        let (launcher_end, watch) = launcher::Watch::over_pipe();
        let mut gone = false;
        let retry = Duration::from_millis(400);
        drop(launcher_end);
        let never = std::future::pending::<()>;
        assert!(
            within(
                Duration::from_millis(300),
                next_stop_request(never(), &watch, &mut gone, retry)
            )
            .await
        );
        assert!(gone);
        // Not again at once, although the launcher stays gone: an attempt
        // that ended unresolved is not repeated in a tight loop.
        assert!(
            !within(
                Duration::from_millis(200),
                next_stop_request(never(), &watch, &mut gone, retry)
            )
            .await
        );
        let started = std::time::Instant::now();
        assert!(
            within(
                Duration::from_secs(10),
                next_stop_request(never(), &watch, &mut gone, retry)
            )
            .await
        );
        assert!(started.elapsed() >= Duration::from_millis(350));
        // A signal is still answered at once meanwhile.
        assert!(
            within(
                Duration::from_millis(100),
                next_stop_request(async {}, &watch, &mut gone, retry)
            )
            .await
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_core_nobody_watches_is_never_stopped_by_a_retry() {
        let watch = launcher::Watch::off();
        let mut gone = false;
        let never = std::future::pending::<()>();
        assert!(
            !within(
                RETRY * 100,
                next_stop_request(never, &watch, &mut gone, RETRY)
            )
            .await
        );
        assert!(!gone);
    }
}

#[cfg(test)]
mod startup_wait_tests {
    use crate::get_available_port;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use carbon_repos::dbtypes::DbDateTime;
    use carbon_repos::repos::account as account_repo;
    use chrono::Utc;
    use std::time::{Duration, Instant};

    /// The GDLauncher API of this test: it knows the terms in force, and
    /// never answers a token exchange. Reports every exchange it is asked.
    async fn api_that_never_exchanges_a_token() -> (String, tokio::sync::mpsc::UnboundedReceiver<()>)
    {
        let (asked, was_asked) = tokio::sync::mpsc::unbounded_channel();
        let api = axum::Router::new()
            .route(
                "/v1/latest_consent_checksum",
                axum::routing::get(|| async { "checksum" }),
            )
            .route(
                "/v1/auth/token",
                axum::routing::post(move || {
                    let asked = asked.clone();
                    async move {
                        let _ = asked.send(());
                        std::future::pending::<&'static str>().await
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, api).await });
        (url, was_asked)
    }

    /// A data folder with one Microsoft account whose id token is still
    /// good, which is the account the start-up pass exchanges a token for.
    /// It has no refresh token, so nothing else has a request to make for it.
    async fn data_folder_with_a_signed_in_account() -> std::path::PathBuf {
        let folder = dunce::canonicalize(tempfile::tempdir().unwrap().into_path()).unwrap();
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let claims =
            URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{}}}"#, Utc::now().timestamp() + 3600));
        let id_token = format!("{header}.{claims}.c2ln");
        let next_day = Utc::now() + chrono::Duration::days(1);

        let db = crate::managers::db_bootstrap::open_for_test(folder.clone()).await;
        account_repo::insert_account_microsoft(
            &db,
            "11111111-2222-3333-4444-555555555555",
            "Alex",
            DbDateTime(Utc::now().into()),
            "minecraft-access-token",
            Some(DbDateTime(next_day.into())),
            None,
            Some(&id_token),
            None,
            None,
        )
        .await
        .unwrap();
        folder
    }

    /// The loading screen stays until the core answers. It used to wait for
    /// the token exchange of every signed-in account first, for up to two
    /// minutes on a network that drops packets.
    ///
    /// The one test that starts the router: its API token is set once per
    /// process, so a second start in this test binary panics.
    #[tokio::test]
    async fn the_launcher_opens_while_a_token_exchange_is_still_waiting_on_the_api() {
        let (api, mut was_asked) = api_that_never_exchanges_a_token().await;
        let folder = data_folder_with_a_signed_in_account().await;
        let listener = get_available_port().await;
        let port = listener.local_addr().unwrap().port();
        let core = tokio::spawn(super::start_router(
            folder,
            api,
            listener,
            crate::launcher::Watch::off(),
        ));

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let started = Instant::now();
        let greeting = loop {
            let answer = client
                .get(format!("http://127.0.0.1:{port}"))
                .timeout(Duration::from_millis(500))
                .send()
                .await;
            if let Ok(answer) = answer {
                assert_eq!(answer.status(), 200);
                break answer.text().await.unwrap();
            }
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the core was not answering after {:?}",
                started.elapsed()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        assert_eq!(greeting, "Hello 'rspc'!");

        // The pass still runs: the exchange was asked, and is never answered.
        tokio::time::timeout(Duration::from_secs(20), was_asked.recv())
            .await
            .expect("no token exchange was asked for the signed-in account")
            .unwrap();
        assert!(!core.is_finished());
        core.abort();
    }
}
