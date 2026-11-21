import { describe, expect, it } from "vitest"
import type { FEWorld, FEWorldDatapack, Mod } from "@gd/core_module/bindings"
import { datapackCopies } from "./datapackCopies"

const modrinth = (project_id: string) => ({
  project_id,
  version_id: "v",
  title: "t",
  version: "1",
  urlslug: "s",
  description: "",
  authors: "",
  has_image: false
})
const curseforge = (project_id: number) => ({
  project_id,
  file_id: 1,
  name: "n",
  version: "1",
  urlslug: "s",
  summary: "",
  authors: "",
  has_image: false
})
const pack = (
  id: string,
  ids: { mr?: string; cf?: number },
  has_update: boolean,
  is_folder = false
): FEWorldDatapack => ({
  id,
  filename: `${id}.zip`,
  file_size: 1,
  is_folder,
  name: ids.mr || ids.cf ? id : null,
  version: null,
  curseforge: ids.cf ? curseforge(ids.cf) : null,
  modrinth: ids.mr ? modrinth(ids.mr) : null,
  has_update
})
const world = (name: string, datapacks: FEWorldDatapack[]): FEWorld => ({
  name,
  addon_id: `addon-${name}`,
  identity: name,
  datapacks
})
const row = (
  id: string,
  addon_type: Mod["addon_type"],
  ids: { mr?: string; cf?: number },
  has_update: boolean
): Mod => ({
  id,
  filename: `${id}.zip`,
  enabled: true,
  addon_type,
  metadata: null,
  curseforge: ids.cf ? curseforge(ids.cf) : null,
  modrinth: ids.mr ? modrinth(ids.mr) : null,
  has_update,
  is_duplicate: false,
  file_size: 1
})

describe("datapackCopies", () => {
  it("counts the pack itself when it is the only copy with an update", () => {
    const own = pack("a", { mr: "terra" }, true)
    expect(datapackCopies([world("A", [own])], [], own)).toEqual({
      withUpdate: 1
    })
  })

  it("counts world copies and instance-level data packs of the same project", () => {
    const own = pack("a", { mr: "terra" }, true)
    const worlds = [
      world("A", [own, pack("other", { mr: "else" }, true)]),
      world("B", [pack("b", { mr: "terra" }, true)]),
      world("C", [pack("c", { mr: "terra" }, false)])
    ]
    const mods = [
      row("i", "datapacks", { mr: "terra" }, true),
      row("j", "datapacks", { mr: "terra" }, false)
    ]
    expect(datapackCopies(worlds, mods, own)).toEqual({ withUpdate: 3 })
  })

  it("matches a copy known only by its CurseForge project", () => {
    const own = pack("a", { mr: "terra", cf: 42 }, true)
    const worlds = [
      world("A", [own]),
      world("B", [pack("b", { cf: 42 }, true)])
    ]
    const mods = [row("i", "datapacks", { cf: 42 }, true)]
    expect(datapackCopies(worlds, mods, own)).toEqual({ withUpdate: 3 })
  })

  it("ignores instance rows that are not data packs, and folders", () => {
    const own = pack("a", { mr: "terra" }, true)
    const worlds = [
      world("A", [own]),
      world("B", [pack("b", { mr: "terra" }, true, true)])
    ]
    const mods = [
      row("m", "mods", { mr: "terra" }, true),
      row("w", "worlds", { mr: "terra" }, true)
    ]
    expect(datapackCopies(worlds, mods, own)).toEqual({ withUpdate: 1 })
  })

  it("finds no copies for a pack no platform knows", () => {
    const own = pack("a", {}, false)
    const worlds = [world("A", [own, pack("b", {}, true)])]
    expect(datapackCopies(worlds, [], own)).toEqual({ withUpdate: 0 })
  })
})
