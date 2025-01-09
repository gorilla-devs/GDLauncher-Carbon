// Intentionally putting this on top to catch any potential error in dependencies as well

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
import os, { platform, release } from "os"
import path, { join, resolve } from "path"
import fs from "fs/promises"
import fss from "fs"
import fse, { ensureDirSync } from "fs-extra"
import { glob } from "glob"
import type { ChildProcessWithoutNullStreams } from "child_process"
import { spawn } from "child_process"
import crypto from "crypto"
import log from "electron-log/main"
import * as Sentry from "@sentry/electron/main"
import "./preloadListeners"
import getAdSize from "./adSize"
import handleUncaughtException from "./handleUncaughtException"
import initAutoUpdater from "./autoUpdater"
import "./appMenu"
import { FELauncherActionOnGameLaunch } from "@gd/core_module/bindings"

console.log("Modules imported successfully")

const timeStart = Date.now()
let isPotatoPcModeSet = false

export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override"
const RUNTIME_PATH_DEFAULT_NAME = "data"

export let CURRENT_RUNTIME_PATH: string | null = null

let win: BrowserWindow | null = null

let isGameRunning = false
let showAppCloseWarning = true

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

function maybeDisableGPU(override: boolean) {
  console.log("Checking if GPU should be disabled...")
  if (app.isReady()) {
    console.error("App is ready, cannot disable GPU")
    return
  }

  const disableGPU = validateArgument("--disable-gpu") || override

  if (disableGPU) {
    console.log("Disabling GPU...")
    app.commandLine.appendSwitch("no-sandbox")
    app.commandLine.appendSwitch("disable-gpu")
    app.commandLine.appendSwitch("disable-software-rasterizer")
    app.commandLine.appendSwitch("disable-gpu-compositing")
    app.commandLine.appendSwitch("disable-gpu-rasterization")
    app.commandLine.appendSwitch("disable-gpu-sandbox")
    app.commandLine.appendSwitch("--no-sandbox")
  }

  // Disable GPU Acceleration for Windows 7
  if (disableGPU || (release().startsWith("6.1") && platform() === "win32")) {
    app.disableHardwareAcceleration()
    console.log("Hardware acceleration disabled")
  }
}

maybeDisableGPU(false)

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
        kill: () => void
      }
    }
  | {
      type: "error"
      logs: Log[]
    }
>

const loadCoreModule: CoreModule = () =>
  new Promise((resolve, _) => {
    console.log("Loading core module...")
    if (isDev) {
      resolve({
        type: "success",
        result: {
          port: 4650,
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

      console.log(`[CORE] Message: ${dataString}`)

      const rows = dataString.split(/\r?\n|\r|\n/g)

      logs.push({
        type: "info",
        message: dataString
      })

      for (const row of rows) {
        if (row.startsWith("_STATUS_:")) {
          const port: number = row.split("|")[1]
          console.log(`[CORE] Port: ${port}`)

          started = true

          resolve({
            type: "success",
            result: {
              port,
              kill: () => coreModule?.kill()
            }
          })
        } else if (row.startsWith("_INSTANCE_STATE_:")) {
          const rightPart = row.split(":")[1]
          const event = rightPart.split("|")[0]
          const action: FELauncherActionOnGameLaunch = rightPart.split("|")[1]

          if (event === "GAME_LAUNCHED") {
            isGameRunning = true
            console.log("Game launched, action:", action)
            switch (action) {
              case "closeWindow":
                win?.close()
                win = null
                break
              case "hideWindow":
                win?.hide()
                break
              case "minimizeWindow":
                win?.minimize()
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
        } else if (row.startsWith("_POTATO_PC_MODE_:")) {
          isPotatoPcModeSet = true
          const rightPart = row.split(":")[1]
          if (rightPart === "true") {
            maybeDisableGPU(true)
          }
          console.log("Potato PC mode set:", isPotatoPcModeSet)
        } else if (row.startsWith("_HASHED_EMAIL_PREFERENCE_CHANGED_:")) {
          const rightPart = row.split(":")[1]
          const enabled = rightPart.split("|")[0] === "true"
          const email = rightPart.split("|")[1]
          if (enabled) {
            if ((app as any).overwolf) {
              ;(app as any).overwolf.generateUserEmailHashes(email)
            }
          } else {
            if ((app as any).overwolf) {
              ;(app as any).overwolf.generateUserEmailHashes({})
            }
          }
          console.log("Hashed email preference changed:", enabled, email)
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

      if (code !== 0) {
        resolve({
          type: "error",
          logs
        })
      }

      resolve({
        type: "success",
        result: {
          port: 0,
          kill: () => coreModule?.kill()
        }
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

if (process.defaultApp) {
  if (process.argv.length >= 2) {
    app.setAsDefaultProtocolClient("gdlauncher", process.execPath, [
      resolve(process.argv[1])
    ])
  }
} else {
  app.setAsDefaultProtocolClient("gdlauncher")
}

let lastDisplay: Display | null = null

let isSpawningWindow = false

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

  win.on("move", () => {
    const bounds = win?.getBounds()

    if (!bounds) {
      return
    }

    const currentDisplay = screen.getDisplayMatching(bounds)
    if (lastDisplay?.id === currentDisplay?.id) {
      return
    }

    lastDisplay = currentDisplay
    const { minWidth, minHeight, adSize } = getAdSize(currentDisplay)
    win?.setMinimumSize(minWidth, minHeight)
    win?.setSize(minWidth, minHeight)
    win?.webContents?.send("adSizeChanged", adSize)
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
      shell.openExternal(url)
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
      console.log("dev tools open:", win?.webContents.isDevToolsOpened())
      win?.webContents.toggleDevTools()
    }
  })

  win.on("ready-to-show", () => {
    isSpawningWindow = false
    console.log("Window is ready to show")

    coreModule.finally(() => {
      if (win && !win?.isDestroyed()) {
        win?.show()
      }
    })

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

    if (import.meta.env.DEV) {
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
  console.info("relaunching app...")

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
  return getAdSize(currentDisplay).adSize
})

ipcMain.handle("openFileDialog", async (_, opts: OpenDialogOptions) => {
  return dialog.showOpenDialog(opts)
})

ipcMain.handle("showSaveDialog", async (_, opts: SaveDialogOptions) => {
  return dialog.showSaveDialog(opts)
})

ipcMain.handle("getCurrentOS", async () => {
  return { platform: os.platform(), arch: os.arch() }
})

ipcMain.handle("openFolder", async (_, path) => {
  shell.showItemInFolder(path)
})

ipcMain.handle("openCMPWindow", async () => {
  // @ts-ignore
  if (app.overwolf.openCMPWindow) {
    // @ts-ignore
    app.overwolf.openCMPWindow()
  }
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

ipcMain.handle("changeRuntimePath", async (_, newPath: string) => {
  interface Progress {
    action: "copy" | "remove"
    currentName: string
    current: number
    total: number
  }

  if (newPath === CURRENT_RUNTIME_PATH) {
    return
  }

  const runtimeOverridePath = path.join(
    app.getPath("userData"),
    RUNTIME_PATH_OVERRIDE_NAME
  )

  await fs.mkdir(newPath, { recursive: true })

  try {
    const cm = await coreModule
    if (cm.type === "success") {
      cm.result.kill()
    }
  } catch {
    // No op
  }

  const files = await glob("**/*", {
    cwd: CURRENT_RUNTIME_PATH!,
    nodir: true,
    dot: true,
    stat: false,
    ignore: ["**/.DS_Store", RUNTIME_PATH_OVERRIDE_NAME]
  })

  const total = files.length

  for (let i = 0; i < total; i++) {
    const file = files[i]

    win?.webContents.send("changeRuntimePathProgress", {
      action: "copy",
      currentName: path.basename(file),
      current: i,
      total: total * 2
    } satisfies Progress)

    await fse.copy(
      path.join(CURRENT_RUNTIME_PATH!, file),
      path.join(newPath, file),
      {
        overwrite: true,
        errorOnExist: false,
        recursive: true
      }
    )
  }

  await fse.writeFile(runtimeOverridePath, newPath)

  for (let i = 0; i < total; i++) {
    const file = files[i]

    win?.webContents.send("changeRuntimePathProgress", {
      action: "remove",
      currentName: path.basename(file),
      current: total + i,
      total: total * 2
    } satisfies Progress)

    await fse.remove(path.join(CURRENT_RUNTIME_PATH!, file))
  }

  app.relaunch()
  app.exit()
})

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
    port: cm.type === "success" ? cm.result.port : undefined
  }
})

app.whenReady().then(async () => {
  console.log("App is ready")
  const accessibility = validateArgument("--enable-accessibility")

  if (accessibility) {
    app.setAccessibilitySupportEnabled(true)
    console.log("Accessibility support enabled")
  }

  console.log("OVERWOLF APP ID", process.env.OVERWOLF_APP_UID)
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
      // eslint-disable-next-line
      delete details.responseHeaders!["Access-Control-Allow-Origin"]
      // eslint-disable-next-line
      delete details.responseHeaders!["access-control-allow-origin"]
      details.responseHeaders!["Access-Control-Allow-Origin"] = ["*"]
      callback({
        cancel: false,
        responseHeaders: details.responseHeaders
      })
    }
  )

  app.on("second-instance", (_e, _argv) => {
    if (win && !win.isDestroyed()) {
      // Focus on the main window if the user tried to open another
      if (win.isMinimized()) win.restore()
      win.focus()
    } else {
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

      const { minWidth, minHeight } = getAdSize(currentDisplay)
      if (changedMetrics.includes("workArea")) {
        win?.setMinimumSize(minWidth, minHeight)
        win?.setSize(minWidth, minHeight)
        win?.webContents.send("adSizeChanged", getAdSize().adSize)
      }
    }
  )

  initAutoUpdater(win)
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
  dialog.showErrorBox("Welcome Back", `You arrived from: ${url}`)
})

const LOOP_TIMEOUT = 4000

// keep event loop busy until potato pc mode is set or timeout is reached
if (!isPotatoPcModeSet && !import.meta.env.DEV) {
  let timeEnd = Date.now()
  while (!isPotatoPcModeSet && timeEnd - timeStart < LOOP_TIMEOUT) {
    timeEnd = Date.now()
  }

  // DO NOT REMOVE THIS CONSOLE LOG as V8 optimizes the loop away
  console.log("First event loop tick done in ", timeEnd - timeStart)
}
