use reqwest_middleware::ClientWithMiddleware;

use crate::{
    domain::metrics::{Event, Pageview},
    iridium_client::get_client,
};

use super::{ManagerRef, GDL_API_BASE};

pub(crate) struct MetricsManager;

impl MetricsManager {
    pub fn new() -> Self {
        Self
    }

    #[tracing::instrument(skip(self, client))]
    pub async fn track_pageview(
        &self,
        client: ClientWithMiddleware,
        page: Pageview,
    ) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/pageview", GDL_API_BASE);
        client.post(endpoint).json(&page).send().await?;

        Ok(())
    }

    #[tracing::instrument(skip(self, client))]
    pub async fn track_event(
        &self,
        client: ClientWithMiddleware,
        event: Event,
    ) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/event", GDL_API_BASE);
        client.post(endpoint).json(&event).send().await?;

        Ok(())
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_pageview(&self, page: Pageview) -> anyhow::Result<()> {
        let client = get_client();
        self.manager.track_pageview(client, page).await
    }

    pub async fn track_event(&self, event: Event) -> anyhow::Result<()> {
        let client = get_client();
        self.manager.track_event(client, event).await
    }
}
