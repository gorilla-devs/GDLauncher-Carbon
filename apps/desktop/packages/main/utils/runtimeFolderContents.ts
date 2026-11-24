import fs from "node:fs/promises"
import path from "node:path"

/**
 * Names are compared as the filesystem does: Windows and macOS ignore case,
 * Linux does not.
 */
const fold = (name: string) =>
  process.platform === "linux" ? name : name.toLowerCase()

// What the system or a file manager puts in a folder without being asked.
const SYSTEM_LEFTOVERS = new Set(
  [
    ".DS_Store",
    ".Spotlight-V100",
    ".fseventsd",
    ".Trashes",
    ".TemporaryItems",
    "desktop.ini",
    "Thumbs.db",
    "lost+found",
    "$RECYCLE.BIN",
    "System Volume Information"
  ].map(fold)
)

/**
 * Whether `name`, at the top of a folder, is something the system put there:
 * Finder's and Explorer's own files, a drive's trash and index folders. A
 * folder holding only these counts as empty, and a move never takes them.
 */
export function isSystemLeftover(name: string): boolean {
  return (
    SYSTEM_LEFTOVERS.has(fold(name)) ||
    // macOS keeps a file's extended attributes in "._<name>" on FAT and exFAT.
    name.startsWith("._") ||
    // The trash folder a Linux desktop keeps at the top of a drive.
    /^\.Trash-\d+$/.test(name)
  )
}

// The folders the core keeps at the top of the data folder (carbon_rt_path).
const LAUNCHER_FOLDERS = new Set(
  [
    "instances",
    "servers",
    "managed_javas",
    "libraries",
    "assets",
    "natives",
    "logging_configs",
    "temp",
    "world_maps",
    "themes",
    "download"
  ].map(fold)
)

/**
 * Whether `name`, at the top of the data folder, is the launcher's: one of
 * the core's folders, the database with its journal, backups and recovery
 * copies (gdl_conf.*), or its logs, markers and records (__gdl_*, .gdl_*).
 */
export function isLauncherEntry(name: string): boolean {
  const folded = fold(name)
  return (
    LAUNCHER_FOLDERS.has(folded) ||
    folded.startsWith("gdl_conf.") ||
    folded.startsWith("__gdl_") ||
    folded.startsWith(".gdl_")
  )
}

/** What a folder that holds launcher data holds, as the switch dialog names it. */
export interface RuntimeDataFound {
  // Folders in `instances` and in `servers`; the core lists a broken one too.
  instances: number
  servers: number
  // The database holds the settings and the accounts.
  database: boolean
}

async function isA(file: string, kind: "file" | "folder"): Promise<boolean> {
  const st = await fs.stat(file).catch(() => null)
  return !!st && (kind === "file" ? st.isFile() : st.isDirectory())
}

async function folderCount(folder: string): Promise<number> {
  const entries = await fs
    .readdir(folder, { withFileTypes: true })
    .catch(() => [])
  return entries.filter((entry) => entry.isDirectory()).length
}

/**
 * What `folder` holds of a launcher's data, or null when nothing in it says
 * it was ever a data folder. Any one of these is enough: the database, an
 * instance, a server, the launcher's Java folder or its log folder. A data
 * folder that lost its database still counts, since the core puts instances
 * and servers back from the files beside them. The folders every launcher
 * names alike (temp, assets, libraries, download) say nothing on their own.
 */
export async function launcherDataIn(
  folder: string
): Promise<RuntimeDataFound | null> {
  const [database, instances, servers, javas, logs] = await Promise.all([
    isA(path.join(folder, "gdl_conf.db"), "file"),
    folderCount(path.join(folder, "instances")),
    folderCount(path.join(folder, "servers")),
    isA(path.join(folder, "managed_javas"), "folder"),
    isA(path.join(folder, "__gdl_logs__"), "folder")
  ])
  return database || instances || servers || javas || logs
    ? { instances, servers, database }
    : null
}

/**
 * The names at the top of the data folder `folder` that are neither the
 * launcher's nor the system's, sorted: what someone else keeps there.
 */
export async function unfamiliarRuntimeEntries(
  folder: string
): Promise<string[]> {
  const names = await fs.readdir(folder)
  return names
    .filter((name) => !isLauncherEntry(name) && !isSystemLeftover(name))
    .sort((a, b) => a.localeCompare(b))
}
