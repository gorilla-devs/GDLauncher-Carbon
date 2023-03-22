// allow dead code during development to keep warning outputs meaningful
#![allow(dead_code)]

use crate::{
    app_version::APP_VERSION,
    managers::{App, AppInner},
};
use rspc::RouterBuilderLike;
use std::{path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

pub mod api;
pub(crate) mod db;
pub mod managers;

mod app_version;
mod error;
pub mod generate_rspc_ts_bindings;
// mod pprocess_keepalive;
mod runtime_path_override;

#[tokio::main]
pub async fn init() {
    // pprocess_keepalive::init();

    println!("Starting Carbon App");
    if cfg!(debug_assertions) {
        let parent_env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".env");

        if parent_env_path.exists() {
            dotenvy::from_filename(parent_env_path).ok();
        }
    }

    let _guard = sentry::init((
        dotenvy_macro::dotenv!("SENTRY_DSN"),
        sentry::ClientOptions {
            release: Some(APP_VERSION.into()),
            ..Default::default()
        },
    ));

    let runtime_path = runtime_path_override::get_runtime_path_override().await;
    let port = get_available_port().await.unwrap();

    start_router(runtime_path, port).await;
}

async fn get_available_port() -> Option<u16> {
    for port in 1025..65535 {
        if (TcpListener::bind(("[::]:", port))).await.is_ok() {
            return Some(port);
        }
    }

    None
}

#[inline(never)]
async fn start_router(runtime_path: PathBuf, port: u16) {
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);

    let router: Arc<rspc::Router<App>> = crate::api::build_rspc_router().expose().build().arced();

    // We disable CORS because this is just an example. DON'T DO THIS IN PRODUCTION!
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = AppInner::new(invalidation_sender, runtime_path).await;

    let app = axum::Router::new()
        .nest("/", crate::api::build_axum_vanilla_router())
        .nest("/rspc", router.endpoint(move || app).axum())
        .layer(cors);

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
                .get(format!("http://localhost:{port}/health"))
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
async fn setup_managers_for_test() -> App {
    let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
    let temp_path = temp_dir.into_path();
    println!("Test RTP: {}", temp_path.to_str().unwrap());
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);
    AppInner::new(invalidation_sender, temp_path).await
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
