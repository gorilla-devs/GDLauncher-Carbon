import { describe, expect, it } from "vitest"
import type {
  FEUnifiedSearchResult,
  ServerAddon
} from "@gd/core_module/bindings"
import { findInstalledAddon, isAddonInstalled } from "./installedAddon"

function result(
  platform: FEUnifiedSearchResult["platform"],
  id: string,
  slug: string
): FEUnifiedSearchResult {
  return {
    title: "Fabric API",
    slug,
    description: "",
    imageUrl: null,
    highResImageUrl: null,
    downloadsCount: 0,
    id,
    releaseDate: "",
    lastUpdated: "",
    platform,
    type: "mod",
    authors: [],
    websiteUrl: null,
    categories: [],
    screenshotUrls: [],
    minecraftVersions: [],
    versions: null,
    mainFileId: null,
    serverPackFileId: null
  }
}

function serverAddon(overrides: Partial<ServerAddon>): ServerAddon {
  return {
    id: "row",
    filename: "fabric-api-0.92.2+1.20.1.jar",
    displayName: "Fabric API",
    enabled: true,
    addonType: "mods",
    fileSize: 1,
    hasImage: false,
    curseforgeProjectId: null,
    modrinthProjectId: null,
    ...overrides
  }
}

const curseforge = result("curseforge", "306612", "fabric-api")
const modrinth = result("modrinth", "P7dR8mSH", "fabric-api")

describe("installed server addons", () => {
  // The core lists a jar installed from Modrinth with the CurseForge project
  // that publishes the same bytes too, so the search shows it installed on
  // both platforms and does not invite a second copy.
  it("a server addon known to both platforms is installed for both results", () => {
    const target = {
      kind: "server" as const,
      addons: [
        serverAddon({
          curseforgeProjectId: 306612,
          modrinthProjectId: "P7dR8mSH"
        })
      ]
    }
    for (const addon of [curseforge, modrinth]) {
      expect(findInstalledAddon(addon, target)?.id).toBe("row")
      expect(isAddonInstalled(addon, target, null)).toBe(true)
    }
  })

  it("a server addon known to one platform is installed only for that one", () => {
    const target = {
      kind: "server" as const,
      addons: [serverAddon({ modrinthProjectId: "P7dR8mSH" })]
    }
    expect(isAddonInstalled(modrinth, target, null)).toBe(true)
    expect(isAddonInstalled(curseforge, target, null)).toBe(false)
  })
})

describe("installed in an instance", () => {
  const row = (addon_type: string, version_id = "V1") =>
    ({
      id: `row-${addon_type}`,
      addon_type,
      modrinth: { project_id: "P", version_id },
      curseforge: null
    }) as any
  const instance = (...addons: any[]) => ({ kind: "instance" as const, addons })
  // Modrinth types a data pack "mod" and lists "datapack" among its loaders.
  const modrinthPack = {
    ...result("modrinth", "P", "pack"),
    loaders: ["datapack"]
  }
  const modrinthHybrid = {
    ...result("modrinth", "P", "pack"),
    loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
  }
  const plainMod = { ...result("modrinth", "P", "mod"), loaders: ["fabric"] }

  it.each([
    ["that is only a data pack", modrinthPack],
    ["that also ships as a mod", modrinthHybrid]
  ])(
    "finds the data pack build of a Modrinth project %s among the instance's data packs",
    (_, addon) => {
      const target = instance(row("datapacks"))
      expect(findInstalledAddon(addon as any, target)?.id).toBe("row-datapacks")
      expect(isAddonInstalled(addon as any, target, "V1")).toBe(true)
      expect(isAddonInstalled(addon as any, target, "V2")).toBe(false)
    }
  )
  it("still finds the mod jar of a project that also ships as a mod", () => {
    expect(
      findInstalledAddon(modrinthHybrid as any, instance(row("mods")))?.id
    ).toBe("row-mods")
  })
  it("never takes a data pack row for a plain mod", () => {
    expect(
      findInstalledAddon(plainMod as any, instance(row("datapacks")))
    ).toBeUndefined()
  })
  it("finds a CurseForge data pack among the instance's data packs only", () => {
    const addon = {
      ...result("curseforge", "12", "pack"),
      type: "datapack" as const,
      loaders: []
    }
    const copy = (addon_type: string) => ({
      id: addon_type,
      addon_type,
      curseforge: { project_id: 12, file_id: 3 },
      modrinth: null
    })
    expect(
      findInstalledAddon(addon, instance(copy("datapacks")) as any)?.id
    ).toBe("datapacks")
    expect(
      findInstalledAddon(addon, instance(copy("mods")) as any)
    ).toBeUndefined()
  })
})
