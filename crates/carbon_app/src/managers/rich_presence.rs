use super::ManagerRef;
use std::{
    sync::Arc,
    time::{self, UNIX_EPOCH},
};
use tokio::sync::Mutex;

pub(crate) struct RichPresenceManager {
    drpc: Arc<Mutex<discord_presence::Client>>,
}

impl RichPresenceManager {
    pub fn new() -> Self {
        let drpc = Arc::new(Mutex::new(discord_presence::Client::new(
            555898932467597312,
        )));
        Self { drpc }
    }
}

impl ManagerRef<'_, RichPresenceManager> {
    pub async fn start_presence(&self) -> anyhow::Result<()> {
        let mut drpc = self.manager.drpc.lock().await;
        let _ = drpc.start();

        Ok(())
    }

    pub async fn update_activity(&self, state: String) -> anyhow::Result<()> {
        if self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .discord_integration
        {
            let drpc = self.manager.drpc.clone();
            let mut drpc = drpc.lock().await;
            drpc.set_activity(|act| {
                act.state(state).timestamps(|ts| {
                    ts.start(
                        time::SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    )
                })
            })?;
        }

        Ok(())
    }

    pub async fn stop_activity(&self) -> anyhow::Result<()> {
        if self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .discord_integration
        {
            let drpc = self.manager.drpc.clone();
            let mut drpc = drpc.lock().await;
            drpc.clear_activity()?;
        }

        Ok(())
    }
}
