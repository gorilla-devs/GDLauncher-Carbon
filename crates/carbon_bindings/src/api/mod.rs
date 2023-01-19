use std::path::PathBuf;

use async_stream::stream;
use rspc::{internal::BaseMiddleware, RouterBuilderLike, Type};
use serde::Serialize;
use tracing::trace;

mod app;
mod java;
mod mc;

pub struct Ctx {}

pub fn build_rspc_router() -> impl RouterBuilderLike<()> {
    let router = rspc::Router::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .yolo_merge("java.", java::mount())
        .yolo_merge("mc.", mc::mount())
        .yolo_merge("app.", app::mount())
        .subscription("invalidateQuery", |t| {
            t(|_ctx, _args: ()| {
                stream! {
                    // trace!("Client subscribed to 'pings'");
                    // for i in 0..5 {
                    //     trace!("Sending ping {}", i);
                    //     yield "ping".to_string();
                    //     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    // }
                }
            })
        });

    router
}

pub fn build_axum_vanilla_router() -> axum::Router<()> {
    axum::Router::new().nest("mc.", mc::mount_axum_router())
}
