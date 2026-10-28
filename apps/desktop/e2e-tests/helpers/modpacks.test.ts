import { describe, expect, it } from "vitest"
import { createHash } from "node:crypto"
import { deflateRawSync } from "node:zlib"
import { crc32 } from "node:zlib"
import {
  isPlatformProjectId,
  packPaths,
  parseMrpackIndex,
  type PackIndex
} from "./modpacks.js"
import { MODPACK_CF_PROJECT, MODPACK_MR_PROJECT } from "./modpackFixtures.js"

/** Matches `parseMrpackIndex`'s own hash choice for override content — see
 *  `PackOverrideFile`'s doc comment for why sha256. */
function sha256(body: string): string {
  return createHash("sha256").update(Buffer.from(body, "utf8")).digest("hex")
}

/** Builds a minimal, spec-conformant ZIP with every entry compressed by
 *  `method` — 0 (stored, where the "compressed" bytes are just the raw
 *  bytes and the two size fields are equal) or 8 (deflate, the default).
 *  Also accepts a method number outside that pair unchanged, so a test can
 *  build an archive `readZip` must reject. Enough for the parser under
 *  test; not a general-purpose zip writer. */
function buildZip(entries: Record<string, string>, method = 8): Buffer {
  const locals: Buffer[] = []
  const central: Buffer[] = []
  let offset = 0

  for (const [name, body] of Object.entries(entries)) {
    const raw = Buffer.from(body, "utf8")
    const compressed = method === 0 ? raw : deflateRawSync(raw)
    const nameBuf = Buffer.from(name, "utf8")
    const crc = crc32(raw)

    const local = Buffer.alloc(30)
    local.writeUInt32LE(0x04034b50, 0)
    local.writeUInt16LE(20, 4)
    local.writeUInt16LE(0, 6)
    local.writeUInt16LE(method, 8)
    local.writeUInt32LE(crc, 14)
    local.writeUInt32LE(compressed.length, 18)
    local.writeUInt32LE(raw.length, 22)
    local.writeUInt16LE(nameBuf.length, 26)
    locals.push(local, nameBuf, compressed)

    const cd = Buffer.alloc(46)
    cd.writeUInt32LE(0x02014b50, 0)
    cd.writeUInt16LE(20, 6)
    cd.writeUInt16LE(method, 10)
    cd.writeUInt32LE(crc, 16)
    cd.writeUInt32LE(compressed.length, 20)
    cd.writeUInt32LE(raw.length, 24)
    cd.writeUInt16LE(nameBuf.length, 28)
    cd.writeUInt32LE(offset, 42)
    central.push(cd, nameBuf)

    offset += local.length + nameBuf.length + compressed.length
  }

  const centralBuf = Buffer.concat(central)
  const eocd = Buffer.alloc(22)
  eocd.writeUInt32LE(0x06054b50, 0)
  eocd.writeUInt16LE(Object.keys(entries).length, 8)
  eocd.writeUInt16LE(Object.keys(entries).length, 10)
  eocd.writeUInt32LE(centralBuf.length, 12)
  eocd.writeUInt32LE(offset, 16)

  return Buffer.concat([...locals, centralBuf, eocd])
}

const INDEX = JSON.stringify({
  formatVersion: 1,
  name: "Test Pack",
  versionId: "1.0",
  dependencies: { minecraft: "1.20.1", "fabric-loader": "0.16.14" },
  files: [
    {
      path: "mods/a.jar",
      hashes: { sha512: "aa", sha1: "bb" },
      fileSize: 123
    },
    {
      path: "mods/b.jar",
      hashes: { sha512: "cc", sha1: "dd" },
      fileSize: 456
    }
  ]
})

describe("parseMrpackIndex", () => {
  const zip = buildZip({
    "modrinth.index.json": INDEX,
    "overrides/config/x.json": "{}",
    "overrides/config/nested/y.txt": "hi"
  })

  it("reads the declared files with their sha512 and size", () => {
    expect(parseMrpackIndex(zip).files).toEqual([
      { path: "mods/a.jar", sha512: "aa", size: 123 },
      { path: "mods/b.jar", sha512: "cc", size: 456 }
    ])
  })

  it("strips the overrides/ prefix from override paths", () => {
    expect(parseMrpackIndex(zip).overrides.sort()).toEqual([
      "config/nested/y.txt",
      "config/x.json"
    ])
  })

  it("pairs every override with a sha256 of its own raw bytes", () => {
    const overrideFiles = [...parseMrpackIndex(zip).overrideFiles].sort(
      (a, b) => a.path.localeCompare(b.path)
    )
    expect(overrideFiles).toEqual([
      { path: "config/nested/y.txt", sha256: sha256("hi") },
      { path: "config/x.json", sha256: sha256("{}") }
    ])
  })

  it("reads the minecraft version and the loader off dependencies", () => {
    const index = parseMrpackIndex(zip)
    expect(index.minecraft).toBe("1.20.1")
    expect(index.loader).toEqual({ type: "fabric", version: "0.16.14" })
  })

  it("round-trips a stored (uncompressed) entry the same as a deflated one", () => {
    const zip = buildZip(
      {
        "modrinth.index.json": INDEX,
        "overrides/config/x.json": "{}"
      },
      0
    )
    expect(parseMrpackIndex(zip)).toEqual({
      files: [
        { path: "mods/a.jar", sha512: "aa", size: 123 },
        { path: "mods/b.jar", sha512: "cc", size: 456 }
      ],
      overrides: ["config/x.json"],
      overrideFiles: [{ path: "config/x.json", sha256: sha256("{}") }],
      minecraft: "1.20.1",
      loader: { type: "fabric", version: "0.16.14" }
    })
  })

  it("throws when the archive holds no modrinth.index.json", () => {
    expect(() => parseMrpackIndex(buildZip({ "a.txt": "x" }))).toThrow(
      /modrinth\.index\.json/
    )
  })

  it("throws when the archive has no end-of-central-directory signature", () => {
    expect(() => parseMrpackIndex(Buffer.from("not a zip"))).toThrow(
      /no end-of-central-directory/
    )
  })

  it("throws when the central directory header signature is corrupted", () => {
    const zip = buildZip({ "a.txt": "x" })
    const corrupted = Buffer.from(zip)
    // No comment field, so the EOCD is exactly the last 22 bytes — read its
    // central-directory-offset field the same way readZip does, then
    // corrupt the signature it points at.
    const eocdOffset = corrupted.length - 22
    const cdOffset = corrupted.readUInt32LE(eocdOffset + 16)
    corrupted.writeUInt32LE(0xdeadbeef, cdOffset)
    expect(() => parseMrpackIndex(corrupted)).toThrow(
      /bad central directory header/
    )
  })

  it("throws when the local file header signature is corrupted", () => {
    const zip = buildZip({ "a.txt": "x" })
    const corrupted = Buffer.from(zip)
    // The archive's single entry is also its first, so its local header
    // sits at offset 0.
    corrupted.writeUInt32LE(0xdeadbeef, 0)
    expect(() => parseMrpackIndex(corrupted)).toThrow(/bad local file header/)
  })

  it("throws naming the method and file when compression is unsupported", () => {
    const zip = buildZip({ "mods/a.jar": "x" }, 12)
    expect(() => parseMrpackIndex(zip)).toThrow(
      /unsupported zip compression method 12 for mods\/a\.jar/
    )
  })
})

describe("packPaths", () => {
  it("unions declared files and overrides, sorted and deduplicated", () => {
    const index: PackIndex = {
      files: [{ path: "mods/a.jar", sha512: "aa", size: 1 }],
      overrides: ["config/x.json", "mods/a.jar"],
      overrideFiles: [
        { path: "config/x.json", sha256: sha256("{}") },
        { path: "mods/a.jar", sha256: sha256("dup") }
      ],
      minecraft: "1.20.1",
      loader: { type: "fabric", version: "0.16.14" }
    }
    expect(packPaths(index)).toEqual(["config/x.json", "mods/a.jar"])
  })
})

import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { verifyFreshPack } from "./freshPack.js"
import { snapshotTree } from "./instanceTree.js"
import { classifyPackinfo } from "./packinfo.js"
it.each([0, 8])(
  "fresh override verification rejects self-consistent wrong bytes from actual ZIP method %s",
  async (method) => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-fresh-override-"))
    try {
      const body = "archive pristine",
        wrong = "wrong but present"
      const index = parseMrpackIndex(
        buildZip(
          {
            "modrinth.index.json": JSON.stringify({
              dependencies: { minecraft: "1.20.1", "fabric-loader": "0.16.14" },
              files: []
            }),
            "overrides/config/a.txt": body
          },
          method
        )
      )
      fs.mkdirSync(path.join(root, "instance/config"), { recursive: true })
      fs.writeFileSync(path.join(root, "instance/config/a.txt"), wrong)
      fs.writeFileSync(
        path.join(root, "packinfo.json"),
        JSON.stringify({
          files: {
            "/config/a.txt": {
              md5: createHash("md5").update(wrong).digest("hex"),
              sha512: createHash("sha512").update(wrong).digest("hex")
            }
          }
        })
      )
      expect(await classifyPackinfo(root)).toEqual({
        pristine: ["/config/a.txt"],
        missing: [],
        modified: []
      })
      await expect(
        verifyFreshPack(
          root,
          index,
          await snapshotTree(path.join(root, "instance"))
        )
      ).rejects.toThrow(/override/i)
      fs.writeFileSync(path.join(root, "instance/config/a.txt"), body)
      await verifyFreshPack(
        root,
        index,
        await snapshotTree(path.join(root, "instance"))
      )
      fs.writeFileSync(
        path.join(root, "packinfo.json"),
        JSON.stringify({
          files: {
            "/config/a.txt": {
              md5: createHash("md5").update(body).digest("hex"),
              sha512: createHash("sha512").update(body).digest("hex")
            }
          }
        })
      )
      expect(await classifyPackinfo(root)).toEqual({
        pristine: ["/config/a.txt"],
        missing: [],
        modified: []
      })
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  }
)
it.each([false, true])(
  "client override wins common bytes with reverse ZIP order=%s",
  async (reverse) => {
    const entries: Record<string, string> = {
      "modrinth.index.json": JSON.stringify({
        dependencies: { minecraft: "1.20.1", "fabric-loader": "0.16.14" },
        files: []
      })
    }
    for (const layer of reverse
      ? ["client-overrides", "overrides"]
      : ["overrides", "client-overrides"])
      entries[`${layer}/config/a.txt`] = layer
    entries["server-overrides/server.txt"] = "server"
    const index = parseMrpackIndex(buildZip(entries))
    expect(index.overrideFiles).toEqual([
      { path: "config/a.txt", sha256: sha256("client-overrides") }
    ])
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-client-override-"))
    try {
      fs.mkdirSync(path.join(root, "instance/config"), { recursive: true })
      fs.writeFileSync(path.join(root, "instance/config/a.txt"), "overrides")
      await expect(
        verifyFreshPack(
          root,
          index,
          await snapshotTree(path.join(root, "instance"))
        )
      ).rejects.toThrow(/override/i)
      fs.writeFileSync(
        path.join(root, "instance/config/a.txt"),
        "client-overrides"
      )
      await verifyFreshPack(
        root,
        index,
        await snapshotTree(path.join(root, "instance"))
      )
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  }
)
it("required client descriptors exclude optional and unsupported client files", () => {
  const index = parseMrpackIndex(
    buildZip({
      "modrinth.index.json": JSON.stringify({
        dependencies: { minecraft: "1.20.1", "fabric-loader": "0.16.14" },
        files: [
          {
            path: "required",
            hashes: { sha512: "aa" },
            fileSize: 1,
            env: { client: "required" }
          },
          {
            path: "optional",
            hashes: { sha512: "bb" },
            fileSize: 1,
            env: { client: "optional" }
          },
          {
            path: "server",
            hashes: { sha512: "cc" },
            fileSize: 1,
            env: { client: "unsupported" }
          }
        ]
      })
    })
  )
  expect(index.files.map((f) => f.path)).toEqual(["required"])
})
it.each(["missing-override", "declared-size", "declared-hash"])(
  "fresh content verifier rejects %s",
  async (mode) => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-fresh-control-"))
    try {
      fs.mkdirSync(path.join(root, "instance"))
      fs.writeFileSync(path.join(root, "instance/a.txt"), "abc")
      const index: PackIndex = {
        minecraft: "1.20.1",
        loader: { type: "fabric", version: "0.16.14" },
        files:
          mode === "missing-override"
            ? []
            : [
                {
                  path: "a.txt",
                  size: mode === "declared-size" ? 4 : 3,
                  sha512:
                    mode === "declared-hash"
                      ? "wrong"
                      : createHash("sha512").update("abc").digest("hex")
                }
              ],
        overrides: mode === "missing-override" ? ["absent"] : [],
        overrideFiles:
          mode === "missing-override"
            ? [{ path: "absent", sha256: sha256("archive") }]
            : []
      }
      await expect(
        verifyFreshPack(
          root,
          index,
          await snapshotTree(path.join(root, "instance"))
        )
      ).rejects.toThrow()
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  }
)
it("fresh oracle explicitly rejects undeclared precedence for descriptor/override collisions", async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-fresh-collision-"))
  try {
    fs.mkdirSync(path.join(root, "instance"))
    fs.writeFileSync(path.join(root, "instance/a.txt"), "abc")
    const index: PackIndex = {
      minecraft: "1.20.1",
      loader: { type: "fabric", version: "0.16.14" },
      files: [
        {
          path: "a.txt",
          size: 3,
          sha512: createHash("sha512").update("abc").digest("hex")
        }
      ],
      overrides: ["a.txt"],
      overrideFiles: [{ path: "a.txt", sha256: sha256("abc") }]
    }
    await expect(
      verifyFreshPack(
        root,
        index,
        await snapshotTree(path.join(root, "instance"))
      )
    ).rejects.toThrow(/unsupported.*collision/i)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
})

describe("isPlatformProjectId", () => {
  it("tells this suite's fixture projects apart by id space", () => {
    expect(isPlatformProjectId(MODPACK_CF_PROJECT, "curseforge")).toBe(true)
    expect(isPlatformProjectId(MODPACK_CF_PROJECT, "modrinth")).toBe(false)
    expect(isPlatformProjectId(MODPACK_MR_PROJECT, "modrinth")).toBe(true)
    expect(isPlatformProjectId(MODPACK_MR_PROJECT, "curseforge")).toBe(false)
  })

  it("rejects the stale CurseForge row the macOS run clicked under a Modrinth filter", () => {
    expect(isPlatformProjectId("1253222", "modrinth")).toBe(false)
  })

  it("rejects an empty id for either platform", () => {
    expect(isPlatformProjectId("", "modrinth")).toBe(false)
    expect(isPlatformProjectId("", "curseforge")).toBe(false)
  })
})
