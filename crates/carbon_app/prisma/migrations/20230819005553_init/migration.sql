-- CreateTable
CREATE TABLE "AppConfiguration" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT DEFAULT 0,
    "theme" TEXT NOT NULL DEFAULT 'main',
    "reducedMotion" BOOLEAN NOT NULL DEFAULT false,
    "language" TEXT NOT NULL DEFAULT 'english',
    "discordIntegration" BOOLEAN NOT NULL DEFAULT true,
    "releaseChannel" TEXT NOT NULL,
    "activeAccountUuid" TEXT,
    "concurrentDownloads" INTEGER NOT NULL DEFAULT 8,
    "showNews" BOOLEAN NOT NULL DEFAULT true,
    "startupResolution" TEXT NOT NULL DEFAULT '854x480',
    "javaCustomArgs" TEXT NOT NULL DEFAULT '',
    "xmx" INTEGER NOT NULL,
    "xms" INTEGER NOT NULL DEFAULT 1024,
    "defaultInstanceGroup" INTEGER,
    "isFirstLaunch" BOOLEAN NOT NULL DEFAULT true,
    "autoManageJava" BOOLEAN NOT NULL DEFAULT true,
    "preferredModChannel" INTEGER NOT NULL DEFAULT 2,
    "randomUserUuid" TEXT NOT NULL,
    "secret" BLOB NOT NULL,
    "termsAndPrivacyAccepted" BOOLEAN NOT NULL DEFAULT false,
    "termsAndPrivacyAcceptedChecksum" TEXT,
    "metricsEnabled" BOOLEAN NOT NULL DEFAULT false,
    "metricsEnabledLastUpdate" DATETIME
);

-- CreateTable
CREATE TABLE "Java" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "path" TEXT NOT NULL,
    "major" INTEGER NOT NULL,
    "fullVersion" TEXT NOT NULL,
    "type" TEXT NOT NULL,
    "os" TEXT NOT NULL,
    "arch" TEXT NOT NULL,
    "vendor" TEXT NOT NULL,
    "isValid" BOOLEAN NOT NULL DEFAULT true
);

-- CreateTable
CREATE TABLE "JavaSystemProfile" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    "javaId" TEXT,
    CONSTRAINT "JavaSystemProfile_javaId_fkey" FOREIGN KEY ("javaId") REFERENCES "Java" ("id") ON DELETE SET NULL ON UPDATE CASCADE
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

-- CreateTable
CREATE TABLE "Instance" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "shortpath" TEXT NOT NULL,
    "favorite" BOOLEAN NOT NULL DEFAULT false,
    "index" INTEGER NOT NULL,
    "groupId" INTEGER NOT NULL,
    CONSTRAINT "Instance_groupId_fkey" FOREIGN KEY ("groupId") REFERENCES "InstanceGroup" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "InstanceGroup" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "groupIndex" INTEGER NOT NULL
);

-- CreateTable
CREATE TABLE "ModFileCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "instanceId" INTEGER NOT NULL,
    "filename" TEXT NOT NULL,
    "filesize" INTEGER NOT NULL,
    "enabled" BOOLEAN NOT NULL,
    "metadataId" TEXT NOT NULL,
    CONSTRAINT "ModFileCache_instanceId_fkey" FOREIGN KEY ("instanceId") REFERENCES "Instance" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "ModFileCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "ModMetadata" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "murmur2" INTEGER NOT NULL,
    "sha512" BLOB NOT NULL,
    "name" TEXT,
    "modid" TEXT,
    "version" TEXT,
    "description" TEXT,
    "authors" TEXT,
    "modloaders" TEXT NOT NULL,
    "logoImage" BLOB
);

-- CreateTable
CREATE TABLE "CurseForgeModCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "murmur2" INTEGER NOT NULL,
    "projectId" INTEGER NOT NULL,
    "fileId" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "urlslug" TEXT NOT NULL,
    "summary" TEXT NOT NULL,
    "authors" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    "metadataId" TEXT NOT NULL,
    CONSTRAINT "CurseForgeModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "ModrinthModCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "sha512" TEXT NOT NULL,
    "sha1" TEXT NOT NULL,
    "filename" TEXT NOT NULL,
    "projectId" TEXT NOT NULL,
    "versionId" TEXT NOT NULL,
    "title" TEXT NOT NULL,
    "urlslug" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "authors" TEXT NOT NULL,
    "cachedAt" DATETIME NOT NULL,
    "metadataId" TEXT NOT NULL,
    CONSTRAINT "ModrinthModCache_metadataId_fkey" FOREIGN KEY ("metadataId") REFERENCES "ModMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateIndex
CREATE UNIQUE INDEX "AppConfiguration_id_key" ON "AppConfiguration"("id");

-- CreateIndex
CREATE UNIQUE INDEX "Java_id_key" ON "Java"("id");

-- CreateIndex
CREATE UNIQUE INDEX "Java_path_key" ON "Java"("path");

-- CreateIndex
CREATE UNIQUE INDEX "JavaSystemProfile_id_key" ON "JavaSystemProfile"("id");

-- CreateIndex
CREATE UNIQUE INDEX "JavaSystemProfile_name_key" ON "JavaSystemProfile"("name");

-- CreateIndex
CREATE UNIQUE INDEX "ActiveDownloads_file_id_key" ON "ActiveDownloads"("file_id");

-- CreateIndex
CREATE UNIQUE INDEX "Instance_shortpath_key" ON "Instance"("shortpath");

-- CreateIndex
CREATE UNIQUE INDEX "ModFileCache_instanceId_filename_key" ON "ModFileCache"("instanceId", "filename");

-- CreateIndex
CREATE UNIQUE INDEX "CurseForgeModCache_metadataId_key" ON "CurseForgeModCache"("metadataId");

-- CreateIndex
CREATE UNIQUE INDEX "CurseForgeModCache_projectId_fileId_key" ON "CurseForgeModCache"("projectId", "fileId");

-- CreateIndex
CREATE UNIQUE INDEX "ModrinthModCache_metadataId_key" ON "ModrinthModCache"("metadataId");

-- CreateIndex
CREATE UNIQUE INDEX "ModrinthModCache_projectId_versionId_key" ON "ModrinthModCache"("projectId", "versionId");
