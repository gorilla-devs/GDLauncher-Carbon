use crate::{domain::metrics::GDLMetricsEvent, iridium_client::get_client};
use carbon_repos::db::{app_configuration, PrismaClient};
use display_info::DisplayInfo;
use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use super::ManagerRef;

pub(crate) struct MetricsManager {
    client: ClientWithMiddleware,
    prisma_client: Arc<PrismaClient>,
    gdl_base_api: String,
    random_session_uuid: Uuid,
}

impl MetricsManager {
    pub fn new(
        prisma_client: Arc<PrismaClient>,
        http_client: ClientWithMiddleware,
        gdl_base_api: String,
    ) -> Self {
        let random_session_uuid = Uuid::new_v4();

        Self {
            client: http_client,
            prisma_client,
            gdl_base_api,
            random_session_uuid,
        }
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_event(&self, event: GDLMetricsEvent) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/event", self.gdl_base_api);

        let Some(metrics_user_id) = self
            .prisma_client
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .and_then(|data| {
                // TODO: Keep a backlog of events if the user has not accepted the terms yet
                if !data.terms_and_privacy_accepted {
                    None
                } else {
                    Some(self.random_session_uuid.to_string())
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
            event: GDLMetricsEvent,
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
            .body(reqwest::Body::from(serialized_event.to_string()))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        Ok(())
    }
}
