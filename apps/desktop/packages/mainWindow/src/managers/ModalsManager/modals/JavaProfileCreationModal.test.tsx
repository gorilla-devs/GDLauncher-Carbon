import { expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import Modal from "./JavaProfileCreationModal"

const control = vi.hoisted(() => ({ finish: () => {}, calls: 0, closed: 0 }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({ data: [] }),
    createMutation: (key: () => { mutationKey: string[] }) => {
      const [pending, setPending] = createSignal(false)
      return {
        get isPending() {
          return pending()
        },
        mutateAsync: async () => {
          expect(key().mutationKey).toEqual(["java.createJavaProfile"])
          control.calls++
          setPending(true)
          await new Promise<void>((resolve) => {
            control.finish = resolve
          })
          setPending(false)
        }
      }
    }
  }
}))
vi.mock("..", () => ({
  useModal: () => ({
    closeModal: () => {
      control.closed++
    }
  })
}))
vi.mock("../ModalLayout", () => ({
  default: (props: any) => <div>{props.children}</div>
}))
vi.mock("@gd/ui", () => ({
  Input: (props: any) => (
    <input
      disabled={props.disabled}
      value={props.value}
      onInput={props.onInput}
    />
  ),
  Button: (props: any) => (
    <button disabled={props.disabled} onClick={props.onClick}>
      {props.children}
    </button>
  ),
  toast: { success: vi.fn() }
}))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/components/JavaPathAutoComplete", () => ({
  default: (props: any) => (
    <input
      disabled={props.disabled}
      onInput={() =>
        props.onSelection({
          id: "jdk8",
          path: "/synthetic/jdk8",
          pending: false
        })
      }
    />
  )
}))

test("actual creation pending state disables profile controls until the mutation settles", async () => {
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(() => <Modal {...({} as any)} />, root)
  const inputs = root.querySelectorAll("input")
  inputs[0].value = "profile"
  inputs[0].dispatchEvent(new Event("input", { bubbles: true }))
  inputs[1].dispatchEvent(new Event("input", { bubbles: true }))
  const buttons = root.querySelectorAll("button")
  expect(buttons[1].disabled).toBe(false)
  buttons[1].click()
  expect(control.calls).toBe(1)
  for (const field of [...inputs, ...buttons]) expect(field.disabled).toBe(true)
  buttons[1].click()
  expect(control.calls).toBe(1)
  control.finish()
  await Promise.resolve()
  await Promise.resolve()
  for (const field of [...inputs, ...buttons])
    expect(field.disabled).toBe(false)
  expect(control.closed).toBe(1)
  dispose()
  root.remove()
})
