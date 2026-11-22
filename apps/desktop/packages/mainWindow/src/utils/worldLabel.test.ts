import { describe, expect, it } from "vitest"
import { labelOfFolder, worldLabel, worldNames } from "./worldLabel"

const world = (name: string, title: string | null) => ({ name, title })

describe("worldLabel", () => {
  it("is the in-game name when there is one", () => {
    expect(worldLabel(world("New World (2)", "Skyblock"))).toBe("Skyblock")
  })
  it("is the folder name without an in-game name", () => {
    expect(worldLabel(world("Creative", null))).toBe("Creative")
  })
  it("is the folder name for an empty in-game name", () => {
    expect(worldLabel(world("Creative", ""))).toBe("Creative")
  })
  it("is the folder name for an in-game name of only spaces", () => {
    expect(worldLabel(world("Creative", "  \t "))).toBe("Creative")
  })
})

describe("worldNames", () => {
  it("adds the folder only to worlds that share a label", () => {
    const names = worldNames([
      world("Survival", "Survival"),
      world("Survival (2)", "Survival"),
      world("Creative", null)
    ])
    expect(names.get("Survival")).toEqual({
      label: "Survival",
      folder: "Survival"
    })
    expect(names.get("Survival (2)")).toEqual({
      label: "Survival",
      folder: "Survival (2)"
    })
    expect(names.get("Creative")).toEqual({ label: "Creative" })
  })
  it("counts a folder name as the label of an untitled world", () => {
    const names = worldNames([world("A", null), world("B", "A")])
    expect(names.get("A")?.folder).toBe("A")
    expect(names.get("B")?.folder).toBe("B")
  })
})

describe("labelOfFolder", () => {
  const worlds = [world("New World", "Skyblock")]
  it("gives the label of the world with that folder", () => {
    expect(labelOfFolder(worlds, "New World")).toBe("Skyblock")
  })
  it("gives the folder name when no world has it", () => {
    expect(labelOfFolder(worlds, "Gone")).toBe("Gone")
  })
  it("gives the folder name without a list", () => {
    expect(labelOfFolder(undefined, "New World")).toBe("New World")
  })
})
