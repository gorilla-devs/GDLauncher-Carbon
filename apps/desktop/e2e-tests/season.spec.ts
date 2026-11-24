import type { Page, Request } from "@playwright/test"
import { byHook, byTestId, byThemeTile, TEST_IDS } from "./helpers/selectors.js"
import {
  checkSeason,
  dialogOpens,
  E2E_SEASON_YEARS,
  goTo,
  pinWindowSize,
  resetSeason,
  restoreMain,
  rspc,
  savedTheme,
  seasonDialog,
  seasonStart,
  setSeasonToday,
  setTheme
} from "./helpers/themes.js"
import { expect, test } from "./fixtures/index.js"

const getSeason = (page: Page) => rspc(page, "query", "themes.getSeason")

// This spec's own seasons: the dialog, kept; the next December's, switched
// back; and the year after, opted out.
const [FIRST, NEXT, AFTER] = E2E_SEASON_YEARS.season

test.describe("Christmas season", () => {
  test("the Christmas season follows the owner's rules", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const theme = () => savedTheme(page)
    const html = page.locator("html")
    const dialog = seasonDialog(page)
    // The dialog only opens from a notice, so once it is null nothing can
    // open one.
    const settled = async () => {
      await expect.poll(async () => (await getSeason(page)).notice).toBeNull()
    }
    const onDay = async (iso: string) => {
      setSeasonToday(runtime, iso)
      await checkSeason(page)
    }

    const writes: string[] = []
    const onRequest = (r: Request) => {
      if (r.url().includes("/rspc/settings.setSettings")) writes.push(r.url())
    }
    page.on("request", onRequest)
    const restoreSize = await pinWindowSize(app)
    try {
      // The scenario ends opted out in the core's season state row, so it
      // starts (and ends) from a reset one and runs on any runtime.
      await resetSeason(page, runtime)
      // 1. The e2e build ignores the real clock.
      setSeasonToday(runtime, null)
      await checkSeason(page)
      expect((await getSeason(page)).today).toBe("2026-06-15")

      // 2. Locked outside the season.
      await goTo(page, "#/settings/appearance")
      const tile = page.locator(byThemeTile("silent-night"))
      await expect(tile).toBeVisible()
      await expect(tile).toHaveAttribute("data-locked", "true")
      await tile.hover()
      await expect(
        page.getByText("Available from December 1 to January 31")
      ).toBeVisible()
      // Above the tile, so the theme's name under it stays readable.
      const tipOverTile = async () => {
        const tip = await page.locator(byHook("tooltip")).boundingBox()
        const box = await tile.boundingBox()
        return tip && box ? tip.y + tip.height - box.y : Infinity
      }
      await expect.poll(tipOverTile).toBeLessThanOrEqual(1)
      await expect(html).toHaveAttribute("data-gdl-theme", "main")
      writes.length = 0
      await tile.click()
      // A round trip through the core means any write the click fired has
      // been processed; the request listener catches the write itself.
      await getSeason(page)
      expect(writes, "a locked click sends no settings write").toEqual([])
      await expect(html).toHaveAttribute("data-gdl-theme", "main")
      expect(await theme()).toBe("main")

      // 3. December auto-selects and announces it.
      await goTo(page, "#/library")
      await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
      await setTheme(page, "frost")
      setSeasonToday(runtime, seasonStart(FIRST))
      const status = await checkSeason(page)
      if (status.notice === null) {
        throw new Error(
          "December 1 did not auto-select Silent Night: the shared runtime is already opted out from an earlier run and needs a fresh runtime"
        )
      }
      await expect.poll(theme).toBe("silent-night")
      await expect(html).toHaveAttribute("data-gdl-theme", "silent-night")
      await dialogOpens(page)
      await expect(page.getByText(/until January 31/)).toBeVisible()
      await page.locator(byTestId("season-keep")).click()
      await expect(dialog).toBeHidden()
      await settled()

      // 4. Still in season: nothing changes and nothing opens.
      await onDay(`${FIRST + 1}-01-15`)
      await settled()
      await expect(dialog).toBeHidden()
      expect(await theme()).toBe("silent-night")

      // 5. The season ends: the previous theme returns with a toast.
      await onDay(`${FIRST + 1}-02-01`)
      // Toasts are short-lived, so look right after the trigger.
      await expect(page.getByText(/over for this year/)).toBeVisible()
      await expect.poll(theme).toBe("frost")
      await expect(html).toHaveAttribute("data-gdl-theme", "frost")

      // 6. Next December: the dialog again; switching back opts out.
      await onDay(seasonStart(NEXT))
      await expect.poll(theme).toBe("silent-night")
      await dialogOpens(page)
      await page.locator(byTestId("season-switch-back")).click()
      await expect.poll(theme).toBe("frost")
      await expect(html).toHaveAttribute("data-gdl-theme", "frost")
      await expect(dialog).toBeHidden()

      // 7. The year after: opted out, but a hand-pick still works.
      await onDay(seasonStart(AFTER))
      await settled()
      expect(await theme()).toBe("frost")
      await expect(dialog).toBeHidden()
      await goTo(page, "#/settings/appearance")
      const open = page.locator(byThemeTile("silent-night"))
      await expect(open).toBeVisible()
      await expect(open).not.toHaveAttribute("data-locked", "true")
      writes.length = 0
      await open.click()
      // Positive control: the listener can see a real write.
      await expect.poll(() => writes.length).toBeGreaterThan(0)
      await expect.poll(theme).toBe("silent-night")
      await settled()
      await expect(dialog).toBeHidden()
      await setTheme(page, "frost")
      await expect.poll(theme).toBe("frost")
      await settled()
      await expect(dialog).toBeHidden()
    } finally {
      page.off("request", onRequest)
      try {
        await restoreSize()
      } finally {
        try {
          // First: it moves the theme off a December date's Silent Night, so
          // the reset below can't leave an Ended notice behind.
          await restoreMain(page, runtime)
        } finally {
          await resetSeason(page, runtime)
        }
      }
    }
  })
})
