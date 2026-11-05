import { afterEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { EventEmitter } from "node:events"
import fs from "node:fs"
import path from "node:path"
import vm from "node:vm"
import ts from "typescript"
import { toast, useSomoto } from "somoto"
import { createUpdaterController } from "../../../main/utils/updaterController"
import { ownsDownloadedUpdate } from "../../../main/utils/downloadedUpdate"
import { createOwnedInstallDispatch } from "../../../main/utils/ownedInstallDispatch"
import { createOwnedNativePreparation } from "../../../main/utils/ownedNativePreparation"
import { installReadyUpdateAction } from "../../../preload/installReadyUpdate"
vi.mock("@gd/ui", async () => ({
  toast: (await import("somoto")).toast,
  Progress: () => <span />
}))
vi.mock("@gd/i18n", () => ({
  i18n: { language: "english", on: vi.fn(), t: (key: string) => key }
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    createQuery: () => {
      throw Error("No query/transport admitted")
    }
  }
}))
const root = path.resolve(__dirname, "../../../../../..")
const macPath = path.join(
  root,
  "node_modules/electron-updater/out/MacUpdater.js"
)
const mac = ts.createSourceFile(
  macPath,
  fs.readFileSync(macPath, "utf8"),
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.JS
)
const definition = mac.statements.find(
  (node) => ts.isClassDeclaration(node) && node.name?.text === "MacUpdater"
) as ts.ClassDeclaration
const method = definition.members.find(
  (node) =>
    ts.isMethodDeclaration(node) && node.name.getText(mac) === "quitAndInstall"
)!
// Only these benign method/callback bodies run, with an in-memory emitter.
// Native imports, constructor, server, file streams and processes never run.
const quit = vm.runInNewContext(`({${method.getText(mac)}}).quitAndInstall`)
const listeners = new Map<string, string>()
function visit(node: ts.Node) {
  if (
    ts.isCallExpression(node) &&
    node.expression.getText(mac) === "this.nativeUpdater.on" &&
    ts.isStringLiteral(node.arguments[0])
  )
    listeners.set(node.arguments[0].text, node.arguments[1].getText(mac))
  ts.forEachChild(node, visit)
}
visit(definition.members.find(ts.isConstructorDeclaration)!)
let finishBody: string | undefined
function findFinish(node: ts.Node) {
  if (
    ts.isCallExpression(node) &&
    node.expression.getText(mac) === "response.on" &&
    node.arguments[0].getText(mac) === '"finish"'
  )
    finishBody = node.arguments[1].getText(mac)
  ts.forEachChild(node, findFinish)
}
findFinish(
  definition.members.find(
    (node) =>
      ts.isMethodDeclaration(node) &&
      node.name.getText(mac) === "updateDownloaded"
  )!
)
if (!finishBody) throw Error("Actual Mac response finish callback absent")
function extract(text: string, name: string, call = false) {
  const parsed = ts.createSourceFile(
    "fixture.ts",
    text,
    ts.ScriptTarget.Latest,
    true
  )
  let body: string | undefined
  function find(node: ts.Node) {
    if (
      !call &&
      ts.isVariableDeclaration(node) &&
      node.name.getText(parsed) === name
    )
      body = node.initializer?.getText(parsed)
    if (
      call &&
      ts.isCallExpression(node) &&
      node.expression.getText(parsed) === "checkUpdatesBtn.addEventListener" &&
      node.arguments[0].getText(parsed) === '"click"'
    )
      body = node.arguments[1].getText(parsed)
    ts.forEachChild(node, find)
  }
  find(parsed)
  if (!body) throw Error("Actual consumer source absent: " + name)
  return body
}
function evaluate(body: string, context: Record<string, unknown>) {
  return vm.runInNewContext(
    ts.transpileModule("(" + body + ")", {
      compilerOptions: {
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.CommonJS
      }
    }).outputText,
    context
  )
}
function deferred() {
  let resolve!: (value: any) => void, reject!: (error: Error) => void
  const promise = new Promise<any>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
let listener: ((_: unknown, state: any) => void) | undefined
let renderer: typeof import("./updater") | undefined,
  dispose: (() => void) | undefined,
  offset = 0
const flush = async () => {
  for (let i = 0; i < 60; i++) await Promise.resolve()
}
afterEach(() => {
  dispose?.()
  toast.dismiss()
  offset += 1000
})
async function fixture(main = false, delayedFirstResult = false) {
  const events: Record<string, (...args: any[]) => void> = {},
    native = new EventEmitter(),
    helper: any = {}
  const operations: {
    info: any
    work: ReturnType<typeof deferred>
    result: ReturnType<typeof deferred>
    downloadPromise: Promise<any>
    active: boolean
  }[] = []
  const nativeCheck = vi.fn(),
    handoff = vi.fn(),
    shutdown = vi.fn(async () => {})
  const updater: any = {
    nativeUpdater: native,
    downloadedUpdateHelper: helper,
    autoInstallOnAppQuit: true,
    squirrelDownloadedUpdate: false,
    _logger: { warn() {} },
    debug() {},
    emit: (event: string, ...args: any[]) => events[event]?.(...args),
    channel: null,
    allowPrerelease: false,
    allowDowngrade: false,
    on: (event: string, cb: any) => {
      events[event] = cb
    },
    handleUpdateDownloaded: handoff,
    quitAndInstall: vi.fn(function (this: any, ...args: any[]) {
      return quit.apply(this, args)
    })
  }
  ;(native as any).checkForUpdates = nativeCheck
  for (const [event, body] of listeners)
    native.on(
      event,
      vm.runInNewContext(`(function () { return ${body} })`).call(updater)
    )
  const baselineError = native.listeners("error"),
    baselineCompletion = native.listeners("update-downloaded")
  updater.checkForUpdates = vi.fn(async () => {
    const info = {
      version: "2.0.0",
      path: "fixture.zip",
      sha512: "owned" + operations.length,
      releaseDate: "2026-10-01",
      files: [{ url: "fixture.zip", sha512: "owned" + operations.length }]
    }
    const work = deferred(),
      result = deferred()
    const downloadPromise = work.promise.catch((error) => {
      updater.emit("error", error)
      throw error
    })
    // A delayed result still has a rejection observer; production's exposed
    // promise is retained by the controller when that check result arrives.
    void downloadPromise.catch(() => {})
    const operation = { info, work, result, downloadPromise, active: true }
    operations.push(operation)
    events["update-available"](info)
    Object.assign(helper, {
      file: "/synthetic/fixture.zip",
      versionInfo: info,
      fileInfo: { info: info.files[0] },
      downloadedFileInfo: { sha512: info.files[0].sha512 }
    })
    events["update-downloaded"]()
    native.once("error", work.reject)
    nativeCheck()
    const value = {
      isUpdateAvailable: true,
      updateInfo: info,
      versionInfo: info,
      downloadPromise
    }
    if (delayedFirstResult && operations.length === 1) return result.promise
    return value
  })
  let getState!: () => any,
    check!: (channel: any) => Promise<any>,
    install!: (id: number) => Promise<void>
  const broadcast = (state: any) =>
    listener?.(null, { ...state, revision: state.revision + offset })
  if (main) {
    const handlers: Record<string, (...args: any[]) => any> = {},
      exports = {},
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
                  send: (_: string, state: any) => broadcast(state)
                }
              }
            ]
          }
        },
        "electron-updater": { autoUpdater: updater },
        "electron-log/main": { transports: { file: {} } },
        "node:fs/promises": {
          readFile: async () => {
            throw Object.assign(Error("missing synthetic marker"), {
              code: "ENOENT"
            })
          }
        },
        "node:path": path,
        "./utils/runtimeOperation": { assertRuntimeOperationAvailable() {} },
        semver: { gt: () => true },
        "./utils/updateProvider":
          await import("../../../main/utils/updateProvider"),
        "./utils/updaterController": { createUpdaterController },
        "./utils/downloadedUpdate": { ownsDownloadedUpdate },
        "./utils/ownedInstallDispatch": { createOwnedInstallDispatch },
        "./utils/appImageInstall": { appImageInstallBlocker: async () => null }
      }
    const source = fs.readFileSync(
      path.join(root, "apps/desktop/packages/main/autoUpdater.ts"),
      "utf8"
    )
    const compiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        esModuleInterop: true
      }
    }).outputText
    vm.runInNewContext(compiled, {
      exports,
      require: (name: string) => {
        if (!(name in mocks)) throw Error("Unexpected wrapper import: " + name)
        return mocks[name]
      },
      __APP_VERSION__: "1.0.0",
      process: {
        platform: "darwin",
        arch: "x64",
        resourcesPath: "/synthetic",
        env: {}
      }
    })
    ;(exports as any).default(undefined, shutdown)
    getState = handlers.getUpdateState
    check = (channel) => handlers.checkForUpdates(null, channel)
    install = (id) => handlers.installUpdate(null, id)
  } else {
    const controller = createUpdaterController({
      version: "1.0.0",
      updater,
      dispatchInstall: createOwnedInstallDispatch(updater),
      isDownloaded: (info) => ownsDownloadedUpdate(updater, info),
      artifact: () => null,
      manual: async () => ({ noUpdate: true }),
      prepareShutdown: shutdown,
      broadcast
    })
    getState = controller.getState
    check = controller.check
    install = controller.install
  }
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: true }),
    getUpdateState: async () => ({
      ...getState(),
      revision: getState().revision + offset
    }),
    onUpdateStateChanged: (cb: any) => {
      listener = cb
    },
    checkForUpdates: check,
    installUpdate: install
  })
  renderer ??= await import("./updater")
  let store!: ReturnType<typeof useSomoto>
  createRoot((d) => {
    dispose = d
    store = useSomoto()
  })
  await flush()
  const presentation = () =>
    store.toasts().find((t) => t.id === "updater-downloading") as any
  const checking = () =>
    store.toasts().find((t) => t.id === "updater-checking") as any
  const fail = (index = 0) => {
    operations[index].active = false
    const error = Error("Injected native preparation failure")
    native.emit("error", error)
    return error
  }
  const finish = async (index: number) => {
    const op = operations[index]
    op.active = false
    native.removeListener("error", op.work.reject)
    op.work.resolve([])
    await flush()
    native.emit("update-downloaded")
    await flush()
  }
  const transferFinished = async (index = 0) => {
    const operation = operations[index]
    vm
      .runInNewContext(`(function () { return ${finishBody} })`, {
        errorOccurred: false,
        reject: operation.work.reject,
        resolve: operation.work.resolve
      })
      .call(updater)()
    await flush()
    expect(updater.squirrelDownloadedUpdate).toBe(false)
  }
  return {
    updater,
    native,
    helper,
    operations,
    nativeCheck,
    handoff,
    shutdown,
    getState,
    check,
    install,
    presentation,
    checking,
    fail,
    finish,
    transferFinished,
    store,
    events,
    baselineError,
    baselineCompletion
  }
}
it.each(["before click", "during install"])(
  "failed source-derived Mac preparation %s retires only its readiness and Retry starts fresh work",
  async (timing) => {
    const f = await fixture()
    await renderer!.manualCheckForUpdates("beta")
    await flush()
    const token = f.getState().readyUpdate.id,
      oldAction = f.presentation().action.onClick
    let clicked: Promise<void> | undefined
    if (timing === "during install") {
      const before = f.native.listeners("update-downloaded")
      expect(before).toHaveLength(2)
      expect(before[0]).toBe(f.baselineCompletion[0])
      // Install waits for Squirrel.Mac with the core still running: nothing
      // is stopped and no install callback is queued yet.
      clicked = oldAction()
      await flush()
      expect(f.native.listeners("update-downloaded")).toEqual(before)
      expect(f.shutdown).not.toHaveBeenCalled()
    }
    f.fail()
    await flush()
    await clicked
    expect(ownsDownloadedUpdate(f.updater, f.operations[0].info)).toBe(true)
    expect(f.getState().readyUpdate).toBeNull()
    expect(f.getState().restartRequired).toBeUndefined()
    expect(f.native.listenerCount("update-downloaded")).toBe(1)
    const failed = timing === "during install" ? f.presentation() : f.checking()
    expect(failed).toMatchObject({
      type: "error",
      duration: 8000,
      closeButton: true
    })
    expect(failed.action).toBeDefined()
    await oldAction()
    await flush()
    expect(f.updater.quitAndInstall).not.toHaveBeenCalled()
    expect(f.shutdown).not.toHaveBeenCalled()
    await failed.action.onClick()
    await flush()
    expect(f.updater.checkForUpdates).toHaveBeenCalledTimes(2)
    expect(f.nativeCheck).toHaveBeenCalledTimes(2)
    expect(f.operations[1].active).toBe(true)
    expect(f.getState().readyUpdate.id).not.toBe(token)
    expect(f.updater.channel).toBe("beta")
    const installing = f.presentation().action.onClick()
    await flush()
    await f.finish(1)
    await installing
    expect(f.shutdown).toHaveBeenCalledTimes(1)
    expect(f.handoff).toHaveBeenCalledTimes(1)
    expect(f.updater.squirrelDownloadedUpdate).toBe(true)
    // Squirrel.Mac already had the update, so no completion callback waits.
    expect(f.native.listenerCount("update-downloaded")).toBe(1)
  }
)
it("healthy outer Mac readiness waits for native preparation before stopping the core, then completes", async () => {
  const f = await fixture()
  await f.check("stable")
  await flush()
  expect((await f.check("beta")).status).toBe("busy")
  expect(f.updater.checkForUpdates).toHaveBeenCalledTimes(1)
  const installing = f.presentation().action.onClick()
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  expect(f.updater.quitAndInstall).not.toHaveBeenCalled()
  expect(f.nativeCheck).toHaveBeenCalledTimes(1)
  await f.finish(0)
  await installing
  expect(f.shutdown).toHaveBeenCalledTimes(1)
  expect(f.updater.quitAndInstall).toHaveBeenCalledWith(true, true)
  expect(f.handoff).toHaveBeenCalledTimes(1)
  expect(f.getState().readyUpdate.version).toBe("2.0.0")
})
it("a delayed old check's rejected preparation cannot retire a newer same-version operation", async () => {
  const f = await fixture(false, true),
    oldCheck = f.check("stable")
  await flush()
  f.fail()
  await flush()
  await f.check("beta")
  await flush()
  const successor = f.getState().readyUpdate.id
  const old = f.operations[0]
  old.result.resolve({
    isUpdateAvailable: true,
    updateInfo: old.info,
    versionInfo: old.info,
    downloadPromise: old.downloadPromise
  })
  await oldCheck
  await flush()
  expect(f.getState().readyUpdate.id).toBe(successor)
  expect(f.getState().state).toBe("downloaded")
  expect(f.presentation().type).toBe("success")
  expect(ownsDownloadedUpdate(f.updater, f.operations[1].info)).toBe(true)
  const installing = f.presentation().action.onClick()
  await flush()
  await f.finish(1)
  await installing
  expect(f.handoff).toHaveBeenCalledTimes(1)
})
it.each([false, true])(
  "actual main IPC policy and fatal consumer retire failed preparation after serving=%s and Retry checks before installing fresh authority",
  async (postServing) => {
    const f = await fixture(true)
    await f.check("stable")
    await flush()
    expect(f.updater.autoInstallOnAppQuit).toBe(true)
    const text = fs.readFileSync(
      path.join(root, "apps/desktop/packages/preload/loading.ts"),
      "utf8"
    )
    const button = { textContent: "", disabled: false, style: {} },
      status = { textContent: "", style: {} },
      context: Record<string, any> = {
        lastUpdateRevision: -1,
        readyDownloadId: undefined,
        updateState: "idle",
        updateInfo: null,
        updateStatusText: status,
        updateProgressContainer: { style: {} },
        updateProgressBar: { style: {} },
        checkUpdatesBtn: button,
        // The loading screen's own install action, as loading.ts builds it.
        installReady: installReadyUpdateAction(
          () => context.readyDownloadId,
          (id) => f.install(id),
          (message) => context.setUpdateState("error", { message })
        ),
        checkForUpdates: async () => {
          await f.check("stable")
          receive(f.getState())
        }
      }
    context.setUpdateState = evaluate(extract(text, "setUpdateState"), context)
    const receive = evaluate(extract(text, "receiveUpdateState"), context),
      click = evaluate(extract(text, "click", true), context)
    receive(f.getState())
    const clicked = click()
    await flush()
    if (postServing) await f.transferFinished()
    f.fail()
    await flush()
    await clicked
    receive(f.getState())
    expect(context.readyDownloadId).toBeUndefined()
    expect(button).toMatchObject({ textContent: "Retry", disabled: false })
    // The failed preparation was waited for before the core was stopped.
    expect(f.shutdown).not.toHaveBeenCalled()
    await click()
    await flush()
    expect(f.updater.checkForUpdates).toHaveBeenCalledTimes(2)
    expect(f.nativeCheck).toHaveBeenCalledTimes(2)
    expect(button.textContent).toBe("Apply & Restart")
    const installing = click()
    await flush()
    await f.finish(1)
    await installing
    expect(f.handoff).toHaveBeenCalledTimes(1)
    expect(f.shutdown).toHaveBeenCalledTimes(1)
  }
)
it.each(["before click", "during install"])(
  "post-serving native failure %s retires the failed authority and starts fresh actual Mac work",
  async (timing) => {
    const f = await fixture(timing === "during install")
    await renderer!.manualCheckForUpdates("beta")
    await flush()
    const token = f.getState().readyUpdate.id,
      oldAction = f.presentation().action.onClick
    let clicked: Promise<void> | undefined
    if (timing === "during install") {
      clicked = oldAction()
      await flush()
    }
    await f.transferFinished()
    f.fail()
    await flush()
    await clicked
    expect(f.shutdown).not.toHaveBeenCalled()
    expect(ownsDownloadedUpdate(f.updater, f.operations[0].info)).toBe(true)
    expect(f.getState().readyUpdate).toBeNull()
    expect(f.native.listeners("update-downloaded")).toEqual(
      f.baselineCompletion
    )
    expect(f.native.listeners("error")).toEqual(f.baselineError)
    const failed = f.presentation()
    expect(failed).toMatchObject({
      type: "error",
      duration: 8000,
      closeButton: true
    })
    expect(failed.action).toBeDefined()
    await oldAction()
    await flush()
    expect(f.updater.quitAndInstall).not.toHaveBeenCalled()
    await failed.action.onClick()
    await flush()
    expect(f.updater.checkForUpdates).toHaveBeenCalledTimes(2)
    expect(f.nativeCheck).toHaveBeenCalledTimes(2)
    expect(f.getState().readyUpdate.id).not.toBe(token)
    expect(f.updater.channel).toBe("beta")
    const installing = f.presentation().action.onClick()
    await flush()
    await f.transferFinished(1)
    f.native.emit("update-downloaded")
    await installing
    await flush()
    expect(f.handoff).toHaveBeenCalledTimes(1)
    expect(f.updater.squirrelDownloadedUpdate).toBe(true)
    expect(f.native.listeners("error")).toEqual(f.baselineError)
  }
)
it("successful actual response finish retains native ownership until native completion while healthy outer Install works", async () => {
  const f = await fixture()
  await f.check("stable")
  await flush()
  await f.transferFinished()
  expect((await f.check("beta")).status).toBe("busy")
  expect(f.updater.checkForUpdates).toHaveBeenCalledTimes(1)
  const installing = f.presentation().action.onClick()
  await flush()
  expect(f.updater.quitAndInstall).not.toHaveBeenCalled()
  f.native.emit("update-downloaded")
  await installing
  expect(f.updater.quitAndInstall).toHaveBeenCalledWith(true, true)
  await flush()
  expect(f.handoff).toHaveBeenCalledTimes(1)
  expect(f.native.listeners("error")).toEqual(f.baselineError)
  // Squirrel.Mac already had the update, so no completion callback waits.
  expect(f.native.listeners("update-downloaded")).toEqual(f.baselineCompletion)
})
it("retired native callbacks and the same known failure cannot corrupt or hand off a newer same-version owner", async () => {
  const f = await fixture()
  await f.check("stable")
  await flush()
  await f.transferFinished()
  const oldError = f.native
    .listeners("error")
    .find((cb) => !f.baselineError.includes(cb))
  const oldCompletion = f.native
    .listeners("update-downloaded")
    .find((cb) => !f.baselineCompletion.includes(cb))
  f.fail()
  await flush()
  expect(f.getState().readyUpdate).toBeNull()
  expect(oldError).toBeDefined()
  expect(oldCompletion).toBeDefined()
  await f.presentation().action.onClick()
  await flush()
  const successor = f.getState().readyUpdate.id
  const state = f.getState()
  oldError!(Error("Disposed old callback"))
  oldCompletion!()
  await flush()
  expect(f.getState()).toBe(state)
  expect(f.presentation().type).toBe("success")
  expect(f.handoff).not.toHaveBeenCalled()
  // A known native error can be forwarded again by AppUpdater's catch after a
  // fresh owner was admitted. Only that exact error object is fenced.
  const known = f.fail(1)
  await flush()
  await f.presentation().action.onClick()
  await flush()
  const third = f.getState(),
    thirdToken = third.readyUpdate.id
  f.events.error(known)
  oldError!(known)
  oldCompletion!()
  await flush()
  expect(f.getState()).toBe(third)
  expect(thirdToken).not.toBe(successor)
  expect(state.readyUpdate.id).toBe(successor)
  expect(f.presentation().type).toBe("success")
  expect(f.handoff).not.toHaveBeenCalled()
  const installing = f.presentation().action.onClick()
  await flush()
  await f.transferFinished(2)
  f.native.emit("update-downloaded")
  await installing
  await flush()
  expect(f.handoff).toHaveBeenCalledTimes(1)
})
it("native preparation adapter disposes only its own listeners once on disposal, completion or error", async () => {
  const native = new EventEmitter(),
    baselineError = vi.fn(),
    baselineCompletion = vi.fn(),
    unrelated = vi.fn()
  native.on("error", baselineError)
  native.on("update-downloaded", baselineCompletion)
  native.on("unrelated", unrelated)
  const source = {
    nativeUpdater: native,
    autoInstallOnAppQuit: true,
    squirrelDownloadedUpdate: false
  }
  const adapter = createOwnedNativePreparation(source),
    failed = vi.fn()
  const first = adapter.watch(failed)!
  const oldError = native.listeners("error")[0],
    oldCompletion = native.listeners("update-downloaded")[1]
  expect(first.pending()).toBe(true)
  first.dispose()
  first.dispose()
  oldError(Error("retired"))
  oldCompletion()
  expect(failed).not.toHaveBeenCalled()
  expect(first.pending()).toBe(false)
  expect(native.listeners("error")).toEqual([baselineError])
  expect(native.listeners("update-downloaded")).toEqual([baselineCompletion])
  const second = adapter.watch(failed)!
  native.emit("update-downloaded")
  expect(second.pending()).toBe(false)
  second.dispose()
  const third = adapter.watch(failed)!,
    error = Error("owned native failure")
  native.emit("error", error)
  expect(third.pending()).toBe(false)
  expect(failed).toHaveBeenCalledExactlyOnceWith(error)
  expect(adapter.ownsError(error)).toBe(true)
  expect(adapter.ownsError(Error("different"))).toBe(false)
  third.dispose()
  expect(native.listeners("error")).toEqual([baselineError])
  expect(native.listeners("update-downloaded")).toEqual([baselineCompletion])
  expect(native.listeners("unrelated")).toEqual([unrelated])
  expect(baselineError).toHaveBeenCalledExactlyOnceWith(error)
  expect(baselineCompletion).toHaveBeenCalledTimes(1)
  // An install waiting on any of them carries on.
  await Promise.all([first.settled, second.settled, third.settled])
  expect(createOwnedNativePreparation({}).watch(failed)).toBeUndefined()
  expect(
    createOwnedNativePreparation({
      ...source,
      autoInstallOnAppQuit: false
    }).watch(failed)
  ).toBeUndefined()
  expect(native.listeners("error")).toEqual([baselineError])
})
it("completed native preparation permits healthy later checks and retains readiness across a generic provider failure", async () => {
  const f = await fixture()
  await f.check("stable")
  await flush()
  await f.transferFinished()
  f.native.emit("update-downloaded")
  await flush()
  const token = f.getState().readyUpdate.id
  expect(f.native.listeners("error")).toEqual(f.baselineError)
  expect(f.native.listeners("update-downloaded")).toEqual(f.baselineCompletion)
  f.updater.checkForUpdates.mockImplementationOnce(async () => {
    f.events["update-not-available"]()
    return {}
  })
  await f.check("stable")
  expect(f.updater.checkForUpdates).toHaveBeenCalledTimes(2)
  expect(f.getState().readyUpdate.id).toBe(token)
  f.updater.checkForUpdates.mockRejectedValueOnce(
    Error("Generic provider check failed")
  )
  await f.check("beta")
  await flush()
  expect(f.getState().state).toBe("error")
  expect(f.getState().readyUpdate.id).toBe(token)
  f.updater.checkForUpdates.mockImplementationOnce(async () => {
    f.events["update-not-available"]()
    return {}
  })
  await renderer!.manualCheckForUpdates("beta")
  await flush()
  expect(f.presentation().type).toBe("success")
  await f.presentation().action.onClick()
  await flush()
  expect(f.handoff).toHaveBeenCalledTimes(1)
  expect(f.updater.quitAndInstall).toHaveBeenCalledWith(true, true)
})
