-- CreateTable
CREATE TABLE "AddonMetadata" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    "version" TEXT NOT NULL,
    "authors" TEXT NOT NULL DEFAULT '',
    "description" TEXT,
    "modFormat" TEXT NOT NULL DEFAULT 'Unknown',
    "minecraftVersions" TEXT NOT NULL DEFAULT '',
    "modLoaders" TEXT NOT NULL DEFAULT '',
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL
);

-- CreateTable
CREATE TABLE "AddonChecksums" (
    "addonId" TEXT NOT NULL PRIMARY KEY,
    "blake3" TEXT NOT NULL,
    "sha256" TEXT NOT NULL,
    "md5" TEXT NOT NULL,
    "murmur2" INTEGER NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AddonChecksums_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonDependency" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "modId" TEXT NOT NULL,
    "versionRequirement" TEXT NOT NULL,
    "dependencyType" TEXT NOT NULL DEFAULT 'Required',
    CONSTRAINT "AddonDependency_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonImage" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "imageType" TEXT NOT NULL DEFAULT 'Icon',
    "data" BLOB NOT NULL,
    "url" TEXT,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AddonImage_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonPlatformData" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "platform" TEXT NOT NULL,
    "projectId" TEXT NOT NULL,
    "fileId" TEXT NOT NULL,
    "downloadUrl" TEXT,
    "projectName" TEXT NOT NULL,
    "projectDescription" TEXT,
    "categories" TEXT NOT NULL DEFAULT '',
    "license" TEXT,
    "websiteUrl" TEXT,
    "sourceUrl" TEXT,
    "issuesUrl" TEXT,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "AddonPlatformData_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonVersion" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "versionNumber" TEXT NOT NULL,
    "versionType" TEXT NOT NULL DEFAULT 'Release',
    "minecraftVersions" TEXT NOT NULL DEFAULT '',
    "modLoaders" TEXT NOT NULL DEFAULT '',
    "releaseDate" TEXT NOT NULL,
    "downloadUrl" TEXT NOT NULL,
    "changelog" TEXT,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AddonVersion_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonHardLink" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "blake3Hash" TEXT NOT NULL,
    "centralPath" TEXT NOT NULL,
    "instancePaths" TEXT NOT NULL DEFAULT '',
    "linkValid" BOOLEAN NOT NULL DEFAULT true,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lastVerified" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AddonHardLink_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonInstanceLink" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "instanceId" TEXT NOT NULL,
    "filePath" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AddonInstanceLink_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "AddonCacheStatus" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "addonId" TEXT NOT NULL,
    "stage" TEXT NOT NULL,
    "completed" BOOLEAN NOT NULL DEFAULT false,
    "error" TEXT,
    "lastUpdated" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AddonCacheStatus_addonId_fkey" FOREIGN KEY ("addonId") REFERENCES "AddonMetadata" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateIndex
CREATE UNIQUE INDEX "AddonChecksums_blake3_key" ON "AddonChecksums"("blake3");

-- CreateIndex
CREATE UNIQUE INDEX "AddonDependency_addonId_modId_key" ON "AddonDependency"("addonId", "modId");

-- CreateIndex
CREATE UNIQUE INDEX "AddonImage_addonId_imageType_key" ON "AddonImage"("addonId", "imageType");

-- CreateIndex
CREATE UNIQUE INDEX "AddonPlatformData_addonId_platform_key" ON "AddonPlatformData"("addonId", "platform");

-- CreateIndex
CREATE UNIQUE INDEX "AddonVersion_addonId_versionNumber_key" ON "AddonVersion"("addonId", "versionNumber");

-- CreateIndex
CREATE UNIQUE INDEX "AddonHardLink_addonId_key" ON "AddonHardLink"("addonId");

-- CreateIndex
CREATE UNIQUE INDEX "AddonInstanceLink_addonId_instanceId_key" ON "AddonInstanceLink"("addonId", "instanceId");

-- CreateIndex
CREATE UNIQUE INDEX "AddonCacheStatus_addonId_stage_key" ON "AddonCacheStatus"("addonId", "stage");
