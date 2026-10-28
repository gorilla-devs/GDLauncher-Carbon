-- Full addon-cache maintenance clear. Run on a top-level connection:
--   sqlite3 -bail /path/to/owned-test-or-maintenance.db < clear_db_caches.sql
-- Other executors must stop on the first error and ROLLBACK or close the
-- connection; SQLite ABORT alone does not undo earlier statements. Do not
-- continue to COMMIT after an error or run inside an existing transaction.
PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

DELETE FROM ModFileCache;
DELETE FROM ServerModFileCache;
DELETE FROM LocalModImageCache;
DELETE FROM CurseForgeModImageCache;
DELETE FROM ModrinthModImageCache;
DELETE FROM ModrinthUpdateCache;
DELETE FROM CurseForgeModCache;
DELETE FROM ModrinthModCache;
DELETE FROM ModMetadata;

COMMIT;
