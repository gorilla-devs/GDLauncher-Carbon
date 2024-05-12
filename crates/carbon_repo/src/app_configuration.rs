use chrono::NaiveDateTime;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct AppConfiguration {
    pub id: i64,
    pub theme: String,
    pub reduced_motion: bool,
    pub language: String,
    pub discord_integration: bool,
    pub release_channel: String,
    pub last_app_version: Option<String>,
    pub active_account_uuid: Option<String>,
    pub concurrent_downloads: i64,
    pub download_dependencies: bool,
    pub instances_tile_size: i64,
    pub instances_group_by: String,
    pub instances_group_by_asc: bool,
    pub instances_sort_by: String,
    pub instances_sort_by_asc: bool,
    pub show_news: bool,
    pub deletion_through_recycle_bin: bool,
    pub game_resolution: Option<String>,
    pub launcher_action_on_game_launch: String,
    pub show_app_close_warning: bool,
    pub java_custom_args: String,
    pub xmx: i64,
    pub xms: i64,
    pub default_instance_group: Option<i64>,
    pub pre_launch_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub post_exit_hook: Option<String>,
    pub is_first_launch: bool,
    pub auto_manage_java_system_profiles: bool,
    pub mod_platform_blacklist: String,
    pub mod_channels: String,
    pub random_user_uuid: String,
    pub secret: Vec<u8>,
    pub terms_and_privacy_accepted: bool,
    pub terms_and_privacy_accepted_checksum: Option<String>,
    pub metrics_enabled: bool,
    pub metrics_enabled_last_update: Option<NaiveDateTime>,
}

use sqlx::sqlite::SqlitePool;

pub struct AppConfigurationRepository {
    pool: SqlitePool,
}

impl AppConfigurationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        AppConfigurationRepository { pool }
    }

    pub async fn add_configuration(
        &self,
        release_channel: String,
        xmx: i64,
        secret: Vec<u8>,
        last_app_version: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO app_configuration (
                release_channel,
                xmx,
                secret,
                last_app_version
            ) VALUES ( ?, ?, ?, ?)",
            release_channel,
            xmx,
            secret,
            last_app_version,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_configuration(&self) -> Result<AppConfiguration, sqlx::Error> {
        let config = sqlx::query_as!(
            AppConfiguration,
            "SELECT * FROM app_configuration WHERE id = 0"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(config)
    }
}
