-- AlterTable
ALTER TABLE "AppConfiguration" ADD COLUMN "postExitHook" TEXT;
ALTER TABLE "AppConfiguration" ADD COLUMN "preLaunchHook" TEXT;
ALTER TABLE "AppConfiguration" ADD COLUMN "wrapperCommand" TEXT;
