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
  MODPACK_MR_PROJECT,
  MODPACK_MR_QUERY,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW,
  MODPACK_MR_V_OLD
} from "./helpers/modpackFixtures.js"

/**
 * Covers `change_modpack`
 * (`crates/carbon_app/src/managers/instance/modpack/mod.rs:137-183`) started
 * while the game is running — the one half of this feature's launch-state
 * story that `modpackReinstall.spec.ts` does not reach.
 *
 * **The finding this pins, stated correctly.** `reinstall_modpack` refuses
 * outright while the instance is launching, queued, running or being deleted
 * (`modpack/mod.rs:210-217`); `change_modpack` has no such guard. The
 * consequence is **not** a mid-game `mods/` rewrite, which is what an earlier
 * reading of this claimed: both functions end at `prepare_game`, and
 * `prepare_game` bails on `LaunchState::Running` of its own accord
 * (`run/mod.rs:194-196`), so no staging pass ever runs under a live JVM.
 *
 * What actually happens is narrower and more interesting. `change_modpack`
 * creates `.setup/` and writes `change-pack-version.json` into it
 * (`modpack/mod.rs:161-176`) **before** it ever calls `prepare_game`, so the
 * refusal arrives too late to undo either. The version change is therefore
 * not cancelled — it is **deferred**. The next launch finds
 * `change-pack-version.json` sitting in `.setup/` and applies it, at a moment
 * the user never asked for and with no UI having reported that anything was
 * pending. Meanwhile every further `change_modpack` call bails with
 * "Instance has not completed the setup phase, attempting to change the
 * modpack may irreparably damage it", because `.setup/` now exists.
 *
 * That whole sequence is what this test walks, in order: the leak, the second
 * call's refusal, and finally the deferred application on the next launch —
 * which is the load-bearing assertion, since it is the part with real user
 * consequences.
 *
 * **A second finding, discovered by this test's own first run: a refused
 * version change tells the user nothing at all.** `handleUpdate`
 * (`ModPackVersionUpdate/index.tsx:126-151`) awaits the mutation and only
 * afterwards calls `closeModal()` and `navigate("/library")`. There is no
 * `catch` anywhere on that path, so when `change_modpack` rejects — which is
 * exactly what happens mid-game — both are skipped: the modal stays open,
 * unchanged, with no toast, no inline error, and no indication that anything
 * failed or that a change is now pending on disk. The user is left looking at
 * a dialog that simply did not respond to the button they pressed. Pinned
 * below by asserting the route never leaves the instance detail.
 *
 * That behaviour also dictates this file's shape. Because the page stays on
 * the Settings route behind a modal overlay, there is **no instance tile on
 * screen** after the refusal, so the running-state check is made against the
 * core's own `GAME_CLOSED` stream rather than a tile attribute — the first
 * draft asserted the tile and reported "the instance left the running state"
 * when the guard had in fact held perfectly, which is precisely the class of
 * misleading red this suite's sabotage checks exist to catch.
 * For the same reason the second attempt is driven *inside* the open modal
 * rather than navigating back to it, and the modal is explicitly cancelled
 * before any tile is touched.
 *
 * **The second call's refusal is observed on disk, not through the UI.**
 * `change_modpack` bails at its `if setup_path.exists()` check
 * (`modpack/mod.rs:163-167`), which sits *before* both the `create_dir_all`
 * and the `write_file_atomic` below it — so a bailed call provably cannot
 * have touched `change-pack-version.json`. Asserting the file is byte-for-byte
 * what the first call wrote is therefore a sound proxy for "the second call
 * bailed", and a far better one than the rspc error, which the renderer does
 * not surface anywhere this test can read.
 *
 * `change-pack-version.json`'s shape is read off `PackVersionFile`
 * (`modpack/mod.rs:253-264`), which is `#[serde(tag = "platform")]` with no
 * `rename_all`: fields stay snake_case and the tag carries the PascalCase
 * variant name, so the file is exactly
 * `{"platform":"Modrinth","project_id":"…","version_id":"…"}`. Asserted as a
 * whole object rather than by probing one key, so a change to either
 * convention fails loudly instead of silently comparing `undefined`.
 *
 * **Own harness**, like `modpackLifecycle.spec.ts` and
 * `modpackReinstall.spec.ts`'s second test: this leaves a real JVM running
 * and must be free to kill it without disturbing an app other specs share.
 * The `startHarness`/`stopHarness` try/finally with inline `stdout` capture is
 * **copied** from `modpackReinstall.spec.ts`, not imported — importing any
 * value from a `.spec.ts` re-registers that file's own `test()` calls (see
 * `helpers/resolutionFixtures.ts`).
 *
 * `installModpackVersion` retries reaching and clicking the version row
 * internally (`helpers/modpacks.ts`), so this file does not wrap it. It used
 * to: a local `installModpackVersionRetrying` retried the *whole* install,
 * which turned one lost row into a double install — see that helper's own
 * comment for the full-suite run that demonstrated it.
 *
 * **Sabotage result — inverted, and deliberately so.** This test pins a
 * *missing* guard, so the sabotage **adds** the `LaunchState` match
 * `reinstall_modpack` already has to `change_modpack`. `.setup/` is then never
 * created and the `change-pack-version.json` poll goes red. That is exactly
 * what should happen the day someone fixes this bug, and the assertion's own
 * message says so — a tripwire, not decoration. See `task-1b-2-report.md`.
 */

/** Mirrors `modpackReinstall.spec.ts`'s `LAUNCH_TIMEOUT`. */
const LAUNCH_TIMEOUT = 180_000

/** Mirrors `modpackReinstall.spec.ts`'s `STOP_TIMEOUT`. */
const STOP_TIMEOUT = 60_000

/** How long the deferred change is given to apply on the next launch: a full
 *  28 MiB re-download plus the staging apply, on top of the launch that
 *  triggers it. Generous because a slow red here is far better than a false
 *  one — the assertion it guards is this file's whole point. */
const CHANGE_APPLY_TIMEOUT = 300_000

/** How long `change_modpack` is given to leak `.setup/` after the confirm
 *  click. It writes the file before ever calling `prepare_game`, so this is
 *  a fast path; the allowance covers the rspc round trip only. */
const LEAK_TIMEOUT = 15_000

/** How long `.setup/` is given to be swept after the deferred change lands.
 *  Separate from `CHANGE_APPLY_TIMEOUT` because it measures a different gap:
 *  the config is written inside `process_modpack`, while the directory is
 *  removed later, by the launch path that called it. */
const SETUP_SWEEP_TIMEOUT = 120_000

/** Settle window after the second change attempt, before asserting the
 *  pending file is untouched. Long enough that a call which *did* write would
 *  have done so. */
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
  test("a version change started mid-game is deferred, not cancelled", async ({}, testInfo) => {
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
    let bodyFailed = false
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

    try {
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

      // The leak, pinned as current behaviour.
      await expect
        .poll(() => fs.existsSync(pendingFile), {
          timeout: LEAK_TIMEOUT,
          message:
            "change_modpack did not write change-pack-version.json while the " +
            "instance was running. If a LaunchState guard was just added to " +
            "change_modpack, that is a FIX and this test pins the old " +
            "behaviour — update this assertion and the README's product " +
            "findings rather than deleting either"
        })
        .toBe(true)

      const pendingFirst = JSON.parse(
        await fs.promises.readFile(pendingFile, "utf8")
      )
      expect(
        pendingFirst,
        "change-pack-version.json does not describe the version that was " +
          "selected, in the shape PackVersionFile serialises"
      ).toEqual({
        platform: "Modrinth",
        project_id: MODPACK_MR_PROJECT,
        version_id: MODPACK_MR_V_NEW
      })

      // The game must still be running: `prepare_game` refused, even though
      // `change_modpack` had already done its damage. Asserted from the core's
      // own state stream rather than the instance tile, because a refused
      // change leaves the page on the instance's Settings route with the modal
      // still open — there is no tile on screen to read, and asserting one
      // here would report "the instance left the running state" for a routing
      // reason while the guard was in fact holding perfectly.
      expect(
        closedCount(),
        "the game stopped after a mid-game version change — prepare_game's " +
          "own LaunchState guard did not hold"
      ).toBe(closedBeforeLaunch)

      // A refused change tells the user nothing. `handleUpdate`
      // (`ModPackVersionUpdate/index.tsx:126-151`) awaits the mutation and
      // only then calls `closeModal()` and `navigate("/library")`, with no
      // catch anywhere — so a rejection skips both and the modal just sits
      // there, unchanged, no toast, no error text. Pinned as current
      // behaviour; see the README's product findings.
      expect(
        page.url(),
        "a refused version change navigated away from the instance — " +
          "handleUpdate must have grown error handling, which is a fix; " +
          "update this assertion and the README rather than deleting it"
      ).toMatch(/#\/library\/\d+/)

      // A SECOND attempt bails at `if setup_path.exists()`, which precedes
      // both the create_dir_all and the write — so the pending file must be
      // untouched, still naming the FIRST target. Driven inside the modal
      // that is still open, since nothing can navigate past its overlay.
      await pickModpackVersionAndConfirm(page, MODPACK_MR_V_OLD)
      await page.waitForTimeout(SECOND_ATTEMPT_SETTLE)
      expect(
        JSON.parse(await fs.promises.readFile(pendingFile, "utf8")),
        "a second change_modpack call overwrote the pending version while " +
          "`.setup/` already existed — it should have bailed on " +
          '"Instance has not completed the setup phase"'
      ).toEqual(pendingFirst)

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

      const beforeDeferred = await readInstanceConfig(root)
      expect(
        beforeDeferred.modpack?.modrinthVersionId,
        "the instance was repinned before the deferred change ever ran"
      ).toBe(MODPACK_MR_V_MID)

      // Launch again. THE load-bearing assertion: the change queued mid-game
      // applies itself now, unprompted.
      closedBeforeLaunch = closedCount()
      await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

      await expect
        .poll(
          async () =>
            (await readInstanceConfig(root)).modpack?.modrinthVersionId,
          {
            timeout: CHANGE_APPLY_TIMEOUT,
            message:
              "the version change queued mid-game never applied on the next " +
              "launch — it was discarded rather than deferred, which would " +
              "make this a smaller bug than the one documented"
          }
        )
        .toBe(MODPACK_MR_V_NEW)

      // ...and the pending file is consumed, so the change does not re-apply
      // on every future launch. Polled, not checked instantly: nothing deletes
      // `change-pack-version.json` on its own — it goes away only when
      // `.setup/` as a whole is removed (`run/mod.rs:527-528`), which happens
      // after `process_modpack` has already written the new version into
      // `instance.json`. So the config flips first and the directory is swept
      // a moment later, and an instant check here races that gap.
      await expect
        .poll(() => fs.existsSync(pendingFile), {
          timeout: SETUP_SWEEP_TIMEOUT,
          message:
            "change-pack-version.json survived the launch that consumed it — " +
            "`.setup/` is never swept, so this change would re-apply on every " +
            "subsequent launch"
        })
        .toBe(false)
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
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
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error("cleanup: stopping the game also failed:", cleanupError)
        }
        await attachCoreLogOnFailure(testInfo, harness.runtimePath)
        await current.app.close()
      }
      // Best-effort: also sweeps any leftover game process via
      // killGameProcesses(harness.runtimePath).
      await stopHarness(harness)
    }
  })
})
