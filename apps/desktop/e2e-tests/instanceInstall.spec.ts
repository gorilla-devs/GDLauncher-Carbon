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
import { readVersionInfo } from "./helpers/versionCache.js"

const raw = process.env.E2E_VERSION_MATRIX
if (!raw) {
  throw new Error(
    "E2E_VERSION_MATRIX is unset — globalSetup did not run. " +
      "Run through `playwright test`, not by importing this spec directly."
  )
}
const MATRIX = decodeMatrix(raw)
const SEED = process.env.E2E_VERSION_SEED ?? "<unset>"

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
        const cachedVersion = readVersionInfo(
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
