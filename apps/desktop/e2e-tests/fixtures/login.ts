import type { Page } from "playwright"
import { TEST_IDS, byTestId } from "../helpers/selectors.js"
import type { Harness } from "./mockIdp.js"

/**
 * Drives the real login UI from the welcome screen to the library.
 *
 * Device code is used rather than browser OAuth: it opens no system browser
 * and needs no protocol handler, so the whole exchange stays inside the app
 * and the mock.
 */
export async function completeLogin(
  page: Page,
  harness: Harness
): Promise<void> {
  await page.waitForSelector("#auth-flow")

  await page.click(byTestId(TEST_IDS.welcomeContinue))

  await page.click(byTestId(TEST_IDS.termsCheckbox))
  await page.click(byTestId(TEST_IDS.termsContinue))

  await page.click(byTestId(TEST_IDS.useDeviceCode))

  // Enrollment polls the mock's token endpoint until this releases it, which
  // is what makes the timing the test's to control rather than a race.
  await page.locator(byTestId(TEST_IDS.deviceCode)).first().waitFor()
  harness.mock.approve()

  // The provisioned user already exists at this OID, so the GDL step offers to
  // sync the existing account rather than register a new one.
  const sync = page.locator(byTestId(TEST_IDS.gdlSyncAccount))
  await sync.waitFor({ timeout: 60_000 })
  await sync.click()

  await page.waitForSelector(byTestId(TEST_IDS.libraryRoot), {
    timeout: 60_000
  })

  await dismissBetaPrompt(page)
}

/**
 * Closes the beta prompt when this run's installation ID lands in its cohort.
 *
 * Membership is a hash of an installation ID that every fresh runtime path
 * regenerates, so the modal appears on a fraction of runs and would otherwise
 * cover the library at random. "Never ask" rather than "maybe later", so it
 * stays closed for the life of the runtime path.
 */
async function dismissBetaPrompt(page: Page): Promise<void> {
  const never = page.locator(byTestId(TEST_IDS.betaPromptNever))

  if (await never.isVisible().catch(() => false)) {
    await never.click()
    await never.waitFor({ state: "hidden" })
  }
}
