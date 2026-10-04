-- Older builds keep one Modrinth cache row per project version. The rows only
-- cache what Modrinth reports for a file, so when files share a version keep
-- the row whose hash matches its file (newest first), drop the rest with their
-- images and let the older build look them up again. Files stay on disk.
DELETE FROM ModrinthModCache WHERE metadataId IN (
    SELECT metadataId FROM (
        SELECT r.metadataId, row_number() OVER (
            PARTITION BY r.projectId, r.versionId
            ORDER BY EXISTS (
                SELECT 1 FROM ModMetadata m
                WHERE m.id = r.metadataId AND lower(r.sha512) = lower(hex(m.sha512))
            ) DESC, r.cachedAt DESC, r.metadataId
        ) AS keep_rank
        FROM ModrinthModCache r
    ) WHERE keep_rank > 1
);
-- Foreign keys are off while a downgrade runs, so remove the images by hand.
DELETE FROM ModrinthModImageCache WHERE NOT EXISTS (
    SELECT 1 FROM ModrinthModCache r WHERE r.metadataId = ModrinthModImageCache.metadataId
);
DROP INDEX ModFileCache_metadataId_idx;
DROP INDEX ServerModFileCache_metadataId_idx;
DROP INDEX ModrinthModCache_projectId_versionId_idx;
CREATE UNIQUE INDEX "ModrinthModCache_projectId_versionId_key" ON "ModrinthModCache"("projectId", "versionId");
