import { describe, expect, it } from "vitest"
import {
  folderDeleteAllLabelKey,
  folderDeletionDialog,
  folderKeepLabelKey,
  folderRemoveLabelKey
} from "./folderDeletion"

describe("which dialog a folder removal gets", () => {
  it("shows only the action its menu entry named for one instance folder", () => {
    expect(folderDeletionDialog("instance", "unlink", 1, 3)).toBe("remove")
    expect(folderDeletionDialog("instance", "deleteAll", 1, 3)).toBe(
      "deleteAll"
    )
  })
  it("asks what to do with the instances when nothing named an action", () => {
    expect(folderDeletionDialog("instance", undefined, 1, 3)).toBe("choice")
    expect(folderDeletionDialog("instance", undefined, 2, 8)).toBe("choice")
    // A menu entry speaks for one folder, never for a selection.
    expect(folderDeletionDialog("instance", "deleteAll", 2, 8)).toBe("choice")
    expect(folderDeletionDialog("instance", "unlink", 2, 8)).toBe("choice")
  })
  it("never offers to delete what is in an empty folder", () => {
    for (const mode of ["unlink", "deleteAll", undefined] as const) {
      expect(folderDeletionDialog("instance", mode, 1, 0)).toBe("remove")
      expect(folderDeletionDialog("instance", mode, 3, 0)).toBe("remove")
    }
  })
  it("never offers to delete servers with their folder", () => {
    for (const mode of ["unlink", "deleteAll", undefined] as const) {
      expect(folderDeletionDialog("server", mode, 1, 2)).toBe("remove")
      expect(folderDeletionDialog("server", mode, 3, 5)).toBe("remove")
      expect(folderDeletionDialog(undefined, mode, 1, 2)).toBe("remove")
    }
  })
})

describe("the folder removal labels", () => {
  it("count the folders", () => {
    expect(folderRemoveLabelKey(1)).toBe("instances:_trn_folder_remove_title")
    expect(folderRemoveLabelKey(2)).toBe("instances:_trn_folders_remove_title")
    expect(folderKeepLabelKey(1)).toBe("instances:_trn_folder_choice_keep")
    expect(folderKeepLabelKey(2)).toBe("instances:_trn_folders_choice_keep")
  })
  it("say one instance without a plural", () => {
    expect(folderDeleteAllLabelKey(1, 1)).toBe(
      "instances:_trn_folder_delete_all_button_single"
    )
    expect(folderDeleteAllLabelKey(1, 3)).toBe(
      "instances:_trn_folder_delete_all_button"
    )
    expect(folderDeleteAllLabelKey(2, 1)).toBe(
      "instances:_trn_folders_delete_all_button_single"
    )
    expect(folderDeleteAllLabelKey(2, 8)).toBe(
      "instances:_trn_folders_delete_all_button"
    )
  })
})
