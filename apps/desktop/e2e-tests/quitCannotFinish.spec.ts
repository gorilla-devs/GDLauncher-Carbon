/**
 * A quit that cannot finish brings the window back with a way out.
 *
 * Closing the window used to destroy it before the core was asked to stop.
 * When the core could not finish (a long copy, a Recycle Bin move, a stuck
 * task), GDLauncher kept running with no window, the core refused every
 * Play, install and settings save, and only Task Manager ended it.
 *
 * The core's `e2e` build has a hook that admits a task which ignores
 * cancellation, as such work does. With it held:
 *
 * 1. Closing the window hides it; within a few seconds it comes back with
 *    the shutdown prompt, shown over the launcher (not inside the hidden
 *    #overlay layer) and naming the task. While it is on screen it keeps
 *    waiting, also past the core's 15 s budget, instead of turning into a
 *    "can't close yet" error. "Keep GDLauncher open" gives the launcher
 *    back: the core admits new work again (a launch goes through the same
 *    admission; Play itself needs a signed-in account, which this harness
 *    does not have).
 * 2. Closing it again and choosing "Quit now" ends both the core and
 *    Electron within 5 s, even though the core's own SIGTERM handler is
 *    refused while the quit is pending (the launcher escalates to SIGKILL).
 *
 * Own harness: the app quits itself, so the shared fixtures' `app.close()`
 * teardown would wait on an exit that has already happened.
 */

import path from "node:path"
import fs from "node:fs"
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

/** Longer than the core's 15 s operation budget and the 1 s retry pause. */
const PAST_ONE_ATTEMPT = 20_000

/** POSTs to one of the core's /runtime endpoints as the renderer would. */
async function runtime(page: Page, endpoint: string): Promise<number> {
  const core = await page.evaluate(async () => {
    const module = await window.getCoreModule()
    return module.type === "success"
      ? { port: module.port, apiToken: module.apiToken }
      : null
  })
  expect(core, "the core never reported READY").not.toBeNull()
  const response = await fetch(
    `http://127.0.0.1:${core!.port}/runtime/${endpoint}`,
    {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-api-token": core!.apiToken
      },
      body: "{}"
    }
  )
  return response.status
}

test.describe("a quit that cannot finish", () => {
  // eslint-disable-next-line no-empty-pattern
  test("brings the window back with Keep GDLauncher open and Quit now", async ({}, testInfo) => {
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

        // The login page renders only once the core reported READY, and
        // inside the App that shows the prompt.
        await expect(page.locator("#auth-flow")).toBeVisible({
          timeout: 120_000
        })
        const corePid = await getCoreProcessId(app)
        expect(corePid, "the core pid is unreadable").not.toBeNull()
        const prompt = page.getByTestId("shutdown-prompt")
        const launcher = await launcherWindow(app, page)

        await test.step("hold a task that ignores cancellation", async () => {
          expect(await runtime(page, "e2e-hold-task")).toBe(200)
        })

        await test.step("closing the window hides it, then shows why it did not quit", async () => {
          await launcher.close()
          expect(await launcher.visible()).toBe(false)
          await expect(prompt).toBeVisible({ timeout: 15_000 })
          expect(await launcher.visible()).toBe(true)
          await expect(prompt).toHaveAttribute("data-stage", "waiting")
          await expect(
            page.getByTestId("shutdown-prompt-blockers")
          ).toBeVisible()
          await expect(page.getByTestId("shutdown-prompt-force")).toBeVisible()
          await expect(page.getByTestId("shutdown-prompt-keep")).toBeVisible()
        })

        await test.step("the prompt keeps waiting past one attempt instead of turning into an error", async () => {
          await page.waitForTimeout(PAST_ONE_ATTEMPT)
          await expect(prompt).toBeVisible()
          await expect(prompt).toHaveAttribute("data-stage", "waiting")
        })

        await test.step("Keep GDLauncher open gives the launcher back", async () => {
          await page.getByTestId("shutdown-prompt-keep").click()
          await expect(prompt).toBeHidden()
          expect(await launcher.visible()).toBe(true)
          await expect(page.locator("#auth-flow")).toBeVisible()
          // Admission is open again: new work is not refused with
          // "Runtime shutdown or relocation is in progress".
          expect(await runtime(page, "e2e-hold-task")).toBe(200)
        })

        await test.step("Quit now ends the core and the launcher within 5 s", async () => {
          await launcher.close()
          await expect(prompt).toBeVisible({ timeout: 15_000 })
          const clicked = Date.now()
          await page
            .getByTestId("shutdown-prompt-force")
            .click()
            .catch(() => {})
          await waitForPidExit(corePid!, { timeoutMs: 5_000 })
          const status = await electronExitStatus(
            app,
            Math.max(0, 5_000 - (Date.now() - clicked))
          )
          expect(
            status.code !== null || status.signal !== null,
            "Electron was still running 5 s after Quit now"
          ).toBe(true)
          appExitConfirmed = true
          assertCleanElectronExit(status)
        })

        const mainLog = path.join(harness.userDataPath, "main.log")
        expect(
          fs.existsSync(mainLog) ? fs.readFileSync(mainLog, "utf8") : "",
          "the forced quit raised an uncaught main-process exception"
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
          "cleanup for the quit-cannot-finish spec also failed:"
        )
      }
    )
  })
})
