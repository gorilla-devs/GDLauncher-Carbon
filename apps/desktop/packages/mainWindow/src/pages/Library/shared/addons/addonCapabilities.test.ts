import { describe, expect, it } from "vitest"
import {
  requiresDeletionConfirmation,
  supportsEnableToggle
} from "./addonCapabilities"

describe("supportsEnableToggle", () => {
  it("offers the toggle for every file-backed addon type", () => {
    for (const t of ["mods", "resourcepacks", "shaders", "datapacks"]) {
      expect(supportsEnableToggle(t)).toBe(true)
    }
  })

  it("withholds the toggle for worlds", () => {
    expect(supportsEnableToggle("worlds")).toBe(false)
  })

  // An unknown type must keep the toggle: the backend already degrades an
  // unrecognised addon_type to `mods`, so hiding the control here would
  // compound one silent fallback with another.
  it("keeps the toggle for an unrecognised type", () => {
    expect(supportsEnableToggle("something-new")).toBe(true)
  })
})

describe("requiresDeletionConfirmation", () => {
  it("confirms only for worlds", () => {
    expect(requiresDeletionConfirmation("worlds")).toBe(true)
    for (const t of ["mods", "resourcepacks", "shaders", "datapacks"]) {
      expect(requiresDeletionConfirmation(t)).toBe(false)
    }
  })
})
