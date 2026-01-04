use super::Set;
use crate::{
    api::{
        keys::{
            self,
            settings::{
                COMPLETE_FIRST_LAUNCH, DISMISS_BETA_PROMPT_PERMANENTLY, GET_PRIVACY_STATEMENT_BODY,
                GET_SEEN_ONBOARDING_TIPS, GET_SETTINGS, GET_TERMS_OF_SERVICE_BODY, IS_FIRST_LAUNCH,
                MARK_CHANGELOG_SEEN, MARK_ONBOARDING_TIP_SEEN, REMIND_BETA_PROMPT_LATER,
                RESET_ONBOARDING_TIPS, SET_SETTINGS, SHOULD_SHOW_BETA_PROMPT,
                SHOULD_SHOW_CHANGELOG,
            },
        },
        router::router,
    },
    app_version::APP_VERSION,
    managers::{App, prisma_client::is_in_beta_prompt_cohort},
    mirror_into,
};
use carbon_repos::db::frontend_preference;
use chrono::{DateTime, Duration, Utc};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::str::FromStr;

/// Internal preference keys - not exposed to frontend
mod preference_keys {
    pub const FIRST_LAUNCH_COMPLETED: &str = "first_launch_completed";
    pub const LAST_SEEN_VERSION: &str = "last_seen_version";
    pub const BETA_PROMPT_DISMISSED: &str = "beta_prompt_dismissed_permanently";
    pub const BETA_PROMPT_LAST_SHOWN: &str = "beta_prompt_last_shown";
    pub const ONBOARDING_TIPS_SEEN: &str = "onboarding_tips_seen";
}

/// Input state for beta prompt decision logic
#[derive(Debug, Clone)]
pub struct BetaPromptState {
    pub release_channel: String,
    pub first_launch_completed: bool,
    pub permanently_dismissed: bool,
    pub installation_id: Option<String>,
    pub last_shown: Option<DateTime<Utc>>,
}

/// Constants for beta prompt logic
pub const BETA_PROMPT_PERCENTAGE: f64 = 0.03; // 3% rollout
pub const REMIND_AFTER_DAYS: i64 = 7;

/// Pure function to determine if beta prompt should be shown.
/// Extracted for testability.
pub fn should_show_beta_prompt_logic(state: &BetaPromptState, now: DateTime<Utc>) -> bool {
    // Only show to stable channel users
    if state.release_channel != "stable" {
        return false;
    }

    // Don't show on first launch (let them complete onboarding first)
    if !state.first_launch_completed {
        return false;
    }

    // Check if permanently dismissed
    if state.permanently_dismissed {
        return false;
    }

    // Check if installation ID exists and is in cohort
    let Some(installation_id) = &state.installation_id else {
        return false;
    };

    if !is_in_beta_prompt_cohort(installation_id, BETA_PROMPT_PERCENTAGE) {
        return false;
    }

    // Check if recently shown (remind after X days)
    if let Some(last_shown) = state.last_shown {
        let remind_after = last_shown + Duration::days(REMIND_AFTER_DAYS);
        if now < remind_after {
            return false;
        }
    }

    true
}

pub(super) fn mount() -> RouterBuilder<App> {
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

        // First Launch endpoints
        query IS_FIRST_LAUNCH[app, _args: ()] {
            let db = &app.prisma_client;

            let pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::FIRST_LAUNCH_COMPLETED.to_string()
                ))
                .exec()
                .await?;

            // First launch is true if the key is absent (not completed yet)
            Ok(pref.is_none())
        }

        mutation COMPLETE_FIRST_LAUNCH[app, _args: ()] {
            let db = &app.prisma_client;

            db.frontend_preference()
                .upsert(
                    frontend_preference::key::equals(preference_keys::FIRST_LAUNCH_COMPLETED.to_string()),
                    frontend_preference::create(
                        preference_keys::FIRST_LAUNCH_COMPLETED.to_string(),
                        "true".to_string(),
                        vec![]
                    ),
                    vec![frontend_preference::value::set("true".to_string())],
                )
                .exec()
                .await?;

            // Invalidate related queries
            app.invalidate(IS_FIRST_LAUNCH, None);
            app.invalidate(SHOULD_SHOW_BETA_PROMPT, None);

            Ok(())
        }

        // Changelog endpoints
        query SHOULD_SHOW_CHANGELOG[app, _args: ()] {
            let db = &app.prisma_client;

            let pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::LAST_SEEN_VERSION.to_string()
                ))
                .exec()
                .await?;

            // Show changelog if no version stored or version differs from current
            match pref {
                Some(p) => Ok(p.value != APP_VERSION),
                None => Ok(true),
            }
        }

        mutation MARK_CHANGELOG_SEEN[app, _args: ()] {
            let db = &app.prisma_client;

            db.frontend_preference()
                .upsert(
                    frontend_preference::key::equals(preference_keys::LAST_SEEN_VERSION.to_string()),
                    frontend_preference::create(
                        preference_keys::LAST_SEEN_VERSION.to_string(),
                        APP_VERSION.to_string(),
                        vec![]
                    ),
                    vec![frontend_preference::value::set(APP_VERSION.to_string())],
                )
                .exec()
                .await?;

            // Invalidate so the query returns fresh data
            app.invalidate(SHOULD_SHOW_CHANGELOG, None);

            Ok(())
        }

        // Beta Prompt endpoints
        query SHOULD_SHOW_BETA_PROMPT[app, _args: ()] {
            let db = &app.prisma_client;
            let config = app.settings_manager().get_settings().await?;

            // Load preferences from database
            let first_launch_pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::FIRST_LAUNCH_COMPLETED.to_string()
                ))
                .exec()
                .await?;

            let dismissed_pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::BETA_PROMPT_DISMISSED.to_string()
                ))
                .exec()
                .await?;

            let last_shown_pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::BETA_PROMPT_LAST_SHOWN.to_string()
                ))
                .exec()
                .await?;

            // Build state for decision logic
            let state = BetaPromptState {
                release_channel: config.release_channel.clone(),
                first_launch_completed: first_launch_pref.is_some(),
                permanently_dismissed: dismissed_pref.map(|p| p.value == "true").unwrap_or(false),
                installation_id: config.installation_id.clone(),
                last_shown: last_shown_pref.and_then(|p| {
                    DateTime::parse_from_rfc3339(&p.value)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
            };

            let result = should_show_beta_prompt_logic(&state, Utc::now());
            tracing::debug!("Beta prompt check: {:?} -> {}", state, result);
            Ok(result)
        }

        mutation DISMISS_BETA_PROMPT_PERMANENTLY[app, _args: ()] {
            let db = &app.prisma_client;

            db.frontend_preference()
                .upsert(
                    frontend_preference::key::equals(preference_keys::BETA_PROMPT_DISMISSED.to_string()),
                    frontend_preference::create(
                        preference_keys::BETA_PROMPT_DISMISSED.to_string(),
                        "true".to_string(),
                        vec![]
                    ),
                    vec![frontend_preference::value::set("true".to_string())],
                )
                .exec()
                .await?;

            Ok(())
        }

        mutation REMIND_BETA_PROMPT_LATER[app, _args: ()] {
            let db = &app.prisma_client;
            let now = Utc::now().to_rfc3339();

            db.frontend_preference()
                .upsert(
                    frontend_preference::key::equals(preference_keys::BETA_PROMPT_LAST_SHOWN.to_string()),
                    frontend_preference::create(
                        preference_keys::BETA_PROMPT_LAST_SHOWN.to_string(),
                        now.clone(),
                        vec![]
                    ),
                    vec![frontend_preference::value::set(now)],
                )
                .exec()
                .await?;

            Ok(())
        }

        // Onboarding Tips endpoints
        query GET_SEEN_ONBOARDING_TIPS[app, _args: ()] {
            let db = &app.prisma_client;

            let pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::ONBOARDING_TIPS_SEEN.to_string()
                ))
                .exec()
                .await?;

            match pref {
                Some(p) => Ok(serde_json::from_str::<Vec<String>>(&p.value).unwrap_or_default()),
                None => Ok(Vec::new()),
            }
        }

        mutation MARK_ONBOARDING_TIP_SEEN[app, tip_id: String] {
            let db = &app.prisma_client;

            // Get existing tips
            let pref = db
                .frontend_preference()
                .find_unique(frontend_preference::key::equals(
                    preference_keys::ONBOARDING_TIPS_SEEN.to_string()
                ))
                .exec()
                .await?;

            let mut tips: Vec<String> = match pref {
                Some(p) => serde_json::from_str(&p.value).unwrap_or_default(),
                None => Vec::new(),
            };

            // Add tip if not already seen
            if !tips.contains(&tip_id) {
                tips.push(tip_id);
            }

            let value = serde_json::to_string(&tips)?;

            db.frontend_preference()
                .upsert(
                    frontend_preference::key::equals(preference_keys::ONBOARDING_TIPS_SEEN.to_string()),
                    frontend_preference::create(
                        preference_keys::ONBOARDING_TIPS_SEEN.to_string(),
                        value.clone(),
                        vec![]
                    ),
                    vec![frontend_preference::value::set(value)],
                )
                .exec()
                .await?;

            // Invalidate so the query returns fresh data
            app.invalidate(GET_SEEN_ONBOARDING_TIPS, None);

            Ok(())
        }

        mutation RESET_ONBOARDING_TIPS[app, _args: ()] {
            let db = &app.prisma_client;

            db.frontend_preference()
                .delete(frontend_preference::key::equals(
                    preference_keys::ONBOARDING_TIPS_SEEN.to_string()
                ))
                .exec()
                .await
                .ok(); // Ignore if not found

            // Invalidate so the query returns fresh data
            app.invalidate(GET_SEEN_ONBOARDING_TIPS, None);

            Ok(())
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
    show_app_close_warning: bool,
    show_featured: bool,
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
    game_resolution: Option<GameResolution>,
    java_custom_args: String,
    auto_manage_java_system_profiles: bool,
    mod_sources: ModSources,
    terms_and_privacy_accepted: bool,
    gdl_account_id: Option<String>,
}

impl TryFrom<carbon_repos::db::app_configuration::Data> for FESettings {
    type Error = anyhow::Error;

    fn try_from(data: carbon_repos::db::app_configuration::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            theme: data.theme,
            language: data.language,
            reduced_motion: data.reduced_motion,
            discord_integration: data.discord_integration,
            release_channel: data.release_channel.try_into()?,
            concurrent_downloads: data.concurrent_downloads,
            download_dependencies: data.download_dependencies,
            show_featured: data.show_featured,
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
            launcher_action_on_game_launch: data.launcher_action_on_game_launch.try_into()?,
            show_app_close_warning: data.show_app_close_warning,
            game_resolution: data
                .game_resolution
                .and_then(|r| GameResolution::from_str(&r).ok()),
            java_custom_args: data.java_custom_args,
            auto_manage_java_system_profiles: data.auto_manage_java_system_profiles,
            mod_sources: ModSources {
                channels: {
                    use carbon_platforms::ModChannelWithUsage as DModChannelWithUsage;

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
                    .map(carbon_platforms::ModPlatform::from_str)
                    .map(|r| r.map(ModPlatform::from))
                    .collect::<Result<_, _>>()?,
            },
            terms_and_privacy_accepted: data.terms_and_privacy_accepted,
            gdl_account_id: data.gdl_account_uuid,
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
    pub show_featured: Option<Set<bool>>,
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
    pub launcher_action_on_game_launch: Option<Set<FELauncherActionOnGameLaunch>>,
    #[specta(optional)]
    pub show_app_close_warning: Option<Set<bool>>,
    #[specta(optional)]
    pub game_resolution: Option<Set<Option<GameResolution>>>,
    #[specta(optional)]
    pub java_custom_args: Option<Set<String>>,
    #[specta(optional)]
    pub auto_manage_java_system_profiles: Option<Set<bool>>,
    #[specta(optional)]
    pub mod_sources: Option<Set<ModSources>>,
    #[specta(optional)]
    pub terms_and_privacy_accepted: Option<Set<bool>>,
    #[specta(optional)]
    pub gdl_account_id: Option<Set<Option<String>>>,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FESearchAPI {
    Curseforge,
    Modrinth,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone, Copy)]
#[repr(i32)]
pub enum ModChannel {
    Alpha = 0,
    Beta,
    Stable,
}
impl Default for ModChannel {
    fn default() -> Self {
        Self::Stable
    }
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

mirror_into!(
    ModChannel,
    carbon_platforms::ModChannel,
    |value| match value {
        Other::Alpha => Self::Alpha,
        Other::Beta => Self::Beta,
        Other::Stable => Self::Stable,
    }
);

#[derive(Type, Debug, Deserialize, Serialize, Clone, Copy)]
pub enum ModPlatform {
    Curseforge,
    Modrinth,
}

mirror_into!(
    ModPlatform,
    carbon_platforms::ModPlatform,
    |value| match value {
        Other::Curseforge => Self::Curseforge,
        Other::Modrinth => Self::Modrinth,
    }
);

#[derive(Type, Debug, Deserialize, Serialize, Clone, Copy)]
pub struct ModChannelWithUsage {
    pub channel: ModChannel,
    pub allow_updates: bool,
}

mirror_into!(
    ModChannelWithUsage,
    carbon_platforms::ModChannelWithUsage,
    |value| {
        Self {
            channel: value.channel.into(),
            allow_updates: value.allow_updates,
        }
    }
);

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
pub struct ModSources {
    pub channels: Vec<ModChannelWithUsage>,
    pub platform_blacklist: Vec<ModPlatform>,
}

mirror_into!(ModSources, carbon_platforms::ModSources, |value| Self {
    channels: value.channels.into_iter().map(Into::into).collect(),
    platform_blacklist: value
        .platform_blacklist
        .into_iter()
        .map(Into::into)
        .collect(),
});

#[cfg(test)]
mod beta_prompt_tests {
    use super::*;
    use chrono::TimeZone;

    /// Helper to create a valid state that would show the prompt
    fn valid_state() -> BetaPromptState {
        BetaPromptState {
            release_channel: "stable".to_string(),
            first_launch_completed: true,
            permanently_dismissed: false,
            // Use an ID that's in the 3% cohort (starts with low hex value)
            installation_id: Some("00000001-0000-0000-0000-000000000000".to_string()),
            last_shown: None,
        }
    }

    #[test]
    fn shows_when_all_conditions_met() {
        let state = valid_state();
        let now = Utc::now();
        assert!(should_show_beta_prompt_logic(&state, now));
    }

    #[test]
    fn not_stable_channel_returns_false() {
        let mut state = valid_state();
        state.release_channel = "beta".to_string();
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));

        state.release_channel = "alpha".to_string();
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));
    }

    #[test]
    fn first_launch_not_completed_returns_false() {
        let mut state = valid_state();
        state.first_launch_completed = false;
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));
    }

    #[test]
    fn permanently_dismissed_returns_false() {
        let mut state = valid_state();
        state.permanently_dismissed = true;
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));
    }

    #[test]
    fn no_installation_id_returns_false() {
        let mut state = valid_state();
        state.installation_id = None;
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));
    }

    #[test]
    fn not_in_cohort_returns_false() {
        let mut state = valid_state();
        // Use an ID that's NOT in the 3% cohort (starts with high hex value)
        state.installation_id = Some("ffffffff-0000-0000-0000-000000000000".to_string());
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));
    }

    #[test]
    fn recently_shown_returns_false() {
        let mut state = valid_state();
        let now = Utc::now();
        // Shown 3 days ago - should still be within the 7-day reminder period
        state.last_shown = Some(now - Duration::days(3));
        assert!(!should_show_beta_prompt_logic(&state, now));
    }

    #[test]
    fn shown_exactly_7_days_ago_returns_true() {
        let mut state = valid_state();
        let now = Utc::now();
        // Shown exactly 7 days ago - reminder period has passed
        state.last_shown = Some(now - Duration::days(7));
        assert!(should_show_beta_prompt_logic(&state, now));
    }

    #[test]
    fn shown_more_than_7_days_ago_returns_true() {
        let mut state = valid_state();
        let now = Utc::now();
        // Shown 10 days ago - should show again
        state.last_shown = Some(now - Duration::days(10));
        assert!(should_show_beta_prompt_logic(&state, now));
    }

    #[test]
    fn remind_later_within_period_does_not_show() {
        let mut state = valid_state();
        let now = Utc::now();

        // Simulate "remind later" clicked 1 day ago
        state.last_shown = Some(now - Duration::days(1));
        assert!(!should_show_beta_prompt_logic(&state, now));

        // Simulate "remind later" clicked 6 days ago
        state.last_shown = Some(now - Duration::days(6));
        assert!(!should_show_beta_prompt_logic(&state, now));
    }

    #[test]
    fn dismiss_permanently_never_shows_again() {
        let mut state = valid_state();
        state.permanently_dismissed = true;

        // Even if all other conditions are met, should not show
        assert!(!should_show_beta_prompt_logic(&state, Utc::now()));

        // Even after a long time
        let future = Utc::now() + Duration::days(365);
        assert!(!should_show_beta_prompt_logic(&state, future));
    }

    #[test]
    fn remind_later_shows_after_period_expires() {
        let mut state = valid_state();
        let now = Utc::now();

        // Clicked "remind later" 8 days ago - should show again
        state.last_shown = Some(now - Duration::days(8));
        assert!(should_show_beta_prompt_logic(&state, now));
    }

    #[test]
    fn boundary_at_exactly_remind_period() {
        let mut state = valid_state();
        // Use a fixed timestamp for deterministic testing
        let now = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();

        // Last shown exactly 7 days ago at the same time
        let seven_days_ago = now - Duration::days(7);
        state.last_shown = Some(seven_days_ago);

        // At exactly the boundary, should show (>= 7 days means show)
        assert!(should_show_beta_prompt_logic(&state, now));

        // 1 second before the boundary, should not show
        let almost_now = now - Duration::seconds(1);
        state.last_shown = Some(almost_now - Duration::days(7) + Duration::seconds(1));
        assert!(!should_show_beta_prompt_logic(&state, almost_now));
    }
}
