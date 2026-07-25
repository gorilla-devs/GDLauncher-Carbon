import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import { isCoreModulePresent, launchApp } from "./fixtures/electronApp.js"
import { getActualUrl } from "./tests_helpers.js"

let electronApp: ElectronApplication
let page: Page
let runtimePath: string

test.describe("Init Tests", () => {
  test.beforeAll(async () => {
    test.setTimeout(120_000)

    expect(isCoreModulePresent()).toBeTruthy()

    runtimePath = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-e2e-init-"))
    const launched = await launchApp({ runtimePath })

    electronApp = launched.app
    page = launched.page

    page.on("pageerror", (error) => {
      console.error(JSON.stringify(error, null, 2))
      expect(error).toBeNull()
    })

    const rootDiv = await (await page.waitForSelector("#root"))?.innerHTML()
    expect(rootDiv).not.toBeUndefined()

    const errorInnerText = await (
      await page.$("#appFatalCrashState")
    )?.innerHTML()
    expect(errorInnerText).toBeUndefined()

    expect(await page.title()).toBe("GDLauncher Carbon")
  })

  test.afterAll(async () => {
    if (electronApp) {
      await electronApp.close()
    }

    if (runtimePath) {
      fs.rmSync(runtimePath, { recursive: true, force: true })
    }
  })

  test("renders the login page", async () => {
    expect(getActualUrl(page.url())).toBe("/")

    const loginContainer = await (
      await page.waitForSelector("#auth-flow")
    )?.innerHTML()

    expect(loginContainer).not.toBeUndefined()
  })
})
