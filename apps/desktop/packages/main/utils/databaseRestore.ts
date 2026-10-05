import fs from "node:fs/promises"
import { constants, lstatSync } from "node:fs"
import path from "node:path"
import { createHash, randomUUID } from "node:crypto"

export type RestoreOutcome =
  | { ok: true; recoveryPath: string }
  | { ok: false; message: string; recoveryPath?: string }
/** What the recovery screen shows after Recover Database. */
export type RestoreRecoveryOutcome =
  | { ok: true; restored: boolean; recoveryPath?: string }
  | { ok: false; message: string; recoveryPath?: string }
/** A refusal already worded for the user: shown as it is, with no generic advice. */
export class DatabaseRefusal extends Error {}
const INTENT = ".gdl-db-restore.json"
const NAMES = ["gdl_conf.db", "gdl_conf.db-wal", "gdl_conf.db-shm"]
const SNAPSHOT = "gdl_conf.pre-downgrade.db"
const STAGED = "snapshot-staged.db"
const BUNDLE_NAME = /^gdl_conf\.restore-originals-[^\\/]+$/
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
/**
 * The data folder's real location, once. It may be reached through a link or
 * junction, as the core does; only the database files inside it must be plain.
 * A volume that cannot report a final path answers realpath with an error
 * other than ENOENT, so stat decides whether the folder can be used as named.
 */
export async function resolveRuntimeRoot(
  root: string,
  io: typeof fs = fs
): Promise<string> {
  const resolved = await io.realpath(root).catch(() => root)
  const stat = await io.stat(resolved).catch(() => null)
  if (!stat?.isDirectory())
    throw new DatabaseRefusal(
      `The data folder ${root} cannot be reached. If it is on another drive or behind a link, reconnect it, then retry.`
    )
  return resolved
}
export async function syncDirectory(files: typeof fs, directory: string) {
  // Node does not support a portable Windows directory flush. File sync remains required.
  if (process.platform === "win32") return
  const handle = await files.open(directory, "r")
  try {
    await handle.sync()
  } finally {
    await handle.close()
  }
}
const inMegabytes = (bytes: number) => `${Math.ceil(bytes / (1024 * 1024))} MB`
/**
 * Says what is needed before a copy of `bytes` starts, instead of failing part
 * way through it. A drive that cannot tell (statfs fails, or reports no blocks)
 * is left to the copy, which still fails safely.
 */
export async function assertRoomForCopy(
  io: typeof fs,
  root: string,
  bytes: number,
  action: string
) {
  const stats = await io.statfs(root).catch(() => null)
  if (!stats?.blocks) return
  const free = stats.bavail * stats.bsize
  if (free < bytes)
    throw new DatabaseRefusal(
      `${action} needs ${inMegabytes(bytes)} free on the drive of ${root} to keep a copy of the database, and it has ${inMegabytes(free)}. Free up some space, then retry.`
    )
}
// Only inability to create a hard link permits exclusive-copy compatibility.
// EPERM (Linux FAT and exFAT), EISDIR (Windows FAT and exFAT) and EINVAL are how
// volumes without hard links answer, as util/publish.rs no_replace_unsupported
// knows. A real permission problem among them fails the guarded copy the same way.
// Disk/IO failures, EACCES and EEXIST are never treated as capability errors.
const LINK_UNAVAILABLE = new Set([
  "ENOTSUP",
  "EOPNOTSUPP",
  "ENOSYS",
  "EXDEV",
  "EPERM",
  "EISDIR",
  "EINVAL"
])
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
/**
 * Whether the data folder's volume can hard-link. Asked before a restore starts:
 * without links, publishing the snapshot takes a second whole copy. Any other
 * failure (EACCES, EIO) comes out here, before an original has moved.
 */
async function canHardLink(io: typeof fs, root: string) {
  const probe = path.join(root, `gdl_conf.restore-link-probe-${randomUUID()}`)
  const link = `${probe}-link`
  await io.writeFile(probe, "", { flag: "wx" })
  try {
    await io.link(probe, link)
    return true
  } catch (error) {
    if (!LINK_UNAVAILABLE.has((error as NodeJS.ErrnoException).code ?? ""))
      throw error
    return false
  } finally {
    for (const file of [link, probe]) await io.unlink(file).catch(() => {})
  }
}
/**
 * Puts an original back from the bundle after a failed restore. A rename within
 * the folder tree is atomic and needs no space, so a crash leaves the whole file
 * on one side or the other. The guarded copy is for when the rename is refused;
 * a part of it left behind is what Recover Database knows to remove.
 */
async function putBack(io: typeof fs, from: string, to: string) {
  try {
    await io.rename(from, to)
  } catch {
    await publishNoClobber(io, from, to)
  }
}
/** Windows may hold a just-closed file for a moment (antivirus, indexing). */
async function retireIntent(io: typeof fs, root: string) {
  for (let attempt = 1; ; attempt++) {
    try {
      await io.unlink(path.join(root, INTENT))
      return
    } catch (error) {
      if (attempt === 3) throw error
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
  }
}
const restoreNeedsRecovery = () =>
  new DatabaseRefusal(
    `Database restore needs recovery. Database files may be split between the data folder and a gdl_conf.restore-originals-* folder. Restart GDLauncher if you do not see Recover Database on this screen, then choose it. Preserve both locations and ${INTENT}; if it cannot recover them, contact support.`
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

/**
 * Whether `snapshot`, as the renderer got it from the core, names the snapshot
 * beside this runtime. The core reports the folder's real location, so a linked
 * data folder gives a different string for the same file. Only compared: the
 * restore always reads the snapshot from `root`.
 */
async function isRuntimeSnapshot(
  snapshot: unknown,
  givenRoot: string,
  root: string,
  io: typeof fs
) {
  if (typeof snapshot !== "string" || path.basename(snapshot) !== SNAPSHOT)
    return false
  const folder = path.dirname(path.resolve(snapshot))
  return (
    folder === path.resolve(givenRoot) ||
    folder === path.resolve(root) ||
    (await io.realpath(folder).catch(() => null)) === root
  )
}

export async function restoreDatabaseSnapshot(
  givenRoot: string,
  snapshot: string,
  io: {
    stopCore(): Promise<void>
    restart(recoveryPath: string): void | Promise<void>
    files?: typeof fs
  }
): Promise<RestoreOutcome> {
  const files = io.files ?? fs
  let root = givenRoot
  let bundle: string | undefined
  let ownsIntent = false,
    published = false
  const moved: string[] = []
  const captured = new Map<
    string,
    { dev: number; ino: number; size: number; sha256: string }
  >()
  try {
    root = await resolveRuntimeRoot(givenRoot, files)
    if (!(await isRuntimeSnapshot(snapshot, givenRoot, root, files)))
      return {
        ok: false,
        message:
          "Choose the previous database snapshot beside this runtime and retry."
      }
    const expected = path.join(root, SNAPSHOT)
    await assertDatabaseRestoreReady(root, files)
    const snapshotStat = await plain(files, expected)
    if (!snapshotStat)
      return {
        ok: false,
        message:
          "The previous database snapshot is missing. Restore it beside this runtime and retry."
      }
    // Validate before stopping; recapture only after observed core exit.
    for (const name of NAMES) await plain(files, path.join(root, name))
    // The staged copy, and on a volume without hard links its publication too.
    await assertRoomForCopy(
      files,
      root,
      snapshotStat.size * ((await canHardLink(files, root)) ? 1 : 2),
      "Restore"
    )
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
    const staged = path.join(bundle, STAGED)
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
    await retireIntent(files, root)
    ownsIntent = false
    await io.restart(bundle)
    return { ok: true, recoveryPath: bundle }
  } catch (error) {
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
          // Size and bytes, not the file number: Windows FAT volumes give a
          // file moved to another folder a new one.
          if (
            stat?.size !== identity.size ||
            (await digest(files, preserved)) !== identity.sha256
          ) {
            recoveryNeeded = true
            continue
          }
          await putBack(files, preserved, path.join(root, name))
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
          await syncDirectory(files, root)
          await retireIntent(files, root)
          ownsIntent = false
        } catch {
          recoveryNeeded = true
        }
      }
    }
    const detail = error instanceof Error ? error.message : "Operation failed"
    return {
      ok: false,
      recoveryPath: bundle,
      message:
        recoveryNeeded || ownsIntent
          ? `Database restore needs manual recovery; active paths may contain partial or foreign bytes. Preserve the active files and recovery bundle at ${bundle}. Restart GDLauncher and choose Recover Database; if it cannot recover them, contact support.`
          : error instanceof DatabaseRefusal
            ? error.message
            : `The database could not be restored. Current database files were retained; check the snapshot and file permissions, then retry. ${detail}`
    }
  }
}

interface RecordedOriginal {
  size: number
  sha256: string
}
/**
 * The intent as a restore wrote it. Null: read, but not JSON. A failed read says
 * nothing about what the marker holds, so it is a refusal the user can retry.
 */
async function readIntent(io: typeof fs, file: string) {
  const stat = await plain(io, file)
  if (!stat || stat.size > 64 * 1024) throw cannotRecover()
  const text = await io.readFile(file, "utf8").catch((error: unknown) => {
    throw new DatabaseRefusal(
      `GDLauncher could not read ${INTENT}, so it changed nothing. Close any program that may be using the data folder (antivirus, backup or sync tools), check that the drive is working, then choose Recover Database again. ${error instanceof Error ? error.message : ""}`.trim()
    )
  })
  let parsed: any
  try {
    parsed = JSON.parse(text)
  } catch {
    return null
  }
  const originals: unknown = parsed?.originals
  if (
    parsed?.version !== 1 ||
    typeof parsed.bundle !== "string" ||
    !BUNDLE_NAME.test(parsed.bundle) ||
    typeof originals !== "object" ||
    originals === null
  )
    throw cannotRecover()
  const recorded = new Map<string, RecordedOriginal>()
  for (const [name, value] of Object.entries(originals)) {
    if (
      !NAMES.includes(name) ||
      typeof value?.size !== "number" ||
      typeof value?.sha256 !== "string"
    )
      throw cannotRecover()
    recorded.set(name, { size: value.size, sha256: value.sha256 })
  }
  return { bundle: parsed.bundle as string, originals: recorded }
}
const cannotRecover = () =>
  new DatabaseRefusal(
    `GDLauncher cannot tell which database files are yours, so it changed nothing. Keep ${INTENT}, the database files and any gdl_conf.restore-originals-* folder, and ask for help on Discord.`
  )
/**
 * Whether `file` holds nothing but the first bytes of `whole`, as an interrupted
 * copy leaves: cut short, or at full size with the rest still zero, since Windows
 * sets a copy's size first. Removing it loses nothing `whole` does not hold.
 */
async function isPartialCopyOf(io: typeof fs, file: string, whole: string) {
  if ((await io.lstat(file)).size > (await io.lstat(whole)).size) return false
  const [partial, complete] = [
    await io.readFile(file),
    await io.readFile(whole)
  ]
  let end = partial.length
  while (end > 0 && partial[end - 1] === 0) end--
  return partial.subarray(0, end).equals(complete.subarray(0, end))
}
/** Whether any bundle in the data folder holds a database file, which got there by moving. */
async function anyOriginalMoved(io: typeof fs, root: string) {
  for (const name of await io.readdir(root)) {
    if (!BUNDLE_NAME.test(name)) continue
    const stat = await io.lstat(path.join(root, name))
    if (!stat.isDirectory()) continue
    const entries = await io.readdir(path.join(root, name))
    if (entries.some((entry) => NAMES.includes(entry))) return true
  }
  return false
}

/**
 * The way out of the startup guard (`assertDatabaseRestoreReady`): looks at what
 * an interrupted restore left and, only when every file can be verified against
 * its recorded size and hash, finishes the restore or undoes it. Anything else is
 * refused with nothing changed. The bundle always stays.
 */
export async function recoverInterruptedRestore(
  givenRoot: string,
  io: {
    stopCore(): Promise<void>
    restart(outcome: {
      restored: boolean
      recoveryPath?: string
    }): void | Promise<void>
    files?: typeof fs
  }
): Promise<RestoreRecoveryOutcome> {
  const files = io.files ?? fs
  let recoveryPath: string | undefined
  try {
    await io.stopCore()
    const root = await resolveRuntimeRoot(givenRoot, files)
    const at = (name: string) => path.join(root, name)
    const matches = async (file: string, recorded?: RecordedOriginal) => {
      const stat = await plain(files, file)
      return (
        !!stat &&
        !!recorded &&
        stat.size === recorded.size &&
        (await digest(files, file)) === recorded.sha256
      )
    }
    if (!(await exists(files, at(INTENT)))) {
      await io.restart({ restored: false })
      return { ok: true, restored: false }
    }
    const intent = await readIntent(files, at(INTENT))
    if (!intent) {
      // A restore syncs its intent before it moves anything, so a marker cut
      // short means nothing moved, unless the drive ignored that sync: an
      // original in a bundle says otherwise, and so does a missing database.
      if (
        !(await plain(files, at(NAMES[0]))) ||
        (await anyOriginalMoved(files, root))
      )
        throw cannotRecover()
      await retireIntent(files, root)
      await io.restart({ restored: false })
      return { ok: true, restored: false }
    }
    const asRecorded = async () => {
      for (const name of NAMES) {
        const recorded = intent.originals.get(name)
        if (
          recorded
            ? !(await matches(at(name), recorded))
            : await plain(files, at(name))
        )
          return false
      }
      return true
    }
    const finish = async (restored: boolean) => {
      await syncDirectory(files, root)
      await retireIntent(files, root)
      await syncDirectory(files, root)
      await io.restart({ restored, recoveryPath })
      return { ok: true as const, restored, recoveryPath }
    }
    // Every file is as the restore found it, so only the marker is left to undo.
    // This needs nothing from the bundle, which the user may have tidied away.
    if (await asRecorded()) return await finish(false)
    const bundle = at(intent.bundle)
    const bundleStat = await exists(files, bundle)
    if (!bundleStat?.isDirectory() || bundleStat.isSymbolicLink())
      throw cannotRecover()
    recoveryPath = bundle
    const staged = path.join(bundle, STAGED)
    const stagedStat = await plain(files, staged)
    if (!stagedStat) throw cannotRecover()
    const snapshot = {
      size: stagedStat.size,
      sha256: await digest(files, staged)
    }
    // Decide everything first: a refusal below leaves every file as it is.
    const moveBack: string[] = []
    const remove: string[] = []
    let inBundle = 0
    for (const name of NAMES) {
      const recorded = intent.originals.get(name)
      const active = await plain(files, at(name))
      if (recorded && (await matches(at(name), recorded))) continue
      if (recorded) {
        if (!(await matches(path.join(bundle, name), recorded)))
          throw cannotRecover()
        inBundle++
        moveBack.push(name)
      }
      if (!active) continue
      // An interrupted copy back from the bundle, which holds all of its bytes
      // and was verified above. Otherwise only the restored database can sit
      // here, whole or as an interrupted copy of the staged file.
      if (
        !(
          recorded &&
          (await isPartialCopyOf(files, at(name), path.join(bundle, name)))
        ) &&
        (name !== NAMES[0] ||
          !(
            (await matches(at(name), snapshot)) ||
            (await isPartialCopyOf(files, at(name), staged))
          ))
      )
        throw cannotRecover()
      remove.push(name)
    }
    // Every original is in the bundle and the whole snapshot is in place:
    // only the intent was left to remove.
    const restored =
      inBundle === intent.originals.size &&
      remove.length === 1 &&
      (await matches(at(NAMES[0]), snapshot))
    if (!restored) {
      for (const name of remove) await files.unlink(at(name))
      for (const name of moveBack)
        await files.rename(path.join(bundle, name), at(name))
      if (!(await asRecorded())) throw cannotRecover()
    }
    return await finish(restored)
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Operation failed"
    return {
      ok: false,
      recoveryPath,
      message:
        error instanceof DatabaseRefusal
          ? error.message
          : `The database could not be recovered. ${detail}`
    }
  }
}
