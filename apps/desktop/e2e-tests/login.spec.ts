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

    expect(getActualPath(page.url())).toBe("/library")
    await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
    // Asserted here only: `authenticatedApp` is worker-scoped, so
    // `pageErrors` accumulates across every test in this file, and asserting
    // it in more than one place would make failures depend on run order.
    // The login flow — device code, Xbox, XSTS, GDL sync — is where an
    // uncaught renderer exception is most likely.
    expect(pageErrors).toEqual([])
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
