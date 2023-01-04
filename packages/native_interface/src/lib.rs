use std::path::PathBuf;

use async_stream::stream;
use axum::{extract::Path, routing::get};
use carbon_bindings::Ctx;
use rspc::Config;
use tower_http::cors::{Any, CorsLayer};
use tracing::trace;

// Since it's module_init, make sure it's not running during tests
#[cfg(not(test))]
#[napi::module_init]
fn init_core() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new();
        runtime
            .unwrap() /* This should never fail */
            .block_on(async {
                start_router().await;
            })
    });
}

async fn start_router() {
    let router = carbon_bindings::build_router().build().arced();
    // We disable CORS because this is just an example. DON'T DO THIS IN PRODUCTION!
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = axum::Router::new()
        .route("/", get(|| async { "Hello 'rspc'!" }))
        .route(
            "/rspc/:id",
            router
                .clone()
                .endpoint(|Path(path): Path<String>| {
                    trace!("Client requested operation '{}'", path);
                    Ctx {}
                })
                .axum(),
        )
        .layer(cors);

    let addr = "[::]:4000".parse::<std::net::SocketAddr>().unwrap(); // This listens on IPv6 and IPv4
    trace!("listening on http://{}/rspc/version", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_router() {
        let server = tokio::spawn(async {
            super::start_router().await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let client = reqwest::Client::new();
        let resp = client
            .get("http://localhost:4000/rspc/version")
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);

        server.abort();
    }
}
