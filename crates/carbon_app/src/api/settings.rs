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
                    .await.unwrap();

            Ok(Into::<FESettings>::into(response))
        }

        mutation SET_SETTINGS[app, new_settings: FESettingsUpdate] {
            app.settings_manager()
                .set_settings(new_settings)
                .await
        }
    }
}

#[derive(Type, Serialize)]
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
}

impl From<crate::db::app_configuration::Data> for FESettings {
    fn from(data: crate::db::app_configuration::Data) -> Self {
        Self {
            theme: data.theme,
            language: data.language,
            reduced_motion: data.reduced_motion,
            discord_integration: data.discord_integration,
            release_channel: data.release_channel,
            concurrent_downloads: data.concurrent_downloads,
            show_news: data.show_news,
            xmx: data.xmx,
            xms: data.xms,
        }
    }
}

#[derive(Type, Deserialize)]
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
    pub startup_resolution: Option<String>,
    #[specta(optional)]
    pub java_custom_args: Option<String>,
}
