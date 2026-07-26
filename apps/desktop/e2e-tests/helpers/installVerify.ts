/**
 * Verifies that a Minecraft install under a GDL runtime path is genuinely
 * correct on disk, independent of anything the app itself reported. Pure
 * Node: no Playwright, no DOM, so it can be exercised directly against
 * synthetic trees in unit tests and reused from Playwright specs alike.
 *
 * Layout is taken from the Rust runtime-path source, not inferred:
 * - client jar: `libraries/net/minecraft/client/<id>/<id>.jar`
 *   (`LibrariesPath::get_mc_client`, crates/carbon_rt_path/src/lib.rs:27-32)
 * - asset indexes: `assets/indexes/<indexId>.json`
 *   (`AssetsPath::get_indexes_path`, crates/carbon_rt_path/src/lib.rs:49-51)
 * - asset objects (modern): `assets/objects/<hash[0..2]>/<hash>`
 *   (`AssetsPath::get_objects_path` + `asset_object_location`,
 *   crates/carbon_rt_path/src/lib.rs:57-59 and
 *   crates/carbon_app/src/domain/minecraft/minecraft.rs:273-276)
 * - asset objects (legacy/virtual): `assets/virtual/<indexId>/<name>`
 *   (`AssetsPath::get_virtual_path`, crates/carbon_rt_path/src/lib.rs:53-55)
 * - libraries: `libraries/<maven-style-relative-path>`
 *   (`LibrariesPath::get_library_path`, crates/carbon_rt_path/src/lib.rs:33-35)
 *
 * Legacy vs. modern asset layout is not guessed from the version id: it is
 * read straight off the asset index JSON's own `"virtual"` field, exactly
 * like the app does. `daedalus::minecraft::AssetsIndex` deserializes that
 * field into `map_virtual` (`#[serde(rename = "virtual")]`,
 * daedalus/src/minecraft.rs:930-940), and
 * `crates/carbon_app/src/managers/minecraft/assets.rs:143-153` (`get_assets_dir`)
 * and `:189-199` (`reconstruct_assets`) both branch on
 * `assets_index.map_virtual` to decide between the virtual/legacy path and
 * the content-addressed objects path. 1.6.4's index sets `"virtual": true`
 * (its index id is literally `legacy`, shared with the rest of the 1.6.x
 * release line), which is what puts its assets under
 * `assets/virtual/legacy/<name>` instead of `assets/objects/..`. Confirmed
 * directly against live Mojang data, not assumed from the version number:
 * 1.7.10 and every later version checked (1.12.2, 1.16.5, 1.20.1) resolve to
 * their own non-virtual index instead — `legacy` is exactly this old. This
 * module mirrors the JSON's own branch rather than special-casing any
 * version id, which is what keeps it correct regardless of which id Mojang
 * currently routes through that branch.
 *
 * `map_to_resources` (assets copied into the instance's own resources dir)
 * is a third layout the same JSON field set can select, but it addresses a
 * per-instance directory outside the shared runtime asset store this module
 * verifies, so it is out of scope here.
 */

import { createHash } from "node:crypto"
import fs from "node:fs/promises"
import path from "node:path"

export interface VerifyResult {
  ok: boolean
  problems: string[]
}

interface RawAssetsIndex {
  virtual?: boolean
  objects?: Record<string, { hash: string; size: number }>
}

/** Objects hashed per asset index, chosen deterministically (see `sampleKeys`). */
const HASH_SAMPLE_SIZE = 20

/** Concurrent filesystem operations in flight for existence sweeps. */
const CONCURRENCY = 64

function okResult(): VerifyResult {
  return { ok: true, problems: [] }
}

function failResult(problems: string[]): VerifyResult {
  return { ok: false, problems }
}

export async function pathExists(target: string): Promise<boolean> {
  try {
    await fs.access(target)
    return true
  } catch {
    return false
  }
}

/** Runs `fn` over `items` with bounded concurrency, preserving result order. */
async function mapConcurrent<T, R>(
  items: readonly T[],
  limit: number,
  fn: (item: T, index: number) => Promise<R>
): Promise<R[]> {
  const results: R[] = new Array(items.length)
  let cursor = 0

  async function worker(): Promise<void> {
    while (cursor < items.length) {
      const current = cursor
      cursor += 1
      results[current] = await fn(items[current], current)
    }
  }

  const workerCount = Math.max(1, Math.min(limit, items.length))
  await Promise.all(Array.from({ length: workerCount }, () => worker()))
  return results
}

export async function sha1OfFile(filePath: string): Promise<string> {
  const data = await fs.readFile(filePath)
  return createHash("sha1").update(data).digest("hex")
}

export function clientJarPath(runtimePath: string, versionId: string): string {
  return path.join(
    runtimePath,
    "libraries",
    "net",
    "minecraft",
    "client",
    versionId,
    `${versionId}.jar`
  )
}

export function assetIndexPath(runtimePath: string, indexId: string): string {
  return path.join(runtimePath, "assets", "indexes", `${indexId}.json`)
}

/** Content-addressed path for a modern (non-virtual) asset object. */
export function assetObjectPath(runtimePath: string, hash: string): string {
  return path.join(runtimePath, "assets", "objects", hash.slice(0, 2), hash)
}

/** Real-filename path for a legacy/virtual asset object. */
export function virtualAssetPath(
  runtimePath: string,
  indexId: string,
  name: string
): string {
  return path.join(runtimePath, "assets", "virtual", indexId, name)
}

export async function verifyClientJar(
  runtimePath: string,
  versionId: string,
  expectedSha1?: string
): Promise<VerifyResult> {
  const jarPath = clientJarPath(runtimePath, versionId)

  if (!(await pathExists(jarPath))) {
    return failResult([`client jar missing at ${jarPath}`])
  }

  if (expectedSha1) {
    const actual = await sha1OfFile(jarPath)
    if (actual.toLowerCase() !== expectedSha1.toLowerCase()) {
      return failResult([
        `client jar at ${jarPath} has sha1 ${actual}, expected ${expectedSha1}`
      ])
    }
  }

  return okResult()
}

/**
 * Deterministically picks up to `sampleSize` keys out of `keys` by sorting
 * them and striding across the sorted list. Depends only on the key set
 * itself (never on wall-clock time or PRNG state), so two runs over the same
 * asset index sample exactly the same objects.
 */
export function sampleKeys(
  keys: readonly string[],
  sampleSize: number = HASH_SAMPLE_SIZE
): string[] {
  if (keys.length <= sampleSize) {
    return [...keys].sort()
  }

  const sorted = [...keys].sort()
  const stride = sorted.length / sampleSize
  const picked: string[] = []
  const seenIndices = new Set<number>()

  for (let i = 0; i < sampleSize; i++) {
    const index = Math.min(sorted.length - 1, Math.floor(i * stride))
    if (!seenIndices.has(index)) {
      seenIndices.add(index)
      picked.push(sorted[index])
    }
  }

  return picked
}

export async function verifyAssetIndex(
  runtimePath: string,
  indexId: string
): Promise<VerifyResult> {
  const indexPath = assetIndexPath(runtimePath, indexId)

  if (!(await pathExists(indexPath))) {
    return failResult([`asset index missing at ${indexPath}`])
  }

  let parsed: RawAssetsIndex
  try {
    parsed = JSON.parse(await fs.readFile(indexPath, "utf8")) as RawAssetsIndex
  } catch (err) {
    return failResult([
      `asset index at ${indexPath} is not valid JSON: ${err instanceof Error ? err.message : String(err)}`
    ])
  }

  // A missing `objects` key and an empty `objects` map are distinct failure
  // shapes worth telling apart when diagnosing: a missing key points at a
  // truncated or wrong-shaped file (e.g. a partial write cut off mid-JSON
  // before the key was ever written, or the wrong document entirely), while
  // a present-but-empty map points at something that wrote a
  // valid-but-hollow index. No legitimate Minecraft asset index — legacy or
  // modern — has zero objects, so treat both as problems rather than
  // trivially verifying clean on zero objects to check.
  if (
    parsed.objects === undefined ||
    parsed.objects === null ||
    typeof parsed.objects !== "object" ||
    Array.isArray(parsed.objects)
  ) {
    return failResult([
      `asset index at ${indexPath} has no "objects" key (missing or malformed — likely a truncated or wrong-shaped file)`
    ])
  }

  const objects = parsed.objects
  const names = Object.keys(objects)

  if (names.length === 0) {
    return failResult([
      `asset index at ${indexPath} has an empty "objects" map (no assets listed — no legitimate asset index is empty)`
    ])
  }

  // Mirrors AssetsIndex::map_virtual (daedalus `"virtual"` field): only that
  // flag decides the layout, never the version id.
  const isVirtual = parsed.virtual === true

  const resolveObjectPath = (name: string, hash: string): string =>
    isVirtual
      ? virtualAssetPath(runtimePath, indexId, name)
      : assetObjectPath(runtimePath, hash)

  const problems: string[] = []
  const missingNames = new Set<string>()

  // An index entry that is `null` or has no string `hash` can't be resolved
  // to a path at all (the modern layout needs `hash.slice(0, 2)`, the legacy
  // layout needs a real hash to compare a sampled object against) — reported
  // as a problem for that one name rather than thrown out of `mapConcurrent`,
  // which would abort the whole call and, in a spec, replace a per-version
  // problem list with an unhelpful exception. Same never-throw contract this
  // module already keeps one level up, at the whole-file JSON parse (see the
  // "invalid JSON" test above).
  const validNames: string[] = []
  for (const name of names) {
    const obj = objects[name] as { hash?: unknown } | null | undefined
    if (!obj || typeof obj.hash !== "string" || obj.hash.length === 0) {
      problems.push(
        `asset index at ${indexPath} has an invalid entry for "${name}" ` +
          `(missing or non-string "hash") — cannot verify this object`
      )
      continue
    }
    validNames.push(name)
  }

  const existence = await mapConcurrent(
    validNames,
    CONCURRENCY,
    async (name) => {
      const obj = objects[name]
      const objectPath = resolveObjectPath(name, obj.hash)
      const exists = await pathExists(objectPath)
      return { name, obj, objectPath, exists }
    }
  )

  for (const entry of existence) {
    if (!entry.exists) {
      missingNames.add(entry.name)
      problems.push(
        `asset object "${entry.name}" (hash ${entry.obj.hash}) missing at ${entry.objectPath}`
      )
    }
  }

  // Hash a deterministic sample of the objects that do exist. Existence is
  // still checked for every object above; hashing all of them would dominate
  // the cost of a test whose point is the install, not this check.
  const presentNames = validNames.filter((name) => !missingNames.has(name))
  const sample = sampleKeys(presentNames)

  const hashResults = await mapConcurrent(sample, CONCURRENCY, async (name) => {
    const obj = objects[name]
    const objectPath = resolveObjectPath(name, obj.hash)
    const actual = await sha1OfFile(objectPath)
    return { name, obj, objectPath, actual }
  })

  for (const entry of hashResults) {
    if (entry.actual.toLowerCase() !== entry.obj.hash.toLowerCase()) {
      problems.push(
        `sampled asset object "${entry.name}" at ${entry.objectPath} has sha1 ${entry.actual}, expected ${entry.obj.hash}`
      )
    }
  }

  return problems.length ? failResult(problems) : okResult()
}

export async function verifyLibrariesPresent(
  runtimePath: string,
  relativePaths: string[]
): Promise<VerifyResult> {
  const results = await mapConcurrent(
    relativePaths,
    CONCURRENCY,
    async (relativePath) => {
      const libraryPath = path.join(runtimePath, "libraries", relativePath)
      return {
        relativePath,
        libraryPath,
        exists: await pathExists(libraryPath)
      }
    }
  )

  const problems = results
    .filter((r) => !r.exists)
    .map((r) => `library missing at ${r.libraryPath}`)

  return problems.length ? failResult(problems) : okResult()
}
