-- CreateTable
CREATE TABLE "VersionInfoCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "versionInfo" BLOB NOT NULL
);

-- CreateTable
CREATE TABLE "PartialVersionInfoCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "partialVersionInfo" BLOB NOT NULL
);

-- CreateTable
CREATE TABLE "LwjglMetaCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "lwjgl" BLOB NOT NULL
);

-- CreateTable
CREATE TABLE "AssetsMetaCache" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "assetsIndex" BLOB NOT NULL
);
