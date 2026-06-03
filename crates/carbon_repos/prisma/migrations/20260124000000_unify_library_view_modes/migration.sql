-- UnifyLibraryViewModes migration
-- Convert instancesGroupBy = "group" to NULL (null = folders mode)
-- Convert instancesSortBy = "manual" to NULL (null = manual order)
-- Add instancesDuplicateFavorites column
-- Make instancesGroupBy and instancesSortBy nullable

-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table

PRAGMA foreign_keys=OFF;

-- Step 1: Create new table with updated schema
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
    "instancesGroupBy" TEXT,
    "instancesGroupByAsc" BOOLEAN NOT NULL DEFAULT true,
    "instancesSortBy" TEXT,
    "instancesSortByAsc" BOOLEAN NOT NULL DEFAULT false,
    "instancesDuplicateFavorites" BOOLEAN NOT NULL DEFAULT true,
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

-- Step 2: Copy data, converting 'group' to NULL for instancesGroupBy and 'manual' to NULL for instancesSortBy
INSERT INTO "new_AppConfiguration" (
    "id", "theme", "reducedMotion", "language", "discordIntegration",
    "releaseChannel", "activeAccountUuid", "concurrentDownloads",
    "downloadDependencies", "instancesTileSize", "instancesGroupBy",
    "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc",
    "instancesDuplicateFavorites", "showFeatured", "deletionThroughRecycleBin",
    "gameResolution", "launcherActionOnGameLaunch", "showAppCloseWarning",
    "javaCustomArgs", "xmx", "xms", "defaultInstanceGroup",
    "preLaunchHook", "wrapperCommand", "postExitHook",
    "autoManageJavaSystemProfiles", "modPlatformBlacklist", "modChannels",
    "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum",
    "gdlAccountUuid", "gdlAccountStatus", "installationId"
)
SELECT
    "id", "theme", "reducedMotion", "language", "discordIntegration",
    "releaseChannel", "activeAccountUuid", "concurrentDownloads",
    "downloadDependencies", "instancesTileSize",
    CASE WHEN "instancesGroupBy" = 'group' THEN NULL ELSE "instancesGroupBy" END,
    "instancesGroupByAsc",
    CASE WHEN "instancesSortBy" = 'manual' THEN NULL ELSE "instancesSortBy" END,
    "instancesSortByAsc",
    true,
    "showFeatured", "deletionThroughRecycleBin",
    "gameResolution", "launcherActionOnGameLaunch", "showAppCloseWarning",
    "javaCustomArgs", "xmx", "xms", "defaultInstanceGroup",
    "preLaunchHook", "wrapperCommand", "postExitHook",
    "autoManageJavaSystemProfiles", "modPlatformBlacklist", "modChannels",
    "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum",
    "gdlAccountUuid", "gdlAccountStatus", "installationId"
FROM "AppConfiguration";

-- Step 3: Drop old table
DROP TABLE "AppConfiguration";

-- Step 4: Rename new table
ALTER TABLE "new_AppConfiguration" RENAME TO "AppConfiguration";

-- Step 5: Recreate index
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");

PRAGMA foreign_keys=ON;
