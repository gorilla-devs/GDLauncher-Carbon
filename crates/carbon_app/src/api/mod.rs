use std::pin::Pin;
use std::sync::Arc;

use crate::managers::{App, AppInner};
use crate::{app_version, managers};
use async_stream::stream;
use futures::Stream;
use pin_project::{pin_project, pinned_drop};
use rspc::Type;
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
mod system_info;
pub mod translation;
mod vtask;

#[derive(Clone, Serialize, Deserialize, Type)]
pub struct InvalidationEvent {
    pub key: &'static str,
    pub args: Option<serde_json::Value>,
}

impl InvalidationEvent {
    pub fn new(key: &'static str, args: Option<serde_json::Value>) -> Self {
        Self { key, args }
    }
}

pub fn build_rspc_router() -> rspc::RouterBuilder<App> {
    rspc::Router::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .query("getAppVersion", |t| {
            t(|_ctx, _: ()| async move { Ok(app_version::APP_VERSION) })
        })
        .merge(keys::account::GROUP_PREFIX, account::mount())
        .merge(keys::java::GROUP_PREFIX, java::mount())
        .merge(keys::mc::GROUP_PREFIX, mc::mount())
        .merge(keys::vtask::GROUP_PREFIX, vtask::mount())
        .merge(keys::instance::GROUP_PREFIX, instance::mount())
        .merge(keys::modplatforms::GROUP_PREFIX, modplatforms::mount())
        .merge(keys::settings::GROUP_PREFIX, settings::mount())
        .merge(keys::metrics::GROUP_PREFIX, metrics::mount())
        .merge(keys::systeminfo::GROUP_PREFIX, system_info::mount())
        .subscription("invalidateQuery", move |t| {
            // https://twitter.com/ep0k_/status/494284207821447168
            // XD
            t(move |app, _args: ()| {
                let s = stream! {
                    let mut channel = app.invalidation_channel.subscribe();
                    println!("Invalidation channel connected");

                    loop {
                        match channel.recv().await {
                            Ok(event) => {
                                yield event;
                            }
                            Err(e) => {
                              println!("Error receiving invalidation request: {}", e);
                            }
                        }
                    }
                };

                #[pin_project(PinnedDrop)]
                struct Dropcheck<T: Stream<Item = InvalidationEvent>>(#[pin] T);

                impl<T: Stream<Item = InvalidationEvent>> Stream for Dropcheck<T> {
                    type Item = InvalidationEvent;

                    fn poll_next(
                        self: Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Option<Self::Item>> {
                        self.project().0.poll_next(cx)
                    }
                }

                #[pinned_drop]
                impl<T: Stream<Item = InvalidationEvent>> PinnedDrop for Dropcheck<T> {
                    fn drop(self: Pin<&mut Self>) {
                        println!("Invalidation stream was dropped!");
                        std::process::exit(1);
                    }
                }

                Dropcheck(s)
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
