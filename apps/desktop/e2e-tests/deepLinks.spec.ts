/**
 * curseforge://, modrinth:// and gdlauncher:// links keep reaching a running
 * launcher after the user clicks an ordinary link inside it.
 *
 * A left click on a plain `<a href>` (a Discord or wiki link in a mod
 * description, a changelog, a news post) starts a main-frame navigation that
 * `main/index.ts`'s will-navigate handler cancels and opens in the browser.
 * The page never reloads, so the App's protocol subscription has to survive
 * it. When the protocol queue was revoked as soon as such a navigation
 * started, every later deep link only focused the window until a restart.
 *
 * A real reload must still hand the queue to the new document, so the check
 * runs again after `page.reload()`.
 *
 * Playwright learns that the click requested a navigation, but nothing tells
 * it the navigation was cancelled. Until the next navigation commits, every
 * auto-waiting locator action and assertion waits for that one and times
 * out. So between a link click and the next commit this spec only uses calls
 * that do not wait for it: a forced click, `dispatchEvent`, `isVisible()`
 * under `expect.poll`, and `page.evaluate`.
 *
 * Runs on the login page: App, which owns the protocol subscription, is
 * mounted there too, and a test-scoped launch keeps the reload away from
 * the shared logged-in app.
 */

import type { ElectronApplication, Locator, Page } from "playwright"
import { expect, test } from "./fixtures/index.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"

const PLAIN_LINK = "https://example.invalid/"
const SHARE_LINK = "gdlauncher://share/FIXTURE"
const LINK_ID = "e2e-plain-link"

/** Records shell.openExternal calls instead of opening a browser. */
async function stubOpenExternal(app: ElectronApplication) {
  await app.evaluate(({ shell }) => {
    const opened: string[] = []
    ;(globalThis as { __e2eOpenedExternal?: string[] }).__e2eOpenedExternal =
      opened
    shell.openExternal = async (url: string) => {
      opened.push(url)
    }
  })
}

function openedExternal(app: ElectronApplication) {
  return app.evaluate(
    () =>
      (globalThis as { __e2eOpenedExternal?: string[] }).__e2eOpenedExternal ??
      []
  )
}

/**
 * A real mouse click that does not wait on navigations: `force` skips the
 * wait for a pending one, which a cancelled link click leaves behind, and
 * `noWaitAfter` the wait for one this click requests.
 */
function clickWithoutNavigationWait(locator: Locator) {
  return locator.click({ force: true, noWaitAfter: true })
}

/** Left-clicks a plain, target-less link the way rendered markdown has them. */
async function clickPlainLink(page: Page) {
  await page.evaluate(
    ({ id, href }) => {
      document.getElementById(id)?.remove()
      const link = document.createElement("a")
      link.id = id
      link.href = href
      link.textContent = "Join our Discord"
      Object.assign(link.style, {
        position: "fixed",
        top: "8px",
        left: "8px",
        zIndex: "2147483647",
        padding: "8px",
        background: "white",
        color: "black"
      })
      document.body.appendChild(link)
    },
    { id: LINK_ID, href: PLAIN_LINK }
  )
  await clickWithoutNavigationWait(page.locator(`#${LINK_ID}`))
}

/** Waits for visibility through `isVisible()`, which does not wait on
 *  navigations, instead of `toBeVisible()`, which does. */
function expectVisibility(locator: Locator, visible: boolean) {
  return expect.poll(() => locator.isVisible()).toBe(visible)
}

/**
 * Marks the current document. The mark surviving a link click proves the
 * page did not reload, so the App that subscribed before the click is the
 * one the next deep link has to reach.
 */
function markDocument(page: Page) {
  return page.evaluate(() => {
    ;(window as { __e2eDocumentMark?: boolean }).__e2eDocumentMark = true
  })
}

function isMarkedDocument(page: Page) {
  return page.evaluate(
    () => (window as { __e2eDocumentMark?: boolean }).__e2eDocumentMark
  )
}

/** What the single-instance lock delivers when a website opens a link. */
async function openDeepLink(app: ElectronApplication, url: string) {
  await app.evaluate(({ app: electronApp }, link) => {
    electronApp.emit("second-instance", {}, ["gdlauncher", link])
  }, url)
}

test.describe("deep links", () => {
  test("a share link opens its preview after an in-app link click, and again after a reload", async ({
    freshApp
  }) => {
    const { app, page } = freshApp
    const authFlow = page.locator("#auth-flow")
    const preview = page.locator(byTestId(TEST_IDS.sharePreview))

    await expect(authFlow).toBeVisible({ timeout: 120_000 })
    await stubOpenExternal(app)

    await test.step("an in-app link opens in the browser and the page stays", async () => {
      await markDocument(page)
      await clickPlainLink(page)
      await expect.poll(() => openedExternal(app)).toEqual([PLAIN_LINK])
      expect(await isMarkedDocument(page)).toBe(true)
      await expectVisibility(authFlow, true)
    })

    await test.step("a share link opened afterwards shows its preview", async () => {
      await openDeepLink(app, SHARE_LINK)
      await expectVisibility(preview, true)
    })

    await test.step("close the preview so the reload starts from a URL without the modal param", async () => {
      // An open modal is written to the URL, which a reload keeps. A reload
      // never reopens it (the modal stack starts empty), so this only keeps
      // the reload below from starting on a URL that names the preview.
      const close = page.locator(byTestId(TEST_IDS.modalClose)).last()
      await expectVisibility(close, true)
      // Dispatched rather than clicked: the modal may still be playing its
      // enter animation, and a forced click aims at wherever the button was
      // a moment ago. dispatchEvent does not wait on navigations either.
      await close.dispatchEvent("click")
      await expectVisibility(preview, false)
    })

    await test.step("after a reload, the same sequence still works", async () => {
      await page.reload()
      // The reload committed, so auto-waiting works again until the next
      // link click.
      await expect(authFlow).toBeVisible({ timeout: 120_000 })
      await expect(preview).toBeHidden()
      await markDocument(page)
      await clickPlainLink(page)
      await expect
        .poll(() => openedExternal(app))
        .toEqual([PLAIN_LINK, PLAIN_LINK])
      expect(await isMarkedDocument(page)).toBe(true)
      await openDeepLink(app, SHARE_LINK)
      await expectVisibility(preview, true)
    })

    expect(await openedExternal(app)).toEqual([PLAIN_LINK, PLAIN_LINK])
  })
})
