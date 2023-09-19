/*
  Warnings:

  - You are about to drop the column `logoImage` on the `ModMetadata` table. All the data in the column will be lost.
  - The primary key for the `CurseForgeModCache` table will be changed. If it partially fails, the table could be left without primary key constraint.
  - You are about to drop the column `id` on the `CurseForgeModCache` table. All the data in the column will be lost.
  - The primary key for the `ModrinthModCache` table will be changed. If it partially fails, the table could be left without primary key constraint.
  - You are about to drop the column `filename` on the `ModrinthModCache` table. All the data in the column will be lost.
  - You are about to drop the column `id` on the `ModrinthModCache` table. All the data in the column will be lost.
  - You are about to drop the column `sha1` on the `ModrinthModCache` table. All the data in the column will be lost.

*/

-- (manually added) clear all the caches
DELETE FROM ModFileCache;
DELETE FROM ModMetadata;
DELETE FROM CurseForgeModCache;
DELETE FROM ModrinthModCache;

-- CreateTable
CREATE TABLE "LocalModImageCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "data" BLOB NOT NULL,
    CONSTRAINT "LocalModImageCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "CurseForgeModImageCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "url" TEXT NOT NULL,
    "data" BLOB,
    "upToDate" INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT "CurseForgeModImageCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "CurseForgeModCache" ("metadataId") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "ModrinthModImageCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "url" TEXT NOT NULL,
    "data" BLOB,
    "upToDate" INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT "ModrinthModImageCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModrinthModCache" ("metadataId") ON DELETE CASCADE ON UPDATE CASCADE
);

-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_ModMetadata" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "murmur2" INTEGER NOT NULL,
    "sha512" BLOB NOT NULL,
    "name" TEXT,
    "modid" TEXT,
    "version" TEXT,
    "description" TEXT,
    "authors" TEXT,
    "modloaders" TEXT NOT NULL
);
INSERT INTO "new_ModMetadata" ("authors", "description", "id", "modid", "modloaders", "murmur2", "name", "sha512", "version") SELECT "authors", "description", "id", "modid", "modloaders", "murmur2", "name", "sha512", "version" FROM "ModMetadata";
DROP TABLE "ModMetadata";
ALTER TABLE "new_ModMetadata" RENAME TO "ModMetadata";
CREATE TABLE "new_CurseForgeModCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "murmur2" INTEGER NOT NULL,
    "projectId" INTEGER NOT NULL,
    "fileId" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "urlslug" TEXT NOT NULL,
    "summary" TEXT NOT NULL,
    "authors" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    CONSTRAINT "CurseForgeModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);
INSERT INTO "new_CurseForgeModCache" ("authors", "cachedAt", "fileId", "metadataId", "murmur2", "name", "projectId", "summary", "urlslug") SELECT "authors", "cachedAt", "fileId", "metadataId", "murmur2", "name", "projectId", "summary", "urlslug" FROM "CurseForgeModCache";
DROP TABLE "CurseForgeModCache";
ALTER TABLE "new_CurseForgeModCache" RENAME TO "CurseForgeModCache";
CREATE UNIQUE INDEX "CurseForgeModCache_projectId_fileId_key" ON "CurseForgeModCache"("projectId", "fileId");
CREATE TABLE "new_ModrinthModCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "sha512" TEXT NOT NULL,
    "projectId" TEXT NOT NULL,
    "versionId" TEXT NOT NULL,
    "title" TEXT NOT NULL,
    "urlslug" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "authors" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    CONSTRAINT "ModrinthModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);
INSERT INTO "new_ModrinthModCache" ("authors", "cachedAt", "description", "metadataId", "projectId", "sha512", "title", "urlslug", "versionId") SELECT "authors", "cachedAt", "description", "metadataId", "projectId", "sha512", "title", "urlslug", "versionId" FROM "ModrinthModCache";
DROP TABLE "ModrinthModCache";
ALTER TABLE "new_ModrinthModCache" RENAME TO "ModrinthModCache";
CREATE UNIQUE INDEX "ModrinthModCache_projectId_versionId_key" ON "ModrinthModCache"("projectId", "versionId");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
