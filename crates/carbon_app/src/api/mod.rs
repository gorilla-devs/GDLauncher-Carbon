use crate::app;
use crate::app::AppContainer;
use async_stream::stream;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

mod configuration;
mod dev;
mod java;
mod mc;

#[macro_export]
macro_rules! try_in_router {
    ($result:expr) => {
        $result.map_err(|error| {
            rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
        })
    };
}

#[macro_export]
macro_rules! into_router_mutation_response {
    ($app:expr, $key:expr, $representation_type:ty, $result:block) => {
        let app = $app.read().await;
        let value = $result
            .map_err(|error| {
                rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
            })
            .map(<$representation_type>::from)?;
        let value = serde_json::to_value(value).map_err(|error| {
            rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
        })?;
        app.invalidate($key, Some(value.into()));
        Ok(())
    };
}
#[macro_export]
macro_rules! into_router_mutation_response_ok {
    ($app:expr, $key:expr, $representation_type:ty, $value:expr) => {
        $app.invalidate($key, Some(<$representation_type>::from($value).into()));
        Ok(())
    };
}

#[macro_export]
macro_rules! into_router_mutation_responses {
    ($app:expr, $key:expr, $representation_type:ty, $result:block) => {
        let app = $app.read().await;
        let value = $result
            .map_err(|error| {
                rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
            })
            .map(|iter| iter.iter().map(<$representation_type>::from)::collect::<Vec<_>>())?;
        let value = serde_json::to_value(value).map_err(|error| {
            rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
        })?;
        app.invalidate($key, Some(value.into()));
        Ok(())
    };
}

#[macro_export]
macro_rules! into_router_query_response {
    ($result:expr,$representation_type:ty) => {
        $result
            .map_err(|error| {
                rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
            })
            .map(<$representation_type>::from)
    };
}

#[macro_export]
macro_rules! into_router_query_response_ok {
    ($result:expr,$representation_type:ty) => {
        Result::Ok(<$representation_type>::from($result))
    };
}

#[macro_export]
macro_rules! into_router_query_responses {
    ($result:expr,$representation_type:ty) => {{
        Ok($result
            .map_err(|error| {
                rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
            })
            .map(|iter|iter.iter().map())
            $result.iter().map(<$representation_type>::from)
        )
    }};
}

#[macro_export]
macro_rules! into_router_query_responses_ok {
    ($result:expr,$representation_type:ty) => {{
        Ok($result
            .iter()
            .map(<$representation_type>::from)
            .collect::<Vec<_>>())
    }};
}

#[derive(Clone, Serialize, Deserialize, Type)]
pub struct InvalidationEvent {
    key: String,
    args: Option<serde_json::Value>,
}

impl InvalidationEvent {
    pub fn new(key: impl Into<String>, args: Option<serde_json::Value>) -> Self {
        Self {
            key: key.into(),
            args,
        }
    }
}

pub fn build_rspc_router() -> impl RouterBuilderLike<AppContainer> {
    rspc::Router::<AppContainer>::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .yolo_merge("dev.", dev::mount())
        .yolo_merge("java.", java::mount())
        .yolo_merge("mc.", mc::mount())
        .subscription("invalidateQuery", move |t| {
            // https://twitter.com/ep0k_/status/494284207821447168
            // XD
            t(move |app_container, _args: ()| {
                stream! {
                    loop {
                        match app_container.read().await.wait_for_invalidation().await {
                            Ok(event) => {
                                yield event;
                            }
                            Err(e) => {
                                println!("Error receiving invalidation request: {}", e);
                            }
                        }
                    }
                }
            })
        })
}

pub fn build_axum_vanilla_router() -> axum::Router<()> {
    axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello 'rspc'!" }))
        .nest("/mc", mc::mount_axum_router())
}
