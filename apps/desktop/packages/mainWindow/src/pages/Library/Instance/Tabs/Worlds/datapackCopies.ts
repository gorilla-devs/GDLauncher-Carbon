import type { FEWorld, Mod } from "@gd/core_module/bindings"

/** What identifies a data pack's project: a world pack or an addon row. */
export interface DatapackProject {
  curseforge: { project_id: number } | null
  modrinth: { project_id: string } | null
}

/** Whether two data packs are files of one Modrinth or CurseForge project. */
export const sameDatapackProject = (a: DatapackProject, b: DatapackProject) =>
  (!!a.modrinth &&
    !!b.modrinth &&
    a.modrinth.project_id === b.modrinth.project_id) ||
  (!!a.curseforge &&
    !!b.curseforge &&
    a.curseforge.project_id === b.curseforge.project_id)

/**
 * The copies of `pack`'s project in the instance that have an update: data
 * packs in every world and the instance's own data packs, matched by Modrinth
 * or CurseForge project, as the core's "update everywhere" matches them. The
 * pack itself is counted when it has an update. Folders are never updated.
 */
export function datapackCopies(
  worlds: readonly FEWorld[],
  instanceMods: readonly Mod[],
  pack: DatapackProject
): { withUpdate: number } {
  let withUpdate = 0
  for (const world of worlds)
    for (const copy of world.datapacks)
      if (!copy.is_folder && copy.has_update && sameDatapackProject(copy, pack))
        withUpdate++
  for (const row of instanceMods)
    if (
      row.addon_type === "datapacks" &&
      row.has_update &&
      sameDatapackProject(row, pack)
    )
      withUpdate++
  return { withUpdate }
}
