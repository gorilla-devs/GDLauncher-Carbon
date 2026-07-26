import { decodeMatrix } from "./versionMatrix.js"
import { expect, test } from "./fixtures/index.js"
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
  for (const entry of MATRIX) {
    test(`installs Minecraft ${entry.id} (${entry.source}, seed ${SEED})`, async ({
      authenticatedApp
    }) => {
      const { page } = authenticatedApp
      const name = `gdl-e2e-${entry.id}`

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
      } finally {
        // Deliberately NOT swallowed. Cleanup runs against shared, worker-
        // scoped state, so a silent failure here does not stay local — it
        // corrupts the starting conditions of every later test in the worker
        // and resurfaces as an unrelated first-click timeout somewhere else.
        await deleteInstanceViaUi(page, name)
      }
    })
  }
})
