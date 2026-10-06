import * as fs from "node:fs"
import { resolve, dirname, basename } from "node:path"
import { randomUUID } from "node:crypto"

const shardPath = (dataDir, platform, type) => resolve(dataDir, `${platform}-${type}.json`)

/** Only a missing file starts an empty registry; damaged history is never discarded. */
export function loadShard(dataDir, platform, type, io = fs) {
  const path = shardPath(dataDir, platform, type)
  let text
  try {
    text = io.readFileSync(path, "utf-8")
  } catch (cause) {
    if (cause.code === "ENOENT") return []
    throw new Error(`Cannot read addon shard ${path}`, { cause })
  }
  try {
    const indexed = JSON.parse(text)
    if (indexed === null || typeof indexed !== "object" || Array.isArray(indexed)) {
      throw new Error("Expected a slug-indexed object")
    }
    return Object.values(indexed)
  } catch (cause) {
    throw new Error(`Invalid addon shard ${path}`, { cause })
  }
}

/** Validate all existing inputs before a serial update can publish any of them. */
export function prevalidateShards(dataDir, shards, io = fs) {
  for (const [platform, type] of shards) loadShard(dataDir, platform, type, io)
}

export function writeShard(dataDir, platform, type, list, io = fs) {
  const indexed = Object.create(null)
  for (const addon of list) {
    if (addon.slug) indexed[addon.slug] = addon
  }
  // Serialize before opening anything: JSON errors cannot damage existing bytes.
  const serialized = JSON.stringify(indexed) + "\n"
  const path = shardPath(dataDir, platform, type)
  io.mkdirSync(dataDir, { recursive: true })
  const temporary = resolve(dirname(path), `.${basename(path)}.${randomUUID()}.tmp`)
  let owned = false
  let descriptor
  const failures = []
  try {
    descriptor = io.openSync(temporary, "wx")
    owned = true
    io.writeFileSync(descriptor, serialized)
    const closing = descriptor
    descriptor = undefined
    io.closeSync(closing)
    io.renameSync(temporary, path)
    owned = false
  } catch (cause) {
    failures.push(cause)
  } finally {
    // A failed close acknowledgement must not skip stage cleanup or retry a
    // descriptor whose ownership is now uncertain.
    if (descriptor !== undefined) {
      try { io.closeSync(descriptor) } catch (cause) { failures.push(cause) }
    }
    if (owned) {
      try { io.unlinkSync(temporary) } catch (cause) { failures.push(cause) }
    }
  }
  if (failures.length) {
    throw new Error(`Cannot publish addon shard ${path}`, {
      cause: failures.length === 1 ? failures[0] : new AggregateError(failures, "Publication and cleanup failed"),
    })
  }
  return path
}

/** The CLI and fixtures share the same dry-run and publication path. */
export function publishShards(dataDir, shards, { dryRun = false, io = fs } = {}) {
  prevalidateShards(dataDir, shards, io)
  if (dryRun) return []
  return shards.map(([platform, type, list]) => writeShard(dataDir, platform, type, list, io))
}
