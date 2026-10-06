import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { button, settle } from "@/task31.fixture"
import InstanceExport, { exportStep, setExportStep, setPayload } from "."
import { failedMsg, setFailedMsg } from "./atoms/Exporting"
import { taskId, setTaskId } from "@/utils/import"
const c = vi.hoisted(() => ({
  query: undefined as any,
  mutate: vi.fn(),
  keys: [] as any[]
}))
vi.mock("@gd/ui", async () => import("@/task31.fixture"))
vi.mock("@gd/i18n", () => ({ useTransContext: () => [(k: string) => k] }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: vi.fn() })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("./atoms/ExportFormat", () => ({ default: () => null }))
vi.mock("./atoms/ExportVersion", () => ({ default: () => null }))
vi.mock("./atoms/FilesSelection", () => ({ default: () => null }))
vi.mock("./atoms/SelfContainedArchive", () => ({ default: () => null }))
vi.mock("./atoms/ExportPath", () => ({ default: () => null }))
vi.mock("./atoms/ExportCheckboxParent", () => ({
  checkedFiles: () => [],
  buildNestedObject: () => ({})
}))
vi.mock("./atoms/CheckIcon", () => ({
  default: () => <span data-testid="check-icon">success icon</span>
}))
vi.mock("@/utils/import", () => {
  const [taskId, setTaskId] = createSignal<number | undefined>()
  return { taskId, setTaskId }
})
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (f: any) => {
      c.keys.push(f)
      return f().queryKey[1] === 11 ? c.query : { data: undefined }
    },
    createMutation: (f: any) => ({
      mutate: (v: any) => {
        c.mutate(v)
        f().onSuccess?.(33)
      }
    })
  }
}))
let dispose: (() => void) | undefined, setResult: any
beforeEach(() => {
  vi.clearAllMocks()
  c.keys = []
  setFailedMsg(undefined)
  setExportStep(1)
  setTaskId(11)
  setPayload({ instance_id: 7, save_path: "synthetic-export.zip" })
  const [result, set] = createSignal<any>()
  setResult = (value: any) => set(value)
  c.query = {
    get data() {
      return result()
    }
  }
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
    () => <InstanceExport title="Synthetic export" data={{ instanceId: 7 }} />,
    root
  )
  return root
}
function publish(progress: any, type = "Failed") {
  setResult(
    c.keys[0]().queryKey[0] === "vtask.getTaskOutcome"
      ? { type, value: { progress } }
      : { progress }
  )
}
it.each(
  [
    [{ display: "outer failure" }],
    [{ display: "outer failure" }, { display: "nested failure" }],
    [],
    [{ display: " " }, { display: "" }]
  ].map((cause) => ({ cause }))
)(
  "0106 failure causes %j compose failed completion without success/Open Folder and clear active task",
  ({ cause }) => {
    const root = mount()
    expect(() => publish({ type: "Failed", value: { cause } })).not.toThrow()
    expect(exportStep()).toBe(2)
    expect(taskId()).toBeUndefined()
    expect(failedMsg()?.trim()).toBeTruthy()
    expect(root.querySelector('[data-testid="check-icon"]')).toBeNull()
    expect(root.textContent).not.toContain("open_folder")
    expect(root.textContent).toContain(
      cause[1]?.display.trim() || cause[0]?.display.trim() || "export_failed"
    )
  }
)
it("0106 initial undefined remains pending, Known/Indeterminate progress is displayed", () => {
  const root = mount()
  expect(exportStep()).toBe(1)
  expect(taskId()).toBe(11)
  publish({ type: "Known", value: 0.42 }, "Running")
  expect(
    root.querySelector('[data-testid="progress"]')?.getAttribute("data-value")
  ).toBe("42")
  publish({ type: "Indeterminate" }, "Running")
  expect(root.textContent).toContain("preparing_export")
  expect(exportStep()).toBe(1)
})
it("0106 absence alone after progress cannot show success", () => {
  const root = mount()
  publish({ type: "Known", value: 0.42 }, "Running")
  setResult(null)
  expect(root.querySelector('[data-testid="check-icon"]')).toBeNull()
  expect(exportStep()).toBe(1)
  expect(taskId()).toBe(11)
})
it("0106 immediate positive Succeeded completes at zero progress and next actual BeginExport resets failure", async () => {
  const root = mount()
  setResult({ type: "Succeeded" })
  expect(exportStep()).toBe(2)
  expect(taskId()).toBeUndefined()
  expect(root.querySelector('[data-testid="check-icon"]')).not.toBeNull()
  expect(root.textContent).toContain("open_folder")
  setFailedMsg("previous failure")
  setExportStep(0)
  button(root, "begin_export").click()
  await settle()
  expect(failedMsg()).toBeUndefined()
  expect(taskId()).toBe(33)
  expect(exportStep()).toBe(1)
})
it("0106 stale active-task result cannot clear a replacement task", () => {
  mount()
  setTaskId(22)
  publish({ type: "Failed", value: { cause: [{ display: "old failure" }] } })
  expect(taskId()).toBe(22)
  expect(exportStep()).toBe(1)
  expect(failedMsg()).toBeUndefined()
})

it.each(["Unavailable", "Dismissed"])(
  "0106 %s positive-outcome contract completes as nonempty failure, never export success",
  (type) => {
    const root = mount()
    setResult({ type })
    expect(exportStep()).toBe(2)
    expect(taskId()).toBeUndefined()
    expect(failedMsg()).toContain("export_failed")
    expect(root.querySelector('[data-testid="check-icon"]')).toBeNull()
    expect(root.textContent).not.toContain("open_folder")
  }
)
