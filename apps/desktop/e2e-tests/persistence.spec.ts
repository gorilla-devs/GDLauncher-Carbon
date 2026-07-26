/**
 * Proves that what the launcher writes survives a real process restart —
 * not a page reload, not a re-mounted component tree, but the Rust core
 * actually exiting and a fresh one reading the same runtime path back off
 * disk. Nothing else in this suite ever does that: every other spec either
 * never closes the app, or closes it only in teardown. A bug that wrote
 * nothing to disk, or wrote it somewhere the next boot can't find, or wrote
 * it in a form the reconciliation-on-startup code path corrupts, passes
 * every other test in this suite. This is the one spec that would catch it.
 *
 * Four things, each written through a different code path, each asserted
 * through two independent channels after one relaunch:
 *
 * 1. An instance (name + Minecraft version) — UI: `instance.getInstanceDetails`
 *    read fresh after the relaunch. Disk: the `Instance` DB row **and** the
 *    instance's own on-disk `instance.json` (`helpers/instanceConfig.ts`)
 *    — two disk-side checks, deliberately, because this plan's whole point is
 *    SQLite survival specifically, not just "some file exists somewhere".
 * 2. An app setting (`reducedMotion`, Settings > General's "Potato mode") —
 *    UI: the switch's own `checked` state. Disk: the `AppConfiguration.reducedMotion`
 *    column (`helpers/versionCache.ts`'s `readAppConfiguration`).
 * 3. An installed mod, left enabled — asserted, but read honestly: nothing
 *    about it, filename/size/enabled/platform-association alike, is unique
 *    proof that any specific SQLite row survived. The launcher's own
 *    boot-time reconciliation (a local disk scan plus a background
 *    Modrinth-fingerprint lookup) reconstructs all of it from the jar file
 *    alone, confirmed by sabotage rather than assumed — see the assertion's
 *    own doc comment and task-2-report.md's "Fix round 1". Kept anyway as a
 *    regression check on that reconciliation pipeline itself, not dropped as
 *    dead weight.
 * 4. A disabled mod — same two channels as (3), `enabled: false`, but
 *    genuinely persistence-adjacent in a way (3) is not: enabled state lives
 *    in two places written non-atomically (the `.disabled` rename, then
 *    `ModFileCache.enabled` — see `README.md`'s "Known product bug" section),
 *    and a reconciliation scan runs on every instance at startup. A restart
 *    is exactly the code path that exercises that scan, so both channels are
 *    checked and compared, not just one — and unlike (3)'s platform
 *    association, there is no network fallback for *this* fact: the disk
 *    state (the `.disabled` suffix) is not a proxy for something else, it
 *    *is* the ground truth being tested.
 *
 * **Its own runtime path, not the shared worker one.** Every other spec in
 * this suite reuses `authenticatedApp` (worker-scoped: one login, one
 * runtime path, shared by every test the worker runs) specifically so it
 * only pays for login/enrollment once. This spec cannot do that: closing and
 * relaunching the app mid-test would kill the Rust core out from under every
 * other test still running against that same worker-scoped app. So this file
 * does not import the shared `test`/fixtures from `fixtures/index.ts` at
 * all — it drives `startHarness`/`launchApp`/`relaunchApp`/`stopHarness`
 * directly, the same building blocks `fixtures/index.ts` itself composes
 * into `freshApp`, but kept outside that fixture's own lifecycle so this
 * test can swap in the relaunched `ElectronApplication` before its own
 * `finally` closes anything. (`freshApp` itself would very nearly work — it
 * is already per-test, not worker-scoped — except its `finally` block closes
 * the *original* `app` object it captured at setup, which `relaunchApp`
 * already closed internally; closing it a second time from outside this
 * file's control isn't a risk worth taking against Playwright's own
 * ElectronApplication lifecycle.)
 *
 * The cost of that isolation, measured on this branch (see task-2-report.md
 * for the full breakdown): a full login+enrollment round trip and a **cold**
 * instance install — this runtime path shares no warm `assets/`/`libraries/`/
 * `managed_javas/` substrate with any other spec, unlike `installedInstance`'s
 * worker-scoped reuse, so the one instance this test creates pays the same
 * "first install in a fresh runtime path" cost `instanceInstall.spec.ts`'s
 * cold entry does. Accepted deliberately: the alternative (reusing a
 * worker-scoped app) would make this test corrupt every other test sharing
 * that worker.
 */

import path from "node:path"
import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  isCoreModulePresent,
  launchApp,
  relaunchApp,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import { resolveModsDir } from "./fixtures/installedInstance.js"
import { byInstanceName, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  createInstanceViaUi,
  waitForInstallComplete
} from "./helpers/instances.js"
import {
  readAppConfiguration,
  readInstanceByName
} from "./helpers/versionCache.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import {
  installModIntoInstance,
  openAddonPage,
  openInstanceAddons,
  searchForMod,
  toggleModEnabled,
  type InstalledMod
} from "./helpers/mods.js"
import { verifyModEnabled, verifyModInstalled } from "./helpers/modVerify.js"

const INSTANCE_NAME = "gdl-e2e-persistence"
const MC_VERSION = "1.20.1"
const LOADER = "fabric"

/** Modrinth project id. The load-bearing dependency of the Fabric mod
 *  ecosystem — same durability reasoning, and the same project id, as
 *  `modInstall.spec.ts`/`modLifecycle.spec.ts`'s Modrinth case. Installed
 *  here and left enabled — this is the "installed mod" case. */
const FABRIC_API_PROJECT_ID = "P7dR8mSH"

/** Modrinth project id. A second, independently popular Fabric/Quilt mod
 *  (confirmed live against api.modrinth.com for this task: `fabric`/`quilt`
 *  loaders, `1.20.1` in its game_versions) — deliberately not Fabric API
 *  again, since this test needs two distinct files present at once (one
 *  enabled, one disabled) after the restart. Installed and then disabled —
 *  this is the "disabled mod" case. Kept on Modrinth rather than reaching for
 *  CurseForge for the second mod too: README.md documents a live CurseForge
 *  search flake this task did not need to also risk. */
const MOD_MENU_PROJECT_ID = "mOgUt4GM"

async function goToLibrary(page: Page): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}

/**
 * Waits for a fresh `instance.getInstanceDetails` response to land, having
 * registered the listener before `action` runs so a response that fires
 * during `action` is never missed.
 *
 * Exists to close a real race this task found live, not a defensive
 * guess: `Addons/index.tsx`'s `defaultSearchType()` — what "Add Addons"
 * uses to pick the search page's content-type filter — reads
 * `hasModloaders()`, which reads `instance.getInstanceDetails`'s own
 * `modloaders` field. On a warm worker (every other spec in this suite)
 * that query is long since cached by the time "Add Addons" is ever
 * clicked. On this spec's cold, single-use runtime path it is not: clicking
 * straight through from `openInstanceAddons` (which only waits on the
 * sibling `instance.getInstanceMods` query, never this one) raced
 * `defaultSearchType()` against the still-in-flight `getInstanceDetails`
 * fetch, lost, and sent the search to the Shaders tab instead of Mods —
 * observed directly (see task-2-report.md), not theorized.
 */
async function waitForInstanceDetailsResponse(
  page: Page,
  action: () => Promise<void>
): Promise<void> {
  let settled = false
  const onResponse = (r: import("@playwright/test").Response) => {
    if (r.url().includes("instance.getInstanceDetails")) settled = true
  }
  page.on("response", onResponse)
  try {
    await action()
    await expect.poll(() => settled, { timeout: 15_000 }).toBe(true)
  } finally {
    page.off("response", onResponse)
  }
}

/**
 * Installs `projectId` off Modrinth into `instanceName` via the real UI (the
 * same click path `modLifecycle.spec.ts`'s `installFabricApi` uses), and
 * returns the app's own freshly-read record for it. Assumes it is called
 * from the library (calls `openInstanceAddons`, which navigates there
 * itself), same precondition every `helpers/mods.ts` UI driver already
 * carries.
 */
async function installModrinthMod(
  page: Page,
  instanceName: string,
  opts: { projectId: string; query: string }
): Promise<InstalledMod> {
  await waitForInstanceDetailsResponse(page, () =>
    openInstanceAddons(page, instanceName).then(() => {})
  )
  await searchForMod(page, { platform: "modrinth", query: opts.query })
  await openAddonPage(page, opts.projectId)
  await installModIntoInstance(page, { instanceName })

  const mods = await openInstanceAddons(page, instanceName)
  const installed = mods.find((m) => m.modrinthProjectId === opts.projectId)
  if (!installed) {
    throw new Error(
      `installModrinthMod: instance.getInstanceMods for "${instanceName}" ` +
        `has no entry matching Modrinth project ${opts.projectId} after ` +
        `install (got ${JSON.stringify(mods)})`
    )
  }
  return installed
}

/**
 * Clicks `instanceName`'s tile from the library, capturing the
 * `instance.getInstanceDetails` response the navigation itself triggers
 * (`Library/Instance/index.tsx` mounts that query as soon as it knows the
 * instance id) — this is the "UI" channel for the instance's name/version,
 * the same "trust the app's own live response, not the DOM" precedent
 * `helpers/mods.ts`'s `openInstanceAddons` already uses for mods.
 */
async function readInstanceDetailsViaUi(
  page: Page,
  instanceName: string
): Promise<{ name: string; version: string | null }> {
  const responsePromise = page.waitForResponse(
    (r) => r.url().includes("instance.getInstanceDetails"),
    { timeout: 15_000 }
  )
  await page.click(byInstanceName(instanceName))
  const response = await responsePromise

  const body = (await response.json()) as {
    result?: {
      type?: string
      data?: { name?: string; version?: string | null }
    }
  }
  if (body.result?.type === "error") {
    throw new Error(
      `readInstanceDetailsViaUi: instance.getInstanceDetails returned an ` +
        `rspc error for "${instanceName}": ${JSON.stringify(body.result.data)}`
    )
  }
  const data = body.result?.data
  if (!data || typeof data.name !== "string") {
    throw new Error(
      `readInstanceDetailsViaUi: instance.getInstanceDetails response for ` +
        `"${instanceName}" has no usable result.data (got ${JSON.stringify(body)})`
    )
  }
  return { name: data.name, version: data.version ?? null }
}

test.describe("state survives a restart", () => {
  // Playwright's test callback always takes the fixtures object first; this
  // test uses none of the custom fixtures (see the module doc comment for
  // why) — only the plain `testInfo` second argument.
  // eslint-disable-next-line no-empty-pattern
  test("an instance, a setting, an installed mod, and a disabled mod all survive a real app restart", async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()

    const harness = await startHarness()
    const launchOpts: LaunchOptions = {
      runtimePath: harness.runtimePath,
      baseApi: `${harness.mock.url}/gdl`,
      e2eAuthBase: harness.mock.url,
      e2eEntitlementKey: harness.entitlementKeyPath
    }

    let current: {
      app: ElectronApplication
      page: Page
      pageErrors: Error[]
    } | null = null

    try {
      current = await launchApp(launchOpts)
      await completeLogin(current.page, harness)
      await dismissStartupModals(current.page)

      // --- Setup: write the four things, pre-restart --------------------

      await test.step("create the instance", async () => {
        await createInstanceViaUi(current!.page, {
          name: INSTANCE_NAME,
          version: MC_VERSION,
          loader: LOADER
        })
        await waitForInstallComplete(current!.page, INSTANCE_NAME)
      })

      await test.step("change a setting", async () => {
        const page = current!.page
        await page.click(byTestId(TEST_IDS.navbarSettings))

        // Click target vs. assertion target are deliberately two different
        // locators: `Switch` (`@gd/ui`) spreads its props onto the native
        // `<input type="checkbox">` it renders, but that input is zero-size
        // (`w-0 h-0`) and cannot itself receive a pointer click — Playwright's
        // actionability wait on it hangs rather than erroring. The wrapping
        // div (sized to the visible `<label>`) is what's clickable; the input
        // underneath is what `toBeChecked()` reads. Same split
        // `helpers/mods.ts`'s `toggleModEnabled` already uses for the
        // structurally identical `modRowToggle` anchor.
        const toggleWrapper = page.locator(
          byTestId(TEST_IDS.settingsReducedMotionToggle)
        )
        const toggleInput = toggleWrapper.locator('input[type="checkbox"]')
        // Sanity: proves the assertion after restart observes a real change,
        // not a value that happened to already match the default (schema
        // default for reducedMotion is `false`).
        await expect(toggleInput, {
          message:
            "setup: reducedMotion is already checked before this test " +
            "touched it — the post-restart assertion could not tell a real " +
            "persisted change from the untouched default"
        }).not.toBeChecked()

        const responsePromise = page.waitForResponse(
          (r) => r.url().includes("settings.setSettings"),
          { timeout: 15_000 }
        )
        await toggleWrapper.click()
        const response = await responsePromise
        const body = (await response.json()) as {
          result?: { type?: string; data?: unknown }
        }
        if (body.result?.type === "error") {
          throw new Error(
            `setup: settings.setSettings returned an rspc error: ` +
              JSON.stringify(body.result.data)
          )
        }
        await expect(toggleInput).toBeChecked()

        await goToLibrary(page)
      })

      let installedMod: InstalledMod | undefined
      await test.step("install a mod, leave it enabled", async () => {
        installedMod = await installModrinthMod(current!.page, INSTANCE_NAME, {
          projectId: FABRIC_API_PROJECT_ID,
          query: "fabric api"
        })

        const modsDir = resolveModsDir(harness.runtimePath, INSTANCE_NAME)
        const result = await verifyModInstalled(modsDir, {
          filename: installedMod.filename,
          expectedSize: installedMod.fileSize,
          expectedSha1: installedMod.sha1 ?? undefined
        })
        if (!result.ok) {
          throw new Error(
            `setup: installed mod failed disk verification before the ` +
              `restart even happened:\n` +
              result.problems.map((p) => `  - ${p}`).join("\n")
          )
        }
      })

      let disabledMod: InstalledMod | undefined
      await test.step("install a second mod and disable it", async () => {
        const installed = await installModrinthMod(
          current!.page,
          INSTANCE_NAME,
          { projectId: MOD_MENU_PROJECT_ID, query: "mod menu" }
        )
        disabledMod = installed

        await toggleModEnabled(current!.page, installed.filename, false)

        const modsDir = resolveModsDir(harness.runtimePath, INSTANCE_NAME)
        const result = await verifyModEnabled(
          modsDir,
          installed.filename,
          false
        )
        if (!result.ok) {
          throw new Error(
            `setup: disabling the second mod did not reach disk before the ` +
              `restart even happened:\n` +
              result.problems.map((p) => `  - ${p}`).join("\n")
          )
        }
      })

      expect(current.pageErrors, {
        message:
          "an uncaught renderer exception happened during setup, before " +
          "the restart this test exists to check was even reached"
      }).toEqual([])

      // --- The restart ----------------------------------------------------

      current = await test.step("restart the app", async () => {
        return relaunchApp(current!, launchOpts)
      })

      await dismissStartupModals(current.page)

      // --- Assertions, all read fresh after the relaunch ------------------

      await test.step("assertion 1: the instance", async () => {
        const ui = await readInstanceDetailsViaUi(current!.page, INSTANCE_NAME)
        expect(ui.name, "instance name via instance.getInstanceDetails").toBe(
          INSTANCE_NAME
        )
        expect(
          ui.version,
          "instance Minecraft version via instance.getInstanceDetails"
        ).toBe(MC_VERSION)

        const dbRow = readInstanceByName(harness.runtimePath, INSTANCE_NAME)
        const instanceRoot = path.join(
          harness.runtimePath,
          "instances",
          dbRow.shortpath
        )
        const onDisk = await readInstanceConfig(instanceRoot)
        expect(onDisk.name, "instance name in instance/config on disk").toBe(
          INSTANCE_NAME
        )
        expect(
          onDisk.mcVersion,
          "instance Minecraft version in instance/config on disk"
        ).toBe(MC_VERSION)
      })

      await test.step("assertion 2: the setting", async () => {
        const page = current!.page
        await page.click(byTestId(TEST_IDS.navbarSettings))
        const toggleInput = page
          .locator(byTestId(TEST_IDS.settingsReducedMotionToggle))
          .locator('input[type="checkbox"]')
        await expect(
          toggleInput,
          "reducedMotion switch after restart"
        ).toBeChecked()

        const onDisk = readAppConfiguration(harness.runtimePath)
        expect(
          onDisk.reducedMotion,
          "AppConfiguration.reducedMotion column after restart"
        ).toBe(true)

        await goToLibrary(page)
      })

      const modsDir = resolveModsDir(harness.runtimePath, INSTANCE_NAME)

      // Read this step's intent honestly before trusting a green result here
      // for more than it proves (see task-2-report.md's "Fix round 1" for
      // the full investigation, including three sabotage runs that all
      // stayed green): every property this step can observe about an
      // installed, enabled mod — presence, filename, size, enabled state,
      // *and* the Modrinth project association — is reconstructible by the
      // launcher itself from the jar file alone, with no dependency on any
      // specific SQLite row surviving the restart. `cache_local`'s boot-time
      // scan (`managers/metadata/cache/mod.rs`, queued for every instance at
      // startup — `managers/instance/mod.rs:272-278`) rebuilds `ModFileCache`
      // and `ModMetadata` from the file and its content hash the instant
      // either is missing or mismatched, entirely locally; a second
      // background task (`cache_modplatform::<ModrinthModCacher>`,
      // `query_platform` in `managers/metadata/cache/modrinth/mod.rs`, driven
      // by `instance_mods_needing_mr_refresh` — `mod_file_cache.rs:354-360`)
      // then finds any mod lacking a `ModrinthModCache` row and re-derives it
      // with a real Modrinth hash-lookup API call, unconditionally, on every
      // boot. Sabotage confirmed empirically, not just reasoned about: deleting
      // only `ModFileCache`'s row stays green (local scan rebuilds it);
      // deleting only `ModrinthModCache`'s row *also* stays green (the
      // background task re-associates it from Modrinth before this step
      // runs); deleting `ModFileCache` **and** `ModMetadata` together
      // (cascading to `ModrinthModCache`/`CurseForgeModCache` too — the
      // entire per-mod DB footprint for this file) still stays green. So
      // this step is not read as proof any particular row persisted in
      // SQLite; it is a regression check that the reconciliation pipeline
      // itself — local scan plus background platform re-association —
      // still runs and still converges correctly after a real restart,
      // which is a real and independently useful thing to know (a bug that
      // silently broke that pipeline, or that the boot sequence stopped
      // invoking it at all, would go undetected without this).
      await test.step("assertion 3: the installed mod, still enabled", async () => {
        const mods = await openInstanceAddons(current!.page, INSTANCE_NAME)
        const found = mods.find((m) => m.filename === installedMod!.filename)
        expect(
          found,
          `instance.getInstanceMods after restart has no entry for Fabric ` +
            `API (got ${JSON.stringify(mods)})`
        ).toBeTruthy()

        expect(
          found!.modrinthProjectId,
          "Fabric API's Modrinth project association after restart"
        ).toBe(FABRIC_API_PROJECT_ID)

        // Against the pre-restart captured values (`installedMod`), never a
        // post-restart value read moments earlier: comparing the app's own
        // just-read report against itself would prove nothing about the
        // restart boundary this whole spec exists to check.
        expect(
          found!.enabled,
          "Fabric API enabled per the app's own UI/API"
        ).toBe(true)

        const diskInstalled = await verifyModInstalled(modsDir, {
          filename: installedMod!.filename,
          expectedSize: installedMod!.fileSize,
          expectedSha1: installedMod!.sha1 ?? undefined
        })
        expect(
          diskInstalled.ok,
          `Fabric API disk verification after restart: ${diskInstalled.problems.join("; ")}`
        ).toBe(true)

        const diskEnabled = await verifyModEnabled(
          modsDir,
          installedMod!.filename,
          true
        )
        expect(
          diskEnabled.ok,
          `Fabric API disk enabled-state after restart: ${diskEnabled.problems.join("; ")}`
        ).toBe(true)
      })

      await test.step("assertion 4: the disabled mod, still disabled", async () => {
        const mods = await openInstanceAddons(current!.page, INSTANCE_NAME)
        // Located by the pre-restart filename, same reasoning as assertion 3
        // — a search key should not double as the thing a later step might
        // want to assert on independently.
        const found = mods.find((m) => m.filename === disabledMod!.filename)
        expect(
          found,
          `instance.getInstanceMods after restart has no entry for Mod ` +
            `Menu (got ${JSON.stringify(mods)})`
        ).toBeTruthy()

        const diskResult = await verifyModEnabled(
          modsDir,
          disabledMod!.filename,
          false
        )

        // Both channels are checked and their agreement is reported
        // explicitly, rather than only asserting one — see the module doc
        // comment on why a restart is exactly the code path that exercises
        // the non-atomic disable + reconciliation-scan interaction.
        const uiSaysDisabled = found!.enabled === false
        const diskSaysDisabled = diskResult.ok

        if (uiSaysDisabled !== diskSaysDisabled) {
          throw new Error(
            `assertion 4: UI and disk disagree on Mod Menu's enabled state ` +
              `after restart — UI (instance.getInstanceMods) reports ` +
              `enabled=${found!.enabled}, disk reports ` +
              `${diskSaysDisabled ? "disabled" : "not disabled"} ` +
              `(${diskResult.problems.join("; ") || "no problems"}). This is ` +
              "exactly the disagreement window README.md's \"Known product " +
              'bug" section describes.'
          )
        }

        expect(
          found!.enabled,
          "Mod Menu enabled per the app's own UI/API"
        ).toBe(false)
        expect(
          diskResult.ok,
          `Mod Menu disk enabled-state after restart: ${diskResult.problems.join("; ")}`
        ).toBe(true)
      })

      expect(current.pageErrors, {
        message:
          "an uncaught renderer exception happened after the restart, " +
          "during the very reconciliation/re-hydration path this test exists " +
          "to exercise"
      }).toEqual([])
    } finally {
      if (current) {
        await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
          () => {}
        )
        await current.app.close().catch(() => {})
      }
      await stopHarness(harness)
    }
  })
})
