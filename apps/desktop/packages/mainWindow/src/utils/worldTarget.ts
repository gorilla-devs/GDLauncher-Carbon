import type {
  FEUnifiedSearchResult,
  FEWorld,
  InstallWorldDatapack,
  WorldDatapackSource
} from "@gd/core_module/bindings"
import { findInstalledAddon } from "./installedAddon"

/** A world the core could read, so a data pack can be installed into it. */
export type ReadableWorld = FEWorld & { identity: string }

/** The world an install goes to. The `world` search param counts only beside
 *  an instance and only for a data pack, so it never reaches another install. */
export function worldTargetName(
  instanceId: number | null | undefined,
  addonType: string | null | undefined,
  world: string | null | undefined
): string | undefined {
  return instanceId && addonType === "datapack" && world ? world : undefined
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
