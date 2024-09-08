-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_AppConfiguration" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT DEFAULT 0,
    "theme" TEXT NOT NULL DEFAULT 'main',
    "reducedMotion" BOOLEAN NOT NULL DEFAULT false,
    "language" TEXT NOT NULL DEFAULT 'english',
    "discordIntegration" BOOLEAN NOT NULL DEFAULT true,
    "releaseChannel" TEXT NOT NULL,
    "lastAppVersion" TEXT,
    "activeAccountUuid" TEXT,
    "concurrentDownloads" INTEGER NOT NULL DEFAULT 10,
    "downloadDependencies" BOOLEAN NOT NULL DEFAULT true,
    "instancesTileSize" INTEGER NOT NULL DEFAULT 2,
    "instancesGroupBy" TEXT NOT NULL DEFAULT 'group',
    "instancesGroupByAsc" BOOLEAN NOT NULL DEFAULT true,
    "instancesSortBy" TEXT NOT NULL DEFAULT 'name',
    "instancesSortByAsc" BOOLEAN NOT NULL DEFAULT true,
    "showNews" BOOLEAN NOT NULL DEFAULT true,
    "deletionThroughRecycleBin" BOOLEAN NOT NULL DEFAULT true,
    "gameResolution" TEXT,
    "launcherActionOnGameLaunch" TEXT NOT NULL DEFAULT 'none',
    "showAppCloseWarning" BOOLEAN NOT NULL DEFAULT true,
    "javaCustomArgs" TEXT NOT NULL DEFAULT '',
    "xmx" INTEGER NOT NULL,
    "xms" INTEGER NOT NULL DEFAULT 1024,
    "defaultInstanceGroup" INTEGER,
    "preLaunchHook" TEXT,
    "wrapperCommand" TEXT,
    "postExitHook" TEXT,
    "isFirstLaunch" BOOLEAN NOT NULL DEFAULT true,
    "autoManageJavaSystemProfiles" BOOLEAN NOT NULL DEFAULT true,
    "modPlatformBlacklist" TEXT NOT NULL DEFAULT '',
    "modChannels" TEXT NOT NULL DEFAULT 'stable:true,beta:true,alpha:true',
    "randomUserUuid" TEXT NOT NULL,
    "secret" BLOB NOT NULL,
    "termsAndPrivacyAccepted" BOOLEAN NOT NULL DEFAULT false,
    "termsAndPrivacyAcceptedChecksum" TEXT,
    "metricsEnabled" BOOLEAN NOT NULL DEFAULT false,
    "metricsEnabledLastUpdate" DATETIME,
    "gdlAccountUuid" TEXT,
    "gdlAccountStatus" BLOB,
    CONSTRAINT "AppConfiguration_activeAccountUuid_fkey" FOREIGN KEY ("activeAccountUuid") REFERENCES "Account" ("uuid") ON DELETE SET NULL ON UPDATE CASCADE
);
INSERT INTO "new_AppConfiguration" ("activeAccountUuid", "autoManageJavaSystemProfiles", "concurrentDownloads", "defaultInstanceGroup", "deletionThroughRecycleBin", "discordIntegration", "downloadDependencies", "gameResolution", "id", "instancesGroupBy", "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc", "instancesTileSize", "javaCustomArgs", "language", "lastAppVersion", "launcherActionOnGameLaunch", "metricsEnabled", "metricsEnabledLastUpdate", "modChannels", "modPlatformBlacklist", "postExitHook", "preLaunchHook", "randomUserUuid", "reducedMotion", "releaseChannel", "secret", "showAppCloseWarning", "showNews", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "wrapperCommand", "xms", "xmx") SELECT "activeAccountUuid", "autoManageJavaSystemProfiles", "concurrentDownloads", "defaultInstanceGroup", "deletionThroughRecycleBin", "discordIntegration", "downloadDependencies", "gameResolution", "id", "instancesGroupBy", "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc", "instancesTileSize", "javaCustomArgs", "language", "lastAppVersion", "launcherActionOnGameLaunch", "metricsEnabled", "metricsEnabledLastUpdate", "modChannels", "modPlatformBlacklist", "postExitHook", "preLaunchHook", "randomUserUuid", "reducedMotion", "releaseChannel", "secret", "showAppCloseWarning", "showNews", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "wrapperCommand", "xms", "xmx" FROM "AppConfiguration";
DROP TABLE "AppConfiguration";
ALTER TABLE "new_AppConfiguration" RENAME TO "AppConfiguration";
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
