/**
 * Drives the Settings -> "Clean up" cache-cleanup flow through the real UI.
 *
 * Separate from `helpers/instances.ts` because what it drives is a settings
 * maintenance action rather than anything instance-scoped — the scopes it
 * clears are runtime-path-wide and affect every instance at once.
 */

import { expect, type Page } from "@playwright/test"
import { byTestId, TEST_IDS } from "./selectors.js"

/** The cleanup walks and deletes whole directory trees (`assets/` alone is
 *  tens of thousands of files on a populated runtime path), so it gets far
 *  more room than a UI interaction normally would. Deliberately bounded
 *  rather than left to the 15-minute test timeout: an unbounded wait here
 *  would surface a wedged cleanup as an abandoned test with no `finally`,
 *  which is the cascade `playwright.config.ts`'s `actionTimeout` comment
 *  describes. */
const CLEANUP_TIMEOUT = 180_000

/** Which caches a cleanup run should clear. Both are stated explicitly —
 *  see `runCacheCleanup`'s doc comment for why neither may be left implicit. */
export interface CacheScopes {
  gdlauncher: boolean
  minecraft: boolean
}

/**
 * Runs a cache cleanup through the real Settings UI and returns once the
 * modal reports it finished.
 *
 * `scopes` is *set*, not toggled. The modal opens with `gdlauncher` checked
 * and `minecraft` unchecked (`CacheCleanup/index.tsx`'s `createSignal`
 * defaults), and clicking a row flips it — so a caller wanting only
 * `minecraft` has to turn `gdlauncher` off as well as turning `minecraft`
 * on. Leaving `gdlauncher` on is not a harmless extra: that scope wipes DB
 * tables, including the `Instance` row for whatever instance the calling
 * test is about to make assertions against.
 *
 * Both scope rows are located by the wrapper `ClickableRow` renders rather
 * than by the `Checkbox` inside them, and their state is read from
 * `data-checked` on that same wrapper. `@gd/ui`'s `Checkbox` forwards no
 * unknown props and renders no queryable input, so it can be neither
 * anchored nor read — see `helpers/selectors.ts`'s note.
 */
export async function runCacheCleanup(
  page: Page,
  scopes: CacheScopes
): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarSettings))
  await page.click(byTestId(TEST_IDS.settingsCacheCleanupOpen))

  const rows: [string, boolean][] = [
    [TEST_IDS.cacheCleanupScopeGdlauncher, scopes.gdlauncher],
    [TEST_IDS.cacheCleanupScopeMinecraft, scopes.minecraft]
  ]

  for (const [testId, wanted] of rows) {
    const row = page.locator(byTestId(testId))
    await expect(row).toBeVisible()

    if ((await row.getAttribute("data-checked")) !== String(wanted)) {
      await row.click()
    }

    // Asserted rather than assumed: the click above is a no-op when the row
    // already matches, so without this a selector that silently stopped
    // matching would leave the default selection (gdlauncher on, minecraft
    // off) in place and the cleanup would clear the wrong scope entirely.
    await expect(row, {
      message:
        `cache cleanup scope "${testId}" did not settle to ${wanted} — the ` +
        "cleanup would have run against the wrong caches"
    }).toHaveAttribute("data-checked", String(wanted))
  }

  await page.click(byTestId(TEST_IDS.cacheCleanupStart))

  // The done panel is rendered only by the modal's `phase() === "done"`
  // branch, so its presence is the completion signal. A `failed` run renders
  // a different branch and this times out rather than passing quietly.
  await expect(page.locator(byTestId(TEST_IDS.cacheCleanupDone)), {
    message: `cache cleanup did not report completion within ${CLEANUP_TIMEOUT}ms`
  }).toBeVisible({ timeout: CLEANUP_TIMEOUT })

  await page.click(byTestId(TEST_IDS.modalClose))
}
