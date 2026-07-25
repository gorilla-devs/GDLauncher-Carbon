import { expect, test } from "./fixtures/index.js"
import { getActualPath } from "./tests_helpers.js"

test.describe("Init", () => {
  test("starts without a fatal crash and titles the window", async ({
    freshApp
  }) => {
    const { page, pageErrors } = freshApp

    await page.waitForSelector("#root")

    // The crash screen replaces the app entirely, so its absence is the real
    // assertion — #root renders either way.
    expect(await page.$("#appFatalCrashState")).toBeNull()
    expect(await page.title()).toBe("GDLauncher Carbon")
    // An uncaught exception outside Solid's error boundary never mounts
    // #appFatalCrashState, so this is what actually catches it.
    expect(pageErrors).toEqual([])
  })

  test("renders the login page when no account exists", async ({
    freshApp
  }) => {
    const { page } = freshApp

    expect(getActualPath(page.url())).toBe("/")
    await expect(page.locator("#auth-flow")).toBeVisible()
  })
})
