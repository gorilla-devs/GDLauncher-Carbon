use std::{path::PathBuf, sync::Arc};

use async_stream::stream;
use rspc::RouterBuilderLike;
use tracing::trace;

mod app;
mod java;
mod mc;

pub type GlobalContext = Arc<tokio::sync::RwLock<GlobalContextInner>>;

#[derive(Clone)]
pub struct GlobalContextInner {
    pub base_dir: PathBuf,
}

impl GlobalContextInner {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

pub fn build_rspc_router() -> impl RouterBuilderLike<GlobalContext> {
    rspc::Router::<GlobalContext>::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .yolo_merge("java.", java::mount())
        .yolo_merge("mc.", mc::mount())
        .yolo_merge("app.", app::mount())
        .subscription("invalidateQuery", |t| {
            t(|_ctx, _args: ()| {
                stream! {
                    trace!("Client subscribed to 'pings'");
                    for i in 0..5 {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        trace!("Sending ping {}", i);
                        yield "ping".to_string();
                    }
                }
            })
        })
}

pub fn build_axum_vanilla_router() -> axum::Router<()> {
    axum::Router::new().nest("/mc", mc::mount_axum_router())
}
