import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import { adText, adViewInfo } from "./helpers/adView.js"
import { AD_HOSTILE_CSS, goToLibrary } from "./helpers/themes.js"
import { expect, test } from "./fixtures/index.js"
import {
  computeAdStackBounds,
  NAVBAR_HEIGHT,
  type AdLayout
} from "../packages/main/adView/geometry.js"

test.describe("Protected ad view", () => {
  test("shows above the library at the computed bounds", async ({
    authenticatedApp
  }) => {
    const { app, page } = authenticatedApp
    await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()

    const layout = (await page.evaluate(() =>
      (window as any).getAdSize()
    )) as AdLayout
    const info = await adViewInfo(app)

    expect(info.adUrl).toMatch(/adview\/index\.html(\?|$)/)
    const ad = info.children.filter((c) => c.visible)
    expect(ad).toHaveLength(1)

    const expected = computeAdStackBounds(
      { width: info.content.width, height: info.content.height },
      layout
    )
    expect(expected).not.toBeNull()
    expect(ad[0].bounds).toEqual(expected)
    expect(ad[0].bounds.y).toBe(NAVBAR_HEIGHT)
  })

  test("e2e builds never load real ads", async ({ authenticatedApp }) => {
    const { app, page } = authenticatedApp
    await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
    const counts = await app.evaluate(({ webContents }) => {
      const ad = webContents
        .getAllWebContents()
        .find((wc) => wc.getURL().includes("adview/index.html"))
      if (!ad) throw new Error("ad view not found")
      return ad.executeJavaScript(
        `({ real: document.querySelectorAll("owadview").length, boxes: document.querySelectorAll("[data-ad-placeholder]").length })`
      ) as Promise<{ real: number; boxes: number }>
    })
    expect(counts.real).toBe(0)
    expect(counts.boxes).toBeGreaterThanOrEqual(1)
  })

  test("a hostile stylesheet can't touch it", async ({ authenticatedApp }) => {
    const { app, page } = authenticatedApp
    const before = await adViewInfo(app)

    const tag = await page.addStyleTag({ content: AD_HOSTILE_CSS })
    try {
      const after = await adViewInfo(app)
      expect(after.children).toEqual(before.children)
      expect(after.adUrl).toBe(before.adUrl)
      expect(after.children.some((c) => c.visible)).toBe(true)
    } finally {
      await tag.evaluate((el) => (el as Element).remove())
    }
  })

  test("the why-ads link opens the dialog", async ({ authenticatedApp }) => {
    const { app, page } = authenticatedApp
    const layout = (await page.evaluate(() =>
      (window as any).getAdSize()
    )) as AdLayout
    test.skip(
      layout.hideAdText === true,
      "this display is too small for the ad page to show its links; run under a bigger screen, e.g. xvfb-run -a --server-args='-screen 0 2200x1300x24'"
    )

    await app.evaluate(({ webContents }) => {
      const ad = webContents
        .getAllWebContents()
        .find((wc) => wc.getURL().includes("adview/index.html"))
      if (!ad) throw new Error("ad view not found")
      return ad.executeJavaScript('document.getElementById("why").click()')
    })

    const modalText = page
      .locator("#overlay")
      .getByText(/why are ads needed/i)
      .first()
    await expect(modalText).toBeVisible()
    await page.keyboard.press("Escape")
    await expect(modalText).toBeHidden()
  })

  test("stays on app routes and follows the language", async ({
    authenticatedApp
  }) => {
    const { app, page } = authenticatedApp
    const layout = (await page.evaluate(() =>
      (window as any).getAdSize()
    )) as AdLayout
    test.skip(
      layout.hideAdText === true,
      "this display is too small for the ad page to show its links; run under a bigger screen, e.g. xvfb-run -a --server-args='-screen 0 2200x1300x24'"
    )

    await page.locator(byTestId(TEST_IDS.navbarSettings)).click()
    await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeHidden()
    expect((await adViewInfo(app)).children.some((c) => c.visible)).toBe(true)

    const english = await adText(app, "#why")
    expect(english).toBe("Why are ads needed?")

    try {
      await page.evaluate(() => {
        window.location.hash = "#/settings/language"
      })
      await page.getByText("Italiano", { exact: true }).click()
      await expect
        .poll(() => adText(app, "#why"), { timeout: 15_000 })
        .not.toBe(english)
      expect((await adViewInfo(app)).children.some((c) => c.visible)).toBe(true)
    } finally {
      await page.getByText("English", { exact: true }).first().click()
      await expect
        .poll(() => adText(app, "#why"), { timeout: 15_000 })
        .toBe(english)
      // The app is shared with later specs, which start from the library.
      await goToLibrary(page)
    }
  })

  test("follows the window when it is resized", async ({
    authenticatedApp
  }) => {
    const { app, page } = authenticatedApp

    // The app is shared with later specs, so the size goes back afterwards.
    const before = await app.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()[0].getSize()
    )
    try {
      await app.evaluate(({ BrowserWindow }) => {
        BrowserWindow.getAllWindows()[0].setSize(1300, 950)
      })
      const layout = (await page.evaluate(() =>
        (window as any).getAdSize()
      )) as AdLayout

      await expect
        .poll(async () => {
          const info = await adViewInfo(app)
          const visible = info.children.find((c) => c.visible)
          return visible ? visible.bounds.x - info.content.width : null
        })
        .toBe(-layout.adSize.width)
    } finally {
      await app.evaluate(({ BrowserWindow }, [w, h]) => {
        BrowserWindow.getAllWindows()[0].setSize(w, h)
      }, before)
    }
  })
})
