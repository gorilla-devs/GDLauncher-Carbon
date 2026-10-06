import type { FEUnifiedSearchType } from "@gd/core_module/bindings"

/** The target scope a version list is filtered by. `undefined` on either
 *  field means "no filter", which is what an addon page opened outside any
 *  target legitimately resolves to — distinct from the scope not being
 *  known yet, which `TargetScope | undefined` carries instead. */
export interface TargetScope {
  modLoaderType: string | null | undefined
  gameVersion: string | null | undefined
}

/** How the selected instance or server scopes an addon's versions:
 *  - "loader": by its Minecraft version and mod loader (mods, plugins, and
 *    any type not known yet);
 *  - "gameOnly": by its Minecraft version only. Resource packs, shaders,
 *    worlds and datapacks carry no mod loader;
 *  - "none": not at all. A modpack is installed as a new instance or server,
 *    never into the selected one, so the selected one's version and loader
 *    say nothing about which pack versions fit. */
export type ScopeMode = "loader" | "gameOnly" | "none"

const LOADERLESS_TYPES: readonly FEUnifiedSearchType[] = [
  "resourcePack",
  "shader",
  "world",
  "datapack"
]

export function scopeMode(type: FEUnifiedSearchType | undefined): ScopeMode {
  if (type === "modpack") return "none"
  return type && LOADERLESS_TYPES.includes(type) ? "gameOnly" : "loader"
}

/** The mod loaders whose builds run on `loader` at `gameVersion`, the
 *  target's own first: Quilt also loads Fabric mods and NeoForge 1.20.1 also
 *  loads Forge mods. Mirrors the core's `accepted_loader_tags`, which both
 *  instance and server installs check a version against. */
export function compatibleLoaders(
  loader: string,
  gameVersion: string | null | undefined
): string[] {
  const own = loader.toLowerCase()
  if (own === "quilt") return [loader, "fabric"]
  if (own === "neoforge" && gameVersion === "1.20.1") return [loader, "forge"]
  return [loader]
}

/** What the selected instance or server runs, as its details report it.
 *  A missing value is not known: an instance whose modpack has not finished
 *  installing has no game version yet, a vanilla one has no loader. */
export interface TargetFacts {
  loader?: string | null
  gameVersion?: string | null
}

/** What the addon's project declares: the loaders and Minecraft versions its
 *  files are tagged with, as the platform summarises them on the project.
 *  A missing or empty list means "not known yet", never "supports nothing". */
export interface AddonSupport {
  loaders?: readonly string[]
  gameVersions?: readonly string[]
}

const known = (list: readonly string[] | undefined) =>
  list && list.length > 0 ? list : undefined

const sameList = (
  a: readonly string[] | undefined,
  b: readonly string[] | undefined
) => {
  const x = known(a),
    y = known(b)
  if (!x || !y) return x === y
  return x.length === y.length && x.every((value, i) => value === y[i])
}

/** Whether two support summaries would scope a list the same way. The
 *  project query hands out new arrays on every refetch. */
export const sameAddonSupport = (a: AddonSupport, b: AddonSupport) =>
  sameList(a.loaders, b.loaders) && sameList(a.gameVersions, b.gameVersions)

/**
 * The filters a target's version list opens with: the target's own Minecraft
 * version and loader, adjusted to what the addon declares.
 *
 * - When the addon declares no builds for the target's own loader but does
 *   for one the target also runs, that one is used: Fabric builds on a Quilt
 *   instance or server, Forge builds on NeoForge 1.20.1.
 * - On an instance, a loader or Minecraft version the addon declares nothing
 *   for is dropped, as 2.0.40 did, so the list shows the addon's other
 *   versions rather than none (an addon tagged "1.20" but not "1.20.1", a
 *   snapshot instance, a Fabric-only mod on Forge).
 * - A server keeps its own version and loader: its list only offers versions
 *   it can run. The empty list then says what it was filtered by and offers
 *   every version, as the Override checkbox does.
 *
 * A target value that is not known filters nothing, so an instance whose
 * game version is not known yet lists every game version instead of failing.
 */
export function targetDefaults(
  kind: "none" | "instance" | "server",
  mode: ScopeMode,
  facts: TargetFacts,
  support: AddonSupport
): TargetScope {
  if (kind === "none" || mode === "none") {
    return { modLoaderType: undefined, gameVersion: undefined }
  }
  const lenient = kind === "instance"
  const gameVersion = facts.gameVersion || undefined

  let modLoaderType: string | undefined
  const loader = mode === "loader" ? facts.loader || undefined : undefined
  if (loader) {
    const declared = known(support.loaders)?.map((value) => value.toLowerCase())
    const runnable = compatibleLoaders(loader, gameVersion)
    const pick = declared
      ? runnable.find((value) => declared.includes(value.toLowerCase()))
      : loader
    modLoaderType = pick ?? (lenient ? undefined : loader)
  }

  const versions = known(support.gameVersions)
  const versionDeclared =
    !gameVersion || !versions || versions.includes(gameVersion)

  return {
    modLoaderType,
    gameVersion: versionDeclared || !lenient ? gameVersion : undefined
  }
}

/** Whether two scopes filter a list the same way. Every "no filter" spelling
 *  is one value: this module resolves a missing filter to `undefined`,
 *  `versionsDefaultQuery` starts it at `null`, and `ExploreVersionsNavbar`
 *  clears it back to `null`. */
export const sameFilter = (
  a: string | null | undefined,
  b: string | null | undefined
) => (a ?? null) === (b ?? null)

export const sameScope = (a: TargetScope, b: TargetScope) =>
  sameFilter(a.modLoaderType, b.modLoaderType) &&
  sameFilter(a.gameVersion, b.gameVersion)
