import { decodeMatrix } from "./versionMatrix.js"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  createInstanceViaUi,
  deleteInstanceViaUi,
  waitForInstallComplete
} from "./helpers/instances.js"

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
        await expect(
          tile.locator(byTestId(TEST_IDS.instancePlay))
        ).toBeEnabled()
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
