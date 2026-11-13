import type { DeletionTarget } from "./deletionTarget"

/** An addon about to be deleted, as the question about it needs it. */
export interface AddonToDelete {
  name: string
  /**
   * Whether GDLauncher can get the file again: it is known on CurseForge or
   * Modrinth. A world, a private build or an edited pack is not.
   */
  downloadable: boolean
}

/**
 * Whether deleting `addons` asks first. More than one always does: a
 * mis-click on a selection is many files. A single file asks only when it
 * would be gone for good: it cannot be downloaded again and does not go to
 * the Recycle Bin. While the setting is unknown it counts as off.
 */
export function asksBeforeDeletingAddons(
  addons: readonly AddonToDelete[],
  target: DeletionTarget
) {
  if (addons.length > 1) return true
  if (addons.length === 0) return false
  return target !== "recycleBin" && !addons[0].downloadable
}

/** How many names the question lists before "and N more". */
export const LISTED_ADDON_NAMES = 5

/** The names to list, and how many are left out. */
export function listedAddonNames(names: readonly string[]) {
  return {
    listed: names.slice(0, LISTED_ADDON_NAMES),
    more: Math.max(0, names.length - LISTED_ADDON_NAMES)
  }
}

/** The confirm button's label. */
export function addonDeleteLabelKey(count: number, target: DeletionTarget) {
  if (target === "permanent") return "instances:_trn_server_delete_permanently"
  return count > 1
    ? "instances:_trn_confirm_world_deletion_confirm_selection"
    : "instances:_trn_instance_confirm_deletion.delete"
}
