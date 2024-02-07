use std::str::FromStr;

use crate::{
    api::{
        keys::settings::{
            GET_PRIVACY_STATEMENT_BODY, GET_SETTINGS, GET_TERMS_OF_SERVICE_BODY, SET_SETTINGS,
        },
        router::router,
    },
    managers::App,
};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

use super::{
    modplatforms::{ModChannelWithUsage, ModPlatform, ModSources},
    Set,
};

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

        query GET_TERMS_OF_SERVICE_BODY[app, _args: ()] {
            app.settings_manager()
                .terms_and_privacy
                .fetch_terms_of_service_body()
                .await
        }

        query GET_PRIVACY_STATEMENT_BODY[app, _args: ()] {
            app.settings_manager()
                .terms_and_privacy
                .fetch_privacy_statement_body()
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

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum GameResolution {
    Standard(u16, u16),
    Custom(u16, u16),
}

impl From<GameResolution> for String {
    fn from(value: GameResolution) -> Self {
        match value {
            GameResolution::Standard(width, height) => format!("standard:{}x{}", width, height),
            GameResolution::Custom(width, height) => format!("custom:{}x{}", width, height),
        }
    }
}

impl FromStr for GameResolution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');
        let kind = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid resolution"))?;
        let game_resolution = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid resolution"))?;
        let mut resolution_parts = game_resolution.split('x');
        let width = resolution_parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid resolution"))?
            .parse::<u16>()?;
        let height = resolution_parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid resolution"))?
            .parse::<u16>()?;

        match kind {
            "standard" => Ok(Self::Standard(width, height)),
            "custom" => Ok(Self::Custom(width, height)),
            _ => Err(anyhow::anyhow!("Invalid resolution")),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstancesSortBy {
    Name,
    LastPlayed,
    LastUpdated,
    Created,
    GameVersion,
    MostPlayed,
}

impl From<InstancesSortBy> for String {
    fn from(value: InstancesSortBy) -> Self {
        match value {
            InstancesSortBy::Name => "name",
            InstancesSortBy::LastPlayed => "last_played",
            InstancesSortBy::LastUpdated => "last_updated",
            InstancesSortBy::GameVersion => "game_version",
            InstancesSortBy::Created => "created",
            InstancesSortBy::MostPlayed => "most_played",
        }
        .to_string()
    }
}

impl TryFrom<String> for InstancesSortBy {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &*value.to_lowercase() {
            "name" => Ok(Self::Name),
            "last_played" => Ok(Self::LastPlayed),
            "last_updated" => Ok(Self::LastUpdated),
            "game_version" => Ok(Self::GameVersion),
            "created" => Ok(Self::Created),
            "most_played" => Ok(Self::MostPlayed),
            _ => Err(anyhow::anyhow!("Invalid sort by")),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstancesGroupBy {
    Group,
    Modloader,
    GameVersion,
    Modplatform,
}

impl From<InstancesGroupBy> for String {
    fn from(value: InstancesGroupBy) -> Self {
        match value {
            InstancesGroupBy::Group => "group",
            InstancesGroupBy::Modloader => "modloader",
            InstancesGroupBy::GameVersion => "game_version",
            InstancesGroupBy::Modplatform => "modplatform",
        }
        .to_string()
    }
}

impl TryFrom<String> for InstancesGroupBy {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &*value.to_lowercase() {
            "group" => Ok(Self::Group),
            "modloader" => Ok(Self::Modloader),
            "game_version" => Ok(Self::GameVersion),
            "modplatform" => Ok(Self::Modplatform),
            _ => Err(anyhow::anyhow!("Invalid group by")),
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
    release_channel: FEReleaseChannel,
    concurrent_downloads: i32,
    download_dependencies: bool,
    launcher_action_on_game_launch: FELauncherActionOnGameLaunch,
    show_news: bool,
    instances_sort_by: InstancesSortBy,
    instances_sort_by_asc: bool,
    instances_group_by: InstancesGroupBy,
    instances_group_by_asc: bool,
    instances_tile_size: i32,
    deletion_through_recycle_bin: bool,
    xmx: i32,
    xms: i32,
    pre_launch_hook: Option<String>,
    wrapper_command: Option<String>,
    post_exit_hook: Option<String>,
    is_first_launch: bool,
    game_resolution: Option<GameResolution>,
    java_custom_args: String,
    auto_manage_java: bool,
    mod_sources: ModSources,
    terms_and_privacy_accepted: bool,
    metrics_enabled: bool,
    random_user_uuid: String,
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
            download_dependencies: data.download_dependencies,
            show_news: data.show_news,
            instances_sort_by: data.instances_sort_by.try_into()?,
            instances_sort_by_asc: data.instances_sort_by_asc,
            instances_group_by: data.instances_group_by.try_into()?,
            instances_group_by_asc: data.instances_group_by_asc,
            instances_tile_size: data.instances_tile_size,
            deletion_through_recycle_bin: data.deletion_through_recycle_bin,
            xmx: data.xmx,
            xms: data.xms,
            pre_launch_hook: data.pre_launch_hook,
            wrapper_command: data.wrapper_command,
            post_exit_hook: data.post_exit_hook,
            is_first_launch: data.is_first_launch,
            launcher_action_on_game_launch: data.launcher_action_on_game_launch.try_into()?,
            game_resolution: data
                .game_resolution
                .and_then(|r| GameResolution::from_str(&r).ok()),
            java_custom_args: data.java_custom_args,
            auto_manage_java: data.auto_manage_java,
            mod_sources: ModSources {
                channels: {
                    use crate::domain::modplatforms::ModChannelWithUsage as DModChannelWithUsage;

                    let mut channels = DModChannelWithUsage::str_to_vec(&data.mod_channels)?;
                    DModChannelWithUsage::fixup_list(&mut channels);

                    channels
                        .into_iter()
                        .map(ModChannelWithUsage::from)
                        .collect()
                },
                platform_blacklist: data
                    .mod_platform_blacklist
                    .split(",")
                    .filter(|p| !p.is_empty())
                    .map(crate::domain::modplatforms::ModPlatform::from_str)
                    .map(|r| r.map(ModPlatform::from))
                    .collect::<Result<_, _>>()?,
            },
            terms_and_privacy_accepted: data.terms_and_privacy_accepted,
            metrics_enabled: data.metrics_enabled,
            random_user_uuid: data.random_user_uuid,
        })
    }
}

#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FELauncherActionOnGameLaunch {
    QuitApp,
    CloseWindow,
    MinimizeWindow,
    HideWindow,
    None,
}

impl From<FELauncherActionOnGameLaunch> for String {
    fn from(value: FELauncherActionOnGameLaunch) -> Self {
        match value {
            FELauncherActionOnGameLaunch::QuitApp => "quitApp",
            FELauncherActionOnGameLaunch::CloseWindow => "closeWindow",
            FELauncherActionOnGameLaunch::MinimizeWindow => "minimizeWindow",
            FELauncherActionOnGameLaunch::HideWindow => "hideWindow",
            FELauncherActionOnGameLaunch::None => "none",
        }
        .to_string()
    }
}

impl TryFrom<String> for FELauncherActionOnGameLaunch {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &*value {
            "quitApp" => Ok(Self::QuitApp),
            "closeWindow" => Ok(Self::CloseWindow),
            "minimizeWindow" => Ok(Self::MinimizeWindow),
            "hideWindow" => Ok(Self::HideWindow),
            "none" => Ok(Self::None),
            _ => Err(anyhow::anyhow!("Invalid action on game launch")),
        }
    }
}

// When updating this, make sure to also update set_settings
#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FESettingsUpdate {
    #[specta(optional)]
    pub theme: Option<Set<String>>,
    #[specta(optional)]
    pub language: Option<Set<String>>,
    #[specta(optional)]
    pub reduced_motion: Option<Set<bool>>,
    #[specta(optional)]
    pub discord_integration: Option<Set<bool>>,
    #[specta(optional)]
    pub release_channel: Option<Set<FEReleaseChannel>>,
    #[specta(optional)]
    pub concurrent_downloads: Option<Set<i32>>,
    #[specta(optional)]
    pub download_dependencies: Option<Set<bool>>,
    #[specta(optional)]
    pub instances_sort_by: Option<Set<InstancesSortBy>>,
    #[specta(optional)]
    pub instances_sort_by_asc: Option<Set<bool>>,
    #[specta(optional)]
    pub instances_group_by: Option<Set<InstancesGroupBy>>,
    #[specta(optional)]
    pub instances_group_by_asc: Option<Set<bool>>,
    #[specta(optional)]
    pub instances_tile_size: Option<Set<i32>>,
    #[specta(optional)]
    pub deletion_through_recycle_bin: Option<Set<bool>>,
    #[specta(optional)]
    pub show_news: Option<Set<bool>>,
    #[specta(optional)]
    pub xmx: Option<Set<i32>>,
    #[specta(optional)]
    pub xms: Option<Set<i32>>,
    #[specta(optional)]
    pub pre_launch_hook: Option<Set<Option<String>>>,
    #[specta(optional)]
    pub wrapper_command: Option<Set<Option<String>>>,
    #[specta(optional)]
    pub post_exit_hook: Option<Set<Option<String>>>,
    #[specta(optional)]
    pub is_first_launch: Option<Set<bool>>,
    #[specta(optional)]
    pub launcher_action_on_game_launch: Option<Set<FELauncherActionOnGameLaunch>>,
    #[specta(optional)]
    pub game_resolution: Option<Set<Option<GameResolution>>>,
    #[specta(optional)]
    pub java_custom_args: Option<Set<String>>,
    #[specta(optional)]
    pub auto_manage_java: Option<Set<bool>>,
    #[specta(optional)]
    pub mod_sources: Option<Set<ModSources>>,
    #[specta(optional)]
    pub terms_and_privacy_accepted: Option<Set<bool>>,
    #[specta(optional)]
    pub metrics_enabled: Option<Set<bool>>,
}
