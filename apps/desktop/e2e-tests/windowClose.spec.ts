/**
 * Closing the main window quits the launcher cleanly.
 *
 * Electron marks a window destroyed before it emits 'closed', so a 'closed'
 * listener that reads the window's `webContents` throws there. Under this
 * harness's `--gdl_disable_sentry` that uncaught exception reaches
 * `handleUncaughtException`, which exits with 1 and skips the owned core's
 * shutdown; in a release build it is a fatal Sentry event on every close.
 *
 * This closes the window the way the window manager's X does, with no game
 * running, and requires exit code 0, an exited core and a main log without
 * an uncaught exception.
 *
 * Own harness: the app quits itself, so the shared fixtures' `app.close()`
 * teardown would wait on an exit that has already happened.
 */

import fs from "node:fs"
import path from "node:path"
import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  assertCleanElectronExit,
  attachCoreLogOnFailure,
  electronExitStatus,
  electronProcess,
  getCoreProcessId,
  isCoreModulePresent,
  launchApp,
  launcherWindow,
  waitForPidExit,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness } from "./fixtures/mockIdp.js"
import { isPidAlive, killProcessTree } from "./helpers/processes.js"
import { runCleanupSteps, withCleanup } from "./helpers/cleanup.js"

const QUIT_TIMEOUT = 60_000

test.describe("main window close", () => {
  // eslint-disable-next-line no-empty-pattern
  test("closing the main window exits with code 0 and stops the owned core", async ({}, testInfo) => {
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

    let current: { app: ElectronApplication; page: Page } | null = null
    let appExitConfirmed = false

    await withCleanup(
      async () => {
        current = await launchApp(launchOpts)
        const { app, page } = current

        // The login page renders only once the core reported READY, so the
        // quit below goes through the owned core's full shutdown.
        await expect(page.locator("#auth-flow")).toBeVisible({
          timeout: 120_000
        })
        const corePid = await getCoreProcessId(app)
        expect(
          corePid,
          "the core pid is unreadable, so its exit cannot be confirmed"
        ).not.toBeNull()

        await test.step("close the window the way the window manager's X does", async () => {
          // The exit status below is the assertion.
          await (await launcherWindow(app, page)).close()
        })

        await test.step("Electron exits with code 0 and the core exits too", async () => {
          const status = await electronExitStatus(app, QUIT_TIMEOUT)
          expect(
            status.code !== null || status.signal !== null,
            `the launcher was still running ${QUIT_TIMEOUT}ms after its ` +
              "only window closed"
          ).toBe(true)
          appExitConfirmed = true
          assertCleanElectronExit(status)
          await waitForPidExit(corePid!, { timeoutMs: QUIT_TIMEOUT })
        })

        const mainLog = path.join(harness.userDataPath, "main.log")
        expect(
          fs.existsSync(mainLog) ? fs.readFileSync(mainLog, "utf8") : "",
          "closing the window raised an uncaught main-process exception"
        ).not.toContain("Uncaught Exception")
      },
      async (alreadyFailed) => {
        await runCleanupSteps(
          [
            async () => {
              if (current)
                await attachCoreLogOnFailure(testInfo, harness.runtimePath)
            },
            async () => {
              // The quit never finished: end what is left so the next spec
              // does not trip on it.
              if (!current || appExitConfirmed) return
              const pid = electronProcess(current.app).pid
              if (pid !== undefined && isPidAlive(pid)) killProcessTree(pid)
            },
            async () => {
              await stopHarness(harness, undefined, {
                preserveRuntime: !appExitConfirmed
              })
            }
          ],
          alreadyFailed,
          "cleanup for the window-close spec also failed:"
        )
      }
    )
  })
})
