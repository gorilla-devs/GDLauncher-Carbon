-- Add server support tables and configuration

-- Create ServerGroup table
CREATE TABLE "ServerGroup" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "groupIndex" INTEGER NOT NULL,
    "libraryPosition" INTEGER
);

-- Create Server table
CREATE TABLE "Server" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "shortpath" TEXT NOT NULL,
    "favorite" BOOLEAN NOT NULL DEFAULT false,
    "index" INTEGER NOT NULL,
    "libraryPosition" INTEGER,
    "groupId" INTEGER NOT NULL,
    "serverType" TEXT NOT NULL DEFAULT 'vanilla',
    "gameVersion" TEXT NOT NULL,
    "port" INTEGER NOT NULL DEFAULT 25565,
    "motd" TEXT NOT NULL DEFAULT 'A Minecraft Server',
    "maxPlayers" INTEGER NOT NULL DEFAULT 20,
    "onlineMode" BOOLEAN NOT NULL DEFAULT true,
    "xmx" INTEGER NOT NULL DEFAULT 2048,
    "xms" INTEGER NOT NULL DEFAULT 1024,
    "extraJavaArgs" TEXT NOT NULL DEFAULT '',
    "autoRestart" BOOLEAN NOT NULL DEFAULT false,
    "dateCreated" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lastStarted" DATETIME,
    "providerType" TEXT NOT NULL DEFAULT 'local',
    "hostedServerId" TEXT,
    CONSTRAINT "Server_groupId_fkey" FOREIGN KEY ("groupId") REFERENCES "ServerGroup" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- Create unique index on Server shortpath
CREATE UNIQUE INDEX "Server_shortpath_key" ON "Server"("shortpath");

-- Add server settings to AppConfiguration
ALTER TABLE "AppConfiguration" ADD COLUMN "serversTileSize" INTEGER NOT NULL DEFAULT 2;
ALTER TABLE "AppConfiguration" ADD COLUMN "serversGroupBy" TEXT;
ALTER TABLE "AppConfiguration" ADD COLUMN "serversGroupByAsc" BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE "AppConfiguration" ADD COLUMN "serversSortBy" TEXT;
ALTER TABLE "AppConfiguration" ADD COLUMN "serversSortByAsc" BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE "AppConfiguration" ADD COLUMN "serversDuplicateFavorites" BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE "AppConfiguration" ADD COLUMN "defaultServerGroup" INTEGER;

-- Create default server group
INSERT INTO "ServerGroup" ("name", "groupIndex") VALUES ('Default', 0);
