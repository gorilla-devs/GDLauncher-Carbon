import fs from "node:fs/promises"
import { constants, lstatSync } from "node:fs"
import path from "node:path"
import { createHash, randomUUID } from "node:crypto"

export type RestoreOutcome =
  | { ok: true; recoveryPath: string }
  | { ok: false; message: string; recoveryPath?: string }
const INTENT = ".gdl-db-restore.json"
const NAMES = ["gdl_conf.db", "gdl_conf.db-wal", "gdl_conf.db-shm"]
async function exists(io: typeof fs, file: string) {
  try {
    return await io.lstat(file)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return null
    throw error
  }
}
async function digest(io: typeof fs, file: string) {
  const handle = await io.open(file, "r")
  const hash = createHash("sha256"),
    buffer = Buffer.alloc(64 * 1024)
  try {
    for (;;) {
      const { bytesRead } = await handle.read(buffer, 0, buffer.length, null)
      if (!bytesRead) break
      hash.update(buffer.subarray(0, bytesRead))
    }
    return hash.digest("hex")
  } finally {
    await handle.close()
  }
}
async function plain(io: typeof fs, file: string) {
  const stat = await exists(io, file)
  if (stat && (!stat.isFile() || stat.isSymbolicLink()))
    throw new Error("Recovery requires regular database files")
  return stat
}
// Only inability to create a hard link permits exclusive-copy compatibility.
// Permission/disk/IO failures and EEXIST are never treated as capability errors.
const LINK_UNAVAILABLE = new Set(["ENOTSUP", "EOPNOTSUPP", "ENOSYS", "EXDEV"])
async function publishNoClobber(io: typeof fs, from: string, to: string) {
  const source = await plain(io, from)
  if (!source) throw new Error("Preserved source is missing")
  const expected = await digest(io, from)
  try {
    await io.link(from, to)
  } catch (error) {
    if (!LINK_UNAVAILABLE.has((error as NodeJS.ErrnoException).code ?? ""))
      throw error
    // Not atomic complete-file publication: the owned intent remains until
    // this destination is byte/size verified and synced. A failed/partial copy
    // is never deleted speculatively; startup remains guarded.
    await io.copyFile(from, to, constants.COPYFILE_EXCL)
  }
  const destination = await plain(io, to)
  if (destination?.size !== source.size || (await digest(io, to)) !== expected)
    throw new Error("Published recovery bytes differ")
  const handle = await io.open(to, "r+")
  try {
    await handle.sync()
  } finally {
    await handle.close()
  }
}
const restoreNeedsRecovery = () =>
  new Error(
    `Database restore needs recovery. Database files may be split between the runtime and a gdl_conf.restore-originals-* folder. Preserve both locations and ${INTENT}; contact support before restarting.`
  )

/** Called before actual core startup. Multi-file interrupted moves must never
 * let SQLite create/open a partial original. Retain the intent and bundle. */
export async function assertDatabaseRestoreReady(
  root: string,
  io: typeof fs = fs
): Promise<void> {
  if (await exists(io, path.join(root, INTENT))) throw restoreNeedsRecovery()
}

/** `assertDatabaseRestoreReady` without awaiting, for the core spawn, which
 * must not wait on the event loop (see `loadCoreModule`). */
export function assertDatabaseRestoreReadySync(root: string): void {
  try {
    lstatSync(path.join(root, INTENT))
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return
    throw error
  }
  throw restoreNeedsRecovery()
}

export async function restoreDatabaseSnapshot(
  root: string,
  snapshot: string,
  io: {
    stopCore(): Promise<void>
    restart(recoveryPath: string): void | Promise<void>
    files?: typeof fs
  }
): Promise<RestoreOutcome> {
  const files = io.files ?? fs
  const expected = path.join(root, "gdl_conf.pre-downgrade.db")
  if (
    typeof snapshot !== "string" ||
    path.resolve(snapshot) !== path.resolve(expected)
  ) {
    return {
      ok: false,
      message:
        "Choose the previous database snapshot beside this runtime and retry."
    }
  }
  let bundle: string | undefined
  let ownsIntent = false,
    published = false
  const moved: string[] = []
  const captured = new Map<
    string,
    { dev: number; ino: number; size: number; sha256: string }
  >()
  try {
    await assertDatabaseRestoreReady(root, files)
    const rootStat = await files.lstat(root)
    if (!rootStat.isDirectory() || rootStat.isSymbolicLink())
      throw new Error("Runtime must be a regular directory")
    if (!(await plain(files, expected)))
      return {
        ok: false,
        message:
          "The previous database snapshot is missing. Restore it beside this runtime and retry."
      }
    // Validate before stopping; recapture only after observed core exit.
    for (const name of NAMES) await plain(files, path.join(root, name))
    await io.stopCore()
    for (const name of NAMES) {
      const stat = await plain(files, path.join(root, name))
      if (stat)
        captured.set(name, {
          dev: stat.dev,
          ino: stat.ino,
          size: stat.size,
          sha256: await digest(files, path.join(root, name))
        })
    }
    bundle = path.join(root, `gdl_conf.restore-originals-${randomUUID()}`)
    await files.mkdir(bundle)
    const staged = path.join(bundle, "snapshot-staged.db")
    await files.copyFile(expected, staged, constants.COPYFILE_EXCL)
    const stage = await files.open(staged, "r+")
    try {
      await stage.sync()
    } finally {
      await stage.close()
    }
    const intent = await files.open(path.join(root, INTENT), "wx", 0o600)
    ownsIntent = true
    try {
      await intent.writeFile(
        JSON.stringify({
          version: 1,
          bundle: path.basename(bundle),
          originals: Object.fromEntries(captured)
        })
      )
      await intent.sync()
    } finally {
      await intent.close()
    }
    // Every original is retained, including WAL/SHM. Required failures stop publication.
    for (const [name, identity] of captured) {
      const original = path.join(root, name)
      const stat = await plain(files, original)
      if (
        stat?.dev !== identity.dev ||
        stat.ino !== identity.ino ||
        (await digest(files, original)) !== identity.sha256
      ) {
        throw new Error("Database files changed during restore")
      }
      await files.rename(original, path.join(bundle, name))
      moved.push(name)
    }
    for (const name of NAMES)
      if (await exists(files, path.join(root, name)))
        throw new Error("Database path became occupied during restore")
    // The active namespace is now empty. Prefer atomic no-clobber linking;
    // link-unavailable fallback retains guarded intent through complete copy.
    await publishNoClobber(files, staged, path.join(root, "gdl_conf.db"))
    published = true
    await files.unlink(path.join(root, INTENT))
    ownsIntent = false
    await io.restart(bundle)
    return { ok: true, recoveryPath: bundle }
  } catch {
    let recoveryNeeded = published
    if (ownsIntent && !published && bundle) {
      for (const name of moved) {
        try {
          if (await exists(files, path.join(root, name))) {
            recoveryNeeded = true
            continue
          }
          const preserved = path.join(bundle, name)
          const identity = captured.get(name)!
          const stat = await plain(files, preserved)
          if (
            stat?.dev !== identity.dev ||
            stat.ino !== identity.ino ||
            (await digest(files, preserved)) !== identity.sha256
          ) {
            recoveryNeeded = true
            continue
          }
          await publishNoClobber(files, preserved, path.join(root, name))
        } catch {
          recoveryNeeded = true
        }
      }
      // Failed publication may have entered the active namespace before its
      // verified completion. Reconcile every captured presence AND absence,
      // including originals that never moved, before retiring startup intent.
      for (const name of NAMES) {
        try {
          const original = path.join(root, name)
          const stat = await plain(files, original),
            identity = captured.get(name)
          if (!identity) {
            if (stat) recoveryNeeded = true
            continue
          }
          if (
            stat?.size !== identity.size ||
            (await digest(files, original)) !== identity.sha256 ||
            (!moved.includes(name) &&
              (stat.dev !== identity.dev || stat.ino !== identity.ino))
          )
            recoveryNeeded = true
        } catch {
          recoveryNeeded = true
        }
      }
      if (!recoveryNeeded) {
        try {
          await files.unlink(path.join(root, INTENT))
          ownsIntent = false
        } catch {
          recoveryNeeded = true
        }
      }
    }
    return {
      ok: false,
      recoveryPath: bundle,
      message:
        recoveryNeeded || ownsIntent
          ? `Database restore needs manual recovery; active paths may contain partial or foreign bytes. Preserve the active files and recovery bundle at ${bundle} and contact support before restarting.`
          : "The database could not be restored. Current database files were retained; check the snapshot and file permissions, then retry."
    }
  }
}
