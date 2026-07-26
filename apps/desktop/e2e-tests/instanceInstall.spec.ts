import { DatabaseSync } from "node:sqlite"
import path from "node:path"
import { decodeMatrix } from "./versionMatrix.js"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  createInstanceViaUi,
  deleteInstanceViaUi,
  ensureLibraryInteractive,
  waitForInstallComplete
} from "./helpers/instances.js"
import { verifyAssetIndex, verifyClientJar } from "./helpers/installVerify.js"

const raw = process.env.E2E_VERSION_MATRIX
if (!raw) {
  throw new Error(
    "E2E_VERSION_MATRIX is unset — globalSetup did not run. " +
      "Run through `playwright test`, not by importing this spec directly."
  )
}
const MATRIX = decodeMatrix(raw)
const SEED = process.env.E2E_VERSION_SEED ?? "<unset>"

/** The subset of Mojang's version JSON this spec needs off of it. */
interface CachedVersionInfo {
  assetIndex?: { id?: string }
  downloads?: { client?: { sha1?: string } }
}

/**
 * Reads the version JSON the app already downloaded for `versionId`, straight
 * off disk — never re-fetched from the network, so verification never depends
 * on a second source (Mojang again, this time from the test) that could
 * disagree with what actually got installed for reasons unrelated to the
 * install itself.
 *
 * The core does not write this JSON as a loose file under the runtime path;
 * it caches the exact response bytes it fetched in the `VersionInfoCache`
 * table of the runtime's own `gdl_conf.db` (see `get_version`'s
 * `version_meta::upsert_version_info` call in
 * `crates/carbon_app/src/managers/minecraft/minecraft.rs`, and the table
 * shape in `crates/carbon_repos/src/repos/version_meta.rs`). That db is
 * opened WAL-mode by the core (`crates/carbon_repos/src/db_exec.rs`), so a
 * separate read-only connection from here is safe to open concurrently.
 *
 * `assetIndex.id` (not the sibling `assets` string field — same value on a
 * well-formed manifest, but `assetIndex.id` is what the core actually names
 * the cached index file after, in `assets.rs`'s `get_assets_dir`) is the
 * asset index id for `verifyAssetIndex`: it is never the version id (e.g.
 * live Mojang data has 1.20.1 sharing bare numeric id `"5"` with a run of
 * other releases, and 1.12.2/1.16.5 sharing the minor-only `"1.12"`/`"1.16"`),
 * so this always reads the real value off the cached JSON rather than
 * assuming one. (1.7.10 itself resolves to plain id `"1.7.10"` — the shared
 * id literally spelled `legacy` currently belongs to 1.6.x, one major
 * version older than this matrix's oldest pinned entry; verified directly
 * against Mojang's live meta while building this. Whichever version(s) it
 * belongs to, `verifyAssetIndex` still branches on the index JSON's own
 * `"virtual"` field, never on the id string, so this is not load-bearing —
 * noted here only because README.md's "legacy vs. modern" framing for why
 * 1.7.10 is pinned describes the id, not the mechanism.)
 */
function readCachedVersionInfo(
  runtimePath: string,
  versionId: string
): CachedVersionInfo {
  const dbPath = path.join(runtimePath, "gdl_conf.db")
  const db = new DatabaseSync(dbPath, { readOnly: true })
  try {
    const row = db
      .prepare("SELECT versionInfo FROM VersionInfoCache WHERE id = ?")
      .get(versionId)

    if (!row) {
      throw new Error(
        `no cached version JSON for "${versionId}" in ${dbPath} ` +
          "(table VersionInfoCache) — the app never downloaded it, or the " +
          "cache key does not match the version id"
      )
    }

    const versionInfo = row.versionInfo
    if (!(versionInfo instanceof Uint8Array)) {
      throw new Error(
        `VersionInfoCache.versionInfo for "${versionId}" in ${dbPath} is not ` +
          `a blob (got ${typeof versionInfo}) — cache row is malformed`
      )
    }

    return JSON.parse(
      Buffer.from(versionInfo).toString("utf8")
    ) as CachedVersionInfo
  } finally {
    db.close()
  }
}

test.describe("instance install", () => {
  // `authenticatedApp` is worker-scoped, so it never receives a per-test
  // `TestInfo` to gate an attachment on (see `attachCoreLogOnFailure`'s
  // doc comment) — `afterEach` is the one hook Playwright runs per test
  // that still gets both the worker fixture's value and a real `TestInfo`.
  test.afterEach(async ({ authenticatedApp }, testInfo) => {
    await attachCoreLogOnFailure(testInfo, authenticatedApp.harness.runtimePath)
    // Restores a known-good, interactive library so one entry timing out on
    // a missing anchor (abandoned by Playwright before its own `finally`
    // runs — see playwright.config.ts's `actionTimeout` comment) cannot leave
    // the creation modal stranded over the shared worker-scoped app and
    // cascade into every remaining matrix entry hanging behind it.
    await ensureLibraryInteractive(authenticatedApp.page)
  })

  for (const entry of MATRIX) {
    test(`installs Minecraft ${entry.id} (${entry.source}, seed ${SEED})`, async ({
      authenticatedApp
    }) => {
      const { page } = authenticatedApp
      const name = `gdl-e2e-${entry.id}`

      // `bodyFailed` records whether the try-block itself failed. A `throw`
      // inside `finally` discards whatever the try-block was throwing — JS
      // semantics, not a Playwright reporting choice — so a cleanup failure
      // must never re-throw over an already-failing body, only over a
      // passing one. An explicit boolean rather than an "error is undefined"
      // sentinel: a literal `throw undefined` from the body would otherwise
      // be misread as "the body succeeded" (same reasoning as
      // `hasFirstError` in `fixtures/mockIdp.ts`'s `stopHarness`).
      let bodyFailed = false
      try {
        await createInstanceViaUi(page, { name, version: entry.id })
        await waitForInstallComplete(page, name)

        // Ready to play. Deliberately not clicked: mocked accounts carry a
        // mock entitlement that real Minecraft rejects, so launching would
        // assert on a failure rather than on success.
        const tile = page.locator(byInstanceName(name))
        await expect(tile).toHaveAttribute("data-instance-state", "inactive")
        await expect(tile).not.toHaveAttribute("data-instance-failed", "true")

        // The play control lives on the tile itself in the library grid
        // (Task 3 anchored it on BaseTile's play button), so this asserts
        // without navigating. Do NOT click through to the instance detail
        // page: the fixture is worker-scoped, and leaving the app on that
        // route strands every later test in the same worker on a page where
        // the library header does not exist.
        //
        // `toBeVisible()`, not `toBeEnabled()`: the control is a plain `div`
        // (BaseTile/index.tsx has no `aria-disabled`), and Playwright treats
        // any non-form element without one as always enabled — that
        // assertion could never fail. Presence *is* the assertion here: the
        // `<Show>` this div lives under only mounts it when
        // `!isLoadingOrWaiting() && !isDeleting && !isInvalid && !failError`
        // (BaseTile/index.tsx), so its visibility is the real "ready to
        // play" signal.
        await expect(
          tile.locator(byTestId(TEST_IDS.instancePlay))
        ).toBeVisible()

        // The app believes it installed. Now prove the files it says it put
        // on disk are actually there and correct, independent of anything it
        // reported through the UI.
        const cachedVersion = readCachedVersionInfo(
          authenticatedApp.harness.runtimePath,
          entry.id
        )
        const assetIndexId = cachedVersion.assetIndex?.id
        const expectedSha1 = cachedVersion.downloads?.client?.sha1

        if (!assetIndexId || !expectedSha1) {
          throw new Error(
            `cached version JSON for "${entry.id}" is missing ` +
              `assetIndex.id or downloads.client.sha1 — cannot verify the ` +
              "install on disk"
          )
        }

        const [clientJarResult, assetIndexResult] = await Promise.all([
          verifyClientJar(
            authenticatedApp.harness.runtimePath,
            entry.id,
            expectedSha1
          ),
          verifyAssetIndex(authenticatedApp.harness.runtimePath, assetIndexId)
        ])

        const problems = [
          ...clientJarResult.problems,
          ...assetIndexResult.problems
        ]
        if (problems.length > 0) {
          throw new Error(
            `disk verification failed for Minecraft ${entry.id}:\n` +
              problems.map((problem) => `  - ${problem}`).join("\n")
          )
        }
      } catch (error) {
        bodyFailed = true
        throw error
      } finally {
        try {
          await deleteInstanceViaUi(page, name)
        } catch (cleanupError) {
          // Cleanup corrupts shared worker state, so it must not pass
          // silently — but it must not bury the failure that caused it
          // either. Re-throw only when the body itself succeeded.
          if (!bodyFailed) {
            // Deliberate: this branch only runs when the try-block
            // succeeded, so there is no try-block error here for the throw
            // to discard.
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(`cleanup for "${name}" also failed:`, cleanupError)
        }
      }
    })
  }
})
