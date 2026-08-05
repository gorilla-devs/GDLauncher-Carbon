import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  deleteInstanceViaUi,
  ensureLibraryInteractive
} from "./helpers/instances.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { readPackinfo } from "./helpers/packinfo.js"
import { readInstallAudit } from "./helpers/installAudit.js"
import { snapshotTree } from "./helpers/instanceTree.js"
import {
  changeModpackVersion,
  fetchMrpackIndex,
  installModpackVersion,
  openInstanceSettings,
  packPaths,
  unlockModpack,
  unpairModpack,
  type PackIndex
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_SLUG,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW
} from "./helpers/modpackFixtures.js"
import {
  installAddonVersion,
  openAddonPage,
  openAddonVersions,
  openInstanceAddons,
  searchForMod,
  type InstalledMod
} from "./helpers/mods.js"
import { verifyModInstalled } from "./helpers/modVerify.js"

/**
 * Covers the one part of the modpack lifecycle none of the other three
 * modpack spec files touch: the `locked` flag itself — a fresh install
 * starts locked, unlocking flips it and unblocks Addons, and unpairing drops
 * the association (and the flag with it) entirely.
 *
 * **One shared install, four serial tests.** `beforeAll` installs
 * `MODPACK_MR_V_MID` once; `test.describe.configure({ mode: "serial" })`
 * makes the four tests run in file order on that one instance, since test 2
 * unlocks it and tests 3-4 depend on that already having happened. `page`/
 * `harness` come from the worker-scoped `authenticatedApp` fixture, which
 * `beforeAll` can use directly — it is a worker fixture, so (unlike the
 * built-in, test-scoped `page`) it is visible there, the same way every
 * other worker fixture in this suite (`installedInstance`, `forgeInstance`)
 * is composed in `fixtures/index.ts`.
 *
 * **Unlock is one-way.** `Settings/index.tsx` renders only a `Set: false`
 * button gated on `modpack.locked`; the re-lock call
 * (`openModal("unlock_confirmation", ...)`) is commented out in source and
 * nothing else in the shipped UI sets the flag back to `true`. So this file
 * asserts what it can reach — locked, then unlocked — and never attempts a
 * re-lock.
 *
 * **`unlockModpack`/`unpairModpack` leave the page on the instance's
 * Settings tab**, not `/library` (`helpers/modpacks.ts`'s own doc comments
 * on both). `afterEach` below returns to `/library` via the navbar logo
 * before asserting the library is interactive, the same pattern
 * `modpackInstall.spec.ts`/`modpackSaveGuard.spec.ts` use — this is what
 * keeps that navigation from being every test body's own problem.
 *
 * **Fabric API collision, found live while writing test 2:
 * `installModIntoInstance` (the addon page's header button, "install
 * latest") cannot be used for `P7dR8mSH` against this pack.** "remarkably"
 * bundles its own copy of Fabric API (`fabric-api-0.92.6+1.20.1.jar`, per
 * `modpackLifecycle.spec.ts`'s own `KNOWN_STALE_SURVIVORS_AFTER_DOWNGRADE`
 * list), and every mod file's Modrinth project association — however it
 * arrived on disk — is resolved automatically (confirmed live, a throwaway
 * probe: `openInstanceAddons` right after install already reports
 * `modrinthProjectId: "P7dR8mSH"` for the pack's own bundled copy). Reading
 * `ModDownloadButton`'s `isInstalled()` (`components/ModDownloadButton/index.tsx`)
 * confirms why that matters: with no `fileId` (the header button's "install
 * latest" path), it returns `!!installedMod()` — true the instant *any* mod
 * with a matching project id exists, independent of which specific build.
 * So the header button already reads "Downloaded" before this file ever
 * clicks it, and `installModIntoInstance`'s own "not already Downloaded"
 * precondition would fail immediately. The fix: install a *specific,
 * different* Fabric API build through the addon page's Versions tab
 * (`openAddonVersions` + `installAddonVersion`, explicitly excluding the
 * pack's own bundled version id) — `isInstalled()` for a set `fileId`
 * compares `modrinth!.version_id` instead, so a genuinely different build
 * correctly reads as not-yet-installed. `modLifecycle.spec.ts`'s own update
 * test already proves this exact mechanism against this exact mod
 * (Fabric API) on a plain instance, so this is a proven path, not a new one.
 *
 * **Why test 3 also hand-edits a pack config.** The Fabric API build
 * installed in test 2 is never a packinfo key, so nothing in
 * `process_modpack_staging` (`run/modpack.rs`) may ever consider it: loop
 * 1 walks `packinfo.files`, loop 2 (`modpack.rs:808-823`, the staging-walk
 * mutation's target) walks `staging_dir`, which `process_modpack`
 * (`modpack.rs:336`)
 * creates *empty* and fills only from the pack's own declared manifest. A
 * path that is in neither input is unreachable by a mutation confined to
 * either loop, by construction — which is exactly test 3's whole point, but
 * it also means the staging-walk mutation (dropping its
 * `!original_file.exists()` guard) has no way to touch it: dropping a guard
 * inside a loop that never iterates this path cannot make an assertion
 * about this path go red. Confirmed independently by `modpackLifecycle.spec.ts`'s
 * own structurally equivalent sabotage
 * (`modpack.rs:815`, there swapped for a packinfo-membership check), which
 * went red on a *pack-tracked, user-deleted* file and nothing else — never on
 * that file's synthetic, wholly-untracked
 * `userMod`/`userSave`. So this test adds one pristine pack config, edited
 * by hand before the version change (the same mechanism
 * `modpackLifecycle.spec.ts`'s own `editTarget` uses, at far smaller scale —
 * one file, no real launch to partition pristine-vs-already-modified first,
 * since this file's instance is never actually launched, so every packinfo
 * entry is still pristine by construction). `ModifiedByUser` leaves that
 * file's staged replacement behind, unconsumed, in `staging_dir`
 * (`modpack.rs:776-783` `continue`s before the rename at :802) — exactly
 * what a dropped guard at :815 would then overwrite. This is what that
 * mutation actually goes red on here; see the test's own comment.
 *
 * **Two bugs in the brief's own test 3 snippet, fixed here.** First,
 * `verifyModInstalled(dir, filename)` — the real signature is
 * `verifyModInstalled(dir, { filename, ... })`; the snippet passes a bare
 * string where an options object is required (a TypeScript error, not a
 * runtime one). Second, `expect(packinfo).not.toHaveProperty(key)` against
 * `readPackinfo`'s return — a `Map`, not a plain object — is vacuous:
 * `toHaveProperty` reads a plain-object property path, which a `Map`
 * exposes none of (entries live behind `.get`/`.has`, not own-enumerable
 * properties), so this always passes regardless of what the map actually
 * contains. Replaced with `expect(map.has(key)).toBe(false)`, which reads
 * the map correctly.
 *
 * **The packinfo-gap comparison in test 3** follows the exact accounting
 * `modpackLifecycle.spec.ts` already established and documents in depth:
 * `packinfo::scan_dir` rebuilds `packinfo.json` purely by hashing whatever
 * physically landed in `staging_dir`, and `prepare_modpack_from_mrpack`
 * skips re-downloading (and thus re-staging) a file whose new-version
 * sha512 already matches what the old packinfo recorded — so a pack file
 * unchanged between `MODPACK_MR_V_MID` and `MODPACK_MR_V_NEW` is silently
 * absent from the rebuilt `packinfo.json`, independent of anything a test
 * does. Asserting the brief's own literal
 * `packPaths(next) === [...packinfo.keys()]` here would be asserting
 * something false under correct operation, exactly as the task brief warns —
 * so this computes the same `expectedMissingFromSkipGap` predicate
 * `modpackLifecycle.spec.ts` does (paths where MID's and NEW's own declared
 * sha512 agree) and asserts equality against *that*, not against the naive
 * full set. Not derived from `classifyPackinfo`/a real launch's partition
 * the way that file's version is, because this file's instance is never
 * launched — every packinfo entry here starts pristine and stays that way
 * except for the one file this test edits by hand.
 *
 * **The unlock-button mutation needs no such workaround.**
 * `Settings/index.tsx`'s unlock
 * button's mutation is a direct, literal `Set: false` — changing it to
 * `Set: true` is exactly what test 2's `locked).toBe(false)` assertion
 * exists to catch.
 *
 * **A pre-existing gap in `installModpackVersion` itself, found live running
 * this file, worked around locally rather than touched.** Its own retry loop
 * (`VERSIONS_TAB_MAX_ATTEMPTS`) covers landing on the Versions tab route, but
 * the `scrollVersionRowIntoView` call immediately after it is not inside
 * that loop, and has no settle-window protection against
 * `InfiniteScrollVersionsQueryWrapper`'s own scoping `createEffect`
 * (`components/InfiniteScrollVersionsQueryWrapper/index.tsx`) wiping and
 * refetching the whole row set out from under it — unlike `openAddonVersions`
 * (`helpers/mods.ts`), which explicitly waits for that same effect's request
 * count to settle before ever returning, for exactly this reason (see its own
 * doc comment). Hit reproducibly this session: 9 consecutive identical
 * failures — `scrollVersionRowIntoView`'s "may have been replaced by a
 * re-render" message, same version id, same call site — and confirmed to be
 * this gap and not this file's own doing by re-running the already-committed,
 * previously-green `modpackSaveGuard.spec.ts` against the identical
 * `installModpackVersion(MODPACK_MR_QUERY, "modrinth", MODPACK_MR_V_MID)`
 * call: it failed the same way, at the same line, in the same session. Not a
 * live-service issue either — a full response log covering the whole install
 * (`page.on("response")`) showed zero non-2xx Modrinth responses; the one
 * console "Failed to load resource: 500" that kept appearing traces to the
 * app's own unrelated YouTube embed, not to anything this suite talks to.
 * `installModpackVersionRetrying` below retries the *whole* call (never
 * partial — every failure mode observed here happens before an instance is
 * ever created, so a retry always starts clean from
 * `installModpackVersion`'s own `navbarLogo` reset), the same "just re-run
 * it" mitigation this suite's own README prescribes for exactly this class
 * of issue, applied locally in this file rather than by hand between
 * invocations. This is additive code in this file only — `helpers/modpacks.ts`
 * itself is untouched, per the brief's "verified, do not reimplement it".
 *
 * **A second pre-existing gap, in `unlockModpack`/`unpairModpack` this time,
 * also worked around locally.** `Settings/index.tsx`'s `updateInstanceMutation`
 * carries an `onMutate` handler that optimistically writes the mutation's
 * effect straight into the `instanceDetails` query cache — for
 * `modpackLocked: { Set: <bool> }` (unlock) or `{ Set: null }` (unpair) alike
 * (lines 56-63) — client-side, before the real rspc round trip (and the disk
 * write it drives, `update_instance`/`managers/instance/mod.rs`) ever
 * completes. That is what actually makes the unlock button unmount /
 * the modpack block disappear, so `unlockModpack`'s and `unpairModpack`'s own
 * `toHaveCount(0)` waits (`helpers/modpacks.ts`) are satisfied by the
 * optimistic update alone and prove nothing about disk. Structurally the
 * same class of gap `helpers/mods.ts`'s `toggleModEnabled`/`deleteModViaUi`
 * already document and guard against for the analogous mod-enable toggle
 * (`optimisticToggleAddon` runs synchronously ahead of `mutateAsync` there
 * too) — neither modpack helper has an equivalent "await the real response"
 * guard. Confirmed live, this task: a bare `readInstanceConfig` read
 * immediately after `unlockModpack` returned observed `locked: true` — the
 * pre-unlock value — twice across three runs. Tests 2 and 4 below poll the
 * disk read instead of reading it once, the same defensive shape this suite
 * already uses everywhere a UI signal races a backend effect (`expect.poll`),
 * rather than trusting either helper's return as proof of anything on disk.
 */
test.describe("modpack lock, unlock and unpair", () => {
  test.describe.configure({ mode: "serial" })

  let name: string
  let root: string
  let midIndex: PackIndex
  let installedUserMod: InstalledMod

  test.beforeAll(async ({ authenticatedApp }) => {
    const { page, harness } = authenticatedApp
    name = await installModpackVersion(
      page,
      MODPACK_MR_QUERY,
      "modrinth",
      MODPACK_MR_V_MID
    )
    const { shortpath } = readInstanceByName(harness.runtimePath, name)
    root = path.join(harness.runtimePath, "instances", shortpath)
    midIndex = await fetchMrpackIndex(MODPACK_MR_V_MID)
  })

  test.afterAll(async ({ authenticatedApp }) => {
    // Best-effort: this only tidies the shared library for whatever spec
    // file runs next in this worker (CI pins `workers: 1`, so every spec
    // file in a full run shares one app instance and runtime path). A
    // cleanup failure here must never be allowed to override or obscure
    // whatever the four tests above already reported.
    if (!name) return
    try {
      await authenticatedApp.page
        .locator(byTestId(TEST_IDS.navbarLogo))
        .click({ timeout: 5_000 })
      await deleteInstanceViaUi(authenticatedApp.page, name)
    } catch (cleanupError) {
      console.error(`cleanup: deleting "${name}" also failed:`, cleanupError)
    }
  })

  test.afterEach(async ({ authenticatedApp }, testInfo) => {
    await attachCoreLogOnFailure(testInfo, authenticatedApp.harness.runtimePath)
    await authenticatedApp.page
      .locator(byTestId(TEST_IDS.navbarLogo))
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(authenticatedApp.page)
  })

  test("a freshly installed modpack is locked", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp

    const config = await readInstanceConfig(root)
    expect(
      config.modpack?.locked,
      "a freshly installed modpack instance must be locked"
    ).toBe(true)

    // `Tabs/Addons/index.tsx:68-69` derives `isInstanceLocked()` from
    // `instanceDetails.data?.modpack?.locked` and passes it down at :180 as
    // `addButtonDisabled`, with a `_trn_locked_cannot_apply_changes` tooltip.
    // `addonsAddButton` anchors the button that receives it
    // (`AddonFilters.tsx`, a real `@gd/ui` `Button` that spreads `disabled`
    // onto its native element, not a class-based fake).
    await openInstanceAddons(page, name)
    await expect(
      page.locator(byTestId(TEST_IDS.addonsAddButton)),
      "a locked modpack instance still allowed addons to be added"
    ).toBeDisabled()
  })

  test("unlocking lets a mod be installed", async ({ authenticatedApp }) => {
    const { page } = authenticatedApp

    await unlockModpack(page, name)
    // Polled, not a single read — see the module doc comment on
    // `updateInstanceMutation`'s optimistic cache update: `unlockModpack`'s
    // own `toHaveCount(0)` wait (`helpers/modpacks.ts`) is satisfied by that
    // optimistic update, which lands client-side before the real rspc round
    // trip — and the disk write it drives (`update_instance`,
    // `managers/instance/mod.rs`) — actually completes. Confirmed live this
    // task: a bare read immediately after `unlockModpack` returned observed
    // `locked: true` twice across three runs.
    await expect
      .poll(async () => (await readInstanceConfig(root)).modpack?.locked, {
        message: "unlockModpack did not flip locked to false on disk"
      })
      .toBe(false)

    // `unlockModpack` leaves the page on the instance's Settings tab (see
    // the module doc comment) — `addonsAddButton` only exists on the Addons
    // tab, so this must navigate there before it can assert anything about
    // that button. See the module doc comment for why the pack's own
    // bundled Fabric API makes the header "install latest" button unusable
    // here: a specific, different build via the Versions tab is what
    // actually exercises "unlocking lets a mod be installed".
    const modsBeforeInstall = await openInstanceAddons(page, name)
    await expect(
      page.locator(byTestId(TEST_IDS.addonsAddButton)),
      "the Add Addons button stayed disabled after unlocking"
    ).toBeEnabled()
    const bundled = modsBeforeInstall.find(
      (m) => m.modrinthProjectId === "P7dR8mSH"
    )

    await searchForMod(page, { platform: "modrinth", query: "fabric api" })
    await openAddonPage(page, "P7dR8mSH")

    const versions = await openAddonVersions(page)
    const target = versions
      .filter((v) => v.fileId !== bundled?.modrinthVersionId)
      .sort(
        (a, b) => Date.parse(b.datePublished) - Date.parse(a.datePublished)
      )[0]
    expect(
      target,
      "no Fabric API build is available other than the one the pack itself " +
        `bundles (version id ${bundled?.modrinthVersionId}) — re-check the ` +
        "project's Versions tab"
    ).toBeDefined()
    await installAddonVersion(page, target)

    const mods = await openInstanceAddons(page, name)
    installedUserMod = mods.find(
      (m) =>
        m.modrinthProjectId === "P7dR8mSH" &&
        m.modrinthVersionId === target.fileId
    )!
    expect(
      installedUserMod,
      "Fabric API did not install into the unlocked instance"
    ).toBeDefined()
    expect(
      await verifyModInstalled(path.join(root, "instance", "mods"), {
        filename: installedUserMod.filename
      })
    ).toMatchObject({ ok: true })
  })

  test("a user's own mod survives a modpack version change", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp
    const data = path.join(root, "instance")
    const next = await fetchMrpackIndex(MODPACK_MR_V_NEW)

    // See the module doc comment: gives the staging-walk mutation a genuinely
    // reachable
    // target, since the Fabric API mod below structurally cannot be one.
    // No real launch ever happens in this file, so every packinfo entry is
    // still pristine at this point — no `classifyPackinfo` partition needed,
    // unlike `modpackLifecycle.spec.ts`. Picked from `/config/` overrides
    // still declared by NEW: overrides are unconditionally re-extracted on
    // every version change regardless of content (`modrinth.rs:321-379`),
    // so a still-declared one is guaranteed a fresh staged copy for this
    // transition — a `/mods/` entry would not do, since a pack mod skips
    // re-staging entirely when its bytes are unchanged (the same mechanism
    // behind the packinfo gap below), leaving nothing in `staging_dir` for
    // the guard to matter against.
    const packinfoBefore = await readPackinfo(root)
    const nextOverridePaths = new Set(next.overrides)
    const editKey = [...packinfoBefore.keys()].find(
      (k) => k.startsWith("/config/") && nextOverridePaths.has(k.slice(1))
    )
    expect(
      editKey,
      `no pack config in "${MODPACK_MR_SLUG}" both lives under /config/ and ` +
        "survives into the new version — re-check the pack's own overrides"
    ).toBeDefined()
    const editedRelPath = editKey!.slice(1)
    const editedBody = `e2e-modpack-lock-edited-${Date.now()}\n`
    await fs.promises.writeFile(path.join(data, editedRelPath), editedBody)

    const before = await snapshotTree(data)

    await changeModpackVersion(page, name, MODPACK_MR_V_NEW)

    const after = await snapshotTree(data)
    const rel = `mods/${installedUserMod.filename}`
    expect(
      after.get(rel)?.sha256,
      "a version change deleted or rewrote a mod the user installed themselves"
    ).toBe(before.get(rel)?.sha256)

    const audit = await readInstallAudit(root)
    expect(audit, "the version change wrote no install audit").not.toBeNull()
    expect(audit!.deleted, `audit claims it deleted ${rel}`).not.toContain(
      `/${rel}`
    )
    expect(audit!.replaced, `audit claims it replaced ${rel}`).not.toContain(
      `/${rel}`
    )
    expect(
      audit!.created.map((p) => p.replace(/^instance\//, "")),
      `audit claims it created ${rel}`
    ).not.toContain(rel)
    expect(
      audit!.skipped.map((s) => s.file),
      `audit claims it skipped ${rel}`
    ).not.toContain(`/${rel}`)

    // The hand-edited config also survives, byte-identical, reported
    // modified-by-user. This is the assertion that mutation actually goes red
    // on — see the module doc comment.
    expect(
      await fs.promises.readFile(path.join(data, editedRelPath), "utf8"),
      "the version change overwrote a user-edited pack config"
    ).toBe(editedBody)
    const editSkip = audit!.skipped.find((s) => s.file === editKey)
    expect(
      editSkip?.reason,
      `audit reason for the edited config ${editKey}`
    ).toBe("modified-by-user")

    // And the pack itself still updated correctly around both of them —
    // accounting for packinfo::scan_dir's own unchanged-file gap. See the
    // module doc comment for why the naive `packPaths(next)` equality the
    // brief's own snippet uses would be false under correct operation.
    const midShaByPath = new Map(midIndex.files.map((f) => [f.path, f.sha512]))
    const nextShaByPath = new Map(next.files.map((f) => [f.path, f.sha512]))
    const packinfoAfter = await readPackinfo(root)
    const packinfoPathsAfter = new Set(
      [...packinfoAfter.keys()].map((k) => k.slice(1))
    )
    const nextPathSet = new Set(packPaths(next))
    const missingFromPackinfo = packPaths(next)
      .filter((p) => !packinfoPathsAfter.has(p))
      .sort()
    const expectedMissingFromSkipGap = packPaths(next)
      .filter((p) => {
        const midSha = midShaByPath.get(p)
        const nextSha = nextShaByPath.get(p)
        return (
          midSha !== undefined && nextSha !== undefined && midSha === nextSha
        )
      })
      .sort()
    expect(
      missingFromPackinfo,
      "packinfo.json is missing pack files beyond the known unchanged-file " +
        "gap in packinfo::scan_dir"
    ).toEqual(expectedMissingFromSkipGap)
    expect(
      [...packinfoPathsAfter].filter((p) => !nextPathSet.has(p)),
      "packinfo.json records a path the new version does not declare"
    ).toEqual([])
    // `readPackinfo` returns a Map, so `.has` is the correct probe — see the
    // module doc comment for why `toHaveProperty` here would be vacuous.
    expect(
      packinfoAfter.has(`/${rel}`),
      "packinfo.json tracks a mod the user installed themselves"
    ).toBe(false)
  })

  test("unpairing removes the modpack association entirely", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp

    await unpairModpack(page, name)

    // Polled for the same reason test 2 polls its disk read: `unpairModpack`
    // sends `modpackLocked: { Set: null }`, which the same optimistic
    // `onMutate` handler in `Settings/index.tsx` (lines 56-63) also covers —
    // `variables.modpackLocked.Set === null` clears the cached `modpack`
    // client-side before the real round trip (and disk write) completes, so
    // `unpairModpack`'s own `toHaveCount(0)` wait proves nothing about disk
    // here either.
    await expect
      .poll(async () => (await readInstanceConfig(root)).modpack, {
        message: "unpair left a modpack on the instance"
      })
      // `parseModpack` (`helpers/instanceConfig.ts`) maps an absent/null
      // `modpack` key to `null`, never `undefined` — `v1::Instance.modpack`
      // is `Option<ModpackInfo>` and the raw JSON key can be entirely
      // absent, but either way the parsed result here is `null`.
      .toBeNull()

    await openInstanceSettings(page, name)
    await expect(
      page.locator(byTestId(TEST_IDS.instanceSettingsChangeVersion))
    ).toHaveCount(0)
    await expect(
      page.locator(byTestId(TEST_IDS.instanceSettingsUnpair))
    ).toHaveCount(0)
  })
})
