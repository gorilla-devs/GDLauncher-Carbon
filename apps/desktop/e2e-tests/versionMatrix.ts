/**
 * Picks which Minecraft versions the e2e install suite exercises on a given
 * run: a fixed set of pinned versions (each straddling a real format
 * boundary — legacy vs. modern asset index, pre/post-1.13 flattening,
 * LWJGL 2→3) plus a seeded random sample of additional releases, so runs
 * are reproducible from a single seed while still covering the wider
 * version surface over time.
 */

/**
 * The manifest's own newest release is resolved at pick time (see
 * `pickMatrix`) and added on top of this list — never hardcode a "latest"
 * id here, since it goes stale the moment Mojang ships a new version.
 */
export const PINNED_VERSIONS: readonly string[] = [
  "1.7.10",
  "1.12.2",
  "1.16.5",
  "1.20.1"
]

export interface ManifestVersion {
  id: string
  type: string
}

export interface MatrixEntry {
  id: string
  source: "pinned" | "random"
}

/**
 * Deterministic PRNG (mulberry32). Given the same seed it produces the same
 * sequence of floats in [0, 1) on every platform, which is what makes
 * `pickMatrix` reproducible.
 */
function mulberry32(seed: number): () => number {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

export function pickMatrix(
  versions: ManifestVersion[],
  opts: { seed: number; randomCount: number }
): MatrixEntry[] {
  const newestRelease = versions.find((v) => v.type === "release")

  const pinnedIds = [...PINNED_VERSIONS]
  if (newestRelease && !pinnedIds.includes(newestRelease.id)) {
    pinnedIds.push(newestRelease.id)
  }
  const pinnedSet = new Set(pinnedIds)

  // Manifest order, filtered down to the pinned ids actually present.
  const pinnedEntries: MatrixEntry[] = versions
    .filter((v) => pinnedSet.has(v.id))
    .map((v) => ({ id: v.id, source: "pinned" as const }))

  // Random candidates: releases only, excluding anything already pinned.
  const pool = versions
    .filter((v) => v.type === "release" && !pinnedSet.has(v.id))
    .map((v) => v.id)

  const drawCount = Math.min(opts.randomCount, pool.length)
  const random = mulberry32(opts.seed)

  // Seeded Fisher-Yates partial shuffle, taking from the front.
  for (let i = 0; i < drawCount; i++) {
    const j = i + Math.floor(random() * (pool.length - i))
    ;[pool[i], pool[j]] = [pool[j], pool[i]]
  }
  const randomEntries: MatrixEntry[] = pool
    .slice(0, drawCount)
    .map((id) => ({ id, source: "random" as const }))

  return [...pinnedEntries, ...randomEntries]
}

export function encodeMatrix(entries: MatrixEntry[]): string {
  return JSON.stringify(entries)
}

export function decodeMatrix(raw: string): MatrixEntry[] {
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch (err) {
    throw new Error(
      `decodeMatrix: invalid JSON: ${err instanceof Error ? err.message : String(err)}`
    )
  }

  if (!Array.isArray(parsed)) {
    throw new Error("decodeMatrix: expected a JSON array")
  }

  parsed.forEach((entry, i) => {
    if (
      typeof entry !== "object" ||
      entry === null ||
      typeof (entry as Record<string, unknown>).id !== "string" ||
      ((entry as Record<string, unknown>).source !== "pinned" &&
        (entry as Record<string, unknown>).source !== "random")
    ) {
      throw new Error(
        `decodeMatrix: entry at index ${i} is not a valid MatrixEntry: ${JSON.stringify(entry)}`
      )
    }
  })

  return parsed as MatrixEntry[]
}
