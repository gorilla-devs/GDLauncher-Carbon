// Intentionally putting this on top to catch any potential error in dependencies as well

declare const __SHOWCASE_MODE__: boolean
/** True only in builds produced by the public `GDL_E2E` build switch — the Electron
 *  counterpart of the core's `e2e` cargo feature. See this file's use of it
 *  and `packages/main/vite.config.mjs`. */
declare const __E2E_BUILD__: boolean

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
import { ensureDirSync } from "fs-extra"
import type { ChildProcessWithoutNullStreams } from "child_process"
import { spawn } from "child_process"
import crypto from "crypto"
import log from "electron-log/main"
import { AccountIdentityReceiver } from "./utils/accountIdentity"
import {
  initializeMainUserData,
  publishE2EMainStart
} from "./utils/e2eBootstrap"
import { createCoreOutput } from "./utils/coreOutput"
import {
  LauncherLifecycle,
  mainWindowClosed,
  userWindows
} from "./utils/windowLifecycle"
import { ProtocolQueue, isSupportedProtocol } from "./utils/protocolQueue"
import { buildCoreModuleArgs } from "./utils/coreArgs.js"
import {
  IncompleteMigrationError,
  forgetUnavailableRuntimePath,
  leftInOldFolder,
  leftPartialCopy,
  relocateRuntime,
  removedPartialCopy,
  runtimeTargetRefusal,
  selectRuntimePath,
  unsupportedRuntimeEntries,
  validateRuntimeDestination,
  type RuntimePathValidation
} from "./utils/runtimeRelocation"
import {
  recordRuntimeMoveNotice,
  takeRuntimeMoveNotice,
  dismissRuntimeMoveNotice
} from "./utils/runtimeMoveNotice"
import {
  ShutdownBlockedError,
  ShutdownRefusedError,
  cancelCoreShutdown,
  checkRelocation,
  forceTerminateCore,
  relocationLeaseHeld,
  stopCoreForRelocation,
  stopCoreForShutdown,
  terminateAndObserveCoreExit,
  type RuntimeBlocker,
  type ShutdownStatus
} from "./utils/coreShutdown"
import {
  ShutdownFlow,
  ShutdownNotPossibleError,
  type ShutdownAction,
  type ShutdownOutcome,
  type ShutdownView
} from "./utils/shutdownFlow"
import {
  ShutdownPrompts,
  nativeShutdownDialog,
  shutdownPromptText,
  type ShutdownPromptText
} from "./utils/shutdownPrompt"
import {
  DatabaseRefusal,
  assertDatabaseRestoreReadySync,
  recoverInterruptedRestore,
  restoreDatabaseSnapshot
} from "./utils/databaseRestore"
import { RECOVERY_COPY_DAYS, type DbRecoveryCategory } from "./utils/dbRecovery"
import { resetDatabaseAndRestart } from "./utils/databaseReset"
import { pruneRecoveryCopies } from "./utils/databaseRecoveryBundles"
import {
  interruptRuntimeOperation,
  interruptibleUntilCommitted,
  runtimeOperationActive,
  runtimeOperationBrief,
  whenRuntimeOperationIdle,
  withRuntimeOperation
} from "./utils/runtimeOperation"
import {
  handleBeforeSendHeaders,
  handleHeadersReceived
} from "./utils/headerRewrite.js"
import * as Sentry from "@sentry/electron/main"
import "./preloadListeners"
import getAdSize from "./adSize"
import handleUncaughtException from "./handleUncaughtException"
import initAutoUpdater from "./autoUpdater"
import "./appMenu"

console.log("Modules imported successfully")

let overwolfReady = false
const accountIdentity = new AccountIdentityReceiver((hashes) => {
  if (!app.overwolf) return
  try {
    app.overwolf.setUserEmailHashes(hashes)
  } catch (error) {
    console.error("Failed to update email hashes:", error)
  }
})
function applyPendingEmail() {
  accountIdentity.setReady(overwolfReady && !!app.overwolf)
}

export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override"
const RUNTIME_PATH_DEFAULT_NAME = "data"

export let CURRENT_RUNTIME_PATH: string | null = null
// The data folder the pointer names when this start could not open it.
let UNAVAILABLE_RUNTIME_PATH: string | null = null

let win: BrowserWindow | null = null
let lastCoreModuleProgress: number | null = null

const getWin = () => {
  return win
}

const lifecycle = new LauncherLifecycle<BrowserWindow>({
  current: () => win,
  clear: (window) => {
    if (win === window) win = null
  },
  create: () => createWindow(),
  quit: () => app.quit()
})
const protocolQueue = new ProtocolQueue<BrowserWindow["webContents"]>(
  (sender, delivery) => sender.send("protocol-url", delivery)
)
ipcMain.handle("protocol-consumer-generation", (event) =>
  event.senderFrame === event.sender.mainFrame
    ? protocolQueue.session(event.sender)
    : null
)
ipcMain.on("protocol-consumer-ready", (event, value) => {
  if (event.senderFrame === event.sender.mainFrame)
    protocolQueue.ready(event.sender, value?.subscription, value?.generation)
})
ipcMain.on("protocol-consumer-ack", (event, delivery) => {
  if (event.senderFrame === event.sender.mainFrame)
    protocolQueue.ack(event.sender, delivery)
})
ipcMain.on("protocol-consumer-revoke", (event, subscription) => {
  if (event.senderFrame === event.sender.mainFrame)
    protocolQueue.unsubscribe(event.sender, subscription)
})
ipcMain.on("close-warning-receiver", (event, ready) => {
  if (event.senderFrame === event.sender.mainFrame)
    lifecycle.warningReceiver(event.sender, ready === true)
})
/** The prompt's text in the user's language, once an App has sent it. */
let promptText: ShutdownPromptText = {}
/** Shows a quit, restart or update that did not finish (utils/shutdownFlow). */
const shutdownPrompts = new ShutdownPrompts<BrowserWindow>(
  async (window, view, signal) => {
    const { options, choices } = nativeShutdownDialog(view, promptText)
    const { response } = await dialog.showMessageBox(window, {
      ...options,
      signal
    })
    return signal.aborted ? null : (choices[response] ?? null)
  }
)
ipcMain.on("shutdown-prompt-receiver", (event, ready) => {
  if (event.senderFrame === event.sender.mainFrame)
    shutdownPrompts.receiver(event.sender, ready === true)
})
ipcMain.on("shutdown-prompt-shown", (event, id) => {
  if (event.senderFrame === event.sender.mainFrame)
    shutdownPrompts.shown(event.sender, id)
})
ipcMain.on("shutdown-prompt-answer", (event, value) => {
  if (event.senderFrame === event.sender.mainFrame)
    shutdownPrompts.answer(event.sender, value?.id, value?.choice)
})
ipcMain.on("shutdown-prompt-text", (event, value) => {
  if (event.senderFrame === event.sender.mainFrame)
    promptText = shutdownPromptText(value)
})
let showAppCloseWarning = true

app.enableSandbox()
app.commandLine.appendSwitch("proxy-bypass-list", "127.0.0.1,localhost")

export function initRTPath(override: string | null | undefined) {
  CURRENT_RUNTIME_PATH = selectRuntimePath(
    override,
    app.getPath("userData"),
    (folder) => {
      UNAVAILABLE_RUNTIME_PATH = folder
      console.warn("Data folder not available, using the default:", folder)
    }
  )
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

/**
 * `arg`'s CLI value, falling back to `envVar` in e2e builds.
 *
 * ow-electron 39.8.11 on Windows exits before Node initialises as soon as two
 * argv tokens contain `://` (upstream; a minimal Electron app reproduces it).
 * The e2e harness needs three URLs, so it passes them by environment instead.
 * CLI still wins, and the fallback is `__E2E_BUILD__`-only so released builds
 * gain no new redirect vector.
 */
function argumentOrE2EEnv(arg: string, envVar: string): Argument | null {
  const fromCli = validateArgument(arg)
  if (fromCli?.value) {
    return fromCli
  }

  if (__E2E_BUILD__) {
    const fromEnv = process.env[envVar]
    if (fromEnv) {
      console.log("Argument taken from environment:", arg, envVar)
      return { argument: arg, value: fromEnv }
    }
  }

  return fromCli
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

const { userDataPath: patchedUserData, existed: skipIntroAnimation } =
  initializeMainUserData({
    e2eBuild: __E2E_BUILD__,
    e2eUserData: __E2E_BUILD__ ? process.env.GDL_E2E_USER_DATA : undefined,
    defaultUserData: getPatchedUserData,
    setPath: (value) => app.setPath("userData", value),
    setLogPath: (resolve) => {
      log.transports.file.resolvePathFn = (variables) =>
        resolve(variables.fileName!)
    },
    initializeLog: () => log.initialize()
  })
console.log("Skip intro animation:", skipIntroAnimation)
console.log("User data path set to:", app.getPath("userData"))
log.eventLogger.startLogging()
console.log("Logging initialized")
Object.assign(console, log.functions)

const overrideCLIDataPath = validateArgument("--runtime_path")
const overrideEnvDataPath = process.env.GDL_RUNTIME_PATH
if (app.isPackaged) {
  initRTPath(overrideCLIDataPath?.value || overrideEnvDataPath)
} else {
  const rtPath = import.meta.env.RUNTIME_PATH
  if (!rtPath) {
    throw new Error("Missing runtime path")
  }
  initRTPath(rtPath)
}

publishE2EMainStart({
  e2eBuild: __E2E_BUILD__,
  runId: __E2E_BUILD__ ? process.env.GDL_E2E_RUN_ID : undefined,
  userDataPath: patchedUserData,
  runtimePath: CURRENT_RUNTIME_PATH
})

console.log("Userdata path:", patchedUserData)
console.log("Runtime path:", CURRENT_RUNTIME_PATH)

const sentrySessionId = crypto.randomUUID()

console.log("SENTRY SESSION ID", sentrySessionId)

const allowMultipleInstances = validateArgument(
  "--gdl_allow_multiple_instances"
)

const overrideBaseApi = argumentOrE2EEnv(
  "--gdl_override_base_api",
  "GDL_OVERRIDE_BASE_API"
)
const e2eAuthBase = argumentOrE2EEnv("--gdl_e2e_auth_base", "GDL_E2E_AUTH_BASE")
const e2eEntitlementKey = validateArgument("--gdl_e2e_entitlement_key")
// Gated at the build, not merely parsed and ignored. Unlike the two flags
// above — which are forwarded to the core and are inert unless it was built
// with the `e2e` cargo feature — this one is consumed by this process and
// hands electron-updater a new feed URL. In a shipped build that is an
// update-channel redirect available to anyone who can influence the
// launcher's command line (an edited shortcut or .desktop file), and on
// Linux the integrity check is the sha512 from that same feed, so it does
// not help. `__E2E_BUILD__` is false in every released artifact, and the
// argument is not even read there.
const e2eUpdateFeed = __E2E_BUILD__
  ? argumentOrE2EEnv("--gdl_e2e_update_feed", "GDL_E2E_UPDATE_FEED")
  : null

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
      // Sanitized startup capture bounded to 1MiB, frozen at the first outcome.
      logs: Log[]
    }
  | {
      type: "error"
      logs: Log[]
      // Present only for `DB_DOWNGRADE_FAILED`: the pre-downgrade snapshot the
      // failure screen can offer to restore.
      snapshotPath?: string
      dbRecovery?: DbRecoveryCategory
    }
  | {
      type: "backwardsMigration"
      logs: Log[]
    }
>

// Must match DEV_API_TOKEN in crates/carbon_app/src/main.rs.
// Debug builds of the rust core accept this fixed token; release builds
// rotate randomly per launch.
const DEV_API_TOKEN = "dev-mode-only-do-not-use-in-production"

// The spawned core process, tracked at module scope so recovery handlers can
// terminate it even when it never reached READY: a hung startup resolves the
// core promise via the timeout below while the process itself stays alive.
let coreProcessHandle: ChildProcessWithoutNullStreams | null = null

const loadCoreModule: CoreModule = async () => {
  // Nothing here may await before the spawn below. Called while this module
  // loads, the core starts before Chromium launches any child process, as in
  // 2.0.40. Spawned later, it inherits Chromium's IPC sockets (Node passes
  // every descriptor not marked close-on-exec) and keeps a child's end open:
  // a GPU process that then exits is never noticed, no fallback starts, and
  // the window never paints.
  try {
    assertDatabaseRestoreReadySync(CURRENT_RUNTIME_PATH!)
  } catch (error) {
    return {
      type: "error",
      logs: [{ type: "error", message: String(error) }],
      // The screen offers the way out of this one (recoverInterruptedRestore).
      dbRecovery:
        error instanceof DatabaseRefusal ? "interrupted_restore" : undefined
    }
  }
  return new Promise((resolve) => {
    console.log("Loading core module...")
    if (isDev) {
      resolve({
        type: "success",
        result: {
          port: 4650,
          apiToken: DEV_API_TOKEN,
          kill: () => {}
        },
        logs: []
      })
      console.log("Core module loaded in development mode")
      return
    }

    const coreModulePath = path.resolve(
      __dirname,
      "../../../../resources/binaries",
      binaryName
    )

    console.log(`[CORE] Spawning core module: ${coreModulePath}`)
    let coreModule: ChildProcessWithoutNullStreams | null = null
    const logs: Log[] = []

    const args = buildCoreModuleArgs({
      runtimePath: CURRENT_RUNTIME_PATH!,
      baseApi: overrideBaseApi?.value,
      e2eAuthBase: e2eAuthBase?.value,
      e2eEntitlementKey: e2eEntitlementKey?.value
    })

    const identitySession = crypto.randomUUID()
    accountIdentity.beginCore(identitySession)
    lifecycle.beginCore(identitySession)
    try {
      coreModule = spawn(coreModulePath, args, {
        shell: false,
        detached: false,
        stdio: "pipe",
        env: {
          ...process.env,
          RUST_BACKTRACE: "full",
          GDL_IDENTITY_SESSION: identitySession
        }
      })
      coreProcessHandle = coreModule
      // Exposed for the e2e harness (`e2e-tests/fixtures/electronApp.ts`'s
      // `relaunchApp`), which runs in a separate OS process from this one and
      // has no other way to read this process's own module-scope state. It
      // reads this via Playwright's `ElectronApplication.evaluate`, which
      // executes directly in this main process, to confirm the core process
      // it just closed has genuinely exited before relaunching onto the same
      // database — see that file for why a fixed sleep isn't good enough
      // here. Harmless outside a test: just an OS pid number on `globalThis`.
      ;(globalThis as Record<string, unknown>).__gdlCoreProcessId =
        coreModule.pid ?? null
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

    const coreOutput = createCoreOutput({
      log: (entry) => console.log(`[CORE] Message: ${entry.message}`),
      settle: (result) => {
        clearTimeout(startupTimer)
        if (result.type === "success")
          resolve({
            ...result,
            result: { ...result.result, kill: () => coreModule?.kill() }
          })
        else resolve(result)
      },
      progress: (event) => {
        const progress: Record<string, number> = {
          VerifyingTermsAndPrivacy: 10,
          LoadAndMigrate: 20,
          RefreshMSAuth: 35,
          XboxAuth: 50,
          McLogin: 65,
          MCEntitlements: 77,
          McProfile: 88,
          AccountRefreshComplete: 95,
          LaunchBackgroundTasks: 100
        }
        lastCoreModuleProgress = progress[event] ?? 0
        const window = getWin()
        if (window && !window.isDestroyed())
          window.webContents.send("coreModuleProgress", lastCoreModuleProgress)
      },
      record: (row) => {
        if (row.startsWith("_INSTANCE_RUNNING_:"))
          lifecycle.snapshot(
            row.slice("_INSTANCE_RUNNING_:".length),
            identitySession
          )
        else if (row.startsWith("_INSTANCE_STATE_:"))
          lifecycle.event(
            row.slice("_INSTANCE_STATE_:".length),
            identitySession
          )
        else if (row.startsWith("_SHOW_APP_CLOSE_WARNING_:"))
          showAppCloseWarning =
            row.slice("_SHOW_APP_CLOSE_WARNING_:".length) === "true"
        else if (row.startsWith("_GDL_ACCOUNT_EMAIL_:"))
          accountIdentity.acceptPayload(
            row.slice("_GDL_ACCOUNT_EMAIL_:".length)
          )
      }
    })
    const startupTimer = setTimeout(
      () => {
        if (coreOutput.settled) return
        Sentry.captureException(new Error("Core module took too long to start"))
        coreOutput.fail("Core module took too long to start")
      },
      60 * 5 * 1000
    )
    coreModule.on("error", (err) => coreOutput.fail(String(err)))
    coreModule.stdout.on("data", (data) => coreOutput.push(data))
    coreModule.stdout.on("end", () => coreOutput.end())
    coreModule.stderr.on("data", (data) => {
      coreOutput.stderr(data.toString())
      console.error(`[CORE] Error: ${data.toString()}`)
    })
    coreModule.on("exit", (code) => {
      const keptAlive = lifecycle.endCore(identitySession)
      coreOutput.end()
      // Retain this launcher-owned handle: its observed exit is recovery evidence.
      coreOutput.fail(
        `Core module exited unexpectedly with code ${code} before READY`
      )
      if (keptAlive) quitAfterCoreExit()
    })
  })
}

const coreModule = loadCoreModule()

if (app.overwolf) {
  app.overwolf.disableAnonymousAnalytics()
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
let adSizeFallbackTimeout: NodeJS.Timeout | null = null

let isSpawningWindow = false

// Every ingress is queued even while the loading window exists.
for (const url of process.argv.filter(isSupportedProtocol))
  protocolQueue.enqueue(url)

async function createWindow(): Promise<BrowserWindow> {
  console.log("Creating window...")
  if (isSpawningWindow) {
    console.log("Window is already being spawned")
    return win!
  }

  // A live window is reused rather than replaced. A destroyed one is only a
  // stale handle: close() and nearly every other method throw on it, so it is
  // dropped without being touched.
  if (win && !win.isDestroyed()) {
    if (win.isMinimized()) win.restore()
    win.focus()
    return win
  }
  win = null

  isSpawningWindow = true
  try {
    return spawnWindow()
  } catch (error) {
    // Not `finally`: on success the flag must stay set until 'ready-to-show'.
    // On failure that event never comes, and a stuck flag would block every
    // later window and the quit.
    isSpawningWindow = false
    throw error
  }
}

function spawnWindow(): BrowserWindow {
  const currentDisplay = screen.getPrimaryDisplay()
  lastDisplay = currentDisplay
  const { minWidth, minHeight, height, width } = getAdSize(currentDisplay)

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

  const createdWindow = win
  // Captured now: once the window is destroyed (already the case when
  // 'closed' fires), its webContents getter throws "Object has been destroyed".
  const contents = createdWindow.webContents
  protocolQueue.bind(contents)
  // A reload or renderer crash replaces the App, taking its protocol
  // subscription and close-warning acknowledgement with it; the next App
  // gives both again once it mounts. 'did-navigate' fires only when a
  // main-frame, cross-document navigation commits. Not 'did-start-navigation':
  // that also fires for every link click will-navigate below cancels, which
  // the App survives without subscribing again, so every later deep link
  // would wait in the queue until a restart.
  const documentReplaced = () => {
    protocolQueue.revoke(contents)
    lifecycle.warningReceiver(contents, false)
    shutdownPrompts.receiver(contents, false)
  }
  contents.on("did-navigate", documentReplaced)
  contents.on("render-process-gone", documentReplaced)
  createdWindow.on("closed", () => {
    shutdownPrompts.receiver(contents, false)
    mainWindowClosed(createdWindow, contents, {
      current: () => win,
      release: () => {
        // 'ready-to-show' never fires for a window closed before it, and a
        // stuck flag would block every later window and the quit.
        isSpawningWindow = false
        pendingDisplayChange = false
        if (adSizeFallbackTimeout) {
          clearTimeout(adSizeFallbackTimeout)
          adSizeFallbackTimeout = null
        }
      },
      lifecycle,
      protocol: protocolQueue
    })
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
    // shrink a window the user made larger. When the work area is smaller than
    // the minimums, the minimums win (matching setMinimumSize above).
    const workArea = display.workArea
    const targetWidth = Math.max(
      Math.min(bounds.width, workArea.width),
      minWidth
    )
    const targetHeight = Math.max(
      Math.min(bounds.height, workArea.height),
      minHeight
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
        return
      }
    }

    if (!pendingDisplayChange) {
      return
    }

    // Fallback for moves that never emit `moved` on Windows (Win+Shift+Arrow,
    // programmatic setPosition, OS relocation on monitor unplug): debounce past
    // the last `move`. During an interactive drag `moved` fires on release and
    // cancels this. A >400ms mid-drag pause could let the timer elapse, but in
    // practice Windows starves libuv timers inside the modal move loop, so the
    // callback only runs after the drag ends; if that ever changes, the resize
    // would fight the drag (the snap-to-seam issue handled below).
    if (adSizeFallbackTimeout) {
      clearTimeout(adSizeFallbackTimeout)
    }
    adSizeFallbackTimeout = setTimeout(() => {
      adSizeFallbackTimeout = null
      if (pendingDisplayChange) {
        applyAdLayoutForCurrentDisplay()
      }
    }, 400)
  })

  // Resizing while the user is still dragging fights the OS drag loop and makes
  // the window snap to the seam between monitors. `moved` fires once when the
  // interactive move ends on Windows (on macOS it's an alias of `move`), so the
  // new ad layout is applied only after the drag is released.
  win.on("moved", () => {
    if (adSizeFallbackTimeout) {
      clearTimeout(adSizeFallbackTimeout)
      adSizeFallbackTimeout = null
    }

    if (pendingDisplayChange) {
      applyAdLayoutForCurrentDisplay()
    }
  })

  createdWindow.on("close", (e) => {
    lifecycle.close(createdWindow, e, showAppCloseWarning)
    if (e.defaultPrevented || shutdownApproved) return
    // A prompt about a quit, restart or update is waiting for an answer.
    if (shutdownPrompts.open) {
      e.preventDefault()
      return
    }
    // Closing the last window quits. It is hidden rather than destroyed, so
    // a quit that cannot finish has a window to say why in and to come back
    // to. A move keeps its own close handling (quitUnlessKeptAlive).
    if (
      !runtimeOperationActive() &&
      lifecycle.quitsOnClose(
        createdWindow,
        e,
        userWindows(BrowserWindow.getAllWindows(), win).filter(
          (w) => w !== createdWindow
        ).length
      )
    ) {
      e.preventDefault()
      createdWindow.hide()
      app.quit()
    }
  })
  contents.on("will-prevent-unload", () =>
    lifecycle.cancelLaunchClose(createdWindow)
  )

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
// Restart shares ordinary shutdown/game-survival when a READY owned core
// exists; if it does not finish, the user decides (utils/shutdownFlow).
ipcMain.handle("relaunch", async () =>
  withRuntimeOperation(async () => {
    console.log("relaunching app...")

    if ((await ownedShutdown("restart")) === "kept")
      return { status: "kept" as const }

    app.relaunch()
    app.exit()
    return { status: "relaunching" as const }
  })
)
/** Keep GDLauncher open on a reset or restore: nothing was changed. */
const keptBeforeDatabaseChange = () =>
  new DatabaseRefusal(
    "GDLauncher was kept open, so the database was not changed. Close what is still running and try again."
  )

/** A recovery's restart notice, attached to the launcher so it cannot open
 *  behind it while the screen waits on it. */
const showRecoveryNotice = (options: Electron.MessageBoxOptions) =>
  win && !win.isDestroyed()
    ? dialog.showMessageBox(win, options)
    : dialog.showMessageBox(options)

ipcMain.handle("deleteDbAndRestart", async () =>
  withRuntimeOperation(async () => {
    console.log("Resetting the database and restarting...")
    const outcome = await resetDatabaseAndRestart(CURRENT_RUNTIME_PATH!, {
      // Resolves only once the core's exit was observed, also when the user
      // chose Reset now: files move only after that.
      stopCore: async () => {
        if ((await ownedShutdown("reset")) === "kept")
          throw keptBeforeDatabaseChange()
      },
      restart: async (recoveryPath) => {
        console.log(`Database reset; recovery files kept in ${recoveryPath}`)
        await showRecoveryNotice({
          type: "info",
          title: "Database reset",
          message:
            "Database metadata was reset. A complete recovery copy has been retained.",
          detail: `Recovery files: ${recoveryPath}\nKeep these files for manual recovery; GDLauncher removes them after ${RECOVERY_COPY_DAYS} days. Instance, world, addon and server directories remain.`,
          buttons: ["Restart"],
          defaultId: 0,
          noLink: true
        })
        app.relaunch()
        app.exit()
      }
    })
    if (!outcome.ok)
      console.error("Database reset did not finish:", outcome.message)
    return outcome
  })
)

ipcMain.handle(
  "restoreDbSnapshotAndRestart",
  async (_event, snapshotPath: string) =>
    withRuntimeOperation(async () => {
      console.log("Restoring the database snapshot and restarting...")
      const outcome = await restoreDatabaseSnapshot(
        CURRENT_RUNTIME_PATH!,
        snapshotPath,
        {
          stopCore: async () => {
            if ((await ownedShutdown("restore")) === "kept")
              throw keptBeforeDatabaseChange()
          },
          restart: async (recoveryPath) => {
            console.log(
              `Database snapshot restored; original files kept in ${recoveryPath}`
            )
            await showRecoveryNotice({
              type: "info",
              title: "Database snapshot restored",
              message:
                "The previous database is restored. Original database files have been retained.",
              detail: `Recovery files: ${recoveryPath}\nKeep these files until the restored launcher is working; GDLauncher removes them after ${RECOVERY_COPY_DAYS} days.`,
              buttons: ["Restart"],
              defaultId: 0,
              noLink: true
            })
            app.relaunch()
            app.exit()
          }
        }
      )
      if (!outcome.ok)
        console.error(
          "Database snapshot restore did not finish:",
          outcome.message
        )
      return outcome
    })
)

// The way out of the startup guard: a core was never started in that state.
ipcMain.handle("recoverInterruptedDbRestore", async () =>
  withRuntimeOperation(async () => {
    console.log("Recovering an interrupted database restore...")
    const outcome = await recoverInterruptedRestore(CURRENT_RUNTIME_PATH!, {
      stopCore: async () => {
        if ((await ownedShutdown("restore")) === "kept")
          throw keptBeforeDatabaseChange()
      },
      restart: async ({ restored, recoveryPath }) => {
        await showRecoveryNotice({
          type: "info",
          title: "Database recovered",
          message: restored
            ? "The interrupted restore is finished: the previous database is in use. Original database files have been retained."
            : "The interrupted restore is undone: your database files are back as they were.",
          ...(restored && {
            detail: `Recovery files: ${recoveryPath}\nKeep these files until the restored launcher is working; GDLauncher removes them after ${RECOVERY_COPY_DAYS} days.`
          }),
          buttons: ["Restart"],
          defaultId: 0,
          noLink: true
        })
        app.relaunch()
        app.exit()
      }
    })
    if (!outcome.ok)
      console.error("Database recovery did not finish:", outcome.message)
    return outcome
  })
)

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
  if (app.overwolf?.openCMPWindow) {
    app.overwolf.openCMPWindow()
    return true
  }
  return false
})

ipcMain.handle("isCMPWindowAvailable", async () => {
  // Availability means "can the CMP window open at all" (i.e. an ow-electron
  // build with the overwolf API injected). Deliberately NOT `isCMPRequired()`:
  // that is country-dependent, and users outside CMP-required regions must
  // still be able to manage their consent.
  return !!app.overwolf?.openCMPWindow
})

ipcMain.handle("closeWindow", async () => lifecycle.confirmQuit())

ipcMain.handle("getUserData", async () => {
  return app.getPath("userData")
})

ipcMain.handle("getInitialRuntimePath", async () => {
  return path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME)
})

ipcMain.handle("getRuntimePath", async () => {
  return CURRENT_RUNTIME_PATH
})

function runtimeDestinationContext() {
  const installationRoots = [
    path.dirname(process.execPath),
    process.resourcesPath
  ]
  if (process.platform === "darwin")
    installationRoots.push(path.resolve(process.resourcesPath, "../.."))
  return { installationRoots }
}

ipcMain.handle(
  "changeRuntimePath",
  async (
    _,
    newPath: string,
    switchOnly = false
  ): Promise<import("./utils/runtimeRelocation").RuntimeRelocationResult> => {
    return withRuntimeOperation(async () => {
      let recovery: "none" | "restart" | "shutdown_unresolved" = "none"
      // Set only when the live core confirmed it holds no move for us.
      let refused: { blockers: RuntimeBlocker[] } | undefined
      // A quit or last-window close before the core is stopped ends the move.
      const interruptible = interruptibleUntilCommitted()
      try {
        if (!CURRENT_RUNTIME_PATH) throw new Error("Runtime is not initialized")
        if (overrideCLIDataPath?.value || overrideEnvDataPath) {
          throw new Error(
            "Remove the explicit runtime path override and restart before relocating"
          )
        }
        const source = CURRENT_RUNTIME_PATH
        const outcome = await relocateRuntime({
          source,
          target: newPath,
          switchOnly,
          pointer: path.join(
            app.getPath("userData"),
            RUNTIME_PATH_OVERRIDE_NAME
          ),
          context: runtimeDestinationContext(),
          interruptible,
          // Read-only and before anything is stopped, so recovery stays "none".
          checkCore: async () => {
            const core = await coreModule
            if (core.type !== "success")
              throw new Error("Core is not ready; restart and retry relocation")
            try {
              await checkRelocation({
                ...core.result,
                process: coreProcessHandle
              })
            } catch (error) {
              // A core that is already gone gets only Restart, as below.
              if (
                coreProcessHandle &&
                (coreProcessHandle.exitCode != null ||
                  coreProcessHandle.signalCode != null)
              )
                recovery = "restart"
              else if (error instanceof ShutdownRefusedError)
                refused = { blockers: error.blockers }
              throw error
            }
          },
          stopCore: async () => {
            const core = await coreModule
            if (core.type !== "success")
              throw new Error("Core is not ready; restart and retry relocation")
            recovery = "shutdown_unresolved"
            try {
              await stopCoreForRelocation(
                { ...core.result, process: coreProcessHandle },
                source
              )
            } catch (error) {
              if (
                coreProcessHandle &&
                (coreProcessHandle.exitCode != null ||
                  coreProcessHandle.signalCode != null)
              )
                recovery = "restart"
              else if (error instanceof ShutdownRefusedError) {
                // Refused or released before commit: the core never froze,
                // so the game, servers and downloads keep running.
                recovery = "none"
                refused = { blockers: error.blockers }
              }
              throw error
            }
            recovery = "restart"
          },
          progress: (value) =>
            win?.webContents.send("changeRuntimePathProgress", value)
        })
        for (const warning of outcome.warnings) console.warn(warning)
        // Whatever stayed in the old folder is named, in the user's language, on every
        // start from the new one until the user acknowledges it. Main restarts now, as
        // 2.0.40 did, so a window closed or crashed during the move cannot leave the
        // launcher on a stopped core.
        await recordRuntimeMoveNotice(
          app.getPath("userData"),
          leftInOldFolder(source, newPath, outcome)
        )
        app.relaunch()
        app.exit()
        return { type: "relaunching" }
      } catch (error) {
        // Ended by a quit before the core was asked to stop: nothing moved,
        // and with no lease left from an earlier attempt nothing is held.
        if (
          error === interruptible.signal.reason &&
          recovery === "none" &&
          !relocationLeaseHeld(coreProcessHandle)
        )
          refused = { blockers: [] }
        return {
          type: "failed",
          recovery,
          message: String(error),
          // Decided from what failed, never by probing the renderer-supplied path.
          incompleteTarget: leftPartialCopy(error),
          unsupported: unsupportedRuntimeEntries(error),
          ...(removedPartialCopy(error) && { copyRemoved: true }),
          ...(refused && { refused })
        }
      }
    })
  }
)

ipcMain.handle("takeRuntimeMoveNotice", async () =>
  CURRENT_RUNTIME_PATH
    ? takeRuntimeMoveNotice(app.getPath("userData"), CURRENT_RUNTIME_PATH)
    : null
)

// The user acknowledged the notice; until then every start shows it again.
ipcMain.handle("dismissRuntimeMoveNotice", async () =>
  dismissRuntimeMoveNotice(app.getPath("userData"))
)

// The data folder could not be opened (its drive not connected, say), so this
// start uses the default one; every such start says so.
ipcMain.handle("getUnavailableRuntimePath", async () =>
  UNAVAILABLE_RUNTIME_PATH && CURRENT_RUNTIME_PATH
    ? { folder: UNAVAILABLE_RUNTIME_PATH, current: CURRENT_RUNTIME_PATH }
    : null
)

// The user keeps the default folder this start uses: later starts open it
// without asking. Only the folder this start could not open is forgotten,
// and only while no move runs. A quit meanwhile waits for it.
ipcMain.handle("keepDefaultRuntimePath", async () =>
  withRuntimeOperation(
    async () => {
      const folder = UNAVAILABLE_RUNTIME_PATH
      if (!folder) return
      await forgetUnavailableRuntimePath(
        path.join(app.getPath("userData"), RUNTIME_PATH_OVERRIDE_NAME),
        folder
      )
      UNAVAILABLE_RUNTIME_PATH = null
    },
    { brief: true }
  )
)

ipcMain.handle(
  "validateRuntimePath",
  async (_, newPath: string | null): Promise<RuntimePathValidation> => {
    if (!newPath) return "invalid"
    try {
      return await validateRuntimeDestination(
        newPath,
        CURRENT_RUNTIME_PATH,
        runtimeDestinationContext()
      )
    } catch (error) {
      // An unfinished move gets its own answer so Settings can name the way out.
      // Only a partial copy of the folder in use is safe to call deletable.
      if (error instanceof IncompleteMigrationError)
        return error.fromSource
          ? "incomplete_migration"
          : "other_incomplete_migration"
      // Settings names the reason; the log keeps the details. Settings checks
      // the folder in use on every visit, which is never a destination.
      if (
        !CURRENT_RUNTIME_PATH ||
        path.resolve(newPath) !== path.resolve(CURRENT_RUNTIME_PATH)
      )
        console.warn("Data folder refused:", newPath, String(error))
      return runtimeTargetRefusal(error)
    }
  }
)

ipcMain.handle("getCoreModule", async () => {
  // we can assume this promise never rejects
  const cm = await coreModule

  return {
    type: cm.type,
    // Present for every outcome, not only `"error"`: the only record of a
    // non-fatal status line (`DB_DOWNGRADED` chief among them) once startup
    // has moved on, since nothing else threads it anywhere observable.
    logs: cm.logs,
    snapshotPath: cm.type === "error" ? cm.snapshotPath : undefined,
    dbRecovery: cm.type === "error" ? cm.dbRecovery : undefined,
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
  if (app.overwolf && process.env.OVERWOLF_APP_UID) {
    overwolfReady = true
    applyPendingEmail()
  }

  // Electron allows exactly one listener per webRequest event per session —
  // a later registration silently replaces an earlier one — so all header
  // rewriting for the renderer session lives in this single pair. See
  // ./utils/headerRewrite.ts for what each listener does and why both guard
  // against missing headers and always invoke `callback` exactly once.
  session.defaultSession.webRequest.onBeforeSendHeaders(handleBeforeSendHeaders)
  session.defaultSession.webRequest.onHeadersReceived(handleHeadersReceived)

  app.on("second-instance", (_e, argv) => {
    for (const url of argv.filter(isSupportedProtocol))
      protocolQueue.enqueue(url)
    if (shutdownInFlight || !win || win.isDestroyed()) revealWindow()
    else {
      if (win.isMinimized()) win.restore()
      win.focus()
    }
  })

  // During a quit that is still finishing, the hidden window comes back
  // with its prompt instead of a second window being created.
  app.on("activate", () => {
    if (shutdownInFlight || !win || win.isDestroyed()) revealWindow()
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

  initAutoUpdater(e2eUpdateFeed?.value, () => ownedShutdown("update"))
})

/**
 * Shows the launcher window again: the one hidden while a quit finishes, or
 * a new one if there is none. A second launch or a Dock click during a quit
 * that is still running shows that quit's prompt instead of a new window.
 */
function revealWindow() {
  if (win && !win.isDestroyed()) {
    if (win.isMinimized()) win.restore()
    win.show()
    win.focus()
  } else void createWindow()
  shutdownFlow?.reveal()
}
function hideWindow() {
  if (win && !win.isDestroyed()) win.hide()
}

let shutdownInFlight: Promise<ShutdownOutcome> | undefined
let shutdownFlow: ShutdownFlow | undefined
let shutdownApproved = false
/** Settles once the core module has: never waits on a core still starting. */
let coreLoaded: Awaited<typeof coreModule> | undefined
void coreModule.then((core) => {
  coreLoaded = core
  // A clean start: old recovery copies of the database have done their job.
  // A quit meanwhile waits for this and then runs.
  if (core.type === "success" && !runtimeOperationActive())
    void withRuntimeOperation(
      () => pruneRecoveryCopies(CURRENT_RUNTIME_PATH!),
      { brief: true }
    ).catch((error) =>
      console.warn("Could not check old database recovery copies:", error)
    )
})
/** Waits for the core to start unless `signal` gives up first. */
function startedCore(signal: AbortSignal) {
  return new Promise<Awaited<typeof coreModule>>((resolve, reject) => {
    const aborted = () =>
      reject(
        new ShutdownBlockedError("GDLauncher is still starting up", "slow")
      )
    if (signal.aborted) return aborted()
    signal.addEventListener("abort", aborted, { once: true })
    void coreModule.then((core) => {
      signal.removeEventListener("abort", aborted)
      resolve(core)
    })
  })
}
/**
 * One attempt to stop the launcher-owned core for `action`. An external
 * development core is left running, as before, except for a reset or restore,
 * which must see the exit. A core that never reported READY is only ended for
 * a restart, reset or restore, and one that was never started has nothing to end.
 */
async function stopOwnedCore(
  action: ShutdownAction,
  signal: AbortSignal,
  onStatus: (status: ShutdownStatus) => void
) {
  const core = await startedCore(signal)
  const owned = coreProcessHandle
  if (!owned) {
    // Database files only move after an observed exit. A core that ended up
    // not started has no process to wait for; a running one here is external.
    if (action === "reset" && core.type === "success")
      throw new ShutdownNotPossibleError(
        "Database reset requires a launcher-owned core exit; external/development core must be stopped and recovered manually"
      )
    if (action === "restore" && core.type === "success")
      throw new ShutdownNotPossibleError(
        "Database restore requires a launcher-owned core exit; external/development core must be stopped and recovered manually"
      )
    return
  }
  if (core.type === "success")
    await stopCoreForShutdown(
      { ...core.result, process: owned },
      action === "update" ? "update" : "quit",
      fetch,
      5000,
      { signal, onStatus }
    )
  else if (action !== "quit" && action !== "update")
    await terminateAndObserveCoreExit(owned)
}
/**
 * Stops the owned core for a quit, update, restart or database reset or
 * restore. If that does not finish within about 3 s, the launcher window
 * shows why, and the user can wait, stop it now, or keep GDLauncher open
 * (which gives the core its work back). Callers for the same action join;
 * another action is refused while one runs.
 */
function ownedShutdown(action: ShutdownAction): Promise<ShutdownOutcome> {
  if (shutdownApproved) return Promise.resolve("stopped")
  if (shutdownInFlight) {
    if (shutdownFlow?.action === action) return shutdownInFlight
    shutdownFlow?.reveal()
    return Promise.reject(
      new Error("GDLauncher is already closing, restarting or updating")
    )
  }
  const flow = new ShutdownFlow(action, {
    stop: (signal, onStatus) => stopOwnedCore(action, signal, onStatus),
    cancel: async () => {
      const owned = coreProcessHandle
      if (coreLoaded?.type !== "success" || !owned) return "released"
      return cancelCoreShutdown({ ...coreLoaded.result, process: owned })
    },
    force: async () => {
      if (coreProcessHandle) await forceTerminateCore(coreProcessHandle)
    },
    prompt: async (view: ShutdownView, signal: AbortSignal) => {
      const window = win && !win.isDestroyed() ? win : await createWindow()
      return shutdownPrompts.ask(window, view, signal)
    },
    update: (view: ShutdownView, signal: AbortSignal) =>
      shutdownPrompts.update(signal, view),
    hide: action === "quit" ? hideWindow : undefined,
    log: (message, error) => console.warn(message, error)
  })
  const run = (async (): Promise<ShutdownOutcome> => {
    const outcome = await flow.run()
    if (outcome !== "kept") {
      shutdownApproved = true
      lifecycle.approveQuit()
    }
    return outcome
  })()
  shutdownFlow = flow
  shutdownInFlight = run
  const done = () => {
    if (shutdownInFlight !== run) return
    shutdownInFlight = undefined
    shutdownFlow = undefined
  }
  void run.then(done, done)
  return run
}
/** A quit or window close ended a move before the core was stopped. */
function closedBeforeMove() {
  return new Error(
    "GDLauncher was closed before the move started; nothing was moved"
  )
}
/** Quits once no window is left, unless something still needs the launcher
 *  without one. */
function quitUnlessKeptAlive() {
  if (runtimeOperationActive()) {
    // No window shows the operation any more. A move that has not stopped
    // the core yet ends here with nothing moved; anything further along
    // finishes first. Either way the launcher then quits instead of staying
    // on without a window.
    interruptRuntimeOperation(closedBeforeMove())
    whenRuntimeOperationIdle(quitIfStillWindowless)
    return
  }
  if (isSpawningWindow || lifecycle.keepAliveWithoutWindows()) return
  app.quit()
}
function quitIfStillWindowless() {
  if (userWindows(BrowserWindow.getAllWindows(), win).length === 0)
    quitUnlessKeptAlive()
}
function quitAgain() {
  app.quit()
}
app.on("window-all-closed", quitUnlessKeptAlive)
/**
 * The core exited while its Close Window games kept the launcher from
 * quitting once windowless, so it could reopen when they exit. A dead core
 * cannot report that, so a launcher without a window would stay invisible
 * for good: take the quit decision 'window-all-closed' deferred. It quits
 * only when no window is left (a relaunch during the game may have opened
 * one) and no quit, restart or update is already under way. The games keep
 * running, and the next launch starts a fresh core that adopts them.
 */
function quitAfterCoreExit() {
  if (
    shutdownApproved ||
    shutdownInFlight ||
    userWindows(BrowserWindow.getAllWindows(), win).length > 0
  )
    return
  quitUnlessKeptAlive()
}
app.on("before-quit", (event) => {
  if (shutdownApproved) return
  event.preventDefault()
  if (runtimeOperationActive()) {
    // A move that has not stopped the core yet gives way, and a brief step
    // such as removing old recovery copies finishes: the quit runs once
    // either has ended. Anything else, such as a move further along, keeps
    // its window and finishes.
    if (
      interruptRuntimeOperation(closedBeforeMove()) ||
      runtimeOperationBrief()
    )
      whenRuntimeOperationIdle(quitAgain)
    return
  }
  if (shutdownInFlight) {
    // Quit again while one is still finishing: show where it stands.
    revealWindow()
    return
  }
  // The window goes away at once; it comes back if the quit does not finish,
  // and not because a game launched with Hide or Minimize exits meanwhile.
  hideWindow()
  lifecycle.quitStarted()
  void ownedShutdown("quit").then(
    (outcome) => {
      if (outcome !== "kept") return app.quit()
      lifecycle.quitFailed()
      revealWindow()
    },
    (error) => {
      // Even SIGKILL could not be seen to end the core. The user chose to
      // quit, so the launcher still goes.
      console.error("Quit could not confirm the core exit", error)
      app.exit()
    }
  )
})

app.on("render-process-gone", (event, webContents, detailed) => {
  console.error("render-process-gone", detailed)
  webContents.reload()
})

// macOS delivers a link here, Windows and Linux through 'second-instance'.
// During a quit that is still finishing, the link shows that quit's prompt
// as 'second-instance' does, instead of reaching a hidden window or a new one
// the quit then closes.
app.on("open-url", (event, url) => {
  if (!isSupportedProtocol(url)) return
  event.preventDefault()
  protocolQueue.enqueue(url)
  if (shutdownInFlight || !win || win.isDestroyed()) {
    // Before ready, the window the launch creates takes the link.
    if (app.isReady()) revealWindow()
  } else {
    if (win.isMinimized()) win.restore()
    win.focus()
  }
})
