//! AppConfiguration model - application settings.

use carbon_macro::FromRow;
use serde::{Deserialize, Serialize};

/// Application configuration stored as a single row (id=0).
///
/// Contains all application-wide settings including UI preferences,
/// Java configuration, instance display settings, and legal acceptances.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AppConfiguration {
    pub id: i32,
    // UI settings
    pub theme: String,
    pub reduced_motion: bool,
    pub language: String,
    pub discord_integration: bool,
    // Version and release
    pub release_channel: String,
    pub last_app_version: Option<String>,
    // Account
    pub active_account_uuid: Option<String>,
    // Instance settings
    pub concurrent_downloads: i32,
    pub download_dependencies: bool,
    pub instances_tile_size: i32,
    pub instances_group_by: String,
    pub instances_group_by_asc: bool,
    pub instances_sort_by: String,
    pub instances_sort_by_asc: bool,
    pub show_featured: bool,
    // Behavior settings
    pub deletion_through_recycle_bin: bool,
    pub game_resolution: Option<String>,
    pub launcher_action_on_game_launch: String,
    pub show_app_close_warning: bool,
    // Java settings
    pub java_custom_args: String,
    pub xmx: i32,
    pub xms: i32,
    pub default_instance_group: Option<i32>,
    // Hooks
    pub pre_launch_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub post_exit_hook: Option<String>,
    // State
    pub is_first_launch: bool,
    pub auto_manage_java_system_profiles: bool,
    // Mod platform settings
    pub mod_platform_blacklist: String,
    pub mod_channels: String,
    // Legal
    pub terms_and_privacy_accepted: bool,
    pub terms_and_privacy_accepted_checksum: Option<String>,
    // GDL Account
    pub gdl_account_uuid: Option<String>,
    pub gdl_account_status: Option<Vec<u8>>,
}

/// Builder for creating/updating AppConfiguration.
#[derive(Debug, Default)]
pub struct AppConfigurationBuilder {
    pub theme: Option<String>,
    pub reduced_motion: Option<bool>,
    pub language: Option<String>,
    pub discord_integration: Option<bool>,
    pub release_channel: Option<String>,
    pub last_app_version: Option<Option<String>>,
    pub active_account_uuid: Option<Option<String>>,
    pub concurrent_downloads: Option<i32>,
    pub download_dependencies: Option<bool>,
    pub instances_tile_size: Option<i32>,
    pub instances_group_by: Option<String>,
    pub instances_group_by_asc: Option<bool>,
    pub instances_sort_by: Option<String>,
    pub instances_sort_by_asc: Option<bool>,
    pub show_featured: Option<bool>,
    pub deletion_through_recycle_bin: Option<bool>,
    pub game_resolution: Option<Option<String>>,
    pub launcher_action_on_game_launch: Option<String>,
    pub show_app_close_warning: Option<bool>,
    pub java_custom_args: Option<String>,
    pub xmx: Option<i32>,
    pub xms: Option<i32>,
    pub default_instance_group: Option<Option<i32>>,
    pub pre_launch_hook: Option<Option<String>>,
    pub wrapper_command: Option<Option<String>>,
    pub post_exit_hook: Option<Option<String>>,
    pub is_first_launch: Option<bool>,
    pub auto_manage_java_system_profiles: Option<bool>,
    pub mod_platform_blacklist: Option<String>,
    pub mod_channels: Option<String>,
    pub terms_and_privacy_accepted: Option<bool>,
    pub terms_and_privacy_accepted_checksum: Option<Option<String>>,
    pub gdl_account_uuid: Option<Option<String>>,
    pub gdl_account_status: Option<Option<Vec<u8>>>,
}
