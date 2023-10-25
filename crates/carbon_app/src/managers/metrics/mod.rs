use std::sync::Arc;

use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    db::{app_configuration, PrismaClient},
    domain::metrics::Event,
    iridium_client::get_client,
};

use super::{ManagerRef, GDL_API_BASE};

pub(crate) struct MetricsManager {
    client: ClientWithMiddleware,
    prisma_client: Arc<PrismaClient>,
}

impl MetricsManager {
    pub fn new(prisma_client: Arc<PrismaClient>) -> Self {
        Self {
            client: get_client().build(),
            prisma_client,
        }
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_event(&self, event: Event) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/event", GDL_API_BASE);

        let Some(metrics_user_id) = self
            .prisma_client
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .and_then(|data| {
                if !data.terms_and_privacy_accepted || !data.metrics_enabled {
                    None
                } else {
                    Some(data.random_user_uuid)
                }
            })
        else {
            return Ok(());
        };

        #[derive(Serialize)]
        struct GDLAppEvent {
            id: String,
            domain: String,
            domain_version: String,
            #[serde(flatten)]
            event: Event,
        }

        self.client
            .post(endpoint)
            .json(&GDLAppEvent {
                id: metrics_user_id,
                domain_version: env!("APP_VERSION").to_string(),
                domain: "gdl-carbon-app".to_string(),
                event,
            })
            .send()
            .await?;

        Ok(())
    }
}
