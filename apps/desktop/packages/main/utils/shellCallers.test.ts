import { it, expect, vi } from "vitest"
import fs from "node:fs"
import path from "node:path"
import vm from "node:vm"
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
