import fs from "node:fs/promises"
import path from "node:path"
import { RECOVERY_COPY_DAYS } from "./dbRecovery"
import { assertDatabaseRestoreReady } from "./databaseRestore"

// The names a reset (ISO date, mkdtemp suffix) and a restore (UUID) generate. A
// folder the user renamed or duplicated to keep it is theirs.
const BUNDLE =
  /^gdl_conf\.(reset-recovery-\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}-\d{3}Z-[0-9A-Za-z]{6}|restore-originals-[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12})$/
// A copy whose removal has started: nobody may recover from it any more.
const RETIRED = "gdl_conf.pruning-"
// What a reset or restore puts in its folder. A folder holding anything else is the user's.
const BUNDLE_FILES = new Set([
  "gdl_conf.db",
  "gdl_conf.db-wal",
  "gdl_conf.db-shm",
  "gdl_conf.pre-downgrade.db",
  "captured-files.json",
  "snapshot-staged.db"
])
const DAY = 24 * 60 * 60 * 1000

/**
 * Removes recovery copies of the database older than RECOVERY_COPY_DAYS. Runs
 * after the core started cleanly: the database in use works, and the copies hold
 * old sign-in tokens and take the space of a whole database. A folder is first
 * renamed to a retired name, which Windows refuses while a file in it is open, so
 * a lock is found before anything is deleted. A retired folder is finished on the
 * next start whatever its age, as deleting from it updates its date. Nothing is
 * removed from a folder holding something else, or while a restore is pending.
 * Returns the folders removed, by their original names.
 */
export async function pruneRecoveryCopies(
  root: string,
  now = Date.now(),
  files: typeof fs = fs
): Promise<string[]> {
  await assertDatabaseRestoreReady(root, files)
  const removed: string[] = []
  for (const name of await files.readdir(root)) {
    const retired = name.startsWith(RETIRED)
    const original = retired ? `gdl_conf.${name.slice(RETIRED.length)}` : name
    if (!BUNDLE.test(original)) continue
    const bundle = path.join(root, name)
    try {
      const stat = await files.lstat(bundle)
      if (
        !stat.isDirectory() ||
        (!retired && now - stat.mtimeMs < RECOVERY_COPY_DAYS * DAY)
      )
        continue
      const entries = await files.readdir(bundle)
      const stats = await Promise.all(
        entries.map((entry) => files.lstat(path.join(bundle, entry)))
      )
      if (
        !entries.every(
          (entry, index) => BUNDLE_FILES.has(entry) && stats[index].isFile()
        )
      )
        continue
      const folder = retired
        ? bundle
        : path.join(root, RETIRED + name.slice("gdl_conf.".length))
      if (!retired) await files.rename(bundle, folder)
      for (const entry of entries) await files.unlink(path.join(folder, entry))
      await files.rmdir(folder)
      removed.push(original)
    } catch (error) {
      // Tried again on the next start; the other folders are not held up.
      console.warn(`Could not remove the recovery copy ${bundle}:`, error)
    }
  }
  return removed
}
