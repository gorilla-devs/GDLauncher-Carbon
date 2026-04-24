-- Add modloader fields to Server
ALTER TABLE "Server" ADD COLUMN "modloaderType" TEXT;
ALTER TABLE "Server" ADD COLUMN "modloaderVersion" TEXT;

-- Create ServerModFileCache table (mirrors ModFileCache but for servers)
CREATE TABLE "ServerModFileCache" (
    "id" TEXT NOT NULL PRIMARY KEY DEFAULT (hex(randomblob(16))),
    "lastUpdatedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "serverId" INTEGER NOT NULL,
    "filename" TEXT NOT NULL,
    "filesize" INTEGER NOT NULL,
    "enabled" BOOLEAN NOT NULL DEFAULT 1,
    "addonType" TEXT NOT NULL DEFAULT 'mods',
    "metadataId" TEXT NOT NULL,
    CONSTRAINT "ServerModFileCache_serverId_fkey" FOREIGN KEY ("serverId") REFERENCES "Server" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "ServerModFileCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

CREATE UNIQUE INDEX "ServerModFileCache_serverId_filename_key" ON "ServerModFileCache"("serverId", "filename");
