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
  deleteInstanceViaUi,
  ensureLibraryInteractive,
  STOP_TIMEOUT
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import { classifyPackinfo, readPackinfo } from "./helpers/packinfo.js"
import { readInstallAudit } from "./helpers/installAudit.js"
import { snapshotTree } from "./helpers/instanceTree.js"
import {
  changeModpackVersion,
  fetchMrpackIndex,
  installModpackVersion,
  openInstanceSettings,
  pickModpackVersionAndConfirm,
  repairModpack
} from "./helpers/modpacks.js"
import {
  MODPACK_CF_FILE,
  MODPACK_CF_FILE_OLD,
  MODPACK_CF_QUERY,
  MODPACK_MR_QUERY,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW
} from "./helpers/modpackFixtures.js"
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

/**
 * The modpack pipeline interrupted part-way, in the two places it can be.
 * Nothing else in this suite kills the app mid-install; `repair_modpack`'s
 * own doc comment calls the resulting poisoned `.setup/` "the state it exists
 * to fix", so the recovery path is real, reachable, and until now untested.
 *
 * ## Why one test crashes for real and the other reconstructs
 *
 * `process_modpack_staging` has two phases a crash can land in, and they are
 * not equally reachable from a test.
 *
 * The **download** phase is seconds wide for a 28 MiB pack, so test 1 simply
 * waits for `.setup/staging` to start filling and `SIGKILL`s the core. That is
 * a genuine crash — no cleanup, no graceful shutdown — and it produces the
 * interrupted state rather than imitating it.
 *
 * The **apply** phase is one md5 pass plus a handful of renames. Racing it
 * would be flaky, and `retries: 0` (`playwright.config.ts`) turns flaky into a
 * red build. There is also no log signal to time against: the core's
 * `debug!`/`trace!` output does not reach this suite's stdout capture, only
 * `_STATUS_:` lines do. Making that race reliable would need a test-only hook
 * in product code, which this suite has never done.
 *
 * So test 2 reproduces the *consequence* of an interrupted apply instead, and
 * does it without faking a staging tree. The key is that promotion is the last
 * step: `process_modpack` writes its fresh scan to `tmp-packinfo.json`
 * (`run/modpack.rs:638`) and only renames it to `packinfo.json` at :899,
 * *after* the apply. So a crash during the apply leaves the **old** packinfo
 * describing files that are already the **new** version on disk — which is
 * exactly what you get by performing a complete version change and then
 * restoring the packinfo you saved beforehand. Same end state, no race, no
 * synthetic `.setup/`.
 *
 * ## Why the two tests use different packs
 *
 * Test 2 needs at least one file whose bytes genuinely differ between the two
 * versions, because that is the only kind of file that can end up recorded in
 * the stale packinfo under one hash while sitting on disk under another.
 * **`remarkably` has none** — its MID -> NEW delta is pure add/remove, with
 * zero same-path replacements among mods (`modpackFixtures.ts`, and
 * `modpackLifecycle.spec.ts`'s own note at :554). Against that pack the
 * misclassification set would come back empty and the assertion would be
 * vacuous rather than failing honestly.
 *
 * `boosted-fps` `4595849` -> `4713831` has **six measured** shared override
 * paths whose bytes differ (`config/fabric/indigo-renderer.properties`,
 * `config/immediatelyfast.json`, `config/iris.properties`,
 * `config/modernfix-mixins.properties`, `config/sodium-options.json`,
 * `options.txt`), so test 2 runs on CurseForge. It is also 4 MiB rather than
 * 28, which makes it the wrong pack for test 1 — the download window would be
 * too narrow to interrupt reliably. Hence one file, two packs, each chosen for
 * the property the test actually depends on.
 *
 * ## Scoping
 *
 * Test 1 owns its harness (it kills the core and relaunches onto the same
 * runtime path); test 2 uses the shared worker-scoped `authenticatedApp`
 * fixture, since it never launches anything. That mixed shape is the same one
 * `modpackReinstall.spec.ts` uses, and for the same reason: a shared
 * `afterEach` naming `authenticatedApp` would force the worker fixture to be
 * built for the own-harness test too, paying for an app launch it never uses.
 *
 * The `startHarness`/`stopHarness` block is **copied**, not imported —
 * importing any value from a `.spec.ts` re-registers that file's `test()`
 * calls (`helpers/resolutionFixtures.ts`).
 *
 * `installModpackVersion` asserts its way to the version row rather than
 * retrying towards it (`helpers/modpacks.ts`), so this file does not wrap it.
 * Wrapping it is actively wrong here: a local `installModpackVersionRetrying`
 * once retried the *whole* install, which turned one lost row into a double
 * install. The row loss it was covering was
 * `InfiniteScrollVersionsQueryWrapper` tearing the list down on an unchanged
 * scope, and that is fixed in the product now.
 *
 * **Sabotage results.**
 *   1. Test 1: `run/modpack.rs`'s `is_setup && !is_modpack_complete` forced to
 *      `false`, so the relaunch never re-runs the modpack steps and the
 *      interrupted install is never resumed.
 *   2. Test 2: the `ModifiedByUser` md5 comparison forced to always match
 *      (`run/modpack.rs:776`), so nothing is classed modified-by-user and the
 *      misclassification set comes back empty.
 */

/** How long the staging directory is given to start filling before the test
 *  gives up on having anything to interrupt. */
const STAGING_FILL_TIMEOUT = 120_000

/** How long the resumed change is given to complete after the relaunch: a
 *  full re-download of a 28 MiB pack plus the apply. */
const RESUME_TIMEOUT = 300_000

test.describe("modpack interrupted staging", () => {
  // eslint-disable-next-line no-empty-pattern
  test("a download interrupted by a core crash is resumed on the next launch", async ({}, testInfo) => {
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
    let closedBeforeLaunch = 0

    const closedCount = () => stdout.join("").split("GAME_CLOSED").length

    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
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
        const { shortpath } = readInstanceByName(harness.runtimePath, name)
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const setupDir = path.join(root, ".setup")
        const stagingDir = path.join(setupDir, "staging")

        // Start a version change and deliberately do NOT await it.
        await openInstanceSettings(page, name)
        await page.click(byTestId(TEST_IDS.instanceSettingsChangeVersion))
        await pickModpackVersionAndConfirm(page, MODPACK_MR_V_NEW)

        // Wait for the staging tree to start filling. Seconds wide for this
        // pack, unlike the apply window — see the module doc comment.
        await expect
          .poll(async () => (await snapshotTree(stagingDir)).size, {
            timeout: STAGING_FILL_TIMEOUT,
            message:
              "the staging directory never began filling, so there was nothing " +
              "to interrupt — the version change may have failed outright"
          })
          .toBeGreaterThan(2)

        // Crash the core. SIGKILL rather than app.close(): a graceful close runs
        // main.rs's termination handler, which is the opposite of what this test
        // is about.
        const corePid = await getCoreProcessId(current.app)
        expect(
          corePid,
          "could not read the core process id, so it cannot be crashed"
        ).not.toBeNull()
        process.kill(corePid!, "SIGKILL")
        await waitForPidExit(corePid!)

        // The interrupted state, asserted before anything is allowed to touch it.
        expect(
          fs.existsSync(setupDir),
          "the crash left no `.setup/`, so there is nothing to resume from"
        ).toBe(true)
        expect(
          fs.existsSync(path.join(setupDir, "modpack-complete")),
          "the pipeline reported itself complete despite being killed mid-download"
        ).toBe(false)
        expect(
          (await readInstanceConfig(root)).modpack?.modrinthVersionId,
          "the instance was repinned to the new version before the change had " +
            "finished — a crash here would strand it claiming a version it does " +
            "not have on disk"
        ).toBe(MODPACK_MR_V_MID)

        // Relaunch onto the same runtime path and resume by launching.
        //
        // No `completeLogin` here: the enrolled account lives in the runtime
        // path this relaunch reuses, so the app boots straight to the library
        // and there is no `#auth-flow` to drive — the same reason
        // `persistence.spec.ts` follows its own `relaunchApp` with
        // `dismissStartupModals` alone.
        current = await relaunchApp(current, launchOpts)
        page = current.page
        stdout = current.stdout
        await dismissStartupModals(page)
        await ensureLibraryInteractive(page)

        const tile = page.locator(byInstanceName(name))
        closedBeforeLaunch = closedCount()
        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

        await expect
          .poll(
            async () =>
              (await readInstanceConfig(root)).modpack?.modrinthVersionId,
            {
              timeout: RESUME_TIMEOUT,
              message:
                "the interrupted version change never completed after a " +
                "relaunch — the instance is stuck at the old version with a " +
                "poisoned `.setup/` and no way forward from the UI"
            }
          )
          .toBe(MODPACK_MR_V_NEW)

        // **The version pin is written before the files are placed**, so it is
        // not a valid signal that the resume has finished. `process_modpack`
        // updates `instance.json` and invalidates `GET_MODPACK_INFO` around
        // `run/modpack.rs:600`, well before `process_modpack_staging` runs its
        // two passes, sweeps `staging/` (:898) and promotes `tmp-packinfo.json`
        // (:899) — and `.setup/` itself is only removed later still, by the
        // launch path that called all of it (`run/mod.rs:527-528`).
        //
        // Asserting the tree on the pin alone raced that gap and produced a
        // thoroughly convincing false positive: a full-suite run reported the
        // five mods the target adds as missing, while the install audit written
        // moments later listed those same five under `Files created:` and
        // `staging/` was already empty. The pipeline had placed them; the
        // snapshot was simply taken first. Waiting for `.setup/` to disappear
        // is the one signal that means every phase is done.
        await expect
          .poll(() => fs.existsSync(setupDir), {
            timeout: RESUME_TIMEOUT,
            message:
              "`.setup/` was never swept after the resumed change, so the " +
              "pipeline did not run to completion — the instance is left " +
              "claiming the new version with an unfinished install behind it"
          })
          .toBe(false)

        // Verify against the pack's own index — an oracle wholly external to the
        // app. Modrinth's CDN is key-free, so this is available here; it is not
        // on the CurseForge path (see modpackCurseforgeVersion.spec.ts).
        const index = await fetchMrpackIndex(MODPACK_MR_V_NEW)
        const tree = await snapshotTree(path.join(root, "instance"))
        const missing = index.files
          .map((f) => f.path)
          .filter((p) => !tree.has(p))
          .sort()

        // Diagnostics, emitted only when the assertion is about to fail. A
        // full-suite run on 2026-08-02 failed here with exactly the five mods
        // the target version *adds* missing and none of the twenty it shares
        // with the source — a categorical split, not a partially-completed
        // download. It has not reproduced since (four clean runs, including one
        // with the crash forced as early as possible), so the next occurrence
        // needs to say more than which files were lost: whether staging
        // survived, and what the apply pass believed it created.
        if (missing.length > 0) {
          const midIndex = await fetchMrpackIndex(MODPACK_MR_V_MID)
          const midPaths = new Set(midIndex.files.map((f) => f.path))
          const stagingLeft = fs.existsSync(stagingDir)
          console.error(
            "interrupted-resume diagnostics:\n" +
              `  missing total:           ${missing.length}\n` +
              `  ...that NEW adds:        ${missing.filter((p) => !midPaths.has(p)).length}\n` +
              `  ...shared with MID:      ${missing.filter((p) => midPaths.has(p)).length}\n` +
              `  .setup/ still present:   ${fs.existsSync(setupDir)}\n` +
              `  staging/ still present:  ${stagingLeft}\n` +
              `  staged files left:       ${stagingLeft ? (await snapshotTree(stagingDir)).size : 0}\n` +
              `  audit created:           ${JSON.stringify((await readInstallAudit(root))?.created ?? null)}`
          )
        }

        expect(
          missing,
          "the resumed change did not land every file the target version declares"
        ).toEqual([])
      },
      async (alreadyFailed) => {
        if (current) {
          try {
            if (name && closedCount() <= closedBeforeLaunch) {
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

  test("a version change whose packinfo promotion was lost leaves the new files untouchable", async ({
    authenticatedApp
  }, testInfo) => {
    const { page, harness } = authenticatedApp
    let name: string | undefined

    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
        name = await installModpackVersion(
          page,
          MODPACK_CF_QUERY,
          "curseforge",
          MODPACK_CF_FILE_OLD
        )
        const { shortpath } = readInstanceByName(harness.runtimePath, name)
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const packinfoPath = path.join(root, "packinfo.json")

        // The old packinfo, captured exactly as it stands before the change.
        const oldPackinfoText = await fs.promises.readFile(packinfoPath, "utf8")

        await changeModpackVersion(page, name, MODPACK_CF_FILE)

        // Restore it. `process_modpack` writes its scan to tmp-packinfo.json and
        // renames it over packinfo.json only at run/modpack.rs:803, after the
        // apply — so this is precisely the state a crash during the apply
        // leaves: new files on disk, old record of them.
        await fs.promises.writeFile(packinfoPath, oldPackinfoText)

        const stale = await readPackinfo(root)
        // The paths whose current on-disk bytes no longer match the stale
        // packinfo's recorded hash — the population the restore above exists
        // to produce: bytes that are already the NEW version's, tracked under
        // the OLD version's hash. `classifyPackinfo` reads `packinfo.json` at
        // call time, which right now is the stale content just written above.
        const staleTracked = (await classifyPackinfo(root)).modified
        expect(
          staleTracked.length,
          "no pack file in the stale packinfo actually differs from its " +
            "current on-disk bytes — the fixture no longer proves anything " +
            "about repairing a stale record; re-measure boosted-fps' OLD -> " +
            "target delta"
        ).toBeGreaterThan(0)
        const beforeRepair = await snapshotTree(path.join(root, "instance"))

        await repairModpack(page, name)

        const afterRepair = await snapshotTree(path.join(root, "instance"))
        const audit = await readInstallAudit(root)
        expect(audit, "the repair pass wrote no install audit").not.toBeNull()

        // None of them are misclassified `modified-by-user` any more. This is
        // the half-upgraded state an interrupted apply strands an instance in
        // — and the fix for it: the repair reconciles against the target
        // version it is repairing onto, not blindly against packinfo's stale
        // record, so a path whose on-disk bytes already match that target is
        // recognised as already correct rather than as a user edit.
        const misclassified = staleTracked.filter((key) =>
          audit!.skipped.some(
            (s) => s.file === key && s.reason === "modified-by-user"
          )
        )
        expect(
          misclassified,
          "a pack file already reconciled by the earlier version change was " +
            "still misclassified modified-by-user against the stale packinfo " +
            "— the repair is reconciling against the recorded version instead " +
            "of the target it is repairing onto"
        ).toEqual([])

        for (const key of staleTracked) {
          // Bytes were already the target's before the repair ran, so "the
          // repair left them untouched" and "the repair recognised them as
          // already correct" are the same fact, checked two ways.
          expect(
            afterRepair.get(key.slice(1))?.sha256,
            `${key}'s bytes changed across the repair, even though they were ` +
              "already correct before it ran"
          ).toBe(beforeRepair.get(key.slice(1))?.sha256)
          expect(
            audit!.unchanged,
            `audit did not record ${key} as unchanged`
          ).toContain(key)
        }

        // packinfo was promoted: the repair's own fresh scan now records
        // these paths' actual hashes, not the stale ones restored into
        // packinfo.json above.
        const packinfoAfterRepair = await readPackinfo(root)
        for (const key of staleTracked) {
          expect(
            packinfoAfterRepair.get(key)?.md5,
            `packinfo still records ${key} under its stale hash after the repair`
          ).not.toBe(stale.get(key)?.md5)
        }
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
              `cleanup for "${name}" also failed:`
            )
          }
        }
        await ensureLibraryInteractive(page)
      }
    )
  })
})
