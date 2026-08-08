import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import type { Locator, Page } from "@playwright/test"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  deleteInstanceViaUi,
  ensureLibraryInteractive,
  waitForInstallComplete
} from "./helpers/instances.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { classifyPackinfo, packinfoDataPath } from "./helpers/packinfo.js"
import { readInstallAudit } from "./helpers/installAudit.js"
import { snapshotTree } from "./helpers/instanceTree.js"
import {
  fetchMrpackIndex,
  installModpackVersion,
  openInstance
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_SLUG,
  MODPACK_MR_V_MID
} from "./helpers/modpackFixtures.js"

/**
 * Covers `instance.getRepairPreview` (`RepairModpack/index.tsx`'s dry-run
 * counts/file-list/checkbox) against what a real repair (`repair_modpack`)
 * THEN actually does to the same instance — two tests, one per checkbox
 * state:
 *
 * 1. The default (`re_enable_disabled: false`): the preview's counts and
 *    per-file verdicts for a deleted, a truncated, and an edited pack file
 *    match the repair that follows, and a disabled pack mod is counted and
 *    reported as kept disabled, not touched.
 * 2. Ticking "Also re-enable disabled pack mods": the same disabled mod
 *    instead counts (and is reported) as re-enabled, and the repair that
 *    follows actually renames it back.
 *
 * **Out of scope, deliberately.** The untracked-file list and its
 * "Check pack origin" button (`repair-untracked-row`/`repair-check-origin`/
 * `repair-select-proven`) are not exercised here: origin-checking hits the
 * live Modrinth API per untracked file, which is expensive and flaky to
 * script into a counts test, and the verdict-matching logic itself
 * (`origin_check.rs`) is already unit-tested. This file only proves the
 * preview/repair correspondence for the packinfo-tracked buckets.
 *
 * **Mechanism.** `repair_preview` (`managers/instance/modpack/mod.rs`) is a
 * pure, read-only dry run of the exact same planner
 * (`apply_plan::plan`/`decide_repair`) a real repair executes: it hashes
 * what's on disk (`disk_scan::scan_disk_state`), treats the recorded
 * `packinfo.json` as the target, and — since nothing is actually staged for
 * a preview — synthesizes every declared path as "obtainable" so the plan
 * never blocks on a real staging directory. `tally_counts` then buckets each
 * `(action, reason)` pair: a present-but-wrong-hash file with the correct
 * bytes assumed available is `Replace(Disabled)`/`RepairOverwrote` ->
 * `restore_modified`; a missing one is `Create`/`RepairRestored` ->
 * `restore_deleted`; an untouched disabled twin is `Keep`/`DisabledByUser`
 * -> `disabled_kept` unless `re_enable_disabled` is set, in which case it's
 * `ReEnable`/`ReEnabled` -> `re_enabled` instead, unconditionally (see
 * `apply_plan.rs`'s own `decide_repair` doc comment — that flag alone
 * decides the whole `Disabled` arm). `RepairModpack/index.tsx`'s
 * `countRows` renders one line per non-zero bucket, and its raw file list
 * (behind `repair-preview-expand`) renders `repairReasonLabel` for every
 * entry, whatever the count.
 *
 * This test 1 uses a HEALTHY instance — a fresh install that has never been
 * through a version change — deliberately, matching `modpackReinstall.spec.ts`'s
 * own precedent. `repair_preview`'s own doc comment names a real
 * preview/execution asymmetry: the preview treats the recorded
 * `packinfo.json` as authoritative for both `old` and `target` and never
 * touches the network, while a real repair re-verifies the pack's true
 * current manifest over the wire. A version-changed instance is where that
 * asymmetry could start to matter; a fresh install removes the variable
 * entirely, which is what lets this test compare the preview's own counts
 * directly against what the real repair that follows actually does.
 *
 * Every key both tests mutate is picked at runtime from `classifyPackinfo`'s
 * own `pristine` list, cross-referenced against the live `.mrpack` index
 * (overrides for the delete/truncate/edit legs, `files[]`-declared `/mods/`
 * entries for the disable leg) — never a hardcoded filename — each carrying
 * a named `toBeDefined()` check, the same convention
 * `modpackReinstall.spec.ts` established for exactly this reason.
 *
 * **The re-enable checkbox has no native input to click.** `repair-reenable-checkbox`
 * (`helpers/selectors.ts`) anchors the `@gd/ui` `Checkbox`'s own label
 * `children`, not the control itself — `Checkbox` doesn't forward unknown
 * props, and unlike a native `<input>`+`<label for>` pair, its toggle
 * handlers live on the wrapping `<div>` all of `Checkbox`'s own rendered
 * content (icon box and label alike) sits inside, so a plain click anywhere
 * within it — the anchored label span included — toggles it via ordinary
 * event bubbling.
 */

/** The counts block renders one line per non-zero `RepairCounts` bucket
 *  (`RepairModpack/index.tsx`'s `countRows`), each the translated,
 *  `{{count}}`-interpolated sentence for that bucket
 *  (`packages/i18n/locale/english/instances.json`). No row carries its own
 *  id, so a specific bucket's count is read by matching its own fixed
 *  phrase against the whole block's text rather than by position. */
const MODIFIED_PHRASE = /(\d+) files you modified will be reset/
const DELETED_PHRASE = /(\d+) files you deleted will be restored/
const DISABLED_KEPT_PHRASE = /(\d+) disabled mods kept disabled/
const RE_ENABLED_PHRASE = /(\d+) disabled mods will be re-enabled/

function extractCount(countsText: string, phrase: RegExp): number {
  const match = countsText.match(phrase)
  return match ? Number(match[1]) : 0
}

/**
 * One `repair-preview-entry` row, located by the exact path in its path
 * span's own `title` attribute — every packinfo-tracked path renders a row
 * once the list is expanded, not just the ones a test mutated, and `title`
 * carries the full path even where the sibling text node is CSS-truncated,
 * so this can never be fooled by a visually-clipped rendering of a
 * different, merely similar-looking path.
 */
function repairEntryRow(page: Page, filePath: string): Locator {
  const escaped = filePath.replace(/(["\\])/g, "\\$1")
  return page
    .locator(byTestId(TEST_IDS.repairPreviewEntry))
    .filter({ has: page.locator(`[title="${escaped}"]`) })
}

test.describe("modpack repair preview", () => {
  test("repair preview counts match what repair then does", async ({
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

      const status = await classifyPackinfo(root)

      // Delete/truncate/edit legs: pristine OVERRIDES, same reasoning
      // `modpackReinstall.spec.ts` gives for why (simplest to seed; no
      // behavioural difference from a `.files`-declared mod under repair).
      const overridePaths = new Set(index.overrides)
      const overrideCandidates = status.pristine.filter((k) =>
        overridePaths.has(k.slice(1))
      )
      const deletedKey = overrideCandidates[0]
      const corruptKey = overrideCandidates[1]
      const editKey = overrideCandidates[2]

      // Disable leg: a pristine `.files`-declared mod — a real jar under
      // `/mods/`, not an override, since the leg under test is specifically
      // "a pack MOD the user disabled".
      const declaredPaths = new Set(index.files.map((f) => f.path))
      const modCandidates = status.pristine.filter(
        (k) =>
          declaredPaths.has(k.slice(1)) &&
          k.startsWith("/mods/") &&
          k !== deletedKey &&
          k !== corruptKey &&
          k !== editKey
      )
      const disabledKey = modCandidates[0]

      for (const [label, key] of [
        ["a first pristine override to delete", deletedKey],
        ["a second pristine override to truncate", corruptKey],
        ["a third pristine override to edit", editKey],
        ["a pristine pack mod to disable", disabledKey]
      ] as const) {
        expect(
          key,
          `no pristine candidate was available as ${label} in ` +
            `"${MODPACK_MR_SLUG}" — the fixture's shape must have changed; ` +
            "re-measure it"
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

      await fs.promises.rm(packinfoDataPath(root, deletedKey))
      await fs.promises.writeFile(packinfoDataPath(root, corruptKey), "")
      const editedBody = "e2e-repair-preview-edit\n"
      await fs.promises.writeFile(packinfoDataPath(root, editKey), editedBody)
      const disabledEnabledPath = packinfoDataPath(root, disabledKey)
      await fs.promises.rename(
        disabledEnabledPath,
        `${disabledEnabledPath}.disabled`
      )

      await openInstance(page, name)
      await page.click(byTestId(TEST_IDS.instanceMenuTrigger))
      const menuEntry = page.locator(byTestId(TEST_IDS.instanceMenuRepair))
      await expect(
        menuEntry,
        `the repair menu entry was disabled for "${name}" — the instance ` +
          "has no modpack association"
      ).toBeEnabled()
      await menuEntry.click()

      const counts = page.locator(byTestId(TEST_IDS.repairPreviewCounts))
      await expect(
        counts,
        "the repair preview's counts block never appeared"
      ).toBeVisible()
      const countsText = await counts.innerText()

      expect(
        extractCount(countsText, MODIFIED_PHRASE),
        `expected 2 files-modified in the preview counts (truncated + ` +
          `edited), got counts text: ${JSON.stringify(countsText)}`
      ).toBe(2)
      expect(
        extractCount(countsText, DELETED_PHRASE),
        `expected 1 file-deleted in the preview counts, got counts text: ` +
          JSON.stringify(countsText)
      ).toBe(1)
      expect(
        extractCount(countsText, DISABLED_KEPT_PHRASE),
        `expected 1 disabled-mod-kept in the preview counts (re-enable ` +
          `unticked), got counts text: ${JSON.stringify(countsText)}`
      ).toBe(1)

      await page.click(byTestId(TEST_IDS.repairPreviewExpand))

      const expectedVerdicts: [string, string][] = [
        [deletedKey, "restored (was missing)"],
        [corruptKey, "restored (was modified)"],
        [editKey, "restored (was modified)"],
        [disabledKey, "disabled — kept disabled"]
      ]
      for (const [filePath, verdict] of expectedVerdicts) {
        const row = repairEntryRow(page, filePath)
        await expect(
          row,
          `no repair-preview-entry row rendered for "${filePath}" after ` +
            "expanding the list"
        ).toHaveCount(1)
        await expect(
          row.locator("span").nth(1),
          `"${filePath}"'s preview row did not show the expected verdict`
        ).toHaveText(verdict)
      }

      await page.click(byTestId(TEST_IDS.repairModpackConfirm))
      await waitForInstallComplete(page, name)

      const after = await snapshotTree(data)
      const audit = await readInstallAudit(root)
      expect(audit, "repair wrote no install audit").not.toBeNull()

      // Deleted -> restored: byte-identical to its pre-deletion pristine
      // copy and to the pack's own declared content, recorded as created.
      expect(
        after.get(deletedKey.slice(1))?.sha256,
        "repair did not restore the deleted pack file"
      ).toBe(before.get(deletedKey.slice(1))?.sha256)
      expect(
        after.get(deletedKey.slice(1))?.sha256,
        "the restored file's bytes do not match the pack's own declared content"
      ).toBe(deletedDeclared!.sha256)
      expect(
        audit!.created,
        `audit did not record creating ${deletedKey}`
      ).toContain(deletedKey)

      // Truncated -> repaired: non-empty again, byte-identical to its own
      // pristine original, recorded as replaced.
      expect(
        after.get(corruptKey.slice(1))?.size,
        "repair did not repair the truncated pack file"
      ).toBeGreaterThan(0)
      expect(
        after.get(corruptKey.slice(1))?.sha256,
        "the repaired file's bytes do not match its own pristine original"
      ).toBe(before.get(corruptKey.slice(1))?.sha256)
      expect(
        audit!.replaced,
        `audit did not record repairing ${corruptKey}`
      ).toContain(corruptKey)

      // Edited config -> reset to the pack's own bytes.
      const editedFileNow = await fs.promises.readFile(
        packinfoDataPath(root, editKey),
        "utf8"
      )
      expect(
        editedFileNow,
        "repair left the user's edit in place instead of repairing it"
      ).not.toBe(editedBody)
      expect(
        after.get(editKey.slice(1))?.sha256,
        "the reset file's bytes do not match its own pristine original"
      ).toBe(before.get(editKey.slice(1))?.sha256)
      expect(
        audit!.replaced,
        `audit did not record repairing ${editKey}`
      ).toContain(editKey)

      // Disabled mod -> left disabled (the default preserves it), recorded
      // as skipped/disabled-by-user, never re-enabled.
      expect(
        fs.existsSync(disabledEnabledPath),
        "repair re-enabled the disabled mod even though the checkbox was " +
          "never ticked"
      ).toBe(false)
      expect(
        fs.existsSync(`${disabledEnabledPath}.disabled`),
        "the disabled mod's .disabled twin is gone after repair"
      ).toBe(true)
      expect(
        audit!.skipped.find((s) => s.file === disabledKey)?.reason,
        `audit did not record ${disabledKey} as skipped/disabled-by-user`
      ).toBe("disabled-by-user")
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
            'cleanup for "repair preview counts match what repair then ' +
              'does" also failed:',
            cleanupError
          )
        }
      }
      await ensureLibraryInteractive(page)
    }
  })

  test("repair re-enables disabled pack mods when asked", async ({
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

      const status = await classifyPackinfo(root)
      const declaredPaths = new Set(index.files.map((f) => f.path))
      const modCandidates = status.pristine.filter(
        (k) => declaredPaths.has(k.slice(1)) && k.startsWith("/mods/")
      )
      const disabledKey = modCandidates[0]
      expect(
        disabledKey,
        `no pristine pack mod was available to disable in ` +
          `"${MODPACK_MR_SLUG}" — the fixture's shape must have changed; ` +
          "re-measure it"
      ).toBeDefined()

      const enabledPath = packinfoDataPath(root, disabledKey)
      await fs.promises.rename(enabledPath, `${enabledPath}.disabled`)

      await openInstance(page, name)
      await page.click(byTestId(TEST_IDS.instanceMenuTrigger))
      const menuEntry = page.locator(byTestId(TEST_IDS.instanceMenuRepair))
      await expect(
        menuEntry,
        `the repair menu entry was disabled for "${name}" — the instance ` +
          "has no modpack association"
      ).toBeEnabled()
      await menuEntry.click()

      const counts = page.locator(byTestId(TEST_IDS.repairPreviewCounts))
      await expect(
        counts,
        "the repair preview's counts block never appeared"
      ).toBeVisible()
      await expect(
        counts,
        "the preview did not count the disabled mod as disabled-kept " +
          "before the re-enable checkbox was ever ticked"
      ).toContainText(DISABLED_KEPT_PHRASE)

      // See this file's module doc comment for why a plain click on the
      // anchored label span toggles the checkbox despite there being no
      // native input under it.
      await page.click(byTestId(TEST_IDS.repairReenableCheckbox))

      await expect(
        counts,
        "the preview counts never reflected the re-enable checkbox (no " +
          '"will be re-enabled" row appeared)'
      ).toContainText(RE_ENABLED_PHRASE)
      const reenabledCountsText = await counts.innerText()
      expect(
        extractCount(reenabledCountsText, RE_ENABLED_PHRASE),
        `expected 1 disabled-mod-to-re-enable in the preview counts, got ` +
          `counts text: ${JSON.stringify(reenabledCountsText)}`
      ).toBe(1)
      expect(
        reenabledCountsText,
        "the disabled-kept row is still rendered after ticking re-enable " +
          "— the mod should now count as re-enabled instead"
      ).not.toMatch(DISABLED_KEPT_PHRASE)

      await page.click(byTestId(TEST_IDS.repairModpackConfirm))
      await waitForInstallComplete(page, name)

      expect(
        fs.existsSync(enabledPath),
        "repair did not re-enable the disabled mod after the checkbox was ticked"
      ).toBe(true)
      expect(
        fs.existsSync(`${enabledPath}.disabled`),
        "the disabled mod's .disabled twin is still present after re-enabling"
      ).toBe(false)

      const audit = await readInstallAudit(root)
      expect(audit, "repair wrote no install audit").not.toBeNull()
      expect(
        audit!.reEnabled,
        `audit did not record re-enabling ${disabledKey}`
      ).toContain(disabledKey)
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
            'cleanup for "repair re-enables disabled pack mods when asked" ' +
              "also failed:",
            cleanupError
          )
        }
      }
      await ensureLibraryInteractive(page)
    }
  })
})
