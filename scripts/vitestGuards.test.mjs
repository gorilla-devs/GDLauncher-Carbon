import assert from "node:assert/strict"
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  statSync,
  utimesSync,
  writeFileSync
} from "node:fs"
import { homedir, tmpdir } from "node:os"
import path from "node:path"
import { afterEach, beforeEach, test } from "node:test"
import {
  assertBuiltAfterSources,
  assertEveryConfigRuns,
  assertNoMachinePaths,
  assertOneProjectPerFile
} from "./vitestGuards.mjs"

let root

beforeEach(() => {
  root = mkdtempSync(path.join(tmpdir(), "vitest-guards-"))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

function write(file, content = "") {
  mkdirSync(path.dirname(path.join(root, file)), { recursive: true })
  writeFileSync(path.join(root, file), content)
}

test("a config no project or script runs names where to wire it", () => {
  write("vitest.config.mjs")
  write("vitest.task1.config.mjs")
  write("vitest.extra.config.mjs")

  assert.throws(
    () =>
      assertEveryConfigRuns(root, ["vitest.task1.config.mjs"], {
        "vitest.contract.config.mjs": "pnpm test:contract"
      }),
    (error) => {
      assert.match(error.message, /^No script runs vitest\.extra\.config\.mjs,/)
      assert.match(
        error.message,
        /projects in the vitest\.config\.mjs next to it/
      )
      assert.match(
        error.message,
        /standalone list that vitest\.config\.mjs passes to assertEveryConfigRuns/
      )
      assert.doesNotMatch(error.message, /task1/)
      return true
    }
  )
})

test("wired and standalone configs pass", () => {
  write("vitest.config.mjs")
  write("vitest.task1.config.mjs")
  write("vitest.contract.config.mjs")

  assertEveryConfigRuns(root, ["vitest.task1.config.mjs"], {
    "vitest.contract.config.mjs": "pnpm test:contract"
  })
})

test("a test file claimed by two projects is named with both owners", () => {
  write("src/a.task1.test.ts")
  write("src/b.test.ts")

  assert.throws(
    () =>
      assertOneProjectPerFile(
        root,
        [
          { name: "task1", include: ["src/**/*.task1.test.ts"] },
          { name: "wide", include: ["src/**/*.test.ts"] }
        ],
        []
      ),
    /src\/a\.task1\.test\.ts: task1, wide/
  )
})

test("a test naming this machine's home directory is reported with its line", (t) => {
  const home = homedir().replaceAll("\\", "/")
  if (home.split("/").filter(Boolean).length < 2) {
    t.skip("this home directory is too short to be told apart from a fixture")
    return
  }
  write("src/ok.test.ts", 'const fixture = "/home/someone/else"\n')
  write("src/bad.test.ts", `\nconst fixture = "${home}/scratch/file"\n`)

  assert.throws(
    () => assertNoMachinePaths(root, ["src/**/*.test.ts"], []),
    (error) => {
      assert.match(error.message, /src\/bad\.test\.ts:2 names /)
      assert.doesNotMatch(error.message, /ok\.test\.ts/)
      assert.match(error.message, /os\.tmpdir\(\)/)
      return true
    }
  )
})

test("a build older than its sources names the rebuild command", () => {
  write("pkg/dist/index.js")
  write("pkg/src/index.ts")
  write("pkg/src/index.test.ts")
  const old = new Date(Date.now() - 60_000)
  utimesSync(path.join(root, "pkg/dist/index.js"), old, old)
  const builds = [
    {
      output: "pkg/dist/index.js",
      sources: ["pkg/src/**/*"],
      ignore: ["**/*.test.*"]
    }
  ]

  assert.throws(
    () => assertBuiltAfterSources(root, builds, "pnpm build:pkg"),
    (error) => {
      assert.match(
        error.message,
        /pkg\/dist\/index\.js is older than pkg\/src\/index\.ts$/m
      )
      assert.match(error.message, /Rebuild with `pnpm build:pkg`/)
      return true
    }
  )

  // Clearly after the source, so a filesystem that stores the source write
  // and `new Date()` in the same millisecond cannot make this flaky.
  const fresh = new Date(
    statSync(path.join(root, "pkg/src/index.ts")).mtimeMs + 60_000
  )
  utimesSync(path.join(root, "pkg/dist/index.js"), fresh, fresh)
  assertBuiltAfterSources(root, builds, "pnpm build:pkg")
})
