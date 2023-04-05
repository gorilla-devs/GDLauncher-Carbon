// allow dead code during development to keep warning outputs meaningful
#![allow(dead_code)]

use crate::{
    app_version::APP_VERSION,
    managers::{
        java::{discovery::RealDiscovery, java_checker::RealJavaChecker},
        App, AppInner,
    },
};
use rspc::RouterBuilderLike;
use std::{ops::Deref, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

pub mod api;
mod app_version;
pub(crate) mod db;
pub mod domain;
mod error;
pub mod generate_rspc_ts_bindings;
pub mod managers;
// mod pprocess_keepalive;
mod runtime_path_override;

#[tokio::main]
pub async fn init() {
    // pprocess_keepalive::init();

    println!("Starting Carbon App");

    if !cfg!(debug_assertions) {
        println!("Initializing Sentry");
        let _guard = sentry::init((
            env!("SENTRY_DSN"),
            sentry::ClientOptions {
                release: Some(APP_VERSION.into()),
                ..Default::default()
            },
        ));
    }

    println!("Initializing runtime path");
    let runtime_path = runtime_path_override::get_runtime_path_override().await;
    println!("Scanning ports");
    let port = get_available_port().await.unwrap();

    start_router(runtime_path, port).await;
}

async fn get_available_port() -> Option<u16> {
    for port in 1025..65535 {
        let conn = TcpListener::bind(format!("127.0.0.1:{port}")).await;
        match conn {
            Ok(_) => return Some(port),
            Err(_) => continue,
        }
    }

    None
}

async fn start_router(runtime_path: PathBuf, port: u16) {
    println!("Starting router");
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

    let addr = format!("[::]:{port}")
        .parse::<std::net::SocketAddr>()
        .unwrap(); // This listens on IPv6 and IPv4

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
                println!("_STATUS_: READY|{port}");
                break;
            }
        }
    });

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
struct TestEnv {
    tmpdir: PathBuf,
    app: App,
}

#[cfg(test)]
impl Deref for TestEnv {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

#[cfg(test)]
impl Drop for TestEnv {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.tmpdir).unwrap();
    }
}

#[cfg(test)]
async fn setup_managers_for_test() -> TestEnv {
    let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
    let temp_path = temp_dir.into_path();
    println!("Test RTP: {}", temp_path.to_str().unwrap());
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);

    TestEnv {
        tmpdir: temp_path.clone(),
        app: AppInner::new(invalidation_sender, temp_path).await,
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_router() {
        let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
        let server = tokio::spawn(async {
            super::start_router(temp_dir.into_path(), 4000).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let client = reqwest::Client::new();
        let resp = client.get("http://localhost:4000").send().await.unwrap();
        let resp_code = resp.status();
        let resp_body = resp.text().await.unwrap();

        assert_eq!(resp_code, 200);
        assert_eq!(resp_body, "Hello 'rspc'!");

        server.abort();
    }
}
