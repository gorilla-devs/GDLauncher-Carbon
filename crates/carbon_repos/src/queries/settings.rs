//! Settings (AppConfiguration) queries.

use crate::define_query;
use crate::models::AppConfiguration;

// Read queries - typed
define_query!(
    GetSettings,
    "SELECT * FROM AppConfiguration WHERE id = 0",
    query_row() -> AppConfiguration
);
define_query!(CountSettings, "SELECT COUNT(*) FROM AppConfiguration");

// Create query (for initial seeding) - typed
define_query!(
    CreateSettings,
    r#"INSERT INTO AppConfiguration (
        id, releaseChannel, xmx, lastAppVersion
    ) VALUES (0, ?1, ?2, ?3)"#,
    execute(release_channel: &str, xmx: i32, last_app_version: Option<&str>)
);

// Individual field update queries - typed
define_query!(
    UpdateTheme,
    "UPDATE AppConfiguration SET theme = ?1 WHERE id = 0",
    execute(theme: &str)
);
define_query!(
    UpdateReducedMotion,
    "UPDATE AppConfiguration SET reducedMotion = ?1 WHERE id = 0"
);
define_query!(
    UpdateLanguage,
    "UPDATE AppConfiguration SET language = ?1 WHERE id = 0"
);
define_query!(
    UpdateDiscordIntegration,
    "UPDATE AppConfiguration SET discordIntegration = ?1 WHERE id = 0"
);
define_query!(
    UpdateReleaseChannel,
    "UPDATE AppConfiguration SET releaseChannel = ?1 WHERE id = 0"
);
define_query!(
    UpdateLastAppVersion,
    "UPDATE AppConfiguration SET lastAppVersion = ?1 WHERE id = 0"
);
define_query!(
    UpdateActiveAccountUuid,
    "UPDATE AppConfiguration SET activeAccountUuid = ?1 WHERE id = 0"
);
define_query!(
    UpdateConcurrentDownloads,
    "UPDATE AppConfiguration SET concurrentDownloads = ?1 WHERE id = 0"
);
define_query!(
    UpdateDownloadDependencies,
    "UPDATE AppConfiguration SET downloadDependencies = ?1 WHERE id = 0"
);
define_query!(
    UpdateInstancesTileSize,
    "UPDATE AppConfiguration SET instancesTileSize = ?1 WHERE id = 0"
);
define_query!(
    UpdateInstancesGroupBy,
    "UPDATE AppConfiguration SET instancesGroupBy = ?1 WHERE id = 0"
);
define_query!(
    UpdateInstancesGroupByAsc,
    "UPDATE AppConfiguration SET instancesGroupByAsc = ?1 WHERE id = 0"
);
define_query!(
    UpdateInstancesSortBy,
    "UPDATE AppConfiguration SET instancesSortBy = ?1 WHERE id = 0"
);
define_query!(
    UpdateInstancesSortByAsc,
    "UPDATE AppConfiguration SET instancesSortByAsc = ?1 WHERE id = 0"
);
define_query!(
    UpdateShowFeatured,
    "UPDATE AppConfiguration SET showFeatured = ?1 WHERE id = 0"
);
define_query!(
    UpdateDeletionThroughRecycleBin,
    "UPDATE AppConfiguration SET deletionThroughRecycleBin = ?1 WHERE id = 0"
);
define_query!(
    UpdateGameResolution,
    "UPDATE AppConfiguration SET gameResolution = ?1 WHERE id = 0"
);
define_query!(
    UpdateLauncherActionOnGameLaunch,
    "UPDATE AppConfiguration SET launcherActionOnGameLaunch = ?1 WHERE id = 0"
);
define_query!(
    UpdateShowAppCloseWarning,
    "UPDATE AppConfiguration SET showAppCloseWarning = ?1 WHERE id = 0"
);
define_query!(
    UpdateJavaCustomArgs,
    "UPDATE AppConfiguration SET javaCustomArgs = ?1 WHERE id = 0"
);
define_query!(
    UpdateXmx,
    "UPDATE AppConfiguration SET xmx = ?1 WHERE id = 0"
);
define_query!(
    UpdateXms,
    "UPDATE AppConfiguration SET xms = ?1 WHERE id = 0"
);
define_query!(
    UpdateDefaultInstanceGroup,
    "UPDATE AppConfiguration SET defaultInstanceGroup = ?1 WHERE id = 0"
);
define_query!(
    UpdatePreLaunchHook,
    "UPDATE AppConfiguration SET preLaunchHook = ?1 WHERE id = 0"
);
define_query!(
    UpdateWrapperCommand,
    "UPDATE AppConfiguration SET wrapperCommand = ?1 WHERE id = 0"
);
define_query!(
    UpdatePostExitHook,
    "UPDATE AppConfiguration SET postExitHook = ?1 WHERE id = 0"
);
define_query!(
    UpdateIsFirstLaunch,
    "UPDATE AppConfiguration SET isFirstLaunch = ?1 WHERE id = 0"
);
define_query!(
    UpdateAutoManageJavaSystemProfiles,
    "UPDATE AppConfiguration SET autoManageJavaSystemProfiles = ?1 WHERE id = 0"
);
define_query!(
    UpdateModPlatformBlacklist,
    "UPDATE AppConfiguration SET modPlatformBlacklist = ?1 WHERE id = 0"
);
define_query!(
    UpdateModChannels,
    "UPDATE AppConfiguration SET modChannels = ?1 WHERE id = 0"
);
define_query!(
    UpdateModSources,
    "UPDATE AppConfiguration SET modPlatformBlacklist = ?1, modChannels = ?2 WHERE id = 0"
);
define_query!(
    UpdateTermsAndPrivacyAccepted,
    "UPDATE AppConfiguration SET termsAndPrivacyAccepted = ?1 WHERE id = 0"
);
define_query!(
    UpdateTermsAndPrivacyAcceptedChecksum,
    "UPDATE AppConfiguration SET termsAndPrivacyAcceptedChecksum = ?1 WHERE id = 0"
);
define_query!(
    UpdateTermsAndPrivacyFull,
    "UPDATE AppConfiguration SET termsAndPrivacyAccepted = ?1, termsAndPrivacyAcceptedChecksum = ?2 WHERE id = 0"
);
define_query!(
    UpdateGdlAccountUuid,
    "UPDATE AppConfiguration SET gdlAccountUuid = ?1 WHERE id = 0"
);
define_query!(
    UpdateGdlAccountStatus,
    "UPDATE AppConfiguration SET gdlAccountStatus = ?1 WHERE id = 0"
);
