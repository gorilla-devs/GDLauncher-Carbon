use crate::domain::metrics::GDLMetricsEvent;
use carbon_repos::{DbPool, models::AppConfiguration, queries};
use display_info::DisplayInfo;
use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use super::ManagerRef;

pub(crate) struct MetricsManager {
    client: ClientWithMiddleware,
    db_pool: DbPool,
    gdl_base_api: String,
    random_session_uuid: Uuid,
}

impl MetricsManager {
    pub fn new(db_pool: DbPool, http_client: ClientWithMiddleware, gdl_base_api: String) -> Self {
        let random_session_uuid = Uuid::new_v4();

        Self {
            client: http_client,
            db_pool,
            gdl_base_api,
            random_session_uuid,
        }
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_event(&self, event: GDLMetricsEvent) -> anyhow::Result<()> {
        let endpoint = format!("{}/v1/metrics/event", self.gdl_base_api);

        // Query app configuration to check if terms are accepted
        let pool = self.db_pool.clone();
        let random_session_uuid = self.random_session_uuid;
        let terms_accepted = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let result = conn.query_row(queries::settings::GetSettings::SQL, [], |row| {
                let config = AppConfiguration::from_row(row)?;
                Ok(config.terms_and_privacy_accepted)
            });

            match result {
                Ok(accepted) => Ok(accepted),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(e) => Err(anyhow::Error::from(e)),
            }
        })
        .await??;

        // TODO: Keep a backlog of events if the user has not accepted the terms yet
        if !terms_accepted {
            return Ok(());
        }
        let metrics_user_id = random_session_uuid.to_string();

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
