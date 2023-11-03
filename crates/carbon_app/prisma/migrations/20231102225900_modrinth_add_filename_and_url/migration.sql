DELETE FROM ModrinthModCache;

/*
  Warnings:

  - Added the required column `fileUrl` to the `ModrinthModCache` table without a default value. This is not possible if the table is not empty.
  - Added the required column `filename` to the `ModrinthModCache` table without a default value. This is not possible if the table is not empty.

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
    "filename" TEXT NOT NULL,
    "fileUrl" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    CONSTRAINT "ModrinthModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);
INSERT INTO "new_ModrinthModCache" ("authors", "cachedAt", "description", "metadataId", "projectId", "sha512", "title", "urlslug", "versionId") SELECT "authors", "cachedAt", "description", "metadataId", "projectId", "sha512", "title", "urlslug", "versionId" FROM "ModrinthModCache";
DROP TABLE "ModrinthModCache";
ALTER TABLE "new_ModrinthModCache" RENAME TO "ModrinthModCache";
CREATE UNIQUE INDEX "ModrinthModCache_projectId_versionId_key" ON "ModrinthModCache"("projectId", "versionId");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
