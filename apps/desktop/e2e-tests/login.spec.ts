import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import { expect, test } from "./fixtures/index.js"
import { getActualPath } from "./tests_helpers.js"

// No per-test timeout override is needed: the `authenticatedApp` fixture
// carries its own 180s setup budget, and these bodies only read state the
// fixture already produced.
test.describe("Authenticated app", () => {
  test("lands on the library after a full device-code enrollment", async ({
    authenticatedApp
  }) => {
    const { page, pageErrors } = authenticatedApp
    // `authenticatedApp` is worker-scoped and its `pageErrors` array lives
    // for the worker's whole life, so by the time this test runs it already
    // carries whatever every earlier spec file produced — CI's `workers: 1`
    // runs spec files in one worker in path order, and `instanceInstall.spec.ts`
    // (seven real installs) sorts before this file. Snapshotting the count
    // this test inherits and asserting only what it adds itself keeps a
    // renderer exception from an earlier install from being blamed on this
    // test.
    const inheritedErrorCount = pageErrors.length

    expect(getActualPath(page.url())).toBe("/library")
    await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
    expect(pageErrors.slice(inheritedErrorCount)).toEqual([])
  })

  test("walked the whole Microsoft chain rather than skipping it", async ({
    authenticatedApp
  }) => {
    // The point of the harness: enrollment is exercised, not bypassed. If a
    // future change seeds an account instead, these routes stop being hit.
    const seen = authenticatedApp.harness.mock.requests()

    expect(seen).toContain("GET /ms/consumers/oauth2/v2.0/devicecode")
    expect(seen).toContain("POST /ms/consumers/oauth2/v2.0/token")
    expect(seen).toContain("POST /xbl/user/authenticate")
    expect(seen).toContain("POST /xsts/xsts/authorize")
    expect(seen).toContain("POST /mc/authentication/login_with_xbox")
    expect(seen).toContain("GET /mc/entitlements/mcstore")
    expect(seen).toContain("GET /mc/minecraft/profile")
  })

  test("exchanged the provisioned GDL token", async ({ authenticatedApp }) => {
    expect(authenticatedApp.harness.mock.requests()).toContain(
      "POST /gdl/v1/auth/token"
    )
  })

  test("reached the real backend for the user profile", async ({
    authenticatedApp
  }) => {
    test.skip(
      authenticatedApp.harness.mode === "standalone",
      "needs TEST_BASE_API and E2E_INTERNAL_AUTH_TOKEN to reach api-test"
    )

    expect(authenticatedApp.harness.mock.requests()).toContain(
      "GET /gdl/v1/users/user"
    )
  })
})
