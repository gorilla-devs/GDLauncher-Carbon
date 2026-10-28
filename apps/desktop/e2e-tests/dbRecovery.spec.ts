/**
 * The database recovery ladder, end to end: plant a damaged or
 * future-versioned `gdl_conf.db` with `helpers/dbSeed.ts` (cross-checked
 * against the real Rust classifier — see that file's own
 * header), launch the real packaged app against it, and assert **both**
 * halves of the contract every launch-time status relies on:
 *
 * 1. The core actually emitted the expected `_STATUS_:<EVENT>` line
 *    (`crates/carbon_app/src/managers/db_bootstrap.rs`'s `DbStatus` funnel —
 *    the single place that formats these).
 * 2. `apps/desktop/packages/main/index.ts` parsed that line and drove the
 *    *correct* rung of the recovery screen
 *    (`apps/desktop/packages/preload/loading.ts`'s `fatalError`/
 *    `backwardsMigrationError`).
 *
 * Asserting only the log line would pass even if the UI rendered nothing;
 * asserting only the UI would pass if the shell happened to guess right for
 * the wrong reason. Every state test below checks both.
 *
 * **Half 1's real channel, and why it isn't `app.process().stdout`.** The
 * brief for this task points at `fixtures/electronApp.ts` for an
 * already-captured core stdout to reuse. That capture (`launchApp`'s
 * `stdout` array, piping `app.process().stdout`) turns out to be racy for
 * exactly the events this file cares about most: `loadCoreModule()` spawns
 * the core at Electron main's *module scope* (`main/index.ts`), which runs
 * before Playwright's own `_electron.launch()` even resolves and hands back
 * a `process()` to attach a listener to — confirmed empirically: a first
 * attempt using this capture for `DB_CORRUPT`
 * (the fastest-failing state — the core can print its status line and exit
 * within milliseconds of spawning) reliably missed the line, while later,
 * slower main-process output (`"Window is ready to show"`) reliably arrived.
 * `getCoreModuleResult` below reads `window.getCoreModule()` instead — the
 * exact IPC call `main.tsx` itself makes to decide what to render, backed by
 * an *already-settled* promise on the main-process side
 * (`ipcMain.handle("getCoreModule", ...)`'s `await coreModule`), so calling
 * it from a test at any point after launch returns the authoritative result
 * with nothing to race. `main/index.ts` and the `Window.getCoreModule` type
 * (`mainWindow/src/global.d.ts`) were extended to carry `logs` on every
 * outcome, not only `"error"`, specifically to close this gap for
 * `DB_DOWNGRADED` (see below) — the one status this file needs that is
 * otherwise not observable once startup has moved on.
 * `launchApp`'s `stdout` capture is still used, later in this file, by the
 * Restart test: by then the app has been fully up for some time and the
 * listener has long since attached, so the race does not apply — confirmed
 * empirically for the line it is used for.
 *
 * **This screen is not the SolidJS app.** `fatalError`/
 * `backwardsMigrationError` build raw `innerHTML` from template strings in
 * the Electron preload and mount into `#appFatalCrash`
 * (`packages/mainWindow/index.html`), entirely outside Solid's render tree —
 * confirmed by reading `loading.ts` and `main.tsx`, not assumed. Neither
 * `helpers/selectors.ts` anchor hazard applies here: there is no `@gd/ui`
 * component to spread props onto (hazard 1), and this code path never goes
 * near `FilterSidebar` (hazard 2). Every `data-testid` this task adds was
 * read back from the real running app's DOM, not
 * inferred from the template source.
 *
 * **Six seedable states, six different outcomes:**
 *
 * | Seed                  | `_STATUS_:` event      | UI                                     |
 * |------------------------|-------------------------|-----------------------------------------|
 * | `DB_CORRUPT`           | `DB_CORRUPT`             | fatal screen, no restore-snapshot step  |
 * | `BACKWARDS_MIGRATION`  | `BACKWARDS_MIGRATION`    | the *other*, simpler screen — no retry/update steps |
 * | `DB_DIVERGED`          | `DB_DIVERGED`            | fatal screen, no restore-snapshot step  |
 * | `DB_DOWNGRADE_FAILED`  | `DB_DOWNGRADE_FAILED`    | fatal screen, no restore-snapshot step (see below) |
 * | `DB_DOWNGRADED`        | `DB_DOWNGRADED`          | *no* recovery screen — non-fatal, boots normally |
 * | `DB_MIGRATION_FAILED`  | `DB_MIGRATION_FAILED`    | fatal screen, no restore-snapshot step  |
 *
 * A seventh test seeds nothing — a genuinely healthy, unseeded first launch
 * — and asserts every one of those same checks goes negative against it
 * (`getCoreModule().type` reports `"success"`, not `"error"`; none of the
 * six `_STATUS_:` events above appear in a healthy boot's log). This is the
 * negative control: it is what proves the six tests above discriminate a
 * real failure state rather than passing by construction — a version of
 * this suite whose recovery assertions were unconditionally true would
 * still fail this one.
 *
 * **Why "Restore Previous Database" is asserted absent, never exercised.**
 * `loading.ts` only renders that step when `_STATUS_:DB_DOWNGRADE_FAILED`
 * carries a snapshot path, which `compat.rs`'s `snapshot_if_restorable`
 * grants only when the on-disk database differs from the pre-down-run
 * snapshot it just took. `down_run` (`crates/carbon_repos/src/compat.rs`)
 * runs every stored down under one transaction and rolls the *whole* thing
 * back on any failure — the no-stored-down branch our seed drives included —
 * so the database `down_run` failed against is, byte for byte, still the
 * snapshot it copied moments earlier; `snapshot_if_restorable` finds no
 * difference and reports no path, which is exactly what
 * `db_bootstrap.rs`'s own `downgrade_failed_line_omits_the_payload_when_there_is_no_snapshot`
 * test locks in. None of the six honest seeds `dbSeed.ts` can produce leaves
 * a differing snapshot behind, so this rung is asserted absent for the one
 * state it could ever apply to, not skipped.
 *
 * **Destructive actions are not clicked blindly.** "Reset Database" is
 * asserted present and enabled on every screen that offers it, never
 * clicked in the per-state tests. The one test that does click it
 * (`"Reset Database & Restart" deletes the database file for real`) says so
 * in its own name and runs against its own freshly seeded, throwaway
 * runtime path — never a path anything else in this file still needs.
 *
 * **"Retry" is exercised for real, once.** Every recovery button that acts
 * (`Restart`, `Reset Database & Restart`) ultimately calls
 * `app.relaunch(); app.exit()` in `main/index.ts` — a genuine OS-level
 * relaunch Playwright never mediated, not something this suite can silently
 * absorb. The retry test below drives it exactly once (rather than once per
 * state, which would multiply the same relaunch six times for the same code
 * path), observes outgoing main exit and a fresh main bootstrap generation
 * for the owned runtime/run with matching executable/start identity
 * (`waitForRelaunchMain`), and
 * cleans up the resulting sibling process itself: on Windows the relaunched
 * main the marker names, with its tree (`stopRelaunchedMainOnWindows`), on
 * Linux/macOS everything new under the harness's runtime path (`pgrep`,
 * best-effort — see `cleanupRelaunchSiblings`'s own comment). The reset
 * test relaunches the same way once more, after its "Database reset"
 * dialog, since that path reaches the relaunch only through the dialog.
 */

import fs from "node:fs"
import { execSync } from "node:child_process"
import path from "node:path"
import {
  captureRelaunchAttempt,
  stopRelaunchedMain,
  waitForRelaunchMain,
  type RelaunchAttempt
} from "./helpers/recoveryRelaunch.js"
import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  closeAppAndCore,
  getBinaryPath,
  getCoreModulePath,
  isCoreModulePresent,
  launchApp,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness, type Harness } from "./fixtures/mockIdp.js"
import { seedDatabase, type SeedState } from "./helpers/dbSeed.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  describeLauncherProcesses,
  isPidAlive,
  pidRuntimePathMatches
} from "./helpers/processes.js"

interface Launched {
  app: ElectronApplication
  page: Page
  pageErrors: Error[]
  stdout: string[]
}

interface CoreModuleLog {
  type: "info" | "error"
  message: string
}

interface CoreModuleResult {
  type: "success" | "error" | "backwardsMigration"
  logs?: CoreModuleLog[]
  snapshotPath?: string
  port?: string
  apiToken?: string
}

/**
 * Reads the exact value `main.tsx`'s own `createResource` used to decide
 * which screen to render (`window.getCoreModule()`, exposed by
 * `packages/preload/core_module_loader.ts`) — see the module doc comment for
 * why this, and not `app.process().stdout`, is Half 1's real channel here.
 * `ipcMain.handle("getCoreModule", ...)` awaits an already-settled,
 * module-scope promise on the main-process side, so this is a promise read,
 * not a live-stream capture: nothing about calling it later can lose data,
 * only hang if the core genuinely never finished starting up, hence the
 * explicit bound below.
 */
async function getCoreModuleResult(
  page: Page,
  timeoutMs = 90_000
): Promise<CoreModuleResult> {
  return Promise.race([
    page.evaluate(() => window.getCoreModule()) as Promise<CoreModuleResult>,
    new Promise<CoreModuleResult>((_, reject) =>
      setTimeout(
        () =>
          reject(
            new Error(
              `getCoreModuleResult: window.getCoreModule() did not settle within ${timeoutMs}ms`
            )
          ),
        timeoutMs
      )
    )
  ])
}

function coreModuleLogText(result: CoreModuleResult): string {
  return (result.logs ?? []).map((l) => l.message).join("\n")
}

function stdoutText(launched: Launched): string {
  return launched.stdout.join("")
}

/**
 * Polls the already-captured stdout (see the module doc comment — reliable
 * for a line logged well after the app is fully up) for `needle`. Bounded so a line that never arrives is a real,
 * diagnosable timeout rather than eating the whole 15-minute test ceiling.
 *
 * A hand-rolled poll rather than `expect.poll` deliberately: Playwright's
 * `message` option for `expect.poll` is a fixed string evaluated once, up
 * front, so it cannot report what actually accumulated in `stdout` by the
 * time the timeout fires — only a manual loop can attach the real captured
 * text to the thrown error, the same reasoning `electronApp.ts`'s own
 * `waitForPidExit` already applies to its pid check.
 */
async function waitForStdout(
  launched: Launched,
  needle: string,
  timeoutMs = 30_000
): Promise<void> {
  const deadline = Date.now() + timeoutMs
  for (;;) {
    if (stdoutText(launched).includes(needle)) return
    if (Date.now() >= deadline) {
      throw new Error(
        `waitForStdout: ${JSON.stringify(needle)} never appeared in the captured ` +
          `stdout within ${timeoutMs}ms. Captured so far:\n${stdoutText(launched)}`
      )
    }
    await new Promise((resolve) => setTimeout(resolve, 100))
  }
}

async function launchAgainstHarness(harness: Harness): Promise<Launched> {
  const opts: LaunchOptions = {
    runtimePath: harness.runtimePath,
    userDataPath: harness.userDataPath,
    runId: harness.runId,
    // Points the pre-DB-load terms/consent fetch (`AppInner::new`, which
    // runs before `load_and_migrate` on every launch, seeded state or not)
    // at the local mock rather than the real production API — same
    // reasoning as `fixtures/index.ts`'s `freshApp`. Never a real network
    // dependency for a suite that is entirely about the DB open path.
    baseApi: `${harness.mock.url}/gdl`,
    // Same reasoning one step further: without this the updater reads the
    // packaged `app-update.yml` and dials whatever is listening on port
    // 9000. These tests assert on the core's `_STATUS_:` stdout rather than
    // on the UI, so a failed check cannot fail them — it just means the one
    // spec here that needs no network at all is the one still depending on
    // what happens to hold a local port.
    e2eUpdateFeed: `${harness.mock.url}/updates/`
  }
  return launchApp(opts)
}

/**
 * The fatal screen's four `_STATUS_:` events — the ones `loading.ts`'s
 * `fatalError` renders. All four are asserted the same shape: no
 * restore-snapshot step (see the module doc comment), a retry button, and a
 * present-and-enabled (never clicked here) reset-database button.
 */
const FATAL_STATES: { seed: SeedState; event: string }[] = [
  { seed: "DB_CORRUPT", event: "DB_CORRUPT" },
  { seed: "DB_DIVERGED", event: "DB_DIVERGED" },
  { seed: "DB_DOWNGRADE_FAILED", event: "DB_DOWNGRADE_FAILED" },
  { seed: "DB_MIGRATION_FAILED", event: "DB_MIGRATION_FAILED" }
]

for (const { seed, event } of FATAL_STATES) {
  // Playwright's test callback always takes the fixtures object first; none
  // of this file's tests use the shared fixtures (each manages its own
  // harness/app lifecycle — see the module doc comment) — only the plain
  // `testInfo` second argument.
  // eslint-disable-next-line no-empty-pattern
  test(`seeding ${seed} drives the fatal recovery screen for _STATUS_:${event}`, async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()

    const harness = await startHarness()
    let launched: Launched | undefined
    try {
      await seedDatabase(harness.runtimePath, seed)
      launched = await launchAgainstHarness(harness)
      const { page } = launched

      // Half 1: the core's own `_STATUS_:` line, read race-free (see the
      // module doc comment).
      const coreResult = await getCoreModuleResult(page)
      expect(coreResult.type, "getCoreModule().type").toBe("error")
      expect(
        coreModuleLogText(coreResult),
        "the core's own reported startup log"
      ).toContain(`_STATUS_:${event}`)

      // Half 2: the shell rendered the fatal screen — not the backwards-
      // migration screen, not a silent boot to the login page.
      const fatalScreen = page.locator(byTestId(TEST_IDS.recoveryFatalScreen))
      await expect(fatalScreen).toBeVisible({ timeout: 60_000 })
      await expect(
        page.locator(byTestId(TEST_IDS.recoveryBackwardsMigrationScreen))
      ).toHaveCount(0)

      // The error detail box names the exact event `main/index.ts` resolved
      // — proves the shell parsed *this* event, not merely "some" fatal one.
      await expect(
        page.locator(byTestId(TEST_IDS.recoveryErrorDetail))
      ).toContainText(`Database error: ${event}`)

      // Non-destructive rungs: present and usable.
      const retryButton = page.locator(byTestId(TEST_IDS.recoveryRetryButton))
      await expect(retryButton).toBeVisible()
      await expect(retryButton).toBeEnabled()

      // See the module doc comment: none of the four seeds in this table
      // can honestly leave a differing pre-downgrade snapshot behind, so
      // the restore-snapshot rung must be genuinely absent, not merely
      // hidden by CSS.
      await expect(
        page.locator(byTestId(TEST_IDS.recoveryRestoreSnapshotButton))
      ).toHaveCount(0)

      // Destructive rung: presence and enablement only (module doc comment
      // — never clicked here).
      const resetButton = page.locator(byTestId(TEST_IDS.recoveryResetDbButton))
      await expect(resetButton).toBeVisible()
      await expect(resetButton).toBeEnabled()

      expect(launched.pageErrors, {
        message: "an uncaught renderer exception accompanied the fatal screen"
      }).toEqual([])
    } finally {
      if (launched) {
        await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
          () => {}
        )
        await closeAppAndCore(launched.app)
      }
      await stopHarness(harness)
    }
  })
}

// eslint-disable-next-line no-empty-pattern
test("seeding BACKWARDS_MIGRATION drives the dedicated version-mismatch screen, not the fatal one", async ({}, testInfo) => {
  expect(isCoreModulePresent()).toBeTruthy()

  const harness = await startHarness()
  let launched: Launched | undefined
  try {
    await seedDatabase(harness.runtimePath, "BACKWARDS_MIGRATION")
    launched = await launchAgainstHarness(harness)
    const { page } = launched

    // Half 1. `main/index.ts`'s parsing switch only ever resolves
    // `type: "backwardsMigration"` from the branch gated on
    // `event === "BACKWARDS_MIGRATION"`, where `event` is the exact token
    // parsed off the raw `_STATUS_:` line's own first `|`-delimited part —
    // so this equality check is already as strict as substring-matching the
    // raw line would be, just observed through the parsed result rather
    // than the text. (Unlike the fatal branch, this resolve path does not
    // thread the raw log text into anything the renderer can name
    // one-to-one against a captured substring beyond that.)
    const coreResult = await getCoreModuleResult(page)
    expect(coreResult.type, "getCoreModule().type").toBe("backwardsMigration")

    // Half 2.
    const backwardsScreen = page.locator(
      byTestId(TEST_IDS.recoveryBackwardsMigrationScreen)
    )
    await expect(backwardsScreen).toBeVisible({ timeout: 60_000 })

    // Proves the shell picked the *right* rung, not just *a* rung: the
    // fatal screen's own controls — retry, restore-snapshot, the error
    // detail box — must be genuinely absent, since `backwardsMigrationError`
    // never renders them.
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryFatalScreen))
    ).toHaveCount(0)
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryRetryButton))
    ).toHaveCount(0)
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryRestoreSnapshotButton))
    ).toHaveCount(0)

    // Destructive rung: presence and enablement only.
    const resetButton = page.locator(byTestId(TEST_IDS.recoveryResetDbButton))
    await expect(resetButton).toBeVisible()
    await expect(resetButton).toBeEnabled()

    expect(launched.pageErrors, {
      message:
        "an uncaught renderer exception accompanied the backwards-migration screen"
    }).toEqual([])
  } finally {
    if (launched) {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
        () => {}
      )
      await closeAppAndCore(launched.app)
    }
    await stopHarness(harness)
  }
})

// eslint-disable-next-line no-empty-pattern
test("seeding DB_DOWNGRADED is non-fatal: the core emits it and the app boots normally", async ({}, testInfo) => {
  expect(isCoreModulePresent()).toBeTruthy()

  const harness = await startHarness()
  let launched: Launched | undefined
  try {
    await seedDatabase(harness.runtimePath, "DB_DOWNGRADED")
    launched = await launchAgainstHarness(harness)
    const { page } = launched

    // Half 1: the core still reports the event — read off the `logs`
    // `getCoreModule()` now carries on the *success* path too (this task's
    // one addition to `main/index.ts` beyond the recovery-screen anchors;
    // see the module doc comment for why this state specifically needed
    // it: nothing else threads a non-fatal status line anywhere
    // observable once startup moves on to READY).
    const coreResult = await getCoreModuleResult(page)
    expect(coreResult.type, "getCoreModule().type").toBe("success")
    expect(
      coreModuleLogText(coreResult),
      "the core's own reported startup log"
    ).toContain("_STATUS_:DB_DOWNGRADED")

    // Half 2: ...but it is informational, not fatal — `main/index.ts` only
    // logs it and lets startup continue to READY. No recovery screen of
    // either shape ever mounts; the app reaches the ordinary login page
    // instead (same anchor `init.spec.ts` uses for a virgin runtime path —
    // this seed's schema is real but carries no account, so login is the
    // correct landing page, not a bug in the assertion).
    await expect(page.locator("#auth-flow")).toBeVisible({ timeout: 60_000 })
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryFatalScreen))
    ).toHaveCount(0)
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryBackwardsMigrationScreen))
    ).toHaveCount(0)

    expect(launched.pageErrors, {
      message:
        "an uncaught renderer exception happened during a downgraded-but-proceeding boot"
    }).toEqual([])
  } finally {
    if (launched) {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
        () => {}
      )
      await closeAppAndCore(launched.app)
    }
    await stopHarness(harness)
  }
})

/**
 * Every `_STATUS_:` event `db_bootstrap.rs`'s `DbStatus` funnel can emit for
 * a fatal or informational DB outcome — the full `SeedState` union
 * (`helpers/dbSeed.ts`) plus `READY`, which every seeded state above is
 * checked to NOT short-circuit into. Duplicated here as a literal list
 * rather than derived from `SeedState` itself: this test's whole point is a
 * negative control independent of the six seed functions, so it must not
 * share a definition with the thing it is cross-checking against.
 */
const ALL_SEEDABLE_STATUS_EVENTS = [
  "DB_CORRUPT",
  "BACKWARDS_MIGRATION",
  "DB_DIVERGED",
  "DB_DOWNGRADE_FAILED",
  "DB_DOWNGRADED",
  "DB_MIGRATION_FAILED"
] as const

// eslint-disable-next-line no-empty-pattern
test("a genuinely healthy, unseeded first launch reports success and none of the six recovery events — the negative control", async ({}, testInfo) => {
  expect(isCoreModulePresent()).toBeTruthy()

  const harness = await startHarness()
  let launched: Launched | undefined
  try {
    // Deliberately no `seedDatabase` call: `startHarness` mints a fresh,
    // empty `runtimePath` (`mockIdp.ts`), so this launch hits the real
    // fresh-install baseline path (`compat.rs`'s `install_baseline`) rather
    // than any of the six seeded terminal states above.
    launched = await launchAgainstHarness(harness)
    const { page } = launched

    // What proves the six tests above actually discriminate a real failure
    // state, rather than passing by construction: every one of those tests'
    // own checks — `getCoreModule().type`, the six `_STATUS_:` events, both
    // recovery screens — is run here too, against a database this test
    // knows is healthy, and every one of them must come back negative. A
    // version of this suite whose recovery assertions were unconditionally
    // true (e.g. `expect(coreResult.type).not.toBe("success")` mistyped as
    // always passing) would still fail this one.
    const coreResult = await getCoreModuleResult(page)
    expect(coreResult.type, "getCoreModule().type").toBe("success")
    const logText = coreModuleLogText(coreResult)
    for (const event of ALL_SEEDABLE_STATUS_EVENTS) {
      expect(
        logText,
        `a healthy boot's log unexpectedly contains _STATUS_:${event}`
      ).not.toContain(`_STATUS_:${event}`)
    }

    await expect(page.locator("#auth-flow")).toBeVisible({ timeout: 60_000 })
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryFatalScreen))
    ).toHaveCount(0)
    await expect(
      page.locator(byTestId(TEST_IDS.recoveryBackwardsMigrationScreen))
    ).toHaveCount(0)

    expect(launched.pageErrors, {
      message:
        "an uncaught renderer exception happened during a genuinely healthy boot"
    }).toEqual([])
  } finally {
    if (launched) {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
        () => {}
      )
      await closeAppAndCore(launched.app)
    }
    await stopHarness(harness)
  }
})

/**
 * Finds every currently-running process whose command line matches
 * `binaryPath` — `pgrep -f` on Linux/macOS, matching the full path (both
 * used by this suite's CI matrix). Windows has no command-line-matching
 * equivalent readily available, so `tasklist` filters by image name (the
 * basename) instead — coarser than `pgrep -f`, but the recovery-screen
 * binaries (`GDLauncher.exe`, `core_module.exe`) are only a coarse cleanup inventory. Neither same-binary matching nor a new
 * PID proves this harness relaunched; the Restart oracle uses owned markers.
 */
function pidsForBinary(binaryPath: string): number[] {
  if (process.platform === "win32") {
    try {
      const output = execSync(
        `tasklist /FI "IMAGENAME eq ${path.basename(binaryPath)}" /FO CSV /NH`,
        { encoding: "utf8" }
      )
      // CSV rows look like: "image.exe","1234","Console","1","12,345 K"
      // `tasklist` prints a plain "INFO: No tasks..." line (no leading
      // quote) instead of CSV when nothing matches — filtered out below.
      return output
        .split(/\r?\n/)
        .filter((line) => line.startsWith('"'))
        .map((line) => Number(line.split('","')[1]?.replace(/"/g, "")))
        .filter((n) => Number.isInteger(n) && n > 0)
    } catch {
      return []
    }
  }

  try {
    return execSync(`pgrep -f ${JSON.stringify(binaryPath)}`, {
      encoding: "utf8"
    })
      .split("\n")
      .map((s) => Number(s.trim()))
      .filter((n) => Number.isInteger(n) && n > 0)
  } catch {
    // pgrep exits 1 with no output when nothing matches — not an error here.
    return []
  }
}

/* `pidRuntimePathMatches` and `isPidAlive` live in `helpers/processes.js`,
   shared with `stopHarness`'s game-JVM sweep. `pidsForBinary` above stays
   local: it is the only one of the three with a Windows (`tasklist`) path,
   which the shared module deliberately does not have. */

/**
 * `app.relaunch()` (used by every button that acts on the recovery screen)
 * spawns a brand new OS process this harness never launched through
 * `_electron.launch()` and has no Playwright handle to — Electron re-execs
 * the binary directly. Clicking a real button for real (deliberately done
 * only twice in this file — see the module doc comment) unavoidably leaves
 * that sibling running. This sweeps up anything matching the app or core
 * binary path that was not part of `before` (the pre-*launch* snapshot, not
 * the outgoing main identity `waitForRelaunchMain` uses — this function's job is
 * "nothing from this whole test is still running", a superset of "the
 * relaunch sibling specifically"), so a real test run does not leave an idle
 * GUI process behind on a developer's own machine.
 *
 * Polls until the found set stops growing for a full second, rather than
 * returning on the first match: Electron's own process tree spawns its
 * zygote/gpu/renderer/utility helpers staggered over time, so stopping at
 * the first sighting can capture only an early straggler and leave later
 * siblings — including the actual relaunched core, which is what this
 * function exists to catch — still running. `found` accumulates across
 * every poll rather than being recomputed from scratch each time, so a pid
 * that briefly appears and is gone by the next scan is still swept.
 *
 * Narrowed by `runtimePath` via `pidRuntimePathMatches` before anything is
 * killed: `before` alone only proves a pid is new *to this test*, not that
 * it belongs to this test's own harness rather than some other worker's app
 * — the runtime path is what actually distinguishes them. A pid whose
 * environment cannot be read (see `pidRuntimePathMatches`) is left alone
 * rather than killed on the assumption it's ours.
 *
 * Best-effort and Linux/macOS-only (`pgrep`/`ps`/`kill` are Unix tools with
 * no Windows equivalent wired up here): a miss here never fails the test.
 * Windows is covered by `stopRelaunchedMainOnWindows` instead.
 */
async function cleanupRelaunchSiblings(
  before: Set<number>,
  runtimePath: string
): Promise<void> {
  if (process.platform === "win32") return

  const deadline = Date.now() + 10_000
  // Once at least one match is found, keep polling until a full second has
  // passed with no *new* pid appearing — long enough for a staggered helper
  // process to show up, short enough not to burn the whole 10s budget once
  // things have genuinely settled.
  const settleWindowMs = 1_000
  const targets = [getBinaryPath(), getCoreModulePath()]
  const found = new Set<number>()
  let lastGrowthAt = Date.now()

  while (Date.now() < deadline) {
    for (const binaryPath of targets) {
      for (const pid of pidsForBinary(binaryPath)) {
        if (!before.has(pid) && !found.has(pid)) {
          found.add(pid)
          lastGrowthAt = Date.now()
        }
      }
    }
    if (found.size > 0 && Date.now() - lastGrowthAt >= settleWindowMs) break
    await new Promise((resolve) => setTimeout(resolve, 250))
  }

  for (const pid of found) {
    // Chromium's own zygote/gpu-process helpers churn on this environment
    // (spawn, crash, retry) fast enough that `pidsForBinary` routinely
    // catches one mid-flight that has already exited by the time this loop
    // reaches it. Checking liveness first, rather than going straight to
    // `pidRuntimePathMatches`, keeps this function's log free of "leaving
    // pid running" noise for pids that were never actually a cleanup
    // candidate in the first place.
    if (!isPidAlive(pid)) continue

    if (!pidRuntimePathMatches(pid, runtimePath)) {
      console.log(
        `cleanupRelaunchSiblings: leaving pid ${pid} running — its ` +
          `environment doesn't confirm it belongs to this harness's own ` +
          `runtime path (${runtimePath})`
      )
      continue
    }
    try {
      process.kill(pid, "SIGKILL")
      console.log(
        `cleanupRelaunchSiblings: killed leaked relaunch sibling pid ${pid}`
      )
    } catch {
      // Already gone by the time we got here — fine.
    }
  }

  // A relaunch that finds a healthy (fresh) database starts a core that never
  // exits on its own, and it can start just after the settle window above. It
  // outlives the killed main (its parent-death keepalive is off), so sweep the
  // core once more, still only by this harness's runtime path.
  const coreDeadline = Date.now() + 3_000
  while (Date.now() < coreDeadline) {
    for (const pid of pidsForBinary(getCoreModulePath())) {
      if (before.has(pid) || !isPidAlive(pid)) continue
      if (!pidRuntimePathMatches(pid, runtimePath)) continue
      try {
        process.kill(pid, "SIGKILL")
        console.log(
          `cleanupRelaunchSiblings: killed relaunched core pid ${pid}`
        )
      } catch {
        // Already gone — fine.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
}

/**
 * Windows: stops the main the click relaunched, with its core and Chromium
 * helpers. Playwright did not start it, so nothing else ends it.
 *
 * Run 41652440682 left them running:
 * - Each kept files open in the harness's runtime folder (its userData lives
 *   there), so `stopHarness` failed with EPERM.
 * - Once the one relaunched after the database reset was up (on the login
 *   screen, which loops a full-screen background video), the runner stayed
 *   busy for the rest of the job: app startup went from about 0.2 s to over
 *   1 s. The e2e build's startup identity probe then timed out in every
 *   later launch ("Cannot identify E2E main startup").
 *
 * Only a main the bootstrap marker attributes to this test's outgoing main
 * (`attempt.before`), and whose live identity still matches, is stopped
 * (`stopRelaunchedMain`). The process list logged first shows its CPU time.
 */
async function stopRelaunchedMainOnWindows(
  attempt: RelaunchAttempt | undefined
): Promise<void> {
  if (process.platform !== "win32" || !attempt) return
  console.log(
    "stopRelaunchedMain: launcher processes before the stop:\n" +
      describeLauncherProcesses([getBinaryPath(), getCoreModulePath()])
  )
  const stopped = await stopRelaunchedMain(attempt.before)
  if (!stopped) {
    console.log(
      "stopRelaunchedMain: no live main this test relaunched was found"
    )
    return
  }
  console.log(
    `stopRelaunchedMain: ended relaunched main ${stopped.main} and its ` +
      `tree (${stopped.killed.join(", ")})` +
      (stopped.survivors.length > 0
        ? `; still running: ${stopped.survivors.join(", ")}`
        : "")
  )
}

// eslint-disable-next-line no-empty-pattern
test("clicking Restart genuinely restarts the app, not just a UI no-op", async ({}, testInfo) => {
  expect(isCoreModulePresent()).toBeTruthy()

  const beforePids = new Set([
    ...pidsForBinary(getBinaryPath()),
    ...pidsForBinary(getCoreModulePath())
  ])

  const harness = await startHarness()
  let launched: Launched | undefined
  let attempt: RelaunchAttempt | undefined
  try {
    await seedDatabase(harness.runtimePath, "DB_CORRUPT")
    launched = await launchAgainstHarness(harness)
    const { page } = launched
    await getCoreModuleResult(page) // wait for the fatal screen's data to be ready

    const retryButton = page.locator(byTestId(TEST_IDS.recoveryRetryButton))
    await expect(retryButton).toBeVisible({ timeout: 60_000 })
    await expect(retryButton).toBeEnabled()

    // Register outgoing exit before the action; the owned main bootstrap
    // marker must match this exact main process and harness. Its pid is read
    // from inside it: on Windows `app.process()` is the shell Playwright
    // starts Electron through, not Electron itself.
    const current = captureRelaunchAttempt(
      launched.app.process(),
      harness,
      await launched.app.evaluate(() => process.pid)
    )
    attempt = current

    try {
      await retryButton.click()

      // `ipcMain.handle("relaunch", ...)` (`main/index.ts`) logs this exact
      // line before calling `killCoreProcess()`, `app.relaunch()`, and
      // `app.exit()`. Waiting for it proves the click's full round trip —
      // renderer button, preload listener, ipcMain handler — reached the real
      // handler. It does NOT by itself prove the relaunch happened: the log
      // line is printed before any of those three calls, so a handler that
      // logged it and then threw or returned early would satisfy this alone.
      await waitForStdout(launched, "relaunching app...")

      await waitForRelaunchMain(current)
    } finally {
      current.dispose()
    }
  } finally {
    if (launched) {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
        () => {}
      )
      await closeAppAndCore(launched.app)
    }
    await stopRelaunchedMainOnWindows(attempt)
    await cleanupRelaunchSiblings(beforePids, harness.runtimePath)
    await stopHarness(harness)
  }
})

interface ResetDialog {
  title?: string
  message: string
  detail: string
  buttons: string[]
}

/**
 * A completed reset ends on a native "Database reset" message box that names
 * the recovery folder and relaunches once the user presses its only button,
 * Restart. Playwright cannot press a native dialog, so for this app only the
 * main process's `dialog.showMessageBox` records what it was asked to show
 * and holds Restart until `pressResetDialogRestart`.
 */
async function stubResetDialog(app: ElectronApplication) {
  await app.evaluate(({ dialog }) => {
    const shown: unknown[] = []
    let press!: () => void
    const pressed = new Promise<void>((resolve) => (press = resolve))
    Object.assign(globalThis, {
      __e2eResetDialogs: shown,
      __e2ePressResetDialog: press
    })
    dialog.showMessageBox = (async (...args: unknown[]) => {
      // `(options)` or `(window, options)`.
      const options = args[args.length - 1] as Record<string, unknown>
      shown.push({
        title: options.title,
        message: options.message,
        detail: options.detail,
        buttons: options.buttons
      })
      await pressed
      return { response: 0, checkboxChecked: false }
    }) as typeof dialog.showMessageBox
  })
}

function resetDialogs(app: ElectronApplication): Promise<ResetDialog[]> {
  return app.evaluate(
    () =>
      ((globalThis as { __e2eResetDialogs?: unknown[] }).__e2eResetDialogs ??
        []) as ResetDialog[]
  )
}

/** Presses Restart just after this call returns, so the evaluate's answer
 *  arrives before the app exits. */
async function pressResetDialogRestart(app: ElectronApplication) {
  await app.evaluate(() => {
    const press = (globalThis as { __e2ePressResetDialog?: () => void })
      .__e2ePressResetDialog
    if (!press) throw new Error("the reset dialog stub is not installed")
    setTimeout(press, 100)
  })
}

// eslint-disable-next-line no-empty-pattern
test('"Reset Database & Restart" deletes the database file for real, on its own throwaway runtime path', async ({}, testInfo) => {
  expect(isCoreModulePresent()).toBeTruthy()

  const beforePids = new Set([
    ...pidsForBinary(getBinaryPath()),
    ...pidsForBinary(getCoreModulePath())
  ])

  // A fresh harness mints its own fresh temp `runtimePath`
  // (`mockIdp.ts`'s `startHarness`) — nothing else in this file, or in any
  // other spec, still needs this directory once this test is done with it.
  const harness = await startHarness()
  let launched: Launched | undefined
  let attempt: RelaunchAttempt | undefined
  try {
    await seedDatabase(harness.runtimePath, "DB_CORRUPT")
    const dbPath = `${harness.runtimePath}/gdl_conf.db`
    expect(
      fs.existsSync(dbPath),
      "setup: the seeded corrupt database must exist before Reset is exercised"
    ).toBe(true)

    launched = await launchAgainstHarness(harness)
    const { page } = launched
    await getCoreModuleResult(page) // wait for the fatal screen's data to be ready

    const resetButton = page.locator(byTestId(TEST_IDS.recoveryResetDbButton))
    await expect(resetButton).toBeVisible({ timeout: 60_000 })
    await expect(resetButton).toBeEnabled()

    const seeded = fs.readFileSync(dbPath)
    await stubResetDialog(launched.app)

    await resetButton.click()

    // `deleteDbAndRestart` (`main/index.ts`, via `utils/databaseReset.ts`)
    // stops the core (already exited here after `_STATUS_:DB_CORRUPT`),
    // copies every database file into a recovery folder, removes them, and
    // only then asks for this dialog. So the dialog being asked for proves
    // the click reached the real handler and the reset completed.
    await expect
      .poll(async () => (await resetDialogs(launched!.app)).length, {
        timeout: 30_000
      })
      .toBe(1)
    const [shown] = await resetDialogs(launched.app)
    expect(shown.title).toBe("Database reset")
    expect(shown.buttons).toEqual(["Restart"])
    const recoveryPath = /^Recovery files: (.+)$/m.exec(shown.detail)?.[1]
    expect(recoveryPath, "the dialog names the recovery folder").toBeTruthy()

    // The real, on-disk proof: the corrupt file this test seeded is gone from
    // the runtime path, and the folder the dialog names, inside that same
    // runtime path, keeps it byte for byte. The dialog spells that path as
    // its real location, which on macOS (`/var` links to `/private/var`) and
    // on Windows (`RUNNER~1` expands to the full name) differs from the
    // spelling this harness was handed, so both sides are resolved first.
    expect(fs.existsSync(dbPath)).toBe(false)
    expect(fs.realpathSync.native(path.dirname(recoveryPath!))).toBe(
      fs.realpathSync.native(harness.runtimePath)
    )
    expect(
      fs.readFileSync(path.join(recoveryPath!, "gdl_conf.db")).equals(seeded),
      "the recovery folder keeps the database exactly as it was"
    ).toBe(true)

    // Pressing Restart then genuinely relaunches, as the retry test proves
    // for the Restart step.
    const current = captureRelaunchAttempt(
      launched.app.process(),
      harness,
      await launched.app.evaluate(() => process.pid)
    )
    attempt = current
    try {
      await pressResetDialogRestart(launched.app)
      await waitForRelaunchMain(current)
    } finally {
      current.dispose()
    }
  } finally {
    if (launched) {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
        () => {}
      )
      await closeAppAndCore(launched.app)
    }
    await stopRelaunchedMainOnWindows(attempt)
    await cleanupRelaunchSiblings(beforePids, harness.runtimePath)
    await stopHarness(harness)
  }
})
