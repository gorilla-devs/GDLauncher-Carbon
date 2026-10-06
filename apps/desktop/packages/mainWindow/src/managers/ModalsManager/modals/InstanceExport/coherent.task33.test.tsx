import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import InstanceExport, {
  exportStep,
  payload,
  setExportStep,
  setPayload
} from "."
import { failedMsg, setFailedMsg } from "./atoms/Exporting"
import { taskId, setTaskId } from "@/utils/import"
import { button, settle } from "@/task33.fixture"
const c = vi.hoisted(() => ({
  query: undefined as any,
  mutation: vi.fn(),
  keys: [] as any[]
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({ useTransContext: () => [(k: string) => k] }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: vi.fn() })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("./atoms/ExportVersion", () => ({ default: () => null }))
vi.mock("./atoms/FilesSelection", () => ({ default: () => null }))
vi.mock("./atoms/SelfContainedArchive", () => ({ default: () => null }))
vi.mock("./atoms/ExportCheckboxParent", () => ({
  checkedFiles: () => ["selected.jar"],
  buildNestedObject: () => ({ "selected.jar": null })
}))
vi.mock("./atoms/CheckIcon", () => ({
  default: () => <span data-testid="success">success icon</span>
}))
vi.mock("@/utils/import", () => {
  const [taskId, setTaskId] = createSignal<number>()
  return { taskId, setTaskId }
})
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (f: any) => {
      c.keys.push(f)
      return c.query
    },
    createMutation: (f: any) => ({
      mutate: (args: any) => {
        c.mutation(JSON.parse(JSON.stringify(args)))
        f().onSuccess(99)
      }
    })
  }
}))
let dispose: undefined | (() => void), publish: any
beforeEach(() => {
  vi.clearAllMocks()
  c.keys = []
  setExportStep(0)
  setTaskId(undefined)
  setFailedMsg(undefined)
  setPayload({
    instance_id: 7,
    target: "Curseforge",
    save_path: undefined,
    version: "2.3",
    self_contained_addons_bundling: true,
    filter: { entries: {} }
  })
  const [state, set] = createSignal<any>({ data: undefined, isError: false })
  publish = (data: any, isError = false) => set({ data, isError })
  c.query = {
    get data() {
      return state().data
    },
    get isError() {
      return state().isError
    }
  }
  window.showSaveDialog = vi.fn(async () => ({ canceled: true, filePath: "" }))
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  setTaskId(undefined)
  setExportStep(0)
})
function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <InstanceExport title="Synthetic" data={{ instanceId: 7 }} />,
    root
  )
  return root
}
function type(root: HTMLElement, value: string) {
  const input = root.querySelector("input")!
  input.value = value
  input.dispatchEvent(new Event("input", { bubbles: true }))
  input.dispatchEvent(new Event("blur", { bubbles: true }))
  return input
}
function format(root: HTMLElement, index: number) {
  root.querySelectorAll("li")[index].click()
}
it("0169 choosing another actual format keeps displayed destination and submitted pair coherent", async () => {
  const root = mount()
  const input = type(root, "/dotted.zip.dir/My.pack.zip")
  format(root, 2)
  await settle()
  expect(payload.save_path).toBe("/dotted.zip.dir/My.pack.mrpack")
  expect(input.value).toBe(payload.save_path)
  button(root, "begin_export").click()
  expect(c.mutation).toHaveBeenCalledWith(
    expect.objectContaining({
      target: "Modrinth",
      save_path: "/dotted.zip.dir/My.pack.mrpack",
      instance_id: 7,
      version: "2.3",
      self_contained_addons_bundling: true,
      filter: { entries: { "selected.jar": null } }
    })
  )
})
it.each([
  ["Curseforge", ".zip", 1],
  ["Gdlauncher", ".gdlpack", 0],
  ["Modrinth", ".mrpack", 2]
] as const)(
  "0169 repeated format %s replaces only recognized terminal archive suffix",
  async (target, suffix, index) => {
    const root = mount()
    type(root, "/dotted.mrpack.dir/Unrelated.name.gdlpack")
    format(root, index)
    format(root, index)
    await settle()
    expect(payload.save_path).toBe(`/dotted.mrpack.dir/Unrelated.name${suffix}`)
    expect(payload.target).toBe(target)
  }
)
it("0169 picker normalizes terminal suffix, cancel preserves draft, clearing refuses export", async () => {
  const root = mount()
  const field = type(root, "/dir/stem")
  expect(payload.save_path).toBe("/dir/stem.zip")
  window.showSaveDialog = vi.fn(async () => ({
    canceled: false,
    filePath: "/dir/Chosen.mrpack"
  }))
  root
    .querySelector<HTMLElement>(".i-material-symbols\\:folder-open-outline")!
    .click()
  await settle()
  expect(field.value).toBe("/dir/Chosen.zip")
  expect(payload.save_path).toBe(field.value)
  window.showSaveDialog = vi.fn(async () => ({ canceled: true, filePath: "" }))
  root
    .querySelector<HTMLElement>(".i-material-symbols\\:folder-open-outline")!
    .click()
  await settle()
  expect(payload.save_path).toBe("/dir/Chosen.zip")
  root.querySelector<HTMLElement>(".i-material-symbols\\:close")!.click()
  expect(payload.save_path).toBe("")
  expect(button(root, "begin_export").disabled).toBe(true)
})
it.each(["Unknown", "Modrinth"])(
  "0169 invalid target/path pair %s is refused visibly and by forced handler",
  (target) => {
    setPayload({ target: target as any, save_path: "/dir/wrong.zip" })
    const root = mount()
    const b = button(root, "begin_export")
    expect(b.disabled).toBe(true)
    void (b as any).invoke()
    expect(c.mutation).not.toHaveBeenCalled()
    expect(exportStep()).toBe(0)
  }
)
it("0169 unrelated extension and current lower-case matching archive remain exact", () => {
  const root = mount()
  type(root, "/dir/file.notes")
  expect(payload.save_path).toBe("/dir/file.notes.zip")
  type(root, "/dir/file.zip")
  button(root, "begin_export").click()
  expect(c.mutation).toHaveBeenCalledWith(
    expect.objectContaining({
      save_path: "/dir/file.zip",
      target: "Curseforge"
    })
  )
})
it.each([
  undefined,
  { type: "Known", value: 0 },
  { type: "Known", value: 0.004 },
  { type: "Known", value: 0.19 },
  { type: "Indeterminate" }
])(
  "0147 actual composed positive completion independent of sampled %j",
  async (progress) => {
    const root = mount()
    type(root, "/dir/export")
    button(root, "begin_export").click()
    expect(taskId()).toBe(99)
    expect(exportStep()).toBe(1)
    if (progress) publish({ type: "Running", value: { progress } })
    publish({ type: "Succeeded" })
    await settle()
    expect(exportStep()).toBe(2)
    expect(taskId()).toBeUndefined()
    expect(root.querySelector('[data-testid="success"]')).not.toBeNull()
    expect(root.textContent).toContain("open_folder")
    expect(failedMsg()).toBeUndefined()
    expect(c.keys[0]().queryKey).toEqual(["vtask.getTaskOutcome", 99])
  }
)
it.each([undefined, null])(
  "0147 absent task result %j remains pending including read error",
  (data) => {
    setExportStep(1)
    setTaskId(31)
    const root = mount()
    publish(data, true)
    expect(exportStep()).toBe(1)
    expect(taskId()).toBe(31)
    expect(root.querySelector('[data-testid="success"]')).toBeNull()
  }
)
it.each(["Failed", "Dismissed", "Unavailable"])(
  "0147 terminal %s stays distinct from success",
  (type) => {
    setExportStep(1)
    setTaskId(31)
    const root = mount()
    publish(
      type === "Failed"
        ? {
            type,
            value: {
              progress: {
                type: "Failed",
                value: { cause: [{ display: "synthetic owned failure" }] }
              }
            }
          }
        : { type }
    )
    expect(exportStep()).toBe(2)
    expect(failedMsg()?.trim()).toBeTruthy()
    expect(root.querySelector('[data-testid="success"]')).toBeNull()
    expect(root.textContent).not.toContain("open_folder")
  }
)
it("0147 old accepted task cannot complete replacement owner", () => {
  setExportStep(1)
  setTaskId(31)
  const root = mount()
  setTaskId(32)
  publish({ type: "Succeeded" })
  expect(exportStep()).toBe(1)
  expect(taskId()).toBe(32)
  expect(root.querySelector('[data-testid="success"]')).toBeNull()
})

it("0147 an unset accepted ID cannot authorize a cached positive outcome", () => {
  setExportStep(1)
  setTaskId(undefined)
  const root = mount()
  publish({ type: "Succeeded" })
  expect(exportStep()).toBe(1)
  expect(taskId()).toBeUndefined()
  expect(root.querySelector('[data-testid="success"]')).toBeNull()
})

it("0169 an editable path draft survives format publication before synthetic blur", () => {
  const root = mount()
  const field = type(root, "/dir/Old.zip")
  field.value = "/dir/Edited.gdlpack"
  field.dispatchEvent(new Event("input", { bubbles: true }))
  format(root, 2)
  expect(field.value).toBe("/dir/Edited.mrpack")
  expect(payload.save_path).toBe(field.value)
})
it("0169 direct submit sees current editable destination, never the prior committed path", () => {
  const root = mount()
  const field = type(root, "/dir/Old.zip")
  field.value = "/dir/New.zip"
  field.dispatchEvent(new Event("input", { bubbles: true }))
  void (button(root, "begin_export") as any).invoke()
  expect(c.mutation).toHaveBeenCalledWith(
    expect.objectContaining({ save_path: "/dir/New.zip" })
  )
})
