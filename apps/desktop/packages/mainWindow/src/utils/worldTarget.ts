import type {
  FEUnifiedSearchResult,
  FEWorld,
  InstallWorldDatapack,
  MRFEVersion,
  WorldDatapackSource
} from "@gd/core_module/bindings"
import { findInstalledAddon, type InstanceInstall } from "./installedAddon"
import type { LinkTarget } from "./linkedFile"
import { compatibleLoaders, MOD_LOADERS } from "./modLoaders"

/** A world the core could read, so a data pack can be installed into it. */
export type ReadableWorld = FEWorld & { identity: string }

/** What tells a data pack project apart: CurseForge types it "datapack",
 *  Modrinth types it "mod" and lists "datapack" among its loaders. */
type ProjectKind =
  | { type?: string | null; loaders?: readonly string[] | null }
  | null
  | undefined

export function canBeDatapack(project: ProjectKind): boolean {
  return (
    project?.type === "datapack" || !!project?.loaders?.includes("datapack")
  )
}

const lowerCased = (loaders: readonly string[]) =>
  loaders.map((loader) => loader.toLowerCase())

/** A project typed as a resource pack or a shader goes into an instance as
 *  that, whatever its versions are tagged (the core's `addon_type`): there
 *  its data pack build is never what an install is. */
const installsAsTyped = (project: ProjectKind) =>
  project?.type === "resourcePack" || project?.type === "shader"

/** False for a project that also ships as a mod, like Terralith: one that
 *  lists a mod loader. Any other loader beside "datapack" (a resource pack's
 *  "minecraft", a plugin's "paper") is nothing an instance runs as a mod. */
export function isOnlyDatapack(project: ProjectKind): boolean {
  return (
    project?.type === "datapack" ||
    (canBeDatapack(project) &&
      !installsAsTyped(project) &&
      !lowerCased(project?.loaders ?? []).some((loader) =>
        MOD_LOADERS.includes(loader)
      ))
  )
}

/** Whether an install with no target chosen asks which world it goes to. A
 *  project that also ships as a mod asks only while data packs are what is
 *  searched (`searchedType`), so a mod search installs its mod in one click. */
export function asksWhichWorld(
  project: ProjectKind,
  searchedType: string | null | undefined
): boolean {
  return (
    isOnlyDatapack(project) ||
    (searchedType === "datapack" && canBeDatapack(project))
  )
}

/** Whether an install into the instance itself is the project's data pack
 *  build, which goes to its `datapacks` folder. A project that also ships as
 *  a mod installs its mod jar where the instance runs one of the loaders it
 *  lists, as the core picks that jar, and until the instance is known. */
export function instanceInstallIsDatapack(
  project: ProjectKind,
  instance: LinkTarget | undefined
): boolean {
  if (isOnlyDatapack(project)) return true
  if (!canBeDatapack(project) || installsAsTyped(project) || !instance)
    return false
  if (!instance.modloader) return true
  const listed = lowerCased(project?.loaders ?? [])
  return !lowerCased(
    compatibleLoaders(instance.modloader, instance.mc_version)
  ).some((loader) => listed.includes(loader))
}

/** A version picked by hand, not left to the instance. Whether it is a data
 *  pack build is its own matter, not the project's; `undefined` when that
 *  could not be told. */
export interface PickedVersion {
  datapackBuild: boolean | undefined
}

/** What a click that installs into the instance itself puts there: `picked`
 *  as what it is, else what the project installs as on that instance. */
export function instanceInstall(
  project: ProjectKind,
  instance: LinkTarget | undefined,
  picked?: PickedVersion
): InstanceInstall {
  if (!canBeDatapack(project) || installsAsTyped(project)) return "addon"
  // Every file of a project typed as a data pack (CurseForge) is one.
  if (project?.type === "datapack") return "datapack"
  if (picked)
    return picked.datapackBuild === undefined
      ? "unknown"
      : picked.datapackBuild
        ? "datapack"
        : "addon"
  return instanceInstallIsDatapack(project, instance) ? "datapack" : "addon"
}

interface VersionFiles {
  files: readonly Pick<MRFEVersion["files"][number], "filename" | "primary">[]
}

/** Whether the file of a Modrinth version an install puts in place, its
 *  primary file or its first when none is primary, is a zip. */
const installsZip = (version: VersionFiles) => {
  const file = version.files.find((file) => file.primary) ?? version.files[0]
  return !!file?.filename.toLowerCase().endsWith(".zip")
}

/** Whether a Modrinth version is a data pack build rather than a mod jar:
 *  tagged "datapack", with a zip to install. The core tells the two apart
 *  the same way (`modrinth_datapack_file`). */
export function isDatapackBuild(
  version: VersionFiles & Pick<MRFEVersion, "loaders">
): boolean {
  return version.loaders.includes("datapack") && installsZip(version)
}

const STABILITY = ["release", "beta", "alpha"] as const

/** The build an instance gets among the data pack builds Modrinth lists for
 *  its game version: the newest release whose primary file (or first, when
 *  none is primary) is a zip, else the newest beta, else the newest alpha.
 *  The core picks the same one for a world. */
export function newestDatapackBuild<
  Version extends Pick<MRFEVersion, "version_type" | "date_published"> &
    VersionFiles
>(versions: readonly Version[]): Version | undefined {
  const builds = versions
    .filter(installsZip)
    .sort((a, b) => b.date_published.localeCompare(a.date_published))
  for (const stability of STABILITY) {
    const build = builds.find((version) => version.version_type === stability)
    if (build) return build
  }
  return undefined
}

/** The world an install goes to. The `world` search param counts only beside
 *  an instance and only for a project that can be a data pack, so it never
 *  reaches another install. */
export function worldTargetName(
  instanceId: number | null | undefined,
  project: ProjectKind,
  world: string | null | undefined
): string | undefined {
  return instanceId && canBeDatapack(project) && world ? world : undefined
}

export function readableWorld(
  world: FEWorld | undefined
): ReadableWorld | undefined {
  return world?.identity != null ? (world as ReadableWorld) : undefined
}

// The core invalidates a world list when the packs in its worlds change, but
// not when the game creates a world, so a read made only to offer the worlds
// is reused just long enough to cover closing and reopening a menu.
export const WORLDS_STALE_TIME = 30 * 1000

/** What the "where" step of a data pack install offers for one instance. */
export interface InstallPlaces {
  /** The worlds a data pack can go into, and whether each holds the project. */
  worlds: readonly { world: ReadableWorld; holds: boolean }[]
  instanceHolds: boolean
  /** "choice": there is something to choose; "instance": only the instance
   *  can take it, so nothing is asked; "nothing": every place holds it. */
  offers: "choice" | "instance" | "nothing"
}

export function installPlaces(
  addon: FEUnifiedSearchResult | undefined,
  worlds: readonly FEWorld[] | undefined,
  instanceHolds: boolean
): InstallPlaces {
  const listed = (worlds ?? [])
    .filter((world): world is ReadableWorld => world.identity !== null)
    .map((world) => ({
      world,
      holds:
        !!addon &&
        !!findInstalledAddon(addon, {
          kind: "world",
          datapacks: world.datapacks
        })
    }))
  const offers = listed.some((entry) => !entry.holds)
    ? "choice"
    : instanceHolds
      ? "nothing"
      : listed.length > 0
        ? "choice"
        : "instance"
  return { worlds: listed, instanceHolds, offers }
}

/** The "where" step once the instance's worlds are read; undefined until
 *  then. Worlds that could not be read leave the instance to install into. */
export function readInstallPlaces(
  addon: FEUnifiedSearchResult | undefined,
  worlds: { data: readonly FEWorld[] | undefined; isError?: boolean },
  instanceHolds: boolean
): InstallPlaces | undefined {
  return worlds.isError || worlds.data !== undefined
    ? installPlaces(addon, worlds.data, instanceHolds)
    : undefined
}

/** Whether a Download for an instance shows as installed. Where it asks
 *  where (`asks`), a data pack the instance holds stays installed until a
 *  world is known that could still take it. */
export function installedInInstance(
  instanceHolds: boolean,
  asks: boolean,
  places: InstallPlaces | undefined
): boolean {
  return instanceHolds && (!asks || !places || places.offers === "nothing")
}

/** `null` asks the core for the newest data pack build for the game version. */
export function worldDatapackSource(
  addon: Pick<FEUnifiedSearchResult, "id" | "platform">,
  fileId?: number | string | null
): WorldDatapackSource {
  const chosen = fileId == null || fileId === "" ? null : fileId.toString()
  return addon.platform === "curseforge"
    ? {
        Curseforge: {
          project_id: parseInt(addon.id.toString(), 10),
          file_id: chosen === null ? null : parseInt(chosen, 10)
        }
      }
    : {
        Modrinth: {
          project_id: addon.id.toString(),
          version_id: chosen
        }
      }
}

/** The core refuses a file the world already holds unless it is told which
 *  copy the new one replaces: the world's own copy of the project. */
export function worldDatapackInstall(
  addon: FEUnifiedSearchResult,
  fileId: number | string | null | undefined,
  instanceId: number,
  world: ReadableWorld
): InstallWorldDatapack {
  return {
    instance_id: instanceId,
    world: world.name,
    identity: world.identity,
    source: worldDatapackSource(addon, fileId),
    replaces:
      findInstalledAddon(
        addon,
        { kind: "world", datapacks: world.datapacks },
        fileId
      )?.id ?? null
  }
}

/** A location search string without its `world` param. */
export function withoutWorldParam(search: string): string {
  const params = new URLSearchParams(search)
  if (!params.has("world")) return search
  params.delete("world")
  const rest = params.toString()
  return rest ? `?${rest}` : ""
}
