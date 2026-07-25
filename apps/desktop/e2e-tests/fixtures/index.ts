import { test as base, expect } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import { isCoreModulePresent, launchApp } from "./electronApp.js"
import { completeLogin } from "./login.js"
import { startHarness, stopHarness, type Harness } from "./mockIdp.js"

interface Fixtures {
  /** A launched app on a virgin runtime path with nobody logged in. */
  freshApp: { app: ElectronApplication; page: Page; harness: Harness }
}

interface WorkerFixtures {
  /** A logged-in app, shared by every test this worker runs. */
  authenticatedApp: { app: ElectronApplication; page: Page; harness: Harness }
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
        const { app, page } = await launchApp({
          runtimePath: harness.runtimePath,
          baseApi: `${harness.mock.url}/gdl`,
          e2eAuthBase: harness.mock.url,
          e2eEntitlementKey: harness.entitlementKeyPath
        })

        try {
          await use({ app, page, harness })
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
        const { app, page } = await launchApp({
          runtimePath: harness.runtimePath,
          baseApi: `${harness.mock.url}/gdl`,
          e2eAuthBase: harness.mock.url,
          e2eEntitlementKey: harness.entitlementKeyPath
        })

        try {
          await completeLogin(page, harness)
          await use({ app, page, harness })
        } finally {
          await app.close()
        }
      } finally {
        await stopHarness(harness)
      }
    },
    // Launch plus a full device-code enrollment, paid once per worker.
    { scope: "worker", timeout: 180_000 }
  ]
})

export { expect }
