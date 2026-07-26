import { pathToFileURL } from "node:url"
import {
  encodeMatrix,
  pickMatrix,
  type ManifestVersion
} from "./versionMatrix.js"

const MANIFEST_URL =
  "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json"

async function fetchManifest(): Promise<ManifestVersion[]> {
  const res = await fetch(MANIFEST_URL)
  if (!res.ok) {
    throw new Error(
      `version manifest fetch failed: ${res.status} ${res.statusText}`
    )
  }
  const body = (await res.json()) as { versions?: ManifestVersion[] }
  if (!Array.isArray(body.versions) || body.versions.length === 0) {
    throw new Error("version manifest contained no versions")
  }
  return body.versions
}

function resolveSeed(): number {
  const raw = process.env.E2E_VERSION_SEED
  if (raw === undefined || raw === "") {
    // Not crypto — this only needs to vary between runs and be printable.
    return Math.floor(Date.now() % 2147483647)
  }
  const parsed = Number.parseInt(raw, 10)
  if (!Number.isFinite(parsed)) {
    throw new Error(`E2E_VERSION_SEED is not an integer: ${raw}`)
  }
  return parsed
}

function resolveRandomCount(): number {
  const raw = process.env.E2E_VERSION_RANDOM_COUNT
  if (raw === undefined || raw === "") return 2
  const parsed = Number.parseInt(raw, 10)
  if (!Number.isFinite(parsed) || parsed < 0) {
    throw new Error(
      `E2E_VERSION_RANDOM_COUNT is not a non-negative integer: ${raw}`
    )
  }
  return parsed
}

async function globalSetup(): Promise<void> {
  const seed = resolveSeed()
  const randomCount = resolveRandomCount()
  const matrix = pickMatrix(await fetchManifest(), { seed, randomCount })

  process.env.E2E_VERSION_SEED = String(seed)
  process.env.E2E_VERSION_MATRIX = encodeMatrix(matrix)

  // Printed prominently: a failing run must be replayable from its output
  // alone via E2E_VERSION_SEED=<seed>.
  console.log(
    [
      "",
      "  e2e version matrix",
      `  seed: ${seed}   (replay with E2E_VERSION_SEED=${seed})`,
      ...matrix.map((e) => `    - ${e.id} (${e.source})`),
      ""
    ].join("\n")
  )
}

export default globalSetup

// Playwright imports this module for its `default` export and calls it
// itself — it never runs this file as the process entry point, so this
// branch is dead weight for that path and only fires when the file is
// invoked directly (`pnpm exec tsx e2e-tests/globalSetup.ts`), which is the
// supported way to inspect a matrix without paying for a full run. This is
// also the reason `playwright test --list` can't be used for that instead:
// list mode skips global setup entirely (see the README), so there is no
// other supported way to see the matrix short of running it.
//
// `pathToFileURL` rather than a manual `file://${...}` template: on Windows
// `process.argv[1]` is a backslash path (`C:\repo\...`) while
// `import.meta.url` is `file:///C:/repo/...` — a string-built comparison
// never matches there, which would silently restore the exact no-op this
// self-invocation exists to eliminate.
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  void globalSetup()
}
