use reqwest_middleware::ClientWithMiddleware;

use crate::{
    domain::metrics::{Event, Pageview},
    iridium_client::get_client,
};

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

    #[tracing::instrument(skip(self))]
    pub async fn track_pageview(&self, page: Pageview) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/pageview", GDL_API_BASE);
        self.client.post(endpoint).json(&page).send().await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn track_event(&self, event: Event) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/event", GDL_API_BASE);
        self.client.post(endpoint).json(&event).send().await?;

        Ok(())
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_pageview(&self, page: Pageview) -> anyhow::Result<()> {
        self.manager.track_pageview(page).await
    }

    pub async fn track_event(&self, event: Event) -> anyhow::Result<()> {
        self.manager.track_event(event).await
    }
}
