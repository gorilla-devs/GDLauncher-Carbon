import type {
  FEServerDetails,
  FEUnifiedSearchResult,
  FEUnifiedSearchType,
  ListInstance
} from "@gd/core_module/bindings"

/**
 * The file a pasted or website install link pinned (for example the
 * curseforge://install?addonId=…&fileId=… link curseforge.com's Install
 * buttons send). Those buttons send CurseForge's current main file, which is
 * usually the newest Minecraft build, not one the user chose, so the pin is
 * only honoured where the file fits the target instance.
 */
export interface LinkedFile {
  /** The pinned file ID, as carried by `requestedVersionId`. */
  id: string
  displayName: string
  /** The file's name on disk (CurseForge `fileName`), when known. GDLauncher
   *  installs a server addon under this name, so a server copy named like it
   *  is this file. */
  fileName?: string
  /** CurseForge `gameVersions`: Minecraft releases plus loader tags. */
  gameVersions: string[]
}

/** A search row, plus the linked file's metadata when it came from a link. */
export type LinkedSearchResult = FEUnifiedSearchResult & {
  linkedFile?: LinkedFile
  /** The link named a file that could not be loaded (deleted, unavailable
   *  or a failed lookup). The row then has no pin and installs the version
   *  resolved for each target, as 2.0.40 did; the row says so. */
  linkedFileUnavailable?: boolean
}

/** The instance or server a linked file is checked against. */
export interface LinkTarget {
  mc_version: string | null | undefined
  modloader: string | null | undefined
}

// CurseForge lists loaders as game-version tags next to the releases.
const LOADER_NAMES = new Map([
  ["forge", "Forge"],
  ["neoforge", "NeoForge"],
  ["fabric", "Fabric"],
  ["quilt", "Quilt"],
  ["liteloader", "LiteLoader"],
  ["rift", "Rift"],
  ["cauldron", "Cauldron"]
])

const MC_VERSION = /^\d+\.\d+(?:\.\d+)?(?:-[\w.]+)?$/

const isLoaderTag = (tag: string) => LOADER_NAMES.has(tag.trim().toLowerCase())

export function toLinkedFile(
  id: string,
  file: { displayName?: string; fileName?: string; gameVersions?: string[] }
): LinkedFile {
  return {
    id,
    displayName: file.displayName || file.fileName || id,
    fileName: file.fileName || undefined,
    gameVersions: [...(file.gameVersions ?? [])]
  }
}

/**
 * Whether a linked file can go into the instance as-is. Mirrors what
 * `install_latest_curseforge_mod` filters on: the instance's exact release
 * must be listed, and for mods a listed loader must be the instance's
 * (Fabric files run on Quilt; files with no loader tag are legacy Forge
 * uploads and count as a match). An instance without a known release keeps
 * the pin, since there is nothing to resolve a compatible file against.
 */
export function linkedFileFitsInstance(
  gameVersions: readonly string[],
  addonType: FEUnifiedSearchType | undefined,
  target: LinkTarget
): boolean {
  if (!target.mc_version) return true
  if (!gameVersions.includes(target.mc_version)) return false
  if (addonType !== "mod") return true
  const loaders = gameVersions
    .filter(isLoaderTag)
    .map((tag) => tag.trim().toLowerCase())
  if (loaders.length === 0) return true
  const loader = target.modloader?.toLowerCase()
  if (!loader) return false
  return (
    loaders.includes(loader) ||
    (loader === "quilt" && loaders.includes("fabric"))
  )
}

/**
 * The file to install for a target: the pinned one when there is no linked
 * file to check or it fits, otherwise undefined, which installs the version
 * the backend resolves for that instance (2.0.40's path for link rows).
 * A link's file is not trusted where it cannot be checked: an instance that
 * cannot be read, or metadata that describes a different file than the pin.
 */
export function effectiveFileId<T extends string | number>(
  fileId: T | undefined,
  linkedFile: LinkedFile | undefined,
  addonType: FEUnifiedSearchType | undefined,
  target: LinkTarget | undefined
): T | undefined {
  if (fileId === undefined || fileId === "" || !linkedFile) return fileId
  if (!target || String(fileId) !== linkedFile.id) return undefined
  return linkedFileFitsInstance(linkedFile.gameVersions, addonType, target)
    ? fileId
    : undefined
}

/** Release and loader of a server, or undefined while its details load. */
export function serverLinkTarget(
  server:
    | Pick<FEServerDetails, "gameVersion" | "modloaderType">
    | null
    | undefined
): LinkTarget | undefined {
  if (!server) return undefined
  return {
    mc_version: server.gameVersion || null,
    modloader: server.modloaderType?.toLowerCase() ?? null
  }
}

/** Release and loader of a library instance, or undefined when unknown. */
export function instanceLinkTarget(
  instances: readonly ListInstance[] | undefined,
  instanceId: number | undefined
): LinkTarget | undefined {
  if (instanceId === undefined) return undefined
  const status = instances?.find((i) => i.id === instanceId)?.status
  if (status?.status !== "valid") return undefined
  return {
    mc_version: status.value.mc_version,
    modloader: status.value.modloader
  }
}

const compareVersions = (a: string, b: string) => {
  const pa = a.split(/[.-]/).map((p) => parseInt(p, 10))
  const pb = b.split(/[.-]/).map((p) => parseInt(p, 10))
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = Number.isNaN(pa[i]) ? -1 : (pa[i] ?? -1)
    const y = Number.isNaN(pb[i]) ? -1 : (pb[i] ?? -1)
    if (x !== y) return x - y
  }
  return a.localeCompare(b)
}

/** The Minecraft releases a linked file lists, oldest first. */
function linkedFileReleases(gameVersions: readonly string[]): string[] {
  return [
    ...new Set(
      gameVersions.map((v) => v.trim()).filter((v) => MC_VERSION.test(v))
    )
  ].sort(compareVersions)
}

function linkedFileLoaders(gameVersions: readonly string[]): string {
  const names = gameVersions
    .map((tag) => LOADER_NAMES.get(tag.trim().toLowerCase()))
    .filter((name): name is string => !!name)
  return [...new Set(names)].join("/")
}

/**
 * Short "26.3 · NeoForge" style summary of what a linked file targets, or ""
 * when it lists no release or loader. More than three releases show the
 * newest two and how many more there are ("1.20.6, 1.21.1 +2 · Forge"): a
 * range would seem to include the releases a list with gaps leaves out.
 */
export function describeLinkedFile(gameVersions: readonly string[]): string {
  const releases = linkedFileReleases(gameVersions)
  const shown =
    releases.length > 3
      ? `${releases.slice(-2).join(", ")} +${releases.length - 2}`
      : releases.join(", ")
  return [shown, linkedFileLoaders(gameVersions)].filter(Boolean).join(" · ")
}

/** Every release and loader a linked file lists, for a tooltip. */
export function describeLinkedFileInFull(
  gameVersions: readonly string[]
): string {
  return [
    linkedFileReleases(gameVersions).join(", "),
    linkedFileLoaders(gameVersions)
  ]
    .filter(Boolean)
    .join(" · ")
}
