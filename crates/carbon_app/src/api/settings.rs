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
            let response: FESettings = app.settings_manager()
                .get_settings()
                .await?
                .try_into()?;

            Ok(response)
        }

        mutation SET_SETTINGS[app, new_settings: FESettingsUpdate] {
            app.settings_manager()
                .set_settings(new_settings)
                .await
        }
    }
}

#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FEReleaseChannel {
    Stable,
    Alpha,
    Beta,
}

impl TryFrom<String> for FEReleaseChannel {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &*value.to_lowercase() {
            "stable" => Ok(Self::Stable),
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            _ => Err(anyhow::anyhow!("Invalid release channel")),
        }
    }
}

impl From<FEReleaseChannel> for String {
    fn from(value: FEReleaseChannel) -> Self {
        match value {
            FEReleaseChannel::Stable => "stable",
            FEReleaseChannel::Alpha => "alpha",
            FEReleaseChannel::Beta => "beta",
        }
        .to_string()
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct FESettings {
    theme: String,
    language: String,
    reduced_motion: bool,
    discord_integration: bool,
    release_channel: FEReleaseChannel,
    concurrent_downloads: i32,
    show_news: bool,
    xmx: i32,
    xms: i32,
    is_first_launch: bool,
    startup_resolution: String,
    java_custom_args: String,
    auto_manage_java: bool,
    is_legal_accepted: bool,
    metrics_level: Option<i32>,
}

impl TryFrom<crate::db::app_configuration::Data> for FESettings {
    type Error = anyhow::Error;

    fn try_from(data: crate::db::app_configuration::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            theme: data.theme,
            language: data.language,
            reduced_motion: data.reduced_motion,
            discord_integration: data.discord_integration,
            release_channel: data.release_channel.try_into()?,
            concurrent_downloads: data.concurrent_downloads,
            show_news: data.show_news,
            xmx: data.xmx,
            xms: data.xms,
            is_first_launch: data.is_first_launch,
            startup_resolution: data.startup_resolution,
            java_custom_args: data.java_custom_args,
            auto_manage_java: data.auto_manage_java,
            is_legal_accepted: data.is_legal_accepted,
            metrics_level: data.metrics_level,
        })
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
    pub release_channel: Option<FEReleaseChannel>,
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
    pub is_legal_accepted: Option<bool>,
    #[specta(optional)]
    pub metrics_level: Option<i32>,
}
