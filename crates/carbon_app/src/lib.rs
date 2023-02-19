use crate::managers::{Managers, ManagersInner};
use rspc::RouterBuilderLike;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub mod api;
pub(crate) mod db;
pub mod managers;

pub mod generate_rspc_ts_bindings;
mod runtime_path_override;

// Since it's module_init, make sure it's not running during tests
#[cfg(not(test))]
pub fn init() {
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
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);

    let router: Arc<rspc::Router<Managers>> =
        crate::api::build_rspc_router().expose().build().arced();

    // We disable CORS because this is just an example. DON'T DO THIS IN PRODUCTION!
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let runtime_path = runtime_path_override::get_runtime_path_override().await;

    let app = ManagersInner::new(invalidation_sender, runtime_path).await;

    let app = axum::Router::new()
        .nest("/", crate::api::build_axum_vanilla_router())
        .nest("/rspc", router.endpoint(move || app).axum())
        .layer(cors);

    let addr = "[::]:4000".parse::<std::net::SocketAddr>().unwrap(); // This listens on IPv6 and IPv4
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
        let resp = client.get("http://localhost:4000").send().await.unwrap();
        let resp_code = resp.status();
        let resp_body = resp.text().await.unwrap();

        assert_eq!(resp_code, 200);
        assert_eq!(resp_body, "Hello 'rspc'!");

        server.abort();
    }
}
