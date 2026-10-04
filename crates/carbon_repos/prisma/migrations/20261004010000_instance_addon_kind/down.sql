-- Older builds keep one row per instance and file name. These rows cache a
-- folder scan, so when one name sits in two addon folders keep the mods row
-- (else the first by id), drop the rest and let the older build rescan.
-- Files stay on disk.
DELETE FROM ModFileCache WHERE id IN (
    SELECT id FROM (
        SELECT id, row_number() OVER (
            PARTITION BY instanceId, filename ORDER BY addonType = 'mods' DESC, id
        ) AS keep_rank
        FROM ModFileCache
    ) WHERE keep_rank > 1
);
DROP INDEX ModFileCache_instanceId_addonType_filename_key;
CREATE UNIQUE INDEX "ModFileCache_instanceId_filename_key" ON "ModFileCache"("instanceId", "filename");
ALTER TABLE ModFileCache DROP COLUMN fileStamp;
DROP TABLE ModrinthUpdateCache;
