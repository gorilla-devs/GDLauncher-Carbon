/**
 * Proves the launcher can actually start Minecraft and get it to a stable
 * main menu — not merely that it spawned a process.
 *
 * A plausible-looking assumption is that the game "is never launched" because
 * mock-IdP accounts "hold a mock entitlement real Minecraft rejects". That
 * reasoning does not hold, and this spec exists because it was checked
 * rather than believed: the Minecraft client does not validate its access
 * token at startup — which is why offline-mode launchers work at all — and
 * the launcher's own entitlement gate is exactly what
 * `--gdl_e2e_entitlement_key` satisfies. A captured launch reaches the title
 * screen under llvmpipe software GL, logging `Sound engine started`, eleven
 * stitched texture atlases, and finally a Realms authorization failure that
 * is the mock token being rejected in the one place it should be.
 *
 * **What "reached the main menu" is asserted as.** Minecraft logs no
 * title-screen event, and every plausible textual proxy has changed wording
 * across versions, so a fingerprint of exact lines would go red on a
 * Minecraft release for no real reason. The hard assertions here are
 * therefore behavioural and carry no log wording at all:
 *
 *   1. the core reports GAME_LAUNCHED;
 *   2. no GAME_CLOSED follows within the observation window — a crashed
 *      client exits, so surviving rules out the largest failure class
 *      (missing libraries, bad classpath, wrong Java, absent natives);
 *   3. the log grows and then goes quiescent — the client finished loading
 *      rather than still working or hanging mid-startup;
 *   4. the log holds no JVM-fatal signature.
 *
 * A crash cannot satisfy (2) and (3) together: it exits, which produces the
 * GAME_CLOSED that (2) forbids. A hang cannot either — it keeps logging, or
 * never reaches quiescence after having grown.
 *
 * Textual evidence is kept as corroboration only, through
 * `gameLog.ts`'s `LAUNCH_MARKERS` quorum: at least
 * `LAUNCH_MARKER_QUORUM` of a loose, loader-agnostic pool. One line going
 * stale degrades the check instead of breaking it, and there is a single
 * place to update when a version drifts.
 *
 * Own harness rather than the shared fixtures, like `persistence.spec.ts`:
 * this leaves a real JVM running mid-test, and must be free to kill it
 * without touching an app other specs are sharing.
 *
 * **File name.** `gameLaunch` sorts after `dbRecovery`, whose position first
 * is load-bearing: its process cleanup must not run while another spec's app
 * is alive. It owns its runtime path outright, so it cannot pre-satisfy any
 * artifact assertion in another spec.
 *
 * **Cost and portability.** A full startup under software GL took roughly 90
 * seconds when this was written. It is also the only spec that needs working
 * OpenGL, so it is the first candidate to become Linux-only if the Windows
 * or macOS runners turn out to lack it.
 */

import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  isCoreModulePresent,
  launchApp,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  clickPlayAndAwaitLaunched,
  createInstanceViaUi,
  STOP_TIMEOUT,
  waitForInstallComplete
} from "./helpers/instances.js"
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
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

const INSTANCE_NAME = "gdl-e2e-game-launch"
const MC_VERSION = "1.20.1"
const LOADER = "forge"

test.describe("game launch", () => {
  // eslint-disable-next-line no-empty-pattern
  test("starts Minecraft and reaches a stable main menu", async ({}, testInfo) => {
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

        await test.step("wait for the client to finish loading", async () => {
          await waitForLogQuiescence(
            logsDir,
            () => closedCount() === closedBeforeLaunch
          )
        })

        await test.step("assert it is idle at a menu, not dead", async () => {
          // Still running. This is the assertion that separates "reached the
          // main menu" from "crashed and stopped writing".
          expect(
            closedCount(),
            "the game exited before the test could observe it idling at the " +
              "main menu"
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
              "client stayed alive and went quiet, so either it idled somewhere " +
              "that is not the main menu, or these markers have drifted with a " +
              "Minecraft version and need updating in gameLog.ts."
          ).toBeGreaterThanOrEqual(LAUNCH_MARKER_QUORUM)
        })
      },
      async (alreadyFailed) => {
        if (current) {
          try {
            const before = closedCount()

            // Only stop what is still running. The tile's play control doubles
            // as stop *while the instance runs* (`Tile.tsx`'s `handlePlay`
            // calls killInstance when isRunning) — so clicking it
            // unconditionally would **launch a fresh game** in exactly the
            // case where the body failed because the client had already died,
            // and then wait out the full timeout for a close that cannot come.
            // Observed doing precisely that. A GAME_CLOSED since launch is the
            // signal that there is nothing left to kill.
            if (before > closedBeforeLaunch) {
              console.log(
                `"${INSTANCE_NAME}" already stopped on its own; nothing to kill`
              )
            } else {
              const tile = current.page.locator(byInstanceName(INSTANCE_NAME))
              await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

              await expect
                .poll(() => closedCount(), {
                  timeout: STOP_TIMEOUT,
                  message:
                    "the game never reported a new GAME_CLOSED after its stop " +
                    "control was clicked — a Minecraft process may have been " +
                    "left running"
                })
                .toBeGreaterThan(before)
            }
          } catch (cleanupError) {
            // See `withCleanup`'s doc comment (`helpers/cleanup.ts`): only
            // re-throw over a body that itself succeeded, so cleanup failure
            // never buries the real failure.
            reportCleanupFailure(
              cleanupError,
              alreadyFailed,
              `cleanup for "${INSTANCE_NAME}" also failed:`
            )
          }
          await attachCoreLogOnFailure(testInfo, harness.runtimePath)
          await current.app.close()
        }
        await stopHarness(harness)
      }
    )
  })
})
