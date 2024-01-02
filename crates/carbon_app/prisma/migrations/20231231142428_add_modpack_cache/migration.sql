-- CreateTable
CREATE TABLE "CurseForgeModpackCache" (
    "projectId" INTEGER NOT NULL,
    "fileId" INTEGER NOT NULL,
    "modpackName" TEXT NOT NULL,
    "versionName" TEXT NOT NULL,
    "urlSlug" TEXT NOT NULL,
    "updatedAt" DATETIME NOT NULL,

    PRIMARY KEY ("projectId", "fileId")
);

-- CreateTable
CREATE TABLE "ModrinthModpackCache" (
    "projectId" TEXT NOT NULL,
    "versionId" TEXT NOT NULL,
    "modpackName" TEXT NOT NULL,
    "versionName" TEXT NOT NULL,
    "urlSlug" TEXT NOT NULL,
    "updatedAt" DATETIME NOT NULL,

    PRIMARY KEY ("projectId", "versionId")
);

-- CreateTable
CREATE TABLE "CurseForgeModpackImageCache" (
    "projectId" INTEGER NOT NULL,
    "fileId" INTEGER NOT NULL,
    "url" TEXT NOT NULL,
    "data" BLOB,

    PRIMARY KEY ("projectId", "fileId"),
    CONSTRAINT "CurseForgeModpackImageCache_projectId_fileId_fkey" FOREIGN KEY ("projectId", "fileId") REFERENCES "CurseForgeModpackCache" ("projectId", "fileId") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "ModrinthModpackImageCache" (
    "projectId" TEXT NOT NULL,
    "versionId" TEXT NOT NULL,
    "url" TEXT NOT NULL,
    "data" BLOB,

    PRIMARY KEY ("projectId", "versionId"),
    CONSTRAINT "ModrinthModpackImageCache_projectId_versionId_fkey" FOREIGN KEY ("projectId", "versionId") REFERENCES "ModrinthModpackCache" ("projectId", "versionId") ON DELETE CASCADE ON UPDATE CASCADE
);
