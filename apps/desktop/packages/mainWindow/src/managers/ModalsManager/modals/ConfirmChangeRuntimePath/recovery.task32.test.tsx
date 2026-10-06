import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import fs from "node:fs/promises"
import fss from "node:fs"
import os from "node:os"
import path from "node:path"
import ts from "typescript"
import englishJava from "../../../../../../../../../packages/i18n/locale/english/java.json"
import {
  relocateRuntime,
  selectRuntimePath,
  MIGRATION_MARKER
} from "../../../../../../main/utils/runtimeRelocation"
import { withRuntimeOperation } from "../../../../../../main/utils/runtimeOperation"
import { ModalProvider, useModal } from "@/managers/ModalsManager"
import {
  isChangingRuntimePath,
  setIsChangingRuntimePath,
  setRuntimeRecoveryRequired
} from "./state"
import { button, settle } from "@/task32.fixture"
const c = vi.hoisted(() => {
  window.changeRuntimePathProgress = vi.fn()
  return {
    close: vi.fn(),
    protection: () => false,
    manager: undefined as any,
    params: new URLSearchParams()
  }
})
vi.mock("@gd/ui", async () => import("@/task32.fixture"))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key],
  Trans: (p: any) => (
    <span>
      {p.key.includes("runtime_recovery.")
        ? (englishJava as Record<string, string>)[p.key.split(":")[1]]
        : p.key}
    </span>
  )
}))
vi.mock("@solidjs/router", () => ({
  useLocation: () => ({
    get search() {
      return `?${c.params}`
    }
  }),
  useSearchParams: () => [
    undefined,
    (values: Record<string, string | null>) => {
      for (const [k, v] of Object.entries(values)) {
        if (v === null) c.params.delete(k)
        else c.params.set(k, v)
      }
    }
  ]
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/utils/adhelper", () => ({ default: { width: 0 } }))
vi.mock("@/utils/memoryWarningBridge", () => ({
  listenMemoryWarning: () => () => {}
}))
vi.mock("@/utils/serverEulaBridge", () => ({
  listenServerEula: () => () => {}
}))
vi.mock("../CacheCleanup/state", () => ({ cleanupRunning: () => false }))
vi.mock("../ShaderLoaderSetup/state", () => ({
  shaderInstallRunning: () => false
}))
const roots: string[] = []
let dispose: (() => void) | undefined
// The manager loads this modal with `lazy(() => import(...))`. Load it once,
// under the hook timeout, so the first test's `vi.waitFor` does not race its
// cold transform under parallel load.
beforeAll(async () => {
  await import("./index")
})
beforeEach(() => {
  vi.clearAllMocks()
  c.params = new URLSearchParams()
  setIsChangingRuntimePath(false)
  setRuntimeRecoveryRequired(false)
  const overlay = document.createElement("div")
  overlay.id = "overlay"
  document.body.append(overlay)
})
afterEach(async () => {
  dispose?.()
  dispose = undefined
  setIsChangingRuntimePath(false)
  setRuntimeRecoveryRequired(false)
  document.body.textContent = ""
  vi.restoreAllMocks()
  for (const root of roots.splice(0))
    await fs.rm(root, { recursive: true, force: true })
})
function callback(
  globals: Record<string, unknown>,
  name = "changeRuntimePath"
) {
  const s = fss.readFileSync("packages/main/index.ts", "utf8"),
    ast = ts.createSourceFile("main.ts", s, ts.ScriptTarget.Latest, true)
  let expression = ""
  function visit(n: ts.Node) {
    if (
      ts.isCallExpression(n) &&
      n.expression.getText(ast) === "ipcMain.handle" &&
      ts.isStringLiteral(n.arguments[0]) &&
      n.arguments[0].text === name
    )
      expression = n.arguments[1].getText(ast)
    ts.forEachChild(n, visit)
  }
  visit(ast)
  const js = ts.transpileModule(`const callback = ${expression}`, {
    compilerOptions: { target: ts.ScriptTarget.ES2022 }
  }).outputText
  return new Function(...Object.keys(globals), `${js}; return callback`)(
    ...Object.values(globals)
  )
}
async function fixture(relocate: typeof relocateRuntime = relocateRuntime) {
  // Canonical, like the paths the relocation code compares against: the
  // system temp dir is behind a symlink on macOS (/var -> /private/var).
  const root = await fs.realpath(
    await fs.mkdtemp(path.join(os.tmpdir(), "gdl-task32-runtime-"))
  )
  roots.push(root)
  const source = path.join(root, "source"),
    target = path.join(root, "target"),
    pointer = path.join(root, "runtime_path_override")
  await fs.mkdir(source)
  await fs.mkdir(target)
  await fs.writeFile(path.join(source, "a"), "first bytes")
  await fs.writeFile(path.join(source, "b"), "second bytes")
  await fs.writeFile(pointer, source)
  const stop = vi.fn(async () => {}),
    restart = vi.fn(async (_selected?: string) => {}),
    notice = vi.fn(async () => {})
  const globals = {
    withRuntimeOperation,
    relocateRuntime,
    CURRENT_RUNTIME_PATH: source,
    overrideCLIDataPath: undefined,
    overrideEnvDataPath: undefined,
    runtimeDestinationContext: () => ({}),
    path,
    app: { getPath: () => root, relaunch: restart, exit: vi.fn() },
    RUNTIME_PATH_OVERRIDE_NAME: "runtime_path_override",
    coreModule: Promise.resolve({ type: "success", result: {} }),
    coreProcessHandle: {},
    stopCoreForRelocation: stop,
    win: null,
    console,
    dialog: { showMessageBox: notice }
  }
  const move = callback(globals)
  window.getRuntimePath = async () => source
  window.changeRuntimePath = (target, switchOnly) =>
    move(null, target, switchOnly)
  window.relaunch = callback(
    {
      withRuntimeOperation,
      killCoreProcess: async () => {
        const selected = selectRuntimePath(null, root)
        await restart(selected)
      },
      app: { relaunch: vi.fn(), exit: vi.fn() },
      console
    },
    "relaunch"
  )
  const element = document.createElement("div")
  document.body.append(element)
  const Capture = () => {
    c.manager = useModal()
    return null
  }
  dispose = render(
    () => (
      <ModalProvider>
        <Capture />
      </ModalProvider>
    ),
    element
  )
  await settle()
  c.manager.openModal(
    { name: "ConfirmChangeRuntimePath" },
    { runtimePath: target, isTargetFolderAlreadyUsed: false }
  )
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain("confirm_change_confirm_button")
  )
  c.protection = () => {
    const before = c.manager.hasOpenModals()
    document.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
    )
    document.querySelector<HTMLElement>('[data-testid="modal-close"]')?.click()
    const modal = document.querySelector(".animate-modalEnter")
    modal?.parentElement?.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true })
    )
    return before && c.manager.hasOpenModals()
  }
  return {
    root,
    source,
    target,
    pointer,
    stop,
    restart,
    notice,
    element: document.body,
    move
  }
}
async function submit(f: Awaited<ReturnType<typeof fixture>>) {
  button(f.element, "confirm_change_confirm_button").click()
  await vi.waitFor(() => expect(isChangingRuntimePath()).toBe(false))
  await settle()
}
it("0242 actual second copy failure protects error and full recovery chooses persisted OLD pointer", async () => {
  const f = await fixture(),
    copy = fs.copyFile.bind(fs)
  let copies = 0
  vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
    if (++copies === 2) throw Error("second copy failed")
    return (copy as any)(...args)
  }) as any)
  await submit(f)
  expect(f.stop).toHaveBeenCalledTimes(1)
  expect(selectRuntimePath(null, f.root)).toBe(f.source)
  expect(fss.existsSync(path.join(f.target, MIGRATION_MARKER))).toBe(true)
  expect(c.protection()).toBe(true)
  expect(document.body.textContent).toContain("Restart")
  expect(
    Array.from(document.querySelectorAll("button")).some((b) =>
      b.textContent?.includes("general:_trn_dismiss")
    )
  ).toBe(false)
  button(document.body, "Restart").click()
  await settle()
  expect(f.restart).toHaveBeenCalledTimes(1)
  expect(f.restart).toHaveBeenCalledWith(f.source)
})
it("0242 post-stop revalidation failure protects retained data", async () => {
  const f = await fixture()
  f.stop.mockImplementation(async () => {
    await fs.writeFile(path.join(f.target, "foreign"), "foreign")
  })
  await submit(f)
  expect(c.protection()).toBe(true)
  expect(selectRuntimePath(null, f.root)).toBe(f.source)
  expect(await fs.readFile(path.join(f.target, "foreign"), "utf8")).toBe(
    "foreign"
  )
})
it("0242 committed pointer survives notice failure and recovery selects NEW pointer", async () => {
  const f = await fixture()
  f.notice.mockRejectedValue(Error("notice unavailable"))
  await submit(f)
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
  expect(f.restart).not.toHaveBeenCalled()
  expect(c.protection()).toBe(true)
  button(document.body, "Restart").click()
  await settle()
  expect(f.restart).toHaveBeenCalledTimes(1)
  expect(f.restart).toHaveBeenCalledWith(f.target)
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it("0242 ordinary pre-stop validation error remains dismissible", async () => {
  const f = await fixture()
  await fs.writeFile(path.join(f.target, "occupied"), "data")
  await submit(f)
  expect(f.stop).not.toHaveBeenCalled()
  button(document.body, "general:_trn_dismiss").click()
  expect(c.manager.hasOpenModals()).toBe(false)
})
it("0242 unresolved shutdown retries same relocation and remains protected without falsely claiming recovery", async () => {
  const f = await fixture()
  f.stop.mockRejectedValue(
    Error("Shutdown unresolved, live core ownership retained")
  )
  f.restart.mockRejectedValue(Error("Shutdown owned by relocation"))
  await submit(f)
  expect(c.protection()).toBe(true)
  expect(await fs.readdir(f.target)).toEqual([])
  button(document.body, "Retry relocation").click()
  await vi.waitFor(() => expect(f.stop).toHaveBeenCalledTimes(2))
  await settle()
  expect(c.protection()).toBe(true)
  expect(c.manager.hasOpenModals()).toBe(true)
  expect(f.restart).not.toHaveBeenCalled()
  expect(document.body.textContent).toContain("Shutdown unresolved")
})

it("0242 unresolved retry joins original operation after guard release and can finish relocation", async () => {
  const f = await fixture()
  f.stop.mockRejectedValueOnce(Error("Shutdown unresolved"))
  await submit(f)
  expect(c.protection()).toBe(true)
  button(document.body, "Retry relocation").click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(f.stop).toHaveBeenCalledTimes(2)
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
  expect(c.manager.hasOpenModals()).toBe(false)
})
it("0242 failed actual guarded Restart remains protected and can be retried", async () => {
  const f = await fixture()
  f.notice.mockRejectedValue(Error("notice unavailable"))
  f.restart.mockRejectedValueOnce(Error("guarded restart refused"))
  await submit(f)
  button(document.body, "Restart").click()
  await settle()
  expect(c.manager.hasOpenModals()).toBe(true)
  expect(c.protection()).toBe(true)
  expect(document.body.textContent).toContain("guarded restart refused")
  button(document.body, "Restart").click()
  await settle()
  expect(f.restart).toHaveBeenCalledTimes(2)
})
