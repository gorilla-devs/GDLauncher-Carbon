/*
  Warnings:

  - You are about to drop the column `metricsEnabled` on the `AppConfiguration` table. All the data in the column will be lost.
  - You are about to drop the column `metricsEnabledLastUpdate` on the `AppConfiguration` table. All the data in the column will be lost.
  - You are about to drop the column `randomUserUuid` on the `AppConfiguration` table. All the data in the column will be lost.
  - You are about to drop the column `secret` on the `AppConfiguration` table. All the data in the column will be lost.

*/
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
    "isFirstLaunch" BOOLEAN NOT NULL DEFAULT true,
    "autoManageJavaSystemProfiles" BOOLEAN NOT NULL DEFAULT true,
    "modPlatformBlacklist" TEXT NOT NULL DEFAULT '',
    "modChannels" TEXT NOT NULL DEFAULT 'stable:true,beta:true,alpha:true',
    "termsAndPrivacyAccepted" BOOLEAN NOT NULL DEFAULT false,
    "termsAndPrivacyAcceptedChecksum" TEXT,
    "hashedEmailAccepted" BOOLEAN NOT NULL DEFAULT false,
    "gdlAccountUuid" TEXT,
    "gdlAccountStatus" BLOB,
    CONSTRAINT "AppConfiguration_activeAccountUuid_fkey" FOREIGN KEY ("activeAccountUuid") REFERENCES "Account" ("uuid") ON DELETE SET NULL ON UPDATE CASCADE
);
-- Temporarily remove foreign key constraints to avoid errors during data migration
PRAGMA defer_foreign_keys=ON;
INSERT INTO "new_AppConfiguration" ("activeAccountUuid", "autoManageJavaSystemProfiles", "concurrentDownloads", "defaultInstanceGroup", "deletionThroughRecycleBin", "discordIntegration", "downloadDependencies", "gameResolution", "id", "instancesGroupBy", "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc", "instancesTileSize", "isFirstLaunch", "javaCustomArgs", "language", "lastAppVersion", "launcherActionOnGameLaunch", "modChannels", "modPlatformBlacklist", "postExitHook", "preLaunchHook", "reducedMotion", "releaseChannel", "showAppCloseWarning", "showNews", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "wrapperCommand", "xms", "xmx") SELECT "activeAccountUuid", "autoManageJavaSystemProfiles", "concurrentDownloads", "defaultInstanceGroup", "deletionThroughRecycleBin", "discordIntegration", "downloadDependencies", "gameResolution", "id", "instancesGroupBy", "instancesGroupByAsc", "instancesSortBy", "instancesSortByAsc", "instancesTileSize", "isFirstLaunch", "javaCustomArgs", "language", "lastAppVersion", "launcherActionOnGameLaunch", "modChannels", "modPlatformBlacklist", "postExitHook", "preLaunchHook", "reducedMotion", "releaseChannel", "showAppCloseWarning", "showNews", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "wrapperCommand", "xms", "xmx" FROM "AppConfiguration";
PRAGMA defer_foreign_keys=OFF;
DROP TABLE "AppConfiguration";
ALTER TABLE "new_AppConfiguration" RENAME TO "AppConfiguration";
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");

-- reset since azure app id changed
UPDATE "AppConfiguration" SET "activeAccountUuid" = NULL;
DELETE FROM "Account";

PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
