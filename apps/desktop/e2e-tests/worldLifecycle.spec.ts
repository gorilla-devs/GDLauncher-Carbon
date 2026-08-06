import fs from "node:fs"
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
import { startHarness, stopHarness, type Harness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import {
  createInstanceViaUi,
  waitForInstallComplete
} from "./helpers/instances.js"
import {
  ADDON_FIXTURES,
  addonDir,
  type AddonFixture
} from "./helpers/addonFixtures.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { byModRow, byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  deleteModViaUi,
  INSTALL_TIMEOUT,
  installModIntoInstance,
  openAddonPage,
  openInstanceAddons,
  searchForMod,
  type InstalledMod
} from "./helpers/mods.js"

/**
 * Worlds are the one addon type that is a **directory** on disk, and every
 * behaviour below follows from that.
 *
 * Install extracts the downloaded zip into `saves/` and deletes the zip
 * (`CurseforgeModInstaller::post_process`, `managers/instance/installer/mod.rs`),
 * so the end state is a directory and no archive. Enable/disable is not
 * offered, because renaming a save to `<name>.disabled` does not hide it from
 * Minecraft and our own scanner re-reports it as a new, enabled world
 * (`supportsEnableToggle`, `pages/Library/shared/addons/addonCapabilities.ts`).
 * Delete asks first, because a world is the only addon whose removal destroys
 * unrecoverable user data (`requiresDeletionConfirmation`, same file) —
 * unless "don't ask again" was ticked on an earlier delete, which persists as
 * a Settings toggle (`settings.get/setWorldDeletionWarningDismissed`) and
 * suppresses the dialog on every later delete until it's flipped back on.
 *
 * The third test's relaunch assertion is why this file exists at all, rather
 * than one more case bolted onto `addonLifecycle.spec.ts`. The bug it guards
 * against: a world delete that removes nothing from disk while dropping the
 * cache row anyway (`remove_addon_from_disk`'s doc comment,
 * `managers/instance/mods.rs`), so the entry vanishes from the UI and the
 * next scan re-inserts it from the directory that is still there.
 *
 * That "next scan" is not gated on a boot, and this test does not claim it
 * is. The scan that would re-insert an orphaned world directory as a "new"
 * one is `cache_local` (`managers/metadata/cache/mod.rs`), driven by the
 * `instance.prioritizeInstanceCache` mutation that `useAddonData.tsx`'s
 * `onMount` fires on *every* mount of the Addons tab, not only at startup —
 * the same mechanism `installWorld`'s own reconciliation poll below already
 * depends on to see a freshly installed world appear. Confirmed directly:
 * sabotaging both `remove_addon_from_disk` and `delete_mod`'s bail-check to
 * reproduce that state, then polling the same live session's own list
 * instead of relaunching, resurrects
 * the orphaned world within the same process — no restart required. A
 * same-session re-poll would therefore also have caught this bug; the
 * relaunch below is not the only assertion capable of it.
 *
 * It is still the right one to write here. A relaunch exercises the actual,
 * end-to-end path a user would hit the bug through — closing and reopening
 * the launcher, not clicking a tab twice in one sitting — and a full cold
 * boot (process start, DB open, the same reconciliation scan a real next
 * launch runs) is strictly more of the real product than a same-session
 * re-poll happens to share with it. Read this assertion as the stronger,
 * more product-realistic of two checks that would both have caught the
 * historical bug, not as the only one that could.
 *
 * **This describe block drives its own app, harness and instance instead of
 * the shared `installedInstance` fixture every other addon spec in this suite
 * uses.** `installedInstance` is worker-scoped (`fixtures/index.ts`) —
 * computed once and handed, as the same object, to every test in every spec
 * file this worker runs, and this suite is pinned to `workers: 1` specifically
 * so that holds across a whole run (`playwright.config.ts`'s own comment on
 * why). The third test below has to close that app and replace it with a
 * freshly launched one (`relaunchApp`) to prove anything at all — fine for an
 * app this file owns outright, but doing that to the shared fixture would
 * leave its `.app`/`.page` pointing at a closed Electron process for every
 * later test in the worker that requests it, breaking them regardless of
 * whether this file happens to sort alphabetically last (which is the only
 * thing that would save it today, and exactly the kind of ordering hazard
 * `fixtures/installedInstance.ts`'s own doc comment warns is fragile).
 * `persistence.spec.ts` hit the identical conflict for the identical reason —
 * it also has to close and relaunch the app mid-test — and resolved it the
 * same way: drive `startHarness`/`launchApp`/`relaunchApp`/`stopHarness`
 * directly instead of going through `fixtures/index.ts`. This file follows
 * that precedent rather than inventing a second one.
 *
 * The cost is the same trade `persistence.spec.ts` already made: paying for
 * its own login/enrollment and a **cold** instance install rather than
 * reusing `installedInstance`'s warm, worker-shared substrate
 * (assets/libraries/managed_javas) — accepted because the alternative is
 * silently corrupting every other test that shares the worker.
 *
 * All five tests share one instance across the file's lifetime
 * (`test.describe.configure({ mode: "serial" })` plus `beforeAll`/`afterAll`
 * holding the app/harness). The second and third tests reuse the one world
 * the first test installs — they're specifically about the state that
 * install leaves behind, so re-installing per test would just re-prove the
 * first test for no added coverage. The fourth and fifth tests each need
 * their own fresh, not-yet-deleted world instead (a delete consumes the
 * world it's proving something about), so they share a small `installWorld`
 * helper that repeats the search-install-reconcile sequence rather than the
 * full placement assertions the first test already owns.
 */
test.describe("world addon lifecycle", () => {
  test.describe.configure({ mode: "serial" })

  const maybeFixture = ADDON_FIXTURES.find((f) => f.addonType === "worlds")
  if (!maybeFixture) {
    throw new Error(
      'worldLifecycle.spec.ts: ADDON_FIXTURES has no "worlds" entry — ' +
        "helpers/addonFixtures.ts may have changed shape"
    )
  }
  // Rebound with an explicit, non-optional type rather than relying on
  // narrowing surviving into `installWorld` below (a named function
  // declaration, not a `test(...)` callback in the same scope the guard
  // ran in) — `tsc` does not carry the `if (!maybeFixture) throw` guard's
  // narrowing that far, so without this rebinding `installWorld` sees
  // `fixture` as possibly `undefined` again.
  const fixture: AddonFixture = maybeFixture

  const INSTANCE_NAME = "gdl-e2e-world-lifecycle"
  const MC_VERSION = "1.20.1"
  // Fastest loader to install (~8s warm, per fixtures/installedInstance.ts's
  // own measurement) — worth picking here too even on this file's cold
  // substrate, since a world's own behaviour has nothing to do with which
  // loader the instance runs.
  const LOADER = "fabric"

  // True for a download-in-progress marker, never a finished addon's real
  // on-disk name — see addonPlacement.spec.ts's identical constant for the
  // full mechanism (carbon_net's `.__gdl_part~` suffix while a download is in
  // flight). Copied, not imported: a three-line predicate, not worth a
  // cross-file dependency.
  const isPartFile = (name: string) => name.endsWith(".__gdl_part~")

  // How long the app's own addon list is given to catch up with a file
  // already confirmed on disk. Mirrors addonPlacement.spec.ts's identical
  // constant and rationale: a world's completion signal is a raw disk poll,
  // not the button-text wait that's otherwise guaranteed to already be caught
  // up by the time it resolves.
  const RECONCILIATION_WAIT = 30_000

  let harness!: Harness
  let launchOpts!: LaunchOptions
  let current!: { app: ElectronApplication; page: Page; pageErrors: Error[] }
  // `saves/` for this file's one instance, resolved once its shortpath is
  // known in beforeAll. Stable for the file's whole life, relaunch included —
  // a relaunch never changes an instance's on-disk directory name.
  let saves!: string

  /**
   * Installs a fresh copy of the world fixture into `saves/` and returns the
   * new directory's name, diffed against whatever was already there.
   *
   * Shared by the first test (which additionally asserts the placement
   * contract — directory, no leftover zip — on top of this) and the fourth
   * and fifth tests (which only need an undeleted world to run a delete
   * scenario against, not a re-proof of placement). Ends with `page` parked
   * on the instance's Addons tab with the new world's row already rendered —
   * every caller immediately locates that row via `byModRow`, so this waits
   * for the app's own list to report it rather than returning the instant
   * the file lands on disk (the periodic `cache_local` pass that inserts the
   * `ModFileCache` row runs decoupled from the install completing — see
   * `RECONCILIATION_WAIT`'s doc comment).
   */
  async function installWorld(page: Page): Promise<string> {
    const before = fs.existsSync(saves) ? fs.readdirSync(saves) : []

    await openInstanceAddons(page, INSTANCE_NAME)
    await searchForMod(page, {
      query: fixture.query,
      platform: fixture.platform,
      searchType: fixture.searchType
    })
    // Required before installModIntoInstance: `addon-install-button` only
    // exists on the addon's own page, never on the search-results list
    // `searchForMod` leaves `page` on — see TEST_IDS.addonInstallButton's doc
    // comment in helpers/selectors.ts.
    await openAddonPage(page, fixture.projectId)

    // What `saves/` gained since `before`, minus the in-flight download
    // marker. The single source of truth for both "is the install finished?"
    // below and "what did it install?" after it, so the two cannot disagree
    // about what they are looking at.
    const addedEntries = () =>
      (fs.existsSync(saves) ? fs.readdirSync(saves) : []).filter(
        (f) => !before.includes(f) && !isPartFile(f)
      )

    // installModIntoInstance's own button-text completion check cannot see a
    // world finishing — ModDownloadButton's `isWorld` spinner logic conflates
    // "not started" with "finished" (see that function's doc comment in
    // helpers/mods.ts). The real target directory is the completion signal
    // instead.
    //
    // The condition is the install's **settled** end state, not the first
    // sign of progress toward it: exactly one new entry, and it is not the
    // downloaded archive. `CurseforgeModInstaller::post_process`
    // (`managers/instance/installer/mod.rs`) decompresses the zip into
    // `saves/` and unlinks it *afterwards*, so for the entire duration of the
    // extraction the new directory and the zip it came from sit in `saves/`
    // side by side. A condition satisfied by "some new non-zip entry exists"
    // is true throughout that whole window, which makes every read taken
    // after it a coin flip on whether the zip is gone yet — and the window
    // widens with machine load (a 4MB extraction), so it opens up precisely
    // when the whole suite is running rather than this file alone. Requiring
    // the diff to have collapsed to the one extracted directory waits the
    // window out by construction.
    const waitForCompletion = async () => {
      await expect
        .poll(
          () => {
            const added = addedEntries()
            return (
              added.length === 1 && !added[0].toLowerCase().endsWith(".zip")
            )
          },
          {
            timeout: INSTALL_TIMEOUT,
            message:
              `installWorld: ${saves} never settled on exactly one new, ` +
              "fully-processed entry — a `.zip` still sitting beside the " +
              "extracted directory means post_process never deleted it"
          }
        )
        .toBe(true)
    }

    await installModIntoInstance(page, {
      instanceName: INSTANCE_NAME,
      waitForCompletion
    })

    const added = addedEntries()
    if (added.length !== 1) {
      throw new Error(
        `installWorld: expected exactly one new entry in ${saves} after ` +
          `install, got ${JSON.stringify(added)}`
      )
    }
    const worldName = added[0]

    // The app's own list can lag disk by several seconds after an install
    // (the periodic metadata-cache pass runs decoupled from the mutation
    // installModIntoInstance already awaited) — poll rather than read once,
    // the same hazard addonPlacement.spec.ts's own RECONCILIATION_WAIT
    // documents.
    let installed: InstalledMod | undefined
    await expect
      .poll(
        async () => {
          const listed = await openInstanceAddons(page, INSTANCE_NAME)
          installed = listed.find((m) => m.filename === worldName)
          return installed !== undefined
        },
        {
          timeout: RECONCILIATION_WAIT,
          message: `installWorld: the app's own list never reported "${worldName}"`
        }
      )
      .toBe(true)
    if (installed!.addonType !== "worlds") {
      throw new Error(
        `installWorld: "${worldName}" reconciled with addonType ` +
          `"${installed!.addonType}", not "worlds"`
      )
    }

    return worldName
  }

  test.beforeAll(async () => {
    expect(isCoreModulePresent()).toBeTruthy()

    harness = await startHarness()
    launchOpts = {
      runtimePath: harness.runtimePath,
      baseApi: `${harness.mock.url}/gdl`,
      e2eAuthBase: harness.mock.url,
      e2eEntitlementKey: harness.entitlementKeyPath,
      e2eUpdateFeed: `${harness.mock.url}/updates/`
    }

    current = await launchApp(launchOpts)
    await completeLogin(current.page, harness)
    await dismissStartupModals(current.page)

    await createInstanceViaUi(current.page, {
      name: INSTANCE_NAME,
      version: MC_VERSION,
      loader: LOADER
    })
    await waitForInstallComplete(current.page, INSTANCE_NAME)

    const { shortpath } = readInstanceByName(harness.runtimePath, INSTANCE_NAME)
    const instanceDataPath = path.join(
      harness.runtimePath,
      "instances",
      shortpath,
      "instance"
    )
    saves = addonDir(instanceDataPath, "worlds")
  })

  test.afterAll(async () => {
    await current?.app.close().catch(() => {})
    if (harness) {
      await stopHarness(harness)
    }
  })

  // eslint-disable-next-line no-empty-pattern
  test.afterEach(async ({}, testInfo) => {
    await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(() => {})
  })

  test("installs a world as an extracted directory with no leftover zip", async () => {
    const worldName = await installWorld(current.page)

    const worldPath = path.join(saves, worldName)
    expect(fs.statSync(worldPath).isDirectory(), {
      message: `${worldName} must be an extracted directory, not an archive`
    }).toBe(true)
    expect(
      fs.readdirSync(saves).filter((f) => f.endsWith(".zip")),
      {
        message: "the downloaded world zip must be deleted after extraction"
      }
    ).toHaveLength(0)
  })

  test("offers no enable/disable toggle for a world", async () => {
    const { page } = current
    const worldName = fs.readdirSync(saves)[0]

    await openInstanceAddons(page, INSTANCE_NAME)
    const row = page.locator(byModRow(worldName))
    // Sanity precondition: a toHaveCount(0) on the toggle below would pass
    // just as well if the row itself failed to render at all, which would
    // prove nothing about the behaviour this test exists to check.
    await expect(row).toBeVisible()
    await expect(row.locator(byTestId(TEST_IDS.modRowToggle)), {
      message:
        "a world row must render no enable toggle — renaming a save " +
        "directory cannot disable it, and our own scanner would re-report " +
        "<name>.disabled as a new, enabled world"
    }).toHaveCount(0)

    // The column toggle is only one of the routes to `handleToggleMod`. The
    // row's own right-click menu offered the same Enable/Disable action
    // unconditionally, so a check that stopped at the column above would go
    // green while the behaviour was still two clicks away — and, with the
    // column hidden, with no control left to undo it.
    await row.click({ button: "right" })
    // A positive anchor in the same open menu, asserted first: without it a
    // `toHaveCount(0)` on the toggle would be satisfied just as well by a
    // menu that never opened, or by `data-testid` silently not reaching the
    // DOM through `@gd/ui`'s ContextMenuItem (hazard 1 in
    // `helpers/selectors.ts`'s header).
    await expect(page.locator(byTestId(TEST_IDS.addonContextDelete)), {
      message:
        "the world row's context menu did not open (or its entries carry no " +
        "test anchors), so the absence check below would prove nothing"
    }).toBeVisible()
    await expect(page.locator(byTestId(TEST_IDS.addonContextToggle)), {
      message:
        "a world's context menu must offer no Enable/Disable entry — same " +
        "reason the column toggle is hidden: the rename it performs cannot " +
        "disable a save, and the scanner re-reports the renamed directory " +
        "as a new, enabled world"
    }).toHaveCount(0)

    // Closed again before leaving: this file's tests share one page, and the
    // next one clicks the same row's delete control, which an open menu
    // would intercept.
    await page.keyboard.press("Escape")
    await expect(
      page.locator(byTestId(TEST_IDS.addonContextDelete))
    ).toHaveCount(0)
  })

  test("deleting a world asks first, then removes the save for good", async () => {
    const worldName = fs.readdirSync(saves)[0]

    await openInstanceAddons(current.page, INSTANCE_NAME)
    await current.page
      .locator(byModRow(worldName))
      .locator(byTestId(TEST_IDS.modRowDelete))
      .click()

    const confirm = current.page.locator(
      byTestId(TEST_IDS.worldDeletionConfirm)
    )
    await expect(confirm, {
      message: "deleting a world must raise a confirmation dialog"
    }).toBeVisible()
    await confirm.click()

    await expect
      .poll(() => fs.existsSync(path.join(saves, worldName)), {
        message: "confirming must remove the save directory from disk"
      })
      .toBe(false)

    // A relaunch is not the only thing that could observe this bug — a
    // same-session re-poll of the Addons tab would too, since the scan that
    // would re-insert an orphaned world (`cache_local`,
    // `managers/metadata/cache/mod.rs`) is driven by every tab mount, not
    // gated on a boot (see this file's own header doc comment for the
    // experiment that confirmed this). A relaunch is used here anyway
    // because it exercises the real, end-to-end path a user would actually
    // hit the bug through, not merely one mechanism among several capable of
    // detecting it.
    //
    // `current` is reassigned here rather than destructured into a local
    // `page` at the top of this test, so every reference below (and
    // `afterEach`'s log attachment) reaches the new page/app rather than a
    // stale handle to the one just closed.
    current = await relaunchApp(current, launchOpts)
    await dismissStartupModals(current.page)

    const listed = await openInstanceAddons(current.page, INSTANCE_NAME)
    expect(
      listed.find((m) => m.filename === worldName),
      {
        message:
          "the world came back after a relaunch — its directory was never " +
          "removed and the next scan re-inserted it"
      }
    ).toBeUndefined()
  })

  test('ticking "don\'t ask again" still deletes the world and marks the warning dismissed in Settings', async () => {
    const worldName = await installWorld(current.page)

    await current.page
      .locator(byModRow(worldName))
      .locator(byTestId(TEST_IDS.modRowDelete))
      .click()

    const confirm = current.page.locator(
      byTestId(TEST_IDS.worldDeletionConfirm)
    )
    await expect(confirm, {
      message: "deleting a world must raise a confirmation dialog"
    }).toBeVisible()

    // The testid sits on a `<span>` inside `Checkbox`'s children, not on the
    // Checkbox itself — `Checkbox` (packages/ui) puts its pointer handlers on
    // the outer wrapping div, not a native `<label>`/`<input>` pair, and
    // renders `props.children` as a plain sibling inside that same div. A
    // click anywhere inside it — including here — bubbles up to those
    // handlers, so this reaches a real toggle rather than an inert label.
    await current.page.locator(byTestId(TEST_IDS.worldDeletionDontAsk)).click()
    await confirm.click()

    await expect
      .poll(() => fs.existsSync(path.join(saves, worldName)), {
        message:
          'confirming with "don\'t ask again" ticked must still remove the ' +
          "save directory"
      })
      .toBe(false)

    await current.page.click(byTestId(TEST_IDS.navbarSettings))
    // Same split as every other Switch anchor in this suite
    // (TEST_IDS.settingsReducedMotionToggle, modRowToggle): the testid is on
    // a wrapping div because the native `<input type="checkbox">` Switch
    // renders is zero-size and cannot itself be asserted against visually,
    // but its `checked` property is what's actually readable.
    const warningToggle = current.page
      .locator(byTestId(TEST_IDS.settingsWorldDeletionWarning))
      .locator('input[type="checkbox"]')
    await expect(warningToggle, {
      message:
        'the Settings toggle must reflect "don\'t ask again" as dismissed ' +
        "(unchecked — General.tsx renders it checked={!dismissed})"
    }).not.toBeChecked()
  })

  test("suppresses the confirmation on the next delete, until re-enabled in Settings", async () => {
    // installWorld navigates via the library regardless of where `page`
    // currently is (the previous test left it on Settings), so no extra
    // navigation is needed before calling it here.
    const worldName = await installWorld(current.page)

    // The warning is dismissed from the previous test, so `handleDeleteMod`
    // (useAddonMutations.tsx) must skip straight to performDeleteMod without
    // ever opening the modal — the exact same rspc round trip every other
    // addon type's delete already goes through, which is what makes
    // deleteModViaUi() (helpers/mods.ts) — built for that direct path, not
    // this dialog-gated one — usable here.
    const dialog = current.page.locator(byTestId(TEST_IDS.worldDeletionConfirm))
    await deleteModViaUi(current.page, worldName)
    await expect(dialog, {
      message:
        'the confirmation dialog must not appear once "don\'t ask again" ' +
        "has been dismissed"
    }).toHaveCount(0)
    await expect
      .poll(() => fs.existsSync(path.join(saves, worldName)), {
        message:
          "a suppressed-dialog delete must still remove the save directory"
      })
      .toBe(false)

    // Re-enable the warning from Settings — deleteModViaUi already left
    // `page` on the instance's Addons tab, not Settings, so this needs its
    // own navigation.
    await current.page.click(byTestId(TEST_IDS.navbarSettings))
    // Click target vs. assertion target are deliberately two different
    // locators, same split as every other Switch anchor in this suite
    // (persistence.spec.ts's settingsReducedMotionToggle,
    // helpers/mods.ts's modRowToggle): the native `<input type="checkbox">`
    // Switch renders is zero-size (`w-0 h-0`) and cannot itself receive a
    // pointer click — the wrapping div is the real clickable surface, the
    // input underneath is what `toBeChecked()` reads.
    const warningWrapper = current.page.locator(
      byTestId(TEST_IDS.settingsWorldDeletionWarning)
    )
    const warningToggle = warningWrapper.locator('input[type="checkbox"]')
    await expect(warningToggle, {
      message:
        "sanity: the toggle should still read dismissed before this test " +
        "re-enables it"
    }).not.toBeChecked()
    await warningWrapper.click()
    await expect(warningToggle, {
      message: "clicking the Settings toggle must re-enable the warning"
    }).toBeChecked()

    // A fresh delete must ask again.
    const secondWorldName = await installWorld(current.page)
    await current.page
      .locator(byModRow(secondWorldName))
      .locator(byTestId(TEST_IDS.modRowDelete))
      .click()

    const confirmAgain = current.page.locator(
      byTestId(TEST_IDS.worldDeletionConfirm)
    )
    await expect(confirmAgain, {
      message:
        "re-enabling the warning in Settings must bring the confirmation " +
        "dialog back on the next delete"
    }).toBeVisible()
    await confirmAgain.click()

    await expect
      .poll(() => fs.existsSync(path.join(saves, secondWorldName)), {
        message: "confirming must still remove the save directory"
      })
      .toBe(false)
  })
})
