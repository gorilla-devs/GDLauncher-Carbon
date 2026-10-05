import type { FEUnifiedCategories } from "@gd/core_module/bindings"
/** A failed refetch retains the last healthy dictionary, with its new error. */
export function mergeUnifiedCategories(
  previous: FEUnifiedCategories | undefined,
  incoming: FEUnifiedCategories
): FEUnifiedCategories {
  return {
    ...incoming,
    curseforge:
      incoming.curseforgeError && previous
        ? previous.curseforge
        : incoming.curseforge,
    modrinth:
      incoming.modrinthError && previous ? previous.modrinth : incoming.modrinth
  }
}
