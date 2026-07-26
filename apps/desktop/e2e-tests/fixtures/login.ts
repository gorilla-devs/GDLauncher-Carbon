import { expect } from "@playwright/test"
import type { Locator, Page } from "playwright"
import { TEST_IDS, byTestId } from "../helpers/selectors.js"
import type { Harness } from "./mockIdp.js"

/**
 * Bounded wait for a modal control that may never appear on this run.
 * Both first-launch queries (`settings.isFirstLaunch`,
 * `settings.shouldShowChangelog`) are local rspc calls answered well under a
 * second in practice, so this only needs enough slack to not flake on a
 * loaded CI runner — not enough to meaningfully tax every worker on the
 * (common) path where neither modal appears at all.
 */
const STARTUP_MODAL_POLL_MS = 5_000

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

/** Whether `locator` becomes visible within `timeout`, without throwing. */
async function appearsWithin(
  locator: Locator,
  timeout: number
): Promise<boolean> {
  return locator
    .waitFor({ state: "visible", timeout })
    .then(() => true)
    .catch(() => false)
}

/**
 * Dismisses the onboarding wizard and "what's new" changelogs modal that a
 * fresh runtime path queues on top of the library (`app.tsx`'s first-launch
 * effect opens onboarding; `Library/index.tsx`'s version-bump effect stacks
 * changelogs on top of it). Every worker gets a fresh runtime path, so every
 * worker hits this — call once, directly after `completeLogin`.
 *
 * Neither modal is guaranteed to appear (onboarding is gated on first
 * launch, changelogs on a version change), so each wait is short and
 * bounded and a missing modal is not an error.
 *
 * Dismissal goes through each modal's own control, not the `m[N]`
 * query-string stack: the changelogs header's close (X), and onboarding's
 * own wizard — two "Next" clicks into the final step's "Skip" control,
 * which is what actually calls `closeModal()` there (onboarding has no
 * header/X of its own, `noHeader: true`).
 */
export async function dismissStartupModals(page: Page): Promise<void> {
  // Changelogs stacks on top of onboarding (later entry in the modal stack
  // renders with the higher z-index), so it blocks onboarding underneath
  // and must be dismissed first.
  const changelogsClose = page.locator(byTestId(TEST_IDS.modalClose)).first()
  if (await appearsWithin(changelogsClose, STARTUP_MODAL_POLL_MS)) {
    await changelogsClose.click()
    await changelogsClose.waitFor({ state: "hidden" })
  }

  const onboardingNext = page.locator(byTestId(TEST_IDS.onboardingNext))
  if (await appearsWithin(onboardingNext, STARTUP_MODAL_POLL_MS)) {
    await page.click(byTestId(TEST_IDS.onboardingNext)) // step 1 -> step 2
    await page.click(byTestId(TEST_IDS.onboardingNext)) // step 2 -> step 3
    await page.click(byTestId(TEST_IDS.onboardingSkip))
    await page
      .locator(byTestId(TEST_IDS.onboardingSkip))
      .waitFor({ state: "hidden" })
  }

  // Confirm the overlay actually cleared rather than trusting the click
  // sequence above blindly — a failure here reports "startup modals
  // blocked the library" instead of surfacing later as an inexplicable
  // click timeout inside createInstanceViaUi.
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
  await expect(page.locator("#overlay")).toBeHidden()
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
