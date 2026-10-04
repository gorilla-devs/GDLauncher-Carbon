import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  getCoreProcessId,
  isCoreModulePresent,
  launchApp,
  relaunchApp,
  waitForPidExit,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  clickPlayAndAwaitLaunched,
  ensureLibraryInteractive,
  STOP_TIMEOUT,
  waitForInstallComplete
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import {
  installModpackVersion,
  openInstanceSettings,
  pickModpackVersionAndConfirm
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW
} from "./helpers/modpackFixtures.js"
import { armInstallOperation } from "./helpers/operationObserver.js"
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

/**
 * An instance left with an update that stopped while applying is never
 * stuck: it can't be cancelled and its changes can't be previewed, but
 * Repair, opened from the library tile, offers "Discard unfinished setup and
 * repair", which retires the unfinished `.setup` (kept as
 * `.setup-cancelled-*`), reinstalls the configured pack, and the instance
 * launches afterwards.
 *
 * The failed update is produced for real, as in
 * `modpackInterruptedStaging.spec.ts`'s first test: a version change is
 * started and the core is SIGKILLed once the new version's archive has
 * landed in `.setup`. The core's own apply marker is then written beside it,
 * which is what an update that stopped part-way through applying its files
 * leaves, the case Cancel can't undo. That spec's
 * `startHarness`/`stopHarness` block is copied rather than imported for the
 * same reason it gives (importing a `.spec.ts` re-registers its tests).
 */

/** How long the version change is given to download the new archive. */
const ARCHIVE_TIMEOUT = 180_000

/** How long the repair is given: a fresh download of the pack plus the
 *  game install. */
const REPAIR_TIMEOUT = 11 * 60_000

test.describe("modpack repair with an unfinished setup", () => {
  // eslint-disable-next-line no-empty-pattern
  test("an update that stopped while applying is discarded by Repair and the instance launches", async ({}, testInfo) => {
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
    let name: string | undefined
    let launched = false
    let closedBeforeLaunch = 0
    const closedCount = () => stdout.join("").split("GAME_CLOSED").length

    await withCleanup(
      async () => {
        current = await launchApp(launchOpts)
        let page = current.page
        stdout = current.stdout
        await completeLogin(page, harness)
        await dismissStartupModals(page)

        name = await installModpackVersion(
          page,
          MODPACK_MR_QUERY,
          "modrinth",
          MODPACK_MR_V_MID
        )
        const { id, shortpath } = readInstanceByName(harness.runtimePath, name)
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const setupDir = path.join(root, ".setup")

        // Start an update and crash the core once its archive arrived.
        await openInstanceSettings(page, name)
        await page.click(byTestId(TEST_IDS.instanceSettingsChangeVersion))
        await pickModpackVersionAndConfirm(page, MODPACK_MR_V_NEW)
        await expect
          .poll(() => fs.existsSync(path.join(setupDir, "modrinth")), {
            timeout: ARCHIVE_TIMEOUT,
            message:
              "the new version's archive never arrived, so there is no " +
              "unfinished update to leave behind"
          })
          .toBe(true)
        const corePid = await getCoreProcessId(current.app)
        expect(
          corePid,
          "could not read the core process id, so it cannot be crashed"
        ).not.toBeNull()
        process.kill(corePid!, "SIGKILL")
        await waitForPidExit(corePid!)
        expect(
          fs.existsSync(setupDir),
          "the crash left no `.setup/`, so there is nothing to repair"
        ).toBe(true)
        // What an apply that started and never finished leaves beside it.
        fs.writeFileSync(path.join(setupDir, "apply-started"), "")

        // The enrolled account lives in the reused runtime path, so the
        // relaunch boots straight to the library (no `completeLogin`).
        current = await relaunchApp(current, launchOpts)
        page = current.page
        stdout = current.stdout
        await dismissStartupModals(page)
        await ensureLibraryInteractive(page)

        // Repair from the tile's context menu ("More options" submenu),
        // opened with the keyboard. Opened by hovering, it closed again
        // within ~100 ms under the resting pointer on macOS, while the menu
        // around it stayed open (run 41799780827, all three attempts); with
        // the keyboard no pointer rests on the menu. Retried whole, from a
        // closed menu, in case the submenu still closes before the click.
        const tile = byInstanceName(name)
        const repairEntry = page.locator(
          byTestId(TEST_IDS.instanceContextRepair)
        )
        await expect(async () => {
          await page.keyboard.press("Escape")
          await page.click(tile, { button: "right" })
          await page.focus(byTestId(TEST_IDS.instanceContextMore))
          await page.keyboard.press("Enter")
          await repairEntry.click({ timeout: 5_000 })
        }, "the More options submenu never let Repair be clicked").toPass({
          timeout: 60_000
        })
        const pendingBlock = page.locator(byTestId(TEST_IDS.repairPendingSetup))
        await expect(
          pendingBlock,
          "the Repair dialog did not show the unfinished update"
        ).toBeVisible()
        // Its changes can't be listed and it can't be cancelled: Repair is
        // the way out, and the dialog says why.
        await expect(pendingBlock).toContainText(/can't be listed in advance/)
        await expect(
          page.locator(byTestId(TEST_IDS.repairCancelPending))
        ).toHaveCount(0)
        await expect(
          page.locator(byTestId(TEST_IDS.repairPreviewCounts))
        ).toHaveCount(0)
        const confirm = page.locator(byTestId(TEST_IDS.repairModpackConfirm))
        await expect(confirm).toHaveText(/Discard unfinished setup and repair/)
        await expect(
          confirm,
          "Repair stayed disabled while an unfinished update was pending"
        ).toBeEnabled({ timeout: 30_000 })

        const operation = armInstallOperation(page, {
          route: "instance.repairModpack",
          instance: id
        })
        try {
          await confirm.click()
          await waitForInstallComplete(page, name, {
            operation,
            installTimeout: REPAIR_TIMEOUT
          })
        } finally {
          operation.dispose()
        }

        expect(
          fs.existsSync(setupDir),
          "the repair finished but left a `.setup/` behind"
        ).toBe(false)
        const retired = fs
          .readdirSync(root)
          .filter((entry) => entry.startsWith(".setup-cancelled-"))
        expect(
          retired,
          "the discarded update was not kept as `.setup-cancelled-*`"
        ).toHaveLength(1)
        // It may have changed files, so its records stay; its download goes
        // once the repair finished.
        const kept = path.join(root, retired[0])
        expect(fs.existsSync(path.join(kept, "setup-session.json"))).toBe(true)
        expect(
          fs.existsSync(path.join(kept, "modrinth")),
          "the discarded update's archive outlived the repair that replaced it"
        ).toBe(false)
        expect(
          (await readInstanceConfig(root)).modpack?.modrinthVersionId,
          "Repair reinstalls the configured version; the update never applied"
        ).toBe(MODPACK_MR_V_MID)

        await ensureLibraryInteractive(page)
        closedBeforeLaunch = closedCount()
        await clickPlayAndAwaitLaunched(page, name, {
          stdout,
          message: "the repaired instance never launched"
        })
        launched = true
      },
      async (alreadyFailed) => {
        if (current) {
          try {
            if (name && launched && closedCount() <= closedBeforeLaunch) {
              await current.page
                .locator(byTestId(TEST_IDS.navbarLogo))
                .click({ timeout: 5_000 })
                .catch(() => {})
              const tile = current.page.locator(byInstanceName(name))
              await tile
                .locator(byTestId(TEST_IDS.instancePlay))
                .click({ timeout: 10_000 })
              await expect
                .poll(() => closedCount(), { timeout: STOP_TIMEOUT })
                .toBeGreaterThan(closedBeforeLaunch)
            }
          } catch (cleanupError) {
            reportCleanupFailure(
              cleanupError,
              alreadyFailed,
              "cleanup: stopping the game also failed:"
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
