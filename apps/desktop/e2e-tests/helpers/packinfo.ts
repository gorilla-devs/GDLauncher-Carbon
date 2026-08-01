/**
 * Reads `packinfo.json` — the modpack pipeline's own record of which files
 * the pack owns and what they hashed to when it installed them.
 *
 * Written by `packinfo::scan_dir`
 * (`crates/carbon_app/src/managers/instance/modpack/packinfo/scan.rs`) from a
 * scan of the staging tree, and read back by `process_modpack_staging` on the
 * next version change to decide, file by file, what may be replaced.
 * `classifyPackinfo` below computes exactly the partition that code will
 * compute, which is what lets a test predict an upgrade's decisions instead
 * of merely observing them.
 *
 * Layout, which is easy to get wrong: the file lives at the instance
 * **root**, its keys are relative to `<root>/instance` and carry a leading
 * slash, and the scanner strips a trailing `.disabled` before recording a
 * key — so a disabled pack mod is tracked under its enabled name and sits on
 * disk under the suffixed one.
 */

import { createHash } from "node:crypto"
import fs from "node:fs/promises"
import path from "node:path"

export interface PackinfoEntry {
  sha512: string
  md5: string
}

/** Packinfo key (leading `/`, relative to `<root>/instance`) -> its hashes. */
export type Packinfo = Map<string, PackinfoEntry>

export interface PackinfoStatus {
  /** On disk, md5 still equal to what the pack recorded. Only these are
   *  eligible for replacement or deletion by the next version change. */
  pristine: string[]
  /** On disk, md5 differs — `SkipReplaceReason::ModifiedByUser`. */
  modified: string[]
  /** Not on disk under either its own name or `<name>.disabled` —
   *  `SkipReplaceReason::DeletedByUser`. */
  missing: string[]
}

export function packinfoDataPath(instanceRoot: string, key: string): string {
  return path.join(instanceRoot, "instance", key.replace(/^\//, ""))
}

export async function readPackinfo(instanceRoot: string): Promise<Packinfo> {
  const file = path.join(instanceRoot, "packinfo.json")
  let raw: string
  try {
    raw = await fs.readFile(file, "utf8")
  } catch (cause) {
    throw new Error(`could not read packinfo.json at ${file}`, { cause })
  }

  const parsed = JSON.parse(raw) as {
    files?: Record<string, { sha512: string; md5: string }>
  }
  if (!parsed.files) {
    throw new Error(`packinfo.json at ${file} has no "files" object`)
  }

  return new Map(Object.entries(parsed.files))
}

export async function classifyPackinfo(
  instanceRoot: string
): Promise<PackinfoStatus> {
  const info = await readPackinfo(instanceRoot)
  const status: PackinfoStatus = { pristine: [], modified: [], missing: [] }

  for (const [key, entry] of info) {
    const enabled = packinfoDataPath(instanceRoot, key)
    let body: Buffer | undefined
    for (const candidate of [enabled, `${enabled}.disabled`]) {
      try {
        body = await fs.readFile(candidate)
        break
      } catch (err) {
        if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err
      }
    }

    if (!body) {
      status.missing.push(key)
      continue
    }

    const md5 = createHash("md5").update(body).digest("hex")
    if (md5 === entry.md5) status.pristine.push(key)
    else status.modified.push(key)
  }

  status.pristine.sort()
  status.modified.sort()
  status.missing.sort()
  return status
}
