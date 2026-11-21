import type {
  FEUnifiedSearchResult,
  FEWorld,
  InstallWorldDatapack,
  WorldDatapackSource
} from "@gd/core_module/bindings"
import { findInstalledAddon } from "./installedAddon"

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

/** False for a project that also ships as a mod, like Terralith. */
export function isOnlyDatapack(project: ProjectKind): boolean {
  return (
    project?.type === "datapack" ||
    (project?.loaders?.length === 1 && project.loaders[0] === "datapack")
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
