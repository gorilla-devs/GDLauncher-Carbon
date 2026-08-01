/**
 * Snapshots an instance's file tree so a test can assert what a modpack
 * version change did to it — and, more importantly, what it left alone.
 *
 * Pure Node: no Playwright, no DOM, so it unit-tests directly and is
 * reusable by any spec. Deliberately hashes every file rather than sampling
 * (unlike `installVerify.ts`'s asset-index sampling): a modpack instance is
 * a few hundred files of a few MB, and a version-change test's whole value
 * is knowing exactly which bytes moved.
 */

import { createHash } from "node:crypto"
import fs from "node:fs/promises"
import path from "node:path"

export interface TreeEntry {
  size: number
  sha256: string
}

/** Relative forward-slash path -> its size and hash. */
export type Tree = Map<string, TreeEntry>

export interface TreeDiff {
  added: string[]
  removed: string[]
  changed: string[]
  same: string[]
}

async function walk(dir: string, prefix: string, out: Tree): Promise<void> {
  let entries
  try {
    entries = await fs.readdir(dir, { withFileTypes: true })
  } catch (err) {
    // A tree that does not exist is an empty tree, not a throw: callers
    // snapshot directories the app creates lazily (`mods/` is not created
    // until the first mod install — see `installedInstance.ts`), and an
    // ENOENT there is information, not an error.
    if ((err as NodeJS.ErrnoException).code === "ENOENT") return
    throw err
  }

  for (const entry of entries) {
    const abs = path.join(dir, entry.name)
    const rel = prefix === "" ? entry.name : `${prefix}/${entry.name}`

    // Symlinks are recorded as absent rather than followed. Nothing in the
    // modpack pipeline creates one, so encountering one means something
    // outside this suite's model happened, and silently following it could
    // hash the same bytes under two paths.
    if (entry.isDirectory()) {
      await walk(abs, rel, out)
    } else if (entry.isFile()) {
      const body = await fs.readFile(abs)
      out.set(rel, {
        size: body.byteLength,
        sha256: createHash("sha256").update(body).digest("hex")
      })
    }
  }
}

export async function snapshotTree(dir: string): Promise<Tree> {
  const out: Tree = new Map()
  await walk(dir, "", out)
  return out
}

export function diffTrees(before: Tree, after: Tree): TreeDiff {
  const added: string[] = []
  const removed: string[] = []
  const changed: string[] = []
  const same: string[] = []

  for (const [rel, entry] of after) {
    const prior = before.get(rel)
    if (!prior) added.push(rel)
    else if (prior.sha256 !== entry.sha256) changed.push(rel)
    else same.push(rel)
  }
  for (const rel of before.keys()) {
    if (!after.has(rel)) removed.push(rel)
  }

  return {
    added: added.sort(),
    removed: removed.sort(),
    changed: changed.sort(),
    same: same.sort()
  }
}
