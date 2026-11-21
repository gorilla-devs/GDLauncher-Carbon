import { describe, expect, it } from "vitest"
import type { FEWorld, FEWorldDatapack } from "@gd/core_module/bindings"
import {
  asksWhichWorld,
  canBeDatapack,
  installPlaces,
  instanceInstallIsDatapack,
  isOnlyDatapack,
  newestDatapackBuild,
  readableWorld,
  withoutWorldParam,
  worldDatapackInstall,
  worldDatapackSource,
  worldTargetName
} from "./worldTarget"
import { findInstalledAddon, isAddonInstalled } from "./installedAddon"

const pack = (
  id: string,
  platform: "modrinth" | "curseforge",
  project: string,
  version: string
): FEWorldDatapack => ({
  id,
  filename: `${id}.zip`,
  file_size: 1,
  is_folder: false,
  name: null,
  version: null,
  has_update: false,
  modrinth:
    platform === "modrinth"
      ? {
          project_id: project,
          version_id: version,
          title: "",
          version: "",
          urlslug: "",
          description: "",
          authors: "",
          has_image: false
        }
      : null,
  curseforge:
    platform === "curseforge"
      ? {
          project_id: Number(project),
          file_id: Number(version),
          name: "",
          version: "",
          urlslug: "",
          summary: "",
          authors: "",
          has_image: false
        }
      : null
})
const world = (
  name: string,
  datapacks: FEWorldDatapack[] = [],
  identity: string | null = `id-${name}`
): FEWorld => ({ name, addon_id: `addon-${name}`, identity, datapacks })
const addon = (
  type = "datapack",
  platform: "modrinth" | "curseforge" = "modrinth",
  id = "P"
) => ({ id, platform, type, title: "Pack", slug: "pack" }) as any

// The rows the platforms really return for a data pack search.
const modrinthHybrid = {
  type: "mod",
  loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
}
const modrinthPack = { type: "mod", loaders: ["datapack"] }
const curseforgePack = { type: "datapack", loaders: [] }
const plainMod = { type: "mod", loaders: ["fabric"] }

describe("data pack projects", () => {
  it("a project can be a data pack by its type or by a datapack loader", () => {
    expect(canBeDatapack(modrinthHybrid)).toBe(true)
    expect(canBeDatapack(modrinthPack)).toBe(true)
    expect(canBeDatapack(curseforgePack)).toBe(true)
    expect(canBeDatapack(plainMod)).toBe(false)
    expect(canBeDatapack({ type: "mod" })).toBe(false)
    expect(canBeDatapack({ type: "resourcePack", loaders: [] })).toBe(false)
    expect(canBeDatapack(undefined)).toBe(false)
  })
  it("a project is only a data pack when nothing else can load it", () => {
    expect(isOnlyDatapack(modrinthHybrid)).toBe(false)
    expect(isOnlyDatapack(modrinthPack)).toBe(true)
    expect(isOnlyDatapack(curseforgePack)).toBe(true)
    expect(isOnlyDatapack(plainMod)).toBe(false)
    expect(isOnlyDatapack(undefined)).toBe(false)
  })
  it("a world is asked for a pure data pack always, for a hybrid only in a data pack search", () => {
    for (const searched of ["datapack", "mod", null, undefined]) {
      expect(asksWhichWorld(modrinthPack, searched)).toBe(true)
      expect(asksWhichWorld(curseforgePack, searched)).toBe(true)
      expect(asksWhichWorld(plainMod, searched)).toBe(false)
    }
    expect(asksWhichWorld(modrinthHybrid, "datapack")).toBe(true)
    expect(asksWhichWorld(modrinthHybrid, "mod")).toBe(false)
    expect(asksWhichWorld(modrinthHybrid, undefined)).toBe(false)
  })
})

describe("what an install into the instance itself is", () => {
  it("is the data pack build of a project that is only a data pack, with or without a mod loader", () => {
    for (const hasModLoader of [true, false]) {
      expect(instanceInstallIsDatapack(curseforgePack, hasModLoader)).toBe(true)
      expect(instanceInstallIsDatapack(modrinthPack, hasModLoader)).toBe(true)
    }
  })
  it("is the data pack build of a project that also ships as a mod only without a mod loader", () => {
    expect(instanceInstallIsDatapack(modrinthHybrid, false)).toBe(true)
    expect(instanceInstallIsDatapack(modrinthHybrid, true)).toBe(false)
  })
  it("is never a data pack for a mod or another addon", () => {
    for (const hasModLoader of [true, false]) {
      expect(instanceInstallIsDatapack(plainMod, hasModLoader)).toBe(false)
      expect(
        instanceInstallIsDatapack({ type: "shader", loaders: [] }, hasModLoader)
      ).toBe(false)
      expect(instanceInstallIsDatapack(undefined, hasModLoader)).toBe(false)
    }
  })
})

describe("the data pack build an instance gets from Modrinth", () => {
  const build = (
    id: string,
    day: number,
    type: "release" | "beta" | "alpha",
    files: [string, boolean?][]
  ) => ({
    id,
    version_type: type,
    date_published: `2026-01-${String(day).padStart(2, "0")}T00:00:00Z`,
    files: files.map(([filename, primary = false]) => ({ filename, primary }))
  })
  it("is the newest release, even under a newer beta or alpha", () => {
    expect(
      newestDatapackBuild([
        build("alpha", 9, "alpha", [["a.zip", true]]),
        build("beta", 8, "beta", [["b.zip", true]]),
        build("new", 7, "release", [["new.zip", true]]),
        build("old", 2, "release", [["old.zip", true]])
      ])?.id
    ).toBe("new")
  })
  it("is the newest whatever order the versions come in", () => {
    expect(
      newestDatapackBuild([
        build("old", 2, "release", [["old.zip", true]]),
        build("new", 7, "release", [["new.zip", true]])
      ])?.id
    ).toBe("new")
  })
  it("is the newest beta without a release, and the newest alpha without either", () => {
    expect(
      newestDatapackBuild([
        build("alpha", 9, "alpha", [["a.zip", true]]),
        build("beta", 8, "beta", [["b.zip", true]])
      ])?.id
    ).toBe("beta")
    expect(
      newestDatapackBuild([
        build("alpha-new", 9, "alpha", [["a.zip", true]]),
        build("alpha-old", 3, "alpha", [["a.zip", true]])
      ])?.id
    ).toBe("alpha-new")
  })
  it("judges a version by its primary file, or its first without one", () => {
    expect(
      newestDatapackBuild([
        build("jar", 9, "release", [["extra.zip"], ["mod.jar", true]]),
        build("first-zip", 8, "release", [["Pack.ZIP"], ["mod.jar"]]),
        build("zip", 7, "release", [["mod.jar"], ["pack.zip", true]])
      ])?.id
    ).toBe("first-zip")
    expect(
      newestDatapackBuild([
        build("first-jar", 9, "release", [["mod.jar"], ["pack.zip"]]),
        build("no-file", 8, "release", [])
      ])
    ).toBeUndefined()
  })
  it("is none when the project has no version", () => {
    expect(newestDatapackBuild([])).toBeUndefined()
  })
})

describe("worldTargetName", () => {
  it("names the world only beside an instance and only for a project that can be a data pack", () => {
    expect(worldTargetName(3, curseforgePack, "My World")).toBe("My World")
    expect(worldTargetName(3, modrinthHybrid, "My World")).toBe("My World")
    expect(worldTargetName(3, modrinthPack, "My World")).toBe("My World")
    expect(
      worldTargetName(undefined, curseforgePack, "My World")
    ).toBeUndefined()
    expect(worldTargetName(NaN, modrinthHybrid, "My World")).toBeUndefined()
    expect(worldTargetName(3, plainMod, "My World")).toBeUndefined()
    expect(
      worldTargetName(3, { type: "resourcePack" }, "My World")
    ).toBeUndefined()
    expect(worldTargetName(3, { type: "world" }, "My World")).toBeUndefined()
    expect(worldTargetName(3, undefined, "My World")).toBeUndefined()
    expect(worldTargetName(3, curseforgePack, undefined)).toBeUndefined()
    expect(worldTargetName(3, modrinthHybrid, "")).toBeUndefined()
  })
})

describe("readableWorld", () => {
  it("keeps a world only when the core could read it", () => {
    expect(readableWorld(world("A"))?.identity).toBe("id-A")
    expect(readableWorld(world("A", [], null))).toBeUndefined()
    expect(readableWorld(undefined)).toBeUndefined()
  })
})

describe("worldDatapackSource", () => {
  it("asks for the newest build when no file is chosen", () => {
    expect(worldDatapackSource(addon(), undefined)).toEqual({
      Modrinth: { project_id: "P", version_id: null }
    })
    expect(worldDatapackSource(addon("datapack", "curseforge", "12"))).toEqual({
      Curseforge: { project_id: 12, file_id: null }
    })
  })
  it("carries a chosen file", () => {
    expect(worldDatapackSource(addon(), "V2")).toEqual({
      Modrinth: { project_id: "P", version_id: "V2" }
    })
    expect(
      worldDatapackSource(addon("datapack", "curseforge", "12"), "34")
    ).toEqual({ Curseforge: { project_id: 12, file_id: 34 } })
  })
})

describe("worldDatapackInstall", () => {
  it("installs into the world with its identity and replaces nothing new", () => {
    const target = readableWorld(world("A"))!
    expect(worldDatapackInstall(addon(), "V1", 3, target)).toEqual({
      instance_id: 3,
      world: "A",
      identity: "id-A",
      source: { Modrinth: { project_id: "P", version_id: "V1" } },
      replaces: null
    })
  })
  it("replaces the world's own copy of the same project", () => {
    const target = readableWorld(
      world("A", [
        pack("other", "modrinth", "Q", "V9"),
        pack("copy", "modrinth", "P", "V1")
      ])
    )!
    expect(worldDatapackInstall(addon(), "V2", 3, target).replaces).toBe("copy")
    const cf = readableWorld(
      world("A", [pack("cf-copy", "curseforge", "12", "30")])
    )!
    expect(
      worldDatapackInstall(addon("datapack", "curseforge", "12"), 34, 3, cf)
        .replaces
    ).toBe("cf-copy")
  })
})

describe("installed in a world", () => {
  const packs = [pack("copy", "modrinth", "P", "V1")]
  it("judges the project and the exact version against the world's packs", () => {
    const target = { kind: "world" as const, datapacks: packs }
    expect(findInstalledAddon(addon(), target)?.id).toBe("copy")
    expect(isAddonInstalled(addon(), target, undefined)).toBe(true)
    expect(isAddonInstalled(addon(), target, "V1")).toBe(true)
    expect(isAddonInstalled(addon(), target, "V2")).toBe(false)
    expect(
      isAddonInstalled(addon(), { kind: "world", datapacks: [] }, "V1")
    ).toBe(false)
  })
  it("finds and replaces the world's copy of a Modrinth data pack typed as a mod", () => {
    const hybrid = { ...addon("mod"), loaders: modrinthHybrid.loaders }
    const target = { kind: "world" as const, datapacks: packs }
    expect(findInstalledAddon(hybrid, target)?.id).toBe("copy")
    expect(isAddonInstalled(hybrid, target, "V2")).toBe(false)
    expect(
      worldDatapackInstall(hybrid, "V2", 3, readableWorld(world("A", packs))!)
        .replaces
    ).toBe("copy")
  })
  it("never matches another platform or another addon type", () => {
    const target = { kind: "world" as const, datapacks: packs }
    expect(
      findInstalledAddon(addon("datapack", "curseforge", "P"), target)
    ).toBeUndefined()
    expect(findInstalledAddon(addon("mod"), target)).toBeUndefined()
  })
})

describe("where a data pack install can go", () => {
  const held = [pack("copy", "modrinth", "P", "V1")]
  const project = { ...addon("mod"), loaders: modrinthPack.loaders }
  it("lists the readable worlds with whether each holds the project", () => {
    const places = installPlaces(
      project,
      [world("A", held), world("Broken", [], null), world("B")],
      false
    )
    expect(
      places.worlds.map((entry) => [entry.world.name, entry.holds])
    ).toEqual([
      ["A", true],
      ["B", false]
    ])
    expect(places.offers).toBe("choice")
  })
  it("asks while a world can take it, even when the instance holds it", () => {
    expect(installPlaces(project, [world("A")], true).offers).toBe("choice")
  })
  it("asks while only the instance can take it and there are worlds", () => {
    expect(installPlaces(project, [world("A", held)], false).offers).toBe(
      "choice"
    )
  })
  it.each([
    ["no world", []],
    ["only unreadable worlds", [world("Broken", [], null)]],
    ["worlds that could not be read", undefined]
  ])("goes into the instance without asking with %s", (_, worlds) => {
    expect(installPlaces(project, worlds, false).offers).toBe("instance")
  })
  it.each([
    ["no world", []],
    ["only worlds that hold it too", [world("A", held)]]
  ])("offers nothing once the instance holds it with %s", (_, worlds) => {
    expect(installPlaces(project, worlds, true).offers).toBe("nothing")
  })
  it("matches a world's copy by the real row shapes only", () => {
    const worlds = [world("A", held)]
    const hybrid = { ...addon("mod"), loaders: modrinthHybrid.loaders }
    expect(installPlaces(hybrid, worlds, false).worlds[0].holds).toBe(true)
    const plain = { ...addon("mod"), loaders: plainMod.loaders }
    expect(installPlaces(plain, worlds, false).worlds[0].holds).toBe(false)
    const curse = {
      ...addon("datapack", "curseforge", "12"),
      loaders: curseforgePack.loaders
    }
    expect(
      installPlaces(
        curse,
        [world("A", [pack("c", "curseforge", "12", "3")])],
        false
      ).worlds[0].holds
    ).toBe(true)
  })
})

describe("withoutWorldParam", () => {
  it("drops only the world", () => {
    expect(withoutWorldParam("?instanceId=3&world=My%20World")).toBe(
      "?instanceId=3"
    )
    expect(withoutWorldParam("?world=A")).toBe("")
    expect(withoutWorldParam("?instanceId=3")).toBe("?instanceId=3")
    expect(withoutWorldParam("")).toBe("")
  })
})
