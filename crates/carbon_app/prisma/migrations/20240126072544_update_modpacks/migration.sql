-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_Instance" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "shortpath" TEXT NOT NULL,
    "favorite" BOOLEAN NOT NULL DEFAULT false,
    "hasPackUpdate" BOOLEAN NOT NULL DEFAULT false,
    "index" INTEGER NOT NULL,
    "groupId" INTEGER NOT NULL,
    "javaProfileId" TEXT,
    CONSTRAINT "Instance_groupId_fkey" FOREIGN KEY ("groupId") REFERENCES "InstanceGroup" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT "Instance_javaProfileId_fkey" FOREIGN KEY ("javaProfileId") REFERENCES "JavaProfile" ("id") ON DELETE SET NULL ON UPDATE CASCADE
);
INSERT INTO "new_Instance" ("favorite", "groupId", "id", "index", "javaProfileId", "name", "shortpath") SELECT "favorite", "groupId", "id", "index", "javaProfileId", "name", "shortpath" FROM "Instance";
DROP TABLE "Instance";
ALTER TABLE "new_Instance" RENAME TO "Instance";
CREATE UNIQUE INDEX "Instance_shortpath_key" ON "Instance"("shortpath");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
