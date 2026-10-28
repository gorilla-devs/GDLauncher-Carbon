/**
 * Minecraft process/startup-log smoke test, using its own isolated harness.
 *
 * Observes GAME_LAUNCHED, continued liveness through a bounded quiet-log
 * window, a loader-agnostic startup-marker quorum and no known JVM-fatal
 * signature. A client that exits fails the liveness check. A silent live
 * client can still be unresponsive: no render, menu or input response is
 * observed, so this does not prove a stable main menu or exclude hangs.
 *
 * The mock entitlement satisfies the launcher's gate; Minecraft accepts the
 * offline startup token, while its online services can reject it later.
 * Real game/loader downloads and working OpenGL are still required.
 * This owns the runtime and cleanup because a JVM can outlive its launcher.
 */

import fs from "node:fs"
import path from "node:path"
import { expect, test, type TestInfo } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  closeAppAndCore,
  isCoreModulePresent,
  launchApp,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import {
  byInstanceName,
  byLaunchActionOption,
  byTestId,
  TEST_IDS
} from "./helpers/selectors.js"
import {
  clickPlayAndAwaitLaunched,
  createInstanceViaUi,
  LAUNCH_TIMEOUT,
  STOP_TIMEOUT,
  waitForInstallComplete
} from "./helpers/instances.js"
import { isPidAlive } from "./helpers/processes.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import {
  countMatchedMarkers,
  findFatalSignature,
  instanceLogsDir,
  LAUNCH_MARKERS,
  LAUNCH_MARKER_QUORUM,
  newestLogFile,
  readLogMessages,
  waitForLogQuiescence
} from "./helpers/gameLog.js"
import { runCleanupSteps, withCleanup } from "./helpers/cleanup.js"

const INSTANCE_NAME = "gdl-e2e-game-launch"
const MC_VERSION = "1.20.1"
const LOADER = "forge"
const CLOSE_WINDOW_INSTANCE_NAME = "gdl-e2e-close-window"
/** `PID_FILE_NAME` in `managers/instance/mod.rs`. */
const PID_FILE_NAME = ".gdl_instance.pid"
/** How long a window the launcher opens by itself gets to reach the library:
 *  a fresh renderer replays the loading screen's minimum time. */
const WINDOW_READY_TIMEOUT = 120_000

/** Picks the launch action in Settings > General and returns to the library. */
async function setLaunchAction(page: Page, action: string): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarSettings))
  const trigger = page.locator(byTestId(TEST_IDS.settingsLaunchActionSelect))
  await expect(trigger).toBeVisible()
  const responsePromise = page.waitForResponse(
    (r) => r.url().includes("settings.setSettings"),
    { timeout: 15_000 }
  )
  await trigger.click()
  await page.locator(byLaunchActionOption(action)).click()
  const body = (await (await responsePromise).json()) as {
    result?: { type?: string; data?: unknown }
  }
  if (body.result?.type === "error") {
    throw new Error(
      "setup: settings.setSettings returned an rspc error: " +
        JSON.stringify(body.result.data)
    )
  }
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}

/** The pidfile's first line is the pid; later lines hold start-time data. */
function readGamePid(pidFile: string): number {
  const pid = Number(fs.readFileSync(pidFile, "utf8").split("\n")[0].trim())
  expect(
    Number.isInteger(pid) && pid > 0,
    `pid file did not contain a usable pid (read ${JSON.stringify(pid)})`
  ).toBe(true)
  return pid
}

test.describe("game launch", () => {
  // eslint-disable-next-line no-empty-pattern
  test("starts Minecraft and observes a live process with quiet startup logs", async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()

    const harness = await startHarness()
    const launchOpts: LaunchOptions = {
      runtimePath: harness.runtimePath,
      userDataPath: harness.userDataPath,
      runId: harness.runId,
      baseApi: `${harness.mock.url}/gdl`,
      e2eAuthBase: harness.mock.url,
      e2eEntitlementKey: harness.entitlementKeyPath,
      e2eUpdateFeed: `${harness.mock.url}/updates/`
    }

    let current: {
      app: ElectronApplication
      page: Page
      pageErrors: Error[]
      stdout: string[]
    } | null = null
    let stdout: string[] = []
    /** GAME_CLOSED count at the moment before Play was clicked. Declared out
     *  here because `finally` is a different block scope from `try`, and
     *  teardown needs it to tell "still running" from "already died". */
    let closedBeforeLaunch = 0

    /** Counted rather than searched for: `change_launch_state`
     *  (`run/mod.rs`) prints GAME_CLOSED on every transition to Inactive,
     *  and the *install* ends with one — so the string is already present
     *  before anything launches, and a plain `includes` check is satisfied
     *  instantly and proves nothing. */
    const closedCount = () => stdout.join("").split("GAME_CLOSED").length

    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
        current = await launchApp(launchOpts)
        const page = current.page
        stdout = current.stdout
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
        const logsDir = instanceLogsDir(harness.runtimePath, row.shortpath)
        closedBeforeLaunch = closedCount()

        await test.step("launch the game", async () => {
          await clickPlayAndAwaitLaunched(page, INSTANCE_NAME, { stdout })
        })

        await test.step("observe startup-log quiescence", async () => {
          await waitForLogQuiescence(
            logsDir,
            () => closedCount() === closedBeforeLaunch
          )
        })

        await test.step("assert live process and startup-log evidence", async () => {
          // Process liveness is independent of silence; responsiveness is unobserved.
          expect(
            closedCount(),
            "the game exited before the quiet startup-log observation completed"
          ).toBe(closedBeforeLaunch)

          const messages = readLogMessages(newestLogFile(logsDir))

          const fatal = findFatalSignature(messages)
          expect(
            fatal,
            `the game log contains a JVM-fatal signature: ${fatal}`
          ).toBeUndefined()

          // Corroboration, deliberately a quorum — see LAUNCH_MARKERS.
          const matched = countMatchedMarkers(messages, LAUNCH_MARKERS)
          expect(
            matched,
            `only ${matched} of ${LAUNCH_MARKERS.length} startup markers ` +
              `appeared in the game log (need ${LAUNCH_MARKER_QUORUM}). The ` +
              "client stayed alive and went quiet, but startup corroboration was " +
              "insufficient; inspect the trace and possible Minecraft marker drift."
          ).toBeGreaterThanOrEqual(LAUNCH_MARKER_QUORUM)
        })
      },
      async (alreadyFailed) => {
        let appExitConfirmed = !current
        await runCleanupSteps(
          [
            async () => {
              if (!current) return
              const before = closedCount()
              // The play control becomes Stop only while this launch is alive.
              // A later GAME_CLOSED means clicking it would start another game.
              if (before > closedBeforeLaunch) return
              const tile = current.page.locator(byInstanceName(INSTANCE_NAME))
              await tile.locator(byTestId(TEST_IDS.instancePlay)).click()
              await expect
                .poll(() => closedCount(), {
                  timeout: STOP_TIMEOUT,
                  message:
                    "the game never reported a new GAME_CLOSED after Stop"
                })
                .toBeGreaterThan(before)
            },
            async () => {
              if (current)
                await attachCoreLogOnFailure(testInfo, harness.runtimePath)
            },
            async () => {
              if (current) {
                await closeAppAndCore(current.app, { strict: true })
                appExitConfirmed = true
              }
            },
            async () => {
              await stopHarness(harness, undefined, {
                preserveRuntime: !appExitConfirmed
              })
            }
          ],
          alreadyFailed,
          `cleanup for "${INSTANCE_NAME}" also failed:`
        )
      }
    )
  })

  /**
   * "Launcher Action on Game Launch: Close Window". The window closes when
   * the game starts, the launcher stays alive without a window, and exactly
   * one window comes back when the game exits.
   *
   * Electron marks a window destroyed before it emits 'closed'. A 'closed'
   * listener that read the window's webContents threw there: under this
   * harness's --gdl_disable_sentry the launcher exited with 1 the moment the
   * game started, and in a release build it stayed alive with no window,
   * never reopened and ignored every later launch.
   */
  // eslint-disable-next-line no-empty-pattern
  test("Close Window on launch keeps the launcher alive and reopens one window when the game exits", async ({}, testInfo) => {
    await withCloseWindowLauncher(testInfo, async (session) => {
      await test.step("launching closes the window and the launcher stays alive", async () => {
        await session.launchAndAwaitClose()
        const electron = session.app().process()
        expect(
          electron.exitCode === null && electron.signalCode === null,
          "the launcher exited when its window closed on game launch"
        ).toBe(true)
        expect(
          session.mainLog(),
          "closing the window on launch raised an uncaught exception"
        ).not.toContain("Uncaught Exception")
      })

      await test.step("exactly one window comes back when the game exits", async () => {
        const reopened = session.app().waitForEvent("window", {
          timeout: STOP_TIMEOUT
        })
        await session.exitGame()
        await session.useWindow(await reopened)
        expect(await session.windowCount()).toBe(1)
        expect(session.mainLog()).not.toContain("Uncaught Exception")
      })
    })
  })

  /**
   * While a Close Window game runs and no window exists, a second launch of
   * GDLauncher (the single-instance lock's 'second-instance') shows one, and
   * the game exiting later reuses it instead of opening another. Before the
   * 'closed' listener stopped throwing, that second launch showed nothing.
   */
  // eslint-disable-next-line no-empty-pattern
  test("launching GDLauncher again during a Close Window game shows one window, which the game exit reuses", async ({}, testInfo) => {
    await withCloseWindowLauncher(testInfo, async (session) => {
      await test.step("launching closes the window", async () => {
        await session.launchAndAwaitClose()
        expect(session.mainLog()).not.toContain("Uncaught Exception")
      })

      await test.step("launching GDLauncher again while no window exists shows one", async () => {
        const shown = session.app().waitForEvent("window", {
          timeout: 60_000
        })
        // The harness passes --gdl_allow_multiple_instances, which skips the
        // single-instance lock, so a real second launch would start its own
        // app. Emit what the lock delivers to the running one.
        await session.app().evaluate(({ app }) => {
          app.emit("second-instance", {}, [])
        })
        await session.useWindow(await shown)
        expect(await session.windowCount()).toBe(1)
      })

      await test.step("the game exiting later reuses that window", async () => {
        await session.exitGame()
        // Give a duplicate reopen time to show up before counting.
        await session.page().waitForTimeout(3_000)
        expect(await session.windowCount()).toBe(1)
        await expect(
          session.page().locator(byTestId(TEST_IDS.libraryRoot))
        ).toBeVisible()
        expect(session.mainLog()).not.toContain("Uncaught Exception")
      })
    })
  })
})

/** What a Close Window test drives once the instance is installed. */
interface CloseWindowLauncher {
  app(): ElectronApplication
  /** The window later steps and teardown drive. */
  page(): Page
  windowCount(): Promise<number>
  mainLog(): string
  /** Play, then wait for the window to close on launch. */
  launchAndAwaitClose(): Promise<void>
  /** Stands in for the user quitting Minecraft. */
  exitGame(): Promise<void>
  /** Makes `page` the window later steps and teardown drive. */
  useWindow(page: Page): Promise<void>
}

/**
 * Logs in, sets "Launcher Action on Game Launch: Close Window", installs a
 * Forge instance and runs `body`. Owns the harness and the cleanup, and ends
 * the game first: it outlives the launcher by design. Each Close Window test
 * gets its own launcher so one install and at most two windows fit well
 * inside the per-test budget.
 */
async function withCloseWindowLauncher(
  testInfo: TestInfo,
  body: (launcher: CloseWindowLauncher) => Promise<void>
): Promise<void> {
  expect(isCoreModulePresent()).toBeTruthy()

  const harness = await startHarness()
  const launchOpts: LaunchOptions = {
    runtimePath: harness.runtimePath,
    userDataPath: harness.userDataPath,
    runId: harness.runId,
    baseApi: `${harness.mock.url}/gdl`,
    e2eAuthBase: harness.mock.url,
    e2eEntitlementKey: harness.entitlementKeyPath,
    e2eUpdateFeed: `${harness.mock.url}/updates/`
  }

  let current: {
    app: ElectronApplication
    page: Page
    pageErrors: Error[]
    stdout: string[]
  } | null = null
  let stdout: string[] = []
  let gamePid: number | undefined

  // Counted, not searched for: see `closedCount` in the first test.
  const closedCount = () => stdout.join("").split("GAME_CLOSED").length
  const windowCount = () =>
    current!.app.evaluate(
      ({ BrowserWindow }) => BrowserWindow.getAllWindows().length
    )
  const mainLog = () => {
    const file = path.join(harness.userDataPath, "main.log")
    return fs.existsSync(file) ? fs.readFileSync(file, "utf8") : ""
  }

  await withCleanup(
    async () => {
      current = await launchApp(launchOpts)
      stdout = current.stdout
      await completeLogin(current.page, harness)
      await dismissStartupModals(current.page)

      await test.step("set the launch action to Close Window", async () => {
        await setLaunchAction(current!.page, "closeWindow")
      })

      await test.step("install a Forge instance", async () => {
        await createInstanceViaUi(current!.page, {
          name: CLOSE_WINDOW_INSTANCE_NAME,
          version: MC_VERSION,
          loader: LOADER
        })
        await waitForInstallComplete(current!.page, CLOSE_WINDOW_INSTANCE_NAME)
      })

      const row = readInstanceByName(
        harness.runtimePath,
        CLOSE_WINDOW_INSTANCE_NAME
      )
      const pidFile = path.join(
        harness.runtimePath,
        "instances",
        row.shortpath,
        PID_FILE_NAME
      )

      await body({
        app: () => current!.app,
        page: () => current!.page,
        windowCount,
        mainLog,
        launchAndAwaitClose: async () => {
          const page = current!.page
          const closed = page.waitForEvent("close", { timeout: LAUNCH_TIMEOUT })
          await clickPlayAndAwaitLaunched(page, CLOSE_WINDOW_INSTANCE_NAME, {
            stdout
          })
          await closed
          await expect
            .poll(windowCount, {
              message: "a window was still open after the game launched"
            })
            .toBe(0)
          await expect
            .poll(() => fs.existsSync(pidFile), {
              timeout: 30_000,
              message: `no pid file appeared at ${pidFile} after launching`
            })
            .toBe(true)
          gamePid = readGamePid(pidFile)
        },
        exitGame: async () => {
          const before = closedCount()
          process.kill(gamePid!, "SIGKILL")
          await expect
            .poll(closedCount, {
              timeout: STOP_TIMEOUT,
              message: "the core never reported GAME_CLOSED for the killed game"
            })
            .toBeGreaterThan(before)
          gamePid = undefined
        },
        useWindow: async (page) => {
          current!.page = page
          await expect(
            page.locator(byTestId(TEST_IDS.libraryRoot))
          ).toBeVisible({ timeout: WINDOW_READY_TIMEOUT })
          await dismissStartupModals(page)
        }
      })
    },
    async (alreadyFailed) => {
      let appExitConfirmed = !current
      await runCleanupSteps(
        [
          async () => {
            // The game outlives the launcher by design; end it first.
            if (gamePid !== undefined && isPidAlive(gamePid))
              process.kill(gamePid, "SIGKILL")
          },
          async () => {
            if (current)
              await attachCoreLogOnFailure(testInfo, harness.runtimePath)
          },
          async () => {
            if (current) {
              await closeAppAndCore(current.app, { strict: true })
              appExitConfirmed = true
            }
          },
          async () => {
            await stopHarness(harness, undefined, {
              preserveRuntime: !appExitConfirmed
            })
          }
        ],
        alreadyFailed,
        `cleanup for "${CLOSE_WINDOW_INSTANCE_NAME}" also failed:`
      )
    }
  )
}
