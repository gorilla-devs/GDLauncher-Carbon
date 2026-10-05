import fs from "node:fs/promises"
import { constants } from "node:fs"
import path from "node:path"
import { createHash } from "node:crypto"
import {
  assertDatabaseRestoreReady,
  assertRoomForCopy,
  resolveRuntimeRoot,
  syncDirectory,
  type RestoreOutcome
} from "./databaseRestore"

const NAMES = [
  "gdl_conf.db",
  "gdl_conf.db-wal",
  "gdl_conf.db-shm",
  "gdl_conf.pre-downgrade.db"
]
interface Identity {
  dev: number
  ino: number
  size: number
  mtimeMs: number
  sha256: string
}
async function plain(files: typeof fs, file: string) {
  try {
    const stat = await files.lstat(file)
    if (!stat.isFile() || stat.isSymbolicLink())
      throw new Error(
        "Reset requires plain database files; links and special files must be recovered manually"
      )
    return stat
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return null
    throw error
  }
}
async function readIdentity(
  files: typeof fs,
  file: string,
  copyTo?: string
): Promise<Identity | null> {
  const before = await plain(files, file)
  if (!before) return null
  const source = await files.open(
    file,
    constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0)
  )
  try {
    const stat = await source.stat()
    if (!stat.isFile() || stat.dev !== before.dev || stat.ino !== before.ino)
      throw new Error("Database file identity changed")
    const target = copyTo ? await files.open(copyTo, "wx", 0o600) : undefined
    const hash = createHash("sha256"),
      buffer = Buffer.alloc(64 * 1024)
    let size = 0
    try {
      for (;;) {
        const { bytesRead } = await source.read(buffer, 0, buffer.length, null)
        if (!bytesRead) break
        hash.update(buffer.subarray(0, bytesRead))
        size += bytesRead
        if (target) {
          let written = 0
          while (written < bytesRead) {
            const result = await target.write(
              buffer,
              written,
              bytesRead - written,
              null
            )
            if (!result.bytesWritten)
              throw new Error("Database recovery copy made no progress")
            written += result.bytesWritten
          }
        }
      }
      if (target) await target.sync()
      const after = await source.stat()
      if (
        after.dev !== stat.dev ||
        after.ino !== stat.ino ||
        after.size !== stat.size ||
        after.mtimeMs !== stat.mtimeMs ||
        size !== stat.size
      ) {
        throw new Error("Database bytes changed during recovery copy")
      }
      return {
        dev: stat.dev,
        ino: stat.ino,
        size,
        mtimeMs: stat.mtimeMs,
        sha256: hash.digest("hex")
      }
    } finally {
      if (target) await target.close()
    }
  } finally {
    await source.close()
  }
}
function same(left: Identity | null, right: Identity | null) {
  return left === null || right === null
    ? left === right
    : left.dev === right.dev &&
        left.ino === right.ino &&
        left.size === right.size &&
        left.mtimeMs === right.mtimeMs &&
        left.sha256 === right.sha256
}

/** The registered IPC entry copies a complete stopped-owned-core file set first.
 * Failed/partial removal keeps the complete bundle; no automatic rollback can
 * overwrite a new occupant. External writers and power loss are not certified. */
export async function resetDatabaseAndRestart(
  givenRoot: string,
  io: {
    stopCore(): Promise<void>
    restart(recoveryPath: string): void | Promise<void>
    files?: typeof fs
  }
): Promise<RestoreOutcome> {
  const files = io.files ?? fs
  let bundle: string | undefined,
    complete = false,
    removalStarted = false,
    removed = 0
  try {
    // Exit acceptance/timeouts/external cores must reject here before filesystem access.
    await io.stopCore()
    const root = await resolveRuntimeRoot(givenRoot, files)
    await assertDatabaseRestoreReady(root, files)
    const captured = new Map<string, Identity | null>()
    for (const name of NAMES)
      captured.set(name, await readIdentity(files, path.join(root, name)))
    await assertRoomForCopy(
      files,
      root,
      [...captured.values()].reduce((sum, file) => sum + (file?.size ?? 0), 0),
      "Reset"
    )
    const date = new Date().toISOString().replace(/[:.]/g, "-")
    bundle = await files.mkdtemp(
      path.join(root, `gdl_conf.reset-recovery-${date}-`)
    )
    await files.chmod(bundle, 0o700)
    await syncDirectory(files, root)
    for (const [name, identity] of captured) {
      if (!identity) continue
      const copied = await readIdentity(
        files,
        path.join(root, name),
        path.join(bundle, name)
      )
      if (!same(copied, identity))
        throw new Error("Database source changed before copy")
      const retained = await readIdentity(files, path.join(bundle, name))
      if (
        retained?.size !== identity.size ||
        retained.sha256 !== identity.sha256
      )
        throw new Error("Recovery copy verification failed")
    }
    const record = await files.open(
      path.join(bundle, "captured-files.json"),
      "wx",
      0o600
    )
    try {
      await record.writeFile(
        JSON.stringify({
          version: 1,
          capturedAt: new Date().toISOString(),
          files: Object.fromEntries(captured)
        })
      )
      await record.sync()
    } finally {
      await record.close()
    }
    await syncDirectory(files, bundle)
    await syncDirectory(files, root)
    // Check the entire namespace before any removal, including captured absences.
    for (const [name, identity] of captured) {
      if (!same(await readIdentity(files, path.join(root, name)), identity))
        throw new Error("Database namespace changed; current files retained")
    }
    complete = true
    for (const [name, identity] of captured) {
      if (!identity) continue
      if (!same(await readIdentity(files, path.join(root, name)), identity))
        throw new Error("Database file changed; current occupant retained")
      removalStarted = true
      await files.unlink(path.join(root, name))
      removed++
    }
    for (const name of NAMES)
      if (await plain(files, path.join(root, name)))
        throw new Error(
          "Database path became occupied; current occupant retained"
        )
    await syncDirectory(files, root)
    await io.restart(bundle)
    return { ok: true, recoveryPath: bundle }
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Operation failed"
    return {
      ok: false,
      recoveryPath: bundle,
      message: complete
        ? `Reset did not finish (${removed} database files removed${removalStarted ? "; active files may be partial" : ""}). Complete recovery copy: ${bundle}. Preserve current files and this copy; resolve the failure before retrying. ${detail}`
        : `Reset did not finish. Current database files were retained.${bundle ? ` Incomplete recovery folder: ${bundle}; preserve it.` : ""} ${detail}`
    }
  }
}
