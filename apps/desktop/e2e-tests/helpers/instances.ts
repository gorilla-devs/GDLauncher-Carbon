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
  const option = byTestId(`instance-creation-version-option-${opts.version}`)
  await page.click(option)

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
  // Under the 15m per-test ceiling so this throws its own message rather
  // than being cut off by Playwright with no diagnosis.
  const installTimeout = opts.installTimeout ?? 13 * 60_000

  await page
    .waitForFunction(
      (sel) =>
        document.querySelector(sel)?.getAttribute("data-instance-state") !==
        "inactive",
      selector,
      { timeout: startTimeout }
    )
    .catch(() => {
      throw new Error(
        `instance "${name}" never started preparing within ${startTimeout}ms ` +
          `(state stayed "inactive" — prepareInstance likely never ran)`
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
    .catch(() => {
      throw new Error(
        `instance "${name}" did not finish installing within ${installTimeout}ms`
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
  name: string
): Promise<void> {
  // Deletion lives on the tile's context menu (components/Instance/Tile.tsx
  // opens the `confirmInstanceDeletion` modal from a ContextMenuItem), so
  // this right-clicks rather than left-clicks.
  await page.click(byInstanceName(name), { button: "right" })
  await page.click(byTestId(TEST_IDS.instanceContextDelete))
  await page.click(byTestId(TEST_IDS.confirmInstanceDeletion))
  await expect(page.locator(byInstanceName(name))).toHaveCount(0)
}
