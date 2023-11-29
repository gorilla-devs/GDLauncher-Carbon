use std::sync::Arc;

use display_info::DisplayInfo;
use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;
use serde_json::json;
use tracing::info;
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
    pub fn new(prisma_client: Arc<PrismaClient>, http_client: ClientWithMiddleware) -> Self {
        Self {
            client: http_client,
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
                // TODO: Keep a backlog of events if the user has not accepted the terms yet
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
            screen_resolutions: Option<Vec<String>>,
            cpus_count: u32,
            ram_mb: u64,
            os: String,
            os_version: Option<String>,
            #[serde(flatten)]
            event: Event,
        }

        let display_infos = DisplayInfo::all()
            .map(|infos| {
                infos
                    .into_iter()
                    .map(|info| format!("{}x{}", info.width, info.height))
                    .collect::<Vec<_>>()
            })
            .ok();

        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "unknown"
        };

        let os_version = self.app.system_info_manager().get_os_version().await;

        let serialized_event = json!(GDLAppEvent {
            id: metrics_user_id,
            domain: "gdl-carbon-app".to_string(),
            domain_version: env!("APP_VERSION").to_string(),
            screen_resolutions: display_infos,
            cpus_count: self.app.system_info_manager().get_cpus().await as u32,
            ram_mb: self.app.system_info_manager().get_total_ram().await / 1024 / 1024,
            os: os.to_string(),
            os_version,
            event,
        });

        info!("Sending event: {:?}", serialized_event);

        self.client
            .post(endpoint)
            .json(&serialized_event)
            .send()
            .await?;

        Ok(())
    }
}
