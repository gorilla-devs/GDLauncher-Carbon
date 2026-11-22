import type { FEWorld } from "@gd/core_module/bindings"

type Named = Pick<FEWorld, "name" | "title">

/** What the player calls a world: its in-game name, else (also for a name
 *  of only spaces) its folder name. */
export function worldLabel(world: Named): string {
  return world.title?.trim() ? world.title : world.name
}

export interface WorldName {
  label: string
  /** Set only when another world has the same label, so they can be told
   *  apart. */
  folder?: string
}

/** The label of each world, keyed by folder name. */
export function worldNames(worlds: readonly Named[]): Map<string, WorldName> {
  const counts = new Map<string, number>()
  for (const world of worlds) {
    const label = worldLabel(world)
    counts.set(label, (counts.get(label) ?? 0) + 1)
  }
  return new Map(
    worlds.map((world) => {
      const label = worldLabel(world)
      const shared = (counts.get(label) ?? 0) > 1
      return [world.name, shared ? { label, folder: world.name } : { label }]
    })
  )
}

/** The label for a name the core sends as a folder name. */
export function labelOfFolder(
  worlds: readonly Named[] | undefined,
  folder: string
): string {
  const world = worlds?.find((candidate) => candidate.name === folder)
  return world ? worldLabel(world) : folder
}
