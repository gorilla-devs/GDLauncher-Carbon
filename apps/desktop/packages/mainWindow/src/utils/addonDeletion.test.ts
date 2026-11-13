import { describe, expect, it } from "vitest"
import {
  addonDeleteLabelKey,
  asksBeforeDeletingAddons,
  LISTED_ADDON_NAMES,
  listedAddonNames
} from "./addonDeletion"

const fromPlatform = { name: "sodium.jar", downloadable: true }
const ownFile = { name: "my-tweaks.jar", downloadable: false }

describe("when deleting addons asks first", () => {
  it("never asks for one file that can be downloaded again", () => {
    for (const target of ["recycleBin", "permanent", undefined] as const) {
      expect(asksBeforeDeletingAddons([fromPlatform], target)).toBe(false)
    }
  })
  it("asks for one file of the user's own only when it would be gone for good", () => {
    expect(asksBeforeDeletingAddons([ownFile], "recycleBin")).toBe(false)
    expect(asksBeforeDeletingAddons([ownFile], "permanent")).toBe(true)
    // Unknown counts as off.
    expect(asksBeforeDeletingAddons([ownFile], undefined)).toBe(true)
  })
  it("always asks for more than one, and never for none", () => {
    for (const target of ["recycleBin", "permanent", undefined] as const) {
      expect(
        asksBeforeDeletingAddons([fromPlatform, fromPlatform], target)
      ).toBe(true)
      expect(asksBeforeDeletingAddons([], target)).toBe(false)
    }
  })
})

describe("what the question shows", () => {
  it("lists a few names and counts the rest", () => {
    expect(listedAddonNames(["a", "b"])).toEqual({
      listed: ["a", "b"],
      more: 0
    })
    const many = Array.from(
      { length: LISTED_ADDON_NAMES + 3 },
      (_, i) => `${i}`
    )
    expect(listedAddonNames(many)).toEqual({
      listed: many.slice(0, LISTED_ADDON_NAMES),
      more: 3
    })
  })
  it("labels the button with what the click does", () => {
    expect(addonDeleteLabelKey(1, "recycleBin")).toBe(
      "instances:_trn_instance_confirm_deletion.delete"
    )
    expect(addonDeleteLabelKey(1, undefined)).toBe(
      "instances:_trn_instance_confirm_deletion.delete"
    )
    expect(addonDeleteLabelKey(37, "recycleBin")).toBe(
      "instances:_trn_confirm_world_deletion_confirm_selection"
    )
    expect(addonDeleteLabelKey(1, "permanent")).toBe(
      "instances:_trn_server_delete_permanently"
    )
    expect(addonDeleteLabelKey(37, "permanent")).toBe(
      "instances:_trn_server_delete_permanently"
    )
  })
})
