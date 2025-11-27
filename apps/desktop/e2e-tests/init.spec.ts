import { expect, test } from "@playwright/test"
import fs from "fs"
import path from "path"
import { _electron as electron } from "playwright"
import type { ElectronApplication, Page } from "playwright"
import { getActualUrl } from "./tests_helpers.js"
import { fileURLToPath } from "url"
import { dirname } from "path"

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

let electronApp: ElectronApplication

const getRootPath = () => {
  let basePath = path.resolve(__dirname, "../", "release")

  if (process.platform === "win32") {
    basePath = path.join(basePath, "win-unpacked")
  } else if (process.platform === "linux") {
    basePath = path.join(basePath, "linux-unpacked")
  } else if (process.platform === "darwin") {
    basePath = path.join(basePath, "mac-universal", "GDLauncher.app")
  }

  return basePath
}

const getBinaryPath = async () => {
  const rootPath = getRootPath()

  if (process.platform === "win32") {
    return path.join(rootPath, "GDLauncher.exe")
  } else if (process.platform === "linux") {
    return path.join(rootPath, "@gddesktop")
  } else if (process.platform === "darwin") {
    return path.join(rootPath, "Contents", "MacOS", "GDLauncher")
  }
}

const isCoreModulePresent = () => {
  const rootPath = getRootPath()

  if (process.platform === "win32") {
    const core_path = path.join(
      rootPath,
      "resources",
      "binaries",
      "core_module.exe"
    )
    console.log("Core module path:", core_path)
    return fs.existsSync(core_path)
  } else if (process.platform === "linux") {
    const core_path = path.join(
      rootPath,
      "resources",
      "binaries",
      "core_module"
    )
    console.log("Core module path:", core_path)
    return fs.existsSync(core_path)
  } else if (process.platform === "darwin") {
    const core_path = path.join(
      rootPath,
      "Contents",
      "Resources",
      "binaries",
      "core_module"
    )
    console.log("Core module path:", core_path)
    return fs.existsSync(core_path)
  }
}

test.describe("Init Tests", () => {
  test.beforeAll(async () => {
    expect(isCoreModulePresent()).toBeTruthy()

    // set the CI environment variable to true
    process.env.CI = "e2e"

    const binaryPath = await getBinaryPath()
    console.log("Launching Electron from:", binaryPath)
    console.log("Binary exists:", fs.existsSync(binaryPath!))
    if (binaryPath) {
      const stats = fs.statSync(binaryPath)
      console.log(
        "Binary is executable:",
        !!(stats.mode & fs.constants.S_IXUSR)
      )
      console.log("Binary size:", stats.size)
    }
    console.log("Environment:", {
      DISPLAY: process.env.DISPLAY,
      CI: process.env.CI
    })

    // Direct execution test - commented out but kept for debugging if needed
    // Useful for capturing early startup errors before Playwright launch
    // console.log("=== Testing direct binary execution ===")
    // try {
    //   const result = spawnSync(binaryPath!, ["--version"], {
    //     timeout: 5000,
    //     encoding: "utf8",
    //     env: { ...process.env, DISPLAY: process.env.DISPLAY || ":1" }
    //   })
    //   console.log("Direct execution exit code:", result.status)
    //   console.log("Direct execution signal:", result.signal)
    //   if (result.stdout) console.log("Direct execution stdout:", result.stdout)
    //   if (result.stderr) console.log("Direct execution stderr:", result.stderr)
    //   if (result.error) console.error("Direct execution error:", result.error)
    // } catch (e: any) {
    //   console.error("Direct execution exception:", e.message)
    // }
    // console.log("=== End direct binary test ===\n")

    electronApp = await electron
      .launch({
        args: [],
        executablePath: binaryPath,
        env: { ...process.env } as any
      })
      .catch((error) => {
        console.error("=== Electron Launch Failed ===")
        console.error("Error message:", error.message)
        console.error("Error name:", error.name)
        console.error("Error stack:", error.stack)

        // Try to get process exit info if available
        if (error.cause) {
          console.error("Error cause:", error.cause)
        }

        console.error(
          "Full error object:",
          JSON.stringify(error, Object.getOwnPropertyNames(error), 2)
        )
        throw error
      })

    electronApp.on("console", (msg) => {
      console.log(msg.text())
    })

    electronApp.process().stdout?.on("data", (data) => {
      console.log(data.toString())
    })

    electronApp.process().stderr?.on("data", (data) => {
      console.log(data.toString())
    })

    page = await electronApp.firstWindow()

    // capture errors
    page.on("pageerror", (error) => {
      console.error(JSON.stringify(error, null, 2))
      expect(error).toBeNull()
    })
    // capture console messages
    page.on("console", (msg) => {
      console.log(msg.text())
      // expect(msg.type()).not.toBe("error");
    })

    const rootDiv = await (await page.waitForSelector("#root"))?.innerHTML()
    expect(rootDiv).not.toBeUndefined()

    const errorInnerText = await (
      await page.$("#appFatalCrashState")
    )?.innerHTML()
    expect(errorInnerText).toBeUndefined()

    const title = await page.title()
    expect(title).toBe("GDLauncher Carbon")
  })

  test.afterAll(async () => {
    if (!electronApp) return

    await electronApp.close()
  })

  let page: Page

  test("renders the login page", async () => {
    page = await electronApp.firstWindow()

    const currentUrl = page.url()
    expect(getActualUrl(currentUrl)).toBe("/")

    const loginContainer = await (
      await page.waitForSelector("#auth-flow")
    )?.innerHTML()

    expect(loginContainer).not.toBeUndefined()
  })

  // Also test missing core_module
})
