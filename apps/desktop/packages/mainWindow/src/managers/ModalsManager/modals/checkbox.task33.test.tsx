import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { Checkbox } from "../../../../../../../../packages/ui/src/Checkbox"
import ServerEulaAcceptance from "./ServerEulaAcceptance"
import {
  button,
  deferred,
  pointerGesture,
  settle,
  toast
} from "@/task33.fixture"
const c = vi.hoisted(() => ({
  accept: vi.fn(),
  start: vi.fn(),
  close: vi.fn(),
  order: [] as string[]
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (f: any) => ({
      mutateAsync: (id: any) => {
        c.order.push("accept")
        return c.accept(id)
      },
      mutate: (id: any) => {
        expect(f().mutationKey[0]).toBe("server.startServer")
        c.order.push("start")
        c.start(id)
      }
    })
  }
}))
let dispose: undefined | (() => void)
beforeEach(() => {
  vi.clearAllMocks()
  c.order = []
  c.accept.mockReset().mockResolvedValue(undefined)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
function gesture(root: HTMLElement) {
  const input = root.querySelector('input[type="checkbox"]')
  pointerGesture(
    (input?.closest("label") ??
      root.querySelector(".cursor-pointer.rounded-md"))!
  )
}
it("0110 shared control exposes labelled focusable reactive native checked disabled indeterminate semantics", () => {
  const [checked, setChecked] = createSignal(false),
    [disabled, setDisabled] = createSignal(false),
    [indeterminate, setIndeterminate] = createSignal(true)
  const change = vi.fn((v: boolean) => setChecked(v))
  const root = mount(() => (
    <Checkbox
      checked={checked()}
      disabled={disabled()}
      indeterminate={indeterminate()}
      onChange={change}
    >
      Synthetic consent
    </Checkbox>
  ))
  const input = root.querySelector<HTMLInputElement>('input[type="checkbox"]')
  expect(input).not.toBeNull()
  expect(input!.labels?.[0].textContent).toContain("Synthetic consent")
  expect(input!.disabled).toBe(false)
  expect(input!.indeterminate).toBe(true)
  input!.focus()
  expect(document.activeElement).toBe(input)
  input!.click()
  expect(change).toHaveBeenCalledExactlyOnceWith(true)
  expect(input!.checked).toBe(true)
  setIndeterminate(false)
  setDisabled(true)
  expect(input!.indeterminate).toBe(false)
  expect(input!.disabled).toBe(true)
  input!.click()
  expect(change).toHaveBeenCalledTimes(1)
})
it("0110 native Enter toggles once and repeat is inert; native Space remains default", () => {
  const change = vi.fn()
  const root = mount(() => <Checkbox onChange={change}>Synthetic</Checkbox>)
  const input = root.querySelector<HTMLInputElement>('input[type="checkbox"]')
  expect(input).not.toBeNull()
  input!.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Enter",
      bubbles: true,
      cancelable: true
    })
  )
  expect(change).toHaveBeenCalledExactlyOnceWith(true)
  input!.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Enter",
      repeat: true,
      bubbles: true,
      cancelable: true
    })
  )
  expect(change).toHaveBeenCalledTimes(1)
  const space = new KeyboardEvent("keydown", {
    key: " ",
    bubbles: true,
    cancelable: true
  })
  input!.dispatchEvent(space)
  expect(space.defaultPrevented).toBe(false)
})
it("0110 pointer gesture notifies one boolean update", () => {
  const change = vi.fn()
  const root = mount(() => <Checkbox onChange={change}>Synthetic</Checkbox>)
  gesture(root)
  expect(change).toHaveBeenCalledExactlyOnceWith(true)
})
it("0110 callback-less visual checkbox adds no ineffective tab stop", () => {
  const root = mount(() => <Checkbox checked={true} />)
  const input = root.querySelector<HTMLInputElement>('input[type="checkbox"]')
  expect(input === null || input.tabIndex === -1).toBe(true)
})
it("0110 actual EULA native control accepts before start and pending direct submit is inert", async () => {
  const p = deferred()
  c.accept.mockReturnValue(p.promise)
  const root = mount(() => (
    <ServerEulaAcceptance title="Synthetic" data={{ server_id: 7 }} />
  ))
  const b = button(root, "accept_and_start")
  expect(b.disabled).toBe(true)
  const input = root.querySelector<HTMLInputElement>('input[type="checkbox"]')
  expect(input).not.toBeNull()
  input!.click()
  expect(b.disabled).toBe(false)
  b.click()
  b.click()
  void (b as any).invoke()
  expect(c.accept).toHaveBeenCalledExactlyOnceWith(7)
  expect(input!.disabled).toBe(true)
  expect(c.start).not.toHaveBeenCalled()
  p.resolve(undefined)
  await settle()
  expect(c.order).toEqual(["accept", "start"])
  expect(c.start).toHaveBeenCalledExactlyOnceWith(7)
  expect(c.close).toHaveBeenCalledTimes(1)
})
it("0110 EULA current pointer consent and rejected accept remain usable without starting", async () => {
  c.accept.mockRejectedValue(new Error("synthetic denied"))
  const root = mount(() => (
    <ServerEulaAcceptance title="Synthetic" data={{ server_id: 7 }} />
  ))
  gesture(root)
  button(root, "accept_and_start").click()
  await settle()
  expect(c.start).not.toHaveBeenCalled()
  expect(toast.error).toHaveBeenCalledTimes(1)
  expect(button(root, "accept_and_start").disabled).toBe(false)
})
