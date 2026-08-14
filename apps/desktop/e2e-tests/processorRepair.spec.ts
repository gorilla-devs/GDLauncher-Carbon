/**
 * Proves that a Forge instance whose processor-generated libraries have been
 * wiped repairs itself on the next normal launch.
 *
 * This is a regression lock on an already-shipped fix, not a reproduction of
 * a live bug. Forge and NeoForge run install processors that generate patched
 * and SRG client jars into `libraries/` at maven coordinates derived from the
 * loader build's own JSON. The Cache Cleanup modal's "minecraft" scope
 * deletes `assets/`, `libraries/` and `natives/` wholesale
 * (`managers/settings/mod.rs`), so it takes those generated jars with it. The
 * launcher regenerates them whenever the required outputs are missing
 * (`518ab9b5c`), not on the instance's `.setup` marker alone — gating on the
 * marker alone leaves a cleaned cache permanently unlaunchable with
 * "minecraft dependency missing".
 *
 * **Why this clicks Play rather than driving `instance.prepareInstance`.**
 * The fix is the third clause of `run/minecraft.rs`'s gate:
 *
 *     let needs_lock_path = is_setup
 *         || deep_check
 *         || !processor_outputs::missing_files(&required, false).await.is_empty();
 *
 * `deep_check` short-circuits the `||` before `missing_files` is ever
 * consulted. Of the ten `prepare_game` call sites, only two pass
 * `deep_check: false` — `LAUNCH_INSTANCE` (the Play button) and
 * `importer/curseforge.rs`, whose path is instance creation and therefore
 * takes the `is_setup` branch anyway. Every other route, `prepareInstance`
 * included, passes `true`. So a version of this test that drove
 * `prepareInstance` would pass with the entire fix deleted — a test that
 * exercises the code path without depending on the fix it claims to guard.
 * Play is the only route on which the clause is load-bearing.
 *
 * Launching is safe here, and does not depend on Minecraft actually starting:
 * `run/mod.rs` reaches `match launch_account { Some(account) => ... }` only
 * after the install phase has run the processors, so the disk assertion is
 * satisfiable before a game process exists at all. The instance is stopped
 * immediately afterwards via the same Play control, which `handlePlay`
 * (`Library/Instance/index.tsx`) turns into a kill while the instance runs.
 *
 * Like `persistence.spec.ts` and `dbRecovery.spec.ts`, this spec drives
 * `startHarness`/`launchApp`/`stopHarness` directly instead of importing the
 * shared fixtures. That is not a style choice: the cleanup deletes
 * runtime-path-level directories shared by every instance in a worker, so
 * running it against `installedInstance`'s worker-scoped app would destroy
 * the substrate every other mod spec depends on.
 *
 * **File name.** `processorRepair` sorts after `dbRecovery`, whose position
 * first is load-bearing (its process cleanup must not run while another
 * spec's app is alive). A name sorting ahead of it, `cacheCleanup.spec.ts`
 * being the obvious one, would break that.
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
  createInstanceViaUi,
  waitForInstallComplete
} from "./helpers/instances.js"
import {
  listPartialVersionInfoIds,
  readPartialVersionInfo
} from "./helpers/versionCache.js"
import { requiredLibraryPaths } from "./helpers/processorOutputs.js"
import {
  verifyLibrariesAbsent,
  verifyLibrariesPresent
} from "./helpers/installVerify.js"
import { runCacheCleanup } from "./helpers/cacheCleanup.js"

const INSTANCE_NAME = "gdl-e2e-processor-repair"

/** Forge 1.20.1 is the combination `loaderInstall.spec.ts` already pins and
 *  installs cleanly, and Forge/NeoForge are the only loaders that run install
 *  processors at all — Fabric and Quilt never populate `processors`, so
 *  neither can exercise this path. Forge over NeoForge because its build list
 *  for a pinned Minecraft version is stable, where "newest supported
 *  NeoForge" moves. */
const MC_VERSION = "1.20.1"
const LOADER = "forge"

/** The cleanup deleted `assets/` and `libraries/` wholesale, so the repairing
 *  launch re-downloads the entire asset index and library set for 1.20.1
 *  before the processors run again — hundreds of megabytes against live
 *  CDNs, not just the handful of generated jars this test asserts on. */
const REGEN_TIMEOUT = 600_000

/** Minecraft exits promptly once killed, but it is a real JVM shutting down
 *  a real window, so this is generous rather than tight. */
const GAME_STOP_TIMEOUT = 60_000

async function goToLibrary(page: Page): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}

test.describe("processor output repair", () => {
  // eslint-disable-next-line no-empty-pattern
  test("regenerates wiped processor libraries on the next launch", async ({}, testInfo) => {
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
    /** The core's stdout, captured so cleanup can wait on the process-level
     *  GAME_CLOSED event rather than on a UI state that races it. */
    let stdout: string[] = []

    try {
      current = await launchApp(launchOpts)
      const page = current.page
      stdout = current.stdout
      await completeLogin(page, harness)
      await dismissStartupModals(page)

      let paths: string[] = []

      await test.step("install a Forge instance", async () => {
        // The app's own default build is used rather than a pinned or seeded
        // one: this spec asserts a repair path, not version coverage, and
        // `loaderInstall.spec.ts` already carries the seeded-matrix coverage.
        // Pinning a Forge build here would only add a string that Forge's
        // release cadence eventually invalidates.
        await createInstanceViaUi(page, {
          name: INSTANCE_NAME,
          version: MC_VERSION,
          loader: LOADER
        })
        await waitForInstallComplete(page, INSTANCE_NAME)

        // Which build the app picked is discovered rather than assumed. This
        // runtime path is this test's alone and has had exactly one Forge
        // instance installed into it, so exactly one `forge-` cache row must
        // exist — asserted, because two would mean the id below was a guess
        // between them and zero would mean the install never cached anything.
        const forgeIds = listPartialVersionInfoIds(
          harness.runtimePath,
          `${LOADER}-`
        )
        expect(
          forgeIds,
          "expected exactly one cached Forge build on a runtime path with " +
            "exactly one Forge instance"
        ).toHaveLength(1)
        const loaderVersion = forgeIds[0].slice(`${LOADER}-`.length)

        const cached = readPartialVersionInfo(harness.runtimePath, forgeIds[0])
        const required = requiredLibraryPaths(
          cached.processors ?? [],
          cached.data
        )
        // Explicit rather than a bare `if (required.length)` guard: deriving
        // zero would otherwise silently turn every assertion below into a
        // no-op over an empty list, and the whole test would pass having
        // checked nothing at all.
        expect(
          required.length,
          `expected forge-${loaderVersion} to declare at least one client ` +
            "processor artifact — derived zero. Either this build genuinely " +
            "stopped declaring any (a real finding), or requiredLibraryPaths " +
            "mis-derived the set (check processorOutputsGolden.test.ts)."
        ).toBeGreaterThan(0)
        paths = required.map((r) => r.relativePath)

        const present = await verifyLibrariesPresent(harness.runtimePath, paths)
        if (!present.ok) {
          throw new Error(
            `processor-generated libraries missing straight after installing ` +
              `forge-${loaderVersion}, before this test wiped anything:\n` +
              present.problems.map((p) => `  - ${p}`).join("\n")
          )
        }
      })

      await test.step("wipe the Minecraft cache", async () => {
        // `gdlauncher: false` is load-bearing, not tidiness: that scope wipes
        // DB tables, including this instance's own row.
        await runCacheCleanup(page, { gdlauncher: false, minecraft: true })

        // Without this the final assertion is satisfied by files the cleanup
        // never touched, and the test would pass just as happily against a
        // cleanup that deleted nothing.
        const absent = await verifyLibrariesAbsent(harness.runtimePath, paths)
        if (!absent.ok) {
          throw new Error(
            "the minecraft cache cleanup did not delete the processor-" +
              "generated libraries, so the regeneration assertion that " +
              "follows would prove nothing:\n" +
              absent.problems.map((p) => `  - ${p}`).join("\n")
          )
        }
      })

      await test.step("launch, and assert the libraries are rebuilt", async () => {
        await goToLibrary(page)
        const tile = page.locator(byInstanceName(INSTANCE_NAME))
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

        await expect
          .poll(
            async () =>
              (await verifyLibrariesPresent(harness.runtimePath, paths)).ok,
            {
              timeout: REGEN_TIMEOUT,
              message:
                "processor-generated libraries were not rebuilt after " +
                "launching an instance whose cache had been wiped. This is " +
                "the regression this spec exists to catch: run/minecraft.rs " +
                "must re-run the processors when missing_files() is " +
                "non-empty, even though deep_check is false on the launch " +
                "path."
            }
          )
          .toBe(true)
      })
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      if (current) {
        try {
          // The launch really does start Minecraft — `_INSTANCE_STATE_:
          // GAME_LAUNCHED` is observable in the core's stdout — so a real JVM
          // is left running and has to be stopped, or it outlives the test.
          // The tile's play control doubles as the stop control while the
          // instance runs: `Tile.tsx`'s `handlePlay` calls `killInstance`
          // when `props.isRunning`, and the `instance-play` anchor stays
          // mounted, swapping only its icon and label.
          const tile = current.page.locator(byInstanceName(INSTANCE_NAME))
          const play = tile.locator(byTestId(TEST_IDS.instancePlay))

          // Counted, not merely searched for. `change_launch_state`
          // (`run/mod.rs`) prints GAME_CLOSED on every transition to
          // Inactive, and the *install* ends with one of those — so this
          // string is already in stdout long before anything launches, and a
          // plain `includes("GAME_CLOSED")` is satisfied instantly, waits for
          // nothing, and lets teardown race the kill it was supposed to be
          // waiting on. Only a new occurrence means this stop happened.
          const closedCount = () => stdout.join("").split("GAME_CLOSED").length
          const before = closedCount()

          await play.click()

          await expect
            .poll(() => closedCount(), {
              timeout: GAME_STOP_TIMEOUT,
              message:
                "the launched instance never reported a new GAME_CLOSED " +
                "after its stop control was clicked — a Minecraft process " +
                "may have been left running"
            })
            .toBeGreaterThan(before)

          // The instance is deliberately not deleted. This spec owns its
          // runtime path outright and `stopHarness` removes the whole
          // temporary tree, so a delete would buy nothing while adding a
          // second UI interaction — one that has to race the same
          // running-to-stopped transition — to teardown.
        } catch (cleanupError) {
          // See instanceInstall.spec.ts's identical branch: only re-throw
          // over a body that itself succeeded, so cleanup failure never
          // buries the real failure.
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(
            `cleanup for "${INSTANCE_NAME}" also failed:`,
            cleanupError
          )
        }
        await attachCoreLogOnFailure(testInfo, harness.runtimePath)
        await current.app.close()
      }
      await stopHarness(harness)
    }
  })
})
