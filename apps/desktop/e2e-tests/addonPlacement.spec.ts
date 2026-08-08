import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import { ensureLibraryInteractive } from "./helpers/instances.js"
import { ADDON_FIXTURES, addonDir } from "./helpers/addonFixtures.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import {
  INSTALL_TIMEOUT,
  installModIntoInstance,
  openAddonPage,
  openInstanceAddons,
  searchForMod,
  type InstalledMod
} from "./helpers/mods.js"

/**
 * How long the app's own addon list is given to catch up with a file that
 * has already been confirmed on disk, after `installModIntoInstance`
 * returns. Needed specifically because a **world**'s completion signal
 * (`installModIntoInstance`'s `waitForCompletion` below) is a raw disk poll,
 * not the button-text wait every other addon type uses — and that
 * button-text wait is itself gated on `isInstalled()`, which reads the same
 * reconciled list `openInstanceAddons` fetches, so by construction it can
 * never return before that list is caught up. A disk poll has no such
 * guarantee: confirmed live (this spec's own curseforge-worlds case, core
 * log timestamps) that `managers/metadata/cache/mod.rs`'s periodic
 * `local mod caching` pass — the one that actually inserts the DB row
 * `instance.getInstanceMods` reads — can still be several seconds away from
 * its next cycle at the exact moment the file already exists on disk (one
 * observed gap: file present, but the next `Completed local mod caching...
 * updated 1 entries` line landed ~4.5s later). 30s is generous margin over
 * that, not a tuned minimum. A no-op for every non-world fixture: their own
 * completion wait already guarantees the list is caught up by the time
 * `installModIntoInstance` returns, so the very first poll attempt here
 * succeeds immediately.
 */
const RECONCILIATION_WAIT = 30_000

/**
 * True for a download-in-progress marker, never a finished addon's real
 * on-disk name: `carbon_net`'s downloader writes to `<file>.__gdl_part~`
 * while a download is in flight and renames it away once complete
 * (`PART_POSTFIX`, `crates/carbon_net/src/lib.rs:24`) — a mechanism shared
 * by every addon type's download, not just worlds'. Excluded from every
 * diff below so a poll landing mid-download can never be mistaken for the
 * addon actually finishing. Confirmed live: before this filter existed,
 * this spec's own curseforge-worlds case once caught exactly this file
 * (`Find The Button 2.zip.__gdl_part~`) as its whole `added` set, which
 * then could never appear in the app's own list — that filename was never
 * going to exist long enough to be read back, by design.
 */
const isPartFile = (name: string) => name.endsWith(".__gdl_part~")

/** How long the `ShaderLoaderSetup` wizard's intro step is given to appear
 *  after the install click, for the cancel leg below. Mirrors `helpers/mods.ts`'s
 *  own private `SHADER_WIZARD_WAIT` (same mechanism — a real
 *  `instance.checkShaderRequirements` round trip gates the modal — not
 *  reused directly since that constant is not exported and this file needs
 *  a hard wait here, not a race). */
const SHADER_WIZARD_OPEN_TIMEOUT = 5_000

/**
 * Proves each non-mod addon type installs into its **own** folder and is
 * typed correctly on the way back out — one test per `ADDON_FIXTURES` entry
 * (`helpers/addonFixtures.ts`): CurseForge and Modrinth for resourcepacks and
 * shaders, CurseForge-only for datapacks and worlds. That file's own doc
 * comment records why the latter two are a real platform asymmetry (verified
 * live) rather than a gap in this suite's coverage.
 *
 * The three assertions per combination are deliberately different in kind.
 * The folder check catches routing regressions — two of `AddonType`'s five
 * `get_folder_path` mappings do not match their type name (`shaders` ->
 * `shaderpacks/`, `worlds` -> `saves/`). The `addon_type` read-back catches
 * the app mislabelling what it installed. And the empty-`mods/` check catches
 * the failure mode neither of the others can see: each platform's
 * `ResourceInstaller::get_install_path` has its own unrecognised-type
 * fallback arm that writes straight into `mods/` regardless of what the
 * file actually is — `_ => instance_path.get_mods_path()` at
 * `managers/instance/installer/mod.rs:655` (`CurseforgeModInstaller`) and
 * `:973` (`ModrinthModInstaller`) — which would land a shader's file in
 * `mods/` at install time with nothing failing anywhere. (A same-shaped
 * fallback, `AddonType::from_db_string(..).unwrap_or(AddonType::Mods)`
 * in `managers/instance/mods.rs`, exists read-side too, but that one only
 * changes how an already-written file's type is reported back through
 * `list_mods` — it has no say in where installing a file actually puts it,
 * so it is not what this particular assertion is guarding.)
 *
 * All six installs share `installedInstance`'s one warm Fabric instance
 * (worker-scoped, serial mode here) rather than one instance per fixture —
 * the before/after directory diff each test takes isolates what *that* test
 * added regardless of what earlier fixtures already left behind in the same
 * folder (both `resourcepacks` fixtures share `resourcepacks/`, both
 * `shaders` fixtures share `shaderpacks/`).
 */
test.describe("addon placement", () => {
  test.describe.configure({ mode: "serial" })

  // `installedInstance` is worker-scoped and shared with every mod spec this
  // worker runs; mirrors `modInstall.spec.ts`'s identical hook so a failure
  // here is diagnosable the same way and the library is left interactive
  // regardless of where a failed test left `page`.
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

  for (const fixture of ADDON_FIXTURES) {
    const title = `installs a ${fixture.platform} ${fixture.addonType} into its own folder`

    test(title, async ({ installedInstance }) => {
      const { page, harness, instanceName } = installedInstance
      const { shortpath } = readInstanceByName(
        harness.runtimePath,
        instanceName
      )

      // `<runtimePath>/instances/<shortpath>` is the instance's root; every
      // addon folder `AddonType::get_folder_path` names lives one level
      // deeper, under its `instance/` data subdirectory — confirmed directly
      // off `InstancePath::get_data_path` (`crates/carbon_rt_path/src/lib.rs`)
      // and matching every other spec in this suite that resolves an
      // instance's on-disk tree (`fixtures/installedInstance.ts`'s
      // `resolveModsDir`, `helpers/packinfo.ts`, every `modpack*.spec.ts`
      // file's own `path.join(root, "instance")`).
      const instanceRoot = path.join(
        harness.runtimePath,
        "instances",
        shortpath
      )
      const instanceDataPath = path.join(instanceRoot, "instance")

      const targetDir = addonDir(instanceDataPath, fixture.addonType)
      const before = fs.existsSync(targetDir) ? fs.readdirSync(targetDir) : []
      const modsDir = path.join(instanceDataPath, "mods")
      const modsBefore = fs.existsSync(modsDir) ? fs.readdirSync(modsDir) : []

      await openInstanceAddons(page, instanceName)
      await searchForMod(page, {
        query: fixture.query,
        platform: fixture.platform,
        searchType: fixture.searchType
      })
      await openAddonPage(page, fixture.projectId)

      // Shader-cancel leg, run once (curseforge only — either shader
      // fixture works equally well here, see the constant's own comment; the
      // point is a loaderless instance, not a platform). `installedInstance`
      // is a bare Fabric instance with neither Iris nor Oculus installed,
      // and stays that way through both shader fixtures below: the ordinary
      // path this loop otherwise takes for a shader install
      // (`installModIntoInstance`'s "Continue anyway" branch) installs only
      // the shaderpack file, deliberately never a loader mod — see that
      // function's own doc comment. So this always finds the wizard, whether
      // it runs before or after the other shader fixture's normal install.
      //
      // Drives the wizard directly rather than through `installModIntoInstance`
      // (which only ever clicks "Continue anyway") to reach the *Cancel*
      // button instead: the wizard's `Intro` step
      // (`ShaderLoaderSetup/index.tsx`) calls `complete(null)` on cancel,
      // which fires the `notifications:_trn_shader_install_cancelled` toast
      // (via `onCleanup`'s guarded `complete`) and never calls either
      // install mutation.
      if (
        fixture.platform === "curseforge" &&
        fixture.addonType === "shaders"
      ) {
        const installButton = page.locator(
          byTestId(TEST_IDS.addonInstallButton)
        )
        await installButton.click()

        // Proves the wizard actually opened before looking for its Cancel
        // control — `shaderLoaderContinueAnyway` is the one sibling button
        // in the same `Intro` step that already carries a `data-testid`
        // (installModIntoInstance's own wizard race uses it for the same
        // "did this really open" signal).
        await expect(
          page.locator(byTestId(TEST_IDS.shaderLoaderContinueAnyway)),
          {
            message:
              "shader-cancel leg: the ShaderLoaderSetup wizard never opened " +
              `after clicking install for "${fixture.query}" on a loaderless ` +
              "instance — checkShaderRequirements may have reported LoaderPresent"
          }
        ).toBeVisible({ timeout: SHADER_WIZARD_OPEN_TIMEOUT })

        // Cancel carries no `data-testid` (`ShaderLoaderSetup/index.tsx`'s
        // `Intro` step anchors only "Continue anyway" — this task's binding
        // constraints leave `mainWindow/src` untouched, so a new anchor isn't
        // an option here). Scoped to `#overlay`, the portal every modal in
        // this app mounts into (`ModalsManager/index.tsx`) and the same
        // element `ensureLibraryInteractive` already reads to prove no modal
        // is open — the identical "role-selector, no testid" precedent
        // `installModpackVersion` already uses for the Versions tab.
        await page
          .locator("#overlay")
          .getByRole("button", { name: "Cancel" })
          .click()

        // Toast markup from `somoto` (`@gd/ui`'s `Sonner`): each toast is an
        // `[data-somoto-toast]` `<li role="status">` holding the message in a
        // nested `[data-title]`. Filtered by text rather than matched
        // structurally so a second, unrelated toast still on screen can never
        // make this pass for the wrong reason.
        await expect(
          page
            .locator("[data-somoto-toast]")
            .filter({ hasText: "Shader install cancelled" }),
          {
            message:
              'shader-cancel leg: no toast containing "Shader install ' +
              'cancelled" appeared after clicking Cancel on the wizard'
          }
        ).toBeVisible({ timeout: 5_000 })

        await expect(installButton, {
          message:
            'shader-cancel leg: the install button reported "Downloaded" ' +
            "after the wizard was cancelled — cancelling must never install " +
            "anything"
        }).not.toHaveText(/downloaded/i)
      }

      // `installModIntoInstance`'s own text-based completion check cannot
      // see a world finishing (see its doc comment for the confirmed
      // `ModDownloadButton` bug behind that) — poll the real target
      // directory instead, the same one this test's own placement assertion
      // below reads, so a world install's completion is read off disk
      // rather than a button state that never reports it either way.
      //
      // A `.zip` here is never a world's final form the way it would be for
      // a resourcepack/shader/datapack (whose real installed file *is* a
      // `.zip`, which is exactly why the exclusion below is scoped to the
      // worlds branch only, not applied to every fixture):
      // `CurseforgeModInstaller::post_process` (`installer/mod.rs`)
      // decompresses the downloaded archive into this same directory and
      // unlinks it *afterwards*.
      //
      // The condition is therefore the settled end state — at least one new
      // entry that is neither the in-flight `.__gdl_part~` marker nor a
      // `.zip`, **and** no new `.zip` left over — rather than the first sign
      // of progress toward it. The extracted directory and the archive it
      // came from coexist in `targetDir` for the whole extraction, so a
      // condition satisfied the moment the directory appears leaves whatever
      // reads the directory next racing an unlink whose window widens with
      // machine load.
      const waitForCompletion =
        fixture.addonType === "worlds"
          ? async () => {
              await expect
                .poll(
                  () => {
                    if (!fs.existsSync(targetDir)) return false
                    const added = fs
                      .readdirSync(targetDir)
                      .filter((f) => !before.includes(f) && !isPartFile(f))
                    return (
                      added.some((f) => !f.toLowerCase().endsWith(".zip")) &&
                      !added.some((f) => f.toLowerCase().endsWith(".zip"))
                    )
                  },
                  {
                    timeout: INSTALL_TIMEOUT,
                    message:
                      `installModIntoInstance: ${targetDir} never settled on ` +
                      `a fully-processed entry for the ${fixture.platform} ` +
                      "world install — a leftover `.zip` beside the extracted " +
                      "directory means post_process never deleted it"
                  }
                )
                .toBe(true)
            }
          : undefined

      await installModIntoInstance(page, {
        instanceName,
        waitForCompletion,
        // World only (see `installModIntoInstance`'s own doc comment): a
        // world's completion signal above is a disk poll that says nothing
        // about the button's own visible state, which is exactly the gap
        // the `pendingInstall` fix closed — assert it actually shows.
        assertLoadingVisible: fixture.addonType === "worlds"
      })

      const after = fs.existsSync(targetDir) ? fs.readdirSync(targetDir) : []
      const added = after.filter((f) => !before.includes(f) && !isPartFile(f))

      expect(added, {
        message:
          `${fixture.addonType} from ${fixture.platform} produced no new entry ` +
          `in ${targetDir} — check AddonType::get_folder_path`
      }).not.toHaveLength(0)

      const modsAfter = fs.existsSync(modsDir) ? fs.readdirSync(modsDir) : []
      expect(modsAfter, {
        message:
          `installing a ${fixture.addonType} added a file to mods/ — an ` +
          "addon type fell through to AddonType::Mods"
      }).toEqual(modsBefore)

      let installed: InstalledMod | undefined
      await expect
        .poll(
          async () => {
            const listed = await openInstanceAddons(page, instanceName)
            installed = listed.find((m) => added.includes(m.filename))
            return installed !== undefined
          },
          {
            timeout: RECONCILIATION_WAIT,
            message: `the app's own list never reported ${added.join(", ")}`
          }
        )
        .toBe(true)
      expect(installed!.addonType).toBe(fixture.addonType)
    })
  }
})
