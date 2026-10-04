-- Older builds keep one row per server and file name, and find addons only in
-- mods/ and world/datapacks/. These rows cache a folder scan, so drop the ones
-- that shape cannot hold (datapacks of another world, the same name twice,
-- keeping the mods row) and let the older build rescan. Files stay on disk.
DELETE FROM ServerModFileCache
WHERE addonType NOT IN ('mods', 'datapacks')
   OR location != CASE addonType
        WHEN 'mods' THEN 'mods/' || filename
        ELSE 'world/datapacks/' || filename END;
DELETE FROM ServerModFileCache WHERE id IN (
    SELECT id FROM (
        SELECT id, row_number() OVER (
            PARTITION BY serverId, filename ORDER BY addonType = 'mods' DESC, id
        ) AS keep_rank
        FROM ServerModFileCache
    ) WHERE keep_rank > 1
);
DROP INDEX ServerModFileCache_owner_location_key;
-- The older build reads platform identity from the hash caches, as 2.0.40 did.
ALTER TABLE ServerModFileCache DROP COLUMN sourceSha512;
ALTER TABLE ServerModFileCache DROP COLUMN sourceVersionId;
ALTER TABLE ServerModFileCache DROP COLUMN sourceProjectId;
ALTER TABLE ServerModFileCache DROP COLUMN sourcePlatform;
ALTER TABLE ServerModFileCache DROP COLUMN location;
CREATE UNIQUE INDEX "ServerModFileCache_serverId_filename_key" ON "ServerModFileCache"("serverId", "filename");
