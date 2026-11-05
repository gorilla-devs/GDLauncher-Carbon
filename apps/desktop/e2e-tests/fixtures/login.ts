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
 *
 * `AuthFlow.tsx`'s `handleExit` now guards its own deferred post-login
 * `navigate()` against the route having already moved on, so callers no
 * longer need to wait out that window here.
 *
 * Also marks the search bar's onboarding tip seen (`markSearchTipSeen`
 * below), the one first-run interruption that is not a modal.
 */
export async function dismissStartupModals(page: Page): Promise<void> {
  // `ModalsManager` gives each modal `z-index: index + 1`, so only the
  // topmost one is clickable: a lower modal's X sits under the higher one's
  // backdrop, which swallows every click until the timeout. Dismiss from the
  // top of the stack down, hence `.last()` rather than `.first()`.
  //
  // The wait is keyed on the stack shrinking rather than on a locator going
  // hidden, because `.last()` re-resolves to the next modal down the instant
  // the top one closes — waiting for it to hide never completes while any
  // modal is left.
  const modalCloses = page.locator(byTestId(TEST_IDS.modalClose))
  while (await appearsWithin(modalCloses.last(), STARTUP_MODAL_POLL_MS)) {
    const before = await modalCloses.count()
    await modalCloses.last().click()
    await expect
      .poll(() => modalCloses.count(), {
        timeout: 15_000,
        message:
          "a startup modal did not close after its X was clicked — the " +
          "stack never shrank"
      })
      .toBeLessThan(before)
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

  await markSearchTipSeen(page)
}

/** `EnhancedSearchBar`'s `OnboardingTip` id, the one onboarding tip the
 *  suite's flows can open. */
const SEARCH_TIP_ID = "search-input-syntax"

/** How long the renderer is given to refetch its seen-tips list once the core
 *  has marked the tip: a local round trip plus the invalidation push. */
const SEEN_TIPS_REFETCH_TIMEOUT = 15_000

/**
 * Marks the search bar's onboarding tip as already seen, through the same
 * `settings.markOnboardingTipSeen` call dismissing it makes, so no spec
 * search can open it.
 *
 * The tip opens 200ms after the first click inside the search bar, as a
 * full-screen spotlight backdrop that takes every click until dismissed.
 * `dismissSearchOnboardingTip` (`helpers/mods.ts`) watches for it for 1s
 * after that click, but on a busy macOS CI renderer it opened later than
 * that and blocked the next click for its whole 30s timeout
 * (modpackInterruptedStaging, modpackLifecycle). Waiting longer there would
 * add a fixed delay to every search; marking it here means it never opens.
 * No spec tests the tip itself. One that does can bring it back with
 * `settings.resetOnboardingTips`.
 *
 * Waits until the renderer has refetched its seen-tips list with the tip in
 * it, so the first search after login cannot act on the old list.
 */
async function markSearchTipSeen(page: Page): Promise<void> {
  const core = await page.evaluate(async () => {
    const module = await window.getCoreModule()
    return module.type === "success"
      ? { port: module.port, apiToken: module.apiToken }
      : null
  })
  if (!core) {
    throw new Error("markSearchTipSeen: the core never reported READY")
  }

  // Armed before the mark, since the core's invalidation push is what makes
  // the renderer refetch, and that can come back before the mark's own reply.
  const refetched = page.waitForResponse(
    async (response) => {
      if (
        new URL(response.url()).pathname !==
        "/rspc/settings.getSeenOnboardingTips"
      ) {
        return false
      }
      const body = (await response.json().catch(() => null)) as {
        result?: { data?: unknown }
      } | null
      const seen = body?.result?.data
      return Array.isArray(seen) && seen.includes(SEARCH_TIP_ID)
    },
    { timeout: SEEN_TIPS_REFETCH_TIMEOUT }
  )
  // Awaited below; handled here too, so a failing mark does not also leave
  // this rejection unhandled.
  refetched.catch(() => {})

  const response = await fetch(
    `http://127.0.0.1:${core.port}/rspc/settings.markOnboardingTipSeen`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-token": core.apiToken
      },
      body: JSON.stringify(SEARCH_TIP_ID)
    }
  )
  const body = (await response.json().catch(() => null)) as {
    result?: { type?: string; data?: unknown }
  } | null
  if (!response.ok || body?.result?.type === "error") {
    throw new Error(
      `markSearchTipSeen: settings.markOnboardingTipSeen failed with ` +
        `${response.status}: ${JSON.stringify(body)}`
    )
  }

  await refetched.catch((cause: unknown) => {
    throw new Error(
      `markSearchTipSeen: the core marked "${SEARCH_TIP_ID}" seen, but the ` +
        "renderer did not refetch settings.getSeenOnboardingTips with it " +
        `within ${SEEN_TIPS_REFETCH_TIMEOUT}ms, so a search could still open it`,
      { cause }
    )
  })
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
