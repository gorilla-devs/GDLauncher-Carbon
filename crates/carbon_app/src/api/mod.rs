use crate::managers;
use crate::managers::Managers;
use async_stream::stream;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

mod account;
mod configuration;
mod java;
pub mod keys;
mod mc;
pub mod router;

#[derive(Clone, Serialize, Deserialize, Type)]
pub struct InvalidationEvent {
    key: &'static str,
    args: Option<serde_json::Value>,
}

impl InvalidationEvent {
    pub fn new(key: &'static str, args: Option<serde_json::Value>) -> Self {
        Self { key, args }
    }
}

pub fn build_rspc_router() -> impl RouterBuilderLike<Managers> {
    rspc::Router::<Managers>::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .yolo_merge(keys::account::GROUP_PREFIX, account::mount())
        .yolo_merge(keys::java::GROUP_PREFIX, java::mount())
        .yolo_merge(keys::mc::GROUP_PREFIX, mc::mount())
        .yolo_merge(keys::app::GROUP_PREFIX, configuration::mount())
        .subscription("invalidateQuery", move |t| {
            // https://twitter.com/ep0k_/status/494284207821447168
            // XD
            t(move |app, _args: ()| {
                stream! {
                    loop {
                        match app.wait_for_invalidation().await {
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
