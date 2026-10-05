import type {
  AddonType,
  FEUnifiedSearchResult,
  Mod,
  ServerAddon
} from "@gd/core_module/bindings"

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

/** Match actual provider/kind ownership. Server project metadata cannot prove a version. */
export function findInstalledAddon(
  addon: FEUnifiedSearchResult,
  target: InstalledAddonTarget,
  requested?: string | number | null
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
  const byProject = candidates.find((cached) =>
    addon.platform === "curseforge"
      ? cached.curseforgeProjectId === parseInt(projectId, 10)
      : cached.modrinthProjectId === projectId
  )
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

export function isAddonInstalled(
  addon: FEUnifiedSearchResult,
  target: InstalledAddonTarget
): boolean {
  return isInstalledArtifact(
    findInstalledAddon(addon, target, addon.requestedVersionId),
    addon.requestedVersionId
  )
}
