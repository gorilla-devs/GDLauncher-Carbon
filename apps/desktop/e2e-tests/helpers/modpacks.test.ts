import { describe, expect, it } from "vitest"
import { deflateRawSync } from "node:zlib"
import { crc32 } from "node:zlib"
import { packPaths, parseMrpackIndex, type PackIndex } from "./modpacks.js"

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
      minecraft: "1.20.1",
      loader: { type: "fabric", version: "0.16.14" }
    }
    expect(packPaths(index)).toEqual(["config/x.json", "mods/a.jar"])
  })
})
