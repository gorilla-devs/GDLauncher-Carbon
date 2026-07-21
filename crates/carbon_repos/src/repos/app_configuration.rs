//! Repository queries for the `AppConfiguration` singleton (row `id = 0`).

use crate::queries;
use crate::registry::DynamicQuery;

/// Mirrors every column of the `AppConfiguration` model. Field names are
/// snake_case; the `FromRow` derive maps them to their camelCase columns.
#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct AppConfigurationRow {
    pub id: i32,
    pub theme: String,
    pub reduced_motion: bool,
    pub language: String,
    pub discord_integration: bool,
    pub release_channel: String,
    pub active_account_uuid: Option<String>,
    pub concurrent_downloads: i32,
    pub download_dependencies: bool,
    pub instances_tile_size: i32,
    pub instances_group_by: Option<String>,
    pub instances_group_by_asc: bool,
    pub instances_sort_by: Option<String>,
    pub instances_sort_by_asc: bool,
    pub instances_duplicate_favorites: bool,
    pub show_featured: bool,
    pub deletion_through_recycle_bin: bool,
    pub game_resolution: Option<String>,
    pub launcher_action_on_game_launch: String,
    pub show_app_close_warning: bool,
    pub java_custom_args: String,
    pub xmx: i32,
    pub xms: i32,
    pub default_instance_group: Option<i32>,
    pub pre_launch_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub post_exit_hook: Option<String>,
    pub auto_manage_java_system_profiles: bool,
    pub mod_platform_blacklist: String,
    pub mod_channels: String,
    pub terms_and_privacy_accepted: bool,
    pub terms_and_privacy_accepted_checksum: Option<String>,
    pub gdl_account_uuid: Option<String>,
    pub gdl_account_status: Option<Vec<u8>>,
    pub installation_id: Option<String>,
    pub servers_tile_size: i32,
    pub servers_group_by: Option<String>,
    pub servers_group_by_asc: bool,
    pub servers_sort_by: Option<String>,
    pub servers_sort_by_asc: bool,
    pub servers_duplicate_favorites: bool,
    pub default_server_group: Option<i32>,
}

queries! {
    fn get_app_configuration() -> Option<AppConfigurationRow> =
        "SELECT id, theme, reducedMotion, language, discordIntegration, releaseChannel, activeAccountUuid, concurrentDownloads, downloadDependencies, instancesTileSize, instancesGroupBy, instancesGroupByAsc, instancesSortBy, instancesSortByAsc, instancesDuplicateFavorites, showFeatured, deletionThroughRecycleBin, gameResolution, launcherActionOnGameLaunch, showAppCloseWarning, javaCustomArgs, xmx, xms, defaultInstanceGroup, preLaunchHook, wrapperCommand, postExitHook, autoManageJavaSystemProfiles, modPlatformBlacklist, modChannels, termsAndPrivacyAccepted, termsAndPrivacyAcceptedChecksum, gdlAccountUuid, gdlAccountStatus, installationId, serversTileSize, serversGroupBy, serversGroupByAsc, serversSortBy, serversSortByAsc, serversDuplicateFavorites, defaultServerGroup FROM AppConfiguration WHERE id = 0";
    fn count_app_configuration() -> i64 =
        "SELECT COUNT(*) FROM AppConfiguration";
}

/// The `INSERT` executed by `insert_app_configuration` and validated by
/// `INSERT_APP_CONFIGURATION_CHECK`. Shared so the checker covers the exact
/// SQL the fn runs. `id` is pinned to the singleton value 0; every column not
/// listed takes its DDL default.
const INSERT_APP_CONFIGURATION_SQL: &str =
    "INSERT INTO AppConfiguration (id, releaseChannel, xmx, installationId)
         VALUES (0, :release_channel, :xmx, :installation_id)";

/// Inserts the singleton `AppConfiguration` row (id = 0). Hand-written because
/// the macro's arg list only takes scalar params and the remaining columns rely
/// on their DDL defaults.
pub fn insert_app_configuration(
    conn: &rusqlite::Connection,
    release_channel: &str,
    xmx: i32,
    installation_id: Option<&str>,
) -> Result<usize, rusqlite::Error> {
    let mut st = conn.prepare_cached(INSERT_APP_CONFIGURATION_SQL)?;
    st.execute(rusqlite::named_params! {
        ":release_channel": release_channel,
        ":xmx": xmx,
        ":installation_id": installation_id,
    })
}

const INSERT_APP_CONFIGURATION_CHECK: crate::registry::QueryCheck = crate::registry::QueryCheck {
    name: "insert_app_configuration",
    sql: INSERT_APP_CONFIGURATION_SQL,
    params: &[":release_channel", ":xmx", ":installation_id"],
    columns: None,
};

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    let mut all: Vec<crate::registry::QueryCheck> = QUERIES.to_vec();
    all.push(INSERT_APP_CONFIGURATION_CHECK);
    all
}

/// A partial update to the `AppConfiguration` singleton. Each present field
/// becomes one `SET col = :param` clause; absent fields are left untouched.
/// `Option<Option<T>>` fields target nullable columns — outer `Some` means
/// "write this column", inner value (`Some`/`None`) is the value/NULL to store.
/// `default_server_group` is intentionally absent: it has no writer.
#[derive(Debug, Default, Clone)]
pub struct AppConfigurationPatch {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub reduced_motion: Option<bool>,
    pub discord_integration: Option<bool>,
    pub release_channel: Option<String>,
    pub launcher_action_on_game_launch: Option<String>,
    pub show_app_close_warning: Option<bool>,
    pub concurrent_downloads: Option<i32>,
    pub download_dependencies: Option<bool>,
    pub show_featured: Option<bool>,
    pub instances_sort_by: Option<Option<String>>,
    pub instances_sort_by_asc: Option<bool>,
    pub instances_group_by: Option<Option<String>>,
    pub instances_group_by_asc: Option<bool>,
    pub instances_duplicate_favorites: Option<bool>,
    pub instances_tile_size: Option<i32>,
    pub deletion_through_recycle_bin: Option<bool>,
    pub xmx: Option<i32>,
    pub xms: Option<i32>,
    pub game_resolution: Option<Option<String>>,
    pub java_custom_args: Option<String>,
    pub pre_launch_hook: Option<Option<String>>,
    pub post_exit_hook: Option<Option<String>>,
    pub wrapper_command: Option<Option<String>>,
    pub auto_manage_java_system_profiles: Option<bool>,
    pub mod_platform_blacklist: Option<String>,
    pub mod_channels: Option<String>,
    pub terms_and_privacy_accepted: Option<bool>,
    pub terms_and_privacy_accepted_checksum: Option<Option<String>>,
    pub installation_id: Option<Option<String>>,
    pub active_account_uuid: Option<Option<String>>,
    pub gdl_account_uuid: Option<Option<String>>,
    pub gdl_account_status: Option<Option<Vec<u8>>>,
    pub default_instance_group: Option<Option<i32>>,
}

impl AppConfigurationPatch {
    /// Assembles `UPDATE AppConfiguration SET ... WHERE id = 0` from the present
    /// fields. Returns `None` when no field is set (nothing to write).
    pub fn build(self) -> Option<DynamicQuery> {
        let mut sets: Vec<&'static str> = Vec::new();
        let mut params: Vec<(&'static str, Box<dyn rusqlite::types::ToSql + Send>)> = Vec::new();

        macro_rules! push {
            ($field:expr, $set:literal, $param:literal) => {
                if let Some(v) = $field {
                    sets.push($set);
                    params.push(($param, Box::new(v)));
                }
            };
        }

        push!(self.theme, "theme = :theme", ":theme");
        push!(self.language, "language = :language", ":language");
        push!(self.reduced_motion, "reducedMotion = :reducedMotion", ":reducedMotion");
        push!(self.discord_integration, "discordIntegration = :discordIntegration", ":discordIntegration");
        push!(self.release_channel, "releaseChannel = :releaseChannel", ":releaseChannel");
        push!(self.launcher_action_on_game_launch, "launcherActionOnGameLaunch = :launcherActionOnGameLaunch", ":launcherActionOnGameLaunch");
        push!(self.show_app_close_warning, "showAppCloseWarning = :showAppCloseWarning", ":showAppCloseWarning");
        push!(self.concurrent_downloads, "concurrentDownloads = :concurrentDownloads", ":concurrentDownloads");
        push!(self.download_dependencies, "downloadDependencies = :downloadDependencies", ":downloadDependencies");
        push!(self.show_featured, "showFeatured = :showFeatured", ":showFeatured");
        push!(self.instances_sort_by, "instancesSortBy = :instancesSortBy", ":instancesSortBy");
        push!(self.instances_sort_by_asc, "instancesSortByAsc = :instancesSortByAsc", ":instancesSortByAsc");
        push!(self.instances_group_by, "instancesGroupBy = :instancesGroupBy", ":instancesGroupBy");
        push!(self.instances_group_by_asc, "instancesGroupByAsc = :instancesGroupByAsc", ":instancesGroupByAsc");
        push!(self.instances_duplicate_favorites, "instancesDuplicateFavorites = :instancesDuplicateFavorites", ":instancesDuplicateFavorites");
        push!(self.instances_tile_size, "instancesTileSize = :instancesTileSize", ":instancesTileSize");
        push!(self.deletion_through_recycle_bin, "deletionThroughRecycleBin = :deletionThroughRecycleBin", ":deletionThroughRecycleBin");
        push!(self.xmx, "xmx = :xmx", ":xmx");
        push!(self.xms, "xms = :xms", ":xms");
        push!(self.game_resolution, "gameResolution = :gameResolution", ":gameResolution");
        push!(self.java_custom_args, "javaCustomArgs = :javaCustomArgs", ":javaCustomArgs");
        push!(self.pre_launch_hook, "preLaunchHook = :preLaunchHook", ":preLaunchHook");
        push!(self.post_exit_hook, "postExitHook = :postExitHook", ":postExitHook");
        push!(self.wrapper_command, "wrapperCommand = :wrapperCommand", ":wrapperCommand");
        push!(self.auto_manage_java_system_profiles, "autoManageJavaSystemProfiles = :autoManageJavaSystemProfiles", ":autoManageJavaSystemProfiles");
        push!(self.mod_platform_blacklist, "modPlatformBlacklist = :modPlatformBlacklist", ":modPlatformBlacklist");
        push!(self.mod_channels, "modChannels = :modChannels", ":modChannels");
        push!(self.terms_and_privacy_accepted, "termsAndPrivacyAccepted = :termsAndPrivacyAccepted", ":termsAndPrivacyAccepted");
        push!(self.terms_and_privacy_accepted_checksum, "termsAndPrivacyAcceptedChecksum = :termsAndPrivacyAcceptedChecksum", ":termsAndPrivacyAcceptedChecksum");
        push!(self.installation_id, "installationId = :installationId", ":installationId");
        push!(self.active_account_uuid, "activeAccountUuid = :activeAccountUuid", ":activeAccountUuid");
        push!(self.gdl_account_uuid, "gdlAccountUuid = :gdlAccountUuid", ":gdlAccountUuid");
        push!(self.gdl_account_status, "gdlAccountStatus = :gdlAccountStatus", ":gdlAccountStatus");
        push!(self.default_instance_group, "defaultInstanceGroup = :defaultInstanceGroup", ":defaultInstanceGroup");

        if sets.is_empty() {
            return None;
        }

        let sql = format!(
            "UPDATE AppConfiguration SET {} WHERE id = 0",
            sets.join(", ")
        );
        Some(DynamicQuery { sql, params })
    }
}
