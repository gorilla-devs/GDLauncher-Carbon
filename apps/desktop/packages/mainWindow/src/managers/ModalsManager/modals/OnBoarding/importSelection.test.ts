import { describe, expect, it } from "vitest"
import { scanRows, selectedImport } from "./importSelection"
import type { ImportEntry } from "@gd/core_module/bindings"
const alpha: ImportEntry = {
  Valid: { filename: "synthetic-a", instance_name: "Alpha", warnings: [] }
}
const zulu: ImportEntry = {
  Valid: { filename: "synthetic-z", instance_name: "Zulu", warnings: [] }
}
const invalid: ImportEntry = {
  Invalid: {
    name: "Broken",
    reason: { translation: "InstanceImportLegacyBadConfigFile" }
  }
}
describe("import selection", () => {
  it.each([
    [zulu, alpha],
    [invalid, alpha]
  ])("retains backend identity before sorting", (...entries) => {
    const rows = scanRows(entries, "opaque-A")
    expect(rows[0]).toMatchObject({ filename: "synthetic-a" })
    expect(selectedImport(rows, rows[0].selectionId)).toEqual({
      index: 1,
      name: "Alpha",
      session: "opaque-A"
    })
  })
  it("distinguishes equal folder names from separate associated roots", () => {
    const rows = scanRows([alpha, alpha], "opaque-duplicates")
    expect(selectedImport(rows, "opaque-duplicates:1")).toEqual({
      index: 1,
      name: "Alpha",
      session: "opaque-duplicates"
    })
  })
  it("preserves sorted input and rejects an unavailable selection", () => {
    const rows = scanRows([alpha, zulu], "opaque-B")
    expect(selectedImport(rows, rows[0].selectionId).index).toBe(0)
    expect(() => selectedImport(rows, "missing")).toThrow()
  })
})
