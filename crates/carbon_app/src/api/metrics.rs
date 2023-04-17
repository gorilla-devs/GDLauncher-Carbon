use crate::api::keys::metrics::SEND_PAGEVIEW;
use crate::api::router::router;
use crate::api::{keys::metrics::SEND_EVENT, managers::App};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        mutation SEND_EVENT[app, event: Event] {
            app.metrics_manager().track_event(event.into()).await;
            Ok(())
         }
        mutation SEND_PAGEVIEW[app, pageview: Pageview] {
            app.metrics_manager().track_pageview(pageview.into()).await;
            Ok(())
         }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub properties: HashMap<String, String>,
}

impl From<Event> for crate::domain::metrics::Event {
    fn from(event: Event) -> Self {
        Self {
            name: event.name,
            properties: event.properties,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct Pageview {
    pub path: String,
}

impl From<Pageview> for crate::domain::metrics::Pageview {
    fn from(pageview: Pageview) -> Self {
        Self {
            path: pageview.path,
        }
    }
}
