/*
  Warnings:

  - You are about to drop the `Accounts` table. If the table is not empty, all the data it contains will be lost.
  - The primary key for the `MinecraftManifest` table will be changed. If it partially fails, the table could be left without primary key constraint.

*/
-- DropIndex
DROP INDEX "Accounts_id_key";

-- AlterTable
ALTER TABLE "AppConfiguration" ADD COLUMN "activeAccountUuid" TEXT;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "Accounts";
PRAGMA foreign_keys=on;

-- CreateTable
CREATE TABLE "Account" (
    "uuid" TEXT NOT NULL PRIMARY KEY,
    "username" TEXT NOT NULL,
    "accessToken" TEXT,
    "tokenExpires" DATETIME,
    "msRefreshToken" TEXT
);

-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_MinecraftManifest" (
    "id" TEXT NOT NULL,
    "type" TEXT NOT NULL,
    "url" TEXT NOT NULL,
    "time" TEXT NOT NULL,
    "releaseTime" TEXT NOT NULL,
    "sha1" TEXT NOT NULL,
    CONSTRAINT "MinecraftManifest_sha1_fkey" FOREIGN KEY ("sha1") REFERENCES "MinecraftVersion" ("idSha1") ON DELETE RESTRICT ON UPDATE CASCADE
);
INSERT INTO "new_MinecraftManifest" ("id", "releaseTime", "sha1", "time", "type", "url") SELECT "id", "releaseTime", "sha1", "time", "type", "url" FROM "MinecraftManifest";
DROP TABLE "MinecraftManifest";
ALTER TABLE "new_MinecraftManifest" RENAME TO "MinecraftManifest";
CREATE UNIQUE INDEX "MinecraftManifest_id_key" ON "MinecraftManifest"("id");
CREATE UNIQUE INDEX "MinecraftManifest_sha1_key" ON "MinecraftManifest"("sha1");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
