-- Add modpack tracking fields to Server
ALTER TABLE "Server" ADD COLUMN "modpackPlatform" TEXT;
ALTER TABLE "Server" ADD COLUMN "modpackProjectId" TEXT;
ALTER TABLE "Server" ADD COLUMN "modpackFileId" TEXT;
