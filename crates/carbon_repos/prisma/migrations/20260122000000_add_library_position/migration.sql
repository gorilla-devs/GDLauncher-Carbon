-- AlterTable - Add libraryPosition to Instance
ALTER TABLE "Instance" ADD COLUMN "libraryPosition" INTEGER;

-- AlterTable - Add libraryPosition to InstanceGroup
ALTER TABLE "InstanceGroup" ADD COLUMN "libraryPosition" INTEGER;

-- Migrate Instance data:
-- For instances in default group: set libraryPosition = index
-- For instances in folders (non-default groups): leave libraryPosition = null
UPDATE "Instance" SET "libraryPosition" = "index"
WHERE "groupId" = (SELECT "defaultInstanceGroup" FROM "AppConfiguration" WHERE id = 0);

-- Migrate InstanceGroup data:
-- For non-default groups: set libraryPosition = groupIndex
-- For default group: leave libraryPosition = null
UPDATE "InstanceGroup" SET "libraryPosition" = "groupIndex"
WHERE "id" != COALESCE((SELECT "defaultInstanceGroup" FROM "AppConfiguration" WHERE id = 0), -1);
