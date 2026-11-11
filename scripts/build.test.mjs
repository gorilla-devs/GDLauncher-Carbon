import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import { mkdtempSync, rmSync, symlinkSync, unlinkSync } from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import { afterEach, beforeEach, test } from "node:test"
import { fileURLToPath, pathToFileURL } from "node:url"

const scriptsDir = path.dirname(fileURLToPath(import.meta.url))
const buildPath = path.join(scriptsDir, "build.mjs")

let root
let link

beforeEach(() => {
  root = mkdtempSync(path.join(tmpdir(), "build-driver-"))
  link = path.join(root, "linked-scripts")
  // A junction needs no admin rights on Windows; POSIX ignores the type.
  symlinkSync(scriptsDir, link, "junction")
})

afterEach(() => {
  // Remove the link on its own first so nothing can descend into the real
  // scripts folder.
  unlinkSync(link)
  rmSync(root, { recursive: true, force: true })
})

// An unknown target makes the driver fail before it spawns anything, so these
// runs never start a real build.
function runDriver(scriptPath) {
  return spawnSync(process.execPath, [scriptPath, "no-such-target"], {
    encoding: "utf8"
  })
}

test("the build driver runs from its real path", () => {
  const result = runDriver(buildPath)

  assert.equal(result.status, 1)
  assert.match(result.stderr, /Unknown build target "no-such-target"/)
})

test("the build driver runs when started through a linked folder", () => {
  const result = runDriver(path.join(link, "build.mjs"))

  assert.equal(result.status, 1)
  assert.match(result.stderr, /Unknown build target "no-such-target"/)
})

test("importing the build driver does not run it", () => {
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `await import(${JSON.stringify(pathToFileURL(buildPath).href)})`,
      "no-such-target"
    ],
    { encoding: "utf8" }
  )

  assert.equal(result.status, 0)
  assert.equal(result.stderr, "")
})
