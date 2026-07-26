import { expect, type Page } from "@playwright/test"
import { byInstanceName, byTestId, TEST_IDS } from "./selectors.js"

/** Opens the creation modal and creates a custom instance on `version`. */
export async function createInstanceViaUi(
  page: Page,
  opts: { name: string; version: string }
): Promise<void> {
  await page.click(byTestId(TEST_IDS.addInstance))
  await page.click(byTestId(TEST_IDS.instanceCreationCustomTab))

  await page.fill(byTestId(TEST_IDS.instanceCreationName), opts.name)

  await page.click(byTestId(TEST_IDS.instanceCreationVersionTrigger))
  // Kobalte portals the option list outside the modal subtree, so this is
  // queried from the page root rather than scoped to the dialog.
  const option = page.locator(
    byTestId(`instance-creation-version-option-${opts.version}`)
  )

  // The matrix (globalSetup.ts) draws from Mojang's version_manifest_v2.json;
  // this dropdown is fed by `mc.getMinecraftVersions`, which the core
  // resolves from GDL's own meta.gdl.gg. The two sources can diverge — most
  // plausibly for the newest release, right after a Mojang release or during
  // a daedalus ingest stall — so a matrix version absent from the dropdown is
  // a real cross-source condition, not a broken test. Checked with a bound of
  // its own, short next to the 60s action timeout, so that case fails in
  // seconds with a named cause instead of hanging on the click below and
  // surfacing as an unrelated timeout.
  const offered = await option
    .first()
    .waitFor({ state: "visible", timeout: 5_000 })
    .then(() => true)
    .catch(() => false)
  if (!offered) {
    throw new Error(
      `version "${opts.version}" is in the e2e matrix (drawn from Mojang's ` +
        "version_manifest_v2.json, launchermeta.mojang.com) but is not " +
        "offered by the instance-creation dropdown (drawn from GDL's own " +
        "meta, meta.gdl.gg, via mc.getMinecraftVersions) — the two sources " +
        "have diverged, most likely because meta has not yet ingested a " +
        "version Mojang just shipped. Not caused by the commit under test."
    )
  }
  await option.first().click()

  await page.click(byTestId(TEST_IDS.instanceCreationSubmit))
  await expect(page.locator(byInstanceName(opts.name))).toBeVisible()
}

/**
 * Waits for an instance to finish installing.
 *
 * `inactive` is both the pre-prepare and the post-install state, so this
 * waits for the tile to leave it before waiting for it to come back.
 */
export async function waitForInstallComplete(
  page: Page,
  name: string,
  opts: { startTimeout?: number; installTimeout?: number } = {}
): Promise<void> {
  const selector = byInstanceName(name)
  const startTimeout = opts.startTimeout ?? 90_000
  // 90s + 11m = 12.5m against the 15m per-test ceiling, leaving 2.5m of
  // margin for the creation-modal interactions, the post-install assertions,
  // and a bounded cleanup in `finally` — so this throws its own message
  // rather than the whole budget being exhausted first and the message being
  // discarded along with it when Playwright cuts the test off with no
  // diagnosis.
  const installTimeout = opts.installTimeout ?? 11 * 60_000

  await page
    .waitForFunction(
      (sel) =>
        document.querySelector(sel)?.getAttribute("data-instance-state") !==
        "inactive",
      selector,
      { timeout: startTimeout }
    )
    .catch((cause) => {
      throw new Error(
        `instance "${name}" never started preparing within ${startTimeout}ms ` +
          `(state stayed "inactive" — prepareInstance likely never ran)`,
        { cause }
      )
    })

  await page
    .waitForFunction(
      (sel) =>
        document.querySelector(sel)?.getAttribute("data-instance-state") ===
        "inactive",
      selector,
      { timeout: installTimeout }
    )
    .catch((cause) => {
      throw new Error(
        `instance "${name}" did not finish installing within ${installTimeout}ms`,
        { cause }
      )
    })

  const tile = page.locator(selector)
  if ((await tile.getAttribute("data-instance-failed")) === "true") {
    const reason = await tile.getAttribute("data-instance-fail-reason")
    throw new Error(
      `instance "${name}" finished with a failed install task` +
        (reason ? `: ${reason}` : " (no cause reported)")
    )
  }
}

/** Removes an instance so the next matrix entry starts from a clean library. */
export async function deleteInstanceViaUi(
  page: Page,
  name: string,
  opts: { timeout?: number } = {}
): Promise<void> {
  // Bounded well under the global 60s action timeout: this runs inside a
  // `finally` after whatever the test body already spent, and the context
  // menu's delete item stays `disabled` while an install is still running
  // (components/Instance/Tile.tsx) — after `waitForInstallComplete` times
  // out the instance is stuck in exactly that state, so the click below can
  // never succeed. Failing in a few seconds rather than waiting the full
  // action timeout keeps that case from eating the rest of the test's
  // remaining budget and risking the same silent-abandonment failure mode
  // `waitForInstallComplete`'s own timeout exists to avoid.
  const timeout = opts.timeout ?? 15_000

  // Deletion lives on the tile's context menu (components/Instance/Tile.tsx
  // opens the `confirmInstanceDeletion` modal from a ContextMenuItem), so
  // this right-clicks rather than left-clicks.
  await page.click(byInstanceName(name), { button: "right", timeout })
  await page.click(byTestId(TEST_IDS.instanceContextDelete), { timeout })
  await page.click(byTestId(TEST_IDS.confirmInstanceDeletion), { timeout })
  await expect(page.locator(byInstanceName(name))).toHaveCount(0, { timeout })
}

/**
 * Restores the app to an interactive library, regardless of how the
 * previous test ended. A test that exhausts the 15-minute budget across
 * several bounded waits is abandoned by Playwright without running its own
 * `finally`, which can leave the creation modal open over the shared
 * worker-scoped app — every remaining
 * matrix entry would then hang clicking `library-add-instance` behind the
 * modal's own `#overlay`. Closing the modal is best-effort (never throws):
 * a test that already failed must not have that failure buried by a
 * recovery one. The final checks do throw, since a library that is still
 * not interactive after the close attempt means the next test would hang
 * anyway and should fail loudly here instead, with a clear cause, rather
 * than silently.
 */
export async function ensureLibraryInteractive(page: Page): Promise<void> {
  const closeButton = page.locator(byTestId(TEST_IDS.modalClose)).first()
  if (await closeButton.isVisible().catch(() => false)) {
    await closeButton.click({ timeout: 5_000 }).catch(() => {})
  }

  await expect(page.locator("#overlay")).toBeHidden()
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}
