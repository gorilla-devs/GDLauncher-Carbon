use super::ManagerRef;
use crate::{
    api::{keys::settings::*, settings::FESettingsUpdate},
    db::app_configuration,
};
use anyhow::anyhow;
use std::path::PathBuf;

pub mod runtime_path;

pub(crate) struct SettingsManager {
    pub runtime_path: runtime_path::RuntimePath,
}

impl SettingsManager {
    pub fn new(runtime_path: PathBuf) -> Self {
        Self {
            runtime_path: runtime_path::RuntimePath::new(runtime_path),
        }
    }
}

impl ManagerRef<'_, SettingsManager> {
    pub async fn get_settings(self) -> anyhow::Result<crate::db::app_configuration::Data> {
        self.get().await
    }

    pub async fn set_settings(self, incoming_settings: FESettingsUpdate) -> anyhow::Result<()> {
        let db = &self.app.prisma_client;
        let mut queries = vec![];

        let mut something_changed = false;
        if let Some(theme) = incoming_settings.theme {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetTheme(theme)],
            ));
            something_changed = true;
        }

        if let Some(language) = incoming_settings.language {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetLanguage(language)],
            ));
            something_changed = true;
        }

        if let Some(reduced_motion) = incoming_settings.reduced_motion {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetReducedMotion(
                    reduced_motion,
                )],
            ));
            something_changed = true;
        }

        if let Some(discord_ingration) = incoming_settings.discord_integration {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetDiscordIntegration(
                    discord_ingration,
                )],
            ));
            something_changed = true;
        }

        if let Some(release_channel) = incoming_settings.release_channel {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetReleaseChannel(
                    release_channel,
                )],
            ));
            something_changed = true;
        }

        if let Some(concurrent_downloads) = incoming_settings.concurrent_downloads {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetConcurrentDownloads(
                    concurrent_downloads,
                )],
            ));
            something_changed = true;
        }

        if let Some(show_news) = incoming_settings.show_news {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetShowNews(show_news)],
            ));
            something_changed = true;
        }

        if let Some(xmx) = incoming_settings.xmx {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetXmx(xmx)],
            ));
            something_changed = true;
        }

        if let Some(xms) = incoming_settings.xms {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![app_configuration::SetParam::SetXms(xms)],
            ));
            something_changed = true;
        }

        if something_changed {
            db._batch(queries).await?;
            self.app.invalidate(GET_SETTINGS, None);
        }

        Ok(())
    }

    pub async fn set(self, value: app_configuration::SetParam) -> anyhow::Result<()> {
        self.app
            .prisma_client
            .app_configuration()
            .update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![value],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn get(self) -> anyhow::Result<app_configuration::Data> {
        self.app
            .prisma_client
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(anyhow!("Can't find this key"))
    }
}
