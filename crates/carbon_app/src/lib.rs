// allow dead code during development to keep warning outputs meaningful
#![allow(dead_code)]

use crate::managers::{
    java::{discovery::RealDiscovery, java_checker::RealJavaChecker},
    App, AppInner,
};

use rspc::RouterBuilderLike;
use std::{path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, metadata::LevelFilter};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

pub mod api;
mod app_version;
pub(crate) mod db;
pub mod domain;
mod error;
pub mod generate_rspc_ts_bindings;
pub mod managers;
// mod pprocess_keepalive;
mod iridium_client;
mod once_send;
mod runtime_path_override;

#[tokio::main]
pub async fn init() {
    // pprocess_keepalive::init();

    #[cfg(feature = "production")]
    #[cfg(not(test))]
    let _guard = {
        sentry::init((
            env!("CORE_MODULE_DSN"),
            sentry::ClientOptions {
                release: Some(app_version::APP_VERSION.into()),
                ..Default::default()
            },
        ))
    };

    #[cfg(feature = "production")]
    iridium::startup_check();

    info!("Initializing runtime path");
    let runtime_path = runtime_path_override::get_runtime_path_override().await;

    let filter = EnvFilter::try_new(
        "debug,carbon_app=trace,hyper::client::pool=warn,hyper::proto::h1::io=warn,hyper::proto::h1::decode=warn",
    )
    .unwrap();

    let logs_path = runtime_path.join("__gdl_logs__");

    println!("Logs path: {}", logs_path.display());

    if !logs_path.exists() {
        tokio::fs::create_dir_all(&logs_path).await.unwrap();
    }

    let file_appender = tracing_appender::rolling::daily(logs_path, "logs.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_forest::ForestLayer::default())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(filter)
        .init();

    info!("Starting Carbon App v{}", app_version::APP_VERSION);

    info!("Scanning ports");
    let listener = if cfg!(debug_assertions) {
        TcpListener::bind("127.0.0.1:4650").await.unwrap()
    } else {
        get_available_port().await
    };

    start_router(runtime_path, listener).await;
}

async fn get_available_port() -> TcpListener {
    for port in 1025..65535 {
        let conn = TcpListener::bind(format!("127.0.0.1:{port}")).await;
        match conn {
            Ok(listener) => return listener,
            Err(_) => continue,
        }
    }

    panic!("No available port found");
}

async fn start_router(runtime_path: PathBuf, listener: TcpListener) {
    info!("Starting router");
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);

    let router: Arc<rspc::Router<App>> = crate::api::build_rspc_router().expose().build().arced();

    // We disable CORS because this is just an example. DON'T DO THIS IN PRODUCTION!
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = AppInner::new(invalidation_sender, runtime_path).await;
    crate::managers::java::JavaManager::scan_and_sync(
        &app.prisma_client,
        &RealDiscovery,
        &RealJavaChecker,
    )
    .await
    .unwrap();

    let app1 = app.clone();
    let app = axum::Router::new()
        .nest("/", crate::api::build_axum_vanilla_router())
        .nest("/rspc", router.endpoint(move || app).axum())
        .layer(cors)
        .with_state(app1);

    let port = listener.local_addr().unwrap().port();

    // As soon as the server is ready, notify via stdout
    tokio::spawn(async move {
        let mut counter = 0;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
        let reqwest_client = reqwest::Client::new();
        loop {
            counter += 1;
            // If we've waited for 10 seconds, give up
            if counter > 50 {
                panic!("Server failed to start in time");
            }

            interval.tick().await;
            let res = reqwest_client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await;

            if res.is_ok() {
                info!("_STATUS_: READY|{port}");
                break;
            }
        }
    });

    let std_tcp_listener = listener.into_std().unwrap();

    axum::Server::from_tcp(std_tcp_listener)
        .unwrap()
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
struct TestEnv {
    tmpdir: PathBuf,
    app: App,
    invalidation_recv: tokio::sync::broadcast::Receiver<api::InvalidationEvent>,
}

#[cfg(test)]
impl TestEnv {
    async fn restart_in_place(&mut self) {
        let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);
        self.app = AppInner::new(invalidation_sender, self.tmpdir.clone()).await;
    }
}

#[cfg(test)]
impl std::ops::Deref for TestEnv {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

#[cfg(test)]
impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.tmpdir);
    }
}

#[cfg(test)]
async fn setup_managers_for_test() -> TestEnv {
    let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
    let temp_path = temp_dir.into_path();
    info!("Test RTP: {}", temp_path.to_str().unwrap());
    let (invalidation_sender, invalidation_recv) = tokio::sync::broadcast::channel(200);

    TestEnv {
        tmpdir: temp_path.clone(),
        invalidation_recv,
        app: AppInner::new(invalidation_sender, temp_path).await,
    }
}

#[cfg(test)]
mod test {
    use crate::get_available_port;

    #[tokio::test]
    async fn test_router() {
        let tcp_listener = get_available_port().await;
        let port = &tcp_listener.local_addr().unwrap().port();
        let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
        let server = tokio::spawn(async move {
            super::start_router(temp_dir.into_path(), tcp_listener).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let client = reqwest::Client::new();
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
