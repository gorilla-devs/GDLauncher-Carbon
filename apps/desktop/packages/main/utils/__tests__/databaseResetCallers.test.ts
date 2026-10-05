import { describe, it, expect, vi } from "vitest"
import fs from "node:fs/promises"
import fss from "node:fs"
import path from "node:path"
import os from "node:os"
import ts from "typescript"
import vm from "node:vm"
import { EventEmitter } from "node:events"
import { resetDatabaseAndRestart } from "../databaseReset"
import {
  terminateAndObserveCoreExit,
  stopCoreForShutdown
} from "../coreShutdown"
import { withRuntimeOperation } from "../runtimeOperation"
import { createCoreOutput } from "../coreOutput"
import getAdSize from "../../adSize"
const displayFixture = vi.hoisted(() => ({
  platform: "win32",
  primary: vi.fn()
}))
vi.mock("electron", () => ({
  screen: { getPrimaryDisplay: displayFixture.primary }
}))
vi.mock("os", async (importOriginal) => {
  const original = await importOriginal<typeof import("node:os")>()
  return {
    ...original,
    default: { ...original, platform: () => displayFixture.platform }
  }
})
function actual(kind: string, name: string, globals: Record<string, unknown>) {
  const text = fss.readFileSync(path.resolve("packages/main/index.ts"), "utf8")
  const ast = ts.createSourceFile(
    "index.ts",
    text,
    ts.ScriptTarget.Latest,
    true
  )
  let expression = ""
  function visit(node: ts.Node) {
    if (
      kind === "ipc" &&
      ts.isCallExpression(node) &&
      node.expression.getText(ast) === "ipcMain.handle" &&
      ts.isStringLiteral(node.arguments[0]) &&
      node.arguments[0].text === name
    )
      expression = node.arguments[1].getText(ast)
    if (
      kind === "function" &&
      ts.isFunctionDeclaration(node) &&
      node.name?.text === name
    )
      expression = node.getText(ast)
    if (
      kind === "exit" &&
      ts.isCallExpression(node) &&
      node.expression.getText(ast) === "coreModule.on" &&
      ts.isStringLiteral(node.arguments[0]) &&
      node.arguments[0].text === "exit"
    )
      expression = node.arguments[1].getText(ast)
    if (
      kind === "core-loaded" &&
      ts.isCallExpression(node) &&
      node.expression.getText(ast) === "coreModule.then" &&
      node.arguments[0].getText(ast).includes(name)
    )
      expression = node.arguments[0].getText(ast)
    ts.forEachChild(node, visit)
  }
  visit(ast)
  if (!expression) throw Error("Actual callback unavailable")
  const statement =
    kind === "function"
      ? expression + `; const actual = ${name}`
      : `const actual = ${expression}`
  const js = ts.transpileModule(statement, {
    compilerOptions: { target: ts.ScriptTarget.ES2022 }
  }).outputText
  globals.Error = Error
  return vm.runInNewContext(`(() => { ${js}; return actual })()`, globals)
}
async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "task24-caller-"))
  await fs.writeFile(path.join(root, "gdl_conf.db"), "metadata")
  const child = Object.assign(new EventEmitter(), {
    exitCode: null,
    signalCode: null,
    kill: vi.fn(() => true)
  })
  const app = { relaunch: vi.fn(), exit: vi.fn() },
    dialog = { showMessageBox: vi.fn(async () => ({ response: 0 })) }
  const globals = {
    CURRENT_RUNTIME_PATH: root,
    RECOVERY_COPY_DAYS: 30,
    coreModule: Promise.resolve({ type: "error", logs: [] }),
    coreProcessHandle: child,
    resetDatabaseAndRestart,
    terminateAndObserveCoreExit,
    stopCoreForShutdown,
    withRuntimeOperation,
    dialog,
    showRecoveryNotice: dialog.showMessageBox,
    app,
    console
  }
  return {
    root,
    child,
    app,
    dialog,
    globals,
    reset: actual("ipc", "deleteDbAndRestart", globals)
  }
}
it("actual reset IPC waits for observed owned pre-READY exit, then retains bytes and restarts", async () => {
  const f = await fixture()
  const pending = f.reset()
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(f.child.kill).toHaveBeenCalledOnce()
  expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
    "metadata"
  )
  expect(f.app.relaunch).not.toHaveBeenCalled()
  f.child.emit("exit", 0)
  const result = await pending
  expect(result.ok).toBe(true)
  expect(
    await fs.readFile(path.join(result.recoveryPath, "gdl_conf.db"), "utf8")
  ).toBe("metadata")
  expect(f.dialog.showMessageBox).toHaveBeenCalledOnce()
  expect(f.app.relaunch).toHaveBeenCalledOnce()
  expect(f.app.exit).toHaveBeenCalledOnce()
})
it("actual reset IPC refuses a running external core without touching files", async () => {
  const f = await fixture()
  f.globals.coreProcessHandle = null as any
  const reset = actual("ipc", "deleteDbAndRestart", f.globals)
  const result = await reset()
  expect(result.ok).toBe(false)
  expect(result.message).toContain("launcher-owned")
  expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
    "metadata"
  )
  expect(f.app.relaunch).not.toHaveBeenCalled()
})
it("actual reset goes ahead when the core was never started: there is no process to wait for", async () => {
  const f = await fixture()
  const { reset, prompts } = f.resetWith({ coreProcessHandle: null })
  const result = await reset()
  expect(result.ok).toBe(true)
  expect(prompts).toEqual([])
  expect(
    await fs.readFile(path.join(result.recoveryPath, "gdl_conf.db"), "utf8")
  ).toBe("metadata")
  expect(f.app.relaunch).toHaveBeenCalledOnce()
})
it("actual reset behind the interrupted restore guard points to Recover Database, not at a development core", async () => {
  const f = await fixture()
  await fs.writeFile(path.join(f.root, ".gdl-db-restore.json"), "{}")
  const { reset } = f.resetWith({ coreProcessHandle: null })
  const result = await reset()
  expect(result.ok).toBe(false)
  expect(result.message).toContain("Recover Database")
  expect(result.message).not.toContain("development core")
  expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
    "metadata"
  )
  expect(f.app.relaunch).not.toHaveBeenCalled()
})
it("actual reset uses quit-purpose readiness and observed exit without selecting game stop", async () => {
  const f = await fixture()
  const calls: string[] = []
  const transport = vi.fn(async (url: any, options: any) => {
    calls.push(String(url).split("/").at(-1)!)
    const body = JSON.parse(options.body)
    expect(body.purpose).toBe("quit")
    return {
      ok: true,
      json: async () => ({
        ...body,
        status: { ...body, phase: "ready", blockers: [] }
      })
    }
  })
  const stop = (core: any, purpose: any) =>
    stopCoreForShutdown(core, purpose, undefined, transport as any)
  f.child.kill.mockImplementation(() => {
    f.child.emit("exit", 0)
    return true
  })
  const reset = actual("ipc", "deleteDbAndRestart", {
    ...f.globals,
    coreModule: Promise.resolve({
      type: "success",
      result: { port: 1, apiToken: "synthetic" }
    }),
    stopCoreForShutdown: stop
  })
  expect((await reset()).ok).toBe(true)
  expect(calls).toEqual(["prepare-shutdown", "shutdown"])
})
it("actual reset preserves sources on unresolved shutdown and reuses request identity", async () => {
  const f = await fixture()
  const ids: string[] = []
  const transport = vi.fn(async (_url: any, options: any) => {
    const body = JSON.parse(options.body)
    ids.push(body.requestId)
    return {
      ok: true,
      json: async () => ({
        ...body,
        status: {
          ...body,
          phase: "unresolved",
          blockers: ["unknown installer"]
        }
      })
    }
  })
  const reset = actual("ipc", "deleteDbAndRestart", {
    ...f.globals,
    coreModule: Promise.resolve({
      type: "success",
      result: { port: 1, apiToken: "synthetic" }
    }),
    stopCoreForShutdown: (core: any, purpose: any) =>
      stopCoreForShutdown(core, purpose, undefined, transport as any)
  })
  for (let i = 0; i < 2; i++) expect((await reset()).ok).toBe(false)
  expect(new Set(ids).size).toBe(1)
  expect(f.child.kill).not.toHaveBeenCalled()
  expect(f.app.relaunch).not.toHaveBeenCalled()
  expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
    "metadata"
  )
})
it("actual reset accepts observed already-exited owned core without signaling", async () => {
  const f = await fixture()
  ;(f.child as any).exitCode = 0
  expect((await f.reset()).ok).toBe(true)
  expect(f.child.kill).not.toHaveBeenCalled()
})
it("actual core exit callback retains owned exit evidence and does not replace newer handle", () => {
  const newer = {}
  const globals = {
    coreProcessHandle: newer,
    coreOutput: { end: vi.fn(), fail: vi.fn() },
    identitySession: "exited-session",
    lifecycle: { endCore: vi.fn(() => false) },
    quitAfterCoreExit: vi.fn()
  }
  actual("exit", "exit", globals)(0)
  expect(globals.coreProcessHandle).toBe(newer)
  // A dead core can no longer arm the close warning or keep the launcher alive.
  expect(globals.lifecycle.endCore).toHaveBeenCalledWith("exited-session")
  expect(globals.quitAfterCoreExit).not.toHaveBeenCalled()
  // Its Close Window game was keeping the windowless launcher alive.
  globals.lifecycle.endCore.mockReturnValue(true)
  actual("exit", "exit", globals)(0)
  expect(globals.quitAfterCoreExit).toHaveBeenCalledOnce()
})
it("actual nondeleting Restart preserves metadata and supports external development restart", async () => {
  const f = await fixture()
  const stop = actual("function", "killCoreProcess", {
    ...f.globals,
    coreProcessHandle: null
  })
  const restart = actual("ipc", "relaunch", {
    ...f.globals,
    killCoreProcess: stop
  })
  await restart()
  expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
    "metadata"
  )
  expect(await fs.readdir(f.root)).toEqual(["gdl_conf.db"])
  expect(f.app.relaunch).toHaveBeenCalledOnce()
})
it("actual IPC serialization retains structured cause and complete Windows snapshot path", async () => {
  let outcome: any
  const core = createCoreOutput({
    log: () => {},
    settle: (r) => {
      outcome = r
    },
    record: () => {},
    progress: () => {}
  })
  core.push(
    Buffer.from(
      "_STATUS_:DB_DOWNGRADE_FAILED|C:\\synthetic\\gdl_conf.pre-downgrade.db\n"
    )
  )
  const result = await actual("ipc", "getCoreModule", {
    coreModule: Promise.resolve(outcome)
  })()
  expect(result).toMatchObject({
    type: "error",
    dbRecovery: "downgrade",
    snapshotPath: "C:\\synthetic\\gdl_conf.pre-downgrade.db"
  })
})
it("actual ad IPC uses the matching display's DIP workArea", async () => {
  vi.stubGlobal("__SHOWCASE_MODE__", false)
  const bounds = { x: 1, y: 1, width: 900, height: 700 }
  const screen = {
    getDisplayMatching: vi.fn(() => ({
      workArea: { width: 1920, height: 1040 },
      scaleFactor: 2
    }))
  }
  const result = await actual("ipc", "getAdSize", {
    screen,
    win: { getBounds: () => bounds },
    getAdSize
  })()
  expect(screen.getDisplayMatching).toHaveBeenCalledWith(bounds)
  expect(result.adSize).toEqual({ width: 440, height: 730, shouldShow: true })
})
describe("actual Recover Database IPC", () => {
  const INTENT = ".gdl-db-restore.json"
  const BUNDLE = "gdl_conf.restore-originals-fixture"
  const recorded = (bytes: string) => ({
    size: Buffer.byteLength(bytes),
    sha256: createHash("sha256").update(bytes).digest("hex")
  })
  /** A restore cut off with its original in the bundle and `active` in the data folder. */
  async function interrupted(active: string) {
    const f = await fixture()
    const bundle = path.join(f.root, BUNDLE)
    await fs.mkdir(bundle)
    await fs.writeFile(path.join(bundle, "snapshot-staged.db"), "snapshot")
    await fs.rename(
      path.join(f.root, "gdl_conf.db"),
      path.join(bundle, "gdl_conf.db")
    )
    await fs.writeFile(path.join(f.root, "gdl_conf.db"), active)
    await fs.writeFile(
      path.join(f.root, INTENT),
      JSON.stringify({
        version: 1,
        bundle: BUNDLE,
        originals: { "gdl_conf.db": recorded("metadata") }
      })
    )
    return { ...f, bundle }
  }
  function recover(
    f: Awaited<ReturnType<typeof interrupted>>,
    extra: Record<string, unknown> = {}
  ) {
    // Behind the guard no core was started: there is nothing to wait for.
    const globals = { ...f.globals, coreProcessHandle: null }
    return actual("ipc", "recoverInterruptedDbRestore", {
      ...globals,
      recoverInterruptedRestore,
      keptBeforeDatabaseChange: () => new DatabaseRefusal("kept open"),
      ownedShutdown: owned(globals).ownedShutdown,
      ...extra
    })
  }
  it("finishing a restore says where the originals are and when they go, then restarts", async () => {
    const f = await interrupted("snapshot")
    const result = await recover(f)()
    expect(result).toMatchObject({ ok: true, restored: true })
    await assertDatabaseRestoreReady(f.root)
    expect(f.dialog.showMessageBox).toHaveBeenCalledOnce()
    const notice = (f.dialog.showMessageBox.mock.calls as any[][])[0][0]
    expect(notice.message).toContain("previous database is in use")
    expect(notice.message).toContain("retained")
    // Named where it really is: /private/var on macOS, long names on Windows.
    expect(notice.detail).toContain(
      `Recovery files: ${await fs.realpath(f.bundle)}`
    )
    expect(notice.detail).toContain("30 days")
    expect(f.app.relaunch).toHaveBeenCalledOnce()
    expect(f.app.exit).toHaveBeenCalledOnce()
  })
  it("undoing a restore says the files are back, with no recovery files to mention", async () => {
    const f = await interrupted("snap")
    const result = await recover(f)()
    expect(result).toMatchObject({ ok: true, restored: false })
    expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
      "metadata"
    )
    const notice = (f.dialog.showMessageBox.mock.calls as any[][])[0][0]
    expect(notice.message).toContain("back as they were")
    expect(notice.detail).toBeUndefined()
    expect(f.app.relaunch).toHaveBeenCalledOnce()
  })
  it("Keep GDLauncher open is reported as that, with every file as it was and no restart", async () => {
    const f = await interrupted("snap")
    const result = await recover(f, { ownedShutdown: async () => "kept" })()
    expect(result).toMatchObject({ ok: false, message: "kept open" })
    expect(await fs.readFile(path.join(f.root, "gdl_conf.db"), "utf8")).toBe(
      "snap"
    )
    await expect(assertDatabaseRestoreReady(f.root)).rejects.toThrow(
      "needs recovery"
    )
    expect(f.dialog.showMessageBox).not.toHaveBeenCalled()
    expect(f.app.relaunch).not.toHaveBeenCalled()
  })
  it("a refusal is given back with no notice and no restart", async () => {
    const f = await interrupted("foreign")
    const error = vi.spyOn(console, "error").mockImplementation(() => {})
    const result = await recover(f)()
    expect(error).toHaveBeenCalled()
    error.mockRestore()
    expect(result.ok).toBe(false)
    expect(result.message).toContain("Discord")
    expect(f.dialog.showMessageBox).not.toHaveBeenCalled()
    expect(f.app.relaunch).not.toHaveBeenCalled()
  })
})
describe("actual removal of old recovery copies when the core has loaded", () => {
  const OLD = "gdl_conf.reset-recovery-2026-01-02T03-04-05-678Z-aBc123"
  async function loaded(core: unknown, active = false) {
    const f = await fixture()
    const folder = path.join(f.root, OLD)
    await fs.mkdir(folder)
    await fs.writeFile(path.join(folder, "gdl_conf.db"), "old tokens")
    const when = new Date(Date.now() - 90 * 24 * 60 * 60 * 1000)
    await fs.utimes(folder, when, when)
    const prune = vi.fn((root: string) => pruneRecoveryCopies(root))
    actual("core-loaded", "pruneRecoveryCopies", {
      CURRENT_RUNTIME_PATH: f.root,
      coreLoaded: undefined,
      runtimeOperationActive: () => active,
      withRuntimeOperation,
      pruneRecoveryCopies: prune,
      console
    })(core)
    await new Promise((resolve) => setTimeout(resolve, 50))
    return { ...f, folder, prune }
  }
  const success = {
    type: "success",
    result: { port: 1, apiToken: "synthetic" }
  }
  it("a clean start removes them", async () => {
    const { folder, prune, root } = await loaded(success)
    expect(prune).toHaveBeenCalledExactlyOnceWith(root)
    await expect(fs.lstat(folder)).rejects.toMatchObject({ code: "ENOENT" })
  })
  it("is a brief step: a quit meanwhile waits for it instead of being dropped", async () => {
    const f = await fixture()
    const brief: boolean[] = []
    actual("core-loaded", "pruneRecoveryCopies", {
      CURRENT_RUNTIME_PATH: f.root,
      coreLoaded: undefined,
      runtimeOperationActive: () => false,
      withRuntimeOperation,
      pruneRecoveryCopies: async (root: string) => {
        brief.push(runtimeOperationBrief())
        return pruneRecoveryCopies(root)
      },
      console
    })(success)
    await vi.waitFor(() => expect(brief).toEqual([true]))
    await vi.waitFor(() => expect(runtimeOperationBrief()).toBe(false))
  })
  it("a failed start keeps them: the user may still need them", async () => {
    const { folder, prune } = await loaded({ type: "error", logs: [] })
    expect(prune).not.toHaveBeenCalled()
    expect(await fs.readFile(path.join(folder, "gdl_conf.db"), "utf8")).toBe(
      "old tokens"
    )
  })
  it("a reset, restore or move in progress keeps them", async () => {
    const { folder, prune } = await loaded(success, true)
    expect(prune).not.toHaveBeenCalled()
    expect(await fs.readFile(path.join(folder, "gdl_conf.db"), "utf8")).toBe(
      "old tokens"
    )
  })
  it("a failure only warns", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const f = await fixture()
    await fs.writeFile(path.join(f.root, ".gdl-db-restore.json"), "{}")
    actual("core-loaded", "pruneRecoveryCopies", {
      CURRENT_RUNTIME_PATH: f.root,
      coreLoaded: undefined,
      runtimeOperationActive: () => false,
      withRuntimeOperation,
      pruneRecoveryCopies,
      console
    })(success)
    await vi.waitFor(() => expect(warn).toHaveBeenCalled())
    warn.mockRestore()
  })
})
