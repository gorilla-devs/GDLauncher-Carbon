-- Server addon scans skip files whose observed stamp still matches the row.
ALTER TABLE ServerModFileCache ADD COLUMN fileStamp TEXT;
