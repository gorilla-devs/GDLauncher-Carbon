-- CreateTable
CREATE TABLE "AppConfiguration" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT DEFAULT 0,
    "theme" TEXT NOT NULL DEFAULT 'main',
    "reducedMotion" BOOLEAN NOT NULL DEFAULT false,
    "discordIntegration" BOOLEAN NOT NULL DEFAULT true,
    "releaseChannel" TEXT NOT NULL DEFAULT 'stable'
);

-- CreateTable
CREATE TABLE "Accounts" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
);

-- CreateTable
CREATE TABLE "MinecraftManifest" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "type" TEXT NOT NULL,
    "url" TEXT NOT NULL,
    "time" TEXT NOT NULL,
    "releaseTime" TEXT NOT NULL,
    "sha1" TEXT NOT NULL,
    CONSTRAINT "MinecraftManifest_id_fkey" FOREIGN KEY ("id") REFERENCES "MinecraftVersion" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "MinecraftVersion" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "json" BLOB NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "Accounts_id_key" ON "Accounts"("id");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftManifest_id_key" ON "MinecraftManifest"("id");

-- CreateIndex
CREATE UNIQUE INDEX "MinecraftVersion_id_key" ON "MinecraftVersion"("id");
