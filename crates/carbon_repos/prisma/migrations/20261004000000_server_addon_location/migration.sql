-- Legacy producers scanned only mods/ and world/datapacks/. Never infer a
-- historical datapack owner from the operator's current level-name.
ALTER TABLE ServerModFileCache ADD COLUMN location TEXT NOT NULL DEFAULT '';
UPDATE ServerModFileCache SET location = CASE addonType
  WHEN 'mods' THEN 'mods/' || filename
  WHEN 'datapacks' THEN 'world/datapacks/' || filename END
WHERE addonType IN ('mods', 'datapacks') AND filename NOT IN ('', '.', '..')
  AND instr(filename, '/') = 0 AND instr(filename, char(92)) = 0 AND instr(filename, ':') = 0;
-- Only ambiguous cache rows are invalidated. Files and shared metadata remain.
DELETE FROM ServerModFileCache WHERE location = '';
DROP INDEX ServerModFileCache_serverId_filename_key;
CREATE UNIQUE INDEX ServerModFileCache_owner_location_key ON ServerModFileCache(serverId, addonType, location);

-- Selected source belongs to this owner, independently of shared hash metadata.
ALTER TABLE ServerModFileCache ADD COLUMN sourcePlatform TEXT;
ALTER TABLE ServerModFileCache ADD COLUMN sourceProjectId TEXT;
ALTER TABLE ServerModFileCache ADD COLUMN sourceVersionId TEXT;
ALTER TABLE ServerModFileCache ADD COLUMN sourceSha512 BLOB;
