import { describe, expect, it } from "vitest"
import {
  describeLinkedFile,
  describeLinkedFileInFull,
  effectiveFileId,
  instanceLinkTarget,
  linkedFileFitsInstance,
  serverLinkTarget,
  toLinkedFile
} from "./linkedFile"

const forge1201 = { mc_version: "1.20.1", modloader: "forge" }

describe("linkedFileFitsInstance", () => {
  it("fits on an exact version and loader match", () => {
    expect(
      linkedFileFitsInstance(["1.20.1", "Forge", "Client"], "mod", forge1201)
    ).toBe(true)
  })

  it("does not fit a file for a newer Minecraft version", () => {
    expect(linkedFileFitsInstance(["26.3", "Forge"], "mod", forge1201)).toBe(
      false
    )
  })

  it("does not fit a NeoForge file in a Forge instance", () => {
    expect(
      linkedFileFitsInstance(["1.20.1", "NeoForge"], "mod", forge1201)
    ).toBe(false)
  })

  it("fits a Fabric file in a Quilt instance", () => {
    expect(
      linkedFileFitsInstance(["1.21.1", "Fabric"], "mod", {
        mc_version: "1.21.1",
        modloader: "quilt"
      })
    ).toBe(true)
  })

  it("does not fit a Quilt-only file in a Fabric instance", () => {
    expect(
      linkedFileFitsInstance(["1.21.1", "Quilt"], "mod", {
        mc_version: "1.21.1",
        modloader: "fabric"
      })
    ).toBe(false)
  })

  it("fits a file with no loader tag in a Forge instance (legacy Forge)", () => {
    expect(
      linkedFileFitsInstance(["1.12.2"], "mod", {
        mc_version: "1.12.2",
        modloader: "forge"
      })
    ).toBe(true)
  })

  it("fits a multi-loader file that lists the instance's loader", () => {
    expect(
      linkedFileFitsInstance(["1.20.1", "Forge", "NeoForge"], "mod", forge1201)
    ).toBe(true)
  })

  it("fits a resource pack with a matching version and no loader tag", () => {
    expect(linkedFileFitsInstance(["1.20.1"], "resourcePack", forge1201)).toBe(
      true
    )
  })

  it("ignores loader tags for non-mod addons", () => {
    expect(
      linkedFileFitsInstance(["1.20.1", "Fabric"], "shader", forge1201)
    ).toBe(true)
  })

  it("does not fit a resource pack for another version", () => {
    expect(linkedFileFitsInstance(["1.21.4"], "resourcePack", forge1201)).toBe(
      false
    )
  })

  it("keeps the pin for an instance with a null mc_version", () => {
    expect(
      linkedFileFitsInstance(["26.3", "NeoForge"], "mod", {
        mc_version: null,
        modloader: null
      })
    ).toBe(true)
  })

  it("does not fit a file that lists no versions at all", () => {
    expect(linkedFileFitsInstance([], "mod", forge1201)).toBe(false)
  })
})

describe("effectiveFileId", () => {
  const jei = toLinkedFile("9207610", {
    displayName: "jei-26.3-neoforge-31.14.0.72.jar",
    gameVersions: ["26.3", "NeoForge"]
  })

  it("keeps an explicit pick that has no linked file", () => {
    expect(effectiveFileId(9207610, undefined, "mod", forge1201)).toBe(9207610)
  })

  it("drops a linked file that does not fit", () => {
    expect(effectiveFileId("9207610", jei, "mod", forge1201)).toBeUndefined()
  })

  it("keeps a linked file that fits", () => {
    expect(
      effectiveFileId("9207610", jei, "mod", {
        mc_version: "26.3",
        modloader: "neoforge"
      })
    ).toBe("9207610")
  })

  it("does not trust a linked file for an instance it cannot read", () => {
    expect(effectiveFileId("9207610", jei, "mod", undefined)).toBeUndefined()
  })

  it("keeps the pin for a known instance with no release", () => {
    expect(
      effectiveFileId("9207610", jei, "mod", {
        mc_version: null,
        modloader: null
      })
    ).toBe("9207610")
  })

  it("does not trust metadata of a different file", () => {
    expect(
      effectiveFileId("111", jei, "mod", {
        mc_version: "26.3",
        modloader: "neoforge"
      })
    ).toBeUndefined()
  })
})

describe("instanceLinkTarget", () => {
  const instances = [
    {
      id: 1,
      status: {
        status: "valid",
        value: { mc_version: "1.20.1", modloader: "forge" }
      }
    },
    { id: 2, status: { status: "invalid", value: "JsonMissing" } }
  ] as any

  it("reads a valid instance's release and loader", () => {
    expect(instanceLinkTarget(instances, 1)).toEqual(forge1201)
  })

  it("is unknown for invalid, missing or unselected instances", () => {
    expect(instanceLinkTarget(instances, 2)).toBeUndefined()
    expect(instanceLinkTarget(instances, 3)).toBeUndefined()
    expect(instanceLinkTarget(instances, undefined)).toBeUndefined()
    expect(instanceLinkTarget(undefined, 1)).toBeUndefined()
  })
})

describe("serverLinkTarget", () => {
  it("reads a server's release and loader, lowercased", () => {
    expect(
      serverLinkTarget({ gameVersion: "1.20.1", modloaderType: "Forge" })
    ).toEqual(forge1201)
  })

  it("has no loader for a vanilla server", () => {
    expect(
      serverLinkTarget({ gameVersion: "1.21.1", modloaderType: null })
    ).toEqual({ mc_version: "1.21.1", modloader: null })
  })

  it("is unknown while the server's details load", () => {
    expect(serverLinkTarget(undefined)).toBeUndefined()
    expect(serverLinkTarget(null)).toBeUndefined()
  })

  it("drops a NeoForge 26.3 link on a 1.20.1 Forge server", () => {
    const jei = toLinkedFile("9207610", {
      gameVersions: ["26.3", "NeoForge"]
    })
    expect(
      effectiveFileId(
        "9207610",
        jei,
        "mod",
        serverLinkTarget({ gameVersion: "1.20.1", modloaderType: "forge" })
      )
    ).toBeUndefined()
  })
})

describe("describeLinkedFile", () => {
  it("names the release and loader", () => {
    expect(describeLinkedFile(["26.3", "NeoForge", "Client", "Server"])).toBe(
      "26.3 · NeoForge"
    )
  })

  it("lists up to three releases, oldest first", () => {
    expect(describeLinkedFile(["1.20.2", "1.20.1", "Fabric", "Quilt"])).toBe(
      "1.20.1, 1.20.2 · Fabric/Quilt"
    )
  })

  it("shortens longer lists to the newest two releases and a count", () => {
    expect(
      describeLinkedFile(["1.20.4", "1.20", "1.20.2", "1.20.1", "Forge"])
    ).toBe("1.20.2, 1.20.4 +2 · Forge")
  })

  it("never shows a range that seems to include a release a list skips", () => {
    // 1.20.2 is not listed; "1.20.1–1.21.1" would seem to include it.
    const gapped = ["1.20.1", "1.20.4", "1.20.6", "1.21.1", "Client"]
    expect(describeLinkedFile(gapped)).toBe("1.20.6, 1.21.1 +2")
    expect(
      linkedFileFitsInstance(gapped, "resourcePack", {
        mc_version: "1.20.2",
        modloader: null
      })
    ).toBe(false)
  })

  it("uses loader display names for lowercase tags", () => {
    expect(describeLinkedFile(["1.20.1", "neoforge", "forge"])).toBe(
      "1.20.1 · NeoForge/Forge"
    )
  })

  it("is empty when nothing is known", () => {
    expect(describeLinkedFile(["Client"])).toBe("")
  })

  it("spells every release out in full for the tooltip", () => {
    expect(
      describeLinkedFileInFull(["1.20.4", "1.20", "1.20.2", "1.20.1", "Forge"])
    ).toBe("1.20, 1.20.1, 1.20.2, 1.20.4 · Forge")
  })
})
