use self::terms_and_privacy::TermsAndPrivacy;

use super::ManagerRef;
use crate::{
    api::{keys::settings::*, settings::FESettingsUpdate},
    db::app_configuration,
    domain::runtime_path,
};
use anyhow::anyhow;
use chrono::Utc;
use std::path::PathBuf;

mod terms_and_privacy;

pub(crate) struct SettingsManager {
    pub runtime_path: runtime_path::RuntimePath,
    pub terms_and_privacy: TermsAndPrivacy,
}

impl SettingsManager {
    pub fn new(runtime_path: PathBuf) -> Self {
        Self {
            runtime_path: runtime_path::RuntimePath::new(runtime_path),
            terms_and_privacy: TermsAndPrivacy::new(),
        }
    }
}

impl ManagerRef<'_, SettingsManager> {
    pub async fn get_settings(self) -> anyhow::Result<crate::db::app_configuration::Data> {
        self.app
            .prisma_client
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(anyhow!("Can't find this key"))
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_settings(self, incoming_settings: FESettingsUpdate) -> anyhow::Result<()> {
        let db = &self.app.prisma_client;

        let crate::db::app_configuration::Data {
            secret,
            random_user_uuid,
            ..
        } = self.get_settings().await?;

        let random_user_uuid = uuid::Uuid::parse_str(&random_user_uuid)?;

        let mut queries = vec![];

        let mut something_changed = false;
        if let Some(theme) = incoming_settings.theme {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::theme::set(theme)],
            ));
            something_changed = true;
        }

        if let Some(language) = incoming_settings.language {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::language::set(language)],
            ));
            something_changed = true;
        }

        if let Some(reduced_motion) = incoming_settings.reduced_motion {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::reduced_motion::set(reduced_motion)],
            ));
            something_changed = true;
        }

        if let Some(discord_integration) = incoming_settings.discord_integration {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::discord_integration::set(
                    discord_integration,
                )],
            ));
            something_changed = true;
        }

        if let Some(release_channel) = incoming_settings.release_channel {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::release_channel::set(release_channel)],
            ));
            something_changed = true;
        }

        if let Some(concurrent_downloads) = incoming_settings.concurrent_downloads {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::concurrent_downloads::set(
                    concurrent_downloads,
                )],
            ));
            something_changed = true;
        }

        if let Some(show_news) = incoming_settings.show_news {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::show_news::set(show_news)],
            ));
            something_changed = true;
        }

        if let Some(xmx) = incoming_settings.xmx {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::xmx::set(xmx)],
            ));
            something_changed = true;
        }

        if let Some(xms) = incoming_settings.xms {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::xms::set(xms)],
            ));
            something_changed = true;
        }

        if let Some(is_first_launch) = incoming_settings.is_first_launch {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::is_first_launch::set(is_first_launch)],
            ));
            something_changed = true;
        }

        if let Some(startup_resolution) = incoming_settings.startup_resolution {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::startup_resolution::set(
                    startup_resolution,
                )],
            ));
            something_changed = true;
        }

        if let Some(java_custom_args) = incoming_settings.java_custom_args {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::java_custom_args::set(java_custom_args)],
            ));
            something_changed = true;
        }

        if let Some(auto_manage_java) = incoming_settings.auto_manage_java {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::auto_manage_java::set(auto_manage_java)],
            ));
            something_changed = true;
        }

        if let Some(preferred_mod_channel) = incoming_settings.preferred_mod_channel {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::preferred_mod_channel::set(
                    preferred_mod_channel as i32,
                )],
            ));
        }

        if let Some(terms_and_privacy_accepted) = incoming_settings.terms_and_privacy_accepted {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![app_configuration::terms_and_privacy_accepted::set(
                    terms_and_privacy_accepted,
                )],
            ));

            self.terms_and_privacy
                .record_consent(
                    terms_and_privacy::ConsentType::TermsAndPrivacy,
                    terms_and_privacy_accepted,
                    &random_user_uuid,
                    &secret,
                )
                .await?;

            something_changed = true;
        }

        if let Some(metrics_enabled) = incoming_settings.metrics_enabled {
            queries.push(self.app.prisma_client.app_configuration().update(
                app_configuration::id::equals(0),
                vec![
                    app_configuration::metrics_enabled::set(metrics_enabled),
                    app_configuration::metrics_enabled_last_update::set(Some(Utc::now().into())),
                ],
            ));

            self.terms_and_privacy
                .record_consent(
                    terms_and_privacy::ConsentType::Metrics,
                    metrics_enabled,
                    &random_user_uuid,
                    &secret,
                )
                .await?;

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
            .update(app_configuration::id::equals(0), vec![value])
            .exec()
            .await?;

        Ok(())
    }
}
