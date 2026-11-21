/** The mod loaders GDLauncher runs an instance with. */
export const MOD_LOADERS: readonly string[] = [
  "forge",
  "fabric",
  "quilt",
  "neoforge"
]

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
