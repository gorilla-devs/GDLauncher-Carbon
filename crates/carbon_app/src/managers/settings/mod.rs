use self::terms_and_privacy::TermsAndPrivacy;
use super::ManagerRef;
use crate::api::{keys::settings::*, settings::FESettingsUpdate};
use anyhow::anyhow;
use carbon_platforms::{ModChannelWithUsage, ModPlatform};
use carbon_repos::{models::AppConfiguration, queries};
use itertools::Itertools;
use reqwest_middleware::ClientWithMiddleware;
use std::path::PathBuf;

pub mod terms_and_privacy;

pub(crate) struct SettingsManager {
    pub runtime_path: carbon_rt_path::RuntimePath,
    pub terms_and_privacy: TermsAndPrivacy,
    pub gdl_base_api_url: String,
    pub latest_consent_checksum: Option<String>,
}

impl SettingsManager {
    pub fn new(
        runtime_path: PathBuf,
        http_client: ClientWithMiddleware,
        gdl_base_api_url: String,
        latest_consent_checksum: Option<String>,
    ) -> Self {
        Self {
            runtime_path: carbon_rt_path::RuntimePath::new(runtime_path),
            terms_and_privacy: TermsAndPrivacy::new(http_client, gdl_base_api_url.clone()),
            gdl_base_api_url,
            latest_consent_checksum,
        }
    }
}

impl ManagerRef<'_, SettingsManager> {
    pub async fn get_settings(self) -> anyhow::Result<AppConfiguration> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::GetSettings::fetch_one(&conn)
                .map_err(|e| anyhow!("Failed to get settings: {}", e))
        })
        .await?
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_settings(self, incoming_settings: FESettingsUpdate) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        let latest_consent_checksum = self.latest_consent_checksum.clone();

        // Collect all the values we need to update
        let theme = incoming_settings.theme.map(|s| s.inner());
        let language = incoming_settings.language.map(|s| s.inner());
        let reduced_motion = incoming_settings.reduced_motion.map(|s| s.inner());
        let discord_integration = incoming_settings.discord_integration.map(|s| s.inner());
        let release_channel = incoming_settings
            .release_channel
            .map(|s| -> String { s.inner().into() });
        let launcher_action_on_game_launch = incoming_settings
            .launcher_action_on_game_launch
            .map(|s| -> String { s.inner().into() });
        let show_app_close_warning = incoming_settings
            .show_app_close_warning
            .clone()
            .map(|s| s.inner());
        let last_app_version = incoming_settings
            .last_app_version
            .clone()
            .map(|s| s.inner());
        let concurrent_downloads = incoming_settings.concurrent_downloads.map(|s| s.inner());
        let download_dependencies = incoming_settings.download_dependencies.map(|s| s.inner());
        let show_featured = incoming_settings.show_featured.map(|s| s.inner());
        let instances_sort_by = incoming_settings
            .instances_sort_by
            .map(|s| -> String { s.inner().into() });
        let instances_sort_by_asc = incoming_settings.instances_sort_by_asc.map(|s| s.inner());
        let instances_group_by = incoming_settings
            .instances_group_by
            .map(|s| -> String { s.inner().into() });
        let instances_group_by_asc = incoming_settings.instances_group_by_asc.map(|s| s.inner());
        let instances_tile_size = incoming_settings.instances_tile_size.map(|s| s.inner());
        let deletion_through_recycle_bin = incoming_settings
            .deletion_through_recycle_bin
            .map(|s| s.inner());
        let xmx = incoming_settings.xmx.map(|s| s.inner());
        let xms = incoming_settings.xms.map(|s| s.inner());
        let is_first_launch = incoming_settings.is_first_launch.map(|s| s.inner());
        let game_resolution = incoming_settings
            .game_resolution
            .map(|s| s.inner().map(|r| -> String { r.into() }));
        let java_custom_args = incoming_settings.java_custom_args.map(|s| s.inner());
        let pre_launch_hook = incoming_settings.pre_launch_hook.map(|s| s.inner());
        let post_exit_hook = incoming_settings.post_exit_hook.map(|s| s.inner());
        let wrapper_command = incoming_settings.wrapper_command.map(|s| s.inner());
        let auto_manage_java_system_profiles = incoming_settings
            .auto_manage_java_system_profiles
            .as_ref()
            .map(|s| s.clone().inner());
        let terms_and_privacy_accepted = incoming_settings
            .terms_and_privacy_accepted
            .map(|s| s.inner());

        // Process mod_sources
        let mod_sources = incoming_settings.mod_sources.map(|s| {
            let mod_sources = s.inner();
            let platform_blacklist = mod_sources
                .platform_blacklist
                .into_iter()
                .map(ModPlatform::from)
                .map(|p| ModPlatform::as_str(&p))
                .join(",");

            let channels = mod_sources
                .channels
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>();
            ModChannelWithUsage::validate_list(&channels)?;

            let channels_str = ModChannelWithUsage::slice_to_str(&channels);
            Ok::<_, anyhow::Error>((platform_blacklist, channels_str))
        });

        // Check if mod_sources is valid before proceeding
        let mod_sources = match mod_sources {
            Some(Ok(v)) => Some(v),
            Some(Err(e)) => return Err(e),
            None => None,
        };

        let has_updates = theme.is_some()
            || language.is_some()
            || reduced_motion.is_some()
            || discord_integration.is_some()
            || release_channel.is_some()
            || launcher_action_on_game_launch.is_some()
            || show_app_close_warning.is_some()
            || last_app_version.is_some()
            || concurrent_downloads.is_some()
            || download_dependencies.is_some()
            || show_featured.is_some()
            || instances_sort_by.is_some()
            || instances_sort_by_asc.is_some()
            || instances_group_by.is_some()
            || instances_group_by_asc.is_some()
            || instances_tile_size.is_some()
            || deletion_through_recycle_bin.is_some()
            || xmx.is_some()
            || xms.is_some()
            || is_first_launch.is_some()
            || game_resolution.is_some()
            || java_custom_args.is_some()
            || pre_launch_hook.is_some()
            || post_exit_hook.is_some()
            || wrapper_command.is_some()
            || auto_manage_java_system_profiles.is_some()
            || mod_sources.is_some()
            || terms_and_privacy_accepted.is_some();

        if has_updates {
            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;

                if let Some(theme) = theme {
                    queries::settings::UpdateTheme::execute(&conn, &theme)?;
                }

                if let Some(language) = language {
                    queries::settings::UpdateLanguage::execute(&conn, &language)?;
                }

                if let Some(reduced_motion) = reduced_motion {
                    queries::settings::UpdateReducedMotion::execute(&conn, reduced_motion)?;
                }

                if let Some(discord_integration) = discord_integration {
                    queries::settings::UpdateDiscordIntegration::execute(
                        &conn,
                        discord_integration,
                    )?;
                }

                if let Some(release_channel) = release_channel {
                    queries::settings::UpdateReleaseChannel::execute(&conn, &release_channel)?;
                }

                if let Some(launcher_action_on_game_launch) = launcher_action_on_game_launch {
                    queries::settings::UpdateLauncherActionOnGameLaunch::execute(
                        &conn,
                        &launcher_action_on_game_launch,
                    )?;
                }

                if let Some(show_app_close_warning) = show_app_close_warning {
                    queries::settings::UpdateShowAppCloseWarning::execute(
                        &conn,
                        show_app_close_warning,
                    )?;
                }

                if let Some(last_app_version) = last_app_version {
                    queries::settings::UpdateLastAppVersion::execute(
                        &conn,
                        last_app_version.as_deref(),
                    )?;
                }

                if let Some(concurrent_downloads) = concurrent_downloads {
                    queries::settings::UpdateConcurrentDownloads::execute(
                        &conn,
                        concurrent_downloads,
                    )?;
                }

                if let Some(download_dependencies) = download_dependencies {
                    queries::settings::UpdateDownloadDependencies::execute(
                        &conn,
                        download_dependencies,
                    )?;
                }

                if let Some(show_featured) = show_featured {
                    queries::settings::UpdateShowFeatured::execute(&conn, show_featured)?;
                }

                if let Some(instances_sort_by) = instances_sort_by {
                    queries::settings::UpdateInstancesSortBy::execute(&conn, &instances_sort_by)?;
                }

                if let Some(instances_sort_by_asc) = instances_sort_by_asc {
                    queries::settings::UpdateInstancesSortByAsc::execute(
                        &conn,
                        instances_sort_by_asc,
                    )?;
                }

                if let Some(instances_group_by) = instances_group_by {
                    queries::settings::UpdateInstancesGroupBy::execute(&conn, &instances_group_by)?;
                }

                if let Some(instances_group_by_asc) = instances_group_by_asc {
                    queries::settings::UpdateInstancesGroupByAsc::execute(
                        &conn,
                        instances_group_by_asc,
                    )?;
                }

                if let Some(instances_tile_size) = instances_tile_size {
                    queries::settings::UpdateInstancesTileSize::execute(
                        &conn,
                        instances_tile_size,
                    )?;
                }

                if let Some(deletion_through_recycle_bin) = deletion_through_recycle_bin {
                    queries::settings::UpdateDeletionThroughRecycleBin::execute(
                        &conn,
                        deletion_through_recycle_bin,
                    )?;
                }

                if let Some(xmx) = xmx {
                    queries::settings::UpdateXmx::execute(&conn, xmx)?;
                }

                if let Some(xms) = xms {
                    queries::settings::UpdateXms::execute(&conn, xms)?;
                }

                if let Some(is_first_launch) = is_first_launch {
                    queries::settings::UpdateIsFirstLaunch::execute(&conn, is_first_launch)?;
                }

                if let Some(game_resolution) = game_resolution {
                    queries::settings::UpdateGameResolution::execute(
                        &conn,
                        game_resolution.as_deref(),
                    )?;
                }

                if let Some(java_custom_args) = java_custom_args {
                    queries::settings::UpdateJavaCustomArgs::execute(&conn, &java_custom_args)?;
                }

                if let Some(pre_launch_hook) = pre_launch_hook {
                    queries::settings::UpdatePreLaunchHook::execute(
                        &conn,
                        pre_launch_hook.as_deref(),
                    )?;
                }

                if let Some(post_exit_hook) = post_exit_hook {
                    queries::settings::UpdatePostExitHook::execute(
                        &conn,
                        post_exit_hook.as_deref(),
                    )?;
                }

                if let Some(wrapper_command) = wrapper_command {
                    queries::settings::UpdateWrapperCommand::execute(
                        &conn,
                        wrapper_command.as_deref(),
                    )?;
                }

                if let Some(auto_manage_java_system_profiles) = auto_manage_java_system_profiles {
                    queries::settings::UpdateAutoManageJavaSystemProfiles::execute(
                        &conn,
                        auto_manage_java_system_profiles,
                    )?;
                }

                if let Some((platform_blacklist, channels_str)) = mod_sources {
                    queries::settings::UpdateModSources::execute(
                        &conn,
                        &platform_blacklist,
                        &channels_str,
                    )?;
                }

                if terms_and_privacy_accepted.is_some() {
                    // We default to empty value in case our APIs fail so we don't block the user.
                    // We are gonna ask again on next run anyway once the APIs are back up
                    let latest_consent_sha = latest_consent_checksum
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_default();

                    queries::settings::UpdateTermsAndPrivacyFull::execute(
                        &conn,
                        true,
                        Some(latest_consent_sha.as_str()),
                    )?;
                }

                Ok::<_, anyhow::Error>(())
            })
            .await??;

            self.app.invalidate(GET_SETTINGS, None);

            if let Some(show_app_close_warning) = incoming_settings.show_app_close_warning {
                println!(
                    "_SHOW_APP_CLOSE_WARNING_:{}",
                    show_app_close_warning.inner()
                );
            }
        }

        if let Some(auto_manage_java_system_profiles) =
            incoming_settings.auto_manage_java_system_profiles
        {
            if auto_manage_java_system_profiles.inner() {
                // TODO: Migrate Java domain to rusqlite, then re-enable this call
                // super::java::scan_and_sync::sync_system_java_profiles(db).await?;
                tracing::warn!(
                    "sync_system_java_profiles is temporarily disabled during rusqlite migration"
                );
            }
        }

        Ok(())
    }

    pub async fn set_theme(self, theme: String) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateTheme::execute(&conn, &theme)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_language(self, language: String) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateLanguage::execute(&conn, &language)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_reduced_motion(self, reduced_motion: bool) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateReducedMotion::execute(&conn, reduced_motion)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_discord_integration(self, discord_integration: bool) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateDiscordIntegration::execute(&conn, discord_integration)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_release_channel(self, release_channel: String) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateReleaseChannel::execute(&conn, &release_channel)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_last_app_version(
        self,
        last_app_version: Option<String>,
    ) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateLastAppVersion::execute(&conn, last_app_version.as_deref())?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_concurrent_downloads(self, concurrent_downloads: i32) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateConcurrentDownloads::execute(&conn, concurrent_downloads)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_xmx(self, xmx: i32) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateXmx::execute(&conn, xmx)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_xms(self, xms: i32) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateXms::execute(&conn, xms)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_is_first_launch(self, is_first_launch: bool) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateIsFirstLaunch::execute(&conn, is_first_launch)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_auto_manage_java_system_profiles(
        self,
        auto_manage_java_system_profiles: bool,
    ) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateAutoManageJavaSystemProfiles::execute(
                &conn,
                auto_manage_java_system_profiles,
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_active_account_uuid(self, uuid: Option<String>) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateActiveAccountUuid::execute(&conn, uuid.as_deref())?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_gdl_account_uuid(self, uuid: Option<String>) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateGdlAccountUuid::execute(&conn, uuid.as_deref())?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn set_gdl_account_status(self, status: Option<Vec<u8>>) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::settings::UpdateGdlAccountStatus::execute(&conn, status.as_deref())?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }
}
