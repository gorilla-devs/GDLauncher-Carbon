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
  reinstallModpack
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_SLUG,
  MODPACK_MR_V_MID
} from "./helpers/modpackFixtures.js"

/**
 * Covers "Reinstall" — the instance overflow menu's repair action
 * (`Library/Instance/index.tsx`'s `menuItems()`, `instanceMenuReinstall`) —
 * which re-runs the modpack download/staging pipeline against the instance's
 * own **current** version. Two behaviours, one test each:
 *
 * 1. It repairs a missing pack file but preserves a damaged one — the exact
 *    same asymmetry, not a fix, per the controller's ruling below.
 * 2. It refuses to run at all while the instance is launching, queued,
 *    running, or being deleted (`modpack/mod.rs:210-217`'s `LaunchState`
 *    match inside `reinstall_modpack`), so it can never race a live
 *    `.setup/` directory a running instance depends on.
 *
 * **The repair/preserve asymmetry, traced through
 * `run/modpack.rs:747-823`.** `process_modpack_staging` decides file by file
 * in two independent passes: a first loop over `packinfo.files` that can only
 * ever *skip* a file (never repair one on its own), and a second loop that
 * walks whatever physically landed in the staging directory and moves a
 * staged file into any path that is currently empty.
 *
 *   - A **truncated/corrupted** file still exists on disk and its md5 no
 *     longer matches packinfo's recorded value, so the first loop classes it
 *     `ModifiedByUser` (`:776-783`) and `continue`s — never replaced, and the
 *     second loop's own guard (`:815`, `!original_file.exists()`) is false
 *     for it regardless, since the path is occupied. **Reinstall does not
 *     repair a corrupted file; it preserves the corruption as a user
 *     modification.**
 *   - A **deleted** file hits `DeletedByUser` (`:759-764`) and is skipped by
 *     the first loop the same way, but the second loop *does* recreate it —
 *     its path is empty, so `!original_file.exists()` is true — **provided
 *     something was actually staged at that path in the first place.**
 *
 * That proviso is the one thing neither the plan nor the brief's own Step 1
 * sample code accounts for, and it materially changed which files this test
 * can use — see the next two sections.
 *
 * **Corollary, found empirically writing this test and not previously
 * documented anywhere in this wave: on a same-version reinstall specifically
 * (as opposed to a genuine version change), a `.files`-declared mod can
 * *never* be resurrected once deleted — only an override can.** The staging
 * directory starts empty (`modpack.rs:336`) and is filled two ways:
 * downloads (`modrinth.rs:261-319`) and unconditional override extraction
 * (`modrinth.rs:321-379`, no packinfo check at all, ever). A download is
 * skipped — nothing physically staged — whenever packinfo's already-recorded
 * hash for that path matches what the *target* version declares
 * (`modrinth.rs:277-289`'s `existing_path`/`skip` optimization, the same one
 * `modpackLifecycle.spec.ts` and `modpackLock.spec.ts` already document for a
 * genuine version bump). Reinstall's target version is, by definition, the
 * instance's own current version, so that comparison is packinfo-against-
 * itself and trivially always matches for any file the user has not
 * corrupted the *record* of — meaning **every** `.files`-declared mod is
 * skip-optimized on every reinstall, unconditionally, regardless of what is
 * or is not on disk. A deleted mod therefore has nothing staged for the
 * second loop to find, and stays deleted forever: confirmed live, deleting a
 * pristine mod and reinstalling left it absent, with an empty `Files
 * created:` section and no `Files deleted:` entry either — the pipeline
 * simply never touches it again. An override has no such optimization, so it
 * is *always* freshly re-extracted into staging on every pass and is always
 * available for the second loop to move into a path the user emptied.
 * Confirmed the other direction too: deleting a pristine override and
 * reinstalling brought it back byte-identical, audited in *both*
 * `Files that could not be replaced:` (`deleted by user`) and
 * `Files created:` — the same "two independent passes, so a deletion is
 * silently reinstated" oddity `modpackLifecycle.spec.ts` already pins for a
 * version change, now confirmed for a same-version reinstall too. This is
 * why `deletedKey` below is drawn from `index.overrides`, not `/mods/` as
 * the brief's Step 1 snippet does — literally following the brief here would
 * pin a false claim about reinstall specifically (it would still be true for
 * a genuine version change, which is a different code path this file does
 * not exercise).
 *
 * **`corruptKey` and `editKey` are overrides too, not mods, for a second and
 * separate reason: sabotage provability.** A truncated *mod* is genuinely,
 * correctly left untouched by real code (confirmed live, same as an
 * override) — but proving that assertion is load-bearing needs a sabotage
 * that can flip it, and the natural one-token sabotage here (disabling the
 * `ModifiedByUser` md5 comparison at `:776`) only defeats the *decision*, not
 * the skip-optimization that governs whether anything is staged to repair
 * *with*. Against a mod, nothing is ever staged (see above), so disabling
 * the md5 check changes nothing observable — the assertion would stay green
 * under a real behavioural regression, which is worse than not having it.
 * Against an override, a correct staged copy is *always* waiting, so
 * disabling the check lets it silently overwrite the "corruption" — a real,
 * provable flip. Using overrides for all three keys, rather than mixing in a
 * mod, is what makes one surgical sabotage (`:776`) able to prove both the
 * "truncated -> not repaired" and "edited -> preserved" halves in a single
 * pass, and keeps the mechanism identical across all three assertions rather
 * than needing three different stories.
 *
 * Every key is picked at runtime from `classifyPackinfo`'s own `pristine`
 * list, cross-referenced against the live `.mrpack` index's own `overrides`
 * — never a hardcoded filename — and each carries a named
 * `toBeDefined()` check (never a bare `!`) so a future re-pin of
 * `modpackFixtures.ts` fails loudly here instead of throwing an unrelated
 * `TypeError`.
 *
 * **The audit's path-format split** (`Files created:` carries
 * staging-relative, no-leading-slash, `instance/`-prefixed paths; the other
 * three sections carry packinfo's own leading-slash keys — see
 * `helpers/installAudit.ts`'s module doc) is normalised at every comparison
 * site below with the same `.replace(/^instance\//, "")` this suite already
 * uses elsewhere, so a comparison against the wrong format cannot pass
 * vacuously.
 *
 * **Test 2 — refused while running.** Not implemented with `reinstallModpack`
 * (`helpers/modpacks.ts`): that helper's last step is
 * `waitForInstallComplete`, which is exactly what must never be reached here
 * — the whole point is that the mutation is rejected before it touches
 * anything. This drives the same three clicks
 * (`instanceMenuTrigger` -> `instanceMenuReinstall` -> `confirmReinstallConfirm`)
 * directly and asserts the refusal instead: the tile must still read
 * `running`, and `.setup/` — which `reinstall_modpack` only ever creates
 * *after* its `LaunchState::Inactive` guard — must not exist.
 *
 * **`.setup/`'s absence is the load-bearing half of that pair, and the
 * still-running check is only corroborating.** `reinstall_modpack`'s guard is
 * defence in depth over `prepare_game`'s own
 * `LaunchState::Running(_) => bail!` (`run/mod.rs:194-196`), which every path
 * into this feature ends at. Remove `reinstall_modpack`'s guard and
 * `prepare_game` still refuses, so the instance genuinely does stay running
 * and the first assertion stays green — but by then `reinstall_modpack` has
 * already `remove_dir_all`'d and recreated `.setup/` and written
 * `change-pack-version.json` into it, on a live instance. That leak is what
 * this test actually detects, and it is the same leak `change_modpack`
 * exhibits unguarded today (see the README's product findings): a mid-game
 * version change is not cancelled, it is deferred to the next launch.
 * `ConfirmReinstall.tsx`'s
 * `navigateAwayIfInsideDetail()` runs synchronously on the confirm click,
 * before the mutation is even dispatched, and unconditionally navigates back
 * to `/library` regardless of whether the mutation later succeeds or fails —
 * which is what lets this test read the tile's `data-instance-state`
 * straight off the library grid afterward, the same way
 * `waitForInstallComplete` itself relies on for a real reinstall.
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
 * `installModpackVersion` retries reaching and clicking the version row
 * internally (`helpers/modpacks.ts`), so this file does not wrap it. It used
 * to: a local `installModpackVersionRetrying` retried the *whole* install,
 * which turned one lost row into a double install — see that helper's own
 * comment for the full-suite run that demonstrated it.
 *
 * **Sabotage results.** Three, each one surgical edit against a provably
 * clean build, each reverted and sha256-confirmed byte-identical to HEAD
 * before the next.
 *   1. **`run/modpack.rs:776`**, forcing the `ModifiedByUser` md5 comparison
 *      to always match (`if false` in place of
 *      `original_md5 != oldfilehash.md5`). Red at the truncated-file
 *      assertion: *"reinstall repaired a truncated pack file"*,
 *      `Expected: 0 / Received: 54` — 54 being the pristine override's real
 *      size, so it was genuinely re-staged over. Because that assertion sits
 *      below them, this also proves the five above it execute and pass
 *      against a changed product: audit-not-null, deleted-restored,
 *      declared-sha256 match, `Files created:`, and `deleted-by-user`.
 *   2. **`run/modpack.rs:815`**, the staging walk's file creation, gated so
 *      it fires only on a reinstall:
 *      `&& !instance_root.join("packinfo.json").exists()`. Red at
 *      *"reinstall did not restore a pack file the user had deleted"*,
 *      `Received: undefined`.
 *
 *      The gate is **not** decoration. Disabling that guard outright — the
 *      obvious sabotage, and the one first attempted — does not weaken a
 *      reinstall at all; it stops the instance ever installing, and this test
 *      dies at its own precondition with zero assertions executed.
 *      `process_modpack` writes its scan to `tmp-packinfo.json`
 *      (`run/modpack.rs:638`) and only renames it to `packinfo.json` at :899,
 *      *after* the staging apply, so on a **fresh install** the packinfo read
 *      at :739-743 returns `None`, the packinfo pass is skipped entirely, and
 *      the staging walk is performing 100% of the file placement. Gating on
 *      `packinfo.json` existing is what confines the sabotage to the
 *      second-and-later passes.
 *   3. **`modpack/mod.rs:210-217`**, deleting `reinstall_modpack`'s
 *      `LaunchState` refusal (`_ => { bail!(…) }` → `_ => {}`). Red at
 *      *"reinstall created .setup/ even though the instance was running"*,
 *      `Expected: false / Received: true` — with test 2's still-running
 *      assertion above it passing, exactly as the section on that pair
 *      predicts.
 */

/** Mirrors `gameLaunch.spec.ts`'s `FIRST_OUTPUT_TIMEOUT` /
 *  `modpackLifecycle.spec.ts`'s `LAUNCH_TIMEOUT`. */
const LAUNCH_TIMEOUT = 180_000

/** Mirrors `gameLaunch.spec.ts`'s `GAME_STOP_TIMEOUT`. */
const STOP_TIMEOUT = 60_000

test.describe("modpack reinstall", () => {
  test("reinstalling restores a deleted pack file but preserves a damaged one", async ({
    authenticatedApp
  }, testInfo) => {
    const { page, harness } = authenticatedApp
    let bodyFailed = false
    let name: string | undefined
    try {
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

      // All three keys are pristine OVERRIDES, not mods — see the module doc
      // comment for why a mod cannot prove either half of this test on a
      // same-version reinstall.
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

      await reinstallModpack(page, name)

      const after = await snapshotTree(data)
      const audit = await readInstallAudit(root)
      expect(audit, "reinstall wrote no install audit").not.toBeNull()

      // Deleted -> restored: byte-identical to both the pristine copy that
      // existed before the sabotage and the pack's own declared content, and
      // recorded as created.
      expect(
        after.get(deletedKey.slice(1))?.sha256,
        "reinstall did not restore a pack file the user had deleted"
      ).toBe(before.get(deletedKey.slice(1))?.sha256)
      expect(
        after.get(deletedKey.slice(1))?.sha256,
        "the restored file's bytes do not match the pack's own declared content"
      ).toBe(deletedDeclared!.sha256)
      expect(
        audit!.created.map((p) => p.replace(/^instance\//, "")),
        `audit did not record creating ${deletedKey}`
      ).toContain(deletedKey.slice(1))
      expect(
        audit!.skipped.find((s) => s.file === deletedKey)?.reason,
        `audit reason for ${deletedKey}`
      ).toBe("deleted-by-user")

      // Truncated -> NOT repaired. Pinned deliberately; see the module doc
      // comment for why this is correct behaviour, not a bug.
      expect(
        after.get(corruptKey.slice(1))?.size,
        "reinstall repaired a truncated pack file — the product behaviour " +
          "this test pins has changed, which is good news but needs the " +
          "assertion and the module doc comment updated rather than deleted"
      ).toBe(0)
      expect(
        audit!.skipped.find((s) => s.file === corruptKey)?.reason,
        `audit reason for ${corruptKey}`
      ).toBe("modified-by-user")

      // Edited config -> preserved.
      expect(
        await fs.promises.readFile(packinfoDataPath(root, editKey), "utf8"),
        "reinstall overwrote a user-edited config"
      ).toBe(editedBody)
      expect(
        audit!.skipped.find((s) => s.file === editKey)?.reason,
        `audit reason for ${editKey}`
      ).toBe("modified-by-user")
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      await attachCoreLogOnFailure(testInfo, harness.runtimePath)
      if (name) {
        try {
          await page
            .locator(byTestId(TEST_IDS.navbarLogo))
            .click({ timeout: 5_000 })
            .catch(() => {})
          await deleteInstanceViaUi(page, name)
        } catch (cleanupError) {
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(
            'cleanup for "reinstalling restores a deleted pack file but ' +
              'preserves a damaged one" also failed:',
            cleanupError
          )
        }
      }
      await ensureLibraryInteractive(page)
    }
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
    let bodyFailed = false
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

      // Drive the overflow menu directly — NOT reinstallModpack, which
      // awaits waitForInstallComplete, exactly what must NOT happen when the
      // mutation is refused. openInstance is the same navigation
      // reinstallModpack itself uses internally; only the wait afterward is
      // skipped.
      await openInstance(page, name)
      await page.click(byTestId(TEST_IDS.instanceMenuTrigger))
      const entry = page.locator(byTestId(TEST_IDS.instanceMenuReinstall))
      await expect(
        entry,
        `the reinstall menu entry was disabled for "${name}" — the instance ` +
          "has no modpack association"
      ).toBeEnabled()
      await entry.click()
      await page.click(byTestId(TEST_IDS.confirmReinstallConfirm))

      // The mutation rejects; the instance must still be running and its
      // files untouched. `confirmReinstallConfirm`'s click synchronously
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
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
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
      // killGameProcesses(harness.runtimePath), the final safety net if the
      // graceful stop above never ran at all.
      await stopHarness(harness)
    }
  })
})
