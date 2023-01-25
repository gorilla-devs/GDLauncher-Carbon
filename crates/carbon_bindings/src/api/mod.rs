use std::path::PathBuf;

use async_stream::stream;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use crate::api::app::AppContainer;

pub mod app;
mod java;
mod mc;
mod configuration;
mod persistence;

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
    pub app: AppContainer,// todo : make app our GlobalContext(lot more organic) ?
    // Not sure how to hide this..
    invalidation_sender: tokio::sync::broadcast::Sender<InvalidationEvent>,
    // instances: Vec<Instance>,
    // javas: Vec<Javas>
}

impl GlobalContextInner {
    pub fn new(
        app: AppContainer,
        base_dir: PathBuf,
        invalidation_sender: tokio::sync::broadcast::Sender<InvalidationEvent>,
    ) -> Self {
        Self {
            base_dir,
            app,
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
