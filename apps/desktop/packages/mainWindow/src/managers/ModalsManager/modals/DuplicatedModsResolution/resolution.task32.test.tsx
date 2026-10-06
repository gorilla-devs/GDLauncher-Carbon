import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { Radio, button, deferred, settle } from "@/task32.fixture"
import Wizard from "."
const c = vi.hoisted(() => ({
  mutate: vi.fn(),
  close: vi.fn(),
  steps: [] as any[],
  radios: new Map<string, (v: string) => void>(),
  version: undefined as any,
  action: undefined as any
}))
vi.mock("@gd/ui", async () => {
  const f = await import("@/task32.fixture")
  return {
    ...f,
    Steps: (p: any) => {
      c.steps = p.steps
      return <f.Steps {...p} />
    },
    Radio: (p: any) => {
      c.radios.set(p.value, p.onChange)
      return <f.Radio {...p} />
    }
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div data-protected={p.preventClose}>{p.children}</div>
}))
vi.mock("@/utils/instances", () => ({ getModImageUrl: () => "" }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (f: any) => ({
      mutateAsync: (v: any) => c.mutate(f().mutationKey[0], v)
    })
  }
}))
let dispose: (() => void) | undefined
beforeEach(() => {
  vi.clearAllMocks()
  c.radios.clear()
  c.mutate.mockReset().mockResolvedValue(undefined)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount(ids = ["keep", "a", "b"]) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <Wizard
        title="Synthetic duplicate resolution"
        data={{
          instanceId: 7,
          duplicatedMods: [
            {
              groupKey: "actual-A",
              name: "Same",
              versions: ids.map((id) => ({
                id,
                fileName: `${id}.jar`,
                version: id,
                dateAdded: "",
                fileSize: "1"
              }))
            }
          ]
        }}
      />
    ),
    root
  )
  return root
}
function summary(root: HTMLElement, action = "disable") {
  c.steps[2].onClick()
  root
    .querySelector<HTMLInputElement>(`input[value="${action}"]`)!
    .dispatchEvent(new Event("change", { bubbles: true }))
  button(root, "action.button_continue").click()
}
it("0241 pending actual header and captured navigation callbacks cannot leave summary", async () => {
  const p = deferred()
  c.mutate.mockReturnValue(p.promise)
  const root = mount()
  summary(root)
  button(root, "summary.button_apply").click()
  c.steps[2].onClick()
  expect(root.querySelector('input[value="remove"]')).toBeNull()
  expect(root.querySelector('[aria-disabled="true"]')).not.toBeNull()
  p.resolve(undefined)
  await settle()
})
it("0241 Task31 frozen whole action/unwanted-list control survives late live edits", async () => {
  const p = deferred()
  c.mutate.mockReturnValueOnce(p.promise).mockResolvedValue(undefined)
  const root = mount()
  summary(root)
  button(root, "summary.button_apply").click()
  c.steps[2].onClick()
  const remove = root.querySelector<HTMLInputElement>('input[value="remove"]')
  remove?.dispatchEvent(new Event("change", { bubbles: true }))
  c.steps[1].onClick()
  const keep = root.querySelector<HTMLInputElement>('input[value="a"]')
  keep?.dispatchEvent(new Event("change", { bubbles: true }))
  p.resolve(undefined)
  await settle()
  expect(c.mutate.mock.calls.map(([k, v]) => [k, v.mod_id])).toEqual([
    ["instance.disableMod", "a"],
    ["instance.disableMod", "b"]
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})
it.each(["disable", "remove"])(
  "0101 %s acknowledgement journal retries only incomplete artifact with stable token",
  async (action) => {
    let first = true
    c.mutate.mockImplementation(async (_k, v) => {
      if (v.mod_id === "b" && first) {
        first = false
        throw Error("late rejection")
      }
    })
    const root = mount()
    summary(root, action)
    button(root, "summary.button_apply").click()
    await settle()
    expect(c.close).not.toHaveBeenCalled()
    button(root, "summary.button_apply").click()
    await settle()
    expect(c.mutate.mock.calls.map(([, v]) => v.mod_id)).toEqual([
      "a",
      "b",
      "b"
    ])
    const b = c.mutate.mock.calls.filter(([, v]) => v.mod_id === "b")
    expect(b[0][1].operation_token).toMatch(/^[0-9a-f-]{36}$/)
    expect(b[1][1].operation_token).toBe(b[0][1].operation_token)
    expect(c.close).toHaveBeenCalledTimes(1)
  }
)
it("0101 uncertain action stays owned across navigation and Retry keeps original action/token", async () => {
  c.mutate.mockRejectedValueOnce(Error("uncertain"))
  const root = mount()
  summary(root)
  button(root, "summary.button_apply").click()
  await settle()
  const previous = c.mutate.mock.calls[0][1]
  c.steps[2].onClick()
  const remove = root.querySelector<HTMLInputElement>('input[value="remove"]')!
  expect(remove.disabled).toBe(true)
  c.radios.get("remove")!("remove")
  expect(c.mutate).toHaveBeenCalledTimes(1)
  button(root, "action.button_continue").click()
  button(root, "summary.button_apply").click()
  await settle()
  expect(c.mutate.mock.calls[1][0]).toBe("instance.disableMod")
  expect(c.mutate.mock.calls[1][1].operation_token).toBe(
    previous.operation_token
  )
})
it("0241 Finish direct reentry stays single and retained member is never dispatched", async () => {
  const p = deferred()
  c.mutate.mockReturnValueOnce(p.promise).mockResolvedValue(undefined)
  const root = mount()
  summary(root)
  const apply = button(root, "summary.button_apply") as any
  apply.invoke()
  apply.invoke()
  expect(c.mutate).toHaveBeenCalledTimes(1)
  expect(apply.disabled).toBe(true)
  p.resolve(undefined)
  await settle()
  expect(c.mutate.mock.calls.map(([, v]) => v.mod_id)).toEqual(["a", "b"])
})
it("0241/0101 partially removed member cannot become retained and cause removal of surviving member", async () => {
  const files = new Set(["keep", "a", "b"])
  let fail = true
  c.mutate.mockImplementation(async (_key, v) => {
    if (v.mod_id === "b" && fail) {
      fail = false
      throw Error("uncertain B")
    }
    files.delete(v.mod_id)
  })
  const root = mount()
  summary(root, "remove")
  button(root, "summary.button_apply").click()
  await settle()
  expect(files.has("a")).toBe(false)
  expect(files.has("keep")).toBe(true)
  c.steps[1].onClick()
  const removed = root.querySelector<
    HTMLInputElement & { choose: (v: string) => void }
  >('input[value="a"]')!
  expect(removed.disabled).toBe(true)
  c.radios.get("a")!("a")
  button(root, "selection.button_continue").click()
  button(root, "action.button_continue").click()
  button(root, "summary.button_apply").click()
  await settle()
  expect(files.has("keep")).toBe(true)
  expect(c.mutate.mock.calls.map(([, v]) => v.mod_id)).toEqual(["a", "b", "b"])
})

it("0241/0101 untouched candidate stays eligible after partial remove and edit dispatches nothing", async () => {
  let fail = true
  c.mutate.mockImplementation(async (_key, v) => {
    if (v.mod_id === "b" && fail) {
      fail = false
      throw Error("uncertain B")
    }
  })
  const root = mount(["keep", "a", "b", "untouched"])
  summary(root, "remove")
  button(root, "summary.button_apply").click()
  await settle()
  c.steps[1].onClick()
  const next = root.querySelector<HTMLInputElement>('input[value="untouched"]')!
  expect(next.disabled).toBe(false)
  c.radios.get("untouched")!("untouched")
  expect(c.mutate).toHaveBeenCalledTimes(2)
  button(root, "selection.button_continue").click()
  button(root, "action.button_continue").click()
  button(root, "summary.button_apply").click()
  await settle()
  expect(c.mutate.mock.calls.map(([, v]) => v.mod_id)).toEqual([
    "a",
    "b",
    "keep",
    "b"
  ])
  expect(c.mutate.mock.calls[3][1].operation_token).toBe(
    c.mutate.mock.calls[1][1].operation_token
  )
  expect(c.close).toHaveBeenCalledTimes(1)
})
it("0241 normal pre-attempt action changes and retained choices remain supported", async () => {
  const root = mount()
  c.steps[1].onClick()
  c.radios.get("a")!("a")
  summary(root, "remove")
  expect(c.mutate).not.toHaveBeenCalled()
  button(root, "summary.button_apply").click()
  await settle()
  expect(c.mutate.mock.calls.map(([key, v]) => [key, v.mod_id])).toEqual([
    ["instance.deleteMod", "keep"],
    ["instance.deleteMod", "b"]
  ])
})

it("0241 actual disabled Radio blocks native/label/change/Space and button activation", () => {
  const root = document.createElement("div")
  document.body.append(root)
  const calls = vi.fn()
  dispose = render(
    () => (
      <>
        <Radio value="a" checked allowDeselect disabled onChange={calls}>
          A
        </Radio>
        <Radio
          value="b"
          checked={false}
          buttonStyle="button"
          disabled
          onChange={calls}
        >
          B
        </Radio>
      </>
    ),
    root
  )
  const input = root.querySelector("input")!
  expect(input.disabled).toBe(true)
  input.click()
  input.dispatchEvent(new Event("change", { bubbles: true }))
  input.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }))
  root.querySelector("label")!.click()
  expect(root.querySelector("button")!.disabled).toBe(true)
  root.querySelector("button")!.click()
  root.querySelectorAll("label")[1].click()
  expect(calls).not.toHaveBeenCalled()
})
