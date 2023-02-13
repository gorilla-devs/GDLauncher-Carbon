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
    "sha1" TEXT NOT NULL,
    CONSTRAINT "MinecraftManifest_sha1_fkey" FOREIGN KEY ("sha1") REFERENCES "MinecraftVersion" ("idSha1") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "MinecraftVersion" (
    "idSha1" TEXT NOT NULL PRIMARY KEY,
    "json" BLOB NOT NULL,
    "assetsIdSha1" TEXT NOT NULL,
    CONSTRAINT "MinecraftVersion_assetsIdSha1_fkey" FOREIGN KEY ("assetsIdSha1") REFERENCES "MinecraftAssets" ("idSha1") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "MinecraftAssets" (
    "idSha1" TEXT NOT NULL PRIMARY KEY,
    "json" BLOB NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftManifest_id_key" ON "MinecraftManifest"("id");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftManifest_sha1_key" ON "MinecraftManifest"("sha1");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftVersion_idSha1_key" ON "MinecraftVersion"("idSha1");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftAssets_idSha1_key" ON "MinecraftAssets"("idSha1");
