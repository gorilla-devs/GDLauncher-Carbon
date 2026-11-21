import { describe, expect, it } from "vitest"
import type { FEWorld, FEWorldDatapack } from "@gd/core_module/bindings"
import {
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

describe("worldTargetName", () => {
  it("names the world only beside an instance and only for a data pack", () => {
    expect(worldTargetName(3, "datapack", "My World")).toBe("My World")
    expect(worldTargetName(undefined, "datapack", "My World")).toBeUndefined()
    expect(worldTargetName(NaN, "datapack", "My World")).toBeUndefined()
    expect(worldTargetName(3, "mod", "My World")).toBeUndefined()
    expect(worldTargetName(3, "resourcePack", "My World")).toBeUndefined()
    expect(worldTargetName(3, "world", "My World")).toBeUndefined()
    expect(worldTargetName(3, "datapack", undefined)).toBeUndefined()
    expect(worldTargetName(3, "datapack", "")).toBeUndefined()
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
  it("never matches another platform or another addon type", () => {
    const target = { kind: "world" as const, datapacks: packs }
    expect(
      findInstalledAddon(addon("datapack", "curseforge", "P"), target)
    ).toBeUndefined()
    expect(findInstalledAddon(addon("mod"), target)).toBeUndefined()
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
