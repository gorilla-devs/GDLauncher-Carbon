import { describe, expect, it } from "vitest"
import type {
  CleanupSelection,
  FEUntrackedFile
} from "@gd/core_module/bindings"
import { freezeSelection, reconcileSelections } from "./selections"

const selection = (): CleanupSelection => ({
  path: "/config/retired.cfg",
  sha512: "a".repeat(128),
  context: {
    platform: "modrinth",
    project_id: "pack",
    version_id: "installed"
  },
  proven: true
})
const row = (s: CleanupSelection): FEUntrackedFile => ({
  path: s.path,
  size: 8,
  label: "Unknown",
  deletable: true,
  origin: { ShippedIn: { version_id: "old", version_name: "Old" } },
  selection: s
})

describe("repair selections", () => {
  it("freezes the exact selected content and context", () => {
    const original = selection()
    const frozen = freezeSelection(original)
    original.context.version_id = "other"
    original.sha512 = "b".repeat(128)
    expect(frozen.context.version_id).toBe("installed")
    expect(frozen.sha512).toBe("a".repeat(128))
  })
  it("deselects changed or no longer proven rows on refetch", () => {
    const old = selection()
    const selected = new Map([[old.path, freezeSelection(old)]])
    for (const fresh of [
      { ...old, sha512: "b".repeat(128) },
      { ...old, proven: false },
      { ...old, context: { ...old.context, version_id: "other" } }
    ]) {
      expect(reconcileSelections(selected, [row(fresh)]).size).toBe(0)
    }
    expect(reconcileSelections(selected, []).size).toBe(0)
    expect(
      reconcileSelections(selected, [row(selection())]).get(old.path)
    ).toEqual(old)
  })
})
