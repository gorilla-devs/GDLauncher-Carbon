import { createHash } from "node:crypto"
import fs from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { DatabaseSync } from "node:sqlite"
import { afterEach, beforeEach, expect, it } from "vitest"
import { readVersionInfo } from "./versionCache.js"
import {
  assetIndexPath,
  assetObjectPath,
  clientJarPath,
  verifyCachedVanillaInstall
} from "./installVerify.js"

const sha1 = (bytes: string | Buffer) =>
  createHash("sha1").update(bytes).digest("hex")
let runtime: string
beforeEach(async () => {
  runtime = await fs.mkdtemp(path.join(os.tmpdir(), "gdl-cached-install-"))
})
afterEach(async () => {
  await fs.rm(runtime, { recursive: true, force: true })
})
async function write(target: string, bytes: string) {
  await fs.mkdir(path.dirname(target), { recursive: true })
  await fs.writeFile(target, bytes)
}
async function fixture(expected: unknown = sha1(rawOriginal)) {
  const cached = {
    assetIndex: { id: "shared-index", sha1: expected },
    downloads: { client: { sha1: sha1("client") } }
  }
  const db = new DatabaseSync(path.join(runtime, "gdl_conf.db"))
  try {
    db.exec(
      "CREATE TABLE VersionInfoCache(id TEXT PRIMARY KEY, versionInfo BLOB NOT NULL)"
    )
    db.prepare(
      "INSERT INTO VersionInfoCache(id, versionInfo) VALUES (?, ?)"
    ).run("fixture-version", Buffer.from(JSON.stringify(cached)))
  } finally {
    db.close()
  }
  await write(clientJarPath(runtime, "fixture-version"), "client")
  await write(assetIndexPath(runtime, "shared-index"), rawOriginal)
  await write(assetObjectPath(runtime, sha1("a")), "a")
  await write(assetObjectPath(runtime, sha1("b")), "b")
  return readVersionInfo(runtime, "fixture-version")
}
// Expected identity is retained independently of the suspect disk file.
const rawOriginal = JSON.stringify({
  objects: { a: { hash: sha1("a"), size: 1 }, b: { hash: sha1("b"), size: 1 } }
})
it("reads expected SHA1 from the real cached blob into the shared caller used by both install specs", async () => {
  const cached = await fixture()
  expect(cached.assetIndex).toEqual({
    id: "shared-index",
    sha1: sha1(rawOriginal)
  })
  expect(
    await verifyCachedVanillaInstall(runtime, "fixture-version", cached)
  ).toEqual({ ok: true, problems: [] })
  const subset = JSON.stringify({
    objects: { a: { hash: sha1("a"), size: 1 } }
  }).padEnd(rawOriginal.length, " ")
  await fs.rm(assetObjectPath(runtime, sha1("b")))
  await write(assetIndexPath(runtime, "shared-index"), subset)
  const result = await verifyCachedVanillaInstall(
    runtime,
    "fixture-version",
    cached
  )
  expect(result.ok).toBe(false)
  expect(result.problems.join("\n")).toContain(sha1(rawOriginal))
  expect(cached.assetIndex?.sha1).toBe(sha1(rawOriginal))
})
it.each([undefined, "", "malformed", 42])(
  "keeps optional persisted metadata readable but fails missing/invalid identity %s",
  async (expected) => {
    const cached = await fixture(expected)
    // default parameters would replace undefined; deliberately remove it from persisted JSON instead.
    if (expected === undefined) {
      const db = new DatabaseSync(path.join(runtime, "gdl_conf.db"))
      try {
        db.prepare("UPDATE VersionInfoCache SET versionInfo = ?").run(
          Buffer.from(
            JSON.stringify({
              assetIndex: { id: "shared-index" },
              downloads: cached.downloads
            })
          )
        )
      } finally {
        db.close()
      }
    }
    const result = await verifyCachedVanillaInstall(
      runtime,
      "fixture-version",
      readVersionInfo(runtime, "fixture-version")
    )
    expect(result.ok).toBe(false)
    expect(result.problems.join("\n")).toContain("expected sha1")
  }
)
it.each(["assetIndex", "downloads"])(
  "fails absent required cached %s fields",
  async (field) => {
    const cached = await fixture()
    delete cached[field as keyof typeof cached]
    expect(
      (await verifyCachedVanillaInstall(runtime, "fixture-version", cached)).ok
    ).toBe(false)
  }
)
