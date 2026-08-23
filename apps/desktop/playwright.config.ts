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
  /* Backstop for the whole run, kept below the workflow's timeout-minutes so
     a wedged run is ended here — with a report — rather than by the runner,
     which kills the job and leaves no results at all. */
  globalTimeout: 2 * 60 * 60 * 1000,
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
  /* This suite drives real services: meta.gdl.gg, Mojang's CDNs, CurseForge
     and Modrinth, so a throttle or a dropped download reddens a 50-minute
     run. Two retries absorb that. A genuine breakage — the CurseForge CDN
     once began requiring an API key — fails all three attempts and still
     reports, so what gets hidden is transport, not behaviour. Retried tests
     are reported as flaky rather than passed: read that count, a rising one
     means something real is degrading. */
  retries: 2,
  /* One worker everywhere, and this is a correctness setting rather than a
     performance one — do not make it conditional.

     Two independent reasons. First, the suite's ordering invariants assume
     it: `fixtures/installedInstance.ts` documents three load-bearing
     dependencies that hold only because Playwright runs spec files in
     alphabetical order through a single worker. Spread the files across
     workers and each gets its own runtime path in its own order, so those
     guarantees quietly stop applying — the assertions they protect still
     run, and still pass, while no longer proving what they were written to
     prove.

     Second, every test here drives a real launcher against real CDNs. N
     workers means N packaged apps concurrently downloading Minecraft assets,
     JREs, libraries and mods. Measured on a 32-core host, where Playwright's
     default picked 9: three failures, all of them saturation
     (`UnknownHostException` thrown from inside a spawned Forge processor
     JVM, and a failed JRE download) — versus a fully green run serialized.
     It is not even a speed trade: 6.5 minutes at one worker against 4.8 at
     nine, because the contention costs back most of what the parallelism
     wins. */
  workers: 1,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: "html",
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Maximum time each action such as `click()` can take. Bounded rather
       than the 0 (no limit) default: with a 15-minute test timeout, an
       unbounded action on a missing anchor hangs for the full 15 minutes and
       is then abandoned by Playwright without running its `finally` —
       leaving the creation modal open over the shared worker-scoped app, so
       every remaining matrix entry hangs too. 60s is comfortably above the
       explicit waits already used for slow anchors (e.g. `dismissStartupModals`'s
       60s waits in fixtures/login.ts) while still failing fast on a genuinely
       missing one. */
    actionTimeout: 60_000,
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
