import { describe, expect, it } from "vitest"
import {
  addonPageTarget,
  explainNotDownloaded,
  isSearchRequestFor,
  updateTaskId,
  type AddonPageTarget,
  type InstalledMod
} from "./mods.js"

/** Builds the URL `rspcFetch` (`utils/rspcClient.ts`) sends for a query. */
const rspcUrl = (key: string, input: unknown) =>
  `http://127.0.0.1:4650/rspc/${key}?input=${encodeURIComponent(JSON.stringify(input))}`

/** The input `platformInfiniteOptions` (`utils/platformSearch.ts`) sends. */
const searchInput = (searchQuery: string, searchApi: string | null) => ({
  searchQuery,
  categories: null,
  gameVersions: ["1.20.1"],
  modloaders: ["fabric"],
  pageSize: 20,
  projectType: "resourcePack",
  index: 0,
  searchApi,
  environment: null,
  platformFilters: null
})

describe("isSearchRequestFor", () => {
  it("matches the filled query's search on the chosen platform", () => {
    const url = rspcUrl(
      "modplatforms.unifiedSearch",
      searchInput("Low On Fire", "modrinth")
    )
    expect(isSearchRequestFor(url, "Low On Fire", "modrinth")).toBe(true)
  })

  it("refuses the earlier empty-query search that left rows on screen", () => {
    const url = rspcUrl(
      "modplatforms.unifiedSearch",
      searchInput("", "modrinth")
    )
    expect(isSearchRequestFor(url, "Low On Fire", "modrinth")).toBe(false)
  })

  it("refuses the same text on the other platform or on both", () => {
    for (const searchApi of ["curseforge", null]) {
      const url = rspcUrl(
        "modplatforms.unifiedSearch",
        searchInput("cloth config", searchApi)
      )
      expect(isSearchRequestFor(url, "cloth config", "modrinth")).toBe(false)
    }
  })

  it("refuses a query text that only starts like the filled one", () => {
    const url = rspcUrl(
      "modplatforms.unifiedSearch",
      searchInput("cloth", "modrinth")
    )
    expect(isSearchRequestFor(url, "cloth config", "modrinth")).toBe(false)
  })

  it("refuses other procedures and malformed URLs", () => {
    const input = searchInput("fabric api", "modrinth")
    expect(
      isSearchRequestFor(
        rspcUrl("modplatforms.unifiedGetProject", input),
        "fabric api",
        "modrinth"
      )
    ).toBe(false)
    expect(
      isSearchRequestFor(
        "http://127.0.0.1:4650/rspc/modplatforms.unifiedSearch",
        "fabric api",
        "modrinth"
      )
    ).toBe(false)
    expect(
      isSearchRequestFor(
        "http://127.0.0.1:4650/rspc/modplatforms.unifiedSearch?input=%7Bnot-json",
        "fabric api",
        "modrinth"
      )
    ).toBe(false)
    expect(isSearchRequestFor("not a url", "fabric api", "modrinth")).toBe(
      false
    )
  })
})

describe("addonPageTarget", () => {
  const app = "file:///opt/gdlauncher/resources/app.asar/index.html"

  it("reads the addon, platform and instance off the page and its tabs", () => {
    expect(
      addonPageTarget(`${app}#/addon/P7dR8mSH/modrinth?instanceId=5`)
    ).toEqual({ projectId: "P7dR8mSH", platform: "modrinth", instanceId: 5 })
    expect(
      addonPageTarget(`${app}#/addon/394468/curseforge/versions?instanceId=12`)
    ).toEqual({ projectId: "394468", platform: "curseforge", instanceId: 12 })
  })

  it("refuses other routes, a server's page and malformed URLs", () => {
    expect(addonPageTarget(`${app}#/library/5/addons`)).toBeUndefined()
    expect(
      addonPageTarget(`${app}#/addon/P7dR8mSH/modrinth?serverId=3`)
    ).toBeUndefined()
    expect(
      addonPageTarget(`${app}#/addon/P7dR8mSH/hangar?instanceId=5`)
    ).toBeUndefined()
    expect(addonPageTarget(app)).toBeUndefined()
    expect(addonPageTarget("not a url")).toBeUndefined()
  })
})

describe("explainNotDownloaded", () => {
  const target: AddonPageTarget = {
    projectId: "9s6osm5g",
    platform: "modrinth",
    instanceId: 5
  }
  const mod = (
    filename: string,
    modrinth?: { project: string; version: string }
  ): InstalledMod => ({
    id: filename,
    filename,
    fileSize: 1,
    enabled: true,
    addonType: "mods",
    curseforgeProjectId: null,
    modrinthProjectId: modrinth?.project ?? null,
    modrinthVersionId: modrinth?.version ?? null,
    curseforgeFileId: null,
    sha1: null,
    modloaders: [],
    hasUpdate: false
  })
  const fabricApi = mod("fabric-api.jar", { project: "P7dR8mSH", version: "a" })

  it("names a file that landed without the addon's identity", () => {
    const message = explainNotDownloaded(
      target,
      [fabricApi],
      [fabricApi, mod("cloth-config.jar")]
    )
    expect(message).toContain(
      '"cloth-config.jar" is installed but has no Modrinth identity'
    )
    expect(message).not.toContain("could not be read")
  })

  it("says the file is not installed when nothing new is listed", () => {
    const message = explainNotDownloaded(target, [fabricApi], [fabricApi])
    expect(message).toBe(
      "The file is not installed: instance 5 lists no new file for " +
        "Modrinth project 9s6osm5g."
    )
  })

  it("does not call an anonymous file that was there before new", () => {
    const leftover = mod("cloth-config.jar")
    const message = explainNotDownloaded(target, [leftover], [leftover])
    expect(message).toContain("The file is not installed")
    expect(message).toContain(
      'Already there before the click with no Modrinth identity: "cloth-config.jar"'
    )
  })

  it("separates a file that carries the identity from one that does not", () => {
    const installed = mod("cloth-config.jar", {
      project: "9s6osm5g",
      version: "2xQdCMyG"
    })
    expect(explainNotDownloaded(target, [], [installed])).toBe(
      'The file "cloth-config.jar" is installed as Modrinth project ' +
        '9s6osm5g, yet the button never read "Downloaded".'
    )
  })

  it("holds a Versions-tab row to its own version", () => {
    const older = mod("cloth-config-old.jar", {
      project: "9s6osm5g",
      version: "old"
    })
    const message = explainNotDownloaded(target, [older], [older], "new")
    expect(message).toContain(
      "lists no new file for Modrinth project 9s6osm5g version new"
    )
    expect(message).toContain(
      'Listed for that project: "cloth-config-old.jar" (Modrinth project ' +
        "9s6osm5g version old)"
    )
  })

  it("matches CurseForge identities by number", () => {
    const curseforge: AddonPageTarget = {
      projectId: "348521",
      platform: "curseforge",
      instanceId: 5
    }
    const installed = {
      ...mod("cloth-config.jar"),
      curseforgeProjectId: 348521,
      curseforgeFileId: 4973441
    }
    expect(
      explainNotDownloaded(curseforge, [], [installed], "4973441")
    ).toContain('"cloth-config.jar" is installed as CurseForge project 348521')
  })

  it("flags that an anonymous file may be older when the first list failed", () => {
    expect(
      explainNotDownloaded(target, undefined, [mod("cloth-config.jar")])
    ).toContain("may be older than this install")
  })
})

describe("updateTaskId", () => {
  it("reads the task id of an accepted update", () => {
    expect(updateTaskId('{"result":{"type":"response","data":73}}')).toBe(73)
  })

  it("has none for an rspc error, a non-number or an unreadable body", () => {
    expect(
      updateTaskId('{"result":{"type":"error","data":{"message":"no"}}}')
    ).toBeUndefined()
    expect(
      updateTaskId('{"result":{"type":"response","data":"73"}}')
    ).toBeUndefined()
    expect(updateTaskId("<no body: aborted>")).toBeUndefined()
  })
})
