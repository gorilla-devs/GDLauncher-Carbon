import { beforeEach, afterEach, it, expect, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import JavaProfileCreationModal from "./JavaProfileCreationModal"
import JavaPathAutoComplete from "@/components/JavaPathAutoComplete"
import { button, deferred, input, settle, toast } from "@/task32.fixture"
const c = vi.hoisted(() => ({
  available: (() => ({})) as () => any,
  profiles: (() => []) as () => any,
  pending: (() => false) as () => boolean,
  register: vi.fn(),
  create: vi.fn(),
  close: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task32.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (f: any) => ({
      get data() {
        return f().queryKey[0] === "java.getAvailableJavas"
          ? c.available()
          : c.profiles()
      },
      isPending: false,
      isError: false,
      isFetching: false
    }),
    createMutation: (f: any) => ({
      get isPending() {
        return (
          (Array.isArray(f().mutationKey)
            ? f().mutationKey[0]
            : f().mutationKey) === "java.createCustomJavaVersion" && c.pending()
        )
      },
      mutate: (v: any) => c.register(v).catch(() => undefined),
      mutateAsync: (v: any) =>
        (Array.isArray(f().mutationKey)
          ? f().mutationKey[0]
          : f().mutationKey) === "java.createCustomJavaVersion"
          ? c.register(v)
          : c.create(v)
    })
  }
}))
let dispose: (() => void) | undefined, setAvailable: any, setPending: any
const java = (id: string, path: string, valid = true) => ({
  id,
  path,
  isValid: valid,
  version: "21",
  type: "custom"
})
beforeEach(() => {
  vi.clearAllMocks()
  c.register.mockReset().mockResolvedValue(undefined)
  c.create.mockReset().mockResolvedValue(undefined)
  const [available, set] = createSignal<any>({
    custom: [
      java("A", "/synthetic/java-A"),
      java("B", "/synthetic/java-B"),
      java("bad", "/synthetic/java-invalid", false)
    ]
  })
  c.available = available
  setAvailable = set
  const [pending, update] = createSignal(false)
  c.pending = pending
  setPending = update
  c.profiles = () => []
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <JavaProfileCreationModal title="Synthetic Java profile" />,
    root
  )
  input(root, 0, "New profile")
  return root
}
function path(root: HTMLElement, value: string) {
  root.querySelectorAll("input")[1].dispatchEvent(new Event("blur"))
  input(root, 1, value)
}
function registerOption(root: HTMLElement) {
  const text = Array.from(root.querySelectorAll("span")).find((x) =>
    x.textContent?.includes("create_new_custom")
  )
  const target = text?.closest(".flex.w-full.flex-col.gap-2") as
    | HTMLElement
    | undefined
  if (!target)
    throw new Error(`Registration option fixture missing: ${root.innerHTML}`)
  target.click()
}
it.each(["", "/synthetic/unregistered"])(
  "0102 A then current path %j clears id and refuses Create at handler",
  async (value) => {
    const root = mount()
    path(root, "/synthetic/java-A")
    path(root, value)
    expect(button(root, "general:_trn_create").disabled).toBe(true)
    ;(button(root, "general:_trn_create") as any).invoke()
    await settle()
    expect(c.create).not.toHaveBeenCalled()
    expect(c.close).not.toHaveBeenCalled()
  }
)
it("0102 invalid registered Java path never becomes a submit id", async () => {
  const root = mount()
  path(root, "/synthetic/java-invalid")
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  ;(button(root, "general:_trn_create") as any).invoke()
  await settle()
  expect(c.create).not.toHaveBeenCalled()
})
it("0102 successful registration with no returned id waits for current valid query match", async () => {
  const registration = deferred()
  c.register.mockReturnValue(registration.promise)
  const root = mount()
  path(root, "/synthetic/java-A")
  path(root, "/synthetic/new-B")
  setPending(true)
  registerOption(root)
  expect(c.register).toHaveBeenCalledWith("/synthetic/new-B")
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  registration.resolve(undefined)
  setPending(false)
  await settle()
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  setAvailable({ custom: [java("new-B", "/synthetic/new-B")] })
  expect(button(root, "general:_trn_create").disabled).toBe(false)
  button(root, "general:_trn_create").click()
  await settle()
  expect(c.create).toHaveBeenCalledWith({
    profileName: "New profile",
    javaId: "new-B"
  })
})
it("0102 old B availability after changing to C cannot restore A or B", async () => {
  const root = mount()
  path(root, "/synthetic/java-A")
  path(root, "/synthetic/new-B")
  path(root, "/synthetic/new-C")
  setAvailable({
    custom: [java("A", "/synthetic/java-A"), java("new-B", "/synthetic/new-B")]
  })
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  ;(button(root, "general:_trn_create") as any).invoke()
  await settle()
  expect(c.create).not.toHaveBeenCalled()
})
it("0102 registration pending cannot publish even a currently available id", async () => {
  const root = mount()
  path(root, "/synthetic/java-A")
  setPending(true)
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  ;(button(root, "general:_trn_create") as any).invoke()
  await settle()
  expect(c.create).not.toHaveBeenCalled()
})
it("0102 valid B replaces A and exact current id is submitted", async () => {
  const root = mount()
  path(root, "/synthetic/java-A")
  path(root, "/synthetic/java-B")
  button(root, "general:_trn_create").click()
  await settle()
  expect(c.create).toHaveBeenCalledWith({
    profileName: "New profile",
    javaId: "B"
  })
  expect(c.close).toHaveBeenCalledTimes(1)
  expect(toast.success).toHaveBeenCalledWith("Profile created")
})
it("0102 initially empty and duplicate profile names remain blocked at handler", async () => {
  const root = mount()
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  path(root, "/synthetic/java-A")
  c.profiles = () => [{ name: "New profile" }]
  input(root, 0, "Existing")
  c.profiles = () => [{ name: "Existing" }]
  ;(button(root, "general:_trn_create") as any).invoke()
  await settle()
  expect(c.create).not.toHaveBeenCalled()
})
it("0102 pending profile Create has synchronous reentry guard and rejection keeps retry", async () => {
  const wait = deferred()
  c.create.mockReturnValueOnce(wait.promise)
  const root = mount()
  path(root, "/synthetic/java-A")
  const action = (button(root, "general:_trn_create") as any).invoke
  const first = action()
  const second = action()
  expect(c.create).toHaveBeenCalledTimes(1)
  wait.reject(new Error("synthetic profile refusal"))
  const outcomes = await Promise.allSettled([first, second])
  expect(outcomes.every((result) => result.status === "fulfilled")).toBe(true)
  await settle()
  expect(c.close).not.toHaveBeenCalled()
  expect(button(root, "general:_trn_create").disabled).toBe(false)
  await action()
  expect(c.create).toHaveBeenCalledTimes(2)
})
it("0102 blur-only path consumer retains callback path and normal valid id semantics", () => {
  const root = document.createElement("div")
  document.body.append(root)
  const update = vi.fn()
  dispose = render(
    () => <JavaPathAutoComplete updateValueOnlyOnBlur updateValue={update} />,
    root
  )
  input(root, 0, "/synthetic/java-B")
  expect(update).not.toHaveBeenCalled()
  root.querySelector("input")!.dispatchEvent(new Event("blur"))
  expect(update).toHaveBeenCalledWith("B", "/synthetic/java-B")
})

it("0102 rejected unmatched registration preserves invalid current path without stale Create", async () => {
  const failed = deferred()
  c.register.mockReturnValue(failed.promise)
  const root = mount()
  path(root, "/synthetic/java-A")
  path(root, "/synthetic/new-B")
  registerOption(root)
  failed.reject(new Error("synthetic registration refusal"))
  await settle()
  expect(c.register).toHaveBeenCalledWith("/synthetic/new-B")
  expect(button(root, "general:_trn_create").disabled).toBe(true)
  ;(button(root, "general:_trn_create") as any).invoke()
  await settle()
  expect(c.create).not.toHaveBeenCalled()
})
