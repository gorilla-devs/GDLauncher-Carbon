import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  deleteInstanceViaUi,
  ensureLibraryInteractive
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import { readPackinfo, type Packinfo } from "./helpers/packinfo.js"
import { snapshotTree, type Tree } from "./helpers/instanceTree.js"
import {
  changeModpackVersion,
  installModpackVersion
} from "./helpers/modpacks.js"
import {
  MODPACK_CF_FILE,
  MODPACK_CF_FILE_OLD,
  MODPACK_CF_PROJECT,
  MODPACK_CF_QUERY
} from "./helpers/modpackFixtures.js"
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

/**
 * The only CurseForge **version change** in this suite. Everything else that
 * changes a modpack's version — `modpackLifecycle.spec.ts`,
 * `modpackLock.spec.ts`, `modpackReinstall.spec.ts` — runs on Modrinth, and
 * CurseForge goes through a different `prepare_modpack_from_*` implementation
 * (`managers/minecraft/curseforge.rs`, not `modrinth.rs`).
 * `modpackInstall.spec.ts` installs CurseForge once and never upgrades it.
 *
 * ## The oracle problem, and the twin
 *
 * `modpackInstall.spec.ts`'s header explains why its CurseForge test asserts
 * only internal consistency: `packinfo::scan_dir` hashes the **staging** copy
 * of each file, *before* the rename into its final location, so comparing
 * on-disk bytes against packinfo is self-referential. It can catch a change
 * made after the install (a user edit), never a bug that staged the wrong
 * bytes in the first place.
 *
 * The Modrinth answer — fetch the version's own `.mrpack` index live and
 * assert against it — does not port. CurseForge file downloads require an
 * `x-api-key` (see the 2026-07-19 CDN incident), and this suite holds no such
 * key in standalone mode.
 *
 * **So the oracle is a twin instance.** A *fresh install* of the target file
 * is produced by the install path; the subject reaches the same target
 * through the version-change path. Comparing them asserts the property that
 * matters most about the whole feature and that nothing here proves today,
 * not even for Modrinth: **a version change lands you where a fresh install
 * of the target would have.** No API key, and the oracle is genuinely
 * external to the code under test.
 *
 * ## Why the twin is built and torn down *before* the subject exists
 *
 * The obvious shape — install both, then compare — cannot work, and would
 * fail confusingly rather than usefully. `next_folder`
 * (`managers/instance/mod.rs:1563`) de-duplicates an instance's **shortpath**
 * (its directory), but nothing de-duplicates its **display name**: two
 * installs of one modpack produce two rows with the identical `name`. That
 * breaks three things at once — `newestTileName` diffs tile names and would
 * see no new name at all, `byInstanceName` would match two tiles under
 * Playwright's strict mode, and `readInstanceByName` throws outright on a
 * duplicate (deliberately: its own doc comment says every caller in this
 * suite creates a uniquely-named instance, so a duplicate is a test bug worth
 * surfacing).
 *
 * Running them sequentially — install the twin, snapshot it, delete it, then
 * build the subject — sidesteps all three without renaming anything through
 * the UI, and costs only the delete that cleanup would have done anyway.
 *
 * ## What is asserted, and what a failure means
 *
 * The tree comparison is exact, path-for-path and sha256-for-sha256. It is
 * predicted from reading `process_modpack_staging`: the apply planner
 * (`apply_plan::plan`, `crates/carbon_app/src/managers/instance/modpack/apply_plan.rs`)
 * reconciles `old ∪ target` against disk and staged content in a single pass
 * — files only OLD ships are pristine and get deleted, files only the target
 * ships are staged and get created, shared files whose bytes differ are
 * replaced. **A divergence here is a finding, not a broken assertion** —
 * characterise it and pin it the way `modpackLifecycle.spec.ts` pins its own
 * findings, rather than loosening this to `toContain`.
 *
 * `packinfo.json` is expected to match the twin's exactly, and that is
 * asserted rather than assumed. `scan_dir` still rebuilds packinfo from
 * what physically landed in staging, and the skip-if-unchanged download
 * optimisation still means a file whose recorded hash already matches the
 * target's is never re-staged — but `process_modpack`'s snapshot block now
 * merges the skip-oracle's hash back in for every such skip-optimised path
 * before packinfo is written, so nothing drops out of the version-changed
 * instance's packinfo. Confirmed
 * here on the CurseForge path, independent of the Modrinth-side confirmation
 * in `modpackLifecycle.spec.ts`.
 *
 * Neither instance is ever launched, so nothing rewrites configs and every
 * packinfo entry stays pristine by construction.
 *
 * `installModpackVersion` asserts its way to the version row rather than
 * retrying towards it (`helpers/modpacks.ts`), so this file does not wrap it.
 * Wrapping it is actively wrong here: a local `installModpackVersionRetrying`
 * once retried the *whole* install, which turned one lost row into a double
 * install. The row loss it was covering was
 * `InfiniteScrollVersionsQueryWrapper` tearing the list down on an unchanged
 * scope, and that is fixed in the product now.
 *
 * **Sabotage result.** `run/modpack.rs`'s pass-1 deletion branch — the
 * `tokio::fs::remove_file(original_file)` that removes a pristine file the
 * target version does not ship — commented out, leaving its `deleted_files`
 * push and `continue` in place. The failure message names the two jars
 * 1.1.9 ships as overrides and 1.2.0 drops.
 */

function rootFor(runtimePath: string, name: string): string {
  const { shortpath } = readInstanceByName(runtimePath, name)
  return path.join(runtimePath, "instances", shortpath)
}

/** `Tree` -> sorted `[path, sha256]` pairs, so a mismatch prints the offending
 *  paths instead of an opaque `Map` diff. */
function comparable(tree: Tree): [string, string][] {
  return [...tree.entries()]
    .map(([p, e]) => [p, e.sha256] as [string, string])
    .sort(([a], [b]) => a.localeCompare(b))
}

test.describe("curseforge modpack version change", () => {
  test.afterEach(async ({ authenticatedApp }, testInfo) => {
    await attachCoreLogOnFailure(testInfo, authenticatedApp.harness.runtimePath)
    await authenticatedApp.page
      .locator(byTestId(TEST_IDS.navbarLogo))
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(authenticatedApp.page)
  })

  test("a version change lands where a fresh install of the target would have", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    let subjectName: string | undefined

    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
        // --- The twin: a FRESH install of the target file, snapshotted and
        // then removed before the subject is created (see the header for why
        // they cannot coexist).
        const twinName = await installModpackVersion(
          page,
          MODPACK_CF_QUERY,
          "curseforge",
          MODPACK_CF_FILE
        )
        const twinRoot = rootFor(harness.runtimePath, twinName)
        const twinTree: Tree = await snapshotTree(
          path.join(twinRoot, "instance")
        )
        const twinPackinfo: Packinfo = await readPackinfo(twinRoot)

        expect(
          twinTree.size,
          "the twin install produced an empty instance tree"
        ).toBeGreaterThan(0)

        await page.click(byTestId(TEST_IDS.navbarLogo))
        await deleteInstanceViaUi(page, twinName)

        // --- The subject: installed at the OLD file, then version-changed onto
        // the same target the twin was installed at directly.
        subjectName = await installModpackVersion(
          page,
          MODPACK_CF_QUERY,
          "curseforge",
          MODPACK_CF_FILE_OLD
        )
        const subjectRoot = rootFor(harness.runtimePath, subjectName)

        const beforeConfig = await readInstanceConfig(subjectRoot)
        expect(
          beforeConfig.modpack?.curseforgeProjectId,
          `the subject instance is not pinned to project ${MODPACK_CF_PROJECT} — ` +
            "CurseForge's search ranking for MODPACK_CF_QUERY may have drifted, " +
            "so the wrong pack was opened"
        ).toBe(Number(MODPACK_CF_PROJECT))
        expect(
          beforeConfig.modpack?.curseforgeFileId,
          "the subject instance did not install the pinned OLD file"
        ).toBe(Number(MODPACK_CF_FILE_OLD))

        await changeModpackVersion(page, subjectName, MODPACK_CF_FILE)

        const subjectTree = await snapshotTree(
          path.join(subjectRoot, "instance")
        )

        // THE assertion. A divergence is a finding — see the header. Do not
        // loosen this to toContain or subtract a hardcoded set without
        // characterising why first.
        expect(
          comparable(subjectTree),
          "a CurseForge version change did not leave the instance byte-identical " +
            "to a fresh install of the same target file"
        ).toEqual(comparable(twinTree))

        const afterConfig = await readInstanceConfig(subjectRoot)
        expect(
          afterConfig.modpack?.curseforgeFileId,
          "the version change did not repin the instance to the target file"
        ).toBe(Number(MODPACK_CF_FILE))
        expect(
          afterConfig.modpack?.locked,
          "the version change altered the instance's lock state"
        ).toBe(beforeConfig.modpack?.locked)

        // packinfo now matches the tree — finding 4 (scan_dir dropping files
        // unchanged between versions, `helpers/packinfo.js`) is fixed:
        // `process_modpack`'s snapshot block merges the skip-oracle's hash
        // back in for every skip-optimised path, so the version-changed
        // instance's packinfo is exactly the same set of paths a fresh
        // install of the same target produces — the per-key bytes check this
        // needs is subsumed by the strict tree assertion above, which already
        // proves every path's bytes are correct.
        const subjectPackinfo = await readPackinfo(subjectRoot)
        const missing = [...twinPackinfo.keys()]
          .filter((k) => !subjectPackinfo.has(k))
          .sort()
        const extra = [...subjectPackinfo.keys()]
          .filter((k) => !twinPackinfo.has(k))
          .sort()

        expect(
          extra,
          "the version-changed instance's packinfo records paths a fresh " +
            "install does not — scan_dir gained a source of paths, which is " +
            "new behaviour"
        ).toEqual([])
        expect(
          missing,
          "packinfo lost entries across a version change — the skip-optimised " +
            "merge fix (process_modpack's snapshot block, run/modpack.rs) " +
            "regressed"
        ).toEqual([])
      },
      async (alreadyFailed) => {
        if (subjectName) {
          try {
            await page
              .locator(byTestId(TEST_IDS.navbarLogo))
              .click({ timeout: 5_000 })
              .catch(() => {})
            await deleteInstanceViaUi(page, subjectName)
          } catch (cleanupError) {
            reportCleanupFailure(
              cleanupError,
              alreadyFailed,
              `cleanup for "${subjectName}" also failed:`
            )
          }
        }
      }
    )
  })
})
