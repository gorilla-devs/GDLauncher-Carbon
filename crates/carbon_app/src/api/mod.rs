use std::sync::Arc;

use crate::managers;
use crate::managers::{App, AppInner};
use async_stream::stream;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

mod account;
mod instance;
mod java;
pub mod keys;
mod mc;
mod metrics;
mod modplatforms;
pub mod router;
pub mod settings;
pub mod translation;
mod vtask;

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

pub fn build_rspc_router() -> impl RouterBuilderLike<App> {
    rspc::Router::<App>::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .yolo_merge(keys::account::GROUP_PREFIX, account::mount())
        .yolo_merge(keys::java::GROUP_PREFIX, java::mount())
        .yolo_merge(keys::mc::GROUP_PREFIX, mc::mount())
        .yolo_merge(keys::vtask::GROUP_PREFIX, vtask::mount())
        .yolo_merge(keys::instance::GROUP_PREFIX, instance::mount())
        .yolo_merge(keys::modplatforms::GROUP_PREFIX, modplatforms::mount())
        .yolo_merge(keys::settings::GROUP_PREFIX, settings::mount())
        .yolo_merge(keys::metrics::GROUP_PREFIX, metrics::mount())
        .subscription("invalidateQuery", move |t| {
            // https://twitter.com/ep0k_/status/494284207821447168
            // XD
            t(move |app, _args: ()| {
                stream! {
                    let mut invalidation_receiver = app.invalidation_channel.subscribe();

                    loop {
                        match invalidation_receiver.recv().await {
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

pub fn build_axum_vanilla_router() -> axum::Router<Arc<AppInner>> {
    axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello 'rspc'!" }))
        .route("/health", axum::routing::get(|| async { "OK" }))
        .nest("/account", account::mount_axum_router())
        .nest("/instance", instance::mount_axum_router())
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    #[allow(clippy::assertions_on_constants)]
    fn verify_iridium_feature() {
        #[cfg(feature = "production")]
        {
            assert!(true);
        }
        #[cfg(not(feature = "production"))]
        {
            assert!(false);
        }
    }
}
