import { describe, expect, it } from "vitest"
import path from "node:path"
import { ADDON_FIXTURES, ADDON_FOLDER, addonDir } from "./addonFixtures.js"

describe("ADDON_FOLDER", () => {
  // Mirrors AddonType::get_folder_path (domain/instance/mod.rs). The two
  // mismatches are the whole point of pinning this: `shaders` lives in
  // `shaderpacks/`, and `worlds` lives in `saves/`.
  it("maps every type to its real on-disk folder", () => {
    expect(ADDON_FOLDER).toEqual({
      resourcepacks: "resourcepacks",
      shaders: "shaderpacks",
      datapacks: "datapacks",
      worlds: "saves"
    })
  })

  it("joins onto an instance root", () => {
    expect(addonDir("/i", "shaders")).toBe(path.join("/i", "shaderpacks"))
  })
})

describe("ADDON_FIXTURES", () => {
  it("covers six platform/type combinations", () => {
    expect(ADDON_FIXTURES).toHaveLength(6)
  })

  // Modrinth has no world project type — FEUnifiedSearchType::World maps to
  // ProjectType::Unknown (api/modplatforms/responses.rs).
  it("has worlds on CurseForge only", () => {
    const worlds = ADDON_FIXTURES.filter((f) => f.addonType === "worlds")
    expect(worlds).toHaveLength(1)
    expect(worlds[0].platform).toBe("curseforge")
  })

  // Modrinth's search facet associates plenty of content with the datapack
  // content type, but a live census (see `ADDON_FIXTURES`'s own doc
  // comment) found zero of those 4,801 projects have the literal
  // `project_type` value "datapack" — the ecosystem files this content as
  // "mod" plus a datapack loader instead, which installs into `mods/` via
  // ModrinthModInstaller::get_install_path, not `datapacks/`. So datapacks
  // are CurseForge-only, the same shape as worlds above, for a different
  // reason.
  it("has datapacks on CurseForge only", () => {
    const datapacks = ADDON_FIXTURES.filter((f) => f.addonType === "datapacks")
    expect(datapacks).toHaveLength(1)
    expect(datapacks[0].platform).toBe("curseforge")
  })

  it("has both platforms for every other type", () => {
    for (const t of ["resourcepacks", "shaders"] as const) {
      const platforms = ADDON_FIXTURES.filter((f) => f.addonType === t).map(
        (f) => f.platform
      )
      expect(platforms.sort()).toEqual(["curseforge", "modrinth"])
    }
  })

  it("gives every fixture a non-empty project id", () => {
    for (const f of ADDON_FIXTURES) {
      expect(f.projectId.length).toBeGreaterThan(0)
    }
  })
})
