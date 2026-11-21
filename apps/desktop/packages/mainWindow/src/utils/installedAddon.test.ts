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
  const instance = (
    install: "datapack" | "addon" | "unknown",
    ...addons: any[]
  ) => ({ kind: "instance" as const, addons, install })
  // Modrinth types a data pack "mod" and lists "datapack" among its loaders.
  const modrinthPack = {
    ...result("modrinth", "P", "pack"),
    loaders: ["datapack"]
  } as any
  const modrinthHybrid = {
    ...result("modrinth", "P", "pack"),
    loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
  } as any
  const plainMod = {
    ...result("modrinth", "P", "mod"),
    loaders: ["fabric"]
  } as any

  it.each([
    ["that is only a data pack", modrinthPack],
    ["that also ships as a mod", modrinthHybrid]
  ])(
    "an install of the data pack build of a Modrinth project %s is compared with the instance's data packs only",
    (_, addon) => {
      const held = instance("datapack", row("mods", "J1"), row("datapacks"))
      expect(findInstalledAddon(addon, held)?.id).toBe("row-datapacks")
      expect(isAddonInstalled(addon, held, "V1")).toBe(true)
      expect(isAddonInstalled(addon, held, "V2")).toBe(false)
      const jarOnly = instance("datapack", row("mods"))
      expect(findInstalledAddon(addon, jarOnly)).toBeUndefined()
      expect(findInstalledAddon(addon, jarOnly, "V1")).toBeUndefined()
      expect(isAddonInstalled(addon, jarOnly, null)).toBe(false)
    }
  )
  it("an install of the mod of a project that also ships as a mod is compared with the instance's mods only", () => {
    const held = instance("addon", row("datapacks", "Z1"), row("mods"))
    expect(findInstalledAddon(modrinthHybrid, held)?.id).toBe("row-mods")
    expect(isAddonInstalled(modrinthHybrid, held, "V1")).toBe(true)
    const packOnly = instance("addon", row("datapacks"))
    expect(findInstalledAddon(modrinthHybrid, packOnly)).toBeUndefined()
    expect(findInstalledAddon(modrinthHybrid, packOnly, "V1")).toBeUndefined()
    expect(isAddonInstalled(modrinthHybrid, packOnly, null)).toBe(false)
  })
  it("a picked version that could be either is only the row of that very version", () => {
    const held = instance("unknown", row("mods", "J1"), row("datapacks", "Z1"))
    expect(findInstalledAddon(modrinthHybrid, held, "J1")?.id).toBe("row-mods")
    expect(findInstalledAddon(modrinthHybrid, held, "Z1")?.id).toBe(
      "row-datapacks"
    )
    expect(isAddonInstalled(modrinthHybrid, held, "Z1")).toBe(true)
    // Another version replaces neither row, and no version names none.
    expect(findInstalledAddon(modrinthHybrid, held, "J2")).toBeUndefined()
    expect(findInstalledAddon(modrinthHybrid, held)).toBeUndefined()
    expect(isAddonInstalled(modrinthHybrid, held, null)).toBe(false)
  })
  it("never takes a data pack row for a plain mod", () => {
    expect(
      findInstalledAddon(plainMod, instance("addon", row("datapacks")))
    ).toBeUndefined()
    expect(
      findInstalledAddon(plainMod, instance("addon", row("mods")))?.id
    ).toBe("row-mods")
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
    for (const install of ["datapack", "addon"] as const) {
      expect(
        findInstalledAddon(addon, instance(install, copy("datapacks")))?.id
      ).toBe("datapacks")
      expect(
        findInstalledAddon(addon, instance(install, copy("mods")))
      ).toBeUndefined()
    }
  })
})
