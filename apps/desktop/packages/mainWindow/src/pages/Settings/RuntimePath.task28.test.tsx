import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import RuntimePath from "./RuntimePath"

const c = vi.hoisted(() => ({
  open: vi.fn(),
  apply: undefined as any,
  moving: false
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: c.open })
}))
vi.mock(
  "@/managers/ModalsManager/modals/ConfirmChangeRuntimePath/state",
  () => ({ isChangingRuntimePath: () => c.moving })
)
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => {
    if (p.type === "primary") c.apply = p
    return (
      <button disabled={p.disabled} onClick={p.onClick}>
        {p.children}
      </button>
    )
  },
  Input: (p: any) => (
    <div>
      <input value={p.value} onInput={p.onInput} />
      {p.icon}
    </div>
  ),
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div>{p.children}</div>
}))
let dispose: () => void
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.open.mockClear()
  c.moving = false
  vi.restoreAllMocks()
})
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
it("GPT-0261 admits only the settled current path in both presentation and direct handler", async () => {
  const pending = new Map<
    string,
    { resolve: (v: any) => void; reject: (v: any) => void }
  >()
  Object.assign(window, {
    getRuntimePath: async () => "C",
    getInitialRuntimePath: async () => "initial",
    validateRuntimePath: vi.fn(
      (path: string) =>
        new Promise((resolve, reject) => pending.set(path, { resolve, reject }))
    ),
    openFileDialog: async () => ({ canceled: false, filePaths: ["dialog"] })
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <RuntimePath />, root)
  await flush()
  const input = root.querySelector("input")!
  const select = (path: string) => {
    input.value = path
    input.dispatchEvent(new Event("input", { bubbles: true }))
  }
  select("A")
  pending.get("A")!.resolve("valid")
  await flush()
  expect(c.apply.disabled).toBe(false)
  select("B")
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  expect(c.apply.disabled).toBe(true)
  pending.get("B")!.resolve("potentially_valid")
  await flush()
  c.apply.onClick()
  expect(c.open).toHaveBeenLastCalledWith(
    { name: "ConfirmChangeRuntimePath" },
    { runtimePath: "B", isTargetFolderAlreadyUsed: true }
  )
  c.open.mockClear()
  select("failed")
  pending.get("failed")!.reject(new Error("synthetic validation"))
  await flush()
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  root.querySelector<HTMLElement>(".i-hugeicons\\:folder-01")!.click()
  await flush()
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  pending.get("dialog")!.resolve("valid")
  await flush()
  c.apply.onClick()
  expect(c.open).toHaveBeenLastCalledWith(expect.anything(), {
    runtimePath: "dialog",
    isTargetFolderAlreadyUsed: false
  })
  c.open.mockClear()
  select("C")
  pending.get("C")!.resolve("valid")
  await flush()
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
})
