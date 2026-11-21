CREATE TABLE WorldDatapackCache (
  id TEXT NOT NULL PRIMARY KEY DEFAULT (hex(randomblob(16))),
  lastUpdatedAt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  instanceId INTEGER NOT NULL,
  world TEXT NOT NULL,
  filename TEXT NOT NULL,
  filesize INTEGER NOT NULL,
  isFolder BOOLEAN NOT NULL DEFAULT 0,
  metadataId TEXT,
  fileStamp TEXT,
  FOREIGN KEY (instanceId) REFERENCES Instance(id) ON DELETE CASCADE ON UPDATE CASCADE,
  FOREIGN KEY (metadataId) REFERENCES ModMetadata(id) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE UNIQUE INDEX WorldDatapackCache_place ON WorldDatapackCache(instanceId, world, filename);
CREATE INDEX WorldDatapackCache_metadataId_idx ON WorldDatapackCache(metadataId);
