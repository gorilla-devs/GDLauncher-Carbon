use crate::api::keys::metrics::SEND_PAGEVIEW;
use crate::api::router::router;
use crate::api::{keys::metrics::SEND_EVENT, managers::App};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        mutation SEND_EVENT[app, event: FEEvent] {
            app.metrics_manager().track_event(event.into()).await?;
            Ok(())
         }
        mutation SEND_PAGEVIEW[app, pageview: FEPageview] {
            app.metrics_manager().track_pageview(pageview.into()).await?;
            Ok(())
         }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub enum FEEventName {
    AppClosed,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct FEEvent {
    pub name: FEEventName,
    pub properties: HashMap<String, String>,
}

impl From<FEEvent> for crate::domain::metrics::Event {
    fn from(event: FEEvent) -> Self {
        Self {
            name: match event.name {
                FEEventName::AppClosed => {
                    crate::domain::metrics::EventName::AppClosed
                }
            },
            properties: event.properties,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct FEPageview {
    pub path: String,
}

impl From<FEPageview> for crate::domain::metrics::Pageview {
    fn from(pageview: FEPageview) -> Self {
        Self {
            path: pageview.path,
        }
    }
}
