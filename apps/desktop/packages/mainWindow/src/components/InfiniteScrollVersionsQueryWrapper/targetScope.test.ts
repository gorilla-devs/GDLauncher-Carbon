import { describe, expect, it } from "vitest"
import {
  compatibleLoaders,
  sameAddonSupport,
  scopeMode,
  targetDefaults
} from "./targetScope"

const scope = (modLoaderType?: string, gameVersion?: string) => ({
  modLoaderType,
  gameVersion
})

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

describe("scopeMode", () => {
  it("scopes mods by loader, loaderless types by game version and modpacks not at all", () => {
    expect(scopeMode(undefined)).toBe("loader")
    expect(scopeMode("mod")).toBe("loader")
    expect(scopeMode("plugin")).toBe("loader")
    expect(scopeMode("resourcePack")).toBe("gameOnly")
    expect(scopeMode("shader")).toBe("gameOnly")
    expect(scopeMode("world")).toBe("gameOnly")
    expect(scopeMode("datapack")).toBe("gameOnly")
    expect(scopeMode("modpack")).toBe("none")
  })
})

describe("targetDefaults", () => {
  const quilt = { loader: "quilt", gameVersion: "1.20.1" }
  const forge = { loader: "forge", gameVersion: "1.20.1" }
  const neoforge = { loader: "neoforge", gameVersion: "1.20.1" }

  it("keeps the target's own loader and version when the addon declares them or nothing is known", () => {
    for (const kind of ["instance", "server"] as const) {
      expect(targetDefaults(kind, "loader", quilt, {})).toEqual(
        scope("quilt", "1.20.1")
      )
      expect(
        targetDefaults(kind, "loader", quilt, {
          loaders: ["quilt", "fabric"],
          gameVersions: ["1.20.1"]
        })
      ).toEqual(scope("quilt", "1.20.1"))
      expect(
        targetDefaults(kind, "loader", forge, { loaders: [], gameVersions: [] })
      ).toEqual(scope("forge", "1.20.1"))
    }
  })

  it("lists Fabric builds on Quilt and Forge builds on NeoForge 1.20.1 when the addon has no own-loader builds", () => {
    for (const kind of ["instance", "server"] as const) {
      // Fabric API and Cloth Config on a Quilt target.
      expect(
        targetDefaults(kind, "loader", quilt, { loaders: ["fabric"] })
      ).toEqual(scope("fabric", "1.20.1"))
      expect(
        targetDefaults(kind, "loader", quilt, {
          loaders: ["fabric", "forge", "neoforge"]
        })
      ).toEqual(scope("fabric", "1.20.1"))
      expect(
        targetDefaults(kind, "loader", neoforge, { loaders: ["forge"] })
      ).toEqual(scope("forge", "1.20.1"))
    }
  })

  it("drops on an instance what the addon declares nothing for, as 2.0.40 did", () => {
    expect(
      targetDefaults("instance", "loader", forge, { loaders: ["fabric"] })
    ).toEqual(scope(undefined, "1.20.1"))
    expect(
      targetDefaults(
        "instance",
        "loader",
        { loader: "neoforge", gameVersion: "1.21.1" },
        { loaders: ["forge"] }
      )
    ).toEqual(scope(undefined, "1.21.1"))
    // A snapshot instance, or a pack tagged "1.20" but not "1.20.1".
    expect(
      targetDefaults(
        "instance",
        "loader",
        { loader: "fabric", gameVersion: "24w14a" },
        { loaders: ["fabric"], gameVersions: ["1.20.4", "1.20.6"] }
      )
    ).toEqual(scope("fabric", undefined))
    expect(
      targetDefaults(
        "instance",
        "gameOnly",
        { loader: "fabric", gameVersion: "1.20.1" },
        { gameVersions: ["1.20"] }
      )
    ).toEqual(scope(undefined, undefined))
  })

  it("keeps a server's own loader and version even when the addon declares nothing for them", () => {
    expect(
      targetDefaults("server", "loader", forge, {
        loaders: ["fabric"],
        gameVersions: ["1.21"]
      })
    ).toEqual(scope("forge", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "gameOnly",
        { loader: "fabric", gameVersion: "1.20.1" },
        { gameVersions: ["1.20"] }
      )
    ).toEqual(scope(undefined, "1.20.1"))
  })

  it("lists a server's datapack versions when a datapack declares none of the server's loaders", () => {
    // Modrinth types a datapack project "mod" and tags its zip versions
    // with the "datapack" loader, which any server runs.
    const fabric = { loader: "fabric", gameVersion: "1.20.1" }
    expect(
      targetDefaults("server", "loader", fabric, { loaders: ["datapack"] })
    ).toEqual(scope("datapack", "1.20.1"))
    expect(
      targetDefaults("server", "loader", fabric, {
        loaders: ["datapack", "forge"]
      })
    ).toEqual(scope("datapack", "1.20.1"))
    const forge121 = { loader: "forge", gameVersion: "1.21.1" }
    expect(
      targetDefaults("server", "loader", forge121, {
        loaders: ["datapack", "minecraft"]
      })
    ).toEqual(scope("datapack", "1.21.1"))
    expect(
      targetDefaults("server", "loader", forge121, {
        loaders: ["datapack", "fabric", "neoforge"]
      })
    ).toEqual(scope("datapack", "1.21.1"))
  })

  it("keeps the server's or a compatible loader when a datapack also declares it, and leaves vanilla servers alone", () => {
    expect(
      targetDefaults(
        "server",
        "loader",
        { loader: "fabric", gameVersion: "1.20.1" },
        { loaders: ["datapack", "fabric"] }
      )
    ).toEqual(scope("fabric", "1.20.1"))
    expect(
      targetDefaults("server", "loader", quilt, {
        loaders: ["datapack", "fabric"]
      })
    ).toEqual(scope("fabric", "1.20.1"))
    expect(
      targetDefaults("server", "loader", neoforge, {
        loaders: ["datapack", "forge"]
      })
    ).toEqual(scope("forge", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "loader",
        { loader: null, gameVersion: "1.20.1" },
        { loaders: ["datapack"] }
      )
    ).toEqual(scope(undefined, "1.20.1"))
  })

  it("lists the datapack builds for an instance that gets the project's data pack build", () => {
    const fabric = { loader: "fabric", gameVersion: "1.20.1" }
    const vanilla = { gameVersion: "1.20.1" }
    const onlyDatapack = { loaders: ["datapack"], gameVersions: ["1.20.1"] }
    const hybrid = {
      loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"],
      gameVersions: ["1.20.1"]
    }
    // A Modrinth project that is only a data pack, whatever the instance runs.
    for (const facts of [fabric, vanilla])
      expect(targetDefaults("instance", "loader", facts, onlyDatapack)).toEqual(
        scope("datapack", "1.20.1")
      )
    // One that also ships as a mod: its data pack build without a mod loader,
    // its mod with one.
    expect(targetDefaults("instance", "loader", vanilla, hybrid)).toEqual(
      scope("datapack", "1.20.1")
    )
    expect(targetDefaults("instance", "loader", fabric, hybrid)).toEqual(
      scope("fabric", "1.20.1")
    )
    // A plain mod and a CurseForge data pack are scoped as before.
    expect(
      targetDefaults("instance", "loader", vanilla, { loaders: ["fabric"] })
    ).toEqual(scope(undefined, "1.20.1"))
    expect(
      targetDefaults("instance", "gameOnly", fabric, { loaders: [] })
    ).toEqual(scope(undefined, "1.20.1"))
  })

  it("lists only the datapack builds for a world when the project tags them, whatever the instance runs", () => {
    const fabric = { loader: "fabric", gameVersion: "1.20.1" }
    const hybrid = {
      loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"],
      gameVersions: ["1.20.1"]
    }
    expect(targetDefaults("instance", "loader", fabric, hybrid, true)).toEqual(
      scope("datapack", "1.20.1")
    )
    expect(
      targetDefaults(
        "instance",
        "loader",
        { gameVersion: "1.20.1" },
        hybrid,
        true
      )
    ).toEqual(scope("datapack", "1.20.1"))
    // Without a world the instance's own loader, as before.
    expect(targetDefaults("instance", "loader", fabric, hybrid)).toEqual(
      scope("fabric", "1.20.1")
    )
    // CurseForge data packs carry no loader tags: the game version only.
    expect(
      targetDefaults(
        "instance",
        "gameOnly",
        fabric,
        { gameVersions: ["1.20.1"] },
        true
      )
    ).toEqual(scope(undefined, "1.20.1"))
  })
  it("filters nothing for values the target does not know", () => {
    // A modpack instance whose first install did not finish has no version.
    expect(
      targetDefaults(
        "instance",
        "loader",
        { loader: "forge", gameVersion: null },
        { gameVersions: ["1.20.1"] }
      )
    ).toEqual(scope("forge", undefined))
    expect(
      targetDefaults(
        "server",
        "loader",
        { loader: null, gameVersion: "" },
        { loaders: ["fabric"] }
      )
    ).toEqual(scope(undefined, undefined))
  })

  it("never scopes modpacks or pages without a target", () => {
    expect(targetDefaults("server", "none", forge, {})).toEqual(
      scope(undefined, undefined)
    )
    expect(targetDefaults("instance", "none", forge, {})).toEqual(
      scope(undefined, undefined)
    )
    expect(targetDefaults("none", "loader", forge, {})).toEqual(
      scope(undefined, undefined)
    )
  })
})

describe("sameAddonSupport", () => {
  it("treats refetched equal lists and empty or missing lists as unchanged", () => {
    expect(
      sameAddonSupport(
        { loaders: ["fabric"], gameVersions: ["1.20.1"] },
        { loaders: ["fabric"], gameVersions: ["1.20.1"] }
      )
    ).toBe(true)
    expect(sameAddonSupport({}, { loaders: [], gameVersions: [] })).toBe(true)
    expect(sameAddonSupport({}, { loaders: ["fabric"] })).toBe(false)
    expect(
      sameAddonSupport(
        { loaders: ["fabric"] },
        { loaders: ["fabric", "quilt"] }
      )
    ).toBe(false)
  })
})
