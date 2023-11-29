use crate::api::router::router;
use crate::api::{keys::metrics::SEND_EVENT, managers::App};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        mutation SEND_EVENT[app, event: FEMetricsEvent] {
            tokio::spawn(async move {
                let _ = app.metrics_manager().track_event(event.into()).await;
            });
            Ok(())
         }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(tag = "event_name", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum FEMetricsEvent {
    PageView(String),
}

impl From<FEMetricsEvent> for crate::domain::metrics::Event {
    fn from(event: FEMetricsEvent) -> Self {
        match event {
            FEMetricsEvent::PageView(page_url) => Self::PageView { page_url },
        }
    }
}
