import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync
} from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import { afterEach, beforeEach, test } from "node:test"
import { fileURLToPath, pathToFileURL } from "node:url"

const websiteDir = fileURLToPath(new URL("..", import.meta.url))

let root
let link

// The updater and the modules it imports are copied into a temporary tree
// with no data folder, so a run reads empty history and never touches the
// real shards. A fetch stub stops the run at its first request, so nothing
// goes to the network.
beforeEach(() => {
  root = mkdtempSync(path.join(tmpdir(), "update-addons-entry-"))
  const real = path.join(root, "real")
  mkdirSync(path.join(real, "scripts"), { recursive: true })
  mkdirSync(path.join(real, "src", "lib"), { recursive: true })
  for (const file of ["scripts/update-addons.mjs", "scripts/addonShards.mjs"]) {
    copyFileSync(path.join(websiteDir, file), path.join(real, file))
  }
  copyFileSync(
    path.join(websiteDir, "src", "lib", "addonUrls.mjs"),
    path.join(real, "src", "lib", "addonUrls.mjs")
  )
  writeFileSync(
    path.join(root, "stub-fetch.mjs"),
    'globalThis.fetch = async () => {\n  console.log("fetch reached")\n  process.exit(0)\n}\n'
  )
  link = path.join(root, "link")
  // A junction needs no admin rights on Windows; POSIX ignores the type.
  symlinkSync(real, link, "junction")
})

afterEach(() => {
  // Remove the link on its own first so nothing can descend through it.
  unlinkSync(link)
  rmSync(root, { recursive: true, force: true })
})

function runUpdater(folder) {
  const env = { ...process.env }
  delete env.CURSEFORGE_API_KEY
  return spawnSync(
    process.execPath,
    [
      "--import",
      pathToFileURL(path.join(root, "stub-fetch.mjs")).href,
      path.join(folder, "scripts", "update-addons.mjs"),
      "--dry-run",
      "--limit",
      "1"
    ],
    { encoding: "utf8", env }
  )
}

test("the addon updater runs when started through a linked folder", () => {
  const result = runUpdater(link)

  assert.match(
    result.stdout,
    /Updating addon registry \(limit=1, dryRun=true\)/
  )
  assert.match(result.stdout, /fetch reached/)
  assert.equal(result.status, 0)
})

test("the addon updater runs when started from its real folder", () => {
  const result = runUpdater(path.join(root, "real"))

  assert.match(
    result.stdout,
    /Updating addon registry \(limit=1, dryRun=true\)/
  )
  assert.match(result.stdout, /fetch reached/)
  assert.equal(result.status, 0)
})
