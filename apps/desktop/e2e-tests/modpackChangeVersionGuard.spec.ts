import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
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
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

/**
 * Covers `change_modpack`
 * (`crates/carbon_app/src/managers/instance/modpack/mod.rs:139-207`) started
 * while the game is running — the one half of this feature's launch-state
 * story that `modpackReinstall.spec.ts` does not reach.
 *
 * **The guard, and what it protects.** `change_modpack` and
 * `repair_modpack` carry the same `LaunchState` match
 * (`modpack/mod.rs:156-163` and `234-241`): either call bails immediately,
 * before touching `.setup/` at all, unless the instance is `Inactive`. For
 * `change_modpack` that guard sits *before* the `setup_path.exists()` check,
 * `create_dir_all`, and the `change-pack-version.json` write
 * (`modpack/mod.rs:176-190`), so a call made while the instance is
 * launching, queued, running, or being deleted leaves nothing on disk at
 * all — not a leak, not a deferred apply, nothing for a later launch to
 * find. `repair_modpack`'s guard was already closing this gap on its own
 * path; this file is what proves `change_modpack` now matches it, since
 * `modpackReinstall.spec.ts` never drives `change_modpack` itself.
 *
 * **The refusal reaches the user.** `handleUpdate`
 * (`ModPackVersionUpdate/index.tsx:170-201`) wraps the mutation in a
 * `try/catch`: a rejection sets an inline-error signal instead of closing the
 * modal and navigating to `/library`, and the modal renders it under
 * `data-testid="modpack-version-update-error"`. Pinned below by asserting
 * that testid becomes visible and the route never leaves the instance
 * detail — proof the user is actually told something, not just that nothing
 * navigated out from under them.
 *
 * That behaviour also dictates this file's shape. Because the page stays on
 * the Settings route behind a modal overlay, there is **no instance tile on
 * screen** after the refusal, so the running-state check is made against the
 * core's own `GAME_CLOSED` stream rather than a tile attribute — the first
 * draft asserted the tile and reported "the instance left the running state"
 * when the guard had in fact held perfectly, which is precisely the class of
 * misleading red this suite's sabotage checks exist to catch.
 * For the same reason a second attempt is driven *inside* the open modal
 * rather than navigating back to it, and the modal is explicitly cancelled
 * only once both attempts have been observed.
 *
 * **A second attempt refuses identically to the first.** Unlike the old
 * `setup_path.exists()` bail it replaced, the `LaunchState` guard reads
 * instance state rather than a marker file on disk, so there is no "the
 * first call poisons `.setup/` for the second" distinction left to prove —
 * both calls take the exact same branch for the exact same reason, and the
 * guard itself never inspects which version was requested. The modal is
 * still open from the first refusal with a version still selected, so the
 * second attempt clicks `modpackVersionUpdateConfirm` directly instead of
 * reopening the version select: the ad SDK element `TopBannerAd`/`AdsBanner`
 * render can tile duplicated ad content over the whole page during this
 * file's longer real-launch run (a found, documented product bug — see the
 * README), which pushes the select "outside of the viewport" and makes it
 * unclickable. Resubmitting the still-selected version proves the same
 * guard behaviour a second, different selection would. Asserts the same two
 * things as the first attempt: the inline error reappears, and
 * `change-pack-version.json` still does not exist.
 *
 * **Own harness**, like `modpackLifecycle.spec.ts` and
 * `modpackReinstall.spec.ts`'s second test: this leaves a real JVM running
 * and must be free to kill it without disturbing an app other specs share.
 * The `startHarness`/`stopHarness` try/finally with inline `stdout` capture is
 * **copied** from `modpackReinstall.spec.ts`, not imported — importing any
 * value from a `.spec.ts` re-registers that file's own `test()` calls (see
 * `helpers/resolutionFixtures.ts`).
 *
 * `installModpackVersion` asserts its way to the version row rather than
 * retrying towards it (`helpers/modpacks.ts`), so this file does not wrap it.
 * Wrapping it is actively wrong here: a local `installModpackVersionRetrying`
 * once retried the *whole* install, which turned one lost row into a double
 * install. The row loss it was covering was
 * `InfiniteScrollVersionsQueryWrapper` tearing the list down on an unchanged
 * scope, and that is fixed in the product now.
 *
 * **Sabotage result.** `change_modpack`'s own `LaunchState` guard
 * (`modpack/mod.rs:156-163`) and `prepare_game`'s independent
 * `LaunchState::Running(_)` bail (`run/mod.rs:194-196`) are redundant layers
 * for a *running* instance, and the `.setup/` cleanup that follows either one
 * (`modpack/mod.rs:200-204`) fires unconditionally on any `Err`. So this file
 * cannot discriminate deleting just `change_modpack`'s own guard: `.setup/`
 * and `change-pack-version.json` still get written, `prepare_game` still
 * bails — sub-second, well inside `SECOND_ATTEMPT_SETTLE` — cleanup still
 * runs, and the mutation still rejects, just with a different message. The
 * leak-absence poll stays green and the inline error still appears; a
 * single-guard sabotage here is invisible to this test.
 *
 * What this file does trip: removing **both** guards (or otherwise letting
 * `prepare_game` proceed while running) resurrects the deferred-apply bug —
 * `.setup/` survives, and the remain-MID relaunch assertion goes red once a
 * later launch picks it up. Removing the `try`/`catch` in `handleUpdate`
 * (`ModPackVersionUpdate/index.tsx:170-201`) kills the inline-error
 * assertion instead, independent of which backend guard is in play.
 */

/** Mirrors `modpackReinstall.spec.ts`'s `LAUNCH_TIMEOUT`. */
const LAUNCH_TIMEOUT = 180_000

/** Mirrors `modpackReinstall.spec.ts`'s `STOP_TIMEOUT`. */
const STOP_TIMEOUT = 60_000

/** How long `.setup/` is given to stay swept — i.e. absent — once a launch
 *  has had the chance to create it. The guard refuses before `create_dir_all`
 *  ever runs, so in practice this bound is never approached; kept generous
 *  rather than re-tuned tight, since a slow pass here is far cheaper than a
 *  false red on the assertion that is this file's whole point. */
const SETUP_SWEEP_TIMEOUT = 120_000

/** Settle window used wherever this test must prove a fixed-window absence
 *  or no-change rather than wait for a positive signal — that
 *  `change-pack-version.json` never appears after a refused change, and that
 *  a later, unrelated launch never moves the pinned version off what the
 *  guard left it at. Long enough that a call which *did* write, or a launch
 *  which *did* re-pin, would have done so. */
const SECOND_ATTEMPT_SETTLE = 3_000

/** Navigates to the instance's Settings tab and opens the version modal. */
async function openVersionModal(
  page: Page,
  instanceName: string
): Promise<void> {
  await openInstanceSettings(page, instanceName)
  await page.click(byTestId(TEST_IDS.instanceSettingsChangeVersion))
}

/** Closes the still-open modal via its Cancel button, which carries no test
 *  id — matched by role and its English label (`_trn_cancel_export`,
 *  "Cancel"). Only one modal is ever open here. */
async function cancelVersionModal(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: "Cancel", exact: true })
    .last()
    .click({ timeout: 15_000 })
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}

test.describe("modpack change version guard", () => {
  // eslint-disable-next-line no-empty-pattern
  test("a version change started mid-game is refused, not deferred", async ({}, testInfo) => {
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
    let name: string | undefined
    /** `GAME_CLOSED` count immediately before Play is clicked, so `finally`
     *  can tell "still running" from "already stopped" — same as
     *  `gameLaunch.spec.ts`. */
    let closedBeforeLaunch = 0

    // Counted rather than searched for: a plain modpack install also ends with
    // a GAME_CLOSED transition to Inactive, so an unscoped `.includes()` would
    // be satisfied before the game ever launches.
    const closedCount = () => stdout.join("").split("GAME_CLOSED").length
    const launchedCount = () => stdout.join("").split("GAME_LAUNCHED").length

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

        name = await installModpackVersion(
          page,
          MODPACK_MR_QUERY,
          "modrinth",
          MODPACK_MR_V_MID
        )
        const { shortpath } = readInstanceByName(harness.runtimePath, name)
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const setupDir = path.join(root, ".setup")
        const pendingFile = path.join(setupDir, "change-pack-version.json")

        expect(
          fs.existsSync(setupDir),
          "the install left `.setup/` behind, so this test cannot tell a leak " +
            "from leftover install state"
        ).toBe(false)

        const tile = page.locator(byInstanceName(name))
        closedBeforeLaunch = closedCount()
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

        await expect
          .poll(() => launchedCount(), {
            timeout: LAUNCH_TIMEOUT,
            message:
              "the core never reported GAME_LAUNCHED after Play was clicked"
          })
          .toBeGreaterThan(1)
        await expect(
          tile,
          "the instance never reached the running state after GAME_LAUNCHED"
        ).toHaveAttribute("data-instance-state", "running")

        // Drive the modal directly — NOT changeModpackVersion, whose last step
        // is waitForInstallComplete, exactly what must not be reached here.
        await openVersionModal(page, name)
        await pickModpackVersionAndConfirm(page, MODPACK_MR_V_NEW)

        // The guard refuses before touching .setup — nothing may appear on disk.
        await page.waitForTimeout(SECOND_ATTEMPT_SETTLE)
        expect(
          fs.existsSync(pendingFile),
          "change_modpack wrote change-pack-version.json while the instance was " +
            "running — the LaunchState guard regressed"
        ).toBe(false)

        // The game must still be running: the guard bails before `prepare_game`
        // is ever called, so this is not `prepare_game`'s own LaunchState check
        // holding — it is `change_modpack` refusing outright, earlier. Asserted
        // from the core's own state stream rather than the instance tile,
        // because a refused change leaves the page on the instance's Settings
        // route with the modal still open — there is no tile on screen to
        // read, and asserting one here would report "the instance left the
        // running state" for a routing reason while the guard was in fact
        // holding perfectly.
        expect(
          closedCount(),
          "the game stopped after a mid-game version change — change_modpack's " +
            "LaunchState guard did not hold"
        ).toBe(closedBeforeLaunch)

        // The refusal must reach the user. `handleUpdate`
        // (`ModPackVersionUpdate/index.tsx`) catches the rejected mutation and
        // renders an inline error in the still-open modal instead of closing it
        // and navigating away.
        const firstErrorLocator = page.locator(
          byTestId("modpack-version-update-error")
        )
        await expect(
          firstErrorLocator,
          "a refused version change surfaced no inline error in the modal"
        ).toBeVisible({ timeout: 15_000 })

        // The inline error must render the parsed backend message
        // (`extractErrorDisplay` in `rspcClient.ts`), not the raw serialized
        // rspc error — which carries the full axum/tokio backtrace (recognized
        // here by its `::poll` async-state-machine frames) plus a trailing
        // JSON fragment. Both assertions would have failed before that helper
        // existed, when `handleUpdate` rendered `e.message` unparsed.
        await expect(
          firstErrorLocator,
          "the inline error did not contain the backend's refusal message"
        ).toContainText("Cannot change the modpack version")
        await expect(
          firstErrorLocator,
          "the inline error rendered a raw backtrace frame instead of the " +
            "parsed display message"
        ).not.toContainText("::poll")

        expect(page.url()).toMatch(/#\/library\/\d+/)

        // A SECOND attempt refuses identically to the first — the guard reads
        // `LaunchState`, not `.setup/`'s presence, so there is no "the first
        // call poisons `.setup/` for the second" distinction left to observe,
        // and it never inspects which version was requested. Driven inside the
        // modal that is still open, with the version still selected from the
        // first attempt: a direct click of Confirm, not
        // `pickModpackVersionAndConfirm`'s open-select/pick-option/confirm
        // sequence — reopening the version select races the ad banner's
        // duplication bug (found, documented product bug; see the README),
        // which can tile enough ad content over the page to push the select
        // outside the viewport.
        await page.click(byTestId(TEST_IDS.modpackVersionUpdateConfirm))
        await expect(
          page.locator(byTestId("modpack-version-update-error")),
          "a second refused version change surfaced no inline error in the modal"
        ).toBeVisible({ timeout: 15_000 })
        expect(
          fs.existsSync(pendingFile),
          "the second change_modpack call wrote change-pack-version.json " +
            "while the instance was running — the LaunchState guard regressed"
        ).toBe(false)

        // Close the still-open modal and get back to the library grid before
        // touching any tile.
        await cancelVersionModal(page)

        // Stop the game.
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()
        await expect
          .poll(() => closedCount(), {
            timeout: STOP_TIMEOUT,
            message: "the game never stopped after the play/stop toggle"
          })
          .toBeGreaterThan(closedBeforeLaunch)

        const beforeRelaunch = await readInstanceConfig(root)
        expect(
          beforeRelaunch.modpack?.modrinthVersionId,
          "the instance was repinned even though every change_modpack call " +
            "while it was running was refused"
        ).toBe(MODPACK_MR_V_MID)

        // Launch again. THE load-bearing assertion: a version change the guard
        // refused must not resurrect itself on a later, perfectly normal
        // launch — there was never anything queued for it to find.
        closedBeforeLaunch = closedCount()
        const launchedBeforeRelaunch = launchedCount()
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

        await expect
          .poll(() => launchedCount(), {
            timeout: LAUNCH_TIMEOUT,
            message:
              "the core never reported GAME_LAUNCHED after the second Play click"
          })
          .toBeGreaterThan(launchedBeforeRelaunch)

        await page.waitForTimeout(SECOND_ATTEMPT_SETTLE)
        expect(
          (await readInstanceConfig(root)).modpack?.modrinthVersionId,
          "the instance's pinned version moved off MID after a launch that had " +
            "nothing pending — a refused change is re-applying itself from " +
            "somewhere"
        ).toBe(MODPACK_MR_V_MID)

        // ...and `.setup/` was never created in the first place, so there is
        // nothing left for a later launch to sweep.
        await expect
          .poll(() => fs.existsSync(pendingFile), {
            timeout: SETUP_SWEEP_TIMEOUT,
            message:
              "change-pack-version.json survived the launch that consumed it — " +
              "`.setup/` is never swept, so this change would re-apply on every " +
              "subsequent launch"
          })
          .toBe(false)
      },
      async (alreadyFailed) => {
        if (current) {
          try {
            // Only stop what is still running — clicking Play again would
            // launch a fresh game if the body failed after the client died.
            if (name && closedCount() <= closedBeforeLaunch) {
              // Get back to the library grid first. A body that failed anywhere
              // between opening the version modal and cancelling it leaves the
              // page on the instance's Settings route behind a modal overlay,
              // where no tile exists — so the stop click below would time out,
              // the game would outlive the test, and `app.close()` would then
              // sit waiting on a live JVM. That is exactly how the first run of
              // this file hung for thirteen minutes after failing in one second.
              await current.page
                .getByRole("button", { name: "Cancel", exact: true })
                .last()
                .click({ timeout: 5_000 })
                .catch(() => {})
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
        // Best-effort: also sweeps any leftover game process via
        // killGameProcesses(harness.runtimePath).
        await stopHarness(harness)
      }
    )
  })
})
