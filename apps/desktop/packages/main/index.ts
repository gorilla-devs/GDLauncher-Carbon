// Intentionally putting this on top to catch any potential error in dependencies as well

declare const __SHOWCASE_MODE__: boolean
/** True only in builds produced by the `build:*-e2e` scripts — the Electron
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
import fs from "fs/promises"
import fss from "fs"
import fse, { ensureDirSync } from "fs-extra"
import type { ChildProcessWithoutNullStreams } from "child_process"
import { spawn } from "child_process"
import crypto from "crypto"
import log from "electron-log/main"
import {
  AccountIdentityReceiver,
  CoreOutputLines,
  redactCoreIdentity
} from "./utils/accountIdentity"
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
  stopCoreForShutdown
} from "./utils/coreShutdown"
import {
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
import {
  CoreModuleStatus,
  FELauncherActionOnGameLaunch
} from "@gd/core_module/bindings"

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

let isGameRunning = false
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
      // The core's own stdout/stderr up to and including `_STATUS_:READY`.
      // The listeners that populate this stop appending once `started`
      // flips true (see `loadCoreModule`), bounding this array from that
      // point on; every fatal path bounds it too, for a different reason —
      // the core process itself has exited, so no further `data` events
      // fire regardless of `started`. Bounded on every path except one: the
      // hung-startup timeout (`setTimeout` below) resolves with `started`
      // still false while the core stays alive by design
      // (`coreProcessHandle`'s own comment), so this buffer keeps growing
      // there for as long as the core does. The listeners themselves stay
      // attached in every case to keep handling later events
      // (`_INSTANCE_STATE_`, account email, ...). `main.tsx` ignores this on
      // the success path, but it is the only record of a *non-fatal* status
      // line — `DB_DOWNGRADED` chief among them — that startup otherwise
      // never surfaces anywhere observable once the app is up. Exposed so
      // `getCoreModule` can answer "what did the core actually report"
      // regardless of outcome, not only on failure.
      logs: Log[]
    }
  | {
      type: "error"
      logs: Log[]
      // Present only for `DB_DOWNGRADE_FAILED`: the pre-downgrade snapshot the
      // failure screen can offer to restore.
      snapshotPath?: string
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

// Terminate the core (if still running) and wait, briefly, for it to exit.
// Recovery must release the database file before deleting or overwriting it;
// on Windows the holding process has to have exited first.
async function killCoreProcess(timeoutMs = 5000): Promise<void> {
  const proc = coreProcessHandle
  if (!proc) {
    return
  }
  if (proc.exitCode !== null || proc.signalCode !== null) {
    return
  }
  await new Promise<void>((resolve) => {
    let settled = false
    const finish = () => {
      if (settled) {
        return
      }
      settled = true
      resolve()
    }
    proc.once("exit", finish)
    const timer = setTimeout(finish, timeoutMs)
    if (typeof timer.unref === "function") {
      timer.unref()
    }
    try {
      proc.kill()
    } catch {
      finish()
    }
  })
}

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
        },
        logs: []
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

    const args = buildCoreModuleArgs({
      runtimePath: CURRENT_RUNTIME_PATH!,
      baseApi: overrideBaseApi?.value,
      e2eAuthBase: e2eAuthBase?.value,
      e2eEntitlementKey: e2eEntitlementKey?.value
    })

    const identitySession = crypto.randomUUID()
    accountIdentity.beginCore(identitySession)
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

    const coreOutputLines = new CoreOutputLines()
    coreModule.stdout.on("data", (data) => {
      const completeLines = coreOutputLines.push(data.toString())
      if (!completeLines.length) return
      const dataString = completeLines.join("\n")

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
      const sanitized = redactCoreIdentity(dataString).replace(
        /(_STATUS_:READY\|\d+\|)[^\s|]+/g,
        "$1<redacted>"
      )

      console.log(`[CORE] Message: ${sanitized}`)

      const rows = dataString.split(/\r?\n|\r|\n/g)

      // Bounds `logs` to output observed no later than `_STATUS_:READY` on
      // the success path, or no later than the core's own exit on every
      // fatal path (see the `CoreModule` type's own comment on `logs` for
      // why those are the only two cases this bounds — the hung-startup
      // timeout does not). This buffer is exposed over IPC via
      // `getCoreModule`, unconditionally, on every call, so once one of
      // those two things has happened it must not keep accumulating for the
      // rest of the process's life. The dispatch loop below still runs
      // unconditionally after this, since `_INSTANCE_STATE_`/account-email/
      // close-warning handling must keep working for as long as the core
      // runs.
      if (!started) {
        logs.push({
          type: "info",
          message: sanitized
        })
      }

      for (const row of rows) {
        if (row.startsWith("_STATUS_:")) {
          // Strip only the `_STATUS_:` prefix rather than splitting on every
          // colon: a payload may itself contain a colon (a Windows snapshot
          // path like `C:\...` in `DB_DOWNGRADE_FAILED|C:\...`).
          const rightPart = row.slice("_STATUS_:".length)
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
              },
              logs
            })
          } else if (event === "BACKWARDS_MIGRATION") {
            console.log("[CORE] Backwards migration detected")
            resolve({
              type: "backwardsMigration",
              logs
            })
          } else if (event === "DB_DOWNGRADED") {
            // A newer database was stepped back to this build's version and
            // verified. Non-fatal: startup continues to READY.
            console.log("[CORE] Database downgraded to this version")
          } else if (
            event === "DB_MIGRATION_FAILED" ||
            event === "DB_DIVERGED" ||
            event === "DB_CORRUPT" ||
            event === "DB_DOWNGRADE_FAILED"
          ) {
            // Fatal database outcomes. The core emits exactly one of
            // these then exits; surface the failure screen with the recovery
            // ladder. `DB_DOWNGRADE_FAILED` carries a pre-downgrade snapshot
            // path, which unlocks the "Restore snapshot" step.
            const snapshotPath =
              event === "DB_DOWNGRADE_FAILED" ? parts[1] : undefined
            console.error(
              `[CORE] Fatal database error: ${event}${snapshotPath ? ` (snapshot: ${snapshotPath})` : ""}`
            )
            resolve({
              type: "error",
              logs: [
                ...logs,
                {
                  type: "error",
                  message: `Database error: ${event}`
                }
              ],
              snapshotPath
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
          const action = rightPart.split("|")[1] as FELauncherActionOnGameLaunch

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
          accountIdentity.acceptPayload(
            row.slice("_GDL_ACCOUNT_EMAIL_:".length)
          )
        }
      }
    })

    coreModule.stderr.on("data", (data) => {
      // Same READY boundary as the stdout listener above, but unlike it,
      // nothing here is redacted before being pushed to `logs` (exposed over
      // IPC on every outcome, including success — see the `CoreModule` type
      // comment). This is deliberately not the same risk as the stdout
      // listener's redaction: `tracing` writes to the release build's file
      // appender, never to stderr (`logger.rs`'s `setup_logger`), so nothing
      // this process prints to its own stdout ever reaches this listener by
      // construction. What can land here pre-READY is Rust's own panic
      // output (a `RUST_BACKTRACE=full` backtrace, since `loadCoreModule`
      // sets that env var) and `logger.rs`'s `cleanup_old_logs`, which
      // `eprintln!`s directly on a failed old-log deletion — neither carries
      // anything more sensitive than a local file path or Rust source
      // location, nothing user-identifying, so no redaction is needed here
      // the way the READY token and account email are on the stdout side.
      if (!started) {
        logs.push({
          type: "error",
          message: data.toString()
        })
      }
      console.error(`[CORE] Error: ${data.toString()}`)
    })

    coreModule.on("exit", (code) => {
      console.log(`[CORE] Exit with code: ${code}`)
      coreProcessHandle = null

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

    if (adSizeFallbackTimeout) {
      clearTimeout(adSizeFallbackTimeout)
      adSizeFallbackTimeout = null
    }
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
  new Error(
    "GDLauncher was kept open, so the database was not changed. Close what is still running and try again."
  )


ipcMain.handle("deleteDbAndRestart", async () =>
  withRuntimeOperation(async () => {
    console.log("deleting database and restarting app...")

    // Kill the core FIRST: on Windows, unlinking a file the core still holds
    // open fails, so the process must exit before we delete the database. This
    // also covers a startup that hung before READY, where the process is still
    // alive.
    await killCoreProcess()

    const dbPath = path.join(CURRENT_RUNTIME_PATH!, "gdl_conf.db")

    // Delete the database, its WAL/SHM sidecars, and any stale pre-downgrade
    // snapshot — leaving a sidecar behind would let SQLite reconstruct the old
    // (broken) state on relaunch. Relaunch then lands on the fresh baseline path.
    const targets = [
      dbPath,
      `${dbPath}-wal`,
      `${dbPath}-shm`,
      path.join(CURRENT_RUNTIME_PATH!, "gdl_conf.pre-downgrade.db")
    ]
    for (const target of targets) {
      try {
        await fs.unlink(target)
        console.log(`deleted ${target}`)
      } catch {
        // File might not exist, that's ok
      }
    }

    app.relaunch()
    app.exit()
  })
)

ipcMain.handle(
  "restoreDbSnapshotAndRestart",
  async (_event, snapshotPath: string) =>
    withRuntimeOperation(async () => {
      console.log(`restoring database from snapshot ${snapshotPath}...`)

      // Kill the core FIRST so the database file is not held open while we
      // overwrite it (same Windows open-file constraint as the reset path). This
      // also covers a startup that hung before READY.
      await killCoreProcess()

      const dbPath = path.join(CURRENT_RUNTIME_PATH!, "gdl_conf.db")

      // Only the core-emitted pre-downgrade snapshot beside the database may be
      // restored. Reject any other path so a compromised renderer cannot copy an
      // arbitrary file over the database.
      const expectedSnapshot = path.join(
        CURRENT_RUNTIME_PATH!,
        "gdl_conf.pre-downgrade.db"
      )
      if (path.resolve(snapshotPath) !== path.resolve(expectedSnapshot)) {
        console.error(
          `refusing to restore from unexpected snapshot path: ${snapshotPath}`
        )
        return
      }

      const tmpPath = `${dbPath}.restore-tmp`
      try {
        // Copy to a sibling temp file, then atomically rename it over the
        // database. A crash or error mid-copy then leaves the original database
        // intact rather than a half-written one whose sidecars we're about to
        // drop below.
        await fs.copyFile(snapshotPath, tmpPath)
        await fs.rename(tmpPath, dbPath)
        console.log("snapshot restored over gdl_conf.db")
      } catch (e) {
        console.error("failed to restore snapshot:", e)
        try {
          await fs.unlink(tmpPath)
        } catch {
          // best effort — the temp file may not have been created
        }
        // Leave the existing database and its sidecars untouched rather than
        // relaunching onto a half-restored file.
        return
      }

      // Drop the WAL/SHM sidecars so the restored file is opened as-is rather
      // than replayed against the pre-restore log.
      for (const suffix of ["-wal", "-shm"]) {
        try {
          await fs.unlink(`${dbPath}${suffix}`)
        } catch {
          // File might not exist, that's ok
        }
      }

      app.relaunch()
      app.exit()
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
  async (_, newPath: string, switchOnly = false) => {
    return withRuntimeOperation(async () => {
      if (!CURRENT_RUNTIME_PATH) throw new Error("Runtime is not initialized")
      if (overrideCLIDataPath?.value || overrideEnvDataPath) {
        throw new Error(
          "Remove the explicit runtime path override and restart before relocating"
        )
      }
      const source = CURRENT_RUNTIME_PATH
      const warnings = await relocateRuntime({
        source,
        target: newPath,
        switchOnly,
        pointer: path.join(app.getPath("userData"), RUNTIME_PATH_OVERRIDE_NAME),
        context: runtimeDestinationContext(),
        stopCore: async () => {
          const core = await coreModule
          if (core.type !== "success")
            throw new Error("Core is not ready; restart and retry relocation")
          await stopCoreForRelocation(
            { ...core.result, process: coreProcessHandle },
            source
          )
        },
        progress: (value) =>
          win?.webContents.send("changeRuntimePathProgress", value)
      })
      for (const warning of warnings) console.warn(warning)
      if (warnings.length)
        await dialog.showMessageBox({
          type: "info",
          title: "Runtime location changed",
          message:
            "The new runtime is ready. Review the retained files before restarting.",
          detail: warnings.join("\n\n"),
          buttons: ["Restart"],
          defaultId: 0,
          noLink: true
        })
      app.relaunch()
      app.exit()
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

  // During a quit that is still finishing, the hidden window comes back
  // with its prompt instead of a second window being created.
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
async function prepareOwnedShutdown(purpose: "quit" | "update"): Promise<void> {
  if (shutdownApproved) return
  if (shutdownInFlight) return shutdownInFlight
  shutdownInFlight = (async () => {
    const core = await coreModule
    if (core.type === "success")
      await stopCoreForShutdown(
        { ...core.result, process: coreProcessHandle },
        purpose
      )
    shutdownApproved = true
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
app.on("window-all-closed", () => {
  if (runtimeOperationActive() || isSpawningWindow) return
  app.quit()
})
app.on("before-quit", (event) => {
  if (shutdownApproved) return
  event.preventDefault()
  if (runtimeOperationActive() || shutdownInFlight) return
  void prepareOwnedShutdown("quit")
    .then(() => app.quit())
    .catch(async (error) => {
      await dialog.showMessageBox({
        type: "error",
        title: "Shutdown incomplete",
        message: String(error)
      })
    })
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
