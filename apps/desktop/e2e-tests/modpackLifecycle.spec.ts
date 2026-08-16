import { createHash } from "node:crypto"
import fs from "node:fs"
import path from "node:path"
import { expect, test, type TestInfo } from "@playwright/test"
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
import { clickPlayAndAwaitLaunched, STOP_TIMEOUT } from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import {
  classifyPackinfo,
  packinfoDataPath,
  readPackinfo
} from "./helpers/packinfo.js"
import { readInstallAudit } from "./helpers/installAudit.js"
import { diffTrees, snapshotTree } from "./helpers/instanceTree.js"
import {
  countMatchedMarkers,
  findFatalSignature,
  instanceLogsDir,
  LAUNCH_MARKER_QUORUM,
  LAUNCH_MARKERS,
  newestLogFile,
  readLogMessages,
  waitForLogQuiescence
} from "./helpers/gameLog.js"
import {
  changeModpackVersion,
  fetchMrpackIndex,
  installModpackVersion,
  packPaths
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_SLUG,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW,
  MODPACK_MR_V_OLD
} from "./helpers/modpackFixtures.js"
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"

/**
 * Proves that changing a modpack's installed version preserves everything a
 * real game session and a real user put into an instance folder, while still
 * correctly replacing, deleting and creating the pack's own files. Nothing
 * else in this suite exercises a version change against a *dirtied*
 * instance — `modpackInstall.spec.ts` only ever asserts a fresh install.
 *
 * **Sequence.** Install `MODPACK_MR_V_MID` -> launch the real game to a
 * stable main menu and quit it (so the instance folder holds real game
 * output, not just pack files) -> partition every pack file into pristine
 * vs. already-modified -> five hand-made mutations -> upgrade to
 * `MODPACK_MR_V_NEW`, asserted against both pack indexes and the install
 * audit -> downgrade to `MODPACK_MR_V_OLD`, asserted the same way, plus that
 * a user edit survives a *second* version change and that playtime never
 * goes backwards.
 *
 * **Own harness, not the shared fixture.** This leaves a real JVM running
 * mid-test and must be free to kill it without disturbing an app other specs
 * share. The setup/teardown below is copied from `gameLaunch.spec.ts` — that
 * file has no `beforeAll`/`afterAll` hooks to copy; its actual pattern is one
 * `test()` with its own inline `startHarness`/`stopHarness` and `stdout`
 * capture in a try/finally, which is what is reproduced here. Copied rather
 * than imported: importing a value from a `.spec.ts` file re-registers that
 * file's own `test()` calls in this one, and the project has a standing
 * decision not to churn passing specs to avoid that. The log-growth-then-
 * stillness wait itself is not subject to that constraint — it lives in
 * `helpers/gameLog.ts`'s `waitForLogQuiescence`, shared with
 * `gameLaunch.spec.ts`.
 *
 * Stopping the game reuses the *same* `instancePlay` control used to start
 * it (there is no separate `instanceStop` test id) — `Tile.tsx`'s play
 * button doubles as a stop control while the instance is running, which is
 * also how `gameLaunch.spec.ts` does it. `GAME_LAUNCHED`/`GAME_CLOSED` are
 * counted rather than searched for with `.includes()`: a plain modpack
 * install/version-change also ends with a `GAME_CLOSED` transition (Inactive
 * on completion), so an unscoped `stdout.join("").includes("GAME_CLOSED")`
 * check after the real launch would already be satisfied by that earlier,
 * unrelated event and prove nothing about the real game actually stopping.
 *
 * **`options.txt` is not usable as evidence the game wrote anything.** This
 * pack ships `overrides/options.txt` itself (confirmed against its own
 * `.mrpack` — the "9th, non-config override" `modpackFixtures.ts`
 * documents), so the file already exists the moment the pack is installed
 * and can never appear in a post-launch tree diff's `added` set, regardless
 * of whether the game ran at all. The premise check below instead looks for
 * a new file under `instance/logs/` — Minecraft's own internal, Log4j-
 * created log directory, distinct from the launcher's own per-launch
 * capture under the sibling `<root>/logs` (`instanceLogsDir`,
 * `helpers/gameLog.ts`) — since no modpack ships a pre-existing, pre-dated
 * Minecraft log file.
 *
 * **A real launch rewrites pack-owned config files.** Sodium and Iris
 * normalise their own configs on first run, and this pack ships both as
 * overrides. Their entries in packinfo still carry the pack's original md5,
 * so once the launch has rewritten them on disk they legitimately classify as
 * `modified by user` for the *next* version change — correct product
 * behaviour (preserve, don't replace), not a bug. So partitioning happens
 * with `classifyPackinfo` *after* the launch and *before* any mutation, and
 * the edit target is picked from the pristine list at runtime rather than
 * assumed — a config the launch happened to touch is simply not eligible.
 *
 * **Two pack files are deleted, and both stay deleted.** `process_modpack_staging`
 * now reconciles every path exactly once (`apply_plan::plan`, consumed by
 * `execute_plan`/`render_audit` in `run/modpack.rs`), so there is no second,
 * independent pass that can recreate what the first decision already left
 * alone:
 *   - `deleteReturning` — pristine, and the NEW version genuinely restages
 *     it (not merely "the same path exists in both" — see below). Disk has
 *     nothing at that path, so the planner decides `Keep`/`DeletedByUser`
 *     regardless of what the target version ships there — that is the only
 *     decision made for it, recorded once, under "could not be replaced".
 *   - `deleteGone` — a pristine jar only the OLD version ships. Dropped from
 *     the target entirely, and disk has nothing there either, so the
 *     planner reaches the identical `Keep`/`DeletedByUser` decision by the
 *     other route (`decide_dropped`). Both converge on the same behaviour,
 *     which is itself worth proving here: a regression that broke only one
 *     of the two input shapes (dropped-by-target vs. still-shipped-by-target)
 *     would still be caught.
 *
 * `deleteReturning` cannot be picked on path-presence alone, in a way only
 * caught by running it. A *mod* only
 * qualifies if its bytes genuinely differ between MID and NEW:
 * `prepare_modpack_from_mrpack` (`crates/carbon_app/src/managers/minecraft/modrinth.rs:277-289`)
 * skips re-downloading a file whose new-version sha512 already matches what
 * the *old* packinfo recorded, so a same-content mod across the bump never
 * gets a fresh copy placed in the staging directory — nothing for the
 * second loop to recreate a deleted one from. Confirmed live two
 * ways: picking on path-presence alone left the deleted jar gone instead of
 * reinstated, and a direct diff of this pack's own MID/NEW `.mrpack`
 * indexes showed *zero* same-path mods with different hashes — its version
 * bump is pure add(5)/remove(4) among mods, unsurprising since a mod update
 * ordinarily changes its own versioned filename rather than keeping it and
 * changing the bytes underneath. A pack *config override* has no such
 * optimization and needs no hash check at all: override extraction
 * (`modrinth.rs:321-379`) unconditionally re-extracts every file under
 * `overrides/` on every version change regardless of content. So this
 * prefers a genuinely-replaced mod when one exists (a future re-pin may have
 * one) and falls back to a pristine config override otherwise, which this
 * pack always has.
 *
 * **`packinfo.json` retains unchanged files across a version change.**
 * `packinfo::scan_dir` (`crates/carbon_app/src/managers/instance/modpack/packinfo/scan.rs`)
 * still rebuilds packinfo.json by hashing whatever physically landed in the
 * staging directory, and the skip-if-unchanged download optimization above
 * still means a file whose bytes are identical between the old and new
 * version is never staged. `process_modpack`'s snapshot block now merges the
 * skip-oracle's hash for every such path back into the freshly scanned
 * packinfo before writing it (the loop over `skipped_mods` in
 * `run/modpack.rs`, just after the `scan_dir` call): a "skipped" path is, by
 * construction of the skip condition itself, one where the oracle's recorded
 * hash already equals the target version's declared hash, so merging that
 * hash back in is not a guess — it *is* the target's hash. The assertions
 * below now expect packinfo to be complete rather than characterising a
 * predicted gap.
 *
 * **A pack file surviving a downgrade forever, invisibly, was the packinfo
 * gap's most serious consequence — and is fixed by the same merge.**
 * `process_modpack_staging`'s deletion now comes from `apply_plan::plan`'s
 * `universe` (`old.keys() ∪ target.keys()`), not a walk over packinfo's own
 * keys in isolation, but `old` is still exactly `packinfo.json`'s content —
 * so a path the upgrade would have dropped from packinfo, pre-fix, was a
 * path the downgrade's deletion could never have visited, no matter what the
 * newly-installed version's own manifest said about it. With packinfo now
 * complete after every version change (previous finding), that no longer
 * happens: every path the downgrade needs to delete is still on record to
 * delete. The downgrade leg's physical-completeness check (near its end,
 * right before the playtime assertions) still checks the file system
 * instead of packinfo, deliberately — it is the one check in this file that
 * does not derive from packinfo at all, so it would catch a *different*
 * mechanism producing the same kind of leak, not just a regression of this
 * specific one.
 *
 * **The Replaced loops verify override content by hash, not just presence.**
 * `PackIndex.overrides` carries hashes, not paths alone: `nextByPath`/
 * `oldByPath` in the Replaced loops are built from `.files`, which never
 * contains an override, so a paths-only `overrides` would leave `declared`
 * `undefined` and let every override pass with no check at all. This was not
 * incidental: mods get
 * real integrity verification at download time regardless, via
 * `.with_checksum(...)` (`modrinth.rs:299`), but override extraction
 * (`modrinth.rs:321-379`) has no checksum step anywhere in the pipeline —
 * these two loops are the *only* place override content is ever verified
 * against anything, and the bug meant they structurally never fired.
 * `parseMrpackIndex` (`helpers/modpacks.ts`) now also returns
 * `overrideFiles`, pairing each override path with a sha256 of its raw
 * archive bytes (there is no declared hash to compare against — a
 * `modrinth.index.json` carries one for `files[]` only — so the archive
 * itself, already held in memory to build `overrides`, is the only
 * available source of truth), and both Replaced loops check that hash
 * against the on-disk file's own sha256 instead of falling through a silent
 * `continue`. Concretely active for this pack: `/options.txt`
 * (`deleteReturning`) is excluded from the *upgrade* leg's Replaced loop by
 * name, but nothing excludes it from the *downgrade* leg's, so that is the
 * leg where this fix is actually exercised on every green run, not merely
 * in theory.
 *
 * **Every audit section shares one path format; `created` is normalised to
 * this file's own spelling at the point it is read.** `render_audit`
 * (`run/modpack.rs`) writes each `PlanEntry`'s own packinfo-style,
 * leading-slash `path` into every section, `created` included. Every other
 * comparison in this file — `afterUpgrade`/`afterDowngrade` (`snapshotTree`),
 * `packinfoPathsAfterUpgrade`, `packPaths()`, `userMod`/`userSave`/
 * `gameWritten` — works in the slash-less, instance-relative spelling, so
 * `auditCreated`/`auditCreatedDown` strip the leading `/` (alongside the
 * dead `instance/` prefix, kept as a no-op) when they are built. Every
 * `auditCreated.has(...)`/`auditCreatedDown.has(...)` call below, positive
 * or negative, depends on that normalisation to compare like-for-like.
 *
 * **`seconds_played` accrues during the launch** and is asserted non-zero and
 * non-decreasing across both version changes — "unchanged" would be the
 * wrong claim, since the instance really was played.
 *
 * **Sabotage results**
 * (`crates/carbon_app/src/managers/instance/run/modpack.rs`, each confirmed
 * red, then reverted and confirmed byte-identical to HEAD before rebuilding
 * green):
 *   1. Forcing the md5 comparison to always match (`if false` in place of
 *      `original_md5 != oldfilehash.md5`, line 776) went red on
 *      `"launch-modified file changed: config/NoChatReports/NCR-Client.json"`
 *      — the first of the seven launch-modified configs hit in iteration
 *      order, not this file's own hand-edited `editTarget`, because the
 *      sabotage disables modified-by-user detection for *every* file, not
 *      just one. Still a full confirmation: the preservation assertion this
 *      sabotage targets is the same assertion class either way.
 *   2. Dropping the staging walk's `!original_file.exists()` guard (line
 *      815) in favour of a packinfo-membership check went red on exactly
 *      `"a user-deleted pack file was not reinstated by the new version"`
 *      and nothing else, as predicted.
 *   3. Returning before the audit file is written (before line 833) went red
 *      on `"could not read packinfo.json at .../packinfo.json"` (ENOENT) —
 *      earlier than the predicted "wrote no install audit" message, because
 *      `process_modpack_staging` is the same code path for the *install*
 *      as for a version change, and this test reads packinfo.json
 *      immediately after install (`packinfoAtInstall`, see above). Skipping
 *      the audit also skips the `tmp-packinfo.json` -> `packinfo.json`
 *      rename a few lines later, so even the first install never produces a
 *      `packinfo.json` at all. Same root cause as the predicted message,
 *      surfacing at the earliest read of the file this test happens to make.
 */

/**
 * Attaches the game's own per-launch log to the Playwright report on a
 * failing run — the game-log analogue of `attachCoreLogOnFailure`, which
 * only ever covers the core's own tracing log, never the game's. Added
 * because `stopHarness` deletes the whole runtime path, including
 * `instances/<shortpath>/logs/`, before any failure diagnosis gets a chance
 * to read it otherwise — a real gap hit live investigating why the launch
 * leg's quiescence reading turned out to be premature on one run, with no
 * way afterward to confirm how far the client actually got.
 */
async function attachGameLogOnFailure(
  testInfo: TestInfo,
  runtimePath: string,
  shortpathToAttach: string | undefined
): Promise<void> {
  if (testInfo.status === testInfo.expectedStatus) return
  if (!shortpathToAttach) return

  const logFile = newestLogFile(instanceLogsDir(runtimePath, shortpathToAttach))
  if (!logFile) return

  await testInfo
    .attach("game-log", { path: logFile, contentType: "text/plain" })
    .catch(() => {})
}

test.describe("modpack lifecycle", () => {
  // eslint-disable-next-line no-empty-pattern
  test("preserves user and game data across a modpack upgrade and downgrade", async ({}, testInfo) => {
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
    /** Set once the instance is installed. `finally` needs it (independent of
     *  `name`) to locate the game log for `attachGameLogOnFailure` even if a
     *  later step fails. */
    let shortpath: string | undefined
    /** `GAME_CLOSED` count at the moment before Play is clicked. Declared out
     *  here because `finally` needs it to tell "still running" from
     *  "already stopped", same as `gameLaunch.spec.ts`. */
    let closedBeforeLaunch = 0

    // Counted rather than searched for — see this file's header for why a
    // plain `.includes()` on GAME_CLOSED would be satisfied instantly by the
    // install's own completion and prove nothing about the real launch.
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

        // Fetched up front so every expectation below is derived from the
        // packs' own declared content rather than hardcoded.
        const mid = await fetchMrpackIndex(MODPACK_MR_V_MID)
        const next = await fetchMrpackIndex(MODPACK_MR_V_NEW)
        const old = await fetchMrpackIndex(MODPACK_MR_V_OLD)

        // --- install leg ------------------------------------------------
        name = await installModpackVersion(
          page,
          MODPACK_MR_QUERY,
          "modrinth",
          MODPACK_MR_V_MID
        )
        shortpath = readInstanceByName(harness.runtimePath, name).shortpath
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const data = path.join(root, "instance")

        const afterInstall = await snapshotTree(data)
        for (const file of mid.files) {
          expect(
            afterInstall.get(file.path)?.size,
            `size of ${file.path}`
          ).toBe(file.size)
        }

        // Captured now because the upgrade rewrites `packinfo.json` in place:
        // the pack's *original* md5 for the file about to be edited is
        // unrecoverable from disk afterwards, and the audit's `original md5:`
        // line is asserted against it.
        const packinfoAtInstall = await readPackinfo(root)

        // --- launch leg ---------------------------------------------------
        const tile = page.locator(byInstanceName(name))
        closedBeforeLaunch = closedCount()
        await clickPlayAndAwaitLaunched(page, name, { stdout })

        const logsDir = instanceLogsDir(harness.runtimePath, shortpath)
        await waitForLogQuiescence(
          logsDir,
          () => closedCount() === closedBeforeLaunch
        )

        // Corroborate quiescence with textual evidence before trusting it —
        // same quorum `gameLaunch.spec.ts` uses, and for the same reason (see
        // `LAUNCH_MARKERS`'s doc comment): "the log stopped growing" alone
        // cannot tell a genuinely idle client apart from one paused mid-load,
        // and this pack's lighter Fabric instance logs sparser than the Forge
        // instance `gameLaunch.spec.ts` was tuned against, leaving more room
        // for a natural gap to be mistaken for quiescence.
        const launchMessages = readLogMessages(newestLogFile(logsDir))
        const fatal = findFatalSignature(launchMessages)
        expect(
          fatal,
          `the game log contains a JVM-fatal signature: ${fatal}`
        ).toBeUndefined()
        const matchedMarkers = countMatchedMarkers(
          launchMessages,
          LAUNCH_MARKERS
        )
        expect(
          matchedMarkers,
          `only ${matchedMarkers} of ${LAUNCH_MARKERS.length} startup markers ` +
            `appeared in the game log (need ${LAUNCH_MARKER_QUORUM}) — the ` +
            "client went quiet without enough corroborating evidence that it " +
            "actually reached the main menu"
        ).toBeGreaterThanOrEqual(LAUNCH_MARKER_QUORUM)

        await tile.locator(byTestId(TEST_IDS.instancePlay)).click()
        await expect
          .poll(() => closedCount(), {
            timeout: STOP_TIMEOUT,
            message:
              "the game never reported a new GAME_CLOSED after its stop " +
              "control was clicked"
          })
          .toBeGreaterThan(closedBeforeLaunch)

        const playtimeAfterLaunch = (await readInstanceConfig(root))
          .secondsPlayed

        const afterLaunch = await snapshotTree(data)
        const launchDiff = diffTrees(afterInstall, afterLaunch)
        // NOT `options.txt`: confirmed live against this pack's own
        // `.mrpack` that it ships `overrides/options.txt` — one of the "9
        // overrides (8 config)" `modpackFixtures.ts` documents — so the file
        // already exists at install time and can never appear in `added`
        // here, regardless of whether the game runs at all. Minecraft's own
        // internal log directory (`instance/logs/`, Log4j-created — distinct
        // from the launcher's own per-launch capture under the sibling
        // `<root>/logs` `instanceLogsDir` points at) is the reliable evidence
        // instead: no modpack ships a pre-existing, pre-dated Minecraft log
        // file, and the client writes into this directory from nearly its
        // first line of output.
        const gameOwnLogFiles = launchDiff.added.filter((p) =>
          p.startsWith("logs/")
        )
        expect(
          gameOwnLogFiles,
          "the game wrote nothing into the instance folder — this test's " +
            "premise is that it does, so every preservation assertion below " +
            "would be vacuous"
        ).not.toEqual([])

        // Files the game created or rewrote. Everything below must leave
        // these byte-identical.
        const gameWritten = [...launchDiff.added, ...launchDiff.changed]

        // --- partition + mutation leg --------------------------------------
        const beforeMutation = await classifyPackinfo(root)
        // Reported unconditionally, not just on failure: which pack configs a
        // real launch normalises is itself part of what this test measures
        // (Sodium/Iris rewrite their own overrides on first run — see the
        // header), and it is worth having in the run's own output rather than
        // only reconstructible from a failure trace.
        console.log(
          `[PARTITION] pristine (${beforeMutation.pristine.length}): ${beforeMutation.pristine.join(", ")}`
        )
        console.log(
          `[PARTITION] modified (${beforeMutation.modified.length}): ${beforeMutation.modified.join(", ")}`
        )
        expect(
          beforeMutation.missing,
          "a pack file went missing during the launch"
        ).toEqual([])

        const midPaths = new Set(packPaths(mid))
        const nextPaths = new Set(packPaths(next))
        // Path -> declared sha512, used below to tell a genuinely *replaced*
        // file (content differs between the two versions) apart from one that
        // merely has the same path in both — see `deleteReturningCandidate`'s
        // comment for why that distinction is load-bearing.
        const midShaByPath = new Map(mid.files.map((f) => [f.path, f.sha512]))
        const nextShaByPath = new Map(next.files.map((f) => [f.path, f.sha512]))

        // 1. Overwrite a still-pristine pack config.
        const editTarget = beforeMutation.pristine.find(
          (k) => k.startsWith("/config/") && nextPaths.has(k.slice(1))
        )
        expect(
          editTarget,
          `no pristine pack config survived the launch to edit in ` +
            `"${MODPACK_MR_SLUG}" — the pack's mods rewrote all of them, so ` +
            "pick a different pack or a different file class"
        ).toBeDefined()
        const editedBody = `e2e-edited-${Date.now()}\n`
        await fs.promises.writeFile(
          packinfoDataPath(root, editTarget!),
          editedBody
        )

        // 2. Delete a pack file the NEW version still ships. The planner
        //    decides `Keep`/`DeletedByUser` for this path and that decision is
        //    final — the deletion is respected, not silently reinstated (see
        //    the module doc comment's "two pack files are deleted" paragraph).
        //    The path is still staged (see the mod/override preference below)
        //    and so still ends up recorded in the promoted packinfo.json, even
        //    though nothing lands on disk at it — which is exactly why
        //    `classifyPackinfo` reports it `missing` after the upgrade, and
        //    `beforeMutation2.missing` below asserts that explicitly rather
        //    than requiring the list empty. A mod (`/mods/`) only qualifies as
        //    a genuinely fresh-staged example if its bytes actually differ
        //    between MID and NEW — `prepare_modpack_from_mrpack`
        //    (`modrinth.rs:277-289`) skips re-downloading a file whose
        //    new-version sha512 already matches the *old* packinfo's recorded
        //    hash, so a same-content mod across the bump never gets a fresh
        //    copy placed in the staging directory to recreate a deleted one
        //    from. This pack's own MID->NEW delta (confirmed live) is
        //    pure add/remove among mods — zero same-path replacements, since a
        //    mod update ordinarily changes its own versioned filename — so a
        //    qualifying mod is preferred when one exists (future re-pins may
        //    have one) but this falls back to any *other* pack override
        //    (`/config/*` or the root `/options.txt`), which has no such
        //    optimization: override extraction (`modrinth.rs:321-379`)
        //    unconditionally re-extracts every file under `overrides/` on
        //    every version change regardless of content, so any pristine,
        //    still-shipped override is always a valid choice. Not narrowed to
        //    `/config/` alone: measured live, this launch normalised
        //    7 of this pack's 8 config overrides (every one except
        //    `lithium.properties`, which is what `editTarget` above picks up),
        //    leaving nothing under `/config/` free for this to pick — the root
        //    `/options.txt` override is what's actually available.
        const replacedModCandidate = beforeMutation.pristine.find((k) => {
          if (!k.startsWith("/mods/")) return false
          const p = k.slice(1)
          if (!nextPaths.has(p)) return false
          const midSha = midShaByPath.get(p)
          const nextSha = nextShaByPath.get(p)
          return (
            midSha !== undefined && nextSha !== undefined && midSha !== nextSha
          )
        })
        const restagedOverrideCandidate = beforeMutation.pristine.find(
          (k) =>
            !k.startsWith("/mods/") &&
            nextPaths.has(k.slice(1)) &&
            k !== editTarget
        )
        const deleteReturningCandidate =
          replacedModCandidate ?? restagedOverrideCandidate
        expect(
          deleteReturningCandidate,
          `no pristine mod with different bytes between MID and NEW, and no ` +
            `spare pristine override, survived the launch in ` +
            `"${MODPACK_MR_SLUG}" — the version deltas must have changed; ` +
            "re-measure and re-pin modpackFixtures.ts"
        ).toBeDefined()
        const deleteReturning = deleteReturningCandidate!
        await fs.promises.rm(packinfoDataPath(root, deleteReturning))

        // 3. Delete a pack jar only the OLD version ships. Expected to stay
        //    gone.
        const deleteGoneCandidate = beforeMutation.pristine.find(
          (k) =>
            k.startsWith("/mods/") &&
            midPaths.has(k.slice(1)) &&
            !nextPaths.has(k.slice(1)) &&
            k !== deleteReturning
        )
        expect(
          deleteGoneCandidate,
          `no pristine mod in "${MODPACK_MR_SLUG}" is both present in MID and ` +
            "absent from NEW — the version deltas must have changed; " +
            "re-measure and re-pin modpackFixtures.ts"
        ).toBeDefined()
        const deleteGone = deleteGoneCandidate!
        await fs.promises.rm(packinfoDataPath(root, deleteGone))

        // 4. A user's own mod, in no pack version.
        const userMod = "mods/zz-e2e-user-mod.jar"
        await fs.promises.writeFile(path.join(data, userMod), "not-a-real-jar")

        // 5. A user's own world file.
        const userSave = "saves/e2e-world/level.dat"
        await fs.promises.mkdir(path.dirname(path.join(data, userSave)), {
          recursive: true
        })
        await fs.promises.writeFile(path.join(data, userSave), "e2e-world")

        const beforeUpgrade = await snapshotTree(data)

        // --- upgrade leg ----------------------------------------------------
        await changeModpackVersion(page, name, MODPACK_MR_V_NEW)

        const afterUpgrade = await snapshotTree(data)
        const audit = await readInstallAudit(root)
        expect(
          audit,
          "the version change wrote no install audit"
        ).not.toBeNull()

        // Leading-slash stripped to match this file's slash-less comparisons —
        // see the module doc comment's "created is normalised" paragraph.
        const auditCreated = new Set(
          audit!.created.map((p) =>
            p.replace(/^instance\//, "").replace(/^\//, "")
          )
        )
        const skipReason = new Map(audit!.skipped.map((s) => [s.file, s]))

        // Created: in the new pack, not in the old one. Skips anything the
        // game itself already wrote during the launch (`gameWritten`): a mod
        // that auto-generates its own config on first run can coincide with a
        // path the pack also ships as an override, and when it does, the file
        // already exists by the time the staging walk gets to it — correctly
        // left alone (its bytes are the game's, not the pack's) and correctly
        // not claimed as a pipeline-created file in the audit either.
        for (const p of packPaths(next)) {
          if (midPaths.has(p)) continue
          if (gameWritten.includes(p)) continue
          expect(afterUpgrade.has(p), `new pack file not created: ${p}`).toBe(
            true
          )
          expect(
            auditCreated.has(p),
            `audit did not record creating ${p}`
          ).toBe(true)
        }

        // Deleted: pristine, in the old pack, not in the new one — except the
        // two deleted by hand and anything under /saves.
        for (const key of beforeMutation.pristine) {
          const p = key.slice(1)
          if (nextPaths.has(p)) continue
          if (key === deleteGone || key === deleteReturning) continue
          expect(afterUpgrade.has(p), `stale pack file not deleted: ${p}`).toBe(
            false
          )
          expect(
            audit!.deleted,
            `audit did not record deleting ${p}`
          ).toContain(key)
        }

        // Replaced: pristine, in both, and the two versions disagree on the
        // bytes. A declared file (`/mods/*`) is checked by size — real content
        // integrity for these already happened at download time, via
        // `.with_checksum(...)` (`modrinth.rs:299`); this is a secondary
        // check. An override has no such download-time check anywhere in the
        // pipeline (`modrinth.rs:321-379` extracts it unconditionally, with no
        // hash involved), so it is checked by sha256 instead of merely
        // existing — see `nextOverrideSha256ByPath` and `PackOverrideFile`'s
        // doc comment for why this is the *only* place override content is
        // ever verified against anything.
        const nextByPath = new Map(next.files.map((f) => [f.path, f]))
        const nextOverrideSha256ByPath = new Map(
          next.overrideFiles.map((f) => [f.path, f.sha256])
        )
        for (const key of beforeMutation.pristine) {
          const p = key.slice(1)
          if (
            key === editTarget ||
            key === deleteGone ||
            key === deleteReturning
          )
            continue
          if (!nextPaths.has(p) || !midPaths.has(p)) continue
          const declaredFile = nextByPath.get(p)
          if (declaredFile) {
            expect(afterUpgrade.get(p)?.size, `size after replace: ${p}`).toBe(
              declaredFile.size
            )
            continue
          }
          // Not a declared file, but `p` came from `nextPaths` (files UNION
          // overrides), so it must be an override — a `.toBeDefined()` rather
          // than falling through silently, so a real parser inconsistency
          // fails loudly here instead of vanishing the way the pre-fix
          // `if (!declared) continue` used to vanish every override
          // unconditionally (see the module doc comment).
          const declaredOverrideSha256 = nextOverrideSha256ByPath.get(p)
          expect(
            declaredOverrideSha256,
            `${p} is in packPaths(next) but neither next.files nor ` +
              "next.overrideFiles declares it"
          ).toBeDefined()
          expect(
            afterUpgrade.get(p)?.sha256,
            `override content after replace: ${p}`
          ).toBe(declaredOverrideSha256)
        }

        // Already-modified-by-the-launch files are preserved, not replaced.
        for (const key of beforeMutation.modified) {
          const p = key.slice(1)
          expect(
            afterUpgrade.get(p)?.sha256,
            `launch-modified file changed: ${p}`
          ).toBe(beforeUpgrade.get(p)?.sha256)
          expect(skipReason.get(key)?.reason, `audit reason for ${key}`).toBe(
            "modified-by-user"
          )
        }

        // Our edited config: byte-identical, reported with both md5s.
        const editedPath = editTarget!.slice(1)
        expect(
          await fs.promises.readFile(path.join(data, editedPath), "utf8"),
          "the upgrade overwrote a user-edited config"
        ).toBe(editedBody)
        const editSkip = skipReason.get(editTarget!)
        expect(
          editSkip?.reason,
          `audit reason for the edited config ${editTarget}`
        ).toBe("modified-by-user")
        expect(
          editSkip?.currentMd5,
          "audit current md5 for the edited config"
        ).toBe(createHash("md5").update(editedBody).digest("hex"))
        // `packinfoAtInstall` is captured right after the install: the upgrade
        // rewrites `packinfo.json`, so the pack's original md5 for this file is
        // unrecoverable from disk afterwards.
        expect(
          editSkip?.originalMd5,
          "audit original md5 for the edited config"
        ).toBe(packinfoAtInstall.get(editTarget!)?.md5)

        // The jar deleted that the new version ships: stays deleted, recorded
        // once, and nowhere else. The planner makes exactly one decision per
        // path, so there is no independent second pass left to reinstate it.
        expect(
          afterUpgrade.has(deleteReturning.slice(1)),
          "a user-deleted pack file was reinstated by the new version"
        ).toBe(false)
        expect(
          skipReason.get(deleteReturning)?.reason,
          `audit reason for ${deleteReturning}`
        ).toBe("deleted-by-user")
        expect(
          auditCreated.has(deleteReturning.slice(1)),
          `audit wrongly also recorded creating ${deleteReturning}`
        ).toBe(false)

        // The jar deleted that the new version does not ship: still gone, and
        // NOT counted as a deletion the pass performed.
        expect(
          afterUpgrade.has(deleteGone.slice(1)),
          `${deleteGone} came back`
        ).toBe(false)
        expect(
          skipReason.get(deleteGone)?.reason,
          `audit reason for ${deleteGone}`
        ).toBe("deleted-by-user")
        expect(
          audit!.deleted,
          `audit wrongly recorded ${deleteGone} as a deletion the pass performed`
        ).not.toContain(deleteGone)

        // Nothing the user or the game put there was touched.
        for (const p of [userMod, userSave, ...gameWritten]) {
          expect(
            afterUpgrade.get(p)?.sha256,
            `untouched file changed: ${p}`
          ).toBe(beforeUpgrade.get(p)?.sha256)
          expect(auditCreated.has(p), `audit claims it created ${p}`).toBe(
            false
          )
          expect(audit!.deleted, `audit claims it deleted ${p}`).not.toContain(
            `/${p}`
          )
          expect(
            audit!.replaced,
            `audit claims it replaced ${p}`
          ).not.toContain(`/${p}`)
        }

        // packinfo now describes the new version completely. The merge fix
        // (`process_modpack`'s snapshot block, `run/modpack.rs`: the loop over
        // `skipped_mods` right after the `scan_dir` call) folds the
        // skip-oracle's hash back in for every skip-optimised path, so a pack
        // file whose bytes are unchanged between MID and NEW is no longer
        // silently dropped from the rebuilt packinfo.json just because
        // `prepare_modpack_from_mrpack` (`modrinth.rs:277-289`) never
        // re-staged it.
        const packinfoAfterUpgrade = await readPackinfo(root)
        const packinfoPathsAfterUpgrade = new Set(
          [...packinfoAfterUpgrade.keys()].map((k) => k.slice(1))
        )
        const missingFromPackinfoAfterUpgrade = packPaths(next)
          .filter((p) => !packinfoPathsAfterUpgrade.has(p))
          .sort()
        expect(
          missingFromPackinfoAfterUpgrade,
          "packinfo.json is missing pack files after the upgrade"
        ).toEqual([])
        // The narrowest, most direct proof the merge fix is doing its job: a
        // `files[]` (mod) entry is exactly the population that CAN be
        // skip-optimised; an override cannot (`modrinth.rs:321-379`
        // re-extracts every override unconditionally, regardless of content),
        // so checking overrides here would prove nothing about this
        // specific fix. Independent of the broader, packPaths-based check
        // above.
        expect(
          next.files
            .map((f) => f.path)
            .every((p) => packinfoPathsAfterUpgrade.has(p)),
          "packinfo.json is missing a files[] path after the upgrade — the " +
            "skip-optimised merge fix regressed"
        ).toBe(true)
        // Every path packinfo *does* record must still belong to the new pack
        // — no stale entries left over from a path the new version dropped.
        const nextPathSet = new Set(packPaths(next))
        expect(
          [...packinfoPathsAfterUpgrade].filter((p) => !nextPathSet.has(p)),
          "packinfo.json after the upgrade records a path the new version " +
            "does not declare"
        ).toEqual([])

        // --- downgrade leg ---------------------------------------------------
        const beforeMutation2 = await classifyPackinfo(root)
        // `deleteReturning`, and only it, is expected here: the upgrade's
        // planner decided `Keep`/`DeletedByUser` for it (the deleted-stays-
        // deleted flip — see point 2 above and the module doc comment), so
        // nothing landed on disk at that path, but it is still staged and so
        // still recorded in the promoted packinfo.json. `classifyPackinfo`
        // has no third bucket for "packinfo tracks it, disk has nothing" other
        // than `missing`, so that is exactly where it lands. Any other member
        // here would mean a *different* pack file went missing.
        expect(
          beforeMutation2.missing,
          "a pack file went missing between the upgrade and the downgrade, " +
            "beyond the already-accounted-for deleteReturning"
        ).toEqual([deleteReturning])

        const oldPaths = new Set(packPaths(old))
        const beforeDowngrade = await snapshotTree(data)

        await changeModpackVersion(page, name, MODPACK_MR_V_OLD)

        const afterDowngrade = await snapshotTree(data)
        const auditDown = await readInstallAudit(root)
        expect(
          auditDown,
          "the version change wrote no install audit"
        ).not.toBeNull()

        // Same leading-slash strip as `auditCreated` above, same reason — see
        // the module doc comment's "created is normalised" paragraph.
        const auditCreatedDown = new Set(
          auditDown!.created.map((p) =>
            p.replace(/^instance\//, "").replace(/^\//, "")
          )
        )
        const skipReasonDown = new Map(
          auditDown!.skipped.map((s) => [s.file, s])
        )

        // Created: in OLD, not in the currently-installed NEW. Same
        // game-written exclusion as the upgrade leg's Created loop above —
        // confirmed live: `config/entityculling.json`, which only
        // OLD ships as an override, was already on disk from the launch
        // (the `entityculling` mod auto-generates it on first run) by the
        // time the downgrade's staging walk reached it, so the pipeline
        // correctly left it alone rather than claiming to have created it.
        for (const p of packPaths(old)) {
          if (nextPaths.has(p)) continue
          if (gameWritten.includes(p)) continue
          expect(afterDowngrade.has(p), `old pack file not created: ${p}`).toBe(
            true
          )
          expect(
            auditCreatedDown.has(p),
            `audit did not record creating ${p}`
          ).toBe(true)
        }

        // Deleted: pristine (as of the post-upgrade partition), in NEW, not in
        // OLD.
        for (const key of beforeMutation2.pristine) {
          const p = key.slice(1)
          if (oldPaths.has(p)) continue
          expect(
            afterDowngrade.has(p),
            `stale pack file not deleted: ${p}`
          ).toBe(false)
          expect(
            auditDown!.deleted,
            `audit did not record deleting ${p}`
          ).toContain(key)
        }

        // Replaced: pristine, in both, and the two versions disagree on the
        // bytes. Same declared-file-vs-override split as the upgrade leg's
        // Replaced loop above. `deleteReturning` (`/options.txt` for this
        // pack) needs no explicit exclusion here the way the upgrade leg
        // excludes it: it no longer appears in `beforeMutation2.pristine` at
        // all — it is `missing` there instead, per the deleted-stays-deleted
        // fix (point 2 above and the module doc comment) — so this loop
        // naturally skips it without being told to. The override-hash check
        // below is still exercised on every green run regardless, by this
        // pack's other pristine config overrides — see the module doc
        // comment's override-verification paragraph.
        const oldByPath = new Map(old.files.map((f) => [f.path, f]))
        const oldOverrideSha256ByPath = new Map(
          old.overrideFiles.map((f) => [f.path, f.sha256])
        )
        for (const key of beforeMutation2.pristine) {
          const p = key.slice(1)
          if (!oldPaths.has(p) || !nextPaths.has(p)) continue
          const declaredFile = oldByPath.get(p)
          if (declaredFile) {
            expect(
              afterDowngrade.get(p)?.size,
              `size after replace: ${p}`
            ).toBe(declaredFile.size)
            continue
          }
          const declaredOverrideSha256 = oldOverrideSha256ByPath.get(p)
          expect(
            declaredOverrideSha256,
            `${p} is in packPaths(old) but neither old.files nor ` +
              "old.overrideFiles declares it"
          ).toBeDefined()
          expect(
            afterDowngrade.get(p)?.sha256,
            `override content after replace: ${p}`
          ).toBe(declaredOverrideSha256)
        }

        // Already-modified files (including our edited config) are preserved,
        // not replaced.
        for (const key of beforeMutation2.modified) {
          const p = key.slice(1)
          expect(
            afterDowngrade.get(p)?.sha256,
            `modified file changed: ${p}`
          ).toBe(beforeDowngrade.get(p)?.sha256)
          expect(
            skipReasonDown.get(key)?.reason,
            `audit reason for ${key}`
          ).toBe("modified-by-user")
        }

        // The edited config survives a SECOND version change.
        expect(
          await fs.promises.readFile(path.join(data, editedPath), "utf8"),
          "the downgrade overwrote a user-edited config that survived the upgrade"
        ).toBe(editedBody)

        // Nothing the user or the game put there was touched, across either
        // version change.
        for (const p of [userMod, userSave, ...gameWritten]) {
          expect(
            afterDowngrade.get(p)?.sha256,
            `untouched file changed: ${p}`
          ).toBe(beforeDowngrade.get(p)?.sha256)
          expect(auditCreatedDown.has(p), `audit claims it created ${p}`).toBe(
            false
          )
          expect(
            auditDown!.deleted,
            `audit claims it deleted ${p}`
          ).not.toContain(`/${p}`)
          expect(
            auditDown!.replaced,
            `audit claims it replaced ${p}`
          ).not.toContain(`/${p}`)
        }

        // packinfo now describes the old version completely — the same merge
        // fix as the upgrade leg, applied again on the way back down.
        const packinfoAfterDowngrade = await readPackinfo(root)
        const packinfoPathsAfterDowngrade = new Set(
          [...packinfoAfterDowngrade.keys()].map((k) => k.slice(1))
        )
        const missingFromPackinfoAfterDowngrade = packPaths(old)
          .filter((p) => !packinfoPathsAfterDowngrade.has(p))
          .sort()
        expect(
          missingFromPackinfoAfterDowngrade,
          "packinfo.json is missing pack files after the downgrade"
        ).toEqual([])
        expect(
          old.files
            .map((f) => f.path)
            .every((p) => packinfoPathsAfterDowngrade.has(p)),
          "packinfo.json is missing a files[] path after the downgrade — the " +
            "skip-optimised merge fix regressed"
        ).toBe(true)
        const oldPathSet = new Set(packPaths(old))
        expect(
          [...packinfoPathsAfterDowngrade].filter((p) => !oldPathSet.has(p)),
          "packinfo.json after the downgrade records a path the old version " +
            "does not declare"
        ).toEqual([])

        // Every assertion above this point is derived from packinfo.json. This
        // checks physical reality instead — what is actually still sitting on
        // disk, regardless of what packinfo says — as independent
        // corroboration that the fix made the record correct rather than just
        // making it *look* correct while a different mechanism still leaked a
        // stale file onto disk.
        //
        // This matters because `process_modpack_staging`'s deletion pass is
        // driven *exclusively* by `apply_plan::plan`'s `universe`
        // (`old.keys() ∪ target.keys()`), built fresh each time from
        // packinfo — so any path silently missing from packinfo (whichever
        // version's) is a path the downgrade's deletion could never visit,
        // no matter what OLD's own manifest says about it. The merge logic
        // keeps packinfo complete precisely so no such leak path exists.
        //
        // So the expectation here is that nothing is left unexplained: every
        // path physically present under a directory this pack has ever owned
        // (the union of `packPaths` across MID, NEW and OLD — `mods/`,
        // `config/`, or the pack root for this pack) must be accounted for by
        // one of (a) a path the currently-installed OLD version actually
        // declares, (b) one of this test's own explicit mutations (`userMod`,
        // `userSave`, the hand-edited config), or (c) something the real game
        // session wrote (`gameWritten` — logs, the seven launch-normalised
        // configs, etc; the game never runs again after the single launch leg
        // near the top of this test, so that set is still exhaustive here).
        const packOwnedPrefixes = new Set(
          [...packPaths(mid), ...packPaths(next), ...packPaths(old)].map(
            (p) => p.split("/")[0]
          )
        )
        const explainedByMutation = new Set([userMod, userSave, editedPath])
        const explainedByGame = new Set(gameWritten)
        const unexplainedOnDisk = [...afterDowngrade.keys()]
          .filter((p) => packOwnedPrefixes.has(p.split("/")[0]))
          .filter(
            (p) =>
              !oldPathSet.has(p) &&
              !explainedByMutation.has(p) &&
              !explainedByGame.has(p)
          )
          .sort()

        expect(
          unexplainedOnDisk,
          "physically present pack-owned files the downgrade cannot account " +
            "for — the stale-survivor leak (fixed by the packinfo merge and " +
            "the planner-driven, universe-wide deletion pass) has returned"
        ).toEqual([])

        // Playtime accrued during the launch and never went backwards across
        // either version change.
        const finalConfig = await readInstanceConfig(root)
        expect(
          finalConfig.secondsPlayed,
          "no playtime was ever recorded for the launch"
        ).toBeGreaterThan(0)
        expect(
          finalConfig.secondsPlayed,
          "playtime went backwards across a version change"
        ).toBeGreaterThanOrEqual(playtimeAfterLaunch)
        expect(
          finalConfig.name,
          "instance name changed across a version change"
        ).toBe(name)
      },
      async (alreadyFailed) => {
        if (current) {
          try {
            // Only stop what is still running, and only if a name was ever
            // resolved — a body that failed before the install finished has
            // nothing to click. Mirrors gameLaunch.spec.ts's own guard against
            // launching a fresh game via the play/stop toggle when there is
            // nothing left to kill.
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
            // Only re-throw over a body that itself succeeded, so cleanup
            // failure never buries the real failure — same precedent as
            // gameLaunch.spec.ts and modpackInstall.spec.ts.
            reportCleanupFailure(
              cleanupError,
              alreadyFailed,
              "cleanup: stopping the game also failed:"
            )
          }
          await attachCoreLogOnFailure(testInfo, harness.runtimePath)
          await attachGameLogOnFailure(testInfo, harness.runtimePath, shortpath)
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
