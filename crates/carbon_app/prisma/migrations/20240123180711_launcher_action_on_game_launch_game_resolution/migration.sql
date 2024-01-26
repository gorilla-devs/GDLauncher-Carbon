/*
  Warnings:

  - You are about to drop the column `startupResolution` on the `AppConfiguration` table. All the data in the column will be lost.

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
    "activeAccountUuid" TEXT,
    "concurrentDownloads" INTEGER NOT NULL DEFAULT 8,
    "showNews" BOOLEAN NOT NULL DEFAULT true,
    "gameResolution" TEXT,
    "launcherActionOnGameLaunch" TEXT NOT NULL DEFAULT 'none',
    "javaCustomArgs" TEXT NOT NULL DEFAULT '',
    "xmx" INTEGER NOT NULL,
    "xms" INTEGER NOT NULL DEFAULT 1024,
    "defaultInstanceGroup" INTEGER,
    "isFirstLaunch" BOOLEAN NOT NULL DEFAULT true,
    "autoManageJava" BOOLEAN NOT NULL DEFAULT true,
    "modPlatformBlacklist" TEXT NOT NULL DEFAULT '',
    "modChannels" TEXT NOT NULL DEFAULT 'stable:true,beta:true,alpha:true',
    "randomUserUuid" TEXT NOT NULL,
    "secret" BLOB NOT NULL,
    "termsAndPrivacyAccepted" BOOLEAN NOT NULL DEFAULT false,
    "termsAndPrivacyAcceptedChecksum" TEXT,
    "metricsEnabled" BOOLEAN NOT NULL DEFAULT false,
    "metricsEnabledLastUpdate" DATETIME
);
INSERT INTO "new_AppConfiguration" ("activeAccountUuid", "autoManageJava", "concurrentDownloads", "defaultInstanceGroup", "discordIntegration", "id", "isFirstLaunch", "javaCustomArgs", "language", "metricsEnabled", "metricsEnabledLastUpdate", "modChannels", "modPlatformBlacklist", "randomUserUuid", "reducedMotion", "releaseChannel", "secret", "showNews", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "xms", "xmx") SELECT "activeAccountUuid", "autoManageJava", "concurrentDownloads", "defaultInstanceGroup", "discordIntegration", "id", "isFirstLaunch", "javaCustomArgs", "language", "metricsEnabled", "metricsEnabledLastUpdate", "modChannels", "modPlatformBlacklist", "randomUserUuid", "reducedMotion", "releaseChannel", "secret", "showNews", "termsAndPrivacyAccepted", "termsAndPrivacyAcceptedChecksum", "theme", "xms", "xmx" FROM "AppConfiguration";
DROP TABLE "AppConfiguration";
ALTER TABLE "new_AppConfiguration" RENAME TO "AppConfiguration";
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
