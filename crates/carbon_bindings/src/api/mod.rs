use std::{path::PathBuf, sync::Arc};

use async_stream::stream;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

mod app;
mod java;
mod mc;

pub type GlobalContext = Arc<tokio::sync::RwLock<GlobalContextInner>>;

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

#[derive(Clone)]
pub struct GlobalContextInner {
    pub base_dir: PathBuf,
    // Not sure how to hide this..
    invalidation_sender: tokio::sync::broadcast::Sender<InvalidationEvent>,
    // instances: Vec<Instance>,
    // javas: Vec<Javas>
}

impl GlobalContextInner {
    pub fn new(
        base_dir: PathBuf,
        invalidation_sender: tokio::sync::broadcast::Sender<InvalidationEvent>,
    ) -> Self {
        Self {
            base_dir,
            invalidation_sender,
        }
    }

    pub fn invalidate(&self, key: impl Into<String>, args: Option<serde_json::Value>) {
        match self
            .invalidation_sender
            .send(InvalidationEvent::new(key, args))
        {
            Ok(_) => (),
            Err(e) => {
                println!("Error sending invalidation request: {}", e);
            }
        }
    }
}

pub fn build_rspc_router() -> impl RouterBuilderLike<GlobalContext> {
    rspc::Router::<GlobalContext>::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .yolo_merge("java.", java::mount())
        .yolo_merge("mc.", mc::mount())
        .yolo_merge("app.", app::mount())
        .subscription("invalidateQuery", move |t| {
            // https://twitter.com/ep0k_/status/494284207821447168
            t(move |ctx, _args: ()| {
                stream! {
                    loop {
                        match ctx.read().await.invalidation_sender.subscribe().recv().await {
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
