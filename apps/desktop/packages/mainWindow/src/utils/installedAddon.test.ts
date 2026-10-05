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
