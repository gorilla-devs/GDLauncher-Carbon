import fs, { type FileHandle } from "node:fs/promises"
import fss from "node:fs"
import childProcess from "node:child_process"
import os from "node:os"
import path from "node:path"
import { promisify } from "node:util"
import { createHash, randomUUID } from "node:crypto"
import {
  assertSafeRuntimeTarget,
  isPathInside,
  RuntimeTargetError,
  type RuntimePathContext,
  type RuntimeTargetRefusal
} from "./runtimePathGuard"
import type { RuntimeBlocker } from "./coreShutdown"

export const MIGRATION_MARKER = ".gdl_migration_in_progress"

/** The destination holds a move that stopped before it finished. */
export class IncompleteMigrationError extends Error {
  constructor(
    readonly target: string,
    // The marker names the data folder in use, so the folder is only a partial copy of it.
    readonly fromSource: boolean
  ) {
    super(
      fromSource
        ? `Incomplete migration at ${target}: this folder holds an unfinished copy of the data folder in use. Delete it or choose another empty folder.`
        : `Incomplete migration at ${target}: this folder holds a move that did not finish. Choose another empty folder.`
    )
    this.name = "IncompleteMigrationError"
  }
}

/** This run marked the target and stopped before finishing; the old folder is untouched and still in use. */
export class UnfinishedCopyError extends Error {
  constructor(
    readonly target: string,
    cause: unknown,
    // Everything this run made in the target was removed again, marker included.
    readonly removed = false
  ) {
    super(cause instanceof Error ? cause.message : String(cause), { cause })
    // Read like the underlying failure ("Error: ENOSPC: ...").
    this.name = cause instanceof Error ? cause.name : "Error"
  }
}

/**
 * Links and special files inside a folder to move. A move copies regular files and
 * deletes the originals, so it cannot carry them over: one left behind would leave the
 * instance that uses it without its files.
 */
export class UnsupportedRuntimeEntriesError extends Error {
  constructor(
    readonly folder: string,
    readonly entries: string[]
  ) {
    super(
      `Unsupported runtime entries (links or special files) in ${folder}: ${entries.join(", ")}`
    )
    this.name = "UnsupportedRuntimeEntriesError"
  }
}

/** The links a failed move named, shortened for the dialog. */
export function unsupportedRuntimeEntries(
  error: unknown
): UnsupportedRuntimeEntries | undefined {
  if (!(error instanceof UnsupportedRuntimeEntriesError)) return undefined
  return {
    folder: error.folder,
    entries: error.entries.slice(0, LEFT_BEHIND_SHOWN),
    count: error.entries.length
  }
}

/** Whether a failed move left the chosen folder holding only a partial copy of the data folder in use. */
export function leftPartialCopy(error: unknown): boolean {
  return error instanceof UnfinishedCopyError
    ? !error.removed
    : error instanceof IncompleteMigrationError && error.fromSource
}

/** Whether a move stopped before it finished and removed what it had copied, leaving the chosen folder as it was. */
export function removedPartialCopy(error: unknown): boolean {
  return error instanceof UnfinishedCopyError && error.removed
}
export interface RuntimeDestinationContext extends RuntimePathContext {
  platform?: NodeJS.Platform
  // Injectable mount table for deterministic policy tests; never renderer input.
  mountInfo?: string
}

const gone = (e: unknown) => (e as NodeJS.ErrnoException).code === "ENOENT"

async function statOrMissing(file: string) {
  try {
    return await fs.lstat(file)
  } catch (e) {
    if (gone(e)) return null
    throw e
  }
}

// Node's own resolver, which reads each link along the path itself.
const realpathByLinks = promisify(fss.realpath)

/**
 * Where `file` really is, links and mapped drives resolved. Windows cannot
 * ask some drives (RAM disks, some network redirectors) where a folder really
 * is, and the native call fails (EISDIR) there; such a path is then resolved
 * a link at a time, which keeps its drive letter. 2.0.40 used these drives.
 */
async function resolvedPath(file: string): Promise<string> {
  try {
    return await fs.realpath(file)
  } catch (e) {
    if (gone(e)) throw e
    return await realpathByLinks(file)
  }
}

export async function canonicalRuntimePath(file: string): Promise<string> {
  try {
    return await resolvedPath(file)
  } catch (e) {
    if (!gone(e)) throw e
    const parent = path.dirname(file)
    if (parent === file) throw e
    return path.join(await canonicalRuntimePath(parent), path.basename(file))
  }
}

export function removableMountFor(
  target: string,
  mountInfo: string
): string | undefined {
  return mountInfo
    .split("\n")
    .map((line) => line.split(" ")[4])
    .filter((mount): mount is string => !!mount)
    .map((mount) =>
      mount.replace(/\\([0-7]{3})/g, (_, octal) =>
        String.fromCharCode(parseInt(octal, 8))
      )
    )
    .filter(
      (mount) =>
        ((mount !== "/run/media" && isPathInside(mount, "/run/media")) ||
          (mount !== "/media" && isPathInside(mount, "/media"))) &&
        target !== mount &&
        isPathInside(target, mount)
    )
    .sort((a, b) => b.length - a.length)[0]
}

/**
 * The mounted drive under /run/media that holds `target`. /run is a system
 * folder, so only a folder on a drive mounted there may be used, never the
 * folder a drive would be mounted on: it is on the system drive while the
 * drive is unplugged. Elsewhere, /media included, nothing is required: a
 * folder there may be a plain folder, the root of a drive or a folder on one,
 * as in 2.0.40.
 */
async function removableMountContext(
  target: string,
  platform: NodeJS.Platform,
  context: RuntimeDestinationContext
): Promise<string | undefined> {
  if (platform !== "linux" || !isPathInside(target, "/run/media"))
    return undefined
  const table =
    context.mountInfo ?? (await fs.readFile("/proc/self/mountinfo", "utf8"))
  const resolved = path.resolve(target)
  const mount = removableMountFor(resolved, table)
  if (!mount) {
    // A drive's own mount point is refused like a drive root elsewhere.
    const root = removableMountFor(path.join(resolved, "x"), table) === resolved
    throw new RuntimeTargetError(
      root ? "filesystem_root" : "removable_not_mounted",
      "Removable drive is not mounted, or target is its root"
    )
  }
  if (!(await fs.stat(mount)).isDirectory())
    throw new Error("Removable mount is not a directory")
  return mount
}

const NOT_WRITABLE = new Set(["EACCES", "EPERM", "EROFS"])

/**
 * Names a refusal the filesystem gave while the folder was checked: one the
 * user may not reach or write, or a path that runs through a file.
 */
function refusedByFilesystem(e: unknown): never {
  const code = (e as NodeJS.ErrnoException).code
  if (code && NOT_WRITABLE.has(code))
    throw new RuntimeTargetError("not_writable", (e as Error).message, {
      cause: e
    })
  if (code === "ENOTDIR")
    throw new RuntimeTargetError("not_a_folder", (e as Error).message, {
      cause: e
    })
  throw e
}

/**
 * Whether a folder that cannot be written is where a drive is mounted while
 * none is, so the user is told to connect the drive instead. macOS mounts
 * every drive in /Volumes/<name>, and /Volumes belongs to root: a missing
 * folder whose nearest existing folder is /Volumes itself is on a drive that
 * is not connected. udisks on Ubuntu and Debian mounts drives in
 * /media/<user>/<label>, and /media/<user> belongs to root; writing there
 * would put the data folder on the system drive. A folder on a mounted drive,
 * one macOS mounts read-only included, is answered as usual.
 */
async function unmountedDrive(
  folder: string,
  canonical: string,
  e: unknown,
  platform: NodeJS.Platform,
  context: RuntimeDestinationContext
): Promise<boolean> {
  const code = (e as NodeJS.ErrnoException).code
  if (!code || !NOT_WRITABLE.has(code)) return false
  const at = path.resolve(folder)
  if (platform === "darwin")
    return at === "/Volumes" && path.resolve(canonical) !== at
  if (platform !== "linux" || !isPathInside(at, "/media")) return false
  const table =
    context.mountInfo ??
    (await fs.readFile("/proc/self/mountinfo", "utf8").catch(() => null))
  // A mount that is the folder itself or holds it.
  return (
    table !== null &&
    !removableMountFor(path.join(path.resolve(canonical), "x"), table)
  )
}

/** Same validation for advisory IPC and the authoritative transaction. Never creates a target. */
export async function validateRuntimeDestination(
  target: string,
  source: string | null,
  context: RuntimeDestinationContext = {}
): Promise<"valid" | "potentially_valid"> {
  const platform = context.platform ?? process.platform
  // Reject relative and unsupported forms before resolving filesystem aliases.
  assertSafeRuntimeTarget(target, source, platform, {
    ...context,
    validatedRemovableMount: await removableMountContext(
      target,
      platform,
      context
    ),
    resolved: false
  })
  // Links and mapped drives resolve to where the data would really go, which
  // is checked again: a link into /run/media needs its drive mounted there.
  const canonical = await canonicalRuntimePath(target).catch((e) => {
    // Not even the root of the path is there: a Windows drive letter that
    // no connected drive has.
    if (gone(e))
      throw new RuntimeTargetError(
        "removable_not_mounted",
        `The drive of this folder is not connected: ${target}`,
        { cause: e }
      )
    return refusedByFilesystem(e)
  })
  const canonicalSource = source ? await canonicalRuntimePath(source) : null
  const installationRoots = await Promise.all(
    (context.installationRoots ?? []).map(canonicalRuntimePath)
  )
  assertSafeRuntimeTarget(canonical, canonicalSource, platform, {
    ...context,
    installationRoots,
    validatedRemovableMount: await removableMountContext(
      canonical,
      platform,
      context
    ),
    resolved: true
  })
  const st = await statOrMissing(target).catch(refusedByFilesystem)
  // A link to a folder is used as that folder, as in 2.0.40; its destination
  // passed the checks above.
  const followed = st?.isSymbolicLink()
    ? await fs.stat(target).catch(() => null)
    : st
  if (st && !followed?.isDirectory())
    throw new RuntimeTargetError(
      "not_a_folder",
      "Runtime target must be a folder, or a link to a folder that exists"
    )
  const marker = path.join(target, MIGRATION_MARKER)
  if (await statOrMissing(marker).catch(refusedByFilesystem)) {
    const from = await markedSource(marker)
    throw new IncompleteMigrationError(
      target,
      !!from && !!source && path.resolve(from) === path.resolve(source)
    )
  }
  // The nearest folder that exists is where the new one would be made.
  const refusedAt = (folder: string) => async (e: unknown) => {
    if (await unmountedDrive(folder, canonical, e, platform, context))
      throw new RuntimeTargetError(
        "removable_not_mounted",
        `No drive is mounted for this folder: ${target}`,
        { cause: e }
      )
    return refusedByFilesystem(e)
  }
  let ancestor = target
  while (!(await statOrMissing(ancestor).catch(refusedAt(ancestor)))) {
    const parent = path.dirname(ancestor)
    if (parent === ancestor)
      throw new RuntimeTargetError(
        "removable_not_mounted",
        `The drive of this folder is not connected: ${target}`
      )
    ancestor = parent
  }
  await fs.access(ancestor, fss.constants.W_OK).catch(refusedAt(ancestor))
  return st && (await fs.readdir(target).catch(refusedByFilesystem)).length
    ? "potentially_valid"
    : "valid"
}

/**
 * What Settings tells the user about a folder validation refused. Anything
 * without a reason of its own (a folder that cannot be read, say) is "invalid".
 */
export function runtimeTargetRefusal(
  error: unknown
): RuntimeTargetRefusal | "invalid" {
  return error instanceof RuntimeTargetError ? error.reason : "invalid"
}

/**
 * The data folder a marker says it was copied from, or null when it does not say:
 * 2.0.40 wrote empty markers, and a marker may be unreadable or not ours. The path is
 * only compared as a string, never opened.
 */
async function markedSource(marker: string): Promise<string | null> {
  try {
    const handle = await fs.open(
      marker,
      fss.constants.O_RDONLY |
        (fss.constants.O_NOFOLLOW ?? 0) |
        (fss.constants.O_NONBLOCK ?? 0)
    )
    try {
      const st = await handle.stat()
      if (!st.isFile() || st.size > 64 * 1024) return null
      const recorded = JSON.parse(await handle.readFile("utf8"))?.source
      return typeof recorded === "string" && path.isAbsolute(recorded)
        ? recorded
        : null
    } finally {
      await handle.close()
    }
  } catch {
    return null
  }
}

interface Entry {
  relative: string
  directory: boolean
}
export async function inventoryRuntimeTree(
  root: string,
  signal?: AbortSignal,
  options: {
    /**
     * `root` is the data folder in use. The pointer already names it, so a
     * marker file at its root is stale: 2.0.40 let users switch into a folder
     * an interrupted move had marked. It is left out of the inventory, so it
     * is neither copied nor deleted, instead of refusing the move.
     */
    liveSource?: boolean
  } = {}
): Promise<Entry[]> {
  const marker = await statOrMissing(path.join(root, MIGRATION_MARKER))
  // Only a plain file, as both 2.0.40 and this version write it, is skipped.
  const staleMarker = !!options.liveSource && !!marker?.isFile()
  if (marker && !staleMarker)
    throw new Error(`Incomplete migration in source: ${root}`)
  const result: Entry[] = []
  const unsupported: string[] = []
  async function visit(relative: string) {
    signal?.throwIfAborted()
    const full = path.join(root, relative)
    // The data folder itself may be reached through a link or junction, as the
    // core does. Only what is inside it has to be real.
    const st = await (relative ? fs.lstat(full) : fs.stat(full))
    if (st.isSymbolicLink() || (!st.isDirectory() && !st.isFile())) {
      unsupported.push(relative)
      return
    }
    if (relative) result.push({ relative, directory: st.isDirectory() })
    if (st.isDirectory())
      for (const name of await fs.readdir(full)) {
        if (staleMarker && !relative && name === MIGRATION_MARKER) continue
        await visit(path.join(relative, name))
      }
  }
  await visit("")
  // All of them at once, so the user fixes them in one go.
  if (unsupported.length)
    throw new UnsupportedRuntimeEntriesError(root, unsupported)
  return result
}

async function syncDirectory(directory: string) {
  // Windows cannot open directories with this API. Atomic rename is still the commit;
  // native power-loss durability is not promised on that platform.
  if (process.platform === "win32") return
  const handle = await fs.open(directory, "r")
  try {
    await handle.sync()
  } finally {
    await handle.close()
  }
}

/** Rename is the commit. A postcommit durability warning must never roll back destination data. */
export async function publishRuntimeOverrideAtomically(
  pointer: string,
  target: string
): Promise<string[]> {
  const temporary = path.join(
    path.dirname(pointer),
    `.${path.basename(pointer)}.${randomUUID()}.tmp`
  )
  let committed = false
  try {
    const file = await fs.open(temporary, "wx", 0o600)
    try {
      await file.writeFile(target, "utf8")
      await file.sync()
    } finally {
      await file.close()
    }
    await fs.rename(temporary, pointer)
    committed = true
    try {
      await syncDirectory(path.dirname(pointer))
      return []
    } catch (e) {
      return [
        `Runtime pointer committed; directory sync failed, source retained: ${String(e)}`
      ]
    }
  } finally {
    if (!committed) await fs.unlink(temporary).catch(() => {})
  }
}

export function selectRuntimePath(
  override: string | null | undefined,
  userData: string,
  /**
   * Told the data folder the pointer names when it cannot be opened (its
   * drive not connected, say), so the user hears why the default one is used.
   */
  unavailable?: (folder: string) => void
): string {
  if (override) return override
  const fallback = path.join(userData, "data")
  let value: string
  try {
    value = fss.readFileSync(
      path.join(userData, "runtime_path_override"),
      "utf8"
    )
  } catch {
    return fallback
  }
  if (!path.isAbsolute(value) || value.includes("\0")) return fallback
  try {
    fss.mkdirSync(value, { recursive: true })
    return value
  } catch {
    if (path.resolve(value) !== path.resolve(fallback)) unavailable?.(value)
    return fallback
  }
}

/**
 * The user keeps the default data folder this start fell back to, instead of
 * `folder`, which the pointer names but could not be opened (a drive that is
 * gone for good, say). The pointer is removed, so later starts open the
 * default folder without a word, as when no folder was ever chosen. Nothing
 * in `folder` is touched: once it can be reached again, Settings switches
 * back to it. A pointer that no longer names `folder` is left as it is.
 */
export async function forgetUnavailableRuntimePath(
  pointer: string,
  folder: string
): Promise<void> {
  if (!(await pointerNames(pointer, folder))) return
  try {
    await fs.unlink(pointer)
  } catch (e) {
    if (gone(e)) return
    throw e
  }
  await syncDirectory(path.dirname(pointer)).catch(() => {})
}

type FileIdentity = Pick<
  fss.Stats,
  "dev" | "ino" | "size" | "mtimeMs" | "ctimeMs"
>

// Any ordinary write or replacement changes one of these.
function statIdentity(st: FileIdentity) {
  return `${st.dev}:${st.ino}:${st.size}:${st.mtimeMs}:${st.ctimeMs}`
}

async function regularFileStat(file: string) {
  const st = await fs.lstat(file)
  if (!st.isFile() || st.isSymbolicLink())
    throw new Error(`Source changed type: ${file}`)
  return st
}

/** The file's identity, read before its bytes, and the SHA-256 of those bytes. */
async function fingerprint(file: string) {
  const st = await regularFileStat(file)
  const hash = createHash("sha256")
  const handle = await fs.open(
    file,
    fss.constants.O_RDONLY | (fss.constants.O_NOFOLLOW ?? 0)
  )
  try {
    for await (const chunk of handle.createReadStream({ autoClose: false }))
      hash.update(chunk)
  } finally {
    await handle.close()
  }
  const { dev, ino, size, mtimeMs, ctimeMs } = st
  return { st: { dev, ino, size, mtimeMs, ctimeMs }, hash: hash.digest("hex") }
}

interface CopiedFile {
  relative: string
  // The original as it was copied; the copy holds exactly these bytes.
  st: FileIdentity
  hash: string
}

/**
 * Whether the original still holds exactly what its verified copy holds, so deleting it
 * loses nothing. Same inode, size and times is the usual answer. A new inode number
 * or change time alone is not a change: deleting another hard link to the file (an earlier
 * one in this move, or the staged copy a database restore keeps) updates the change time,
 * and Linux numbers FAT and exFAT files again when it reloads them. Then the bytes must still
 * match and the file must still be reached through the old folder itself, not through a
 * folder swapped since for a link into the new one.
 */
async function unchangedSinceCopy(
  from: string,
  copied: CopiedFile,
  canonicalSource: string
): Promise<boolean> {
  // Untouched since its copy was checked: any write would have changed one of
  // these, so its bytes are not read again.
  if (statIdentity(await regularFileStat(from)) === statIdentity(copied.st))
    return true
  const current = await fingerprint(from)
  if (current.hash !== copied.hash) return false
  return (
    current.st.dev === copied.st.dev &&
    current.st.size === copied.st.size &&
    current.st.mtimeMs === copied.st.mtimeMs &&
    (await resolvedPath(path.dirname(from))) ===
      path.join(canonicalSource, path.dirname(copied.relative))
  )
}

/**
 * Whether two stats name the same file, unchanged as far as its size and
 * modification time tell. The change time is left out: Windows changes it when
 * OneDrive downloads an online-only file so that it can be read, which copying
 * it does. The bytes are compared as well.
 */
function sameFile(a: FileIdentity, b: FileIdentity) {
  return (
    a.dev === b.dev &&
    a.ino === b.ino &&
    a.size === b.size &&
    a.mtimeMs === b.mtimeMs
  )
}

const CHUNK = 1 << 20

async function readFull(handle: FileHandle, buffer: Buffer) {
  let filled = 0
  while (filled < buffer.length) {
    const { bytesRead } = await handle.read(
      buffer,
      filled,
      buffer.length - filled,
      null
    )
    if (!bytesRead) break
    filled += bytesRead
  }
  return filled
}

/**
 * Reads an original and its copy side by side: the SHA-256 of their bytes when
 * they are the same, otherwise null. `buffers` are two CHUNK-sized buffers.
 */
async function sameBytes(
  from: string,
  to: string,
  buffers: [Buffer, Buffer]
): Promise<string | null> {
  const flags = fss.constants.O_RDONLY | (fss.constants.O_NOFOLLOW ?? 0)
  const hash = createHash("sha256")
  const original = await fs.open(from, flags)
  try {
    const copy = await fs.open(to, flags)
    try {
      const [left, right] = buffers
      for (;;) {
        const [n, m] = await Promise.all([
          readFull(original, left),
          readFull(copy, right)
        ])
        if (n !== m || !left.subarray(0, n).equals(right.subarray(0, m)))
          return null
        if (!n) return hash.digest("hex")
        hash.update(left.subarray(0, n))
      }
    } finally {
      await copy.close()
    }
  } finally {
    await original.close()
  }
}

/**
 * Copies one file and checks the copy: it must hold exactly what the original
 * holds once copied, and nothing may have written to the original meanwhile.
 * Returns the original's identity and the SHA-256 of the copied bytes.
 */
async function copyVerified(
  from: string,
  to: string,
  relative: string,
  buffers: [Buffer, Buffer]
): Promise<{ st: FileIdentity; hash: string }> {
  const before = await regularFileStat(from)
  await fs.copyFile(from, to, fss.constants.COPYFILE_EXCL)
  const after = await regularFileStat(from)
  const hash = sameFile(before, after)
    ? await sameBytes(from, to, buffers)
    : null
  if (hash === null) throw new Error(`Source changed during copy: ${relative}`)
  const { dev, ino, size, mtimeMs, ctimeMs } = after
  return { st: { dev, ino, size, mtimeMs, ctimeMs }, hash }
}

/**
 * Runs `work` on every item, at most `limit` at a time. After a failure no
 * new item starts; the first failure is thrown once the running ones end, so
 * nothing is still writing when the caller cleans up.
 */
async function eachLimited<T>(
  items: T[],
  limit: number,
  work: (item: T, worker: number) => Promise<void>
) {
  let next = 0
  let failure: { error: unknown } | undefined
  await Promise.all(
    Array.from({ length: Math.min(limit, items.length) }, async (_, worker) => {
      while (!failure && next < items.length) {
        const item = items[next++]
        try {
          await work(item, worker)
        } catch (error) {
          failure ??= { error }
        }
      }
    })
  )
  if (failure) throw failure.error
}

/**
 * Files copied, checked and flushed at once. Each flush waits for the drive
 * (on macOS for its whole cache), so several at a time keep the copy moving;
 * Node runs four file operations at once by default.
 */
const COPY_WORKERS = 4

/** What a move made in the new folder before it finished, so a failure can remove it. */
interface MadeInTarget {
  /**
   * The folder that held the new folder before the move (the new folder
   * itself when it was already there) and its drive. Gone, or on another
   * drive, means the drive was disconnected, and the copy is still on it.
   */
  anchor?: { folder: string; dev: number }
  // The new folder and the missing folders above it, deepest first.
  newDirectories: string[]
  // Folders and files of the copy, in the order they were made. A file is
  // listed before its copy starts: a failed copy may leave part of it.
  directories: string[]
  files: string[]
  // A copy that failed because its name was already taken is not ours.
  notOurs: Set<string>
  marker: boolean
}

/**
 * Marks `target` as holding an unfinished copy of `source`, on disk before
 * anything else is written there or removed from it.
 */
async function markUnfinished(
  target: string,
  source: string,
  made: MadeInTarget
) {
  const handle = await fs.open(path.join(target, MIGRATION_MARKER), "wx")
  made.marker = true
  try {
    // Names the folder being copied, so a later check can tell this partial copy
    // from a marked folder that someone may have used as their data folder.
    await handle.writeFile(JSON.stringify({ version: 1, source }))
    await handle.sync()
  } finally {
    await handle.close()
  }
}

/** Whether the folder that held the new folder is still where it was. */
async function anchorReachable(made: MadeInTarget) {
  if (!made.anchor) return true
  const st = await fs.stat(made.anchor.folder).catch(() => null)
  return !!st?.isDirectory() && st.dev === made.anchor.dev
}

/**
 * Removes what an unfinished move made in the new folder: the copies, the
 * folders, the marker, then the folders made to hold the new folder, which
 * are left if something else was put in them. Returns whether the copy is
 * gone. The marker is there the whole time, written again first if the
 * complete copy had lost it, and stays if any of the copy does, so the
 * folder is never taken for a complete one. A full drive may refuse even
 * the marker; the copy is removed all the same, since that is when its space
 * is needed most, and marked if any of it stays. A disconnected drive keeps
 * its copy, so that is not reported as removed either.
 */
async function removeUnfinishedCopy(
  made: MadeInTarget,
  target: string,
  source: string,
  progress: RuntimeRelocationOptions["progress"]
): Promise<boolean> {
  const marker = path.join(target, MIGRATION_MARKER)
  const files = made.files.filter((file) => !made.notOurs.has(file)).reverse()
  // Already disconnected: nothing on it can be removed, so do not try each file.
  if (!(await anchorReachable(made))) return false
  const mark = async () => {
    if (made.marker || !(files.length || made.directories.length)) return
    try {
      await markUnfinished(target, source, made)
      await syncDirectory(target)
    } catch {
      // Removed below all the same; an empty marker left by a failed write
      // still keeps the folder from being taken for a complete one.
    }
  }
  await mark()
  let left = false
  for (const [index, file] of files.entries()) {
    progress?.({
      action: "rollback",
      currentName: path.relative(target, file),
      current: index,
      total: files.length
    })
    await fs.unlink(file).catch((e) => {
      if (!gone(e)) left = true
    })
  }
  for (const directory of [...made.directories].reverse())
    await fs.rmdir(directory).catch((e) => {
      if (!gone(e)) left = true
    })
  if (!(await anchorReachable(made))) return false
  if (left) {
    // The space the removed part took may let the marker be written now.
    await mark()
    return false
  }
  if (made.marker) {
    try {
      await fs.unlink(marker)
    } catch (e) {
      if (!gone(e)) return false
    }
  }
  for (const directory of made.newDirectories)
    await fs.rmdir(directory).catch(() => {})
  return true
}

/** Whether the pointer names `target`; null when it cannot be read. */
async function pointerNames(
  pointer: string,
  target: string
): Promise<boolean | null> {
  try {
    return (await fs.readFile(pointer, "utf8")) === target
  } catch (e) {
    return gone(e) ? false : null
  }
}

/**
 * Flushes a fresh copy to disk. fs.copyFile keeps the source permissions, so a
 * read-only source (GDL's managed Java ships 0444 files; on Windows, the
 * Read-only attribute) gives a read-only copy, and flushing needs a write handle
 * (Windows FlushFileBuffers requires one). Owner-write is granted only around the
 * sync and the copied mode is put back; a failed restore aborts the move.
 */
async function syncCopiedFile(file: string) {
  const mode = (await fs.stat(file)).mode & 0o7777
  const readOnly = (mode & 0o200) === 0
  if (readOnly) await fs.chmod(file, mode | 0o200)
  try {
    const handle = await fs.open(file, "r+")
    try {
      await handle.sync()
    } finally {
      await handle.close()
    }
  } finally {
    if (readOnly) await fs.chmod(file, mode)
  }
}

/**
 * Filesystems whose syncfs writes out every file and folder and then has the
 * drive flush its cache, as fsync does for one file: ext2/3/4, XFS, Btrfs and
 * F2FS (statfs magic numbers). FAT, exFAT, NTFS, FUSE and network
 * filesystems are flushed file by file.
 */
const FLUSHED_WHOLE = new Set([0xef53, 0x58465342, 0x9123683e, 0xf2f52010])

/**
 * Whether the copy in `folder` can be made durable with one flush of its
 * whole filesystem (Linux syncfs) instead of one per file, which on a hard
 * drive, USB drive or SD card costs several milliseconds a file. Only Linux
 * 5.8 and later tells syncfs about a write that failed in the meantime.
 */
async function flushesWhole(folder: string): Promise<boolean> {
  if (process.platform !== "linux") return false
  const [major, minor] = os
    .release()
    .split(".")
    .map((part) => parseInt(part, 10))
  if (!(major > 5 || (major === 5 && minor >= 8))) return false
  try {
    return FLUSHED_WHOLE.has((await fs.statfs(folder)).type)
  } catch {
    return false
  }
}

/**
 * Flushes the whole filesystem holding `folder`, as `sync -f` does (coreutils
 * 8.24 and later). Returns whether that worked; if not, each file is flushed.
 */
function flushFilesystem(folder: string): Promise<boolean> {
  return new Promise((resolve) => {
    try {
      childProcess.execFile("sync", ["-f", folder], (error) => resolve(!error))
    } catch {
      resolve(false)
    }
  })
}

export interface RuntimeRelocationOptions {
  source: string
  target: string
  pointer: string
  switchOnly?: boolean
  context?: RuntimeDestinationContext
  /**
   * Asks the core, without stopping or freezing anything, whether something
   * would refuse the move, so a refusal comes before the slow folder scan.
   */
  checkCore?: () => Promise<void>
  // Must atomically freeze core admission, acknowledge shutdown, then observe actual exit.
  // It refuses while any game, server, install or adopted process is live: deleting the
  // copied originals relies on no launcher writer being left.
  stopCore: () => Promise<void>
  /**
   * Lets the user's quit or window close end the move while nothing has
   * changed: the scan before the core is stopped watches `signal`, and
   * `commit()` runs right before `stopCore`.
   */
  interruptible?: { signal: AbortSignal; commit(): void }
  /**
   * "check" is the scan before the core is stopped: closing the app then
   * ends the move with nothing moved. From "scan" on the core is stopping or
   * stopped and the move runs to its end.
   */
  progress?: (value: {
    // "flush" writes the finished copy to the drive; "rollback" removes
    // what a move that stopped had copied.
    action: "check" | "scan" | "copy" | "flush" | "remove" | "rollback"
    currentName: string
    current: number
    total: number
  }) => void
}

export async function relocateRuntime(
  options: RuntimeRelocationOptions
): Promise<RuntimeRelocationOutcome> {
  const { source, target, pointer, context, stopCore, progress } = options
  const check = async () => {
    const classification = await validateRuntimeDestination(
      target,
      source,
      context
    )
    if (options.switchOnly) {
      if (classification !== "potentially_valid")
        throw new Error("Switch target is empty; choose a full move instead")
    } else if (classification !== "valid")
      throw new Error(
        "Destination is populated; confirm switching to it instead"
      )
  }
  const signal = options.interruptible?.signal
  await check()
  // Refuse while something runs before making the user wait for the scan.
  await options.checkCore?.()
  signal?.throwIfAborted()
  progress?.({ action: "check", currentName: "", current: 0, total: 0 })
  // A switch copies nothing and never touches the old folder, so what either
  // folder holds does not matter: `check` already refused an unfinished move.
  if (!options.switchOnly)
    await inventoryRuntimeTree(source, signal, { liveSource: true })
  // Last point where closing the app leaves everything as it was.
  options.interruptible?.commit()
  progress?.({ action: "scan", currentName: "", current: 0, total: 0 })
  await stopCore()
  await check()
  if (options.switchOnly)
    return {
      warnings: await publishRuntimeOverrideAtomically(pointer, target),
      changed: [],
      notDeleted: [],
      keptOldFolder: false
    }
  const entries = await inventoryRuntimeTree(source, undefined, {
    liveSource: true
  })
  // The old folder as the remove phase must still reach it: nothing inside is a link.
  const canonicalSource = await resolvedPath(source)
  const fileCount = entries.filter((entry) => !entry.directory).length
  progress?.({
    action: "scan",
    currentName: "",
    current: 0,
    total: entries.length
  })
  const made: MadeInTarget = {
    // Remember every new name so publication includes its parent-entry barrier.
    newDirectories: [],
    directories: [],
    files: [],
    notOurs: new Set(),
    marker: false
  }
  const marker = path.join(target, MIGRATION_MARKER)
  const copies: (CopiedFile | undefined)[] = []
  let warnings: string[]
  // Until the pointer names the new folder, the old one stays in use and
  // untouched. A failure before that removes what this move made in the new
  // folder, as 2.0.40 did, so its space comes back; anything that cannot be
  // removed keeps the marker, and validation reports the folder as an
  // unfinished copy of the old one.
  let markerRemoved = false
  try {
    const missing: string[] = []
    let anchor = target
    while (!(await statOrMissing(anchor))) {
      // Not even the root is there: the drive was disconnected after the check.
      if (path.dirname(anchor) === anchor)
        throw new Error(
          `The drive of the new folder is not connected: ${target}`
        )
      missing.push(anchor)
      anchor = path.dirname(anchor)
    }
    made.newDirectories.push(...missing)
    made.anchor = { folder: anchor, dev: (await fs.stat(anchor)).dev }
    await fs.mkdir(target, { recursive: true })
    await markUnfinished(target, source, made)
    const files: number[] = []
    for (const [index, entry] of entries.entries()) {
      const from = path.join(source, entry.relative),
        to = path.join(target, entry.relative)
      if (path.resolve(from) === path.resolve(pointer)) continue
      // Entries list each folder before what it holds.
      if (entry.directory) {
        await fs.mkdir(to)
        made.directories.push(to)
      } else files.push(index)
    }
    const buffers = Array.from(
      { length: COPY_WORKERS },
      (): [Buffer, Buffer] => [
        Buffer.allocUnsafe(CHUNK),
        Buffer.allocUnsafe(CHUNK)
      ]
    )
    // Flushed once the copy is complete where the filesystem allows it, file
    // by file otherwise.
    const flushAtEnd = await flushesWhole(target)
    let done = 0
    await eachLimited(files, COPY_WORKERS, async (index, worker) => {
      const { relative } = entries[index]
      const from = path.join(source, relative),
        to = path.join(target, relative)
      progress?.({
        action: "copy",
        currentName: relative,
        current: done,
        total: fileCount * 2
      })
      made.files.push(to)
      let copied
      try {
        copied = await copyVerified(from, to, relative, buffers[worker])
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code === "EEXIST") made.notOurs.add(to)
        throw e
      }
      // Durable before the pointer names this folder and the original is deleted.
      if (!flushAtEnd) await syncCopiedFile(to)
      copies[index] = { relative, ...copied }
      done++
    })
    // One flush writes every copied file and folder, those above the new
    // folder included, to the drive.
    let flushedWhole = false
    if (flushAtEnd) {
      progress?.({
        action: "flush",
        currentName: "",
        current: done,
        total: fileCount * 2
      })
      flushedWhole = await flushFilesystem(target)
      if (!flushedWhole)
        await eachLimited(made.files, COPY_WORKERS, syncCopiedFile)
    }
    if (!flushedWhole) {
      for (const entry of [...entries].reverse())
        if (entry.directory)
          await syncDirectory(path.join(target, entry.relative))
      await syncDirectory(target)
      // Bottom-up through the nearest pre-existing ancestor, before marker completion.
      for (const directory of made.newDirectories)
        await syncDirectory(path.dirname(directory))
    }
    await fs.unlink(marker)
    made.marker = false
    markerRemoved = true
    await syncDirectory(target)
    warnings = await publishRuntimeOverrideAtomically(pointer, target)
  } catch (e) {
    // The pointer is only ever replaced whole; should it name the new folder
    // all the same, that folder is in use and stays.
    if (markerRemoved && (await pointerNames(pointer, target)) !== false)
      throw e
    throw new UnfinishedCopyError(
      target,
      e,
      await removeUnfinishedCopy(made, target, source, progress)
    )
  }
  const copied = copies.filter((copy): copy is CopiedFile => !!copy)
  // The new location might not survive a power loss, and the launcher would then start
  // from the old folder again: keep all of it.
  if (warnings.length)
    return { warnings, changed: [], notDeleted: [], keptOldFolder: true }
  // A move, as in 2.0.40: delete each original whose verified copy is now in use. One
  // that no longer matches its copy stays where it is, as does one that cannot be deleted
  // (Windows refuses files another program has open; a folder may not allow it), and
  // both are named to the user, apart, since only the first can hold newer data.
  const changed: string[] = []
  const notDeleted: string[] = []
  for (const [index, entry] of copied.entries()) {
    const from = path.join(source, entry.relative)
    progress?.({
      action: "remove",
      currentName: entry.relative,
      current: fileCount + index,
      total: fileCount * 2
    })
    try {
      if (!(await unchangedSinceCopy(from, entry, canonicalSource))) {
        changed.push(entry.relative)
        warnings.push(
          `Kept in the old folder, changed after it was copied: ${from}`
        )
        continue
      }
    } catch (e) {
      // Already gone: nothing is left behind.
      if ((e as NodeJS.ErrnoException).code === "ENOENT") continue
      changed.push(entry.relative)
      warnings.push(
        `Kept in the old folder, could not be checked: ${from}: ${String(e)}`
      )
      continue
    }
    try {
      await fs.unlink(from)
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") continue
      notDeleted.push(entry.relative)
      warnings.push(
        `Kept in the old folder, could not be deleted: ${from}: ${String(e)}`
      )
    }
  }
  for (const entry of [...entries].reverse())
    if (entry.directory) {
      await fs.rmdir(path.join(source, entry.relative)).catch(() => {})
    }
  return { warnings, changed, notDeleted, keptOldFolder: false }
}
