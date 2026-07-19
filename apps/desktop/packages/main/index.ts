// Intentionally putting this on top to catch any potential error in dependencies as well

declare const __SHOWCASE_MODE__: boolean

console.log("Initializing application...")

process.on("uncaughtException", handleUncaughtException)

import {
  app,
  BrowserWindow,
  dialog,
  Display,
  ipcMain,
  OpenDialogOptions,
  SaveDialogOptions,
  screen,
  session,
  shell
} from "electron"
import os from "os"
import path, { join, resolve } from "path"
import fs from "fs/promises"
import fss from "fs"
import fse, { ensureDirSync } from "fs-extra"
import fg from "fast-glob"
import type { ChildProcessWithoutNullStreams } from "child_process"
import { spawn } from "child_process"
import crypto from "crypto"
import log from "electron-log/main"
import { hashEmailForOverwolf } from "./utils/emailHash"
import * as Sentry from "@sentry/electron/main"
import "./preloadListeners"
import getAdSize from "./adSize"
import handleUncaughtException from "./handleUncaughtException"
import initAutoUpdater from "./autoUpdater"
import "./appMenu"
import {
  CoreModuleStatus,
  FELauncherActionOnGameLaunch
} from "@gd/core_module/bindings"

console.log("Modules imported successfully")

let overwolfReady = false
let pendingEmail: string | null | undefined = null

function setOverwolfEmail(email: string) {
  if (overwolfReady && (app as any).overwolf) {
    try {
      const hashes = hashEmailForOverwolf(email)
      ;(app as any).overwolf.setUserEmailHashes(hashes)
      console.log("GDL account email hashes sent to Overwolf")
    } catch (error) {
      console.error("Failed to set email hashes:", error)
    }
  } else {
    pendingEmail = email
  }
}

function clearOverwolfEmail() {
  if (overwolfReady && (app as any).overwolf) {
    try {
      ;(app as any).overwolf.setUserEmailHashes({})
      console.log("GDL account email hashes cleared")
    } catch (error) {
      console.error("Failed to clear email hashes:", error)
    }
  } else {
    pendingEmail = null
  }
}

function applyPendingEmail() {
  if (pendingEmail) {
    setOverwolfEmail(pendingEmail)
  } else if (pendingEmail === null) {
    clearOverwolfEmail()
  }
  pendingEmail = undefined
}

export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override"
const RUNTIME_PATH_DEFAULT_NAME = "data"

export let CURRENT_RUNTIME_PATH: string | null = null

let win: BrowserWindow | null = null
let lastCoreModuleProgress: number | null = null

const getWin = () => {
  return win
}

let isGameRunning = false
let showAppCloseWarning = true

app.enableSandbox()
app.commandLine.appendSwitch("proxy-bypass-list", "127.0.0.1,localhost")

export function initRTPath(override: string | null | undefined) {
  console.log("Initializing runtime path...")
  if (override) {
    CURRENT_RUNTIME_PATH = override
    console.log("Runtime path overridden:", CURRENT_RUNTIME_PATH)
    return
  }

  const runtimeOverridePath = path.join(
    app.getPath("userData"),
    RUNTIME_PATH_OVERRIDE_NAME
  )

  let file_override: string | null = null
  try {
    const tmp_path = fss.readFileSync(runtimeOverridePath).toString()
    fse.ensureDirSync(tmp_path)
    file_override = tmp_path
    console.log("Runtime path read from file:", file_override)
  } catch {
    console.log("No runtime path override file found")
  }

  CURRENT_RUNTIME_PATH =
    file_override ||
    path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME)
  console.log("Current runtime path set to:", CURRENT_RUNTIME_PATH)
}

const args = process.argv.slice(1)
console.log("Process arguments:", args)

interface Argument {
  argument: string
  value: string | null
}

function validateArgument(arg: string): Argument | null {
  console.log("Validating argument:", arg)
  const hasValue =
    args.includes(arg) && !args[args.indexOf(arg) + 1]?.startsWith("--")

  if (hasValue) {
    console.log("Argument has value:", arg, args[args.indexOf(arg) + 1])
    return {
      argument: arg,
      value: args[args.indexOf(arg) + 1]
    }
  }

  if (args.includes(arg)) {
    console.log("Argument found without value:", arg)
    return {
      argument: arg,
      value: null
    }
  }

  console.log("Argument not found:", arg)
  return null
}

export function getPatchedUserData() {
  console.log("Getting patched user data...")
  const isSnapshot = __APP_VERSION__.includes("snapshot")
  if (app.isPackaged && isSnapshot) {
    const isDeepBinary = app
      .getPath("exe")
      .endsWith("Contents/MacOS/GDLauncher")
    const isMacOS = process.platform === "darwin"
    const appPackagePath = path.resolve(
      app.getPath("exe"),
      // MacOS .app are compressed folders, the actual executable is in Contents/MacOS/[Binary]
      // but depending on whether you double-click the .app or run it from the terminal,
      // the path will be different
      isMacOS && isDeepBinary ? "../../../../" : "../",
      "gdl_data"
    )

    ensureDirSync(appPackagePath)
    console.log("App package path for snapshot:", appPackagePath)

    return appPackagePath
  }

  let appData = null

  if (os.platform() === "darwin" || os.platform() === "win32") {
    appData = app.getPath("appData")
  } else {
    // monkey patch linux since it defaults to .config instead of .local/share
    const xdgDataHome = process.env.XDG_DATA_HOME
    if (xdgDataHome) {
      appData = xdgDataHome
    } else {
      const homeDir = os.homedir()
      appData = path.join(homeDir, ".local/share")
    }
  }

  console.log("App data path:", appData)
  return path.join(appData, "gdlauncher_carbon")
}

const patchedUserData = getPatchedUserData()

const skipIntroAnimation = fss.existsSync(patchedUserData)
console.log("Skip intro animation:", skipIntroAnimation)

app.setPath("userData", patchedUserData)
console.log("User data path set to:", app.getPath("userData"))

log.transports.file.resolvePathFn = (variables) =>
  path.join(patchedUserData, variables.fileName!)
log.initialize()
log.eventLogger.startLogging()
console.log("Logging initialized")
Object.assign(console, log.functions)

if (app.isPackaged) {
  const overrideCLIDataPath = validateArgument("--runtime_path")
  const overrideEnvDataPath = process.env.GDL_RUNTIME_PATH

  initRTPath(overrideCLIDataPath?.value || overrideEnvDataPath)
} else {
  const rtPath = import.meta.env.RUNTIME_PATH
  if (!rtPath) {
    throw new Error("Missing runtime path")
  }
  initRTPath(rtPath)
}

console.log("Userdata path:", patchedUserData)
console.log("Runtime path:", CURRENT_RUNTIME_PATH)

const sentrySessionId = crypto.randomUUID()

console.log("SENTRY SESSION ID", sentrySessionId)

const allowMultipleInstances = validateArgument(
  "--gdl_allow_multiple_instances"
)

const overrideBaseApi = validateArgument("--gdl_override_base_api")

if (!allowMultipleInstances) {
  if (!app.requestSingleInstanceLock()) {
    console.log("Another instance is already running. Quitting...")
    app.quit()
    process.exit(0)
  }
}

const disableSentry = validateArgument("--gdl_disable_sentry")

if (!disableSentry) {
  if (import.meta.env.VITE_MAIN_DSN) {
    process.removeListener("uncaughtException", handleUncaughtException)

    Sentry.init({
      dsn: import.meta.env.VITE_MAIN_DSN,
      release: __APP_VERSION__,
      dist: os.platform()
    })

    Sentry.setContext("session", {
      gdl_session_id: sentrySessionId
    })
    console.log("Sentry initialized")
  }
}

export interface Log {
  type: "info" | "error"
  message: string
}

const isDev = import.meta.env.MODE === "development"
console.log("Is development mode:", isDev)

const binaryName = os.platform() === "win32" ? "core_module.exe" : "core_module"
console.log("Binary name:", binaryName)

export type CoreModule = () => Promise<
  | {
      type: "success"
      result: {
        port: number
        apiToken: string
        kill: () => void
      }
    }
  | {
      type: "error"
      logs: Log[]
    }
  | {
      type: "backwardsMigration"
    }
>

// Must match DEV_API_TOKEN in crates/carbon_app/src/main.rs.
// Debug builds of the rust core accept this fixed token; release builds
// rotate randomly per launch.
const DEV_API_TOKEN = "dev-mode-only-do-not-use-in-production"

const loadCoreModule: CoreModule = () =>
  new Promise((resolve, _) => {
    console.log("Loading core module...")
    if (isDev) {
      resolve({
        type: "success",
        result: {
          port: 4650,
          apiToken: DEV_API_TOKEN,
          kill: () => {}
        }
      })
      console.log("Core module loaded in development mode")
      return
    }

    let started = false

    const coreModulePath = path.resolve(
      __dirname,
      "../../../../resources/binaries",
      binaryName
    )

    console.log(`[CORE] Spawning core module: ${coreModulePath}`)
    let coreModule: ChildProcessWithoutNullStreams | null = null
    const logs: Log[] = []

    const args = ["--runtime_path", CURRENT_RUNTIME_PATH!]

    if (overrideBaseApi?.value) {
      args.push("--base_api", overrideBaseApi.value)
    }

    try {
      coreModule = spawn(coreModulePath, args, {
        shell: false,
        detached: false,
        stdio: "pipe",
        env: {
          ...process.env,
          RUST_BACKTRACE: "full"
        }
      })
      console.log("Core module spawned successfully")
    } catch (err: unknown) {
      console.error(`[CORE] Spawn error: ${String(err)}`)

      logs.push({
        type: "error",
        message: String(err)
      })

      resolve({
        type: "error",
        logs
      })

      return
    }

    coreModule.on("error", function (err) {
      console.error(`[CORE] Spawn error: ${err}`)

      logs.push({
        type: "error",
        message: err.toString()
      })

      resolve({
        type: "error",
        logs
      })

      return
    })

    coreModule.stdout.on("data", (data) => {
      const dataString = data.toString()

      // Strip sensitive payloads before lines reach the user-facing buffer
      // or main.log:
      //   - `_STATUS_:READY|<port>|<api-token>` — the core API token. The
      //     user can already read it from disk, but log dumps shared via
      //     Discord / GitHub should not leak it.
      //   - `_GDL_ACCOUNT_EMAIL_:<email>` — the signed-in user's email,
      //     forwarded to Overwolf for ad personalisation. PII; must not
      //     end up in shared support logs.
      // IPC parsing below uses the raw `dataString`, so redaction here only
      // affects what gets persisted/displayed.
      const sanitized = dataString
        .replace(/(_STATUS_:READY\|\d+\|)[^\s|]+/g, "$1<redacted>")
        .replace(/(_GDL_ACCOUNT_EMAIL_:)[^\s\r\n]+/g, "$1<redacted>")

      console.log(`[CORE] Message: ${sanitized}`)

      const rows = dataString.split(/\r?\n|\r|\n/g)

      logs.push({
        type: "info",
        message: sanitized
      })

      for (const row of rows) {
        if (row.startsWith("_STATUS_:")) {
          const rightPart = row.split(":")[1]
          const parts = rightPart.split("|")
          const event = parts[0]
          const port: number = parts[1] as unknown as number
          const apiToken: string | undefined = parts[2]
          console.log(`[CORE] Event: ${event}, Port: ${port}`)

          if (event === "READY") {
            if (!apiToken) {
              console.error("[CORE] _STATUS_:READY missing api token")
              resolve({
                type: "error",
                logs: [
                  ...logs,
                  {
                    type: "error",
                    message: "Core module did not provide an api token"
                  }
                ]
              })
              return
            }
            started = true
            resolve({
              type: "success",
              result: {
                port,
                apiToken,
                kill: () => coreModule?.kill()
              }
            })
          } else if (event === "BACKWARDS_MIGRATION") {
            console.log("[CORE] Backwards migration detected")
            resolve({
              type: "backwardsMigration"
            })
          } else {
            let progress = 0
            switch (event as CoreModuleStatus) {
              case "VerifyingTermsAndPrivacy":
                progress = 10
                break
              case "LoadAndMigrate":
                progress = 20
                break
              case "RefreshMSAuth":
                progress = 35
                break
              case "XboxAuth":
                progress = 50
                break
              case "McLogin":
                progress = 65
                break
              case "MCEntitlements":
                progress = 77
                break
              case "McProfile":
                progress = 88
                break
              case "AccountRefreshComplete":
                progress = 95
                break
              case "LaunchBackgroundTasks":
                progress = 100
                break
            }

            lastCoreModuleProgress = progress
            const w = getWin()
            if (w && !w.isDestroyed()) {
              w.webContents.send("coreModuleProgress", progress)
            }
          }
        } else if (row.startsWith("_INSTANCE_STATE_:")) {
          const rightPart = row.split(":")[1]
          const event = rightPart.split("|")[0]
          const action: FELauncherActionOnGameLaunch = rightPart.split("|")[1]

          if (event === "GAME_LAUNCHED") {
            isGameRunning = true
            console.log("Game launched, action:", action)
            switch (action) {
              case "closeWindow":
                if (win && !win.isDestroyed()) {
                  win.close()
                }
                win = null
                break
              case "hideWindow":
                if (win && !win.isDestroyed()) {
                  win.hide()
                }
                break
              case "minimizeWindow":
                if (win && !win.isDestroyed()) {
                  win.minimize()
                }
                break
              case "none":
                break
              case "quitApp":
                showAppCloseWarning = false
                app.quit()
                break
            }
          } else if (event === "GAME_CLOSED") {
            isGameRunning = false
            console.log("Game closed, action:", action)
            switch (action) {
              case "closeWindow":
                if (!win || win.isDestroyed()) {
                  createWindow()
                }
                break
              case "hideWindow":
              case "minimizeWindow":
                if (win && !win.isDestroyed()) {
                  win?.show()
                  win?.focus()
                } else {
                  createWindow()
                }
                break
              case "none":
                break
              case "quitApp":
                // There's nothing we can do
                break
            }
          }
        } else if (row.startsWith("_SHOW_APP_CLOSE_WARNING_:")) {
          const rightPart = row.split(":")[1]
          showAppCloseWarning = rightPart === "true"
          console.log("Show app close warning:", showAppCloseWarning)
        } else if (row.startsWith("_GDL_ACCOUNT_EMAIL_:")) {
          const email = row.split(":")[1]?.trim() || ""
          if (email) {
            setOverwolfEmail(email)
          } else {
            clearOverwolfEmail()
          }
        }
      }
    })

    coreModule.stderr.on("data", (data) => {
      logs.push({
        type: "error",
        message: data.toString()
      })
      console.error(`[CORE] Error: ${data.toString()}`)
    })

    coreModule.on("exit", (code) => {
      console.log(`[CORE] Exit with code: ${code}`)

      // If we get here without `started` being true, the core module exited
      // before emitting `_STATUS_:READY`. That's always an error condition,
      // even if the exit code is 0.
      if (started) {
        return
      }
      resolve({
        type: "error",
        logs: [
          ...logs,
          {
            type: "error",
            message: `Core module exited unexpectedly with code ${code} before READY`
          }
        ]
      })
    })

    setTimeout(
      () => {
        if (coreModule?.killed || started) {
          return
        }

        console.error(`[CORE] Took too long to start`)

        Sentry.captureException(new Error("Core module took too long to start"))

        resolve({
          type: "error",
          logs
        })
      },
      60 * 5 * 1000
    )
  })

const coreModule = loadCoreModule()

if ((app as any).overwolf) {
  ;(app as any).overwolf.disableAnonymousAnalytics()
  console.log("Overwolf anonymous analytics disabled")
}

// Set application name for Windows 10+ notifications
if (process.platform === "win32") app.setAppUserModelId(app.getName())

// Register protocol handlers for gdlauncher, curseforge, and modrinth
const deepLinkProtocols = ["gdlauncher", "curseforge", "modrinth"]
for (const deepLinkProtocol of deepLinkProtocols) {
  if (process.defaultApp) {
    if (process.argv.length >= 2) {
      app.setAsDefaultProtocolClient(deepLinkProtocol, process.execPath, [
        resolve(process.argv[1])
      ])
    }
  } else {
    app.setAsDefaultProtocolClient(deepLinkProtocol)
  }
}

let lastDisplay: Display | null = null
let pendingDisplayChange = false

let isSpawningWindow = false

// Queue for protocol URLs received before window is ready
let pendingProtocolUrl: string | null = null

// Helper to check if a URL is a supported protocol
const isSupportedProtocol = (url: string) =>
  url.startsWith("gdlauncher://") ||
  url.startsWith("curseforge://") ||
  url.startsWith("modrinth://")

// On Windows/Linux cold-start, the protocol URL arrives in process.argv.
// macOS uses `open-url` instead; `second-instance` covers already-running launches.
const initialProtocolUrl = process.argv.find(isSupportedProtocol)
if (initialProtocolUrl) {
  console.log("Protocol URL received via process.argv:", initialProtocolUrl)
  pendingProtocolUrl = initialProtocolUrl
}

async function createWindow(): Promise<BrowserWindow> {
  console.log("Creating window...")
  if (isSpawningWindow) {
    console.log("Window is already being spawned")
    return win!
  }

  isSpawningWindow = true

  const currentDisplay = screen.getPrimaryDisplay()
  lastDisplay = currentDisplay
  const { minWidth, minHeight, height, width } = getAdSize(currentDisplay)

  if (!win || win.isDestroyed()) {
    win?.close()
    win?.destroy()
    win = null
  }

  win = new BrowserWindow({
    title: "GDLauncher Carbon",
    minHeight,
    height,
    minWidth,
    width,
    titleBarStyle: "default",
    frame: true,
    show: true,
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, "../preload/index.cjs"),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: app.isPackaged,
      webSecurity: true,
      additionalArguments: [`--skip-intro-animation=${skipIntroAnimation}`]
    }
  })

  win.on("closed", () => {
    win = null
    pendingDisplayChange = false
  })

  const applyAdLayoutForCurrentDisplay = () => {
    pendingDisplayChange = false

    const bounds = win?.getBounds()
    if (!bounds) {
      return
    }

    const display = screen.getDisplayMatching(bounds)
    const { minWidth, minHeight, adSize, bannerAdSize, hideAdText } =
      getAdSize(display)

    win?.setMinimumSize(minWidth, minHeight)

    // Grow to satisfy the new minimums and stay within the display, but never
    // shrink a window the user made larger
    const workArea = display.workArea
    const targetWidth = Math.min(
      Math.max(bounds.width, minWidth),
      workArea.width
    )
    const targetHeight = Math.min(
      Math.max(bounds.height, minHeight),
      workArea.height
    )
    if (targetWidth !== bounds.width || targetHeight !== bounds.height) {
      win?.setSize(targetWidth, targetHeight)
    }

    win?.webContents?.send("adSizeChanged", {
      adSize,
      bannerAdSize,
      hideAdText
    })
  }

  win.on("move", () => {
    const bounds = win?.getBounds()

    if (!bounds) {
      return
    }

    const currentDisplay = screen.getDisplayMatching(bounds)
    if (lastDisplay?.id !== currentDisplay?.id) {
      lastDisplay = currentDisplay
      pendingDisplayChange = true

      // Linux never emits `moved`, so apply immediately there
      if (process.platform === "linux") {
        applyAdLayoutForCurrentDisplay()
      }
    }
  })

  // Resizing while the user is still dragging fights the OS drag loop and makes
  // the window snap to the seam between monitors. `moved` fires once when the
  // interactive move ends on Windows (on macOS it's an alias of `move`), so the
  // new ad layout is applied only after the drag is released.
  win.on("moved", () => {
    if (pendingDisplayChange) {
      applyAdLayoutForCurrentDisplay()
    }
  })

  win.on("close", (e) => {
    if (!isGameRunning) {
      return
    }

    if (showAppCloseWarning) {
      e.preventDefault()
      win?.webContents.send("showAppCloseWarning")
    }
  })

  win.webContents.on("will-navigate", (e, url) => {
    if (win && !win.isDestroyed() && url !== win.webContents.getURL()) {
      e.preventDefault()
      try {
        const parsed = new URL(url)
        if (
          parsed.protocol === "http:" ||
          parsed.protocol === "https:" ||
          parsed.protocol === "mailto:"
        ) {
          shell.openExternal(url)
        } else {
          console.warn(
            "[will-navigate] blocked navigation to unsafe scheme:",
            url
          )
        }
      } catch {
        console.warn("[will-navigate] blocked invalid URL:", url)
      }
    }
  })

  if (app.isPackaged) {
    win.loadFile(join(__dirname, "../mainWindow/index.html"))
  } else {
    const url = `http://${import.meta.env.VITE_DEV_SERVER_HOST}:${
      import.meta.env.VITE_DEV_MAIN_WINDOW_PORT
    }`

    win.loadURL(url, {
      userAgent: "GDLauncher Carbon"
    })
  }

  win.webContents.on("before-input-event", (event, input) => {
    if (input.alt && input.shift && input.code === "KeyI") {
      event.preventDefault()
      win?.webContents.toggleDevTools()
    }
  })

  win.on("ready-to-show", () => {
    isSpawningWindow = false
    console.log("Window is ready to show")

    // Send any pending protocol URL that was received before window was ready
    if (pendingProtocolUrl) {
      console.log("Sending pending protocol URL:", pendingProtocolUrl)
      // Small delay to ensure renderer is fully initialized
      setTimeout(() => {
        win?.webContents.send("protocol-url", pendingProtocolUrl)
        pendingProtocolUrl = null
      }, 500)
    }

    function upsertKeyValue(obj: any, keyToChange: string, value: any) {
      const keyToChangeLower = keyToChange.toLowerCase()
      for (const key of Object.keys(obj)) {
        if (key.toLowerCase() === keyToChangeLower) {
          return
        }
      }
      // Insert at end instead
      obj[keyToChange] = value
    }

    win?.webContents.session.webRequest.onBeforeSendHeaders(
      (details, callback) => {
        const { requestHeaders } = details
        upsertKeyValue(requestHeaders, "Access-Control-Allow-Origin", ["*"])
        callback({ requestHeaders })
      }
    )

    win?.webContents.session.webRequest.onHeadersReceived(
      (details, callback) => {
        const { responseHeaders } = details
        upsertKeyValue(responseHeaders, "Access-Control-Allow-Origin", ["*"])
        upsertKeyValue(responseHeaders, "Access-Control-Allow-Headers", ["*"])
        callback({
          responseHeaders
        })
      }
    )

    if (import.meta.env.DEV && !__SHOWCASE_MODE__) {
      win?.webContents.openDevTools()
    }
  })

  // Make all links open with the browser, not with the application
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith("https:")) shell.openExternal(url)
    return { action: "deny" }
  })

  return win
}

// Handlers
ipcMain.handle("relaunch", async () => {
  console.log("relaunching app...")

  try {
    const _coreModule = await coreModule
    if (_coreModule.type === "success") {
      _coreModule.result.kill()
    }
  } catch {
    // No op
  }

  app.relaunch()
  app.exit()
})

ipcMain.handle("deleteDbAndRestart", async () => {
  console.log("deleting database and restarting app...")

  const dbPath = path.join(CURRENT_RUNTIME_PATH!, "gdl_conf.db")

  try {
    await fs.unlink(dbPath)
    console.log("database deleted successfully")
  } catch {
    // File might not exist, that's ok
    console.log("database file not found or already deleted")
  }

  try {
    const _coreModule = await coreModule
    if (_coreModule.type === "success") {
      _coreModule.result.kill()
    }
  } catch {
    // No op
  }

  app.relaunch()
  app.exit()
})

ipcMain.handle("getAdSize", async () => {
  const currentDisplay = screen.getDisplayMatching(win?.getBounds()!)
  const { adSize, bannerAdSize, hideAdText } = getAdSize(currentDisplay)
  return { adSize, bannerAdSize, hideAdText }
})

ipcMain.handle("openFileDialog", async (_, opts: OpenDialogOptions) => {
  return dialog.showOpenDialog(opts)
})

ipcMain.handle("showSaveDialog", async (_, opts: SaveDialogOptions) => {
  return dialog.showSaveDialog(opts)
})

ipcMain.handle("getCurrentOS", async () => {
  const platform = os.platform()
  const arch = os.arch()

  // Determine if this build supports auto-updates
  // electron-updater only works with:
  // - Windows: NSIS installer (not portable ZIP)
  // - Linux: AppImage (not other formats)
  // - macOS: DMG and ZIP distributions
  let supportsAutoUpdate = false

  if (platform === "win32") {
    // On Windows, NSIS builds support auto-update
    // Portable builds (ZIP) do not
    // We can detect this by checking if we're installed or portable
    supportsAutoUpdate = app.isPackaged && !process.env.PORTABLE_EXECUTABLE_DIR
  } else if (platform === "linux") {
    // On Linux, only AppImage supports auto-update
    // AppImage sets the APPIMAGE environment variable
    supportsAutoUpdate = !!process.env.APPIMAGE
  } else if (platform === "darwin") {
    // On macOS, packaged builds (DMG/ZIP) support auto-update
    supportsAutoUpdate = app.isPackaged
  }

  return {
    platform,
    arch,
    supportsAutoUpdate
  }
})

ipcMain.handle("openFolder", async (_, path) => {
  shell.showItemInFolder(path)
})

ipcMain.handle("openCMPWindow", async () => {
  if ((app as any).overwolf?.openCMPWindow) {
    ;(app as any).overwolf.openCMPWindow()
    return true
  }
  return false
})

ipcMain.handle("isCMPWindowAvailable", async () => {
  // Availability means "can the CMP window open at all" (i.e. an ow-electron
  // build with the overwolf API injected). Deliberately NOT `isCMPRequired()`:
  // that is country-dependent, and users outside CMP-required regions must
  // still be able to manage their consent.
  return !!(app as any).overwolf?.openCMPWindow
})

ipcMain.handle("closeWindow", async () => {
  win?.close()
  win?.destroy()
})

ipcMain.handle("getUserData", async () => {
  return app.getPath("userData")
})

ipcMain.handle("getInitialRuntimePath", async () => {
  return path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME)
})

ipcMain.handle("getRuntimePath", async () => {
  return CURRENT_RUNTIME_PATH
})

ipcMain.handle(
  "changeRuntimePath",
  async (_, newPath: string, switchOnly = false) => {
    interface Progress {
      action: "scan" | "copy" | "remove"
      currentName: string
      current: number
      total: number
    }

    console.log(
      `[RTP] Migration request: ${CURRENT_RUNTIME_PATH} -> ${newPath} (switchOnly=${switchOnly})`
    )

    if (newPath === CURRENT_RUNTIME_PATH) {
      console.log(`[RTP] No-op: same path`)
      return
    }

    const runtimeOverridePath = path.join(
      app.getPath("userData"),
      RUNTIME_PATH_OVERRIDE_NAME
    )

    if (switchOnly) {
      // Switch-only mode: the renderer already detected the dir exists with
      // user data via validateRuntimePath. Don't mkdir — that would silently
      // recreate the dir empty if it was deleted between confirm and now,
      // resulting in a successful "switch" to an empty runtime.
      if (!(await fse.pathExists(newPath))) {
        console.error(`[RTP] Switch-only: target no longer exists: ${newPath}`)
        throw new Error(
          `Target directory no longer exists: ${newPath}. Aborting switch.`
        )
      }
    } else {
      try {
        await fs.mkdir(newPath, { recursive: true })
        console.log(`[RTP] Destination ready`)
      } catch (e) {
        console.error(`[RTP] Failed to create destination:`, e)
        throw e
      }
    }

    try {
      const cm = await coreModule
      if (cm.type === "success") {
        console.log(`[RTP] Killing core module`)
        cm.result.kill()
        // Give the OS a moment to release file handles (SQLite WAL, logs)
        await new Promise((r) => setTimeout(r, 1500))
      }
    } catch {
      // No op
    }

    // Switch-only path: the target dir already contains the user's data and
    // they want to use it as-is. Don't touch any files in either dir; just
    // update the override pointer and relaunch. The previous runtime data
    // remains as orphan disk space the user can delete manually.
    if (switchOnly) {
      console.log(`[RTP] Switch-only: writing override to point at ${newPath}`)
      await fse.writeFile(runtimeOverridePath, newPath)
      console.log(`[RTP] Switch complete, relaunching`)
      app.relaunch()
      app.exit()
      return
    }

    // Surface activity to the renderer immediately so the user doesn't
    // see only the spinner during the (potentially slow) glob scan.
    win?.webContents.send("changeRuntimePathProgress", {
      action: "scan",
      currentName: "",
      current: 0,
      total: 0
    } satisfies Progress)

    console.log(`[RTP] Scanning source files...`)
    const t0 = Date.now()
    const files = await fg("**/*", {
      cwd: CURRENT_RUNTIME_PATH!,
      onlyFiles: true,
      dot: true,
      followSymbolicLinks: false,
      suppressErrors: true,
      ignore: ["**/.DS_Store", RUNTIME_PATH_OVERRIDE_NAME]
    })
    console.log(`[RTP] Scan: ${files.length} files in ${Date.now() - t0}ms`)

    const total = files.length

    // Drop a marker so a future startup can detect an interrupted migration if
    // the host process crashes (we still won't have written the override file
    // in that case).
    const migrationMarker = path.join(newPath, ".gdl_migration_in_progress")
    try {
      await fse.writeFile(migrationMarker, "")
    } catch (e) {
      console.warn(`[RTP] Failed to write migration marker:`, e)
    }

    // Track which files we successfully copied, so a rollback only removes
    // files we created — not pre-existing content the user had in newPath.
    const copiedFiles: string[] = []
    const cleanupPartial = async () => {
      console.log(
        `[RTP] Rolling back ${copiedFiles.length} files copied to ${newPath}`
      )
      for (const f of copiedFiles) {
        try {
          await fse.remove(path.join(newPath, f))
        } catch {
          // best-effort
        }
      }
      try {
        await fse.remove(migrationMarker)
      } catch {
        // best-effort
      }
    }

    for (let i = 0; i < total; i++) {
      const file = files[i]
      const dest = path.join(newPath, file)

      win?.webContents.send("changeRuntimePathProgress", {
        action: "copy",
        currentName: path.basename(file),
        current: i,
        total: total * 2
      } satisfies Progress)

      // Skip files that already exist with the same content — we don't want to
      // touch (and later potentially roll back) files the user already had.
      const destExisted = await fse.pathExists(dest)

      try {
        await fse.copy(path.join(CURRENT_RUNTIME_PATH!, file), dest, {
          overwrite: true,
          errorOnExist: false,
          recursive: true
        })
        // Only track for rollback if we actually created the destination —
        // overwriting a pre-existing file is destructive and we don't try to
        // recover those.
        if (!destExisted) {
          copiedFiles.push(file)
        }
      } catch (e) {
        console.error(`[RTP] Failed to copy ${file}:`, e)
        await cleanupPartial()
        throw new Error(`Failed to copy ${file}: ${(e as Error).message}`)
      }
    }
    console.log(`[RTP] Copy complete, writing override`)

    try {
      await fse.writeFile(runtimeOverridePath, newPath)
    } catch (e) {
      // Override write failed *after* we copied everything — old path is still
      // authoritative; new path is duplicated data. Clean up the new path.
      console.error(`[RTP] Failed to write override file, rolling back:`, e)
      await cleanupPartial()
      throw e
    }

    // Clear marker — past this point, newPath is the authoritative location.
    try {
      await fse.remove(migrationMarker)
    } catch {
      // best-effort
    }

    console.log(`[RTP] Removing source files`)
    for (let i = 0; i < total; i++) {
      const file = files[i]

      win?.webContents.send("changeRuntimePathProgress", {
        action: "remove",
        currentName: path.basename(file),
        current: total + i,
        total: total * 2
      } satisfies Progress)

      // Don't fail the migration if a single remove fails — data is already
      // safely in the new path and the override file points at it.
      try {
        await fse.remove(path.join(CURRENT_RUNTIME_PATH!, file))
      } catch (e) {
        console.error(`[RTP] Failed to remove ${file}:`, e)
      }
    }

    console.log(`[RTP] Migration complete, relaunching`)
    app.relaunch()
    app.exit()
  }
)

ipcMain.handle("validateRuntimePath", async (_, newPath: string | null) => {
  if (!newPath || newPath === CURRENT_RUNTIME_PATH) {
    return "invalid"
  }

  const pathExists = await fse.pathExists(newPath)
  if (!pathExists) {
    return "valid"
  }

  const newPathStat = await fs.stat(newPath)
  if (!newPathStat.isDirectory()) {
    return "invalid"
  }

  const files = await fs.readdir(newPath)
  if (files.length > 0) {
    return "potentially_valid"
  }

  return "valid"
})

ipcMain.handle("getCoreModule", async () => {
  // we can assume this promise never rejects
  const cm = await coreModule

  return {
    type: cm.type,
    logs: cm.type === "error" ? cm.logs : undefined,
    port: cm.type === "success" ? cm.result.port : undefined,
    apiToken: cm.type === "success" ? cm.result.apiToken : undefined
  }
})

ipcMain.handle("getCurrentProgress", () => {
  return lastCoreModuleProgress
})

app.whenReady().then(async () => {
  console.log("App is ready")
  const accessibility = validateArgument("--enable-accessibility")

  if (accessibility) {
    app.setAccessibilitySupportEnabled(true)
    console.log("Accessibility support enabled")
  }

  console.log("OVERWOLF APP ID", process.env.OVERWOLF_APP_UID)

  // Mark Overwolf as ready and apply any pending email
  if ((app as any).overwolf && process.env.OVERWOLF_APP_UID) {
    overwolfReady = true
    applyPendingEmail()
  }

  session.defaultSession.webRequest.onBeforeSendHeaders(
    {
      urls: ["http://*/*", "https://*/*"]
    },
    (details, callback) => {
      details.requestHeaders.Origin = "https://app.gdlauncher.com"
      callback({ requestHeaders: details.requestHeaders })
    }
  )

  session.defaultSession.webRequest.onHeadersReceived(
    {
      urls: ["http://*/*", "https://*/*"]
    },
    (details, callback) => {
      delete details.responseHeaders!["Access-Control-Allow-Origin"]

      delete details.responseHeaders!["access-control-allow-origin"]
      details.responseHeaders!["Access-Control-Allow-Origin"] = ["*"]

      // Remove X-Frame-Options and CSP frame-ancestors for iframe-embeddable content
      // This allows YouTube and other embeds to work when loaded from file:// origin
      const url = details.url.toLowerCase()
      const isEmbeddableContent =
        url.includes("youtube.com") ||
        url.includes("youtube-nocookie.com") ||
        url.includes("googlevideo.com") || // YouTube video CDN
        url.includes("i.imgur.com") ||
        url.includes("cdn.ko-fi.com")

      if (isEmbeddableContent) {
        // Remove X-Frame-Options header (case-insensitive)
        delete details.responseHeaders!["X-Frame-Options"]
        delete details.responseHeaders!["x-frame-options"]

        // Remove or modify Content-Security-Policy frame-ancestors
        // Note: CSP can have multiple header names
        const cspKeys = Object.keys(details.responseHeaders!).filter(
          (key) =>
            key.toLowerCase() === "content-security-policy" ||
            key.toLowerCase() === "content-security-policy-report-only"
        )
        for (const key of cspKeys) {
          const values = details.responseHeaders![key]
          if (values) {
            // Remove frame-ancestors directive from CSP
            details.responseHeaders![key] = values.map((value) =>
              value.replace(/frame-ancestors\s+[^;]+;?/gi, "")
            )
          }
        }
      }

      callback({
        cancel: false,
        responseHeaders: details.responseHeaders
      })
    }
  )

  app.on("second-instance", (_e, argv) => {
    // Handle protocol URLs on Windows (passed as command line arguments)
    const protocolUrl = argv.find((arg) => isSupportedProtocol(arg))

    if (win && !win.isDestroyed()) {
      // Focus on the main window if the user tried to open another
      if (win.isMinimized()) win.restore()
      win.focus()

      // Forward protocol URL if present
      if (protocolUrl) {
        console.log("Protocol URL received via second-instance:", protocolUrl)
        win.webContents.send("protocol-url", protocolUrl)
      }
    } else {
      // Store URL and create window
      if (protocolUrl) {
        pendingProtocolUrl = protocolUrl
      }
      createWindow()
    }
  })

  app.on("activate", () => {
    if (!win || win.isDestroyed()) {
      createWindow()
    }
  })

  await createWindow()

  screen.addListener(
    "display-metrics-changed",
    (_, display, changedMetrics) => {
      const bounds = win?.getBounds()

      if (!bounds) {
        return
      }

      const currentDisplay = screen.getDisplayMatching(bounds)
      if (lastDisplay?.id === currentDisplay?.id) {
        return
      }

      lastDisplay = currentDisplay

      const { minWidth, minHeight, adSize, bannerAdSize, hideAdText } =
        getAdSize(currentDisplay)
      if (changedMetrics.includes("workArea")) {
        win?.setMinimumSize(minWidth, minHeight)
        win?.setSize(minWidth, minHeight)
        win?.webContents.send("adSizeChanged", {
          adSize,
          bannerAdSize,
          hideAdText
        })
      }
    }
  )

  initAutoUpdater()
})

app.on("window-all-closed", async () => {
  if (isSpawningWindow) {
    return
  }

  try {
    const _coreModule = await coreModule
    if (_coreModule.type === "success") {
      _coreModule.result.kill()
    }
  } catch {
    // No op
  }

  if (win && !win.isDestroyed()) {
    win.close()
    win.destroy()
  }

  win = null
  app.quit()
})

app.on("before-quit", async () => {
  try {
    const _coreModule = await coreModule
    if (_coreModule.type === "success") {
      _coreModule.result.kill()
    }
  } catch {
    // No op
  }
})

app.on("render-process-gone", (event, webContents, detailed) => {
  console.error("render-process-gone", detailed)
  webContents.reload()
})

app.on("open-url", (event, url) => {
  console.log("Protocol URL received:", url)

  // Handle gdlauncher://, curseforge://, and modrinth:// protocol URLs
  if (isSupportedProtocol(url)) {
    event.preventDefault()

    // Focus the window if minimized
    if (win && !win.isDestroyed()) {
      if (win.isMinimized()) win.restore()
      win.focus()

      // Forward the protocol URL to the renderer process
      win.webContents.send("protocol-url", url)
    } else {
      // Window not ready yet, store the URL for later
      console.log("Window not ready, storing protocol URL for later")
      pendingProtocolUrl = url
    }
  }
})
