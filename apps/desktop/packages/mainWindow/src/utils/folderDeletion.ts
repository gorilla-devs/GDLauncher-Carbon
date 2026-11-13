/** What removing a library folder does to what is in it. */
export type FolderDeletionMode = "unlink" | "deleteAll"

/**
 * What the dialog shows: one action that keeps the members, one that deletes
 * them, or both when nothing says which was meant.
 */
export type FolderDeletionDialog = "remove" | "deleteAll" | "choice"

/**
 * Which dialog a folder removal gets. Only instances can be deleted with
 * their folder, and only when there are any. A menu entry names its action
 * for one folder; a selection of folders does not, so it asks.
 */
export function folderDeletionDialog(
  kind: "instance" | "server" | undefined,
  mode: FolderDeletionMode | undefined,
  folderCount: number,
  memberCount: number
): FolderDeletionDialog {
  if (kind !== "instance" || memberCount === 0) return "remove"
  if (folderCount !== 1 || !mode) return "choice"
  return mode === "unlink" ? "remove" : "deleteAll"
}

/** The label of the action that keeps the members; also the dialog title. */
export function folderRemoveLabelKey(folderCount: number) {
  return folderCount > 1
    ? "instances:_trn_folders_remove_title"
    : "instances:_trn_folder_remove_title"
}

/** The label of the choice that keeps the instances. */
export function folderKeepLabelKey(folderCount: number) {
  return folderCount > 1
    ? "instances:_trn_folders_choice_keep"
    : "instances:_trn_folder_choice_keep"
}

/** The label of the action that deletes the instances, with their count. */
export function folderDeleteAllLabelKey(
  folderCount: number,
  instanceCount: number
) {
  if (folderCount > 1)
    return instanceCount === 1
      ? "instances:_trn_folders_delete_all_button_single"
      : "instances:_trn_folders_delete_all_button"
  return instanceCount === 1
    ? "instances:_trn_folder_delete_all_button_single"
    : "instances:_trn_folder_delete_all_button"
}
