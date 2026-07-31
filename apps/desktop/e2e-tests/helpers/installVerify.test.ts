import { createHash } from "node:crypto"
import fs from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import {
  sampleKeys,
  sha1OfFile,
  verifyAssetIndex,
  verifyClientJar,
  verifyLibrariesAbsent,
  verifyLibrariesPresent
} from "./installVerify.js"

function sha1(data: Buffer | string): string {
  return createHash("sha1").update(data).digest("hex")
}

/** Builds `libraries/net/minecraft/client/<id>/<id>.jar` with `content`. */
async function writeClientJar(
  runtimePath: string,
  versionId: string,
  content: Buffer
): Promise<string> {
  const dir = path.join(
    runtimePath,
    "libraries",
    "net",
    "minecraft",
    "client",
    versionId
  )
  await fs.mkdir(dir, { recursive: true })
  const jarPath = path.join(dir, `${versionId}.jar`)
  await fs.writeFile(jarPath, content)
  return jarPath
}

async function writeAssetIndex(
  runtimePath: string,
  indexId: string,
  index: unknown
): Promise<void> {
  const dir = path.join(runtimePath, "assets", "indexes")
  await fs.mkdir(dir, { recursive: true })
  await fs.writeFile(path.join(dir, `${indexId}.json`), JSON.stringify(index))
}

/** Writes arbitrary (possibly non-JSON) bytes as an asset index file. */
async function writeRawAssetIndex(
  runtimePath: string,
  indexId: string,
  raw: string
): Promise<void> {
  const dir = path.join(runtimePath, "assets", "indexes")
  await fs.mkdir(dir, { recursive: true })
  await fs.writeFile(path.join(dir, `${indexId}.json`), raw)
}

/** Writes a modern content-addressed asset object under assets/objects. */
async function writeAssetObject(
  runtimePath: string,
  hash: string,
  content: Buffer
): Promise<void> {
  const dir = path.join(runtimePath, "assets", "objects", hash.slice(0, 2))
  await fs.mkdir(dir, { recursive: true })
  await fs.writeFile(path.join(dir, hash), content)
}

/** Writes a legacy/virtual asset under assets/virtual/<indexId>/<name>. */
async function writeVirtualAsset(
  runtimePath: string,
  indexId: string,
  name: string,
  content: Buffer
): Promise<void> {
  const target = path.join(runtimePath, "assets", "virtual", indexId, name)
  await fs.mkdir(path.dirname(target), { recursive: true })
  await fs.writeFile(target, content)
}

async function writeLibrary(
  runtimePath: string,
  relativePath: string,
  content: Buffer
): Promise<void> {
  const target = path.join(runtimePath, "libraries", relativePath)
  await fs.mkdir(path.dirname(target), { recursive: true })
  await fs.writeFile(target, content)
}

let runtimePath: string

beforeEach(async () => {
  runtimePath = await fs.mkdtemp(
    path.join(os.tmpdir(), "gdl-e2e-installverify-")
  )
})

afterEach(async () => {
  await fs.rm(runtimePath, { recursive: true, force: true })
})

describe("sha1OfFile", () => {
  it("hashes file contents to a known sha1", async () => {
    const filePath = path.join(runtimePath, "hello.txt")
    await fs.writeFile(filePath, "hello")
    // Known test vector: sha1("hello") == aaf4c61d...
    await expect(sha1OfFile(filePath)).resolves.toBe(
      "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
    )
  })
})

describe("verifyClientJar", () => {
  it("verifies clean when the jar exists and matches expectedSha1", async () => {
    const content = Buffer.from("fake client jar bytes")
    await writeClientJar(runtimePath, "1.20.1", content)

    const result = await verifyClientJar(runtimePath, "1.20.1", sha1(content))
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("verifies clean with no expectedSha1 as long as the jar exists", async () => {
    await writeClientJar(runtimePath, "1.20.1", Buffer.from("anything"))
    const result = await verifyClientJar(runtimePath, "1.20.1")
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports a missing client jar", async () => {
    const result = await verifyClientJar(runtimePath, "1.20.1", "deadbeef")
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(
      path.join(
        runtimePath,
        "libraries",
        "net",
        "minecraft",
        "client",
        "1.20.1",
        "1.20.1.jar"
      )
    )
  })

  it("reports a sha1 mismatch naming both the expected and actual hash", async () => {
    const content = Buffer.from("original client jar bytes")
    const jarPath = await writeClientJar(runtimePath, "1.20.1", content)
    const expectedSha1 = sha1(content)

    // Flip a single byte to corrupt the jar without changing its size —
    // this is the "strongest" corruption case: a bit-for-bit-almost-correct
    // file that must still be caught.
    const corrupted = Buffer.from(content)
    corrupted[0] ^= 0xff
    await fs.writeFile(jarPath, corrupted)
    const actualSha1 = sha1(corrupted)

    const result = await verifyClientJar(runtimePath, "1.20.1", expectedSha1)

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(expectedSha1)
    expect(result.problems[0]).toContain(actualSha1)
    expect(expectedSha1).not.toBe(actualSha1)
  })
})

describe("verifyAssetIndex — modern layout", () => {
  async function buildModernTree(indexId: string, count: number) {
    const entries: Record<
      string,
      { name: string; content: Buffer; hash: string }
    > = {}
    const objects: Record<string, { hash: string; size: number }> = {}

    for (let i = 0; i < count; i++) {
      const name = `sounds/asset-${String(i).padStart(4, "0")}.ogg`
      const content = Buffer.from(`content-${indexId}-${i}`)
      const hash = sha1(content)
      entries[name] = { name, content, hash }
      objects[name] = { hash, size: content.length }
      await writeAssetObject(runtimePath, hash, content)
    }

    await writeAssetIndex(runtimePath, indexId, { objects })
    return entries
  }

  it("verifies a well-formed modern tree clean", async () => {
    await buildModernTree("modern-index", 5)
    const result = await verifyAssetIndex(runtimePath, "modern-index")
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports a missing asset index file", async () => {
    const result = await verifyAssetIndex(runtimePath, "does-not-exist")
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(
      path.join(runtimePath, "assets", "indexes", "does-not-exist.json")
    )
  })

  it("reports invalid JSON in the index file as a problem, not a thrown exception", async () => {
    await writeRawAssetIndex(
      runtimePath,
      "truncated-index",
      '{"objects": {"a.png": {"hash": "abc'
    )

    // A parse error must surface as a VerifyResult problem, consistent with
    // the rest of the module's never-throw contract — not propagate as a
    // raw exception that would abort an entire matrix run instead of
    // reporting a clean per-version problem list.
    await expect(
      verifyAssetIndex(runtimePath, "truncated-index")
    ).resolves.toMatchObject({ ok: false })
    const result = await verifyAssetIndex(runtimePath, "truncated-index")
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("not valid JSON")
  })

  it('reports a missing "objects" key as distinct from an empty map', async () => {
    // A partial write cut off before `objects` was ever serialized — the
    // single most likely shape of asset-index corruption in practice — must
    // not fall through a `?? {}` into trivially verifying clean because
    // "every referenced object exists" is vacuously true over zero objects.
    await writeAssetIndex(runtimePath, "no-objects-key-index", {
      virtual: false
    })

    const result = await verifyAssetIndex(runtimePath, "no-objects-key-index")
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain('no "objects" key')
  })

  it('reports an empty "objects" map as distinct from a missing key', async () => {
    await writeAssetIndex(runtimePath, "empty-objects-index", { objects: {} })

    const result = await verifyAssetIndex(runtimePath, "empty-objects-index")
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("empty")
  })

  it("gives the missing-key and empty-map problems different wording", async () => {
    await writeAssetIndex(runtimePath, "no-key", { virtual: false })
    await writeAssetIndex(runtimePath, "empty-map", { objects: {} })

    const [missingKeyResult, emptyMapResult] = await Promise.all([
      verifyAssetIndex(runtimePath, "no-key"),
      verifyAssetIndex(runtimePath, "empty-map")
    ])

    expect(missingKeyResult.problems[0]).not.toBe(emptyMapResult.problems[0])
  })

  it("reports a null or hash-less object entry as a problem, not a thrown exception", async () => {
    // A per-entry counterpart to the "invalid JSON"/"no objects key" tests
    // above: the file parses and `objects` is a real map, but one entry
    // inside it is corrupt. `entry.hash.slice(0, 2)` on either shape below
    // would throw a TypeError out of `mapConcurrent` without the guard this
    // pins, aborting the whole call instead of reporting a clean per-object
    // problem list — the same never-throw contract, one level deeper.
    const content = Buffer.from("valid asset bytes")
    const hash = sha1(content)
    await writeAssetObject(runtimePath, hash, content)

    await writeAssetIndex(runtimePath, "malformed-entries-index", {
      objects: {
        "valid.ogg": { hash, size: content.length },
        "null-entry.ogg": null,
        "no-hash.ogg": { size: 4 }
      }
    })

    await expect(
      verifyAssetIndex(runtimePath, "malformed-entries-index")
    ).resolves.toMatchObject({ ok: false })

    const result = await verifyAssetIndex(
      runtimePath,
      "malformed-entries-index"
    )
    expect(result.problems.some((p) => p.includes("null-entry.ogg"))).toBe(true)
    expect(result.problems.some((p) => p.includes("no-hash.ogg"))).toBe(true)
    // The one well-formed entry alongside the malformed ones is still
    // checked normally and does not itself become a problem.
    expect(result.problems.some((p) => p.includes("valid.ogg"))).toBe(false)
  })

  it("reports an index object absent from disk, naming the object", async () => {
    const entries = await buildModernTree("missing-object-index", 3)
    const missingName = Object.keys(entries)[0]
    const missingHash = entries[missingName].hash
    const objectPath = path.join(
      runtimePath,
      "assets",
      "objects",
      missingHash.slice(0, 2),
      missingHash
    )
    await fs.rm(objectPath)

    const result = await verifyAssetIndex(runtimePath, "missing-object-index")
    expect(result.ok).toBe(false)
    expect(result.problems.some((p) => p.includes(missingName))).toBe(true)
    expect(result.problems.some((p) => p.includes(missingHash))).toBe(true)
  })

  it("reports a sampled object whose content does not match its content-addressed name", async () => {
    // Small tree (below the sample-size threshold) so every object is
    // guaranteed to be part of the sample, keeping this deterministic.
    const entries = await buildModernTree("corrupt-object-index", 4)
    const corruptedName = Object.keys(entries)[0]
    const corruptedHash = entries[corruptedName].hash
    const objectPath = path.join(
      runtimePath,
      "assets",
      "objects",
      corruptedHash.slice(0, 2),
      corruptedHash
    )
    // Overwrite the object's bytes in place, so the path is still named
    // after the original (now wrong) hash — a genuine content-address
    // mismatch, not a missing file.
    await fs.writeFile(objectPath, Buffer.from("tampered bytes"))

    const result = await verifyAssetIndex(runtimePath, "corrupt-object-index")
    expect(result.ok).toBe(false)
    expect(result.problems.some((p) => p.includes(corruptedName))).toBe(true)
    expect(result.problems.some((p) => p.includes(corruptedHash))).toBe(true)
  })

  it("samples deterministically: two runs over the same tree flag the same problems", async () => {
    // A tree well past the hash-sample size, with a single corrupted object
    // whose name sorts into the middle of the key space. If sampling used
    // wall-clock time or an unseeded PRNG instead of a pure function of the
    // sorted key set, the two runs below would disagree on whether the
    // corrupted object was even sampled.
    const entries = await buildModernTree("determinism-index", 60)
    const corruptedName = Object.keys(entries)[30]
    const corruptedHash = entries[corruptedName].hash
    const objectPath = path.join(
      runtimePath,
      "assets",
      "objects",
      corruptedHash.slice(0, 2),
      corruptedHash
    )
    await fs.writeFile(objectPath, Buffer.from("tampered bytes"))

    const first = await verifyAssetIndex(runtimePath, "determinism-index")
    const second = await verifyAssetIndex(runtimePath, "determinism-index")

    // `toEqual` alone cannot tell "deterministically samples the right
    // objects" from "deterministically samples nothing": a `sampleKeys`
    // striding branch that degenerated to returning `[]` would make both
    // runs come back clean and equal, and this test would stay green. The
    // corruption must actually be found, on top of being found the same way
    // twice, for this to exercise the striding branch at all rather than
    // just its stability.
    expect(first.ok).toBe(false)
    expect(first.problems.some((p) => p.includes(corruptedName))).toBe(true)
    expect(first).toEqual(second)
  })
})

describe("verifyAssetIndex — legacy/virtual layout", () => {
  it("verifies a well-formed legacy/virtual tree clean", async () => {
    const objects: Record<string, { hash: string; size: number }> = {}
    const names = [
      "sound/random/click.ogg",
      "lang/en_us.lang",
      "icons/icon_16x16.png"
    ]

    for (const name of names) {
      const content = Buffer.from(`legacy-${name}`)
      const hash = sha1(content)
      objects[name] = { hash, size: content.length }
      await writeVirtualAsset(runtimePath, "legacy", name, content)
    }

    await writeAssetIndex(runtimePath, "legacy", { virtual: true, objects })

    const result = await verifyAssetIndex(runtimePath, "legacy")
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("does not false-positive on a legacy tree by expecting content-addressed objects", async () => {
    // The precise trap: a verifier that only understands the modern layout
    // reports every legacy asset as "missing" because it looks under
    // assets/objects/<hash prefix>/<hash> instead of
    // assets/virtual/<indexId>/<real name>, even though the file is present
    // and correct.
    const content = Buffer.from("legacy sound bytes")
    const hash = sha1(content)
    await writeVirtualAsset(
      runtimePath,
      "legacy",
      "sound/random/click.ogg",
      content
    )
    await writeAssetIndex(runtimePath, "legacy", {
      virtual: true,
      objects: { "sound/random/click.ogg": { hash, size: content.length } }
    })

    const result = await verifyAssetIndex(runtimePath, "legacy")
    expect(result.ok).toBe(true)
    expect(result.problems).toEqual([])
  })

  it("reports a missing object in a legacy tree at its virtual path", async () => {
    const content = Buffer.from("legacy sound bytes")
    const hash = sha1(content)
    // Deliberately do not write the virtual asset file.
    await writeAssetIndex(runtimePath, "legacy-missing", {
      virtual: true,
      objects: { "sound/random/click.ogg": { hash, size: content.length } }
    })

    const result = await verifyAssetIndex(runtimePath, "legacy-missing")
    expect(result.ok).toBe(false)
    expect(
      result.problems.some((p) => p.includes("sound/random/click.ogg"))
    ).toBe(true)
    expect(
      result.problems.some((p) =>
        p.includes(path.join("assets", "virtual", "legacy-missing"))
      )
    ).toBe(true)
  })
})

describe("verifyLibrariesPresent", () => {
  it("verifies clean when every library exists", async () => {
    const relPaths = [
      "com/google/guava/guava/31.1-jre/guava-31.1-jre.jar",
      "org/ow2/asm/asm/9.3/asm-9.3.jar"
    ]
    for (const rel of relPaths) {
      await writeLibrary(runtimePath, rel, Buffer.from("jar bytes"))
    }

    const result = await verifyLibrariesPresent(runtimePath, relPaths)
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports every missing library, not just the first", async () => {
    const present = "com/google/guava/guava/31.1-jre/guava-31.1-jre.jar"
    const missingA = "org/ow2/asm/asm/9.3/asm-9.3.jar"
    const missingB =
      "net/fabricmc/fabric-loader/0.14.21/fabric-loader-0.14.21.jar"

    await writeLibrary(runtimePath, present, Buffer.from("jar bytes"))

    const result = await verifyLibrariesPresent(runtimePath, [
      present,
      missingA,
      missingB
    ])

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(2)
    expect(result.problems.some((p) => p.includes(missingA))).toBe(true)
    expect(result.problems.some((p) => p.includes(missingB))).toBe(true)
  })
})

describe("sampleKeys", () => {
  it("is deterministic for the same input", () => {
    const keys = Array.from({ length: 500 }, (_, i) => `key-${i}`)
    expect(sampleKeys(keys)).toEqual(sampleKeys(keys))
  })

  it("returns every key when there are fewer than the sample size", () => {
    const keys = ["b", "a", "c"]
    expect(sampleKeys(keys, 20)).toEqual(["a", "b", "c"])
  })

  it("caps the sample at the requested size", () => {
    const keys = Array.from({ length: 1000 }, (_, i) => `key-${i}`)
    expect(sampleKeys(keys, 20)).toHaveLength(20)
  })
})

describe("verifyLibrariesAbsent", () => {
  it("verifies clean when none of the paths exist", async () => {
    const result = await verifyLibrariesAbsent(runtimePath, [
      "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-client.jar",
      "org/ow2/asm/asm/9.3/asm-9.3.jar"
    ])
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports every path that still exists, not just the first", async () => {
    const goneA = "org/ow2/asm/asm/9.3/asm-9.3.jar"
    const stillThereA =
      "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-client.jar"
    const stillThereB =
      "net/minecraft/client/1.20.1/client-1.20.1-srg.jar"

    await writeLibrary(runtimePath, stillThereA, Buffer.from("jar bytes"))
    await writeLibrary(runtimePath, stillThereB, Buffer.from("jar bytes"))

    const result = await verifyLibrariesAbsent(runtimePath, [
      goneA,
      stillThereA,
      stillThereB
    ])

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(2)
    expect(result.problems.some((p) => p.includes(stillThereA))).toBe(true)
    expect(result.problems.some((p) => p.includes(stillThereB))).toBe(true)
    expect(result.problems.some((p) => p.includes(goneA))).toBe(false)
  })

  it("verifies clean on an empty path list", async () => {
    const result = await verifyLibrariesAbsent(runtimePath, [])
    expect(result).toEqual({ ok: true, problems: [] })
  })
})
