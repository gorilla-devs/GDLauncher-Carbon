use crate::{
    api::{
        keys::settings::{GET_SETTINGS, SET_SETTINGS},
        router::router,
    },
    managers::App,
};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_SETTINGS[app, _args: ()] {
            let response = app.settings_manager()
                    .get_settings()
                    .await?;

            TryInto::<FESettings>::try_into(response)
        }

        mutation SET_SETTINGS[app, new_settings: FESettingsUpdate] {
            app.settings_manager()
                .set_settings(new_settings)
                .await
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct FESettings {
    theme: String,
    language: String,
    reduced_motion: bool,
    discord_integration: bool,
    release_channel: String,
    concurrent_downloads: i32,
    show_news: bool,
    xmx: i32,
    xms: i32,
    is_first_launch: bool,
    startup_resolution: String,
    java_custom_args: String,
    auto_manage_java: bool,
    preferred_mod_channel: ModChannel,
    is_legal_accepted: bool,
    metrics_enabled: bool,
    random_user_uuid: String,
    metrics_enabled_last_update: Option<chrono::DateTime<chrono::FixedOffset>>,
}

// in the public interface due to `FESettings` also being in the public interface.
#[derive(Debug, Type, Serialize, Deserialize)]
#[repr(i32)]
pub enum ModChannel {
    Alpha = 0,
    Beta,
    Stable,
}

impl TryFrom<i32> for ModChannel {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Alpha),
            1 => Ok(Self::Beta),
            2 => Ok(Self::Stable),
            _ => Err(anyhow::anyhow!(
                "Invalid mod channel id {value} not in range 0..=2"
            )),
        }
    }
}

impl From<ModChannel> for crate::domain::modplatforms::ModChannel {
    fn from(value: ModChannel) -> Self {
        use crate::domain::modplatforms::ModChannel as Domain;

        match value {
            ModChannel::Alpha => Domain::Alpha,
            ModChannel::Beta => Domain::Beta,
            ModChannel::Stable => Domain::Stable,
        }
    }
}

impl Default for ModChannel {
    fn default() -> Self {
        Self::Stable
    }
}

impl TryFrom<crate::db::app_configuration::Data> for FESettings {
    type Error = anyhow::Error;

    fn try_from(data: crate::db::app_configuration::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            theme: data.theme,
            language: data.language,
            reduced_motion: data.reduced_motion,
            discord_integration: data.discord_integration,
            release_channel: data.release_channel,
            concurrent_downloads: data.concurrent_downloads,
            show_news: data.show_news,
            xmx: data.xmx,
            xms: data.xms,
            is_first_launch: data.is_first_launch,
            startup_resolution: data.startup_resolution,
            java_custom_args: data.java_custom_args,
            auto_manage_java: data.auto_manage_java,
            preferred_mod_channel: data.preferred_mod_channel.try_into()?,
            is_legal_accepted: data.is_legal_accepted,
            metrics_enabled: data.metrics_enabled,
            random_user_uuid: data.random_user_uuid,
            metrics_enabled_last_update: data.metrics_enabled_last_update,
        }
    }
}

// When updating this, make sure to also update set_settings
#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FESettingsUpdate {
    #[specta(optional)]
    pub theme: Option<String>,
    #[specta(optional)]
    pub language: Option<String>,
    #[specta(optional)]
    pub reduced_motion: Option<bool>,
    #[specta(optional)]
    pub discord_integration: Option<bool>,
    #[specta(optional)]
    pub release_channel: Option<String>,
    #[specta(optional)]
    pub concurrent_downloads: Option<i32>,
    #[specta(optional)]
    pub show_news: Option<bool>,
    #[specta(optional)]
    pub xmx: Option<i32>,
    #[specta(optional)]
    pub xms: Option<i32>,
    #[specta(optional)]
    pub is_first_launch: Option<bool>,
    #[specta(optional)]
    pub startup_resolution: Option<String>,
    #[specta(optional)]
    pub java_custom_args: Option<String>,
    #[specta(optional)]
    pub auto_manage_java: Option<bool>,
    #[specta(optional)]
    pub preferred_mod_channel: Option<ModChannel>,
    #[specta(optional)]
    pub is_legal_accepted: Option<bool>,
    #[specta(optional)]
    pub metrics_enabled: Option<bool>,
}
