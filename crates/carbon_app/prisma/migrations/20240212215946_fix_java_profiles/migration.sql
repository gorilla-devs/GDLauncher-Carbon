/*
  Warnings:

  - The primary key for the `JavaProfile` table will be changed. If it partially fails, the table could be left without primary key constraint.
  - You are about to drop the column `id` on the `JavaProfile` table. All the data in the column will be lost.
  - You are about to drop the column `javaProfileId` on the `Instance` table. All the data in the column will be lost.

*/
-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_JavaProfile" (
    "name" TEXT NOT NULL PRIMARY KEY,
    "isSystemProfile" BOOLEAN NOT NULL DEFAULT false,
    "javaId" TEXT,
    CONSTRAINT "JavaProfile_javaId_fkey" FOREIGN KEY ("javaId") REFERENCES "Java" ("id") ON DELETE SET NULL ON UPDATE CASCADE
);
INSERT INTO "new_JavaProfile" ("isSystemProfile", "javaId", "name") SELECT "isSystemProfile", "javaId", "name" FROM "JavaProfile";
DROP TABLE "JavaProfile";
ALTER TABLE "new_JavaProfile" RENAME TO "JavaProfile";
CREATE TABLE "new_Instance" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "shortpath" TEXT NOT NULL,
    "favorite" BOOLEAN NOT NULL DEFAULT false,
    "hasPackUpdate" BOOLEAN NOT NULL DEFAULT false,
    "index" INTEGER NOT NULL,
    "groupId" INTEGER NOT NULL,
    CONSTRAINT "Instance_groupId_fkey" FOREIGN KEY ("groupId") REFERENCES "InstanceGroup" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);
INSERT INTO "new_Instance" ("favorite", "groupId", "hasPackUpdate", "id", "index", "name", "shortpath") SELECT "favorite", "groupId", "hasPackUpdate", "id", "index", "name", "shortpath" FROM "Instance";
DROP TABLE "Instance";
ALTER TABLE "new_Instance" RENAME TO "Instance";
CREATE UNIQUE INDEX "Instance_shortpath_key" ON "Instance"("shortpath");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
