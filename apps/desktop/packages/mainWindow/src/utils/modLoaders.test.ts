import { describe, expect, it } from "vitest"
import { compatibleLoaders } from "./modLoaders"

describe("compatibleLoaders", () => {
  it("adds the loaders the core also accepts, own loader first", () => {
    expect(compatibleLoaders("quilt", "1.21")).toEqual(["quilt", "fabric"])
    expect(compatibleLoaders("neoforge", "1.20.1")).toEqual([
      "neoforge",
      "forge"
    ])
    expect(compatibleLoaders("neoforge", "1.21.1")).toEqual(["neoforge"])
    expect(compatibleLoaders("fabric", "1.20.1")).toEqual(["fabric"])
    expect(compatibleLoaders("forge", "1.20.1")).toEqual(["forge"])
  })
})
