-- CreateTable
CREATE TABLE "AppConfiguration" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT DEFAULT 0,
    "theme" TEXT NOT NULL DEFAULT 'main',
    "reducedMotion" BOOLEAN NOT NULL DEFAULT false,
    "discordIntegration" BOOLEAN NOT NULL DEFAULT true,
    "releaseChannel" TEXT NOT NULL DEFAULT 'stable',
    "activeAccountUuid" TEXT
);

-- CreateTable
CREATE TABLE "Account" (
    "uuid" TEXT NOT NULL PRIMARY KEY,
    "username" TEXT NOT NULL,
    "accessToken" TEXT,
    "tokenExpires" DATETIME,
    "msRefreshToken" TEXT
);

-- CreateTable
CREATE TABLE "MinecraftManifest" (
    "id" TEXT NOT NULL,
    "type" TEXT NOT NULL,
    "url" TEXT NOT NULL,
    "time" TEXT NOT NULL,
    "releaseTime" TEXT NOT NULL,
    "sha1" TEXT NOT NULL
);

-- CreateTable
CREATE TABLE "MinecraftVersion" (
    "id" TEXT NOT NULL,
    "json" BLOB NOT NULL
);

-- CreateTable
CREATE TABLE "MinecraftAssets" (
    "assetsIdSha1" TEXT NOT NULL,
    "json" BLOB NOT NULL
);

-- CreateTable
CREATE TABLE "ActiveDownloads" (
    "url" TEXT NOT NULL PRIMARY KEY,
    "file_id" TEXT NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftManifest_id_key" ON "MinecraftManifest"("id");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftManifest_sha1_key" ON "MinecraftManifest"("sha1");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftVersion_id_key" ON "MinecraftVersion"("id");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftAssets_assetsIdSha1_key" ON "MinecraftAssets"("assetsIdSha1");

-- CreateIndex
CREATE UNIQUE INDEX "ActiveDownloads_file_id_key" ON "ActiveDownloads"("file_id");
