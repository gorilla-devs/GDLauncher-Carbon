/**
 * Proves a launcher reopened over a still-running game adopts it.
 *
 * `instanceSurvivesClose.spec.ts` covers the first half — the game outlives
 * the launcher. This covers what the next launch does with it. Both halves
 * are needed together: a startup that stops killing the survivor but does not
 * pick it up leaves a real JVM holding an instance the UI calls Inactive,
 * which invites a second launch of the same instance over the first.
 *
 * Two behaviours, one per test:
 *
 * 1. **Adopted.** The instance reads Running, the Log tab says why there is no
 *    live log, and Stop ends the process and clears the pidfile — all without
 *    this core ever having owned a handle to it.
 * 2. **Released.** With the game killed from outside, the instance returns to
 *    Inactive on its own. This is what makes the liveness poller load-bearing
 *    rather than decorative: nothing else observes an adopted process exiting,
 *    so without it the instance reads Running until the app is restarted —
 *    including after the user simply quits Minecraft, which is the common case
 *    and strictly worse than what adoption replaced.
 *
 * Own harness per test, like `gameLaunch.spec.ts`: each closes and relaunches
 * the app, which would tear the core out from under a shared worker-scoped
 * one.
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
import { startHarness, stopHarness, type Harness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  createInstanceViaUi,
  waitForInstallComplete,
  LAUNCH_TIMEOUT
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import { isPidAlive } from "./helpers/processes.js"

const MC_VERSION = "1.20.1"
const LOADER = "forge"

/** `PID_FILE_NAME` in `managers/instance/mod.rs`. */
const PID_FILE_NAME = ".gdl_instance.pid"

/** `ADOPTED_POLL_INTERVAL` in `managers/instance/run/mod.rs` is 5s, so a
 *  release is noticed within one tick of the game exiting. Left far wider than
 *  that on purpose: this waits on a background task competing with a launcher
 *  that has just finished starting up, and a budget sized to the happy path
 *  would only buy flakiness. */
const RELEASE_TIMEOUT = 60_000

interface RunningApp {
  app: ElectronApplication
  page: Page
  stdout: string[]
}

/**
 * Installs a Forge instance, plays it, and quits the launcher out from under
 * the running game — the state both tests start from.
 *
 * Returns the game's pid and its pidfile path, read before the quit while the
 * launcher was still there to produce them.
 */
async function leaveAGameRunning(
  harness: Harness,
  launchOpts: LaunchOptions,
  instanceName: string
): Promise<{ gamePid: number; pidFile: string }> {
  const current = await launchApp(launchOpts)
  await completeLogin(current.page, harness)
  await dismissStartupModals(current.page)

  await createInstanceViaUi(current.page, {
    name: instanceName,
    version: MC_VERSION,
    loader: LOADER
  })
  await waitForInstallComplete(current.page, instanceName)

  const row = readInstanceByName(harness.runtimePath, instanceName)
  const pidFile = path.join(
    harness.runtimePath,
    "instances",
    row.shortpath,
    PID_FILE_NAME
  )

  // Counted, not searched for: the install itself ends with a transition to
  // Inactive, so the string is already present before anything has launched.
  const launchedCount = () =>
    current.stdout.join("").split("GAME_LAUNCHED").length

  const tile = current.page.locator(byInstanceName(instanceName))
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

  // The pidfile is two lines, `"{pid}\n{start_time}"` — only the first is
  // the pid; `.trim()` alone leaves the embedded newline and start time in
  // the string and `Number(...)` on that is `NaN`.
  const gamePid = Number(fs.readFileSync(pidFile, "utf8").split("\n")[0].trim())
  expect(
    Number.isInteger(gamePid) && gamePid > 0,
    `pid file did not contain a usable pid (read ${JSON.stringify(gamePid)})`
  ).toBe(true)

  await quitLauncherMidGame(current)

  expect(
    isPidAlive(gamePid),
    "the game did not survive the launcher closing, so there is nothing for " +
      "the next launch to adopt — see instanceSurvivesClose.spec.ts"
  ).toBe(true)

  return { gamePid, pidFile }
}

/** Kills `pid` if it is still there. Teardown only — an adopted game outlives
 *  the launcher by design, so no app teardown will end it. */
function killIfAlive(pid: number | undefined): void {
  if (pid === undefined || !isPidAlive(pid)) return
  try {
    process.kill(pid, "SIGKILL")
  } catch {
    // Raced away between the check and here.
  }
}

test.describe("adopting a running game across a launcher restart", () => {
  // eslint-disable-next-line no-empty-pattern
  test("shows it running, explains the missing log, and stops it", async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()

    const instanceName = "gdl-e2e-adoption"
    const harness = await startHarness()
    const launchOpts: LaunchOptions = {
      runtimePath: harness.runtimePath,
      baseApi: `${harness.mock.url}/gdl`,
      e2eAuthBase: harness.mock.url,
      e2eEntitlementKey: harness.entitlementKeyPath,
      e2eUpdateFeed: `${harness.mock.url}/updates/`
    }

    let reopened: RunningApp | null = null
    let gamePid: number | undefined

    try {
      const left = await leaveAGameRunning(harness, launchOpts, instanceName)
      gamePid = left.gamePid

      // Same runtime path, so the same instances directory and the same
      // pidfile the previous session wrote.
      reopened = await launchApp(launchOpts)
      const page = reopened.page
      // No `completeLogin` on a relaunch: the account persisted with the
      // runtime path, so the auth flow never renders and waiting for it hangs
      // (the same reason `persistence.spec.ts` skips it after `relaunchApp`).
      await dismissStartupModals(page)

      const tile = page.locator(byInstanceName(instanceName))

      await test.step("the instance reads running", async () => {
        // The state attribute rather than the play control's icon: the control
        // is present either way, so its presence alone would pass against an
        // instance the launcher thinks is Inactive.
        await expect(
          tile,
          "the reopened launcher did not adopt the running game — it reads " +
            "Inactive, so the JVM is alive but unreachable from the UI and a " +
            "second Play would launch the same instance twice"
        ).toHaveAttribute("data-instance-state", "running")
      })

      await test.step("the log tab explains why there is no live log", async () => {
        await tile.click()
        await page.getByRole("tab", { name: /logs/i }).click()

        await expect(
          page.locator(byTestId(TEST_IDS.instanceAdoptedNoLiveLog)),
          "the Log tab showed no notice for an adopted session — an empty log " +
            "view reads as a log that failed to load rather than one that " +
            "cannot exist"
        ).toBeVisible()
      })

      // Playtime the previous session managed to bank before it was cut off.
      // Expected to be zero — its launch task banks on a 60s tick and once
      // more after `child.wait()`, and the core was killed before either —
      // but read rather than assumed, so this compares against whatever
      // actually happened.
      const instanceRoot = path.dirname(left.pidFile)
      const playtimeBeforeStop = (await readInstanceConfig(instanceRoot))
        .secondsPlayed

      await test.step("stop ends the process and clears the pidfile", async () => {
        await page.locator(byTestId(TEST_IDS.navbarLogo)).click()
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

        await expect
          .poll(() => isPidAlive(gamePid!), {
            timeout: 60_000,
            message:
              "Stop did not end the adopted game — with no kill channel, the " +
              "pid is the only way to reach it"
          })
          .toBe(false)

        await expect
          .poll(() => fs.existsSync(left.pidFile), {
            timeout: 30_000,
            message:
              "the pidfile outlived the process it records, so the next " +
              "startup would try to adopt a pid that is gone"
          })
          .toBe(false)

        await expect(tile).toHaveAttribute("data-instance-state", "inactive")
      })

      await test.step("the adopted session's playtime is banked", async () => {
        // Stopping is the one moment an adopted session's final interval can
        // be banked exactly: the process was alive right up to the signal.
        // The poller deliberately drops the equivalent interval when it finds
        // a process already gone, so this is the path worth pinning down.
        await expect
          .poll(
            async () => (await readInstanceConfig(instanceRoot)).secondsPlayed,
            {
              timeout: 30_000,
              message:
                "no playtime was recorded for the adopted session — nothing " +
                "banks it but the adoption poller and Stop, so the time a " +
                "user spends playing after closing the launcher is lost"
            }
          )
          .toBeGreaterThan(playtimeBeforeStop)
      })
    } finally {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath)
      if (reopened) await reopened.app.close()
      killIfAlive(gamePid)
      await stopHarness(harness)
    }
  })

  // eslint-disable-next-line no-empty-pattern
  test("returns it to inactive once the game exits on its own", async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()

    const instanceName = "gdl-e2e-adoption-release"
    const harness = await startHarness()
    const launchOpts: LaunchOptions = {
      runtimePath: harness.runtimePath,
      baseApi: `${harness.mock.url}/gdl`,
      e2eAuthBase: harness.mock.url,
      e2eEntitlementKey: harness.entitlementKeyPath,
      e2eUpdateFeed: `${harness.mock.url}/updates/`
    }

    let reopened: RunningApp | null = null
    let gamePid: number | undefined

    try {
      const left = await leaveAGameRunning(harness, launchOpts, instanceName)
      gamePid = left.gamePid

      reopened = await launchApp(launchOpts)
      const page = reopened.page
      // See the sibling test: a relaunch onto the same runtime path is already
      // logged in.
      await dismissStartupModals(page)

      const tile = page.locator(byInstanceName(instanceName))
      await expect(tile).toHaveAttribute("data-instance-state", "running")

      // Stands in for the user quitting Minecraft: from the launcher's side
      // the process simply stops existing, which is the only thing the poller
      // can observe either way.
      process.kill(gamePid, "SIGKILL")

      await expect
        .poll(() => tile.getAttribute("data-instance-state"), {
          timeout: RELEASE_TIMEOUT,
          message:
            "the instance stayed Running after its game exited — nothing " +
            "waits on an adopted process, so without the liveness poll it " +
            "reads Running until the launcher is restarted"
        })
        .toBe("inactive")

      await expect
        .poll(() => fs.existsSync(left.pidFile), {
          timeout: 30_000,
          message: "the pidfile outlived the process it records"
        })
        .toBe(false)
    } finally {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath)
      if (reopened) await reopened.app.close()
      killIfAlive(gamePid)
      await stopHarness(harness)
    }
  })
})
