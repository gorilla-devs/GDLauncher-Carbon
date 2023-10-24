use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;

use crate::{domain::metrics::Event, iridium_client::get_client};

use super::{ManagerRef, GDL_API_BASE};

pub(crate) struct MetricsManager {
    client: ClientWithMiddleware,
}

impl MetricsManager {
    pub fn new() -> Self {
        Self {
            client: get_client().build(),
        }
    }

    pub async fn track_event(&self, event: Event) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/event", GDL_API_BASE);
        self.client.post(endpoint).json(&event).send().await?;

        Ok(())
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_event(&self, event: Event) -> anyhow::Result<()> {
        self.manager.track_event(event).await
    }
}
