import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import Modal from "./JavaProfileCreationModal"

const control = vi.hoisted(() => ({
  finish: () => {},
  fail: (_: Error) => {},
  inputs: [] as unknown[],
  profiles: [] as { name: string }[],
  closed: 0
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({
      get data() {
        return control.profiles
      }
    }),
    createMutation: (key: () => { mutationKey: string[] }) => {
      const [pending, setPending] = createSignal(false)
      return {
        get isPending() {
          return pending()
        },
        mutateAsync: async (input: unknown) => {
          expect(key().mutationKey).toEqual(["java.createJavaProfile"])
          control.inputs.push(input)
          setPending(true)
          try {
            await new Promise<void>((resolve, reject) => {
              control.finish = resolve
              control.fail = reject
            })
          } finally {
            setPending(false)
          }
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
// The real Button: a loading one is disabled, which is what stops a second
// Create.
vi.mock("@gd/ui", async () => import("@/test/uiFormsRealButton.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/pages/Settings/settings.java.data", () => ({
  createJavaOverviewQuery: () => ({
    data: undefined,
    isError: false,
    isFetching: false,
    refetch: vi.fn()
  })
}))
vi.mock("@/components/Java/JavaSelect", () => ({
  default: (props: any) => (
    <input disabled={props.disabled} onInput={() => props.onChange("jdk8")} />
  )
}))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  control.inputs = []
  control.profiles = []
  control.closed = 0
})
function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Modal {...({} as any)} />, root)
  const [name, java] = root.querySelectorAll("input")
  const [cancel, create] = root.querySelectorAll("button")
  return {
    name,
    java,
    cancel,
    create,
    type: (value: string) => {
      name.value = value
      name.dispatchEvent(new Event("input", { bubbles: true }))
    },
    pickJava: () => java.dispatchEvent(new Event("input", { bubbles: true }))
  }
}
const settle = async () => {
  for (let i = 0; i < 4; i++) await Promise.resolve()
}

test("every control stays disabled from Create until the profile is saved, and the dialog closes once", async () => {
  const dialog = mount()
  dialog.type("profile")
  dialog.pickJava()
  const controls = [dialog.name, dialog.java, dialog.cancel, dialog.create]
  expect(dialog.create.disabled).toBe(false)
  dialog.create.click()
  expect(control.inputs).toEqual([{ profileName: "profile", javaId: "jdk8" }])
  for (const field of controls) expect(field.disabled).toBe(true)
  dialog.create.click()
  expect(control.inputs).toHaveLength(1)
  expect(control.closed).toBe(0)
  control.finish()
  await settle()
  for (const field of controls) expect(field.disabled).toBe(false)
  expect(control.closed).toBe(1)
})

test("Create waits for a name and a Java, and refuses a name another profile has", () => {
  control.profiles = [{ name: "Existing" }]
  const dialog = mount()
  expect(dialog.create.disabled).toBe(true)
  dialog.type("New profile")
  expect(dialog.create.disabled).toBe(true)
  dialog.pickJava()
  expect(dialog.create.disabled).toBe(false)
  dialog.type("Existing")
  expect(dialog.create.disabled).toBe(true)
  dialog.create.click()
  expect(control.inputs).toEqual([])
})

test("a refused creation keeps the dialog open and Create ready for another try", async () => {
  const dialog = mount()
  dialog.type("profile")
  dialog.pickJava()
  dialog.create.click()
  control.fail(new Error("synthetic profile refusal"))
  await settle()
  expect(control.closed).toBe(0)
  expect(dialog.name.value).toBe("profile")
  expect(dialog.create.disabled).toBe(false)
  dialog.create.click()
  expect(control.inputs).toHaveLength(2)
  control.finish()
  await settle()
  expect(control.closed).toBe(1)
})
