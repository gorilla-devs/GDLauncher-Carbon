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
import {
  deleteInstanceViaUi,
  ensureLibraryInteractive
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { classifyPackinfo, packinfoDataPath } from "./helpers/packinfo.js"
import { readInstallAudit } from "./helpers/installAudit.js"
import { snapshotTree } from "./helpers/instanceTree.js"
import {
  fetchMrpackIndex,
  installModpackVersion,
  openInstance,
  repairModpack
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_SLUG,
  MODPACK_MR_V_MID
} from "./helpers/modpackFixtures.js"
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

/**
 * Covers "Repair" — the instance overflow menu's repair action
 * (`Library/Instance/index.tsx`'s `menuItems()`, `instanceMenuRepair`) —
 * which re-runs the modpack download/staging pipeline, in true repair mode,
 * against the instance's own **current** version. Two behaviours, one test
 * each:
 *
 * 1. It restores a pack file the user deleted AND repairs one they damaged —
 *    the same treatment for both, not an asymmetry.
 * 2. It refuses to run at all while the instance is launching, queued,
 *    running, or being deleted (`modpack/mod.rs`'s `LaunchState` match inside
 *    `repair_modpack`), so it can never race a live `.setup/` directory a
 *    running instance depends on.
 *
 * **Mechanism.** `repair_modpack` writes a `.setup/repair` marker
 * (`RepairMarkerFile`, JSON) alongside `change-pack-version.json`.
 * `process_modpack` (`run/modpack.rs`) checks for that marker before
 * deciding the skip-optimisation oracle it hands to the platform prep
 * functions: marker present -> `disk_scan::scan_instance_as_packinfo`, a
 * live scan of what is actually on disk right now; marker absent -> the
 * recorded `packinfo.json`, unchanged from an ordinary version change.
 * `process_modpack_staging` selects `apply_plan::ApplyMode::Repair` whenever
 * the marker was present. Together these mean a deleted or damaged path is
 * never skip-optimised away — its disk bytes (if any exist at all) cannot
 * match the pack manifest's declared hash, so a fresh copy is always
 * re-fetched into staging — and `apply_plan::decide_repair` then reconciles
 * every pack-tracked path against the **target** version alone, regardless
 * of what the possibly-stale `old` record says: present-and-correct stays
 * `Keep`/`Unchanged`; present-but-wrong (with a staged replacement) becomes
 * `Replace`/`RepairOverwrote`; missing (with a staged replacement) becomes
 * `Create`/`RepairRestored`. `apply_plan::plan` decides each path exactly
 * once, so there is a single `PlanEntry` — and a single audit line — per
 * path, never two independent passes that can contradict each other.
 *
 * Every key this test mutates is picked at runtime from `classifyPackinfo`'s
 * own `pristine` list, cross-referenced against the live `.mrpack` index's
 * own `overrides` — never a hardcoded filename — and each carries a named
 * `toBeDefined()` check (never a bare `!`) so a future re-pin of
 * `modpackFixtures.ts` fails loudly here instead of throwing an unrelated
 * `TypeError`. They are drawn from overrides rather than `.files`-declared
 * mods only because overrides are simplest to seed (extracted unconditionally
 * on every pass, `modrinth.rs`'s override loop has no packinfo/oracle check
 * at all) — not because of any remaining behavioural difference: repair's
 * disk-scanned oracle means a deleted or damaged mod is re-fetched into
 * staging exactly like a deleted or damaged override is, so either would
 * prove the same three assertions below.
 *
 * **Test 2 — refused while running.** Not implemented with `repairModpack`
 * (`helpers/modpacks.ts`): that helper's last step is
 * `waitForInstallComplete`, which is exactly what must never be reached here
 * — the whole point is that the mutation is rejected before it touches
 * anything. This drives the same three clicks
 * (`instanceMenuTrigger` -> `instanceMenuRepair` -> `repairModpackConfirm`)
 * directly and asserts the refusal instead: the tile must still read
 * `running`, and `.setup/` — which `repair_modpack` only ever creates
 * *after* its `LaunchState::Inactive` guard — must not exist.
 *
 * **`.setup/`'s absence is the load-bearing half of that pair, and the
 * still-running check is only corroborating.** `repair_modpack`'s guard is
 * defence in depth over `prepare_game`'s own
 * `LaunchState::Running(_) => bail!` (`run/mod.rs:194-196`), which every path
 * into this feature ends at. Remove `repair_modpack`'s guard and
 * `prepare_game` still refuses, so the instance genuinely does stay running
 * and the first assertion stays green — but by then `repair_modpack` has
 * already `remove_dir_all`'d and recreated `.setup/` and written
 * `change-pack-version.json` into it, on a live instance. That leak is what
 * this test actually detects, and it is the same leak `change_modpack`
 * exhibits unguarded today (see the README's product findings): a mid-game
 * version change is not cancelled, it is deferred to the next launch.
 * `RepairModpack/index.tsx`'s
 * `navigateAwayIfInsideDetail()` runs synchronously on the confirm click,
 * before the mutation is even dispatched. It only navigates when the current
 * route is inside the instance's own detail page, which this flow guarantees
 * by calling `openInstance` first — so it fires here regardless of whether
 * the mutation later succeeds or fails, navigating back to `/library`, which
 * is what lets this test read the tile's `data-instance-state` straight off
 * the library grid afterward, the same way `waitForInstallComplete` itself
 * relies on for a real reinstall.
 *
 * **Own harness for test 2 only, like `modpackLifecycle.spec.ts` —
 * not test 1.** Test 2 leaves a real JVM running and must be free to kill it
 * without disturbing an app other specs share; test 1 launches nothing, so it
 * uses the shared worker-scoped `authenticatedApp` fixture, the same class of
 * test as `modpackSaveGuard.spec.ts`. The harness setup/teardown block in
 * test 2 is **copied**, not imported, from `gameLaunch.spec.ts`'s own
 * `startHarness`/`stopHarness` try/finally with inline `stdout` capture — the
 * established pattern for exactly this
 * (`modpackLifecycle.spec.ts`'s own header explains why: importing a value
 * from a `.spec.ts` file re-registers that file's `test()` calls, and this
 * project has a standing decision not to churn passing specs to avoid that).
 * `helpers/processes.ts`'s `killGameProcesses` is reached transitively via
 * `stopHarness`, the final safety net if the graceful in-body stop below
 * never runs at all.
 *
 * `installModpackVersion` asserts its way to the version row rather than
 * retrying towards it (`helpers/modpacks.ts`), so this file does not wrap it.
 * Wrapping it is actively wrong here: a local `installModpackVersionRetrying`
 * once retried the *whole* install, which turned one lost row into a double
 * install. The row loss it was covering was
 * `InfiniteScrollVersionsQueryWrapper` tearing the list down on an unchanged
 * scope, and that is fixed in the product now.
 */

/** Mirrors `gameLaunch.spec.ts`'s `FIRST_OUTPUT_TIMEOUT` /
 *  `modpackLifecycle.spec.ts`'s `LAUNCH_TIMEOUT`. */
const LAUNCH_TIMEOUT = 180_000

/** Mirrors `gameLaunch.spec.ts`'s `GAME_STOP_TIMEOUT`. */
const STOP_TIMEOUT = 60_000

test.describe("modpack reinstall", () => {
  test("reinstalling restores a deleted pack file and repairs a damaged one", async ({
    authenticatedApp
  }, testInfo) => {
    const { page, harness } = authenticatedApp
    let name: string | undefined
    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
        const index = await fetchMrpackIndex(MODPACK_MR_V_MID)
        name = await installModpackVersion(
          page,
          MODPACK_MR_QUERY,
          "modrinth",
          MODPACK_MR_V_MID
        )
        const { shortpath } = readInstanceByName(harness.runtimePath, name)
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const data = path.join(root, "instance")

        // All three keys are pristine OVERRIDES — see the module doc comment
        // for why (simplest to seed; no remaining behavioural difference from
        // a `.files`-declared mod under repair).
        const status = await classifyPackinfo(root)
        const overridePaths = new Set(index.overrides)
        const overrideCandidates = status.pristine.filter((k) =>
          overridePaths.has(k.slice(1))
        )
        const deletedKey = overrideCandidates[0]
        const corruptKey = overrideCandidates[1]
        const editKey = overrideCandidates[2]
        for (const [label, key] of [
          ["a first pristine override to delete", deletedKey],
          ["a second pristine override to truncate", corruptKey],
          ["a third pristine override to edit", editKey]
        ] as const) {
          expect(
            key,
            `no pristine pack override was available as ${label} in ` +
              `"${MODPACK_MR_SLUG}" — the remarkably fixture's shape must ` +
              "have changed; re-measure it"
          ).toBeDefined()
        }

        const deletedDeclared = index.overrideFiles.find(
          (f) => f.path === deletedKey.slice(1)
        )
        expect(
          deletedDeclared,
          `"${deletedKey}" is in classifyPackinfo's pristine list and in ` +
            `${MODPACK_MR_V_MID}'s own declared overrides, but ` +
            "parseMrpackIndex's overrideFiles has no entry for it"
        ).toBeDefined()

        const before = await snapshotTree(data)

        // Delete one pristine override, truncate a second, and edit a third.
        await fs.promises.rm(packinfoDataPath(root, deletedKey))
        await fs.promises.writeFile(packinfoDataPath(root, corruptKey), "")
        const editedBody = "e2e-reinstall-edit\n"
        await fs.promises.writeFile(packinfoDataPath(root, editKey), editedBody)

        await repairModpack(page, name)

        const after = await snapshotTree(data)
        const audit = await readInstallAudit(root)
        expect(audit, "reinstall wrote no install audit").not.toBeNull()

        // Deleted -> restored: byte-identical to both the pristine copy that
        // existed before the deletion and the pack's own declared content, and
        // recorded as created. `render_audit` writes the plan's own path
        // (packinfo-style, leading slash) into every section now — no more
        // staging-relative format to normalise away — so `deletedKey` is
        // compared as-is, never `.slice(1)`'d, against `created`/`replaced`.
        expect(
          after.get(deletedKey.slice(1))?.sha256,
          "reinstall did not restore a pack file the user had deleted"
        ).toBe(before.get(deletedKey.slice(1))?.sha256)
        expect(
          after.get(deletedKey.slice(1))?.sha256,
          "the restored file's bytes do not match the pack's own declared content"
        ).toBe(deletedDeclared!.sha256)
        expect(
          audit!.created,
          `audit did not record creating ${deletedKey}`
        ).toContain(deletedKey)
        expect(
          audit!.skipped.some((s) => s.file === deletedKey),
          `${deletedKey} was restored by the plan, so it must not also appear ` +
            "in the skipped section — repair decides each path exactly once, " +
            "unlike the old two-independent-passes pipeline"
        ).toBe(false)

        // Truncated -> repaired: non-empty again, byte-identical to its own
        // pristine original (captured into `before` before the truncation),
        // and recorded as replaced, not skipped.
        expect(
          after.get(corruptKey.slice(1))?.size,
          "reinstall did not repair a truncated pack file"
        ).toBeGreaterThan(0)
        expect(
          after.get(corruptKey.slice(1))?.sha256,
          "the repaired file's bytes do not match its own pristine original"
        ).toBe(before.get(corruptKey.slice(1))?.sha256)
        expect(
          audit!.replaced,
          `audit did not record repairing ${corruptKey}`
        ).toContain(corruptKey)

        // Edited config -> reset to the pack's own bytes, not preserved.
        const editedFileNow = await fs.promises.readFile(
          packinfoDataPath(root, editKey),
          "utf8"
        )
        expect(
          editedFileNow,
          "reinstall left the user's edit in place instead of repairing it"
        ).not.toBe(editedBody)
        expect(
          after.get(editKey.slice(1))?.sha256,
          "the reset file's bytes do not match its own pristine original"
        ).toBe(before.get(editKey.slice(1))?.sha256)
        expect(
          audit!.replaced,
          `audit did not record repairing ${editKey}`
        ).toContain(editKey)
      },
      async (alreadyFailed) => {
        await attachCoreLogOnFailure(testInfo, harness.runtimePath)
        if (name) {
          try {
            await page
              .locator(byTestId(TEST_IDS.navbarLogo))
              .click({ timeout: 5_000 })
              .catch(() => {})
            await deleteInstanceViaUi(page, name)
          } catch (cleanupError) {
            reportCleanupFailure(
              cleanupError,
              alreadyFailed,
              'cleanup for "reinstalling restores a deleted pack file and ' +
                'repairs a damaged one" also failed:'
            )
          }
        }
        await ensureLibraryInteractive(page)
      }
    )
  })

  // eslint-disable-next-line no-empty-pattern
  test("reinstalling is refused while the game is running", async ({}, testInfo) => {
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
    let root: string | undefined
    /** `GAME_CLOSED` count at the moment before Play is clicked. Declared out
     *  here because `finally` needs it to tell "still running" from
     *  "already stopped" — same as `gameLaunch.spec.ts`. */
    let closedBeforeLaunch = 0

    // Counted rather than searched for: a plain modpack install also ends
    // with a GAME_CLOSED transition to Inactive, so an unscoped `.includes()`
    // would be satisfied before the game ever launches — see
    // `gameLaunch.spec.ts`'s header.
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
        root = path.join(harness.runtimePath, "instances", shortpath)

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

        // Drive the overflow menu directly — NOT repairModpack, which
        // awaits waitForInstallComplete, exactly what must NOT happen when the
        // mutation is refused. openInstance is the same navigation
        // repairModpack itself uses internally; only the wait afterward is
        // skipped.
        await openInstance(page, name)
        await page.click(byTestId(TEST_IDS.instanceMenuTrigger))
        const entry = page.locator(byTestId(TEST_IDS.instanceMenuRepair))
        await expect(
          entry,
          `the repair menu entry was disabled for "${name}" — the instance ` +
            "has no modpack association"
        ).toBeEnabled()
        await entry.click()
        await page.click(byTestId(TEST_IDS.repairModpackConfirm))

        // The mutation rejects; the instance must still be running and its
        // files untouched. `repairModpackConfirm`'s click synchronously
        // navigates back to /library (see the module doc comment), which is
        // what makes the tile locator resolve here.
        await expect(
          page.locator(byInstanceName(name)),
          "reinstall was not refused while the instance was running — it " +
            "left the running state"
        ).toHaveAttribute("data-instance-state", "running")
        expect(
          fs.existsSync(path.join(root, ".setup")),
          "reinstall created .setup/ even though the instance was running — " +
            "the running-state guard did not fire"
        ).toBe(false)
      },
      async (alreadyFailed) => {
        if (current) {
          try {
            // Only stop what is still running — same guard
            // `modpackLifecycle.spec.ts`/`gameLaunch.spec.ts` use: clicking
            // Play again would launch a fresh game in the case where the body
            // failed because the client had already died.
            if (name && closedCount() <= closedBeforeLaunch) {
              const tile = current.page.locator(byInstanceName(name))
              await tile
                .locator(byTestId(TEST_IDS.instancePlay))
                .click({ timeout: 5_000 })
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
        // killGameProcesses(harness.runtimePath), the final safety net if the
        // graceful stop above never ran at all.
        await stopHarness(harness)
      }
    )
  })
})
