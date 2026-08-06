import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import { ensureLibraryInteractive } from "./helpers/instances.js"
import { ADDON_FIXTURES, addonDir } from "./helpers/addonFixtures.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import {
  cleanupInstalledMod,
  deleteModViaUi,
  installAddonVersion,
  openAddonPage,
  openAddonVersions,
  openInstanceAddons,
  searchForMod,
  toggleModEnabled,
  waitForModFilenameChange,
  type InstalledMod
} from "./helpers/mods.js"

/**
 * True for a download-in-progress marker, never a finished addon's real
 * on-disk name: `carbon_net`'s downloader writes to `<file>.__gdl_part~`
 * while a download is in flight and renames it away once complete
 * (`PART_POSTFIX`, `crates/carbon_net/src/lib.rs:24`). Excluded from the
 * install diff below so a poll landing mid-download can never be mistaken
 * for the addon actually finishing — same hazard, and same fix,
 * `addonPlacement.spec.ts` documents against its own diff. Copied rather
 * than imported: `addonPlacement.spec.ts` does not export it, and it is a
 * three-line pure predicate with no shared state worth introducing a new
 * cross-file dependency for.
 */
const isPartFile = (name: string) => name.endsWith(".__gdl_part~")

/**
 * How long the app's own addon list is given to catch up with a file that
 * has already been confirmed on disk, after `installModIntoInstance`
 * returns. Mirrors `addonPlacement.spec.ts`'s constant of the same name,
 * value and rationale: a resource pack's completion signal is the
 * install button's own text, which is gated on `isInstalled()` reading the
 * same reconciled list `openInstanceAddons` fetches — so by construction the
 * first poll attempt here is expected to succeed immediately. Kept as a poll
 * rather than a single read anyway, per that spec's own hard-won lesson that
 * the app's addon list is not guaranteed to be caught up with disk on every
 * run. The successful call also leaves `page` on the instance's Addons tab,
 * which `toggleModEnabled` below requires.
 */
const RECONCILIATION_WAIT = 30_000

/**
 * Full lifecycle for the **file** family — resource packs, shaders and
 * datapacks share one code path that differs only by folder, and the folder
 * is already pinned per type by `addonPlacement.spec.ts`. Running the
 * identical sequence three times under three folder names would be
 * repetition, not coverage, so one representative carries it.
 *
 * Resource packs are the representative: the type with the widest, most
 * stable catalogue on both platforms.
 *
 * Worlds are deliberately **not** here — they are a directory on disk with a
 * different metadata parser and no enable toggle at all (`AddonTable`'s
 * enabled column has nothing to key off for something that isn't a single
 * file — see `installModIntoInstance`'s doc comment on why `isInstalled()`
 * structurally can never match one either). They get their own spec, not
 * this one.
 */
test.describe("addon lifecycle (file-backed types)", () => {
  test.describe.configure({ mode: "serial" })

  // Same reasoning as addonPlacement.spec.ts/modLifecycle.spec.ts's identical
  // hook: `installedInstance` is worker-scoped, so `afterEach` is the one
  // hook that still gets both its value and a real per-test `TestInfo`.
  test.afterEach(async ({ installedInstance }, testInfo) => {
    await attachCoreLogOnFailure(
      testInfo,
      installedInstance.harness.runtimePath
    )
    await installedInstance.page
      .locator(byTestId(TEST_IDS.navbarLogo))
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(installedInstance.page)
  })

  const fixture = ADDON_FIXTURES.find(
    (f) => f.addonType === "resourcepacks" && f.platform === "modrinth"
  )
  if (!fixture) {
    throw new Error(
      'addonLifecycle.spec.ts: ADDON_FIXTURES has no "resourcepacks"/"modrinth" ' +
        "entry — helpers/addonFixtures.ts may have changed shape"
    )
  }

  // Keyed on the platform's own project id rather than a filename: this is
  // what lets `finally`'s cleanup below find (and remove) whatever this test
  // installed even if it failed before ever computing a filename off disk.
  const matchesFixture = (mod: InstalledMod) =>
    mod.modrinthProjectId === fixture.projectId

  test("installs, disables, re-enables, moves to a newer build and deletes a resource pack", async ({
    installedInstance
  }) => {
    const { page, harness, instanceName } = installedInstance
    const { shortpath } = readInstanceByName(harness.runtimePath, instanceName)

    // `<runtimePath>/instances/<shortpath>` is the instance's root; every
    // addon folder lives one level deeper, under its `instance/` data
    // subdirectory. Copied from `addonPlacement.spec.ts` rather than
    // re-derived — see this suite's task brief for why getting this wrong
    // silently checks the wrong directory instead of failing loudly.
    const instanceRoot = path.join(harness.runtimePath, "instances", shortpath)
    const instanceDataPath = path.join(instanceRoot, "instance")
    const dir = addonDir(instanceDataPath, "resourcepacks")

    const before = fs.existsSync(dir) ? fs.readdirSync(dir) : []

    let bodyFailed = false
    try {
      await openInstanceAddons(page, instanceName)
      await searchForMod(page, {
        query: fixture.query,
        platform: fixture.platform,
        searchType: fixture.searchType
      })
      // Required before opening the Versions tab: the addon page is only
      // reachable from a search result, and only an addon page reached with
      // `?instanceId=` renders an install control at all — see
      // `TEST_IDS.addonInstallButton`'s doc comment in
      // `helpers/selectors.ts`. Same order every other installer in this
      // suite uses (`addonPlacement.spec.ts`, `modLifecycle.spec.ts`'s
      // `installFabricApi`).
      await openAddonPage(page, fixture.projectId)

      // A deliberately-not-newest build, so the version move later in this
      // lifecycle has somewhere real to go — the same `INSTALL_MOD` path
      // (a specific `version_id`, never "latest") `modLifecycle.spec.ts`'s
      // update test uses, and what `helpers/addonFixtures.ts`'s "at least two
      // release-channel versions for 1.20.1" criterion exists to serve.
      const versions = await openAddonVersions(page)
      const byNewestFirst = [...versions].sort(
        (a, b) => Date.parse(b.datePublished) - Date.parse(a.datePublished)
      )
      const oldest = byNewestFirst[byNewestFirst.length - 1]
      const newest = byNewestFirst[0]
      expect(oldest.fileId, {
        message:
          "this lifecycle needs two distinct builds to move between; the " +
          "fixture reported only one for this instance's Minecraft version"
      }).not.toBe(newest.fileId)

      // The oldest, deliberately *not* `pickOlderVersion`'s second-newest.
      // The two newest builds of this fixture are byte-identical — Modrinth
      // reports the same sha1 (96932a034fff4cd0cad08ba12db730450c4772da) for
      // "LowOnFire v26.1§8.zip" and "LowOnFire v26.2§8.zip" — and the app
      // identifies an installed file by hashing it and asking Modrinth which
      // version owns that hash (`managers/metadata/cache/modrinth/mod.rs`).
      // Installing the second-newest therefore reconciles as the *newest*:
      // confirmed live (2026-08-06), where the version row for the build that
      // had just been installed read "Switch Version" rather than
      // "Downloaded" — `InstallButton.tsx` renders that exact label for
      // `installedMod() && !isInstalled()`, i.e. "this project is installed,
      // at a different version than this row". Two versions is not enough on
      // its own; two versions with *distinct file hashes* is the real
      // requirement, and only the oldest of this project's three 1.20.1
      // builds satisfies it against the newest.
      await installAddonVersion(page, oldest)

      const after = fs.existsSync(dir) ? fs.readdirSync(dir) : []
      const added = after.filter((f) => !before.includes(f) && !isPartFile(f))
      expect(added, {
        message: `resource pack install produced no new entry in ${dir}`
      }).toHaveLength(1)
      const filename = added[0]

      // The app's own list can lag disk by several seconds after an install
      // (the periodic metadata-cache pass, decoupled from the mutation this
      // already awaited) — poll rather than read once. See
      // `RECONCILIATION_WAIT`'s doc comment for why this is expected to
      // resolve on the first attempt for a resource pack specifically, and
      // why it is kept as a poll anyway.
      let listed: InstalledMod[] = []
      await expect
        .poll(
          async () => {
            listed = await openInstanceAddons(page, instanceName)
            return listed.some((m) => m.filename === filename)
          },
          {
            timeout: RECONCILIATION_WAIT,
            message: `the app's own list never reported "${filename}"`
          }
        )
        .toBe(true)

      // Disable: the file gains `.disabled` on disk, and the app agrees.
      await toggleModEnabled(page, filename, false)
      await expect
        .poll(() => fs.existsSync(path.join(dir, `${filename}.disabled`)), {
          message: "disabling must rename the pack to <name>.disabled on disk"
        })
        .toBe(true)
      expect(fs.existsSync(path.join(dir, filename))).toBe(false)

      listed = await openInstanceAddons(page, instanceName)
      expect(listed.find((m) => m.filename === filename)?.enabled).toBe(false)

      // Re-enable: back to the original name. `toggleModEnabled` locates its
      // row by the app's cached *base* filename (`byModRow`'s doc comment in
      // `helpers/selectors.ts`, cross-checked against `enable_mod` in
      // `managers/instance/mods.rs` — there is no separate `disable_mod`;
      // the `instance.enableMod`/`instance.disableMod` rspc mutations both
      // call this one function with `enabled: true`/`false`
      // (`api/instance/mod.rs`'s `ENABLE_MOD`/`DISABLE_MOD`), and it derives
      // the enabled/disabled on-disk paths from the same cached `m.filename`
      // regardless of which way it's called — and against `AddonTable.tsx`'s
      // `data-mod-filename={row.original.filename}`) — that cached field is
      // never suffixed with `.disabled` on either side of the toggle, so
      // this passes the same bare `filename`, not `${filename}.disabled`.
      await toggleModEnabled(page, filename, true)
      await expect
        .poll(() => fs.existsSync(path.join(dir, filename)), {
          message: "re-enabling must restore the original filename"
        })
        .toBe(true)
      expect(fs.existsSync(path.join(dir, `${filename}.disabled`))).toBe(false)

      listed = await openInstanceAddons(page, instanceName)
      expect(listed.find((m) => m.filename === filename)?.enabled).toBe(true)

      // Move to the newest build, replacing the one installed above: the
      // file on disk becomes the newer build's, and the older build's file is
      // gone rather than left beside it (the installer removes the addon
      // named by `replaces_mod` once the new file lands —
      // `managers/instance/installer/mod.rs`).
      //
      // Driven from the Versions tab, **not** the row's update button, and
      // that is a deliberate, evidence-backed substitution rather than an
      // oversight — the design (`2026-08-05-non-mod-addon-coverage-design.md`,
      // line 158) named `update` as this step. `Mod.has_update` is
      // structurally always false for every non-mod addon type, so the update
      // button never renders for one at all:
      //
      //   - `list_mods` (`managers/instance/mods.rs`) derives the instance's
      //     update paths from `version.modloaders` alone, as
      //     `(game_version, loader)` pairs, and only reports an update when a
      //     file's own stored path matches one exactly;
      //   - a file's stored paths only ever contain a loader that parses as
      //     `ModLoaderType` — forge/fabric/quilt/neoforge and nothing else
      //     (`ModLoaderType::try_from`, `domain/instance/info.rs`). Both
      //     cachers skip everything else, which
      //     `managers/metadata/cache/modrinth/mod.rs`'s own
      //     `unknown_loaders_are_skipped` unit test already pins;
      //   - resource packs, shaders, datapacks and worlds carry no such
      //     loader. Confirmed live on 2026-08-06: every Modrinth version of
      //     this fixture reports `loaders: ["minecraft"]`, and the CurseForge
      //     resource-pack fixture's files report `gameVersions:
      //     ["1.20.1", "1.20"]` with no loader token.
      //
      // So an "assert an update is offered" step here could only ever fail.
      // The version move below covers what that step was actually for — a
      // file-backed addon moving from one real build to another, with the
      // disk following — through the affordance the product genuinely offers
      // for these types ("Switch Version", `handleSwitchVersion`), and is the
      // first coverage of `replaces_mod` for a non-mod addon. See
      // `final-fix-report.md` in this plan's folder for the full evidence.
      await openInstanceAddons(page, instanceName)
      await searchForMod(page, {
        query: fixture.query,
        platform: fixture.platform,
        searchType: fixture.searchType
      })
      await openAddonPage(page, fixture.projectId)
      await openAddonVersions(page)
      await installAddonVersion(page, newest)

      const updated = await waitForModFilenameChange(page, instanceName, {
        oldFilename: filename,
        matches: matchesFixture
      })
      await expect
        .poll(() => fs.existsSync(path.join(dir, updated.filename)), {
          message: "moving to the newer build must put its file on disk"
        })
        .toBe(true)
      expect(fs.existsSync(path.join(dir, filename))).toBe(false)

      // Delete: gone from disk, both spellings.
      await deleteModViaUi(page, updated.filename)
      await expect
        .poll(() => fs.existsSync(path.join(dir, updated.filename)), {
          message: "deleting a resource pack must remove it from disk"
        })
        .toBe(false)
      expect(
        fs.existsSync(path.join(dir, `${updated.filename}.disabled`))
      ).toBe(false)

      listed = await openInstanceAddons(page, instanceName)
      expect(
        listed.find((m) => m.filename === updated.filename)
      ).toBeUndefined()
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      // The delete under test already returns the instance to a clean state
      // on its own success — this only catches a body that failed before or
      // during that step, which would otherwise leave a resource pack behind
      // for a later spec sharing this worker's `installedInstance` to trip
      // over. Mirrors `modLifecycle.spec.ts`'s identical "deletes an
      // installed mod" cleanup, reusing the same shared
      // `cleanupInstalledMod` helper.
      try {
        // `cleanupInstalledMod` verifies its own removal against a plain,
        // extension-agnostic directory listing, so a `.zip` left behind here
        // fails it — no second, local copy of that check is needed here (see
        // that helper's doc comment).
        await cleanupInstalledMod(
          page,
          instanceName,
          dir,
          matchesFixture,
          "cleanupResourcePack"
        )
      } catch (cleanupError) {
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupError
        }
        console.error(
          'cleanup for "installs, disables, re-enables, moves to a newer ' +
            'build and deletes a resource pack" also failed:',
          cleanupError
        )
      }
    }
  })
})
