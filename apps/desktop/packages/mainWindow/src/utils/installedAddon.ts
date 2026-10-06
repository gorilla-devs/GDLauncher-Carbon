import type {
  AddonType,
  FEUnifiedSearchResult,
  Mod,
  ServerAddon
} from "@gd/core_module/bindings"
import type { LinkedFile } from "./linkedFile"

export interface InstalledArtifact {
  id: string
  version: { kind: "known"; id: string | number } | { kind: "unknown" }
}

export type InstalledAddonTarget =
  | { kind: "instance"; addons: readonly Mod[] }
  | { kind: "server"; addons: readonly ServerAddon[] }

const addonKinds: Partial<Record<FEUnifiedSearchResult["type"], AddonType>> = {
  mod: "mods",
  resourcePack: "resourcepacks",
  shader: "shaders",
  datapack: "datapacks",
  world: "worlds"
}

export function isInstalledArtifact(
  artifact: InstalledArtifact | undefined,
  requested?: string | number | null
): boolean {
  if (!artifact) return false
  if (requested == null || requested === "") return true
  if (artifact.version.kind === "unknown") return false
  return typeof artifact.version.id === "number"
    ? artifact.version.id === parseInt(requested.toString(), 10)
    : artifact.version.id === requested.toString()
}

/** A server addon's file name without its `.disabled` suffix. */
const serverFileName = (addon: ServerAddon) =>
  addon.filename.replace(/\.disabled$/, "")

/** Match actual provider/kind ownership. Server project metadata cannot prove
 *  a version, except for a copy named like a link's file: GDLauncher installs
 *  server addons under the platform's file name, so that copy is the file. */
export function findInstalledAddon(
  addon: FEUnifiedSearchResult,
  target: InstalledAddonTarget,
  requested?: string | number | null,
  linkedFile?: Pick<LinkedFile, "id" | "fileName">
): InstalledArtifact | undefined {
  const kind = addonKinds[addon.type]
  if (!kind) return undefined
  const projectId = addon.id.toString()
  if (target.kind === "instance") {
    const matches = target.addons
      .filter(
        (mod) =>
          mod.addon_type === kind &&
          (addon.platform === "curseforge"
            ? mod.curseforge?.project_id === parseInt(projectId, 10)
            : mod.modrinth?.project_id === projectId)
      )
      .map((mod) => {
        const versionId =
          addon.platform === "curseforge"
            ? mod.curseforge?.file_id
            : mod.modrinth?.version_id
        return {
          id: mod.id,
          version:
            versionId == null || versionId === ""
              ? { kind: "unknown" as const }
              : { kind: "known" as const, id: versionId }
        }
      })
    // Separate saves/duplicates can expose more than one artifact of a project.
    return (
      matches.find((artifact) => isInstalledArtifact(artifact, requested)) ??
      matches[0]
    )
  }
  const candidates = target.addons.filter((cached) => cached.addonType === kind)
  const ofProject = candidates.filter((cached) =>
    addon.platform === "curseforge"
      ? cached.curseforgeProjectId === parseInt(projectId, 10)
      : cached.modrinthProjectId === projectId
  )
  const linkedName = linkedFile?.fileName
  const sameFile = linkedName
    ? ofProject.find((cached) => serverFileName(cached) === linkedName)
    : undefined
  if (sameFile && linkedFile)
    return {
      id: sameFile.id,
      version: { kind: "known", id: linkedFile.id }
    }
  const byProject = ofProject[0]
  const slug = addon.slug?.toLowerCase()
  const found =
    byProject ??
    (slug
      ? candidates.find((cached) => {
          // A slug alias may identify an unclassified addon, never override known provider IDs.
          if (
            cached.curseforgeProjectId != null ||
            cached.modrinthProjectId != null
          )
            return false
          const name = cached.displayName.toLowerCase()
          return name === slug || name.startsWith(slug + "-")
        })
      : undefined)
  return found ? { id: found.id, version: { kind: "unknown" } } : undefined
}

/** `requested` is the file the row would install: its pin, or nothing for a
 *  link row whose linked file does not fit the instance (utils/linkedFile.ts).
 *  `linkedFile` lets a server copy of that very file count as installed. */
export function isAddonInstalled(
  addon: FEUnifiedSearchResult,
  target: InstalledAddonTarget,
  requested: string | null | undefined,
  linkedFile?: Pick<LinkedFile, "id" | "fileName">
): boolean {
  return isInstalledArtifact(
    findInstalledAddon(addon, target, requested, linkedFile),
    requested
  )
}
