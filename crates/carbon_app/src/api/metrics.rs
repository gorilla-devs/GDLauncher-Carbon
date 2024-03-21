use crate::api::router::router;
use crate::api::{keys::metrics::SEND_EVENT, managers::App};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;

pub(super) fn mount() -> RouterBuilder<App> {
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
    FeaturedModpackInstalled {
        campaign_id: String,
        item_id: String,
    },
}

impl From<FEMetricsEvent> for crate::domain::metrics::Event {
    fn from(event: FEMetricsEvent) -> Self {
        match event {
            FEMetricsEvent::PageView(page_url) => Self::PageView { page_url },
            FEMetricsEvent::FeaturedModpackInstalled {
                campaign_id,
                item_id,
            } => Self::FeaturedModpackInstalled {
                campaign_id,
                item_id,
            },
        }
    }
}
