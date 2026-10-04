-- Fixed instance folders (including staged datapacks) are distinct owners.
DROP INDEX ModFileCache_instanceId_filename_key;
CREATE UNIQUE INDEX ModFileCache_instanceId_addonType_filename_key ON ModFileCache(instanceId, addonType, filename);
ALTER TABLE ModFileCache ADD COLUMN fileStamp TEXT;
-- Artifact identity remains shared; queried compatibility contexts do not.
CREATE TABLE ModrinthUpdateCache (
    metadataId TEXT NOT NULL,
    context TEXT NOT NULL,
    updatePaths TEXT NOT NULL,
    cachedAt DATETIME NOT NULL,
    PRIMARY KEY (metadataId, context),
    FOREIGN KEY (metadataId) REFERENCES ModMetadata(id) ON DELETE CASCADE ON UPDATE CASCADE
);
