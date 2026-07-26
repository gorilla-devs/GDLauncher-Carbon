import { test as base, expect } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import { isCoreModulePresent, launchApp } from "./electronApp.js"
import {
  installFixtureInstance,
  type InstalledInstance
} from "./installedInstance.js"
import { completeLogin, dismissStartupModals } from "./login.js"
import { startHarness, stopHarness, type Harness } from "./mockIdp.js"

interface Fixtures {
  /** A launched app on a virgin runtime path with nobody logged in. */
  freshApp: {
    app: ElectronApplication
    page: Page
    harness: Harness
    pageErrors: Error[]
  }
}

interface WorkerFixtures {
  /** A logged-in app, shared by every test this worker runs. */
  authenticatedApp: {
    app: ElectronApplication
    page: Page
    harness: Harness
    pageErrors: Error[]
  }
  /** A logged-in app with one warm, already-installed Fabric instance,
   *  shared by every mod test this worker runs. See
   *  `fixtures/installedInstance.ts`. */
  installedInstance: InstalledInstance
}

export const test = base.extend<Fixtures, WorkerFixtures>({
  freshApp: [
    // Playwright fixtures take the accumulated fixture object first; this
    // fixture depends on none.
    // eslint-disable-next-line no-empty-pattern
    async ({}, use) => {
      expect(isCoreModulePresent()).toBeTruthy()

      const harness = await startHarness()
      try {
        const { app, page, pageErrors } = await launchApp({
          runtimePath: harness.runtimePath,
          baseApi: `${harness.mock.url}/gdl`,
          e2eAuthBase: harness.mock.url,
          e2eEntitlementKey: harness.entitlementKeyPath
        })

        try {
          await use({ app, page, harness, pageErrors })
        } finally {
          await app.close()
        }
      } finally {
        await stopHarness(harness)
      }
    },
    // Launching the packaged app alone outruns the 30s test-body budget.
    { timeout: 120_000 }
  ],

  authenticatedApp: [
    // Playwright fixtures take the accumulated fixture object first; this
    // fixture depends on none.
    // eslint-disable-next-line no-empty-pattern
    async ({}, use) => {
      expect(isCoreModulePresent()).toBeTruthy()

      const harness = await startHarness()
      try {
        const { app, page, pageErrors } = await launchApp({
          runtimePath: harness.runtimePath,
          baseApi: `${harness.mock.url}/gdl`,
          e2eAuthBase: harness.mock.url,
          e2eEntitlementKey: harness.entitlementKeyPath
        })

        try {
          await completeLogin(page, harness)
          await dismissStartupModals(page)
          await use({ app, page, harness, pageErrors })
        } finally {
          await app.close()
        }
      } finally {
        await stopHarness(harness)
      }
    },
    // Launch plus a full device-code enrollment, paid once per worker.
    { scope: "worker", timeout: 180_000 }
  ],

  installedInstance: [
    async ({ authenticatedApp }, use) => {
      const installed = await installFixtureInstance(authenticatedApp)
      await use(installed)
      // No teardown here, deliberately: the instance is torn down along with
      // the rest of `authenticatedApp.harness.runtimePath` when that
      // worker-scoped fixture's own `finally` removes it — staying warm
      // across every test in the worker is this fixture's entire point.
    },
    // Depends on `authenticatedApp` (login plus enrollment, up to 180s) and
    // adds a real Fabric install on top of it — generous next to the ~8s a
    // Fabric install measures at once the substrate (assets/libraries/JRE)
    // is warm, but a cold worker also pays for that substrate the first
    // time, same as `instanceInstall.spec.ts`'s own first-install cost.
    { scope: "worker", timeout: 180_000 }
  ]
})

export { expect }
