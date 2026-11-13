/**
 * Where something the user deletes goes: `undefined` until the "Deletion
 * through Recycle Bin" setting is known.
 */
export type DeletionTarget = "recycleBin" | "permanent" | undefined

export function deletionTarget(
  deletionThroughRecycleBin: boolean | undefined
): DeletionTarget {
  if (deletionThroughRecycleBin === undefined) return undefined
  return deletionThroughRecycleBin ? "recycleBin" : "permanent"
}
