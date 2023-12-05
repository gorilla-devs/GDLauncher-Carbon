/*
  Warnings:

  - Added the required column `updatePaths` to the `ModrinthModCache` table without a default value. This is not possible if the table is not empty.
  - Added the required column `updatePaths` to the `CurseForgeModCache` table without a default value. This is not possible if the table is not empty.

*/
-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_ModrinthModCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "sha512" TEXT NOT NULL,
    "projectId" TEXT NOT NULL,
    "versionId" TEXT NOT NULL,
    "title" TEXT NOT NULL,
    "urlslug" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "authors" TEXT NOT NULL,
    "updatePaths" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    CONSTRAINT "ModrinthModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);
INSERT INTO "new_ModrinthModCache" ("authors", "cachedAt", "description", "metadataId", "projectId", "sha512", "title", "urlslug", "versionId") SELECT "authors", "cachedAt", "description", "metadataId", "projectId", "sha512", "title", "urlslug", "versionId" FROM "ModrinthModCache";
DROP TABLE "ModrinthModCache";
ALTER TABLE "new_ModrinthModCache" RENAME TO "ModrinthModCache";
CREATE UNIQUE INDEX "ModrinthModCache_projectId_versionId_key" ON "ModrinthModCache"("projectId", "versionId");
CREATE TABLE "new_CurseForgeModCache" (
    "metadataId" TEXT NOT NULL PRIMARY KEY,
    "murmur2" INTEGER NOT NULL,
    "projectId" INTEGER NOT NULL,
    "fileId" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "urlslug" TEXT NOT NULL,
    "summary" TEXT NOT NULL,
    "authors" TEXT NOT NULL,
    "updatePaths" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    CONSTRAINT "CurseForgeModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);
INSERT INTO "new_CurseForgeModCache" ("authors", "cachedAt", "fileId", "metadataId", "murmur2", "name", "projectId", "summary", "urlslug") SELECT "authors", "cachedAt", "fileId", "metadataId", "murmur2", "name", "projectId", "summary", "urlslug" FROM "CurseForgeModCache";
DROP TABLE "CurseForgeModCache";
ALTER TABLE "new_CurseForgeModCache" RENAME TO "CurseForgeModCache";
CREATE UNIQUE INDEX "CurseForgeModCache_projectId_fileId_key" ON "CurseForgeModCache"("projectId", "fileId");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;

-- RedefineIndex
DROP INDEX "JavaSystemP∑rofile_id_key";
CREATE UNIQUE INDEX "JavaSystemProfile_id_key" ON "JavaSystemProfile"("id");
