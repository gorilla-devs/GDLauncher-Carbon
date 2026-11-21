import { describe, expect, it } from "vitest"
import { sameAddonSupport, scopeMode, targetDefaults } from "./targetScope"

const scope = (modLoaderType?: string, gameVersion?: string) => ({
  modLoaderType,
  gameVersion
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
      expect(targetDefaults(kind, "loader", quilt, {}, "mod")).toEqual(
        scope("quilt", "1.20.1")
      )
      expect(
        targetDefaults(
          kind,
          "loader",
          quilt,
          { loaders: ["quilt", "fabric"], gameVersions: ["1.20.1"] },
          "mod"
        )
      ).toEqual(scope("quilt", "1.20.1"))
      expect(
        targetDefaults(
          kind,
          "loader",
          forge,
          { loaders: [], gameVersions: [] },
          "mod"
        )
      ).toEqual(scope("forge", "1.20.1"))
    }
  })

  it("lists Fabric builds on Quilt and Forge builds on NeoForge 1.20.1 when the addon has no own-loader builds", () => {
    for (const kind of ["instance", "server"] as const) {
      // Fabric API and Cloth Config on a Quilt target.
      expect(
        targetDefaults(kind, "loader", quilt, { loaders: ["fabric"] }, "mod")
      ).toEqual(scope("fabric", "1.20.1"))
      expect(
        targetDefaults(
          kind,
          "loader",
          quilt,
          { loaders: ["fabric", "forge", "neoforge"] },
          "mod"
        )
      ).toEqual(scope("fabric", "1.20.1"))
      expect(
        targetDefaults(kind, "loader", neoforge, { loaders: ["forge"] }, "mod")
      ).toEqual(scope("forge", "1.20.1"))
    }
  })

  it("drops on an instance what the addon declares nothing for, as 2.0.40 did", () => {
    expect(
      targetDefaults(
        "instance",
        "loader",
        forge,
        { loaders: ["fabric"] },
        "mod"
      )
    ).toEqual(scope(undefined, "1.20.1"))
    expect(
      targetDefaults(
        "instance",
        "loader",
        { loader: "neoforge", gameVersion: "1.21.1" },
        { loaders: ["forge"] },
        "mod"
      )
    ).toEqual(scope(undefined, "1.21.1"))
    // A snapshot instance, or a pack tagged "1.20" but not "1.20.1".
    expect(
      targetDefaults(
        "instance",
        "loader",
        { loader: "fabric", gameVersion: "24w14a" },
        { loaders: ["fabric"], gameVersions: ["1.20.4", "1.20.6"] },
        "mod"
      )
    ).toEqual(scope("fabric", undefined))
    expect(
      targetDefaults(
        "instance",
        "gameOnly",
        { loader: "fabric", gameVersion: "1.20.1" },
        { gameVersions: ["1.20"] },
        "mod"
      )
    ).toEqual(scope(undefined, undefined))
  })

  it("keeps a server's own loader and version even when the addon declares nothing for them", () => {
    expect(
      targetDefaults(
        "server",
        "loader",
        forge,
        { loaders: ["fabric"], gameVersions: ["1.21"] },
        "mod"
      )
    ).toEqual(scope("forge", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "gameOnly",
        { loader: "fabric", gameVersion: "1.20.1" },
        { gameVersions: ["1.20"] },
        "mod"
      )
    ).toEqual(scope(undefined, "1.20.1"))
  })

  it("lists a server's datapack versions when a datapack declares none of the server's loaders", () => {
    // Modrinth types a datapack project "mod" and tags its zip versions
    // with the "datapack" loader, which any server runs.
    const fabric = { loader: "fabric", gameVersion: "1.20.1" }
    expect(
      targetDefaults(
        "server",
        "loader",
        fabric,
        { loaders: ["datapack"] },
        "mod"
      )
    ).toEqual(scope("datapack", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "loader",
        fabric,
        { loaders: ["datapack", "forge"] },
        "mod"
      )
    ).toEqual(scope("datapack", "1.20.1"))
    const forge121 = { loader: "forge", gameVersion: "1.21.1" }
    expect(
      targetDefaults(
        "server",
        "loader",
        forge121,
        { loaders: ["datapack", "minecraft"] },
        "mod"
      )
    ).toEqual(scope("datapack", "1.21.1"))
    expect(
      targetDefaults(
        "server",
        "loader",
        forge121,
        { loaders: ["datapack", "fabric", "neoforge"] },
        "mod"
      )
    ).toEqual(scope("datapack", "1.21.1"))
  })

  it("keeps the server's or a compatible loader when a datapack also declares it, and leaves vanilla servers alone", () => {
    expect(
      targetDefaults(
        "server",
        "loader",
        { loader: "fabric", gameVersion: "1.20.1" },
        { loaders: ["datapack", "fabric"] },
        "mod"
      )
    ).toEqual(scope("fabric", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "loader",
        quilt,
        { loaders: ["datapack", "fabric"] },
        "mod"
      )
    ).toEqual(scope("fabric", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "loader",
        neoforge,
        { loaders: ["datapack", "forge"] },
        "mod"
      )
    ).toEqual(scope("forge", "1.20.1"))
    expect(
      targetDefaults(
        "server",
        "loader",
        { loader: null, gameVersion: "1.20.1" },
        { loaders: ["datapack"] },
        "mod"
      )
    ).toEqual(scope(undefined, "1.20.1"))
  })

  describe("for an instance that gets the project's data pack build", () => {
    const fabric = { loader: "fabric", gameVersion: "1.20.1" }
    const vanilla = { gameVersion: "1.20.1" }
    const onlyDatapack = { loaders: ["datapack"], gameVersions: ["1.20.1"] }
    const hybrid = {
      loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"],
      gameVersions: ["1.20.1"]
    }
    // A resource pack and a plugin that are also data packs.
    const combined = [
      ["datapack", "minecraft"],
      ["datapack", "bukkit", "paper", "purpur", "spigot"]
    ]
    const SEARCHES = ["mod", "resourcePack", "shader", null, undefined]

    it("lists the datapack builds of a project that is only a data pack, whatever the instance runs and whatever is searched", () => {
      for (const facts of [fabric, vanilla])
        for (const searched of ["datapack", ...SEARCHES])
          expect(
            targetDefaults("instance", "loader", facts, onlyDatapack, searched)
          ).toEqual(scope("datapack", "1.20.1"))
    })
    it("in a data pack search, lists the datapack builds of a project that also ships as a mod where the instance runs none of its loaders", () => {
      expect(
        targetDefaults("instance", "loader", vanilla, hybrid, "datapack")
      ).toEqual(scope("datapack", "1.20.1"))
      expect(
        targetDefaults("instance", "loader", fabric, hybrid, "datapack")
      ).toEqual(scope("fabric", "1.20.1"))
      expect(
        targetDefaults(
          "instance",
          "loader",
          fabric,
          { loaders: ["datapack", "forge"] },
          "datapack"
        )
      ).toEqual(scope("datapack", "1.20.1"))
      // Its mod where the instance runs one through another (Quilt).
      expect(
        targetDefaults(
          "instance",
          "loader",
          { loader: "quilt", gameVersion: "1.20.1" },
          { loaders: ["datapack", "fabric"] },
          "datapack"
        )
      ).toEqual(scope("fabric", "1.20.1"))
    })
    it("in a data pack search, lists the datapack builds of a resource pack or a plugin that is also a data pack, whatever the instance runs", () => {
      for (const loaders of combined)
        for (const facts of [fabric, vanilla])
          expect(
            targetDefaults("instance", "loader", facts, { loaders }, "datapack")
          ).toEqual(scope("datapack", "1.20.1"))
    })
    it("outside a data pack search, lists a project that lists more than the datapack loader as it lists a mod", () => {
      for (const searched of SEARCHES) {
        // No loader of the instance to filter by: every build for its game.
        for (const loaders of [...combined, hybrid.loaders])
          expect(
            targetDefaults("instance", "loader", vanilla, { loaders }, searched)
          ).toEqual(scope(undefined, "1.20.1"))
        // A loader the project declares nothing for is dropped.
        for (const loaders of [...combined, ["datapack", "forge"]])
          expect(
            targetDefaults("instance", "loader", fabric, { loaders }, searched)
          ).toEqual(scope(undefined, "1.20.1"))
        expect(
          targetDefaults("instance", "loader", fabric, hybrid, searched)
        ).toEqual(scope("fabric", "1.20.1"))
      }
    })
    it("scopes a plain mod and a CurseForge data pack as before", () => {
      for (const searched of ["datapack", ...SEARCHES]) {
        expect(
          targetDefaults(
            "instance",
            "loader",
            vanilla,
            { loaders: ["fabric"] },
            searched
          )
        ).toEqual(scope(undefined, "1.20.1"))
        expect(
          targetDefaults(
            "instance",
            "gameOnly",
            fabric,
            { loaders: [] },
            searched
          )
        ).toEqual(scope(undefined, "1.20.1"))
      }
    })
  })

  it("lists only the datapack builds for a world when the project tags them, whatever the instance runs", () => {
    const fabric = { loader: "fabric", gameVersion: "1.20.1" }
    const hybrid = {
      loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"],
      gameVersions: ["1.20.1"]
    }
    expect(
      targetDefaults("instance", "loader", fabric, hybrid, "datapack", true)
    ).toEqual(scope("datapack", "1.20.1"))
    expect(
      targetDefaults(
        "instance",
        "loader",
        { gameVersion: "1.20.1" },
        hybrid,
        "datapack",
        true
      )
    ).toEqual(scope("datapack", "1.20.1"))
    // A world takes data packs only, whatever is searched.
    for (const searched of ["mod", "resourcePack", undefined])
      expect(
        targetDefaults(
          "instance",
          "loader",
          fabric,
          { loaders: ["datapack", "minecraft"] },
          searched,
          true
        )
      ).toEqual(scope("datapack", "1.20.1"))
    // Without a world the instance's own loader, as before.
    expect(targetDefaults("instance", "loader", fabric, hybrid, "mod")).toEqual(
      scope("fabric", "1.20.1")
    )
    // CurseForge data packs carry no loader tags: the game version only.
    expect(
      targetDefaults(
        "instance",
        "gameOnly",
        fabric,
        { gameVersions: ["1.20.1"] },
        "datapack",
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
        { gameVersions: ["1.20.1"] },
        "mod"
      )
    ).toEqual(scope("forge", undefined))
    expect(
      targetDefaults(
        "server",
        "loader",
        { loader: null, gameVersion: "" },
        { loaders: ["fabric"] },
        "mod"
      )
    ).toEqual(scope(undefined, undefined))
  })

  it("never scopes modpacks or pages without a target", () => {
    expect(targetDefaults("server", "none", forge, {}, "mod")).toEqual(
      scope(undefined, undefined)
    )
    expect(targetDefaults("instance", "none", forge, {}, "mod")).toEqual(
      scope(undefined, undefined)
    )
    expect(targetDefaults("none", "loader", forge, {}, "mod")).toEqual(
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
