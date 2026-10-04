-- Distinct physical artifacts of one Modrinth version keep their existing owners.
-- Legacy mismatched associations remain stored for reversible migration; readers
-- must qualify their SHA512 against ModMetadata before using artifact/freshness.
DROP INDEX ModrinthModCache_projectId_versionId_key;
CREATE INDEX ModrinthModCache_projectId_versionId_idx ON ModrinthModCache(projectId, versionId);
-- Parent deletion checks must locate live instance/server references by owner.
CREATE INDEX ModFileCache_metadataId_idx ON ModFileCache(metadataId);
CREATE INDEX ServerModFileCache_metadataId_idx ON ServerModFileCache(metadataId);
