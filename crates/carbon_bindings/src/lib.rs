use std::path::PathBuf;

use async_stream::stream;
use axum::{extract::Path, routing::get};
use rspc::Config;
use tower_http::cors::{Any, CorsLayer};
use tracing::trace;

pub struct Ctx {}

pub fn build_router() -> rspc::RouterBuilder<Ctx> {
    rspc::Router::<Ctx>::new()
        .query("version", |t| t(|_, _: ()| env!("CARGO_PKG_VERSION")))
        .query("echo", |t| t(|_, v: String| v))
        .query("error", |t| {
            t(|_, _: ()| {
                Err(rspc::Error::new(
                    rspc::ErrorCode::InternalServerError,
                    "Something went wrong".into(),
                )) as Result<String, rspc::Error>
            })
        })
        .query("transformMe", |t| t(|_, _: ()| "Hello, world!".to_string()))
        .mutation("sendMsg", |t| {
            t(|_, v: String| {
                trace!("Client said '{}'", v);
                v
            })
        })
        .subscription("pings", |t| {
            t(|_ctx, _args: ()| {
                stream! {
                    trace!("Client subscribed to 'pings'");
                    for i in 0..5 {
                        trace!("Sending ping {}", i);
                        yield "ping".to_string();
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            })
        })
    // TODO: Results being returned from subscriptions
    // .subscription("errorPings", |t| t(|_ctx, _args: ()| {
    //     stream! {
    //         for i in 0..5 {
    //             yield Ok("ping".to_string());
    //             sleep(Duration::from_secs(1)).await;
    //         }
    //         yield Err(rspc::Error::new(ErrorCode::InternalServerError, "Something went wrong".into()));
    //     }
    // }))
}
