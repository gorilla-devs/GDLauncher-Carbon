import type { ChildProcess } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { _electron as electron } from "playwright"
import type { ElectronApplication, Page } from "playwright"
import { expect, type TestInfo } from "@playwright/test"
import { byTestId, TEST_IDS } from "../helpers/selectors.js"
import { describeLauncherProcesses, isPidAlive } from "../helpers/processes.js"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

/** Adds `note` to `error`'s message. Playwright's reporter prints the
 *  message from the stack, so the stack's copy is extended too. */
export function appendToError(error: unknown, note: string): unknown {
  if (!(error instanceof Error)) return error
  const original = error.message
  error.message = `${original}\n\n${note}`
  if (error.stack?.includes(original))
    error.stack = error.stack.replace(original, () => error.message)
  return error
}

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
  userDataPath?: string
  runId?: string
  baseApi?: string
  e2eAuthBase?: string
  e2eEntitlementKey?: string
  /** Base URL the auto-updater should read its channel file from — the mock
   *  server's `/updates/` mount (see `mock-idp/server.ts`'s
   *  `UPDATE_CHANNEL_YAML`). Passed per launch rather than baked into the
   *  packaged `app-update.yml` because the mock listens on an ephemeral port,
   *  and because that file is one shared resource inside the package that
   *  every worker would otherwise be rewriting under the others. */
  e2eUpdateFeed?: string
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
 *
 * The returned `stdout` array collects every chunk the Electron *main*
 * process itself writes to its own stdout — this already includes the Rust
 * core's own output, since `main/index.ts`'s `coreModule.stdout.on("data",
 * ...)` handler `console.log`s every line the core prints (sanitized: the
 * `_STATUS_:READY` api token and any account email are redacted, nothing
 * else). This is the one place the core's stdout is captured in this
 * harness, so a test asserting on a `_STATUS_:` event should push a listener
 * onto this same array rather than opening a second capture of the core's
 * output. It is a separate channel from `attachCoreLogOnFailure` below,
 * which reads the core's own `tracing`-backed file log rather than this pipe
 * — see that function's doc comment.
 */
export async function launchApp(opts: LaunchOptions): Promise<{
  app: ElectronApplication
  page: Page
  pageErrors: Error[]
  stdout: string[]
}> {
  const binaryPath = getBinaryPath()
  const args = ["--gdl_allow_multiple_instances", "--gdl_disable_sentry"]

  // Passed by environment, not argv: ow-electron 39.8.11 on Windows exits
  // before Node initialises when two argv tokens contain `://`, which killed
  // every launch here. Read by `main/index.ts`'s `argumentOrE2EEnv`. The
  // entitlement key stays an argument — it is a path, not a URL.
  const urlEnv: Record<string, string> = {}
  if (opts.baseApi) {
    urlEnv.GDL_OVERRIDE_BASE_API = opts.baseApi
  }

  if (opts.e2eAuthBase) {
    urlEnv.GDL_E2E_AUTH_BASE = opts.e2eAuthBase
  }

  if (opts.e2eUpdateFeed) {
    urlEnv.GDL_E2E_UPDATE_FEED = opts.e2eUpdateFeed
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
        GDL_RUNTIME_PATH: opts.runtimePath,
        GDL_E2E_USER_DATA:
          opts.userDataPath ??
          path.join(opts.runtimePath, "electron-user-data"),
        GDL_E2E_RUN_ID: opts.runId ?? path.basename(opts.runtimePath),
        ...urlEnv,
        // A fresh runtime dir derives its heap from total RAM, which on a
        // 7 GB CI runner exceeds what is available by launch time. Pin one
        // that fits; overridable for roomier runners.
        GDL_E2E_DEFAULT_XMX_MB: process.env.GDL_E2E_DEFAULT_XMX_MB ?? "1024"
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

      // In run 41652440682 every launch after the database recovery tests
      // died in startup: the runner had turned busy while launchers those
      // tests relaunched were still running. Name what is running, with its
      // CPU time, in the failure itself.
      const running = `Launcher processes running at the failed launch:\n${describeLauncherProcesses([binaryPath, getCoreModulePath()])}`
      console.error(running)
      throw appendToError(error, running)
    })

  rememberElectronProcess(app)
  app.on("console", (msg) => console.log(msg.text()))
  const stdout: string[] = []
  app.process().stdout?.on("data", (data) => {
    const chunk = data.toString()
    stdout.push(chunk)
    console.log(chunk)
  })
  app.process().stderr?.on("data", (data) => console.log(data.toString()))

  const page = await app.firstWindow()
  page.on("console", (msg) => console.log(msg.text()))

  // A failing request otherwise reaches the log as a bare "Failed to load
  // resource: 500" with no URL, which names nothing.
  page.on("response", (response) => {
    if (response.status() >= 400) {
      console.log(
        `[http ${response.status()}] ${response.request().method()} ${response.url()}`
      )
    }
  })

  const pageErrors: Error[] = []
  page.on("pageerror", (error) => {
    console.error(error)
    pageErrors.push(error)
  })

  return { app, page, pageErrors, stdout }
}

/**
 * Reads the OS pid of the currently-spawned core module process straight out
 * of the Electron main process's own module state.
 *
 * The core is a grandchild of the Playwright test process (main spawns it,
 * Playwright spawns main), so there is no direct handle to it here. `main
 * /index.ts` puts the pid on `globalThis.__gdlCoreProcessId` right after
 * `spawn()` specifically so this function can read it back via
 * `ElectronApplication.evaluate`, which runs the callback inside the target
 * app's own main-process context rather than this one.
 *
 * Returns `null` only for a dev-mode launch (no core process is spawned at
 * all — `loadCoreModule` short-circuits) or a core that failed to spawn.
 * Neither applies to the packaged, `-e2e` builds this suite drives.
 */
export async function getCoreProcessId(
  app: ElectronApplication
): Promise<number | null> {
  return app.evaluate(
    () =>
      (globalThis as Record<string, unknown>).__gdlCoreProcessId as
        | number
        | null
  )
}

/**
 * Polls until `pid` no longer refers to a live process, or throws once
 * `timeoutMs` elapses.
 *
 * `process.kill(pid, 0)` is Node's documented cross-platform existence probe:
 * signal `0` sends nothing, it just asks the OS whether the pid is still
 * addressable, and Node normalizes the "gone" case to an `ESRCH` error on
 * every platform, Windows included (there `kill()` maps to `TerminateProcess`
 * / a handle-based existence check rather than a real POSIX signal, but the
 * pid-existence contract to callers is the same). Polling this instead of a
 * fixed sleep is the point: `crates/carbon_app/src/main.rs`'s termination
 * handler (~line 317-340) shuts down running servers/instances concurrently,
 * bounded to ~3s, before calling `std::process::exit(0)` — genuinely
 * variable, and a wait that isn't tied to the real exit would either race a
 * slow shutdown or waste time on a fast one.
 *
 * Exported so a spec that kills the core *itself* can wait for the kill to
 * land before acting on the result — `modpackInterruptedStaging.spec.ts`
 * SIGKILLs the core mid-download and must know the process is really gone
 * before it inspects the half-written `.setup/` it left behind.
 */
export async function waitForPidExit(
  pid: number,
  { timeoutMs = 15_000, pollIntervalMs = 50 } = {}
): Promise<void> {
  const deadline = Date.now() + timeoutMs

  for (;;) {
    try {
      process.kill(pid, 0)
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ESRCH") {
        return
      }
      // Anything else (e.g. EPERM, meaning the pid exists but isn't ours to
      // signal) is treated as "still alive" and falls through to the next
      // poll — it is not the "gone" signal this function waits for.
    }

    if (Date.now() >= deadline) {
      throw new Error(
        `core process (pid ${pid}) did not exit within ${timeoutMs}ms of app.close(). ` +
          "Relaunching against the same runtime path now would race the old process's SQLite handles " +
          "and surface as a locked-database failure that looks like corruption but isn't."
      )
    }

    await new Promise((resolve) => setTimeout(resolve, pollIntervalMs))
  }
}

/** Each app's main process, kept from launch: once an ElectronApplication
 *  closes, Playwright drops the object behind `app.process()`, which then
 *  throws instead of returning the exited process. */
const electronProcesses = new WeakMap<ElectronApplication, ChildProcess>()

/** Keeps `app`'s main process for `electronProcess`. Safe to call again. */
export function rememberElectronProcess(app: ElectronApplication): void {
  if (electronProcesses.has(app)) return
  try {
    electronProcesses.set(app, app.process())
  } catch {
    // Already closed, or a test double without a process.
  }
}

/** `app`'s main process, also after the app has closed. */
export function electronProcess(app: ElectronApplication): ChildProcess {
  return electronProcesses.get(app) ?? app.process()
}

/**
 * Waits (bounded) for the Electron main process to exit and returns how it
 * exited. `app.close()` resolves after the exit, so this normally returns at
 * once.
 */
export async function electronExitStatus(
  app: ElectronApplication,
  timeoutMs = 30_000
): Promise<{ code: number | null; signal: NodeJS.Signals | null }> {
  const child = electronProcess(app)
  if (child.exitCode === null && child.signalCode === null) {
    await new Promise<void>((resolve) => {
      const timer = setTimeout(resolve, timeoutMs)
      child.once("exit", () => {
        clearTimeout(timer)
        resolve()
      })
    })
  }
  return { code: child.exitCode, signal: child.signalCode }
}

/** Fails unless Electron exited with code 0. The harness runs with
 *  `--gdl_disable_sentry`, so an uncaught main-process exception reaches
 *  `handleUncaughtException`, which exits with 1. */
export function assertCleanElectronExit(status: {
  code: number | null
  signal: NodeJS.Signals | null
}): void {
  if (status.code === 0) return
  throw new Error(
    `Electron exited with code ${status.code}` +
      (status.signal ? ` (signal ${status.signal})` : "") +
      " instead of 0. An uncaught main-process exception exits with 1; " +
      'look for "Uncaught Exception" in main.log.'
  )
}

/**
 * The launcher window that shows `page`, for closing it the way the window
 * manager's X does and for reading whether it is shown.
 *
 * Never `BrowserWindow.getAllWindows()[0]`: ow-electron keeps hidden 1x1
 * windows of its own (the Overwolf consent window for about 10 s after start
 * outside the EU, and on Windows its package manager's for the whole
 * session), and Electron lists the newest window first. On CI that closed
 * the consent window instead of the launcher.
 */
export async function launcherWindow(app: ElectronApplication, page: Page) {
  const handle = await app.browserWindow(page)
  return {
    /** Rejections are swallowed: the app may already be exiting. */
    close: () => handle.evaluate((w) => w.close()).catch(() => {}),
    /** False once the window is gone. */
    visible: () =>
      handle
        .evaluate((w) => !w.isDestroyed() && w.isVisible())
        .catch(() => false)
  }
}

/**
 * Closes `app` and returns only once the core module it spawned has actually
 * exited.
 *
 * `app.close()` resolves when Electron is gone, but the core is a separate
 * child process that outlives it — briefly, and sometimes for well over ten
 * seconds — holding SQLite handles under the runtime path. Deleting that path
 * before the core lets go fails with EPERM out of teardown, which reads as a
 * test failure long after every assertion has passed.
 *
 * Teardown-only, so a core that never exits is reported rather than thrown:
 * masking a real test failure behind a cleanup error is worse than leaking a
 * directory the next run's sweep will collect. `strict` propagates an
 * unconfirmed app/core exit so deleting callers can retain the owned runtime,
 * and an Electron exit code other than 0 (an uncaught main-process exception
 * exits with 1), which a non-strict close only reports.
 */
export async function closeAppAndCore(
  app: ElectronApplication,
  {
    timeoutMs = 30_000,
    strict = false,
    waitForExit = waitForPidExit
  }: {
    timeoutMs?: number
    strict?: boolean
    waitForExit?: typeof waitForPidExit
  } = {}
): Promise<void> {
  const corePid = await getCoreProcessId(app).catch(() => null)
  let failed = false
  let firstError: unknown
  const record = (error: unknown) => {
    if (!failed) {
      failed = true
      firstError = error
    } else console.error("closeAppAndCore:", error)
  }
  rememberElectronProcess(app)
  let timeout: ReturnType<typeof setTimeout> | undefined
  let closed = false
  try {
    await Promise.race([
      app.close(),
      new Promise<never>((_, reject) => {
        timeout = setTimeout(
          () => reject(new Error("Electron close timed out; exit unconfirmed")),
          timeoutMs
        )
      })
    ])
    closed = true
  } catch (error) {
    record(error)
  } finally {
    clearTimeout(timeout)
  }
  if (closed) {
    try {
      assertCleanElectronExit(await electronExitStatus(app, timeoutMs))
    } catch (error) {
      record(error)
    }
  }
  if (corePid !== null) {
    try {
      await waitForExit(corePid, { timeoutMs })
    } catch (error) {
      record(error)
    }
  } else if (strict) {
    record(new Error("Owned core exit could not be confirmed"))
  }
  if (failed) {
    if (strict) throw firstError
    console.error("closeAppAndCore:", firstError)
  }
}

/**
 * Quits the launcher the way a user does while a game is running, and returns
 * once both the Electron main process and the core it spawned are gone.
 *
 * Two things make this different from `app.close()`, and both were found the
 * hard way:
 *
 * 1. **A close while a game runs does not close anything.** `main/index.ts`'s
 *    `win.on("close")` calls `preventDefault()` and asks the renderer for the
 *    `WindowCloseWarning` modal instead; the modal's confirm button
 *    (`window.closeWindow()`) is the only thing that finishes the quit. An
 *    `app.close()` here waits on an exit the modal is holding up — observed
 *    leaving the app alive twenty minutes later.
 * 2. **`app.close()` cannot be awaited afterwards either.** Playwright pairs
 *    it with `attemptToGracefullyClose` and resolves on the spawned process's
 *    `close` event; against an app already quitting itself, that was observed
 *    hanging for the rest of the test budget with the core long since exited.
 *
 * So the exit is observed where it actually happens — the OS pids — and the
 * caller must treat its `ElectronApplication` as dead on return and never
 * call `close()` on it.
 *
 * Throws if the core pid cannot be read: without it there is no way to tell a
 * game that outlived the launcher from one that merely outraced its shutdown.
 */
export async function quitLauncherMidGame(
  current: { app: ElectronApplication; page: Page },
  { timeoutMs = 60_000 } = {}
): Promise<void> {
  const electronPid = electronProcess(current.app).pid
  if (electronPid == null) {
    throw new Error(
      "quitLauncherMidGame: Playwright reported no pid for the Electron process"
    )
  }

  const corePid = await getCoreProcessId(current.app)
  if (corePid == null) {
    throw new Error(
      "quitLauncherMidGame: could not read the core process id — cannot " +
        "confirm the launcher actually exited"
    )
  }

  // The window-manager's X, as the main process sees it.
  await (await launcherWindow(current.app, current.page)).close()

  const confirm = current.page.locator(
    byTestId(TEST_IDS.windowCloseWarningQuit)
  )
  await expect(
    confirm,
    "closing the launcher mid-game did not raise the close warning — the " +
      "quit path this drives no longer exists"
  ).toBeVisible()

  // The click destroys the window that is handling it, so Playwright's own
  // post-click bookkeeping can land on an already-closed page and reject.
  // Swallowed rather than guarded because the click having taken effect is
  // exactly what the two waits below assert.
  await confirm.click().catch(() => {})

  for (const [name, pid] of [
    ["core", corePid],
    ["Electron main process", electronPid]
  ] as const) {
    await expect
      .poll(() => isPidAlive(pid), {
        timeout: timeoutMs,
        message:
          `the ${name} (pid ${pid}) was still alive ${timeoutMs}ms after ` +
          "the quit was confirmed"
      })
      .toBe(false)
  }
}

/**
 * Closes `current.app` and relaunches on the same runtime path, only after
 * confirming the core process it owned has genuinely exited.
 *
 * `app.close()` resolving only means the *Electron* main process is gone —
 * `window-all-closed` in `main/index.ts` fires `coreModule.kill()` and calls
 * `app.quit()` without awaiting the core's own `exit` event, so the core
 * (holding the SQLite connections everything in the next task reads back)
 * can genuinely still be alive after `close()` returns. Relaunching before it
 * has released those handles is exactly the "looks like corruption" failure
 * mode this exists to prevent.
 *
 * Reused SQLite's own `busy_timeout = 5000` (`crates/carbon_repos/src
 * /db_exec.rs`) is the second layer, not this function's job to duplicate:
 * even if the OS is still lazily tearing down file handles after the process
 * itself is confirmed dead (documented for Windows in `mockIdp.ts`'s
 * `stopHarness`), the newly-spawned core's own connection open absorbs a
 * brief overlap by retrying internally rather than failing immediately.
 *
 * Throws rather than silently launching anyway if the pid can't be read at
 * all (see `getCoreProcessId`) — proceeding without having verified the old
 * process is gone would make every later persistence assertion meaningless
 * while still looking green.
 */
export async function relaunchApp(
  current: { app: ElectronApplication; page: Page },
  opts: LaunchOptions
): Promise<{
  app: ElectronApplication
  page: Page
  pageErrors: Error[]
  stdout: string[]
}> {
  const corePidBefore = await getCoreProcessId(current.app)
  if (corePidBefore == null) {
    throw new Error(
      "relaunchApp: could not read the outgoing app's core process id — " +
        "cannot verify it actually exited before relaunching onto the same runtime path"
    )
  }

  await current.app.close()
  await waitForPidExit(corePidBefore)

  return launchApp(opts)
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
