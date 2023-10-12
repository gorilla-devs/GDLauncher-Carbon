use super::ManagerRef;

pub(crate) struct RichPresenceManager {}

impl RichPresenceManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, RichPresenceManager> {
    pub async fn start_presence(&self) -> anyhow::Result<()> {
        if self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .discord_integration
        {
            println!("_DRPC_:INIT");
        }

        Ok(())
    }

    pub async fn stop_presence(&self) -> anyhow::Result<()> {
        if self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .discord_integration
        {
            println!("_DRPC_:SHUTDOWN");
        }

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
            println!("_DRPC_:UPDATE_ACTIVITY|{state}");
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
            println!("_DRPC_:STOP_ACTIVITY");
        }

        Ok(())
    }
}
