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
    "xms" INTEGER NOT NULL DEFAULT 1024,
    "isFirstLaunch" BOOLEAN NOT NULL DEFAULT true,
    "autoManageJava" BOOLEAN NOT NULL DEFAULT true
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
CREATE TABLE "JavaSystemProfile" (
    "name" TEXT NOT NULL,
    "supportedJavaVersionRegex" TEXT,
    "autodetect" BOOLEAN NOT NULL DEFAULT true,
    "useManagedAsFallback" BOOLEAN NOT NULL DEFAULT true,
    "javaPath" TEXT,
    CONSTRAINT "JavaSystemProfile_javaPath_fkey" FOREIGN KEY ("javaPath") REFERENCES "Java" ("path") ON DELETE SET NULL ON UPDATE CASCADE
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
    "url" TEXT NOT NULL PRIMARY KEY,
    "status_code" INTEGER NOT NULL,
    "data" BLOB NOT NULL,
    "expiresAt" DATETIME DEFAULT CURRENT_TIMESTAMP,
    "lastModified" TEXT,
    "etag" TEXT
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
CREATE UNIQUE INDEX "JavaSystemProfile_name_key" ON "JavaSystemProfile"("name");

-- CreateIndex
CREATE UNIQUE INDEX "ActiveDownloads_file_id_key" ON "ActiveDownloads"("file_id");
