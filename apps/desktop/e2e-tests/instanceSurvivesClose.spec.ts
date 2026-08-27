/**
 * Proves a running game outlives the launcher.
 *
 * Closing the launcher aimed to end the user's session: `main.rs`'s
 * termination handler sent `kill_tx` to every running instance, and the game
 * command carried `kill_on_drop(true)`. The comment stated it as a product
 * decision — *"the game is tied to the launcher's lifecycle just like a
 * server"*. A game is the user's session, not launcher-owned infrastructure,
 * so both are gone; servers keep that behaviour deliberately.
 *
 * **What it took to watch this fail.** Against the code as it stood, this
 * spec *passed* — the kill was a race the game usually won.
 * `shutdown_running` only *sends* on `kill_tx`, and `flush_and_exit` then ran
 * `process::exit` before the run task was ever polled, so the game was
 * commonly orphaned rather than killed (`kill_on_drop` cannot fire either:
 * `exit` runs no destructors). Measured, the core's own log showed the
 * termination signal 154ms after `GAME_LAUNCHED` and no "Instance killed"
 * line at all. Failability was therefore proven against a deliberately
 * deterministic version of the intended behaviour — the same shutdown with a
 * 1.5s sleep before the exit, so the kill lands — and this spec went red on
 * exactly the assertion below. Removing the kill outright is what makes
 * survival deterministic rather than lucky, which is what this now guards.
 *
 * **Why it closes the launcher through the modal.** `main/index.ts`'s
 * `win.on("close")` calls `preventDefault()` while a game is running and
 * raises `WindowCloseWarning` instead, so the window close alone never
 * completes; the modal's own confirm button (`window.closeWindow()`) is the
 * only path that finishes the quit. Driving it is therefore both the honest
 * user path and the only one that works — and it is the only coverage that
 * modal has. The mechanics, and why `app.close()` cannot be awaited around
 * it, live in `quitLauncherMidGame`.
 *
 * Own harness, like `gameLaunch.spec.ts`: it closes the app mid-test, which
 * would tear the core out from under any spec sharing a worker-scoped one.
 *
 * **File name.** Sorts after `dbRecovery`, whose first position is
 * load-bearing: its process cleanup must not run while another spec's app is
 * alive.
 */

import fs from "node:fs"
import path from "node:path"
import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  isCoreModulePresent,
  launchApp,
  quitLauncherMidGame,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  createInstanceViaUi,
  waitForInstallComplete,
  LAUNCH_TIMEOUT
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import {
  isPidAlive,
  killProcessTree,
  pidBelongsToRun
} from "./helpers/processes.js"

const INSTANCE_NAME = "gdl-e2e-survives-close"
const MC_VERSION = "1.20.1"
const LOADER = "forge"

/** `PID_FILE_NAME` in `managers/instance/mod.rs`. Lives at the instance root
 *  rather than inside `instance/`, so a repair that wipes the data dir does
 *  not take it with it. */
const PID_FILE_NAME = ".gdl_instance.pid"

/**
 * The game as the OS still sees it: alive *and* still carrying its own
 * command line.
 *
 * Deliberately stronger than `isPidAlive`. A process that has been killed but
 * whose parent has not yet reaped it stays addressable — `kill(pid, 0)`
 * succeeds on a zombie — and the core exiting right after killing the game is
 * exactly the window in which one exists. A liveness probe alone could
 * therefore read a just-killed game as a surviving one, which is the single
 * way this spec could pass while the behaviour it guards is broken. A zombie
 * has no `/proc/<pid>/cmdline` left to match, so requiring the pid to still
 * carry a command line rules that out — the same attribution
 * `killGameProcesses` uses to decide what it may kill.
 *
 * Matched on the runtime path, not on a managed-JRE path: the launcher uses a
 * system JRE when it finds one, and the runtime path is in the game's
 * arguments either way.
 */
function gameIsRunning(runtimePath: string, pid: number): boolean {
  return pidBelongsToRun(pid, runtimePath)
}

test.describe("running game outlives the launcher", () => {
  // eslint-disable-next-line no-empty-pattern
  test("keeps the game running after the launcher is closed", async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()

    const harness = await startHarness()
    const launchOpts: LaunchOptions = {
      runtimePath: harness.runtimePath,
      baseApi: `${harness.mock.url}/gdl`,
      e2eAuthBase: harness.mock.url,
      e2eEntitlementKey: harness.entitlementKeyPath,
      e2eUpdateFeed: `${harness.mock.url}/updates/`
    }

    let current: {
      app: ElectronApplication
      page: Page
      stdout: string[]
    } | null = null
    let gamePid: number | undefined

    try {
      current = await launchApp(launchOpts)
      const page = current.page
      const stdout = current.stdout
      await completeLogin(page, harness)
      await dismissStartupModals(page)

      await test.step("install a Forge instance", async () => {
        await createInstanceViaUi(page, {
          name: INSTANCE_NAME,
          version: MC_VERSION,
          loader: LOADER
        })
        await waitForInstallComplete(page, INSTANCE_NAME)
      })

      const row = readInstanceByName(harness.runtimePath, INSTANCE_NAME)
      const pidFile = path.join(
        harness.runtimePath,
        "instances",
        row.shortpath,
        PID_FILE_NAME
      )

      // Counted, not searched for: the install itself ends with a transition
      // to Inactive, which prints GAME_LAUNCHED's counterpart before anything
      // has launched. See gameLaunch.spec.ts's identical note.
      const launchedCount = () => stdout.join("").split("GAME_LAUNCHED").length

      await test.step("launch the game", async () => {
        const tile = page.locator(byInstanceName(INSTANCE_NAME))
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

        await expect
          .poll(() => launchedCount(), {
            timeout: LAUNCH_TIMEOUT,
            message: "the core never reported GAME_LAUNCHED after Play"
          })
          .toBeGreaterThan(1)

        await expect
          .poll(() => fs.existsSync(pidFile), {
            timeout: 30_000,
            message: `no pid file appeared at ${pidFile} after launching`
          })
          .toBe(true)
      })

      // The pidfile is two lines, `"{pid}\n{start_time}"` — only the first
      // is the pid; `.trim()` alone leaves the embedded newline and start
      // time in the string and `Number(...)` on that is `NaN`.
      gamePid = Number(fs.readFileSync(pidFile, "utf8").split("\n")[0].trim())
      expect(
        Number.isInteger(gamePid) && gamePid > 0,
        `pid file did not contain a usable pid (read ${JSON.stringify(gamePid)})`
      ).toBe(true)
      expect(
        gameIsRunning(harness.runtimePath, gamePid),
        "the pid file's process is not a live JVM from this runtime path — " +
          "there is nothing here whose survival could be measured"
      ).toBe(true)

      await test.step("close the launcher the way a user does", async () => {
        const quitting = current!
        // Cleared before the quit, not after: nothing on `current` is usable
        // once it is in flight, and leaving it set would have teardown call
        // `app.close()` on an app that is already quitting — the call that
        // hung this test twice.
        current = null
        await quitLauncherMidGame(quitting)
      })

      expect(
        gameIsRunning(harness.runtimePath, gamePid),
        "the game was killed when the launcher closed — a running game is " +
          "the user's session and must outlive the launcher"
      ).toBe(true)

      expect(
        fs.existsSync(pidFile),
        "the pid file was removed when the launcher closed — the next " +
          "startup needs it to find and adopt this process"
      ).toBe(true)
    } finally {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath)
      if (current) {
        // `app.close()` never returns while the app still owns a running
        // game, and bounding it alone leaves a launcher the next spec trips on.
        const electronPid = current.app.process().pid
        await Promise.race([
          current.app.close().catch(() => {}),
          new Promise((resolve) => setTimeout(resolve, 15_000))
        ])
        if (electronPid !== undefined && isPidAlive(electronPid)) {
          killProcessTree(electronPid)
        }
      }
      // Kill the game explicitly: it now survives the launcher by design, so
      // nothing else in teardown would end it. `stopHarness` sweeps the same
      // process as a backstop, but this runs first and by pid, so a failure
      // that never got as far as reading the runtime path still cleans up.
      if (gamePid !== undefined && isPidAlive(gamePid)) {
        try {
          process.kill(gamePid, "SIGKILL")
        } catch {
          // Already gone between the check and here.
        }
      }
      await stopHarness(harness)
    }
  })
})
