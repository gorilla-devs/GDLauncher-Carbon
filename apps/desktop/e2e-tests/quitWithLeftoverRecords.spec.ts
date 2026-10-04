/**
 * Leftover process records from an earlier session never keep the launcher
 * from closing.
 *
 * Two records a crash or a reused PID leaves behind used to make the core
 * refuse every quit, restart and update install with "Shutdown incomplete":
 *
 * - an empty `.gdl_instance.pid`, as a power loss right after a launch
 *   leaves it (the marker is not written atomically);
 * - a `__gdl_runtime_writers` record whose PID now belongs to some other,
 *   non-Java process that is still running.
 *
 * Both are seeded before the first launch. Closing the window must then exit
 * with code 0 within the normal quit time, stop the core and leave the
 * unrelated process alone. The empty marker stays where it is: it only keeps
 * a move of the runtime folder from starting. What happens to the reused-PID
 * record depends on the platform, exactly as the core's own
 * `an_update_install_commits_despite_leftover_process_records` asserts:
 * Windows and macOS report a creation time that never moves, so a different
 * start time proves the PID was reused and the startup check removes the
 * record (`recorded_identity_status` in managers/runtime_lifecycle.rs). Linux
 * derives the start time from the wall clock, which a clock step can move,
 * so there the record is kept as unconfirmed.
 *
 * Not covered here: the update install. It runs the same core handshake
 * under purpose "update" (`prepareOwnedShutdown("update")` in main/index.ts)
 * before electron-updater installs, but the e2e update feed only ever
 * reports "no update", so this spec cannot reach an install. Its pieces are
 * tested on their own: updaterController.test.ts (install sends an "update"
 * shutdown and installs only after the core exited) and the core's
 * api/runtime.rs an_update_install_commits_despite_leftover_process_records
 * (the same requests commit with these leftover records present).
 *
 * Own harness: the app quits itself, so the shared fixtures' `app.close()`
 * teardown would wait on an exit that has already happened.
 */

import crypto from "node:crypto"
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
const INSTANCE_PID_FILE = ".gdl_instance.pid"
const RUNTIME_WRITERS = "__gdl_runtime_writers"
/** Where a different start time proves PID reuse (`EPOCH_START_TIMES_ARE_STABLE`
 *  in managers/orphan_pid.rs), so the core removes the reused-PID record. */
const REUSE_PROVEN = process.platform !== "linux"

test.describe("quit with leftover process records", () => {
  // eslint-disable-next-line no-empty-pattern
  test("an empty instance marker and a reused-PID record do not refuse the quit", async ({}, testInfo) => {
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

    // Truncated by a crash: the core cannot parse it.
    const instanceRoot = path.join(
      harness.runtimePath,
      "instances",
      "e2e-crashed-instance"
    )
    fs.mkdirSync(instanceRoot, { recursive: true })
    const emptyMarker = path.join(instanceRoot, INSTANCE_PID_FILE)
    fs.writeFileSync(emptyMarker, "")

    // A record from an earlier session whose PID is now held by this test
    // runner: alive, not Java, and never signalled by the core. The start
    // time does not match it, as after a reuse.
    const writers = path.join(harness.runtimePath, RUNTIME_WRITERS)
    fs.mkdirSync(writers, { recursive: true })
    const reusedRecord = path.join(writers, `${crypto.randomUUID()}.json`)
    const reusedContent = JSON.stringify({
      version: 1,
      process: { pid: process.pid, start_time: 1, identity: null }
    })
    fs.writeFileSync(reusedRecord, reusedContent)

    let current: { app: ElectronApplication; page: Page } | null = null
    let appExitConfirmed = false

    await withCleanup(
      async () => {
        current = await launchApp(launchOpts)
        const { app, page } = current

        // The login page renders only once the core reported READY.
        await expect(page.locator("#auth-flow")).toBeVisible({
          timeout: 120_000
        })
        const corePid = await getCoreProcessId(app)
        expect(
          corePid,
          "the core pid is unreadable, so its exit cannot be confirmed"
        ).not.toBeNull()

        await test.step("close the window the way the window manager's X does", async () => {
          await (await launcherWindow(app, page)).close()
        })

        await test.step("the launcher and its core exit instead of refusing", async () => {
          const status = await electronExitStatus(app, QUIT_TIMEOUT)
          expect(
            status.code !== null || status.signal !== null,
            `the launcher was still running ${QUIT_TIMEOUT}ms after its ` +
              "only window closed: a leftover record refused the quit"
          ).toBe(true)
          appExitConfirmed = true
          assertCleanElectronExit(status)
          await waitForPidExit(corePid!, { timeoutMs: QUIT_TIMEOUT })
        })

        await test.step("the leftover records are kept or proven gone, and the other process lives", async () => {
          expect(isPidAlive(process.pid)).toBe(true)
          expect(fs.readFileSync(emptyMarker, "utf8")).toBe("")
          if (REUSE_PROVEN) {
            expect(
              fs.existsSync(reusedRecord),
              "the reused-PID record is still there, although this platform's " +
                "stable creation times prove its process is gone"
            ).toBe(false)
          } else {
            expect(fs.readFileSync(reusedRecord, "utf8")).toBe(reusedContent)
          }
        })

        // A refused quit keeps the launcher running, which the exit check
        // above already fails on; this only catches a crash on the way out.
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
          "cleanup for the leftover-records quit spec also failed:"
        )
      }
    )
  })
})
