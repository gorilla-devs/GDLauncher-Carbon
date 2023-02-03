use crate::app::{App, AppContainer};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub mod api;
pub mod app;
pub(crate) mod db;
pub mod generate_rspc_ts_bindings;
mod runtime_directory;

#[macro_export]
macro_rules! try_path_fmt {
    ($path:expr) => {{
        $path
            .as_os_str()
            .to_str()
            .unwrap_or("<<unrepresentable fs path!>>")
    }};
}

// Since it's module_init, make sure it's not running during tests
#[cfg(not(test))]
pub fn init() {
    use runtime_directory::set_runtime_directory_override;
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new();
        runtime
            .unwrap() /* This should never fail */
            .block_on(async {
                set_runtime_directory_override().await;
                start_router().await;
            })
    });
}

async fn start_router() {
    let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);

    let router: Arc<rspc::Router<AppContainer>> = api::build_rspc_router().expose().build().arced();

    // We disable CORS because this is just an example. DON'T DO THIS IN PRODUCTION!
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = App::new_with_invalidation_channel(invalidation_sender).await;

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
mod test {}

#[cfg(test)]
mod e2e {
    use crate::db;
    use crate::db::app_configuration::SetParam::SetId;
    use crate::db::app_configuration::{UniqueWhereParam, WhereParam};
    use crate::db::read_filters::IntFilter;
    use log::trace;

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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn persistence_ok() {
        trace!("trying to connect to db ");
        let client = db::new_client()
            .await
            .expect("unable to build app_configuration client using db_url ");
        trace!("connected to db");

        let configuration = client
            .app_configuration()
            .upsert(
                UniqueWhereParam::IdEquals(101),
                vec![SetId(101)],
                vec![SetId(101)],
            )
            .exec()
            .await
            .expect("unable to exec create query for app_configuration");

        trace!("wrote correctly in db : {:#?}", configuration);

        let _serialized_configuration = serde_json::to_string_pretty(&configuration)
            .expect("unable to serialize app_configuration");

        let _count = client
            .app_configuration()
            .count(vec![WhereParam::Id(IntFilter::Equals(101))])
            .exec()
            .await
            .expect("unable to select app_configuration");

        trace!("read correctly from db ");
    }
}
