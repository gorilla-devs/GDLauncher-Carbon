DELETE FROM ModMetadata;

/*
  Warnings:

  - Added the required column `sha1` to the `ModMetadata` table without a default value. This is not possible if the table is not empty.

*/
-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_ModMetadata" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "murmur2" INTEGER NOT NULL,
    "sha512" BLOB NOT NULL,
    "sha1" BLOB NOT NULL,
    "name" TEXT,
    "modid" TEXT,
    "version" TEXT,
    "description" TEXT,
    "authors" TEXT,
    "modloaders" TEXT NOT NULL
);
INSERT INTO "new_ModMetadata" ("authors", "description", "id", "modid", "modloaders", "murmur2", "name", "sha512", "version") SELECT "authors", "description", "id", "modid", "modloaders", "murmur2", "name", "sha512", "version" FROM "ModMetadata";
DROP TABLE "ModMetadata";
ALTER TABLE "new_ModMetadata" RENAME TO "ModMetadata";
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
