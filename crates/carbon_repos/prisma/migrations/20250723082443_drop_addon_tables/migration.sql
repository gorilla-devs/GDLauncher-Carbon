/*
  Warnings:

  - You are about to drop the `AddonCacheStatus` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonChecksums` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonDependency` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonHardLink` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonImage` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonInstanceLink` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonMetadata` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonPlatformData` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `AddonVersion` table. If the table is not empty, all the data it contains will be lost.

*/
-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonCacheStatus";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonChecksums";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonDependency";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonHardLink";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonImage";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonInstanceLink";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonMetadata";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonPlatformData";
PRAGMA foreign_keys=on;

-- DropTable
PRAGMA foreign_keys=off;
DROP TABLE "AddonVersion";
PRAGMA foreign_keys=on;
