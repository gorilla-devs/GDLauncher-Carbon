import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { _electron as electron } from "playwright"
import type { ElectronApplication, Page } from "playwright"
import type { TestInfo } from "@playwright/test"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export function getRootPath(
  platform: NodeJS.Platform = process.platform
): string {
  const basePath = path.resolve(__dirname, "..", "..", "release")

  if (platform === "win32") {
    return path.join(basePath, "win-unpacked")
  }

  if (platform === "linux") {
    return path.join(basePath, "linux-unpacked")
  }

  return path.join(basePath, "mac-universal", "GDLauncher.app")
}

export function getBinaryPath(
  platform: NodeJS.Platform = process.platform
): string {
  const rootPath = getRootPath(platform)

  if (platform === "win32") {
    return path.join(rootPath, "GDLauncher.exe")
  }

  if (platform === "linux") {
    return path.join(rootPath, "@gddesktop")
  }

  return path.join(rootPath, "Contents", "MacOS", "GDLauncher")
}

export function getCoreModulePath(
  platform: NodeJS.Platform = process.platform
): string {
  const rootPath = getRootPath(platform)

  if (platform === "win32") {
    return path.join(rootPath, "resources", "binaries", "core_module.exe")
  }

  if (platform === "linux") {
    return path.join(rootPath, "resources", "binaries", "core_module")
  }

  return path.join(rootPath, "Contents", "Resources", "binaries", "core_module")
}

export function isCoreModulePresent(): boolean {
  const corePath = getCoreModulePath()
  console.log("Core module path:", corePath)

  return fs.existsSync(corePath)
}

export interface LaunchOptions {
  runtimePath: string
  baseApi?: string
  e2eAuthBase?: string
  e2eEntitlementKey?: string
}

/**
 * Launches the packaged app against an isolated runtime path.
 *
 * Multiple instances are allowed so a suite can run while a developer's own
 * launcher is open, and Sentry is off so test crashes never reach the real
 * project.
 *
 * The returned `pageErrors` array collects every uncaught renderer exception
 * for the page's whole life: an error thrown outside Solid's error boundary
 * — in an event handler, or an unhandled rejection — never mounts
 * `#appFatalCrashState`, so nothing else would ever observe it. Collecting
 * into an array and letting the caller assert on it in a test body is the
 * pattern Playwright recommends over throwing from inside the listener,
 * which does not reliably propagate to the runner as a failed assertion.
 */
export async function launchApp(
  opts: LaunchOptions
): Promise<{ app: ElectronApplication; page: Page; pageErrors: Error[] }> {
  const binaryPath = getBinaryPath()
  const args = ["--gdl_allow_multiple_instances", "--gdl_disable_sentry"]

  if (opts.baseApi) {
    args.push("--gdl_override_base_api", opts.baseApi)
  }

  if (opts.e2eAuthBase) {
    args.push("--gdl_e2e_auth_base", opts.e2eAuthBase)
  }

  if (opts.e2eEntitlementKey) {
    args.push("--gdl_e2e_entitlement_key", opts.e2eEntitlementKey)
  }

  console.log("Launching Electron from:", binaryPath)
  console.log("Binary exists:", fs.existsSync(binaryPath))
  if (fs.existsSync(binaryPath)) {
    const stats = fs.statSync(binaryPath)
    console.log("Binary is executable:", !!(stats.mode & fs.constants.S_IXUSR))
    console.log("Binary size:", stats.size)
  }
  console.log("Args:", args.join(" "))
  console.log("Runtime path:", opts.runtimePath)
  console.log("DISPLAY:", process.env.DISPLAY)

  const app = await electron
    .launch({
      args,
      executablePath: binaryPath,
      env: {
        ...process.env,
        GDL_RUNTIME_PATH: opts.runtimePath
      } as Record<string, string>
    })
    .catch((error) => {
      console.error("=== Electron Launch Failed ===")
      console.error("Error message:", error.message)
      console.error("Error name:", error.name)
      console.error("Error stack:", error.stack)

      if (error.cause) {
        console.error("Error cause:", error.cause)
      }

      console.error(
        "Full error object:",
        JSON.stringify(error, Object.getOwnPropertyNames(error), 2)
      )

      throw error
    })

  app.on("console", (msg) => console.log(msg.text()))
  app.process().stdout?.on("data", (data) => console.log(data.toString()))
  app.process().stderr?.on("data", (data) => console.log(data.toString()))

  const page = await app.firstWindow()
  page.on("console", (msg) => console.log(msg.text()))

  const pageErrors: Error[] = []
  page.on("pageerror", (error) => {
    console.error(error)
    pageErrors.push(error)
  })

  return { app, page, pageErrors }
}

/**
 * Attaches the Rust core's session log to the Playwright report when a test
 * finished in a status other than the one it expected.
 *
 * A failure screenshot only shows the last UI frame; for a long install the
 * actual cause almost always lives in the core's own log, not the DOM. The
 * core writes one file per launch to `<runtimePath>/__gdl_logs__` (see
 * `crates/carbon_app/src/logger.rs::setup_logger`) — but only in release
 * builds, which is what the `-e2e` build variants are. A debug build creates
 * the directory but never the file, so an empty or missing log here is
 * expected outside a packaged run, not a bug in this function.
 *
 * No-ops (rather than throwing) on every "nothing to attach" case — a
 * missing log must never fail a test on top of whatever already failed it.
 */
export async function attachCoreLogOnFailure(
  testInfo: TestInfo,
  runtimePath: string
): Promise<void> {
  if (testInfo.status === testInfo.expectedStatus) return

  const logsDir = path.join(runtimePath, "__gdl_logs__")
  if (!fs.existsSync(logsDir)) return

  const logFiles = fs.readdirSync(logsDir).filter((f) => f.endsWith(".log"))
  if (logFiles.length === 0) return

  // One log file per core launch, and the worker launches the core once —
  // but picking the newest by mtime rather than assuming array order keeps
  // this correct if that ever changes.
  const newest = logFiles
    .map((name) => {
      const filePath = path.join(logsDir, name)
      return { filePath, mtimeMs: fs.statSync(filePath).mtimeMs }
    })
    .sort((a, b) => b.mtimeMs - a.mtimeMs)[0]

  if (fs.statSync(newest.filePath).size === 0) return

  await testInfo.attach("core-log", {
    path: newest.filePath,
    contentType: "text/plain"
  })
}
