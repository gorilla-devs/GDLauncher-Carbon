import type { PlaywrightTestConfig } from "@playwright/test"

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// require('dotenv').config();

/**
 * See https://playwright.dev/docs/test-configuration.
 */
const config: PlaywrightTestConfig = {
  testDir: "./e2e-tests",
  /* `e2e-tests/` also holds vitest unit specs (`*.test.ts`), which import
     vitest's `expect`. Playwright's default testMatch would collect those
     files too, and the two `expect` implementations collide defining the
     same `$$jest-matchers-object` global, aborting collection before any
     test runs — so this narrows collection to Playwright specs only. */
  testMatch: "**/*.spec.ts",
  /* Resolves the seeded version matrix once per run and hands it to workers
     via process.env — see e2e-tests/globalSetup.ts. */
  globalSetup: "./e2e-tests/globalSetup.ts",
  /* Maximum time one test *body* can run for. A full install-and-launch
     against a real version can legitimately take most of this; 15 minutes
     is the project's hard ceiling for a single test. Fixture setup is
     budgeted separately — see e2e-tests/fixtures/index.ts. */
  timeout: 15 * 60 * 1000,
  /* Backstop for the whole run: worst case is one 15-minute test per matrix
     entry plus the login fixtures. This is what stops a wedged run holding
     a CI runner indefinitely. */
  globalTimeout: 3 * 60 * 60 * 1000,
  expect: {
    /**
     * Maximum time expect() should wait for the condition to be met.
     * For example in `await expect(locator).toHaveText();`
     */
    timeout: 15000
  },
  /* Run tests in files in parallel */
  fullyParallel: false,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: "html",
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Maximum time each action such as `click()` can take. Defaults to 0 (no limit). */
    actionTimeout: 0,
    /* Base URL to use in actions like `await page.goto('/')`. */
    // baseURL: 'http://localhost:3000',

    /* Capture screenshot on failure */
    screenshot: "only-on-failure",

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: {
      mode: "retain-on-failure",
      screenshots: true,
      snapshots: true,
      sources: true
    }
  },

  /* Folder for test artifacts such as screenshots, videos, traces, etc. */
  outputDir: "test-results/"

  /* Run your local dev server before starting the tests */
  // webServer: {
  //   command: 'npm run start',
  //   port: 3000,
  // },
}

export default config
