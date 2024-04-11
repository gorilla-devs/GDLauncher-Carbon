-- AlterTable
ALTER TABLE "AppConfiguration" ADD COLUMN "lastAppVersion" TEXT;

-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_ModFileCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "instanceId" INTEGER NOT NULL,
    "filename" TEXT NOT NULL,
    "filesize" INTEGER NOT NULL,
    "enabled" BOOLEAN NOT NULL,
    "metadataId" TEXT NOT NULL,
    CONSTRAINT "ModFileCache_instanceId_fkey" FOREIGN KEY ("instanceId") REFERENCES "Instance" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "ModFileCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);
INSERT INTO "new_ModFileCache" ("enabled", "filename", "filesize", "id", "instanceId", "metadataId") SELECT "enabled", "filename", "filesize", "id", "instanceId", "metadataId" FROM "ModFileCache";
DROP TABLE "ModFileCache";
ALTER TABLE "new_ModFileCache" RENAME TO "ModFileCache";
CREATE UNIQUE INDEX "ModFileCache_instanceId_filename_key" ON "ModFileCache"("instanceId", "filename");
CREATE TABLE "new_VersionInfoCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "versionInfo" BLOB NOT NULL
);
INSERT INTO "new_VersionInfoCache" ("id", "versionInfo") SELECT "id", "versionInfo" FROM "VersionInfoCache";
DROP TABLE "VersionInfoCache";
ALTER TABLE "new_VersionInfoCache" RENAME TO "VersionInfoCache";
CREATE TABLE "new_LwjglMetaCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lwjgl" BLOB NOT NULL
);
INSERT INTO "new_LwjglMetaCache" ("id", "lwjgl") SELECT "id", "lwjgl" FROM "LwjglMetaCache";
DROP TABLE "LwjglMetaCache";
ALTER TABLE "new_LwjglMetaCache" RENAME TO "LwjglMetaCache";
CREATE TABLE "new_ModMetadata" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "murmur2" INTEGER NOT NULL,
    "sha512" BLOB NOT NULL,
    "sha1" BLOB NOT NULL,
    "name" TEXT,
    "modid" TEXT,
    "version" TEXT,
    "description" TEXT,
    "authors" TEXT,
    "modloaders" TEXT NOT NULL
);
INSERT INTO "new_ModMetadata" ("authors", "description", "id", "modid", "modloaders", "murmur2", "name", "sha1", "sha512", "version") SELECT "authors", "description", "id", "modid", "modloaders", "murmur2", "name", "sha1", "sha512", "version" FROM "ModMetadata";
DROP TABLE "ModMetadata";
ALTER TABLE "new_ModMetadata" RENAME TO "ModMetadata";
CREATE TABLE "new_AssetsMetaCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "assetsIndex" BLOB NOT NULL
);
INSERT INTO "new_AssetsMetaCache" ("assetsIndex", "id") SELECT "assetsIndex", "id" FROM "AssetsMetaCache";
DROP TABLE "AssetsMetaCache";
ALTER TABLE "new_AssetsMetaCache" RENAME TO "AssetsMetaCache";
CREATE TABLE "new_PartialVersionInfoCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "partialVersionInfo" BLOB NOT NULL
);
INSERT INTO "new_PartialVersionInfoCache" ("id", "partialVersionInfo") SELECT "id", "partialVersionInfo" FROM "PartialVersionInfoCache";
DROP TABLE "PartialVersionInfoCache";
ALTER TABLE "new_PartialVersionInfoCache" RENAME TO "PartialVersionInfoCache";
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
