import { describe, expect, it } from "vitest"
import { decodeMatrix, encodeMatrix, pickMatrix } from "./versionMatrix.js"

// Newest-first, like Mojang's real manifest. Deliberately carries many more
// releases than the plan pins: six of these become pinned entries, so the
// random pool must stay large enough that a draw is a real choice rather
// than a permutation of two candidates.
const MANIFEST = [
  { id: "1.21.4", type: "release" },
  { id: "24w45a", type: "snapshot" },
  { id: "1.21.1", type: "release" },
  { id: "1.20.4", type: "release" },
  { id: "1.20.2", type: "release" },
  { id: "1.20.1", type: "release" },
  { id: "1.19.4", type: "release" },
  { id: "1.19.2", type: "release" },
  { id: "1.18.2", type: "release" },
  { id: "1.17.1", type: "release" },
  { id: "1.16.5", type: "release" },
  { id: "1.15.2", type: "release" },
  { id: "1.14.4", type: "release" },
  { id: "1.13.2", type: "release" },
  { id: "1.12.2", type: "release" },
  { id: "1.11.2", type: "release" },
  { id: "1.10.2", type: "release" },
  { id: "1.9.4", type: "release" },
  { id: "1.8.9", type: "release" },
  { id: "1.7.10", type: "release" },
  { id: "1.6.4", type: "release" },
  { id: "b1.7.3", type: "old_beta" }
]

describe("pickMatrix", () => {
  it("is deterministic for a given seed", () => {
    const a = pickMatrix(MANIFEST, { seed: 12345, randomCount: 2 })
    const b = pickMatrix(MANIFEST, { seed: 12345, randomCount: 2 })
    expect(a).toEqual(b)
  })

  it("lets the seed actually drive the draw", () => {
    // Asserting that two specific seeds differ would be a coin flip, not a
    // test: any given pair can legitimately collide. The real property is
    // that the seed is consulted at all, so sample many seeds and require
    // the draws to spread.
    const drawFor = (seed: number) =>
      pickMatrix(MANIFEST, { seed, randomCount: 2 })
        .filter((e) => e.source === "random")
        .map((e) => e.id)
        .join(",")

    const distinct = new Set(Array.from({ length: 20 }, (_, s) => drawFor(s)))
    expect(distinct.size).toBeGreaterThan(5)
  })

  it("always includes every pinned version present in the manifest", () => {
    const matrix = pickMatrix(MANIFEST, { seed: 7, randomCount: 2 })
    for (const pinned of ["1.6.4", "1.7.10", "1.12.2", "1.16.5", "1.20.1"]) {
      expect(matrix).toContainEqual({ id: pinned, source: "pinned" })
    }
  })

  it("pins the newest release rather than a hardcoded id", () => {
    const entry = pickMatrix(MANIFEST, { seed: 7, randomCount: 0 }).find(
      (e) => e.id === "1.21.4"
    )
    expect(entry).toEqual({ id: "1.21.4", source: "pinned" })
  })

  it("resolves the newest release from the manifest, not a fixed id", () => {
    // The test above can't tell a real resolution from a hardcoded "1.21.4"
    // literal, since every other fixture in this file also happens to have
    // 1.21.4 first. Use a manifest whose newest release is something else
    // entirely to prove the id is actually read off the manifest.
    const olderManifest = [
      { id: "1.18.2", type: "release" },
      { id: "1.17.1", type: "release" },
      { id: "1.16.5", type: "release" },
      { id: "1.12.2", type: "release" },
      { id: "1.7.10", type: "release" }
    ]
    const matrix = pickMatrix(olderManifest, { seed: 11, randomCount: 0 })
    expect(matrix).toContainEqual({ id: "1.18.2", source: "pinned" })
    expect(matrix.map((e) => e.id)).not.toContain("1.21.4")
  })

  it("draws random picks only from releases, never snapshots or old_beta", () => {
    const releases = new Set(
      MANIFEST.filter((v) => v.type === "release").map((v) => v.id)
    )
    for (let seed = 0; seed < 25; seed++) {
      for (const entry of pickMatrix(MANIFEST, { seed, randomCount: 3 })) {
        if (entry.source === "random") expect(releases.has(entry.id)).toBe(true)
      }
    }
  })

  it("never repeats a version between pinned and random", () => {
    for (let seed = 0; seed < 25; seed++) {
      const ids = pickMatrix(MANIFEST, { seed, randomCount: 3 }).map(
        (e) => e.id
      )
      expect(new Set(ids).size).toBe(ids.length)
    }
  })

  it("tolerates a pinned version missing from the manifest", () => {
    const withoutOld = MANIFEST.filter((v) => v.id !== "1.7.10")
    const ids = pickMatrix(withoutOld, { seed: 3, randomCount: 1 }).map(
      (e) => e.id
    )
    expect(ids).not.toContain("1.7.10")
    expect(ids).toContain("1.12.2")
  })

  it("caps randomCount at the number of available releases", () => {
    const tiny = [
      { id: "1.20.1", type: "release" },
      { id: "1.19.2", type: "release" }
    ]
    // "1.20.1" is both a pinned literal and the newest release, leaving
    // exactly one release ("1.19.2") in the random pool. An off-by-one that
    // draws pool.length - 1 entries would silently drop it, so assert the
    // exact expected shape rather than an inequality that under-drawing
    // would still satisfy.
    const matrix = pickMatrix(tiny, { seed: 5, randomCount: 10 })
    expect(matrix).toHaveLength(2)
    expect(matrix).toContainEqual({ id: "1.20.1", source: "pinned" })
    expect(matrix).toContainEqual({ id: "1.19.2", source: "random" })
  })
})

describe("encodeMatrix / decodeMatrix", () => {
  it("round-trips", () => {
    const matrix = pickMatrix(MANIFEST, { seed: 99, randomCount: 2 })
    expect(decodeMatrix(encodeMatrix(matrix))).toEqual(matrix)
  })

  it("rejects malformed input rather than returning a partial matrix", () => {
    expect(() => decodeMatrix("not json")).toThrow()
    expect(() => decodeMatrix(JSON.stringify([{ id: "1.20.1" }]))).toThrow()
  })
})
