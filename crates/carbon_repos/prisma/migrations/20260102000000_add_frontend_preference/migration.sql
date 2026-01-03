-- Create FrontendPreference table for storing generic KV frontend preferences
CREATE TABLE "FrontendPreference" (
    "key" TEXT NOT NULL PRIMARY KEY,
    "value" TEXT NOT NULL,
    "updatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Migrate existing data to FrontendPreference before dropping columns

-- Migrate isFirstLaunch=false to FrontendPreference (meaning first launch is completed)
INSERT OR IGNORE INTO "FrontendPreference" ("key", "value", "updatedAt")
SELECT 'first_launch_completed', 'true', datetime('now')
FROM "AppConfiguration"
WHERE "isFirstLaunch" = 0 AND "id" = 0;

-- Migrate lastAppVersion to FrontendPreference
INSERT OR IGNORE INTO "FrontendPreference" ("key", "value", "updatedAt")
SELECT 'last_seen_version', "lastAppVersion", datetime('now')
FROM "AppConfiguration"
WHERE "lastAppVersion" IS NOT NULL AND "id" = 0;

-- RedefineTables to drop lastAppVersion and isFirstLaunch columns
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_AppConfiguration" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT DEFAULT 0,
    "theme" TEXT NOT NULL DEFAULT 'main',
    "reducedMotion" BOOLEAN NOT NULL DEFAULT false,
    "language" TEXT NOT NULL DEFAULT 'english',
    "discordIntegration" BOOLEAN NOT NULL DEFAULT true,
    "releaseChannel" TEXT NOT NULL,
    "activeAccountUuid" TEXT,
    "concurrentDownloads" INTEGER NOT NULL DEFAULT 10,
    "downloadDependencies" BOOLEAN NOT NULL DEFAULT true,
    "instancesTileSize" INTEGER NOT NULL DEFAULT 2,
    "instancesGroupBy" TEXT NOT NULL DEFAULT 'group',
    "instancesGroupByAsc" BOOLEAN NOT NULL DEFAULT true,
    "instancesSortBy" TEXT NOT NULL DEFAULT 'created',
    "instancesSortByAsc" BOOLEAN NOT NULL DEFAULT false,
    "showFeatured" BOOLEAN NOT NULL DEFAULT true,
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
    "autoManageJavaSystemProfiles" BOOLEAN NOT NULL DEFAULT true,
    "modPlatformBlacklist" TEXT NOT NULL DEFAULT '',
    "modChannels" TEXT NOT NULL DEFAULT 'stable:true,beta:true,alpha:true',
    "termsAndPrivacyAccepted" BOOLEAN NOT NULL DEFAULT false,
    "termsAndPrivacyAcceptedChecksum" TEXT,
    "gdlAccountUuid" TEXT,
    "gdlAccountStatus" BLOB,
    "installationId" TEXT,
    CONSTRAINT "AppConfiguration_activeAccountUuid_fkey" FOREIGN KEY ("activeAccountUuid") REFERENCES "Account" ("uuid") ON DELETE SET NULL ON UPDATE CASCADE
);
INSERT INTO "new_AppConfiguration" ("activeAccountUuid", "autoManageJavaSystemProfiles", "concurrentDownloads", "defaultInstanceGroup", "deletionThroughRecycleBin", "discordIntegration", "downloadDependencies", "gameResolution", "gdlAccountStatus", "gdlAccountUuid", "id", "instancesGroupBy", "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc", "instancesTileSize", "javaCustomArgs", "language", "launcherActionOnGameLaunch", "modChannels", "modPlatformBlacklist", "postExitHook", "preLaunchHook", "reducedMotion", "releaseChannel", "showAppCloseWarning", "showFeatured", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "wrapperCommand", "xms", "xmx", "installationId") SELECT "activeAccountUuid", "autoManageJavaSystemProfiles", "concurrentDownloads", "defaultInstanceGroup", "deletionThroughRecycleBin", "discordIntegration", "downloadDependencies", "gameResolution", "gdlAccountStatus", "gdlAccountUuid", "id", "instancesGroupBy", "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc", "instancesTileSize", "javaCustomArgs", "language", "launcherActionOnGameLaunch", "modChannels", "modPlatformBlacklist", "postExitHook", "preLaunchHook", "reducedMotion", "releaseChannel", "showAppCloseWarning", "showFeatured", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "wrapperCommand", "xms", "xmx", "installationId" FROM "AppConfiguration";
DROP TABLE "AppConfiguration";
ALTER TABLE "new_AppConfiguration" RENAME TO "AppConfiguration";
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");

PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
