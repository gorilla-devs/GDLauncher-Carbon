import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import fs from "node:fs/promises"
import fss from "node:fs"
import os from "node:os"
import path from "node:path"
import ts from "typescript"
import i18next from "i18next"
import englishJava from "../../../../../../../../../packages/i18n/locale/english/java.json"
import {
  forgetUnavailableRuntimePath,
  leftInOldFolder,
  leftPartialCopy,
  unsupportedRuntimeEntries,
  removedPartialCopy,
  relocateRuntime,
  selectRuntimePath,
  MIGRATION_MARKER
} from "../../../../../../main/utils/runtimeRelocation"
import {
  interruptibleUntilCommitted,
  withRuntimeOperation
} from "../../../../../../main/utils/runtimeOperation"
import {
  ShutdownRefusedError,
  relocationLeaseHeld
} from "../../../../../../main/utils/coreShutdown"
import {
  dismissRuntimeMoveNotice,
  recordRuntimeMoveNotice,
  takeRuntimeMoveNotice,
  RUNTIME_MOVE_NOTICE_NAME
} from "../../../../../../main/utils/runtimeMoveNotice"
import {
  showRuntimeMoveNotice,
  showUnavailableRuntimePath
} from "@/utils/runtimeMoveNotice"
import { ModalProvider, useModal } from "@/managers/ModalsManager"
import {
  isChangingRuntimePath,
  runtimeRecoveryRequired,
  setIsChangingRuntimePath,
  setRuntimeRecoveryRequired
} from "./state"
import { button, settle } from "@/task32.fixture"
import { RTsetProgress } from "@/utils/runtimePathProgress"
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
// The app's i18next, with its default escaping, renders these as text like Trans does.
let english: ReturnType<typeof i18next.createInstance> | undefined
function translate(key: string, options?: Record<string, unknown>) {
  if (!english) {
    english = i18next.createInstance()
    void english.init({
      lng: "en",
      initAsync: false,
      resources: { en: { java: englishJava } }
    })
  }
  return english.t(key, options)
}
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key],
  Trans: (p: any) => (
    <span>
      {/runtime_recovery\.|runtime_relocation_busy\.|_trn_copying_file|_trn_removing_file|_trn_removing_partial_copy|_trn_saving_copy/.test(
        p.key
      )
        ? translate(p.key, p.options)
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
  // eslint-disable-next-line @typescript-eslint/no-implied-eval -- evaluates the transpiled IPC callback under test
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
    check = vi.fn(async (_core: unknown) => {}),
    restart = vi.fn(async (_selected?: string) => {}),
    notice = vi.fn(async () => {})
  const globals = {
    withRuntimeOperation,
    interruptibleUntilCommitted,
    relocationLeaseHeld,
    relocateRuntime: relocate,
    leftInOldFolder,
    recordRuntimeMoveNotice,
    leftPartialCopy,
    unsupportedRuntimeEntries,
    removedPartialCopy,
    CURRENT_RUNTIME_PATH: source,
    overrideCLIDataPath: undefined,
    overrideEnvDataPath: undefined,
    runtimeDestinationContext: () => ({}),
    path,
    app: { getPath: () => root, relaunch: restart, exit: vi.fn() },
    RUNTIME_PATH_OVERRIDE_NAME: "runtime_path_override",
    coreModule: Promise.resolve({ type: "success", result: {} }),
    coreProcessHandle: {},
    checkRelocation: check,
    stopCoreForRelocation: stop,
    ShutdownRefusedError,
    win: null,
    console,
    dialog: { showMessageBox: notice }
  }
  const move = callback(globals)
  window.getRuntimePath = async () => source
  window.openFolder = vi.fn()
  window.changeRuntimePath = (target, switchOnly) =>
    move(null, target, switchOnly)
  // The next start asks main, which reads the pointer the move left.
  window.takeRuntimeMoveNotice = () =>
    callback(
      {
        takeRuntimeMoveNotice,
        app: { getPath: () => root },
        CURRENT_RUNTIME_PATH: selectRuntimePath(null, root)
      },
      "takeRuntimeMoveNotice"
    )()
  window.dismissRuntimeMoveNotice = () =>
    callback(
      { dismissRuntimeMoveNotice, app: { getPath: () => root } },
      "dismissRuntimeMoveNotice"
    )()
  window.relaunch = callback(
    {
      withRuntimeOperation,
      ownedShutdown: async () => {
        const selected = selectRuntimePath(null, root)
        await restart(selected)
        return "stopped"
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
    check,
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
  // What the move copied is removed again, marker included.
  expect(await fs.readdir(f.target)).toEqual([])
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
// Windows refuses to delete a file another program holds open.
function keepOpen(file: string) {
  const unlink = fs.unlink.bind(fs)
  vi.spyOn(fs, "unlink").mockImplementation(async (name) => {
    if (String(name) === file)
      throw Object.assign(Error("EBUSY: resource busy or locked"), {
        code: "EBUSY"
      })
    return unlink(name)
  })
}
it("a move that empties the old folder restarts into the NEW folder at once, with no notice", async () => {
  const f = await fixture()
  await submit(f)
  expect(f.restart).toHaveBeenCalledTimes(1)
  expect(f.notice).not.toHaveBeenCalled()
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
  expect(await fs.readdir(f.source)).toEqual([])
  expect(document.body.textContent).not.toContain(
    englishJava["_trn_runtime_recovery.moved_title"]
  )
})
// A start from the new folder, as app.tsx runs it.
async function nextStart() {
  await showRuntimeMoveNotice((notice) =>
    c.manager.openModal({ name: "runtimeMoveNotice" }, notice)
  )
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain(
      englishJava["_trn_runtime_recovery.open_old_folder"]
    )
  )
}
it("files left in the old folder do not hold up the restart, and the first start from the NEW folder names them", async () => {
  const f = await fixture()
  keepOpen(path.join(f.source, "b"))
  await submit(f)
  // Main restarts on its own: nothing waits on this window.
  expect(f.restart).toHaveBeenCalledTimes(1)
  expect(f.notice).not.toHaveBeenCalled()
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
  expect(await fs.readdir(f.source)).toEqual(["b"])
  expect(document.body.textContent).not.toContain("java:_trn_migration_errored")
  expect(c.manager.hasOpenModals()).toBe(false)

  await nextStart()
  const text = document.body.textContent
  expect(text).toContain("java:_trn_runtime_recovery.moved_title")
  expect(text).toContain(
    englishJava["_trn_runtime_recovery.left_behind"]
      .replace("{{newPath}}", f.target)
      .replace("{{oldPath}}", f.source)
  )
  // Identical to its copy, so it is only listed as safe to delete.
  expect(text).toContain(englishJava["_trn_runtime_recovery.left_not_deleted"])
  expect(text).not.toContain(englishJava["_trn_runtime_recovery.left_changed"])
  expect(
    Array.from(document.querySelectorAll("li")).map((li) => li.textContent)
  ).toEqual(["b"])
  button(document.body, "Open old folder").click()
  expect(window.openFolder).toHaveBeenCalledWith(f.source)
  button(document.body, "general:_trn_ok").click()
  expect(c.manager.hasOpenModals()).toBe(false)
  // Acknowledged, so the next start shows nothing.
  await vi.waitFor(() =>
    expect(fss.existsSync(path.join(f.root, RUNTIME_MOVE_NOTICE_NAME))).toBe(
      false
    )
  )
  const again = vi.fn()
  await showRuntimeMoveNotice(again)
  expect(again).not.toHaveBeenCalled()
})
it("the notice stays open through Escape and backdrop clicks, and until then every start shows it again", async () => {
  const f = await fixture()
  keepOpen(path.join(f.source, "b"))
  await submit(f)
  await nextStart()
  const notice = path.join(f.root, RUNTIME_MOVE_NOTICE_NAME)
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
  )
  document
    .querySelector(".animate-modalEnter")!
    .parentElement!.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true })
    )
  await settle()
  expect(c.manager.hasOpenModals()).toBe(true)
  expect(document.body.textContent).toContain(
    englishJava["_trn_runtime_recovery.left_not_deleted"]
  )
  // A reload before it was acknowledged shows it again.
  expect(fss.existsSync(notice)).toBe(true)
  const reload = vi.fn()
  await showRuntimeMoveNotice(reload)
  expect(reload).toHaveBeenCalledWith(
    expect.objectContaining({ notDeleted: ["b"] })
  )
  // The header's close button acknowledges it like OK.
  document.querySelector<HTMLElement>('[data-testid="modal-close"]')!.click()
  expect(c.manager.hasOpenModals()).toBe(false)
  await vi.waitFor(() => expect(fss.existsSync(notice)).toBe(false))
  const again = vi.fn()
  await showRuntimeMoveNotice(again)
  expect(again).not.toHaveBeenCalled()
})
it("a file that changed after its copy is listed apart, as possibly newer than the copy", async () => {
  // Another program saves "a" after its copy was made, before the old one is deleted.
  const f = await fixture((options) =>
    relocateRuntime({
      ...options,
      progress: (p) => {
        if (p.action === "remove" && p.currentName === "a")
          fss.writeFileSync(path.join(options.source, "a"), "newer bytes")
      }
    })
  )
  keepOpen(path.join(f.source, "b"))
  await submit(f)
  expect(f.restart).toHaveBeenCalledTimes(1)
  await nextStart()
  const text = document.body.textContent
  expect(text).toContain(englishJava["_trn_runtime_recovery.left_changed"])
  expect(text).toContain(englishJava["_trn_runtime_recovery.left_not_deleted"])
  expect(
    Array.from(document.querySelectorAll("li")).map((li) => li.textContent)
  ).toEqual(["a", "b"])
  expect(await fs.readFile(path.join(f.source, "a"), "utf8")).toBe(
    "newer bytes"
  )
})
// Windows skips directory syncs in the relocation code, so this fault can't fire there.
it.skipIf(process.platform === "win32")(
  "an unconfirmed new location is named after the restart, with the old folder kept whole",
  async () => {
    const f = await fixture()
    const open = fs.open.bind(fs)
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      const handle = await (open as any)(file, ...args)
      if (file === f.root)
        handle.sync = async () => {
          throw Error("directory sync")
        }
      return handle
    }) as any)
    await submit(f)
    expect(f.restart).toHaveBeenCalledTimes(1)
    expect((await fs.readdir(f.source)).sort()).toEqual(["a", "b"])
    vi.restoreAllMocks()
    await nextStart()
    expect(document.body.textContent).toContain(
      englishJava["_trn_runtime_recovery.kept_old_folder"]
        .replace("{{newPath}}", f.target)
        .replace("{{oldPath}}", f.source)
    )
    expect(document.querySelectorAll("li")).toHaveLength(0)
  }
)
it("the progress names nested files with plain slashes", async () => {
  await fixture()
  setIsChangingRuntimePath(true)
  const file = ["instances", "All the Mods 9", "mods", "jei.jar"].join("/")
  RTsetProgress({ action: "copy", currentName: file, current: 1, total: 4 })
  await settle()
  expect(document.body.textContent).toContain(`Copying ${file}`)
  // The finished copy is written to the drive before anything is removed.
  RTsetProgress({ action: "flush", currentName: "", current: 2, total: 4 })
  await settle()
  expect(document.body.textContent).toContain(englishJava._trn_saving_copy)
  expect(document.body.textContent).toContain("2 / 4")
  RTsetProgress({ action: "remove", currentName: file, current: 3, total: 4 })
  await settle()
  expect(document.body.textContent).toContain(`Removing ${file}`)
  expect(document.body.textContent).not.toContain("&#x2F;")
  RTsetProgress(undefined)
})
it("a move that stopped says it removes the unfinished copy, file by file", async () => {
  await fixture()
  setIsChangingRuntimePath(true)
  const file = ["instances", "pack", "mods", "jei.jar"].join("/")
  RTsetProgress({ action: "rollback", currentName: file, current: 2, total: 5 })
  await settle()
  const text = document.body.textContent
  expect(text).toContain(
    englishJava._trn_removing_partial_copy.replace("{{file}}", file)
  )
  expect(text).toContain("2 / 5")
  // The overlay still asks not to close the app until the copy is gone.
  expect(text).toContain("java:_trn_do_not_close_app_or_pc_title")
  RTsetProgress(undefined)
})
it("a start that could not open the data folder says which one and offers a restart", async () => {
  const f = await fixture()
  const folder = path.join(f.root, "usb", "gdl"),
    current = path.join(f.root, "data")
  window.getUnavailableRuntimePath = async () => ({ folder, current })
  const relaunch = vi.fn(async () => ({ status: "relaunching" as const }))
  window.relaunch = relaunch
  await showUnavailableRuntimePath((value) =>
    c.manager.openModal({ name: "runtimePathUnavailable" }, value)
  )
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain(
      englishJava["_trn_runtime_recovery.unavailable"]
        .replaceAll("{{path}}", folder)
        .replace("{{defaultPath}}", current)
    )
  )
  button(document.body, "Restart").click()
  await settle()
  expect(relaunch).toHaveBeenCalledTimes(1)
  // Nothing to say when the data folder opened.
  window.getUnavailableRuntimePath = async () => null
  const open = vi.fn()
  await showUnavailableRuntimePath(open)
  expect(open).not.toHaveBeenCalled()
})
it("a start whose data folder is gone for good can keep the default folder, once told what that means", async () => {
  const f = await fixture()
  // The drive is gone: the folder the pointer names cannot be made.
  const drive = path.join(f.root, "usb")
  await fs.writeFile(drive, "")
  const folder = path.join(drive, "gdl")
  await fs.writeFile(f.pointer, folder)
  const current = path.join(f.root, "data")
  const unavailable = vi.fn()
  expect(selectRuntimePath(null, f.root, unavailable)).toBe(current)
  window.getUnavailableRuntimePath = async () => ({ folder, current })
  window.keepDefaultRuntimePath = callback(
    {
      withRuntimeOperation,
      forgetUnavailableRuntimePath,
      path,
      app: { getPath: () => f.root },
      RUNTIME_PATH_OVERRIDE_NAME: "runtime_path_override",
      UNAVAILABLE_RUNTIME_PATH: folder
    },
    "keepDefaultRuntimePath"
  )
  await showUnavailableRuntimePath((value) =>
    c.manager.openModal({ name: "runtimePathUnavailable" }, value)
  )
  const keep = englishJava["_trn_runtime_recovery.keep_default"]
  const gone = englishJava["_trn_runtime_recovery.unavailable_gone"]
  await vi.waitFor(() => expect(document.body.textContent).toContain(gone))
  // The notice names the way out, and choosing it first says what it does.
  button(document.body, keep).click()
  await settle()
  const confirm = englishJava["_trn_runtime_recovery.keep_default_confirm"]
    .replace("{{defaultPath}}", current)
    .replaceAll("{{path}}", folder)
  expect(document.body.textContent).toContain(confirm)
  expect(await fs.readFile(f.pointer, "utf8")).toBe(folder)
  button(document.body, "general:_trn_back").click()
  await settle()
  expect(document.body.textContent).toContain(gone)
  expect(document.body.textContent).not.toContain(confirm)
  // A choice that cannot be saved says why and stays open.
  button(document.body, keep).click()
  await settle()
  const unlink = fs.unlink.bind(fs)
  vi.spyOn(fs, "unlink").mockImplementationOnce(async () => {
    throw Object.assign(new Error("EACCES: permission denied"), {
      code: "EACCES"
    })
  })
  button(document.body, keep).click()
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain(
      englishJava["_trn_runtime_recovery.keep_default_failed"].replace(
        "{{error}}",
        "Error: EACCES: permission denied"
      )
    )
  )
  expect(await fs.readFile(f.pointer, "utf8")).toBe(folder)
  vi.mocked(fs.unlink).mockImplementation(unlink)
  button(document.body, keep).click()
  await vi.waitFor(() =>
    expect(document.body.textContent).not.toContain(confirm)
  )
  // Later starts use the default folder and say nothing; the drive's data
  // was never touched.
  expect(fss.existsSync(f.pointer)).toBe(false)
  expect(selectRuntimePath(null, f.root, unavailable)).toBe(current)
  expect(unavailable).toHaveBeenCalledTimes(1)
  expect(await fs.readFile(drive, "utf8")).toBe("")
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
  const f = await fixture(),
    copy = fs.copyFile.bind(fs)
  let copies = 0
  vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
    if (++copies === 2) throw Error("second copy failed")
    return (copy as any)(...args)
  }) as any)
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

it("a copy that stops early says what it removed and that nothing was lost", async () => {
  const f = await fixture(),
    copy = fs.copyFile.bind(fs)
  let copies = 0
  vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
    if (++copies === 2) throw Error("ENOSPC: no space left on device")
    return (copy as any)(...args)
  }) as any)
  await submit(f)
  // The space the copy took comes back at once.
  expect(await fs.readdir(f.target)).toEqual([])
  const text = document.body.textContent
  expect(text).toContain("java:_trn_migration_stopped")
  expect(text).not.toContain("java:_trn_migration_errored")
  expect(text).toContain(
    englishJava["_trn_runtime_recovery.copy_removed"].replace(
      "{{path}}",
      f.target
    )
  )
  expect(text).not.toContain("unfinished move")
  expect(text).not.toContain(englishJava["_trn_runtime_recovery.keep_folders"])
  expect(text).toContain("Error: ENOSPC: no space left on device")
  expect(c.protection()).toBe(true)
  button(document.body, "Restart").click()
  await settle()
  expect(f.restart).toHaveBeenCalledWith(f.source)
})
it("a copy that stops early and cannot be removed names the unfinished folder and the way out", async () => {
  const f = await fixture(),
    copy = fs.copyFile.bind(fs)
  let copies = 0
  vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
    if (++copies === 2) throw Error("ENOSPC: no space left on device")
    return (copy as any)(...args)
  }) as any)
  keepOpen(path.join(f.target, "a"))
  await submit(f)
  expect(fss.existsSync(path.join(f.target, MIGRATION_MARKER))).toBe(true)
  const text = document.body.textContent
  // A calm headline: nothing was published or removed, so nothing is corrupted.
  expect(text).toContain("java:_trn_migration_stopped")
  expect(text).not.toContain("java:_trn_migration_errored")
  // The sentence names the folder to delete; the modal behind it is blurred.
  expect(text).toContain(
    englishJava["_trn_runtime_recovery.incomplete_target"].replace(
      "{{path}}",
      f.target
    )
  )
  expect(text).not.toContain(englishJava["_trn_runtime_recovery.keep_folders"])
  expect(text).toContain("Error: ENOSPC: no space left on device")
  expect(c.protection()).toBe(true)
  button(document.body, "Restart").click()
  await settle()
  expect(f.restart).toHaveBeenCalledWith(f.source)
})
it("a refusal before the core was asked to stop says calmly that nothing was moved", async () => {
  const f = await fixture()
  await fs.writeFile(path.join(f.target, "occupied"), "data")
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_recovery.not_moved_title"])
  expect(text).toContain(javaText["_trn_runtime_recovery.not_moved_body"])
  expect(text).not.toContain("java:_trn_migration_errored")
  expect(document.querySelector(".bg-red-900")).toBeNull()
  expect(hasButton("general:_trn_dismiss")).toBe(true)
})
it("links inside the data folder are named with the way out, and Try again moves once they are gone", async () => {
  const f = await fixture()
  const external = path.join(f.root, "external")
  await fs.mkdir(external)
  const link = path.join(f.source, "shared-saves")
  await fs.symlink(external, link, "junction")
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_recovery.not_moved_title"])
  expect(text).toContain(javaText["_trn_runtime_recovery.links_inside"])
  expect(text).toContain("shared-saves")
  expect(text).not.toContain("Unsupported runtime")
  expect(text).not.toContain("java:_trn_migration_errored")
  expect(f.stop).not.toHaveBeenCalled()
  expect(runtimeRecoveryRequired()).toBe(false)
  button(
    document.body,
    javaText["_trn_runtime_recovery.open_data_folder"]
  ).click()
  expect(window.openFolder).toHaveBeenCalledWith(f.source)
  await fs.rm(link, { recursive: true })
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it("a failure that left no partial copy keeps the generic recovery advice", async () => {
  const f = await fixture()
  f.stop.mockImplementation(async () => {
    await fs.writeFile(path.join(f.target, "foreign"), "foreign")
  })
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(englishJava["_trn_runtime_recovery.keep_folders"])
  expect(text).toContain("java:_trn_migration_errored")
  expect(text).not.toContain("java:_trn_migration_stopped")
  expect(text).not.toContain("unfinished move")
})
const busy = [
  { kind: "game", name: "game 3" },
  { kind: "server", name: "server 7 run" }
] as const
const javaText = englishJava as Record<string, string>
it("a move refused while Minecraft runs explains why, lists what is running and can be dismissed", async () => {
  const f = await fixture()
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [...busy])
  )
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body"])
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.game"])
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.server"])
  // Internal owner names mean nothing to the user.
  expect(text).not.toContain("game 3")
  expect(text).not.toContain("server 7 run")
  expect(text).not.toContain("Retry relocation")
  expect(selectRuntimePath(null, f.root)).toBe(f.source)
  expect(await fs.readdir(f.target)).toEqual([])
  expect(runtimeRecoveryRequired()).toBe(false)
  button(document.body, "general:_trn_dismiss").click()
  await settle()
  expect(c.manager.hasOpenModals()).toBe(false)
  expect(f.restart).not.toHaveBeenCalled()
})
it("Try again after closing the game finishes the move", async () => {
  const f = await fixture()
  f.stop.mockRejectedValueOnce(
    new ShutdownRefusedError("Core relocation refused", [...busy])
  )
  await submit(f)
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(f.stop).toHaveBeenCalledTimes(2)
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it("an unresolved move that the core then refuses becomes dismissible", async () => {
  const f = await fixture()
  f.stop
    .mockRejectedValueOnce(Error("core unreachable"))
    .mockRejectedValueOnce(
      new ShutdownRefusedError("Core relocation refused", [...busy])
    )
  await submit(f)
  expect(c.protection()).toBe(true)
  button(document.body, "Retry relocation").click()
  await vi.waitFor(() => expect(f.stop).toHaveBeenCalledTimes(2))
  await settle()
  expect(document.body.textContent).toContain(
    javaText["_trn_runtime_relocation_busy.title"]
  )
  // The core confirmed it holds no move, so the shared lock is lifted too:
  // the modal closes normally and the next one opens unlocked.
  expect(runtimeRecoveryRequired()).toBe(false)
  expect(c.protection()).toBe(false)
})
it("a stopped core still offers only Restart", async () => {
  const f = await fixture()
  const move = f.move
  window.changeRuntimePath = async (target, switchOnly) => {
    const result = await move(null, target, switchOnly)
    return { ...result, recovery: "restart" }
  }
  f.stop.mockRejectedValue(Error("Core exit was not confirmed"))
  await submit(f)
  expect(c.protection()).toBe(true)
  expect(document.body.textContent).toContain("Restart")
  expect(document.body.textContent).not.toContain("Retry relocation")
  expect(
    Array.from(document.querySelectorAll("button")).some((b) =>
      b.textContent?.includes("general:_trn_dismiss")
    )
  ).toBe(false)
})

const hasButton = (text: string) =>
  Array.from(document.querySelectorAll("button")).some((b) =>
    b.textContent?.includes(text)
  )
it("a failed startup check offers a restart instead of a Try again that can never work", async () => {
  const f = await fixture()
  const failure =
    "Cannot reconcile /data/instances/Pack: invalid digit found in string"
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [
      { kind: "startupCheckFailed", name: failure }
    ])
  )
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body_steps"])
  expect(text).not.toContain(javaText["_trn_runtime_relocation_busy.body"])
  expect(text).toContain(
    javaText["_trn_runtime_relocation_busy.startup_check_failed"]
  )
  // The only thing that names the file or folder to look at.
  expect(text).toContain(failure)
  expect(hasButton(javaText["_trn_runtime_relocation_busy.try_again"])).toBe(
    false
  )
  expect(hasButton("general:_trn_dismiss")).toBe(true)
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.restart"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  // A plain restart keeps the old folder: nothing was moved.
  expect(f.restart).toHaveBeenCalledWith(f.source)
})
it("a restart that fails keeps the calm view, says nothing was moved and can still be dismissed", async () => {
  const f = await fixture()
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [
      {
        kind: "startupCheckFailed",
        name: "Cannot reconcile /data/instances/Pack"
      }
    ])
  )
  const raw =
    "Error: Shutdown unresolved (request 3f2a); core ownership retained. Core shutdown refused: Runtime ownership reconciliation is pending or failed"
  f.restart.mockRejectedValueOnce(Error(raw))
  const logged = vi.spyOn(console, "error").mockImplementation(() => {})
  await submit(f)
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.restart"]
  ).click()
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain(
      javaText["_trn_runtime_relocation_busy.restart_failed"]
    )
  )
  await settle()
  const text = document.body.textContent
  // Nothing was moved: no red "State might be corrupted" screen and no
  // request ids or internal reasons, which go to the log instead.
  expect(text).not.toContain("java:_trn_migration_errored")
  expect(text).not.toContain("3f2a")
  expect(text).not.toContain("reconciliation is pending")
  expect(logged).toHaveBeenCalled()
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.title"])
  expect(text).toContain(
    javaText["_trn_runtime_relocation_busy.startup_check_failed"]
  )
  expect(runtimeRecoveryRequired()).toBe(false)
  expect(selectRuntimePath(null, f.root)).toBe(f.source)
  // Restart stays on offer, and Dismiss closes the modal.
  expect(hasButton(javaText["_trn_runtime_relocation_busy.restart"])).toBe(true)
  button(document.body, "general:_trn_dismiss").click()
  await settle()
  expect(c.manager.hasOpenModals()).toBe(false)
})
it("an unconfirmed task offers Try again, which succeeds once it settles, and no Restart", async () => {
  const f = await fixture()
  f.stop.mockRejectedValueOnce(
    new ShutdownRefusedError("Core relocation refused", [
      { kind: "unconfirmed", name: "temporary child forge installer" }
    ])
  )
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.unconfirmed"])
  expect(text).not.toContain("forge installer")
  // A restart would stop servers and freeze the core while it is still
  // unconfirmed, so it is not offered from here.
  expect(hasButton(javaText["_trn_runtime_relocation_busy.restart"])).toBe(
    false
  )
  // The installer's child is reaped a moment later; Try again then moves.
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it("a running Pre-Launch or Post-Exit Hook is named as one, without its internal name, and Try again moves once it ends", async () => {
  const f = await fixture()
  f.stop.mockRejectedValueOnce(
    new ShutdownRefusedError("Core relocation refused", [
      { kind: "hook", name: "temporary child" }
    ])
  )
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body"])
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.hook"])
  expect(text).not.toContain(javaText["_trn_runtime_relocation_busy.task"])
  expect(text).not.toContain("temporary child")
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it("a refusal without a blocker list stays calm and offers Try again", async () => {
  const f = await fixture()
  f.stop.mockRejectedValueOnce(
    new ShutdownRefusedError("The operation was aborted due to timeout")
  )
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.title"])
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.not_started"])
  expect(text).not.toContain("java:_trn_migration_errored")
  expect(text).not.toContain("aborted due to timeout")
  expect(runtimeRecoveryRequired()).toBe(false)
  expect(hasButton("general:_trn_dismiss")).toBe(true)
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it("a process from an earlier session shows the PID to look for, with the program that holds it", async () => {
  const f = await fixture()
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [
      {
        kind: "previousProcess",
        name: "/data/instances/All the Mods 9",
        pid: 12345,
        program: "javaw.exe"
      },
      // A game published without a folder is named by its PID alone.
      { kind: "previousGame", name: "", pid: 678, program: "java" },
      // The core could not read which program holds it.
      { kind: "previousProcess", name: "/data/servers/Old", pid: 910 }
    ])
  )
  await submit(f)
  const text = document.body.textContent
  const pids = Array.from(
    document.querySelectorAll("[data-testid=relocation-pid]")
  ).map((line) => line.textContent)
  expect(text).toContain("/data/instances/All the Mods 9")
  expect(pids).toEqual([
    "Process ID (PID): 12345 (javaw.exe)",
    "Process ID (PID): 678 (java)",
    "Process ID (PID): 910"
  ])
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.previous_game"])
  expect(text).not.toContain("undefined")
})
it("never tells the user to end a PID that may belong to a program other than Java", async () => {
  const f = await fixture()
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [
      {
        kind: "previousProcess",
        name: "/data/__gdl_runtime_writers/3f1c2b7e.json",
        pid: 9876,
        program: "Discord & Co.exe"
      }
    ])
  )
  await submit(f)
  const row = document.querySelector("li")!.textContent
  // The program is shown as the process table names it, unescaped.
  expect(row).toContain("Process ID (PID): 9876 (Discord & Co.exe)")
  const label = javaText["_trn_runtime_relocation_busy.previous_process"]
  expect(row).toContain(label)
  // Minecraft is closed normally, so it saves. Ending is advised only for
  // Java that is not the user's own server or program; anything else gets
  // a restart instead.
  expect(label).not.toContain("End the process")
  expect(label).toMatch(/If it is Minecraft, close the game normally/)
  expect(label).toMatch(
    /If the process with the PID shown below is Java \(java, java\.exe or javaw\.exe\) and you are not running a Minecraft server or another Java program yourself, end it/
  )
  // Linux has its own tool.
  expect(label).toMatch(
    /Task Manager, in Activity Monitor or in System Monitor/
  )
  expect(label).toMatch(
    /Otherwise do not end anything: restart your computer instead/
  )
})
it("a server left by an earlier session points to its page, which can stop it or clear its record", async () => {
  const f = await fixture()
  const windowsServer =
    "C:\\Users\\Ann\\gdlauncher_carbon\\data\\servers\\Lobby"
  const marker = "/data/servers/Survival/.gdl_server.pid"
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [
      { kind: "previousProcess", name: "/data/servers/Creative", pid: 4242 },
      { kind: "previousProcess", name: windowsServer, pid: 4343 },
      { kind: "previousProcess", name: "/data/instances/Old", pid: 42 },
      { kind: "unreadableRecord", name: marker },
      {
        kind: "unreadableRecord",
        name: "/data/instances/Pack/.gdl_instance.pid"
      }
    ])
  )
  await submit(f)
  const rows = Array.from(document.querySelectorAll("li"))
  const row = (path: string) =>
    rows.find((li) => li.textContent?.includes(path))!.textContent
  const processHint = javaText["_trn_runtime_relocation_busy.previous_server"]
  const markerHint =
    javaText["_trn_runtime_relocation_busy.unreadable_server_marker"]

  expect(row("/data/servers/Creative")).toContain(processHint)
  expect(row(windowsServer)).toContain(processHint)
  expect(row("/data/instances/Old")).not.toContain(processHint)
  expect(row(marker)).toContain(markerHint)
  expect(row("/data/instances/Pack/.gdl_instance.pid")).not.toContain(
    markerHint
  )
  // The way out it already gave stays.
  expect(row("/data/servers/Creative")).toContain(
    javaText["_trn_runtime_relocation_busy.previous_process"]
  )
})
it("a record the launcher cannot read names its file and Try again moves once it is deleted", async () => {
  const f = await fixture()
  const record = "/data/__gdl_runtime_writers/3f1c2b7e.json"
  const marker = "/data/instances/Pack/.gdl_instance.pid"
  f.stop.mockRejectedValueOnce(
    new ShutdownRefusedError("Core relocation refused", [
      { kind: "unreadableRecord", name: record },
      { kind: "unreadableRecord", name: marker }
    ])
  )
  await submit(f)
  const text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body_steps"])
  expect(text).not.toContain(javaText["_trn_runtime_relocation_busy.body"])
  expect(
    text.split(javaText["_trn_runtime_relocation_busy.unreadable_record"])
  ).toHaveLength(3)
  // The file to delete is the only thing the user can act on.
  expect(text).toContain(record)
  expect(text).toContain(marker)
  // Restarting would not clear it; deleting the file and trying again does.
  expect(hasButton(javaText["_trn_runtime_relocation_busy.restart"])).toBe(
    false
  )
  expect(hasButton("general:_trn_dismiss")).toBe(true)
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.restart).toHaveBeenCalledTimes(1))
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
})
it.each([
  ["Linux x86_64", true],
  ["MacIntel", true],
  ["Win32", false]
])(
  "on %s each unreadable file can be shown in the file manager and copied, and a crashed instance marker offers a way out without one",
  async (platform, hidesDotFiles) => {
    vi.spyOn(navigator, "platform", "get").mockReturnValue(platform)
    const f = await fixture()
    // After the fixture, which installs its own.
    const openFolder = vi.fn()
    window.openFolder = openFolder
    const record = "/data/__gdl_runtime_writers/3f1c2b7e.json"
    const marker = "/data/instances/Pack/.gdl_instance.pid"
    const server = "/data/servers/Lobby/.gdl_server.pid"
    f.stop.mockRejectedValueOnce(
      new ShutdownRefusedError("Core relocation refused", [
        { kind: "unreadableRecord", name: record },
        { kind: "unreadableRecord", name: marker },
        { kind: "unreadableRecord", name: server },
        { kind: "previousProcess", name: "/data/instances/Old", pid: 42 }
      ])
    )
    await submit(f)
    const rows = Array.from(document.querySelectorAll("li"))
    const row = (path: string) =>
      rows.find((li) => li.textContent?.includes(path))!
    const show = javaText["_trn_runtime_relocation_busy.show_file"]
    const launchHint =
      javaText["_trn_runtime_relocation_busy.unreadable_instance_marker"]
    const hiddenHint = javaText["_trn_runtime_relocation_busy.hidden_file"]

    // Only the instance marker is replaced by a launch.
    expect(row(marker).textContent).toContain(launchHint)
    expect(row(server).textContent).not.toContain(launchHint)
    expect(row(record).textContent).not.toContain(launchHint)
    // Dot files are hidden in Finder and Linux file managers, not Explorer.
    expect(row(marker).textContent?.includes(hiddenHint)).toBe(hidesDotFiles)
    expect(row(server).textContent?.includes(hiddenHint)).toBe(hidesDotFiles)
    expect(row(record).textContent).not.toContain(hiddenHint)
    // A process row has no file to show.
    expect(row("/data/instances/Old").textContent).not.toContain(show)

    for (const path of [record, marker, server]) {
      button(row(path), show).click()
      expect(openFolder).toHaveBeenLastCalledWith(path)
      const detail = Array.from(row(path).querySelectorAll("div")).find(
        (div) => div.textContent === path
      )!
      expect(detail.style.userSelect).toBe("text")
    }
    expect(openFolder).toHaveBeenCalledTimes(3)
    // Showing a file changes nothing about the refusal.
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect(document.body.textContent).toContain(
      javaText["_trn_runtime_relocation_busy.title"]
    )
  }
)
it("a mixed list keeps the step for each row, and a pending startup check alone is told to wait", async () => {
  const f = await fixture()
  f.stop
    .mockRejectedValueOnce(
      new ShutdownRefusedError("Core relocation refused", [
        { kind: "game", name: "game 3" },
        { kind: "previousProcess", name: "/data/servers/Old", pid: 42 }
      ])
    )
    .mockRejectedValueOnce(
      new ShutdownRefusedError("Core relocation refused", [
        { kind: "startupCheckPending", name: "" }
      ])
    )
  await submit(f)
  let text = document.body.textContent
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body_steps"])
  // The general "close Minecraft" body is gone, so the row says it.
  expect(javaText["_trn_runtime_relocation_busy.game"]).toMatch(/Close it/)
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.game"])
  button(
    document.body,
    javaText["_trn_runtime_relocation_busy.try_again"]
  ).click()
  await vi.waitFor(() => expect(f.stop).toHaveBeenCalledTimes(2))
  await settle()
  text = document.body.textContent
  // Nothing runs: no "close Minecraft, stop your servers" body.
  expect(text).not.toContain(javaText["_trn_runtime_relocation_busy.body"])
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body_steps"])
  expect(text).toContain(
    javaText["_trn_runtime_relocation_busy.startup_check_pending"]
  )
})
it("collapses identical rows and shows only details the user can act on", async () => {
  const f = await fixture()
  f.stop.mockRejectedValue(
    new ShutdownRefusedError("Core relocation refused", [
      { kind: "server", name: "server start_server_impl" },
      { kind: "server", name: "server start_server_impl" },
      { kind: "install", name: "instance 5 setup producer" },
      { kind: "previousProcess", name: "/data/instances/Old" }
    ])
  )
  await submit(f)
  const text = document.body.textContent
  const serverLabel = javaText["_trn_runtime_relocation_busy.server"]
  expect(text.split(serverLabel)).toHaveLength(2)
  expect(text).not.toContain("start_server_impl")
  expect(text).not.toContain("setup producer")
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.install"])
  expect(text).toContain(
    javaText["_trn_runtime_relocation_busy.previous_process"]
  )
  expect(text).toContain("/data/instances/Old")
  // Closing that process clears it, so retrying is offered; a restart is not.
  expect(text).toContain(javaText["_trn_runtime_relocation_busy.body_steps"])
  expect(hasButton(javaText["_trn_runtime_relocation_busy.try_again"])).toBe(
    true
  )
  expect(hasButton(javaText["_trn_runtime_relocation_busy.restart"])).toBe(
    false
  )
})
it("does not warn against closing the app while it only checks before the move", async () => {
  await fixture()
  setIsChangingRuntimePath(true)
  await settle()
  expect(document.body.textContent).toContain("java:_trn_checking_runtime_path")
  RTsetProgress({ action: "check", currentName: "", current: 0, total: 0 })
  await settle()
  expect(document.body.textContent).not.toContain(
    "java:_trn_do_not_close_app_or_pc_title"
  )
  // Once the core is stopped the warning matters.
  RTsetProgress({ action: "scan", currentName: "", current: 0, total: 0 })
  await settle()
  expect(document.body.textContent).toContain(
    "java:_trn_do_not_close_app_or_pc_title"
  )
  expect(document.body.textContent).not.toContain(
    "java:_trn_checking_runtime_path"
  )
  RTsetProgress(undefined)
})
