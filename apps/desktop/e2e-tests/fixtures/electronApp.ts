import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { _electron as electron } from "playwright"
import type { ElectronApplication, Page } from "playwright"

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
 */
export async function launchApp(
  opts: LaunchOptions
): Promise<{ app: ElectronApplication; page: Page }> {
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

  return { app, page }
}
