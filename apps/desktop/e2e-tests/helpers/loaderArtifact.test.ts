import { afterEach, beforeEach, expect, it } from "vitest"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { createHash } from "node:crypto"
import { DatabaseSync } from "node:sqlite"
import { readPartialVersionInfo } from "./versionCache.js"
import { verifyLegacyForgeRuntime } from "./loaderArtifact.js"
import { verifyLibrariesPresent } from "./installVerify.js"
import { requiredLibraryPaths } from "./processorOutputs.js"
let root: string
const version = "1.12.2-14.23.5.2860",
  cacheId = `forge-${version}`
const body = Buffer.from("selected synthetic runtime")
const digest = createHash("sha1").update(body).digest("hex")
const runtime = `net/minecraftforge/forge/${version}/forge-${version}.jar`
const base = {
  processors: [],
  data: {},
  libraries: [
    {
      name: `net.minecraftforge:forge:${version}`,
      downloads: {
        artifact: {
          path: runtime,
          size: body.length,
          sha1: digest.toUpperCase()
        }
      }
    }
  ]
}
beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-loader-artifact-"))
  const db = new DatabaseSync(path.join(root, "gdl_conf.db"))
  db.exec(
    "CREATE TABLE PartialVersionInfoCache (id TEXT PRIMARY KEY, partialVersionInfo BLOB)"
  )
  db.prepare("INSERT INTO PartialVersionInfoCache VALUES (?,?)").run(
    cacheId,
    Buffer.from(JSON.stringify(base))
  )
  db.close()
})
afterEach(() => fs.rmSync(root, { recursive: true, force: true }))
function write(rel: string, bytes: Buffer = body) {
  const file = path.join(root, "libraries", rel)
  fs.mkdirSync(path.dirname(file), { recursive: true })
  fs.writeFileSync(file, bytes)
}
it("zero processors and vanilla libraries cannot prove an absent selected Forge runtime", async () => {
  write("net/minecraft/client/1.12.2/1.12.2.jar")
  const profile = readPartialVersionInfo(root, cacheId)
  expect(requiredLibraryPaths(profile.processors ?? [], profile.data)).toEqual(
    []
  )
  expect(await verifyLibrariesPresent(root, [])).toEqual({
    ok: true,
    problems: []
  })
  expect(
    (await verifyLegacyForgeRuntime(root, cacheId, version, profile)).ok
  ).toBe(false)
})
it.each(["installer", "wrong-version", "wrong-classifier"])(
  "%s does not substitute for runtime",
  async (mode) => {
    const name =
      mode === "installer"
        ? `net.minecraftforge:forge:${version}:installer`
        : mode === "wrong-version"
          ? "net.minecraftforge:forge:1.12.2-14.23.5.9999"
          : `net.minecraftforge:forge:${version}:client`
    write(runtime)
    expect(
      (
        await verifyLegacyForgeRuntime(root, cacheId, version, {
          ...base,
          libraries: [{ name }]
        })
      ).ok
    ).toBe(false)
  }
)
it("exact cached runtime bytes size and case-insensitive SHA1 succeed", async () => {
  write(runtime)
  expect(
    (
      await verifyLegacyForgeRuntime(
        root,
        cacheId,
        version,
        readPartialVersionInfo(root, cacheId)
      )
    ).ok
  ).toBe(true)
})
it.each(["size", "hash", "directory"])("rejects runtime %s", async (mode) => {
  if (mode === "directory")
    fs.mkdirSync(path.join(root, "libraries", runtime), { recursive: true })
  else
    write(
      runtime,
      mode === "size" ? Buffer.from("short") : Buffer.alloc(body.length)
    )
  expect(
    (
      await verifyLegacyForgeRuntime(
        root,
        cacheId,
        version,
        readPartialVersionInfo(root, cacheId)
      )
    ).ok
  ).toBe(false)
})
it("explicit artifact path wins over Maven fallback", async () => {
  const profile = {
    ...base,
    libraries: [
      {
        ...base.libraries[0],
        downloads: {
          artifact: {
            path: "custom/runtime.jar",
            size: body.length,
            sha1: digest
          }
        }
      }
    ]
  }
  write(runtime)
  expect(
    (await verifyLegacyForgeRuntime(root, cacheId, version, profile)).ok
  ).toBe(false)
  write("custom/runtime.jar")
  expect(
    (await verifyLegacyForgeRuntime(root, cacheId, version, profile)).ok
  ).toBe(true)
})
it("Maven fallback and universal classifier work when explicit path absent", async () => {
  write(runtime)
  expect(
    (
      await verifyLegacyForgeRuntime(root, cacheId, version, {
        libraries: [{ name: `net.minecraftforge:forge:${version}` }]
      })
    ).ok
  ).toBe(true)
  write(runtime.replace(".jar", "-universal.jar"))
  expect(
    (
      await verifyLegacyForgeRuntime(root, cacheId, version, {
        libraries: [{ name: `net.minecraftforge:forge:${version}:universal` }]
      })
    ).ok
  ).toBe(true)
})
it("ambiguous runtime and wrong cache identity fail", async () => {
  write(runtime)
  expect(
    (
      await verifyLegacyForgeRuntime(root, cacheId, version, {
        libraries: [
          base.libraries[0],
          { name: `net.minecraftforge:forge:${version}:universal` }
        ]
      })
    ).ok
  ).toBe(false)
  expect(
    (await verifyLegacyForgeRuntime(root, "forge-wrong", version, base)).ok
  ).toBe(false)
})
it("retains nonempty missing processor-path failure", async () => {
  expect((await verifyLibrariesPresent(root, ["missing.jar"])).ok).toBe(false)
})
