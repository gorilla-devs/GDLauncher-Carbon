import { it, expect, vi } from "vitest"
import fs from "node:fs"
import path from "node:path"
import vm from "node:vm"
import { EventEmitter } from "node:events"
import ts from "typescript"
import { createCoreOutput } from "./coreOutput"
import { AccountIdentityReceiver } from "./accountIdentity"
import { ProtocolQueue } from "./protocolQueue"
import { DatabaseRefusal } from "./databaseRestore"
import {
  LauncherLifecycle,
  mainWindowClosed,
  userWindows
} from "./windowLifecycle"
import { ShutdownPrompts } from "./shutdownPrompt"
import {
  ShutdownFlow,
  ShutdownNotPossibleError,
  type ShutdownView
} from "./shutdownFlow"
import {
  ShutdownBlockedError,
  cancelCoreShutdown,
  forceTerminateCore,
  stopCoreForShutdown,
  terminateAndObserveCoreExit
} from "./coreShutdown"
import {
  interruptRuntimeOperation,
  interruptibleUntilCommitted,
  runtimeOperationActive,
  runtimeOperationBrief,
  whenRuntimeOperationIdle,
  withRuntimeOperation
} from "./runtimeOperation"

const root = path.resolve(__dirname, "../../../../..")
function source(relative: string) {
  return fs.readFileSync(path.join(root, relative), "utf8")
}
function initializer(text: string, name: string) {
  const file = ts.createSourceFile(
    "fixture.tsx",
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  )
  let value: string | undefined
  function visit(node: ts.Node) {
    if (ts.isVariableDeclaration(node) && node.name.getText(file) === name)
      value = node.initializer?.getText(file)
    ts.forEachChild(node, visit)
  }
  visit(file)
  if (!value) throw Error("Actual production initializer unavailable: " + name)
  return value
}
function callArgument(text: string, expression: string, argument: string) {
  const file = ts.createSourceFile(
    "fixture.tsx",
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  )
  let value: string | undefined
  function visit(node: ts.Node) {
    if (
      ts.isCallExpression(node) &&
      node.expression.getText(file) === expression &&
      node.arguments[0]?.getText(file) === JSON.stringify(argument)
    )
      value = node.arguments[1].getText(file)
    ts.forEachChild(node, visit)
  }
  visit(file)
  if (!value) throw Error("Actual production callback unavailable: " + argument)
  return value
}
function evaluate(text: string, context: Record<string, unknown>) {
  return vm.runInNewContext(
    ts.transpileModule("(" + text + ")", {
      compilerOptions: {
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.CommonJS,
        jsx: ts.JsxEmit.React,
        jsxFactory: "h"
      }
    }).outputText,
    context
  )
}
it("Task23 actual main output integration returns whole credentials and redacted IPC logs while forwarding current identity", () => {
  const text = source("apps/desktop/packages/main/index.ts"),
    logs: string[] = [],
    results: any[] = [],
    hashes: unknown[] = []
  const accountIdentity = new AccountIdentityReceiver((h) => hashes.push(h))
  accountIdentity.beginCore("fixture")
  accountIdentity.setReady(true)
  const lifecycle = { snapshot() {}, event() {} }
  const context = {
    createCoreOutput,
    clearTimeout() {},
    startupTimer: 1,
    console: { log: (s: string) => logs.push(s) },
    resolve: (r: any) => results.push(r),
    coreModule: null,
    getWin: () => null,
    lastCoreModuleProgress: null,
    identitySession: "fixture",
    accountIdentity,
    lifecycle,
    showAppCloseWarning: true
  }
  const core = evaluate(initializer(text, "coreOutput"), context)
  const handler = evaluate(callArgument(text, "coreModule.stdout.on", "data"), {
    coreOutput: core
  })
  const record = Buffer.from(
    '_STATUS_:READY|1234|synthetic-token\n_GDL_ACCOUNT_EMAIL_:{"session":"fixture","revision":1,"selected":"fixture","email":"café@example.invalid"}\n'
  )
  for (const byte of record) handler(Buffer.from([byte]))
  expect(results[0].result.apiToken).toBe("synthetic-token")
  expect(hashes).toHaveLength(2)
  expect(JSON.stringify(logs)).not.toContain("synthetic-token")
  expect(JSON.stringify(logs)).not.toContain("café@example.invalid")
  expect(JSON.stringify(results[0].logs)).not.toContain("synthetic-token")
})
it("Task23 actual core-load guard refuses interrupted restore before any spawn or development connection", async () => {
  const load = evaluate(
    initializer(
      source("apps/desktop/packages/main/index.ts"),
      "loadCoreModule"
    ),
    {
      CURRENT_RUNTIME_PATH: "/synthetic",
      assertDatabaseRestoreReadySync: () => {
        throw new DatabaseRefusal("database recovery required")
      },
      DatabaseRefusal,
      isDev: false,
      spawn: () => {
        throw Error("native launch forbidden")
      }
    }
  )
  const result = await load()
  expect(result.type).toBe("error")
  expect(result.logs[0].message).toContain("recovery")
  // The startup screen offers Recover Database for this one.
  expect(result.dbRecovery).toBe("interrupted_restore")
})
it("a marker the guard cannot check is not offered as an interrupted restore", async () => {
  const load = evaluate(
    initializer(
      source("apps/desktop/packages/main/index.ts"),
      "loadCoreModule"
    ),
    {
      CURRENT_RUNTIME_PATH: "/synthetic",
      assertDatabaseRestoreReadySync: () => {
        throw Error("EACCES")
      },
      DatabaseRefusal,
      isDev: false,
      spawn: () => {
        throw Error("native launch forbidden")
      }
    }
  )
  const result = await load()
  expect(result.type).toBe("error")
  expect(result.dbRecovery).toBeUndefined()
})
it("Task23 actual main protocol IPC validates current sender frame before ready and ACK", () => {
  const text = source("apps/desktop/packages/main/index.ts"),
    sender = { mainFrame: {} },
    otherFrame = {},
    deliveries: any[] = []
  const protocolQueue = new ProtocolQueue<object>((_, value) =>
    deliveries.push(value)
  )
  protocolQueue.bind(sender)
  protocolQueue.enqueue("gdlauncher://share/FIXTURE")
  const ready = evaluate(
    callArgument(text, "ipcMain.on", "protocol-consumer-ready"),
    { protocolQueue }
  )
  ready(
    { sender, senderFrame: otherFrame },
    { subscription: "1", generation: protocolQueue.session(sender) }
  )
  expect(deliveries).toHaveLength(0)
  ready(
    { sender, senderFrame: sender.mainFrame },
    { subscription: "1", generation: protocolQueue.session(sender) }
  )
  expect(deliveries).toHaveLength(1)
  const ack = evaluate(
    callArgument(text, "ipcMain.on", "protocol-consumer-ack"),
    { protocolQueue }
  )
  ack({ sender, senderFrame: otherFrame }, deliveries[0])
  protocolQueue.revoke(sender)
  ready(
    { sender, senderFrame: sender.mainFrame },
    { subscription: "2", generation: protocolQueue.session(sender) }
  )
  expect(deliveries).toHaveLength(2)
})
it("Task23 actual main-window update consumer clears busy/manual state and Copy Link uses active artifact", async () => {
  let checking = true,
    manual = true,
    action: any,
    clipboard = ""
  const toast = Object.assign(
    (_title: unknown, data: any) => {
      action = data.action
    },
    {
      dismiss() {},
      error() {
        shown.push("error")
      },
      success() {},
      loading() {},
      info() {
        shown.push("info")
      }
    }
  )
  const context: Record<string, unknown> = {
    lastUpdateRevision: -1,
    setHasPendingUpdate() {},
    setPendingUpdateVersion() {},
    pendingReady: undefined,
    lastUpdatePhase: "idle",
    installingPresentation: undefined,
    setIsCheckingForUpdates: (v: boolean) => {
      checking = v
    },
    setIsManualCheck: (v: boolean) => {
      manual = v
    },
    isManualCheck: () => manual,
    isShowingDownloadToast: showing,
    setDownloadProgress() {},
    setDownloadingVersion() {},
    supportsAutoUpdate: () => false,
    toast,
    i18n: { t: (s: string) => s },
    updateErrorText: () => "description",
    TOAST_ID_CHECKING: "checking",
    TOAST_ID_MANUAL_UPDATE: "manual",
    TOAST_ID_DOWNLOADING: "download",
    ManualUpdateTitle: "title",
    ManualUpdateDescription: "description",
    CopyLinkButtonLabel: "copy",
    h: (...args: unknown[]) => args,
    navigator: {
      clipboard: {
        writeText: async (s: string) => {
          clipboard = s
        }
      }
    },
    console: { error() {} }
  }
  const receive = evaluate(
    initializer(
      source("apps/desktop/packages/mainWindow/src/utils/updater.tsx"),
      "receiveUpdateState"
    ),
    context
  )
  receive({ state: "unsupported", revision: 2, error: { message: "retry" } })
  expect(checking).toBe(false)
  expect(manual).toBe(false)
  // The player asked: a notice, not a failure.
  expect(shown).toEqual(["info"])
  receive({ state: "checking", revision: 1 })
  expect(checking).toBe(false)
  receive({
    state: "manual-update",
    revision: 3,
    updateInfo: {
      version: "2",
      downloadUrl: "https://active.invalid/GDLauncher__2__linux__x64.zip"
    }
  })
  await action.onClick()
  expect(clipboard).toBe("https://active.invalid/GDLauncher__2__linux__x64.zip")
})
it("Task23 actual recovery broadcast/hydration reducer resets unsupported button and manual clipboard action", async () => {
  const text = source("apps/desktop/packages/preload/loading.ts"),
    button = { textContent: "Checking...", disabled: true, style: {} },
    status = { textContent: "", style: {}, innerHTML: "" }
  let copied = ""
  const context: Record<string, any> = {
    lastUpdateRevision: -1,
    readyDownloadId: undefined,
    updateState: "checking",
    updateInfo: null,
    updateStatusText: status,
    updateProgressContainer: { style: {} },
    updateProgressBar: { style: {} },
    checkUpdatesBtn: button,
    navigator: {
      clipboard: {
        writeText: async (s: string) => {
          copied = s
        }
      }
    }
  }
  const set = evaluate(initializer(text, "setUpdateState"), context)
  context.setUpdateState = set
  const receive = evaluate(initializer(text, "receiveUpdateState"), context)
  receive({
    state: "unsupported",
    revision: 2,
    error: { message: "Manual update unavailable; retry" }
  })
  expect(button.disabled).toBe(false)
  expect(button.textContent).toBe("Retry")
  expect(status.textContent).toContain("retry")
  receive({ state: "checking", revision: 1 })
  expect(button.disabled).toBe(false)
  expect(context.updateInfo).toBeNull()
  receive({
    state: "manual-update",
    revision: 3,
    updateInfo: {
      version: "2",
      downloadUrl: "https://active.invalid/fixture.zip"
    }
  })
  const click = evaluate(
    callArgument(text, "checkUpdatesBtn.addEventListener", "click"),
    context
  )
  await click()
  expect(copied).toBe("https://active.invalid/fixture.zip")
})
it("Task23 actual updater wrapper null branch hydrates a packaged-provider manual artifact without ambient feed env", async () => {
  const handlers: Record<string, (...args: any[]) => any> = {},
    events: Record<string, (...args: any[]) => any> = {},
    broadcast: any[] = []
  const info = {
    version: "2.0.0",
    files: [{ url: "GDLauncher__2.0.0__linux__x64.zip", sha512: "synthetic" }]
  }
  const updater: any = {
    checkForUpdates: async () => null,
    on: (event: string, handler: any) => {
      events[event] = handler
    },
    clientPromise: Promise.resolve({
      getLatestVersion: async () => info,
      resolveFiles: () => [
        {
          url: new URL(
            "https://packaged.invalid/feed/GDLauncher__2.0.0__linux__x64.zip"
          ),
          info: info.files[0]
        }
      ]
    }),
    quitAndInstall: () => {
      throw Error("inactive install forbidden")
    }
  }
  const exports = {},
    mocks: Record<string, unknown> = {
      electron: {
        ipcMain: {
          handle: (name: string, handler: any) => {
            handlers[name] = handler
          }
        },
        BrowserWindow: {
          getAllWindows: () => [
            {
              isDestroyed: () => false,
              webContents: {
                send: (_: string, value: any) => broadcast.push(value)
              }
            }
          ]
        }
      },
      "electron-updater": { autoUpdater: updater },
      "electron-log/main": { transports: { file: {} } },
      "node:fs/promises": {
        readFile: async () => {
          throw Object.assign(Error("missing marker"), { code: "ENOENT" })
        }
      },
      "node:path": path,
      "./utils/runtimeOperation": { assertRuntimeOperationAvailable() {} }
    }
  const { activeUpdateMetadata } = await import("./updateProvider"),
    { createUpdaterController } = await import("./updaterController")
  mocks["./utils/updateProvider"] = { activeUpdateMetadata }
  mocks["./utils/updaterController"] = { createUpdaterController }
  mocks.semver = { gt: () => true }
  mocks["./utils/downloadedUpdate"] = await import("./downloadedUpdate")
  mocks["./utils/ownedInstallDispatch"] = await import("./ownedInstallDispatch")
  mocks["./utils/appImageInstall"] = await import("./appImageInstall")
  const compiled = ts.transpileModule(
    source("apps/desktop/packages/main/autoUpdater.ts"),
    {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        esModuleInterop: true
      }
    }
  ).outputText
  vm.runInNewContext(compiled, {
    exports,
    require: (name: string) => {
      if (!(name in mocks))
        throw Error("Unexpected actual wrapper dependency: " + name)
      return mocks[name]
    },
    __APP_VERSION__: "1.0.0",
    process: {
      platform: "linux",
      arch: "x64",
      resourcesPath: "/synthetic-packaged",
      env: {}
    }
  })
  ;(exports as any).default()
  expect(handlers.getUpdateState().state).toBe("idle") // handlers registered before async installation identification
  const result = await handlers.checkForUpdates(null, "stable")
  expect(result.status).toBe("manual-update")
  expect(handlers.getUpdateState().updateInfo.downloadUrl).toBe(
    "https://packaged.invalid/feed/GDLauncher__2.0.0__linux__x64.zip"
  )
  expect(broadcast.at(-1)).toBe(result.currentState)
})
it("Task36 actual fatal consumer keeps positive token beside an error and Retry invokes the displayed artifact", async () => {
  const text = source("apps/desktop/packages/preload/loading.ts"),
    button = { textContent: "Install", disabled: false, style: {} },
    status = { textContent: "", style: {}, innerHTML: "" }
  let installToken: number | undefined
  const context: Record<string, any> = {
    lastUpdateRevision: -1,
    readyDownloadId: undefined,
    updateState: "ready",
    updateInfo: null,
    updateStatusText: status,
    updateProgressContainer: { style: {} },
    updateProgressBar: { style: {} },
    checkUpdatesBtn: button,
    installReady: async () => {
      installToken = context.readyDownloadId
    }
  }
  context.setUpdateState = evaluate(
    initializer(text, "setUpdateState"),
    context
  )
  const receive = evaluate(initializer(text, "receiveUpdateState"), context)
  receive({
    state: "error",
    revision: 5,
    readyUpdate: { id: 3, version: "2.0.0" },
    error: { message: "Installer rejected; retry" }
  })
  expect(status.textContent).toContain("Installer rejected")
  expect(button.textContent).toBe("Retry")
  expect(context.readyDownloadId).toBe(3)
  await evaluate(
    callArgument(text, "checkUpdatesBtn.addEventListener", "click"),
    context
  )()
  expect(installToken).toBe(3)
})
it("Task36 actual main wrapper owns a queued install callback through error before a successor completion", async () => {
  const handlers: Record<string, (...args: any[]) => any> = {},
    events: Record<string, (...args: any[]) => any> = {},
    broadcast: any[] = []
  const native = new EventEmitter(),
    baseline = () => {},
    obsolete = () => {
      throw Error("Retired install callback invoked")
    }
  native.on("update-downloaded", baseline)
  const info = {
    version: "2.0.0",
    path: "fixture.zip",
    sha512: "owned",
    releaseDate: "2026-10-01",
    files: [{ url: "fixture.zip", sha512: "owned" }]
  }
  const helper = {
    file: "/synthetic/fixture.zip",
    versionInfo: info,
    fileInfo: { info: info.files[0] },
    downloadedFileInfo: { sha512: "owned" }
  }
  const updater: any = {
    nativeUpdater: native,
    downloadedUpdateHelper: helper,
    checkForUpdates: async () => {
      events["update-available"](info)
      events["update-downloaded"]()
      return {}
    },
    on: (event: string, handler: any) => {
      events[event] = handler
    },
    quitAndInstall: (silent: boolean, run: boolean) => {
      expect([silent, run]).toEqual([true, true])
      native.on("update-downloaded", obsolete)
    }
  }
  const exports = {},
    mocks: Record<string, unknown> = {
      electron: {
        ipcMain: {
          handle: (name: string, handler: any) => {
            handlers[name] = handler
          }
        },
        BrowserWindow: {
          getAllWindows: () => [
            {
              isDestroyed: () => false,
              webContents: {
                send: (_: string, value: any) => broadcast.push(value)
              }
            }
          ]
        }
      },
      "electron-updater": { autoUpdater: updater },
      "electron-log/main": { transports: { file: {} } },
      "node:fs/promises": {
        readFile: async () => {
          throw Object.assign(Error("missing marker"), { code: "ENOENT" })
        }
      },
      "node:path": path,
      "./utils/runtimeOperation": { assertRuntimeOperationAvailable() {} },
      semver: { gt: () => true },
      "./utils/updateProvider": await import("./updateProvider"),
      "./utils/updaterController": await import("./updaterController"),
      "./utils/downloadedUpdate": await import("./downloadedUpdate"),
      "./utils/ownedInstallDispatch": await import("./ownedInstallDispatch"),
      "./utils/appImageInstall": await import("./appImageInstall")
    }
  const compiled = ts.transpileModule(
    source("apps/desktop/packages/main/autoUpdater.ts"),
    {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        esModuleInterop: true
      }
    }
  ).outputText
  vm.runInNewContext(compiled, {
    exports,
    require: (name: string) => {
      if (!(name in mocks))
        throw Error("Unexpected actual wrapper dependency: " + name)
      return mocks[name]
    },
    __APP_VERSION__: "1.0.0",
    process: {
      platform: "linux",
      arch: "x64",
      resourcesPath: "/synthetic-packaged",
      env: {}
    }
  })
  let shutdowns = 0
  ;(exports as any).default(undefined, async () => {
    shutdowns++
  })
  await handlers.checkForUpdates(null, "stable")
  const token = handlers.getUpdateState().readyUpdate.id
  await handlers.installUpdate(null, token)
  expect(shutdowns).toBe(1)
  expect((await handlers.checkForUpdates(null, "beta")).status).toBe("busy")
  events.error(Error("Native pending installation failed"))
  for (let i = 0; i < 20; i++) await Promise.resolve()
  expect(native.listeners("update-downloaded")).toEqual([baseline])
  expect(broadcast.at(-1).state).toBe("error")
  await handlers.checkForUpdates(null, "beta")
  native.emit("update-downloaded")
  expect(handlers.getUpdateState().readyUpdate.id).not.toBe(token)
  expect(handlers.getUpdateState().readyUpdate.version).toBe("2.0.0")
})

function declarations(text: string, names: string[]) {
  const file = ts.createSourceFile(
    "fixture.tsx",
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  )
  const found = new Map<string, string>()
  function visit(node: ts.Node) {
    if (
      ts.isFunctionDeclaration(node) &&
      node.name &&
      names.includes(node.name.text)
    )
      found.set(node.name.text, node.getText(file))
    ts.forEachChild(node, visit)
  }
  visit(file)
  for (const name of names)
    if (!found.has(name))
      throw Error("Actual production function unavailable: " + name)
  return names.map((name) => found.get(name)!).join("\n")
}
/** Electron 39 marks a window destroyed before it emits 'closed'; from then
 *  on its getters and methods throw and destroy() does nothing. */
class ElectronLikeContents extends EventEmitter {
  readonly mainFrame = {}
  sent: unknown[][] = []
  constructor(private readonly owner: ElectronLikeWindow) {
    super()
  }
  send(...args: unknown[]) {
    this.owner.live()
    this.sent.push(args)
  }
  setWindowOpenHandler() {}
  getURL() {
    return "file:///synthetic/index.html"
  }
}
class ElectronLikeWindow extends EventEmitter {
  static created: ElectronLikeWindow[] = []
  static failNext = false
  static getAllWindows() {
    return ElectronLikeWindow.created.filter((w) => !w.destroyed)
  }
  destroyed = false
  focused = 0
  closeCalls = 0
  hidden = false
  shown = 0
  readonly contents: ElectronLikeContents
  constructor() {
    super()
    if (ElectronLikeWindow.failNext) {
      ElectronLikeWindow.failNext = false
      throw Error("synthetic window creation failure")
    }
    this.contents = new ElectronLikeContents(this)
    ElectronLikeWindow.created.push(this)
  }
  live() {
    if (this.destroyed) throw new TypeError("Object has been destroyed")
  }
  get webContents() {
    this.live()
    return this.contents
  }
  isDestroyed() {
    return this.destroyed
  }
  isVisible() {
    this.live()
    return !this.hidden
  }
  close() {
    this.live()
    this.closeCalls++
    let prevented = false
    this.emit("close", {
      preventDefault() {
        prevented = true
      },
      get defaultPrevented() {
        return prevented
      }
    })
    if (!prevented) this.destroy()
  }
  destroy() {
    if (this.destroyed) return
    this.destroyed = true
    this.emit("closed")
  }
  minimized = false
  isMinimized() {
    this.live()
    return this.minimized
  }
  minimize() {
    this.live()
    this.minimized = true
  }
  restore() {
    this.live()
    this.minimized = false
  }
  focus() {
    this.live()
    this.focused++
  }
  hide() {
    this.live()
    this.hidden = true
  }
  show() {
    this.live()
    this.hidden = false
    this.shown++
  }
  async loadFile() {
    this.live()
  }
}
function mainWindow() {
  const text = source("apps/desktop/packages/main/index.ts")
  ElectronLikeWindow.created = []
  ElectronLikeWindow.failNext = false
  let quits = 0,
    appQuits = 0
  const deliveries: unknown[] = []
  const protocolQueue = new ProtocolQueue<object>((sender, value) =>
    deliveries.push({ sender, value })
  )
  const context: Record<string, any> = {
    console: { log() {}, warn() {}, error() {} },
    isSpawningWindow: false,
    win: null,
    lastDisplay: null,
    pendingDisplayChange: false,
    adSizeFallbackTimeout: null,
    screen: { getPrimaryDisplay: () => ({ id: 1 }) },
    getAdSize: () => ({ minWidth: 1, minHeight: 1, height: 1, width: 1 }),
    BrowserWindow: ElectronLikeWindow,
    join: path.join,
    __dirname: "/synthetic",
    app: {
      isPackaged: true,
      quit: () => {
        appQuits++
      }
    },
    runtimeOperationActive: () => false,
    interruptRuntimeOperation: () => false,
    whenRuntimeOperationIdle: () => {},
    shutdownApproved: false,
    shutdownInFlight: undefined,
    shutdownPrompts: new ShutdownPrompts<any>(async () => null),
    skipIntroAnimation: false,
    protocolQueue,
    mainWindowClosed,
    userWindows,
    showAppCloseWarning: true,
    shell: { openExternal() {} },
    process: { platform: "linux" },
    setTimeout,
    clearTimeout,
    URL,
    importMetaEnv: { DEV: false },
    __SHOWCASE_MODE__: false
  }
  const lifecycle = new LauncherLifecycle<any>({
    current: () => context.win,
    clear: (window) => {
      if (context.win === window) context.win = null
    },
    create: () => context.createWindow(),
    quit: () => {
      quits++
    }
  })
  context.lifecycle = lifecycle
  context.createWindow = evaluate(
    "(() => {\n" +
      declarations(text, ["createWindow", "spawnWindow"]).replaceAll(
        "import.meta.env",
        "importMetaEnv"
      ) +
      "\nreturn createWindow\n})()",
    context
  )
  const decisions = evaluate(
    "(() => {\n" +
      declarations(text, [
        "closedBeforeMove",
        "quitUnlessKeptAlive",
        "quitIfStillWindowless",
        "quitAfterCoreExit"
      ]) +
      "\nreturn { quitUnlessKeptAlive, quitAfterCoreExit }\n})()",
    context
  )
  Object.assign(context, decisions)
  return {
    text,
    context,
    lifecycle,
    protocolQueue,
    deliveries,
    createWindow: context.createWindow as () => Promise<ElectronLikeWindow>,
    windowAllClosed: evaluate(
      callArgument(text, "app.on", "window-all-closed"),
      context
    ) as () => void,
    quitAfterCoreExit: decisions.quitAfterCoreExit as () => void,
    get quits() {
      return quits
    },
    get appQuits() {
      return appQuits
    }
  }
}
it("actual createWindow: closing the window neither throws inside 'closed' nor leaves a stale handle", async () => {
  const value = mainWindow()
  const first = await value.createWindow()
  first.emit("ready-to-show")
  // The quit was approved, so the window really closes ('closed' fires).
  value.context.shutdownApproved = true
  expect(value.context.isSpawningWindow).toBe(false)
  expect(value.protocolQueue.session(first.contents)).not.toBeNull()
  expect(() => first.close()).not.toThrow()
  expect(first.destroyed).toBe(true)
  expect(value.context.win).toBeNull()
  expect(value.protocolQueue.session(first.contents)).toBeNull()
  const second = await value.createWindow()
  expect(second).not.toBe(first)
  expect(ElectronLikeWindow.created).toHaveLength(2)
})
it("actual createWindow reuses a live window and drops a destroyed handle without closing it", async () => {
  const value = mainWindow()
  const live = await value.createWindow()
  live.emit("ready-to-show")
  expect(await value.createWindow()).toBe(live)
  expect(live.focused).toBe(1)
  expect(ElectronLikeWindow.created).toHaveLength(1)
  // The handle a throwing 'closed' listener used to leave behind.
  live.destroyed = true
  const next = await value.createWindow()
  expect(next).not.toBe(live)
  expect(live.closeCalls).toBe(0)
  expect(value.context.win).toBe(next)
})
it("actual createWindow does not leave the spawning flag stuck when creation fails or the window closes before ready-to-show", async () => {
  const value = mainWindow()
  ElectronLikeWindow.failNext = true
  await expect(value.createWindow()).rejects.toThrow(
    "synthetic window creation failure"
  )
  expect(value.context.isSpawningWindow).toBe(false)
  const early = await value.createWindow()
  expect(value.context.isSpawningWindow).toBe(true)
  value.context.shutdownApproved = true
  early.close()
  expect(value.context.isSpawningWindow).toBe(false)
  const next = await value.createWindow()
  expect(next).not.toBe(early)
  expect(ElectronLikeWindow.created).toHaveLength(2)
})
it("actual window wiring: a Close Window launch reopens exactly one window after the game exits", async () => {
  const value = mainWindow()
  const first = await value.createWindow()
  first.emit("ready-to-show")
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
  expect(first.destroyed).toBe(true)
  expect(value.context.win).toBeNull()
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
  value.lifecycle.snapshot('{"revision":2,"instances":[]}')
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  expect(ElectronLikeWindow.created).toHaveLength(2)
  const reopened = ElectronLikeWindow.created[1]
  expect(value.context.win).toBe(reopened)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  expect(ElectronLikeWindow.created).toHaveLength(2)
  expect(value.quits).toBe(0)
})
it("actual window wiring: closing the window a relaunch showed during a Close Window game quits, and the game exit opens nothing", async () => {
  const value = mainWindow()
  const first = await value.createWindow()
  first.emit("ready-to-show")
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
  expect(first.destroyed).toBe(true)
  value.windowAllClosed()
  expect(value.appQuits).toBe(0)
  // GDLauncher is started again mid-game ('second-instance'), and the user
  // closes that window while it is still loading.
  const shown = await value.createWindow()
  shown.emit("ready-to-show")
  shown.close()
  expect(shown.destroyed).toBe(true)
  value.windowAllClosed()
  expect(value.appQuits).toBe(1)
  value.lifecycle.snapshot('{"revision":2,"instances":[]}')
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  expect(ElectronLikeWindow.created).toHaveLength(2)
})
it("actual close-warning IPC: only a main-frame App acknowledgement holds a close, and a navigation withdraws it", async () => {
  const value = mainWindow()
  const receiver = evaluate(
    callArgument(value.text, "ipcMain.on", "close-warning-receiver"),
    { lifecycle: value.lifecycle }
  )
  const w = await value.createWindow()
  w.emit("ready-to-show")
  // A game adopted during the startup scan, before App mounted.
  value.lifecycle.snapshot('{"revision":1,"instances":[4]}')
  receiver({ sender: w.contents, senderFrame: {} }, true)
  receiver({ sender: w.contents, senderFrame: w.contents.mainFrame }, true)
  w.close()
  expect(w.destroyed).toBe(false)
  expect(w.hidden).toBe(false)
  expect(w.contents.sent).toEqual([["showAppCloseWarning"]])
  w.contents.emit("did-navigate")
  w.close()
  // The close goes ahead: the last window hides and the quit starts.
  expect(w.contents.sent).toEqual([["showAppCloseWarning"]])
  expect(w.hidden).toBe(true)
  expect(value.appQuits).toBe(1)
  // Once the quit is approved it closes for good.
  value.context.shutdownApproved = true
  w.close()
  expect(w.destroyed).toBe(true)
  expect(value.context.win).toBeNull()
  const next = await value.createWindow()
  receiver(
    { sender: next.contents, senderFrame: next.contents.mainFrame },
    true
  )
  receiver(
    { sender: next.contents, senderFrame: next.contents.mainFrame },
    false
  )
  next.close()
  expect(next.destroyed).toBe(true)
})
it("actual last-window close hides the window and starts the quit instead of destroying it", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  w.close()
  expect(w.destroyed).toBe(false)
  expect(w.hidden).toBe(true)
  expect(value.context.win).toBe(w)
  expect(value.appQuits).toBe(1)
})
it("actual window close while a second window is open, during a move or for a Close Window launch still closes it", async () => {
  // Another window is left.
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const other = new ElectronLikeWindow()
  w.close()
  expect(w.destroyed).toBe(true)
  expect(value.appQuits).toBe(0)
  other.destroy()

  // A move keeps its own close handling (quitUnlessKeptAlive).
  const moving = mainWindow()
  moving.context.runtimeOperationActive = () => true
  const m = await moving.createWindow()
  m.emit("ready-to-show")
  m.close()
  expect(m.destroyed).toBe(true)

  // A Close Window launch action.
  const launch = mainWindow()
  const l = await launch.createWindow()
  l.emit("ready-to-show")
  launch.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
  expect(l.destroyed).toBe(true)
  expect(launch.appQuits).toBe(0)
})
/** A hidden 1x1 window ow-electron opens for itself, such as the Overwolf
 *  consent window or, on Windows, its package manager's. */
function owElectronHelperWindow() {
  const helper = new ElectronLikeWindow()
  helper.hidden = true
  return helper
}
it("actual last-window close still hides the window and quits while ow-electron has a hidden window of its own", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const helper = owElectronHelperWindow()
  w.close()
  expect(w.destroyed).toBe(false)
  expect(w.hidden).toBe(true)
  expect(value.appQuits).toBe(1)
  helper.destroy()
})
it("actual core exit quits a windowless launcher although ow-electron has a hidden window of its own", async () => {
  const value = mainWindow()
  const { child } = await coreProcess({
    lifecycle: value.lifecycle,
    quitAfterCoreExit: value.quitAfterCoreExit
  })
  const first = await value.createWindow()
  first.emit("ready-to-show")
  const helper = owElectronHelperWindow()
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}', "session-1")
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow", "session-1")
  expect(first.destroyed).toBe(true)
  child.emit("exit", 1)
  expect(value.appQuits).toBe(1)
  helper.destroy()
})
it("actual window close is held while a shutdown prompt waits for an answer", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  value.context.shutdownPrompts = { open: true, receiver() {} }
  w.close()
  expect(w.destroyed).toBe(false)
  expect(w.hidden).toBe(false)
  expect(value.appQuits).toBe(0)
})
/** The actual quit path: before-quit, ownedShutdown and its helpers. */
function quitPath(
  value: ReturnType<typeof mainWindow>,
  core: {
    reply(endpoint: string, body: any): { status: number; body: unknown }
    kill(signal?: string): boolean
  }
) {
  const calls: string[] = []
  const process = Object.assign(new EventEmitter(), {
    exitCode: null as number | null,
    signalCode: null as string | null,
    kill: (signal?: string) => {
      calls.push(`kill ${signal ?? "SIGTERM"}`)
      return core.kill(signal)
    }
  })
  const transport = async (url: string, options: any) => {
    const endpoint = String(url).split("/").at(-1)!
    const body = JSON.parse(options.body)
    calls.push(endpoint)
    const answer = core.reply(endpoint, body)
    return {
      ok: answer.status < 300,
      status: answer.status,
      json: async () => answer.body,
      text: async () => JSON.stringify(answer.body)
    }
  }
  const native: { window: unknown; view: ShutdownView }[] = []
  let choose: (choice: "wait" | "force" | "keep") => void = () => {}
  const prompted = new Promise<void>((shown) => {
    value.context.shutdownPrompts = new ShutdownPrompts<any>((window, view) => {
      native.push({ window, view })
      shown()
      return new Promise((resolve) => (choose = resolve))
    })
  })
  const success = {
    type: "success",
    result: { port: 1, apiToken: "fixture" }
  }
  const lifecycleCalls: string[] = []
  Object.assign(value.context, {
    coreModule: Promise.resolve(success),
    coreLoaded: success,
    coreProcessHandle: process,
    ShutdownFlow,
    ShutdownBlockedError,
    ShutdownNotPossibleError,
    stopCoreForShutdown: (c: any, purpose: any, _t: any, ms: any, o: any) =>
      stopCoreForShutdown(c, purpose, transport as any, ms, o),
    cancelCoreShutdown: (c: any) => cancelCoreShutdown(c, transport as any),
    forceTerminateCore: (proc: any) => forceTerminateCore(proc, 50, 1000),
    terminateAndObserveCoreExit,
    fetch: transport,
    AbortController,
    Promise,
    Error
  })
  const quitFailed = value.lifecycle.quitFailed.bind(value.lifecycle)
  value.lifecycle.quitFailed = () => {
    lifecycleCalls.push("quitFailed")
    quitFailed()
  }
  value.context.app.exit = () => calls.push("app.exit")
  Object.assign(
    value.context,
    evaluate(
      "(() => {\n" +
        declarations(value.text, [
          "revealWindow",
          "hideWindow",
          "startedCore",
          "stopOwnedCore",
          "ownedShutdown",
          "closedBeforeMove",
          "quitAgain"
        ]) +
        "\nreturn { revealWindow, hideWindow, ownedShutdown, closedBeforeMove, quitAgain }\n})()",
      value.context
    )
  )
  const beforeQuit = evaluate(
    callArgument(value.text, "app.on", "before-quit"),
    value.context
  ) as (event: { preventDefault(): void }) => void
  return {
    calls,
    native,
    process,
    prompted,
    lifecycleCalls,
    choose: (choice: "wait" | "force" | "keep") => choose(choice),
    beforeQuit: () => beforeQuit({ preventDefault() {} })
  }
}
/** A core that cannot finish: a deletion ignores its cancellation. */
const unresolvedCore = (exitOn: string[] = []) => {
  let proc: EventEmitter & { exitCode: number | null }
  return {
    bind(process: any) {
      proc = process
    },
    reply(endpoint: string, body: any) {
      if (endpoint === "cancel-shutdown")
        return { status: 200, body: { requestId: body.requestId } }
      return {
        status: 200,
        body: {
          requestId: body.requestId,
          status: {
            ...body,
            phase: "unresolved",
            blockers: [{ kind: "deletion", name: "delete instance 12" }],
            serversStopped: true
          }
        }
      }
    },
    kill(signal = "SIGTERM") {
      if (exitOn.includes(signal))
        setTimeout(() => {
          proc.exitCode = 0
          proc.emit("exit", 0)
        }, 0)
      return true
    }
  }
}
it("actual quit that cannot finish shows the hidden window with the prompt parented to it, and Keep GDLauncher open gives the core back", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const core = unresolvedCore()
  const quit = quitPath(value, core)
  core.bind(quit.process)
  quit.beforeQuit()
  expect(w.hidden).toBe(true)
  await quit.prompted
  // No App listens in this fixture, so the native dialog asks, over the window.
  expect(quit.native).toHaveLength(1)
  expect(quit.native[0].window).toBe(w)
  expect(w.hidden).toBe(false)
  expect(quit.native[0].view).toMatchObject({
    action: "quit",
    stage: "blocked",
    reason: "unresolved",
    blockers: [{ kind: "deletion", name: "delete instance 12" }],
    serversStopped: true,
    canWait: true
  })
  quit.choose("keep")
  await vi.waitFor(() => expect(quit.lifecycleCalls).toEqual(["quitFailed"]))
  expect(quit.calls).toEqual(["prepare-shutdown", "cancel-shutdown"])
  expect(w.hidden).toBe(false)
  expect(w.destroyed).toBe(false)
  expect(value.appQuits).toBe(0)
  expect(value.context.shutdownApproved).toBe(false)
  await vi.waitFor(() => expect(value.context.shutdownInFlight).toBeUndefined())
})
it("actual quit that cannot finish ends the core on Quit now, escalating to SIGKILL, and then quits", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  // The core's own SIGTERM handler goes through the same refused handshake.
  const core = unresolvedCore(["SIGKILL"])
  const quit = quitPath(value, core)
  core.bind(quit.process)
  quit.beforeQuit()
  await quit.prompted
  expect(w.hidden).toBe(false)
  quit.choose("force")
  // The window goes at once, not after the few seconds SIGKILL waits.
  await vi.waitFor(() => expect(quit.calls).toContain("kill SIGTERM"))
  expect(w.hidden).toBe(true)
  await vi.waitFor(() => expect(value.appQuits).toBe(1))
  expect(quit.calls).toEqual([
    "prepare-shutdown",
    "kill SIGTERM",
    "kill SIGKILL"
  ])
  expect(value.context.shutdownApproved).toBe(true)
  expect(quit.lifecycleCalls).toEqual([])
})
it("actual pending quit: a Minimize game that exits leaves the window hidden, and after Keep GDLauncher open a game exit shows it again", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  value.lifecycle.snapshot('{"revision":1,"instances":[1,2]}')
  value.lifecycle.event("GAME_LAUNCHED|1|7|minimizeWindow")
  value.lifecycle.event("GAME_LAUNCHED|2|8|minimizeWindow")
  expect(w.minimized).toBe(true)
  const core = unresolvedCore()
  const quit = quitPath(value, core)
  core.bind(quit.process)
  // Cmd+Q: the window hides while the quit waits on the core.
  quit.beforeQuit()
  expect(w.hidden).toBe(true)
  const shown = w.shown
  value.lifecycle.event("GAME_CLOSED|1|7|minimizeWindow")
  expect(w.hidden).toBe(true)
  expect(w.shown).toBe(shown)
  // The quit's own prompt brings it back.
  await quit.prompted
  expect(w.hidden).toBe(false)
  quit.choose("keep")
  await vi.waitFor(() => expect(quit.lifecycleCalls).toEqual(["quitFailed"]))
  w.hide()
  value.lifecycle.event("GAME_CLOSED|2|8|minimizeWindow")
  expect(w.hidden).toBe(false)
})
it("actual second launch during a pending quit shows the hidden window and its prompt instead of a new window", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  w.close()
  expect(w.hidden).toBe(true)
  let revealed = 0
  value.context.shutdownFlow = { reveal: () => revealed++ }
  value.context.shutdownInFlight = new Promise(() => {})
  Object.assign(
    value.context,
    evaluate(
      "(() => {\n" +
        declarations(value.text, ["revealWindow"]) +
        "\nreturn { revealWindow }\n})()",
      value.context
    )
  )
  value.context.isSupportedProtocol = () => false
  const secondInstance = evaluate(
    callArgument(value.text, "app.on", "second-instance"),
    value.context
  ) as (event: unknown, argv: string[]) => void
  secondInstance({}, [])
  expect(w.hidden).toBe(false)
  expect(revealed).toBe(1)
  expect(ElectronLikeWindow.created).toHaveLength(1)
  const activate = evaluate(
    callArgument(value.text, "app.on", "activate"),
    value.context
  ) as () => void
  w.hide()
  activate()
  expect(w.hidden).toBe(false)
  expect(revealed).toBe(2)
  expect(ElectronLikeWindow.created).toHaveLength(1)
})
it("actual macOS link opened during a pending quit shows the hidden window and its prompt, and still reaches it", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const consumer = protocolConsumer(value, w.contents)
  consumer.ready("1", consumer.generation())
  w.close()
  expect(w.hidden).toBe(true)
  let revealed = 0
  value.context.shutdownFlow = { reveal: () => revealed++ }
  value.context.shutdownInFlight = new Promise(() => {})
  Object.assign(
    value.context,
    evaluate(
      "(() => {\n" +
        declarations(value.text, ["revealWindow"]) +
        "\nreturn { revealWindow }\n})()",
      value.context
    )
  )
  value.context.isSupportedProtocol = (url: string) =>
    url.startsWith("gdlauncher://")
  value.context.app.isReady = () => true
  const openUrl = evaluate(
    callArgument(value.text, "app.on", "open-url"),
    value.context
  ) as (event: { preventDefault(): void }, url: string) => void
  let prevented = 0
  openUrl({ preventDefault: () => prevented++ }, "gdlauncher://share/LINK")
  expect(prevented).toBe(1)
  expect(w.hidden).toBe(false)
  expect(revealed).toBe(1)
  expect(ElectronLikeWindow.created).toHaveLength(1)
  expect(consumer.delivered().map((d) => d.url)).toEqual([
    "gdlauncher://share/LINK"
  ])
  // Without a quit under way, a link only brings the window forward.
  value.context.shutdownInFlight = undefined
  const focused = w.focused
  openUrl({ preventDefault() {} }, "gdlauncher://share/LATER")
  expect(w.focused).toBe(focused + 1)
  expect(revealed).toBe(1)
})
/** The actual protocol IPC handlers, called as the main frame of `contents`. */
function protocolConsumer(
  value: ReturnType<typeof mainWindow>,
  contents: ElectronLikeContents
) {
  const event = { sender: contents, senderFrame: contents.mainFrame }
  const ready = evaluate(
    callArgument(value.text, "ipcMain.on", "protocol-consumer-ready"),
    { protocolQueue: value.protocolQueue }
  )
  const ack = evaluate(
    callArgument(value.text, "ipcMain.on", "protocol-consumer-ack"),
    { protocolQueue: value.protocolQueue }
  )
  return {
    ready: (subscription: string, generation: number | null) =>
      ready(event, { subscription, generation }),
    ack: (delivery: unknown) => ack(event, delivery),
    generation: () => value.protocolQueue.session(contents),
    delivered: () =>
      (value.deliveries as { sender: unknown; value: any }[]).map((d) => ({
        sender: d.sender,
        ...d.value
      }))
  }
}
it("actual window wiring: a link click that will-navigate cancels leaves curseforge://, modrinth:// and gdlauncher:// links working", async () => {
  const value = mainWindow()
  const opened: string[] = []
  value.context.shell = { openExternal: (url: string) => opened.push(url) }
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const consumer = protocolConsumer(value, w.contents)
  consumer.ready("1", consumer.generation())
  value.protocolQueue.enqueue("gdlauncher://share/FIRST")
  expect(consumer.delivered()).toHaveLength(1)
  consumer.ack(consumer.delivered()[0])
  const generation = consumer.generation()

  // A left click on a plain <a href> in a mod description: Electron emits
  // 'did-start-navigation' first, then 'will-navigate', which the launcher
  // cancels and opens in the browser. Nothing commits, so no 'did-navigate'.
  const link = "https://discord.example.invalid/invite"
  w.contents.emit("did-start-navigation", {}, link, false, true)
  let prevented = false
  w.contents.emit(
    "will-navigate",
    {
      preventDefault() {
        prevented = true
      }
    },
    link
  )
  expect(prevented).toBe(true)
  expect(opened).toEqual([link])
  expect(consumer.generation()).toBe(generation)

  value.protocolQueue.enqueue("curseforge://install?addonId=1&fileId=2")
  expect(consumer.delivered()).toHaveLength(2)
  expect(consumer.delivered()[1]).toMatchObject({
    sender: w.contents,
    subscription: "1",
    generation,
    url: "curseforge://install?addonId=1&fileId=2"
  })
})
it("actual window wiring: a reload hands an unacknowledged deep link to the new App exactly once", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const consumer = protocolConsumer(value, w.contents)
  const old = consumer.generation()
  consumer.ready("1", old)
  value.protocolQueue.enqueue("gdlauncher://share/RELOAD")
  const first = consumer.delivered()[0]
  expect(first.generation).toBe(old)

  // location.reload(): the navigation starts and then commits.
  const url = w.contents.getURL()
  w.contents.emit("did-start-navigation", {}, url, false, true)
  w.contents.emit("did-navigate", {}, url, 200, "OK")
  expect(consumer.generation()).not.toBe(old)

  // The old document's late handshake and ACK belong to the old generation.
  consumer.ready("1", old)
  consumer.ack(first)
  expect(consumer.delivered()).toHaveLength(1)

  // The new document's preload starts counting subscriptions from 1 again.
  consumer.ready("1", consumer.generation())
  expect(consumer.delivered()).toHaveLength(2)
  const again = consumer.delivered()[1]
  expect(again).toMatchObject({
    id: first.id,
    url: "gdlauncher://share/RELOAD",
    subscription: "1",
    generation: consumer.generation()
  })
  consumer.ack(again)
  consumer.ready("1", consumer.generation())
  expect(consumer.delivered()).toHaveLength(2)
})
it("actual window wiring: a renderer crash holds deep links until the reloaded App subscribes", async () => {
  const value = mainWindow()
  const w = await value.createWindow()
  w.emit("ready-to-show")
  const consumer = protocolConsumer(value, w.contents)
  consumer.ready("1", consumer.generation())
  w.contents.emit("render-process-gone", {}, { reason: "crashed" })
  value.protocolQueue.enqueue("modrinth://mod/fixture")
  expect(consumer.delivered()).toHaveLength(0)
  consumer.ready("1", consumer.generation())
  expect(consumer.delivered()).toHaveLength(1)
  expect(consumer.delivered()[0].url).toBe("modrinth://mod/fixture")
})
it("main index.ts no longer revokes the protocol consumer when a navigation merely starts", () => {
  const text = source("apps/desktop/packages/main/index.ts")
  const file = ts.createSourceFile(
    "index.ts",
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS
  )
  const listeners: string[] = []
  function visit(node: ts.Node) {
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression) &&
      node.expression.name.text === "on" &&
      node.arguments[0]?.getText(file) === '"did-start-navigation"'
    )
      listeners.push(node.getText(file))
    ts.forEachChild(node, visit)
  }
  visit(file)
  expect(
    listeners.filter((l) => /protocolQueue|revoke|documentReplaced/.test(l))
  ).toEqual([])
})
/** Runs the actual `loadCoreModule` against a fake core process. */
async function coreProcess(context: {
  lifecycle: unknown
  quitAfterCoreExit: () => void
}) {
  const child = Object.assign(new EventEmitter(), {
    pid: 7,
    stdout: new EventEmitter(),
    stderr: new EventEmitter(),
    kill: () => true
  })
  const load = evaluate(
    initializer(
      source("apps/desktop/packages/main/index.ts"),
      "loadCoreModule"
    ),
    {
      CURRENT_RUNTIME_PATH: "/synthetic",
      assertDatabaseRestoreReadySync: () => {},
      isDev: false,
      console: { log() {}, error() {} },
      path,
      __dirname: "/synthetic",
      binaryName: "core_module",
      buildCoreModuleArgs: () => [],
      overrideBaseApi: null,
      e2eAuthBase: null,
      e2eEntitlementKey: null,
      crypto: { randomUUID: () => "session-1" },
      accountIdentity: { beginCore() {}, acceptPayload() {} },
      ...context,
      spawn: () => child,
      process: { env: {} },
      createCoreOutput,
      Sentry: { captureException() {} },
      setTimeout: () => 0,
      clearTimeout() {},
      getWin: () => null
    }
  )
  const result = load()
  await new Promise((resolve) => setTimeout(resolve, 0))
  return { child, result }
}
it("actual core load spawns the core before yielding to the event loop", () => {
  // Spawned after an await, the core starts while Chromium launches its own
  // children and inherits their IPC sockets, so a failed GPU process is never
  // noticed and the window never paints.
  let spawned = false
  const child = Object.assign(new EventEmitter(), {
    pid: 7,
    stdout: new EventEmitter(),
    stderr: new EventEmitter(),
    kill: () => true
  })
  const load = evaluate(
    initializer(
      source("apps/desktop/packages/main/index.ts"),
      "loadCoreModule"
    ),
    {
      CURRENT_RUNTIME_PATH: "/synthetic",
      assertDatabaseRestoreReadySync: () => {},
      isDev: false,
      console: { log() {}, error() {} },
      path,
      __dirname: "/synthetic",
      binaryName: "core_module",
      buildCoreModuleArgs: () => [],
      overrideBaseApi: null,
      e2eAuthBase: null,
      e2eEntitlementKey: null,
      crypto: { randomUUID: () => "session-1" },
      accountIdentity: { beginCore() {}, acceptPayload() {} },
      lifecycle: { beginCore() {}, endCore: () => false },
      quitAfterCoreExit() {},
      spawn: () => {
        spawned = true
        return child
      },
      process: { env: {} },
      createCoreOutput,
      Sentry: { captureException() {} },
      setTimeout: () => 0,
      clearTimeout() {},
      getWin: () => null
    }
  )
  void load()
  expect(spawned).toBe(true)
})
it("actual core exit clears the lifecycle for its own session", async () => {
  for (const keptAlive of [false, true]) {
    const calls: string[] = []
    const { child, result } = await coreProcess({
      lifecycle: {
        beginCore: (s: string) => calls.push("begin:" + s),
        endCore: (s: string) => {
          calls.push("end:" + s)
          return keptAlive
        },
        snapshot() {},
        event() {}
      },
      quitAfterCoreExit: () => calls.push("quit decision")
    })
    child.emit("exit", 1)
    expect((await result).type).toBe("error")
    expect(calls).toEqual([
      "begin:session-1",
      "end:session-1",
      ...(keptAlive ? ["quit decision"] : [])
    ])
  }
})
it("actual core exit quits a launcher that a Close Window game left without a window", async () => {
  const value = mainWindow()
  const { child } = await coreProcess({
    lifecycle: value.lifecycle,
    quitAfterCoreExit: value.quitAfterCoreExit
  })
  const first = await value.createWindow()
  first.emit("ready-to-show")
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}', "session-1")
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow", "session-1")
  expect(first.destroyed).toBe(true)
  value.windowAllClosed()
  expect(value.appQuits).toBe(0)
  // core_module panics, or is ended from Task Manager, mid-game. It can no
  // longer report the game's exit, so nothing would ever reopen the window.
  child.emit("exit", 1)
  expect(value.appQuits).toBe(1)
})
it("actual core exit leaves the launcher running when a window is open or a quit, restart or relocation owns it", async () => {
  const cases: [string, (value: ReturnType<typeof mainWindow>) => unknown][] = [
    // GDLauncher was started again mid-game and showed a window.
    [
      "window",
      async (value) => (await value.createWindow()).emit("ready-to-show")
    ],
    ["spawning", (value) => (value.context.isSpawningWindow = true)],
    ["shutdown", (value) => (value.context.shutdownInFlight = {})],
    ["approved", (value) => (value.context.shutdownApproved = true)],
    [
      "runtime operation",
      (value) => (value.context.runtimeOperationActive = () => true)
    ]
  ]
  for (const [name, arrange] of cases) {
    const value = mainWindow()
    const { child } = await coreProcess({
      lifecycle: value.lifecycle,
      quitAfterCoreExit: value.quitAfterCoreExit
    })
    const first = await value.createWindow()
    first.emit("ready-to-show")
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow", "session-1")
    expect(first.destroyed).toBe(true)
    await arrange(value)
    child.emit("exit", 1)
    expect(value.appQuits, name).toBe(0)
  }
})
it("actual core exit during startup leaves the window to show the error", async () => {
  const value = mainWindow()
  const { child, result } = await coreProcess({
    lifecycle: value.lifecycle,
    quitAfterCoreExit: value.quitAfterCoreExit
  })
  // The core can die before the first window exists.
  child.emit("exit", 1)
  expect((await result).type).toBe("error")
  expect(value.appQuits).toBe(0)
})

/** The real runtime-operation guard, as main/index.ts imports it. */
function realRuntimeOperation(context: Record<string, any>) {
  Object.assign(context, {
    runtimeOperationActive,
    runtimeOperationBrief,
    interruptRuntimeOperation,
    whenRuntimeOperationIdle
  })
}
const tick = () => new Promise((resolve) => setTimeout(resolve, 0))
/** A move that waits in its check phase until the app is closed. */
function checkingMove() {
  let started!: () => void
  const checking = new Promise<void>((resolve) => (started = resolve))
  let stopped = false
  const move = withRuntimeOperation(async () => {
    const interruptible = interruptibleUntilCommitted()
    started()
    await new Promise((resolve) =>
      interruptible.signal.addEventListener("abort", resolve)
    )
    // Right before the core would be stopped.
    interruptible.commit()
    stopped = true
  })
  return { checking, move, stopped: () => stopped }
}
/** A move already past the point where the core is asked to stop. */
function committedMove() {
  let finish!: () => void
  const move = withRuntimeOperation(async () => {
    interruptibleUntilCommitted().commit()
    await new Promise<void>((resolve) => (finish = resolve))
  })
  return { move, finish: () => finish() }
}
it("actual last-window close while a move only checks the data folder ends it with nothing moved, then quits", async () => {
  const value = mainWindow()
  realRuntimeOperation(value.context)
  const move = checkingMove()
  await move.checking
  value.windowAllClosed()
  // Nothing quits while the move is still unwinding.
  expect(value.appQuits).toBe(0)
  await expect(move.move).rejects.toThrow(/closed before the move started/)
  await tick()
  expect(move.stopped()).toBe(false)
  expect(runtimeOperationActive()).toBe(false)
  expect(value.appQuits).toBe(1)
})
it("actual last-window close after a move asked the core to stop lets it finish, then quits unless a window is back", async () => {
  for (const reopened of [false, true]) {
    const value = mainWindow()
    realRuntimeOperation(value.context)
    const move = committedMove()
    value.windowAllClosed()
    expect(value.appQuits).toBe(0)
    if (reopened) (await value.createWindow()).emit("ready-to-show")
    move.finish()
    await move.move
    await tick()
    expect(value.appQuits, `reopened: ${reopened}`).toBe(reopened ? 0 : 1)
  }
})
it("actual quit request while a move only checks ends it and quits after; one past its commit keeps its window", async () => {
  const text = source("apps/desktop/packages/main/index.ts")
  let quits = 0,
    prepared = 0,
    prevented = 0
  const context: Record<string, any> = {
    shutdownApproved: false,
    shutdownInFlight: undefined,
    prepareOwnedShutdown: () => {
      prepared++
      return new Promise(() => {})
    },
    lifecycle: { quitFailed() {} },
    dialog: { showMessageBox: async () => ({}) },
    app: {
      quit: () => {
        quits++
      }
    }
  }
  realRuntimeOperation(context)
  Object.assign(
    context,
    evaluate(
      "(() => {\n" +
        declarations(text, ["closedBeforeMove", "quitAgain"]) +
        "\nreturn { closedBeforeMove, quitAgain }\n})()",
      context
    )
  )
  const beforeQuit = evaluate(
    callArgument(text, "app.on", "before-quit"),
    context
  ) as (event: { preventDefault(): void }) => void
  const event = {
    preventDefault() {
      prevented++
    }
  }

  const checking = checkingMove()
  await checking.checking
  beforeQuit(event)
  expect(prevented).toBe(1)
  await expect(checking.move).rejects.toThrow(/closed before the move started/)
  await tick()
  expect(checking.stopped()).toBe(false)
  // The deferred quit; its own before-quit then runs the normal shutdown.
  expect(quits).toBe(1)
  expect(prepared).toBe(0)

  const committed = committedMove()
  beforeQuit(event)
  expect(prevented).toBe(2)
  committed.finish()
  await committed.move
  await tick()
  // The window stays to show how the move ended, as before.
  expect(quits).toBe(1)
  expect(prepared).toBe(0)
})
it("actual quit request during Keep default or another brief step quits once it has ended", async () => {
  const text = source("apps/desktop/packages/main/index.ts")
  let quits = 0,
    prevented = 0
  let forget!: () => void
  const context: Record<string, any> = {
    shutdownApproved: false,
    shutdownInFlight: undefined,
    withRuntimeOperation,
    path,
    RUNTIME_PATH_OVERRIDE_NAME: "runtime_path_override",
    UNAVAILABLE_RUNTIME_PATH: "/unplugged/gdl",
    forgetUnavailableRuntimePath: () =>
      new Promise<void>((resolve) => (forget = resolve)),
    app: {
      getPath: () => "/synthetic",
      quit: () => {
        quits++
      }
    }
  }
  realRuntimeOperation(context)
  Object.assign(
    context,
    evaluate(
      "(() => {\n" +
        declarations(text, ["closedBeforeMove", "quitAgain"]) +
        "\nreturn { closedBeforeMove, quitAgain }\n})()",
      context
    )
  )
  const beforeQuit = evaluate(
    callArgument(text, "app.on", "before-quit"),
    context
  ) as (event: { preventDefault(): void }) => void
  const event = {
    preventDefault() {
      prevented++
    }
  }

  // The user chose Keep the default folder and pressed Cmd+Q right away.
  const keep = evaluate(
    callArgument(text, "ipcMain.handle", "keepDefaultRuntimePath"),
    context
  )()
  expect(runtimeOperationBrief()).toBe(true)
  beforeQuit(event)
  beforeQuit(event)
  expect(prevented).toBe(2)
  await tick()
  expect(quits).toBe(0)
  forget()
  await keep
  await tick()
  expect(context.UNAVAILABLE_RUNTIME_PATH).toBeNull()
  // Once; its own before-quit then runs the normal shutdown.
  expect(quits).toBe(1)

  // A brief step that fails still lets the quit run.
  const step = withRuntimeOperation(
    async () => {
      await tick()
      throw new Error("synthetic failure")
    },
    { brief: true }
  )
  beforeQuit(event)
  await expect(step).rejects.toThrow("synthetic failure")
  await tick()
  expect(runtimeOperationBrief()).toBe(false)
  expect(quits).toBe(2)
})
