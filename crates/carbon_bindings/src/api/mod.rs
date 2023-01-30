
use async_stream::stream;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use crate::app;
use crate::app::AppContainer;

mod java;
mod mc;

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
        .yolo_merge("java.", java::mount())
        .yolo_merge("mc.", mc::mount())
        .yolo_merge("app.", app::mount())
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
