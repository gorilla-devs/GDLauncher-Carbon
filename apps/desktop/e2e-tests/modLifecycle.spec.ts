import type { Page } from "@playwright/test"
import { test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byModRow, byTestId, TEST_IDS } from "./helpers/selectors.js"
import { ensureLibraryInteractive } from "./helpers/instances.js"
import {
  cleanupInstalledMod,
  deleteModViaUi,
  installAddonVersion,
  installModIntoInstance,
  openAddonPage,
  openAddonVersions,
  openInstanceAddons,
  pickOlderVersion,
  searchForMod,
  toggleModEnabled,
  waitForModFilenameChange,
  waitForModUpdateAvailable,
  type InstalledMod
} from "./helpers/mods.js"
import {
  listModFiles,
  verifyModEnabled,
  verifyModInstalled
} from "./helpers/modVerify.js"

/**
 * The installed-mod lifecycle: disable, enable, delete, update — each
 * verified on disk (`modVerify.ts`), never by the UI's own
 * rendering of the same fact. See mods.rs:335-337/348/358 for the disk
 * representation a disabled mod actually has (the file renamed in place
 * with a `.disabled` suffix) and mods.rs:361-367 for the second, non-atomic
 * representation this file deliberately does not trust: `ModFileCache.enabled`
 * in the database, written *after* the rename completes. A crash in that
 * window leaves disk and DB disagreeing permanently — a known, unfixed
 * product bug, not something these tests work around. They read disk only.
 *
 * All four tests drive **Fabric API on Modrinth** (`P7dR8mSH`), the same
 * project `modInstall.spec.ts` already exercises for its Modrinth case.
 * Disable/enable/delete are deliberately not platform-specific — `enable_mod`
 * /`delete_mod` (`managers/instance/mods.rs`) operate purely on the cached
 * DB row and its on-disk filename, with no branch on CurseForge vs. Modrinth
 * — so one well-understood project is enough to exercise the mechanism
 * without paying for a second platform's search+install round trip in every
 * test. The update test specifically needs Modrinth: reaching a
 * deliberately-older, still-installable file requires the addon page's
 * Versions tab (see `openAddonVersions`'s doc comment), and that path was
 * only verified live against Modrinth — CurseForge's
 * equivalent `modplatforms.curseforge.getModFiles` shape was read from
 * source but never driven end-to-end here.
 */
test.describe("mod lifecycle", () => {
  // Same reasoning as modInstall.spec.ts's identical hook: `installedInstance`
  // is worker-scoped, so `afterEach` is the one hook that still gets both its
  // value and a real per-test `TestInfo`.
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

  const FABRIC_API_PROJECT_ID = "P7dR8mSH"
  const matchesFabricApi = (mod: InstalledMod) =>
    mod.modrinthProjectId === FABRIC_API_PROJECT_ID

  /**
   * Installs the latest Fabric API build into the instance via the addon
   * page's main install button (`INSTALL_LATEST_MOD` — no specific version
   * needed for disable/enable/delete, unlike the update test), and returns
   * the app's own freshly-read record for it.
   */
  async function installFabricApi(
    page: Page,
    instanceName: string
  ): Promise<InstalledMod> {
    await openInstanceAddons(page, instanceName)
    await searchForMod(page, { platform: "modrinth", query: "fabric api" })
    await openAddonPage(page, FABRIC_API_PROJECT_ID)
    await installModIntoInstance(page, { instanceName })

    const mods = await openInstanceAddons(page, instanceName)
    const installed = mods.find(matchesFabricApi)
    if (!installed) {
      throw new Error(
        "installFabricApi: instance.getInstanceMods has no entry matching " +
          `project ${FABRIC_API_PROJECT_ID} on modrinth after install ` +
          `(got ${JSON.stringify(mods)})`
      )
    }
    return installed
  }

  /**
   * Thin, test-named wrapper over `helpers/mods.ts`'s shared
   * `cleanupInstalledMod` (also used by `modInstall.spec.ts`) — kept as a
   * local function rather than inlined at each call site purely so the four
   * call sites below stay one line each and every cleanup failure here is
   * labeled "cleanupFabricApi" the same way it always has been.
   */
  async function cleanupFabricApi(
    page: Page,
    instanceName: string,
    modsDir: string
  ): Promise<void> {
    await cleanupInstalledMod(
      page,
      instanceName,
      modsDir,
      matchesFabricApi,
      "cleanupFabricApi"
    )
  }

  test("disables an installed mod", async ({ installedInstance }) => {
    const { page, instanceName, modsDir } = installedInstance

    // See modInstall.spec.ts's identical `bodyFailed` doc comment: a `throw`
    // inside `finally` discards whatever the try-block was throwing, so
    // cleanup failure must only re-throw over a passing body.
    let bodyFailed = false
    try {
      const installed = await installFabricApi(page, instanceName)

      await toggleModEnabled(page, installed.filename, false)

      // The real disk representation, not the switch's own state —
      // see the module doc comment on why the DB's `enabled` column is
      // deliberately not trusted here.
      const result = await verifyModEnabled(modsDir, installed.filename, false)
      if (!result.ok) {
        throw new Error(
          `"disables an installed mod": disk verification failed:\n` +
            result.problems.map((p) => `  - ${p}`).join("\n")
        )
      }
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      try {
        await cleanupFabricApi(page, instanceName, modsDir)
      } catch (cleanupError) {
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupError
        }
        console.error(
          'cleanup for "disables an installed mod" also failed:',
          cleanupError
        )
      }
    }
  })

  test("enables a disabled mod", async ({ installedInstance }) => {
    const { page, instanceName, modsDir } = installedInstance

    let bodyFailed = false
    try {
      const installed = await installFabricApi(page, instanceName)

      // Setup, not the assertion under test: a mod has to already be
      // disabled before "enable" is a meaningful action. Verified on disk
      // before proceeding so a broken disable wouldn't silently make the
      // enable assertion below vacuously true.
      await toggleModEnabled(page, installed.filename, false)
      const setupResult = await verifyModEnabled(
        modsDir,
        installed.filename,
        false
      )
      if (!setupResult.ok) {
        throw new Error(
          `"enables a disabled mod": setup (disabling) failed verification:\n` +
            setupResult.problems.map((p) => `  - ${p}`).join("\n")
        )
      }

      await toggleModEnabled(page, installed.filename, true)

      const result = await verifyModEnabled(modsDir, installed.filename, true)
      if (!result.ok) {
        throw new Error(
          `"enables a disabled mod": disk verification failed:\n` +
            result.problems.map((p) => `  - ${p}`).join("\n")
        )
      }
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      try {
        await cleanupFabricApi(page, instanceName, modsDir)
      } catch (cleanupError) {
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupError
        }
        console.error(
          'cleanup for "enables a disabled mod" also failed:',
          cleanupError
        )
      }
    }
  })

  test("deletes an installed mod", async ({ installedInstance }) => {
    const { page, instanceName, modsDir } = installedInstance

    let bodyFailed = false
    try {
      const installed = await installFabricApi(page, instanceName)

      await deleteModViaUi(page, installed.filename)

      // `verifyModInstalled` is deliberately not reused here inverted: its
      // "ok" only ever means present-and-correct, so a caller checking
      // `!result.ok` for "deleted" cannot tell a genuine absence apart from
      // some other unrelated verification failure (wrong size on a
      // different file at the same name, a stray directory, ...). Listing
      // the directory and asserting the exact filename is gone is the
      // direct, unambiguous check — the same one `modInstall.spec.ts`'s own
      // cleanup already uses for the identical reason.
      const remaining = await listModFiles(modsDir)
      if (remaining.includes(installed.filename)) {
        throw new Error(
          `"deletes an installed mod": "${installed.filename}" is still ` +
            `present in ${modsDir} after deleteModViaUi (found: ` +
            `${JSON.stringify(remaining)})`
        )
      }
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      // The delete under test already returns the instance to a clean
      // state on its own success — this only catches a body that failed
      // before or during the delete, leaving something to remove.
      try {
        await cleanupFabricApi(page, instanceName, modsDir)
      } catch (cleanupError) {
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupError
        }
        console.error(
          'cleanup for "deletes an installed mod" also failed:',
          cleanupError
        )
      }
    }
  })

  /**
   * The awkward one. A real update needs a file
   * genuinely older than what the platform currently offers, so this
   * installs one specific, deliberately-not-newest build off the addon
   * page's Versions tab (`INSTALL_MOD`, a real `version_id` — never
   * `INSTALL_LATEST_MOD`) rather than asserting anything weaker. See
   * `openAddonVersions`'s and `pickOlderVersion`'s doc comments for exactly
   * how "deliberately older" is chosen and why it is safe from the two
   * timing races this took several live runs to pin down:
   * the Versions tab's own unfiltered-then-scoped double fetch, and the
   * update button only becoming clickable once the metadata cache has
   * actually run for the freshly installed file.
   */
  test("updates an installed mod to a newer version", async ({
    installedInstance
  }) => {
    const { page, instanceName, modsDir } = installedInstance

    let bodyFailed = false
    try {
      await openInstanceAddons(page, instanceName)
      await searchForMod(page, { platform: "modrinth", query: "fabric api" })
      await openAddonPage(page, FABRIC_API_PROJECT_ID)

      const versions = await openAddonVersions(page)
      const older = pickOlderVersion(versions)
      await installAddonVersion(page, older)

      const mods = await openInstanceAddons(page, instanceName)
      const installed = mods.find(matchesFabricApi)
      if (!installed) {
        throw new Error(
          '"updates an installed mod to a newer version": ' +
            "instance.getInstanceMods has no entry matching project " +
            `${FABRIC_API_PROJECT_ID} on modrinth after installing the ` +
            `deliberately-older build (fileId ${older.fileId})`
        )
      }

      // The older build is genuinely on disk before ever touching update —
      // otherwise a later "filename changed" observation could not be
      // trusted to mean what it claims.
      const installedResult = await verifyModInstalled(modsDir, {
        filename: installed.filename,
        expectedSize: installed.fileSize,
        expectedSha1: installed.sha1 ?? undefined
      })
      if (!installedResult.ok) {
        throw new Error(
          '"updates an installed mod to a newer version": older-build disk ' +
            "verification failed:\n" +
            installedResult.problems.map((p) => `  - ${p}`).join("\n")
        )
      }

      // Waits for the update to become available, not asserts it: this
      // helper cannot return without `hasUpdate: true` on the matched mod —
      // it either finds that or throws its own named timeout — so an
      // `expect` on its result here would check a condition the call above
      // it already guarantees, proving nothing. The genuinely-failable
      // assertions are the
      // disk checks below, after the update has actually run.
      await waitForModUpdateAvailable(page, instanceName, matchesFabricApi)

      const row = page.locator(byModRow(installed.filename))
      await row.locator(byTestId(TEST_IDS.modRowUpdate)).click()

      // Same reasoning as the wait above: `waitForModFilenameChange` cannot
      // return without the filename having changed, so there is nothing to
      // usefully assert about `updated.filename` itself here — the real
      // observation this whole test exists for is that the *disk* — not
      // just the app's own record — reflects a genuinely different,
      // verifiable file, which the two checks below establish.
      const updated = await waitForModFilenameChange(page, instanceName, {
        oldFilename: installed.filename,
        matches: matchesFabricApi
      })

      const updatedResult = await verifyModInstalled(modsDir, {
        filename: updated.filename,
        expectedSize: updated.fileSize,
        expectedSha1: updated.sha1 ?? undefined
      })
      if (!updatedResult.ok) {
        throw new Error(
          '"updates an installed mod to a newer version": updated-build ' +
            "disk verification failed:\n" +
            updatedResult.problems.map((p) => `  - ${p}`).join("\n")
        )
      }

      // The old build's file is genuinely gone, not left behind alongside
      // the new one — `update_mod`'s installer deletes the replaced mod
      // after the new file lands (`replaces_mod_id`,
      // `managers/instance/installer/mod.rs`), so two jars coexisting here
      // would itself be a real regression, not a harmless leftover.
      const remaining = await listModFiles(modsDir)
      if (remaining.includes(installed.filename)) {
        throw new Error(
          '"updates an installed mod to a newer version": the pre-update ' +
            `file "${installed.filename}" is still present in ${modsDir} ` +
            `after updating to "${updated.filename}" (found: ` +
            `${JSON.stringify(remaining)})`
        )
      }
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      try {
        await cleanupFabricApi(page, instanceName, modsDir)
      } catch (cleanupError) {
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupError
        }
        console.error(
          'cleanup for "updates an installed mod to a newer version" also failed:',
          cleanupError
        )
      }
    }
  })
})
