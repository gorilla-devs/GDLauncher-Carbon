-- CreateTable
CREATE TABLE "AppConfiguration" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT DEFAULT 0,
    "theme" TEXT NOT NULL DEFAULT 'main',
    "reducedMotion" BOOLEAN NOT NULL DEFAULT false,
    "language" TEXT NOT NULL DEFAULT 'en',
    "discordIntegration" BOOLEAN NOT NULL DEFAULT true,
    "releaseChannel" TEXT NOT NULL DEFAULT 'stable',
    "activeAccountUuid" TEXT,
    "concurrentDownloads" INTEGER NOT NULL DEFAULT 8,
    "showNews" BOOLEAN NOT NULL DEFAULT true,
    "startupResolution" TEXT NOT NULL DEFAULT '854x480',
    "javaCustomArgs" TEXT NOT NULL DEFAULT '',
    "xmx" INTEGER NOT NULL DEFAULT 1024,
    "xms" INTEGER NOT NULL DEFAULT 1024
);

-- CreateTable
CREATE TABLE "Java" (
    "path" TEXT NOT NULL PRIMARY KEY,
    "major" INTEGER NOT NULL,
    "fullVersion" TEXT NOT NULL,
    "type" TEXT NOT NULL,
    "arch" TEXT NOT NULL,
    "isValid" BOOLEAN NOT NULL DEFAULT false
);

-- CreateTable
CREATE TABLE "DefaultJava" (
    "path" TEXT NOT NULL PRIMARY KEY,
    "major" INTEGER NOT NULL
);

-- CreateTable
CREATE TABLE "Account" (
    "uuid" TEXT NOT NULL PRIMARY KEY,
    "username" TEXT NOT NULL,
    "accessToken" TEXT,
    "tokenExpires" DATETIME,
    "msRefreshToken" TEXT,
    "lastUsed" DATETIME NOT NULL,
    "skinId" TEXT
);

-- CreateTable
CREATE TABLE "Skin" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "skin" BLOB NOT NULL
);

-- CreateTable
CREATE TABLE "HTTPCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "url" TEXT NOT NULL,
    "data" BLOB NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- CreateTable
CREATE TABLE "ActiveDownloads" (
    "url" TEXT NOT NULL PRIMARY KEY,
    "file_id" TEXT NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");

-- CreateIndex
CREATE UNIQUE INDEX "Java_path_key" ON "Java"("path");

-- CreateIndex
CREATE UNIQUE INDEX "DefaultJava_major_key" ON "DefaultJava"("major");

-- CreateIndex
CREATE UNIQUE INDEX "ActiveDownloads_file_id_key" ON "ActiveDownloads"("file_id");
