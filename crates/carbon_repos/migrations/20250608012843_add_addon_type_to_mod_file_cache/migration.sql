-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_ModFileCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "instanceId" INTEGER NOT NULL,
    "filename" TEXT NOT NULL,
    "filesize" INTEGER NOT NULL,
    "enabled" BOOLEAN NOT NULL,
    "addonType" TEXT NOT NULL DEFAULT 'mods',
    "metadataId" TEXT NOT NULL,
    CONSTRAINT "ModFileCache_instanceId_fkey" FOREIGN KEY ("instanceId") REFERENCES "Instance" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "ModFileCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);
INSERT INTO "new_ModFileCache" ("enabled", "filename", "filesize", "id", "instanceId", "lastUpdatedAt", "metadataId") SELECT "enabled", "filename", "filesize", "id", "instanceId", "lastUpdatedAt", "metadataId" FROM "ModFileCache";
DROP TABLE "ModFileCache";
ALTER TABLE "new_ModFileCache" RENAME TO "ModFileCache";
CREATE UNIQUE INDEX "ModFileCache_instanceId_filename_key" ON "ModFileCache"("instanceId", "filename");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
