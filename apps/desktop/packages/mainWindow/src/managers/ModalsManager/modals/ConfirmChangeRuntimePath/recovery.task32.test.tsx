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
