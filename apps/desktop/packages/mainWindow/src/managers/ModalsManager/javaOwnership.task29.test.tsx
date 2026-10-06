import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { ModalProvider, useModal, useModalStackEntry } from "."
import { deferred, settle, toast } from "../../task29.fixture"

const control = vi.hoisted(() => ({
  mutation: undefined as any,
  manager: undefined as any,
  entries: [] as any[],
  params: new URLSearchParams(),
  memory: undefined as any
}))

vi.mock("@gd/ui", async () => import("../../task29.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({
  useLocation: () => ({
    get search() {
      return `?${control.params}`
    }
  }),
  useSearchParams: () => [
    undefined,
    (values: Record<string, string | null>) => {
      for (const [key, value] of Object.entries(values)) {
        if (value === null) control.params.delete(key)
        else control.params.set(key, value)
      }
    }
  ]
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/utils/adhelper", () => ({ default: { width: 0 } }))
vi.mock("@/utils/memoryWarningBridge", () => ({
  listenMemoryWarning: (callback: any) => {
    control.memory = callback
    return () => {
      control.memory = undefined
    }
  }
}))
vi.mock("@/utils/serverEulaBridge", () => ({
  listenServerEula: () => () => {}
}))
vi.mock("./modals/CacheCleanup/state", () => ({ cleanupRunning: () => false }))
vi.mock("./modals/ShaderLoaderSetup/state", () => ({
  shaderInstallRunning: () => false
}))
vi.mock("./modals/ConfirmChangeRuntimePath/state", () => ({
  isChangingRuntimePath: () => false,
  runtimeRecoveryRequired: () => false
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (options: any) => control.mutation(options().mutationKey[0])
  }
}))
vi.mock("./modals/Java/AddCustomJava", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("./modals/Java/AddCustomJava")>()
  return {
    default: (props: any) => {
      control.entries.push(useModalStackEntry())
      return <actual.default {...props} />
    }
  }
})
vi.mock("./modals/InsufficientMemory", () => ({
  default: () => <div data-testid="later-warning">Synthetic warning</div>
}))

let dispose: (() => void) | undefined
// The manager loads the modal with `lazy(() => import(...))`. Loading it here,
// under the hook timeout, keeps the first test's short `vi.waitFor` from
// racing a cold transform of the whole modal module graph when the suite
// runs alongside the other projects.
beforeAll(async () => {
  await import("./modals/Java/AddCustomJava")
})
beforeEach(() => {
  control.entries = []
  control.params = new URLSearchParams()
  control.manager = undefined
  const overlay = document.createElement("div")
  overlay.id = "overlay"
  document.body.append(overlay)
})
afterEach(async () => {
  dispose?.()
  dispose = undefined
  // Let the actual manager's bounded overlay transitions settle before
  // discarding this synthetic DOM; no native timers or listeners are used.
  await new Promise((resolve) => setTimeout(resolve, 110))
  document.body.textContent = ""
  vi.clearAllMocks()
})

async function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  const Capture = () => {
    control.manager = useModal()
    return null
  }
  dispose = render(
    () => (
      <ModalProvider>
        <Capture />
      </ModalProvider>
    ),
    root
  )
  control.manager.openModal({ name: "addCustomJava" })
  await vi.waitFor(() =>
    expect(document.querySelectorAll("input")).toHaveLength(1)
  )
}
async function submit(path: string) {
  const inputs = document.querySelectorAll("input")
  const input = inputs[inputs.length - 1]
  input.value = path
  input.dispatchEvent(new Event("input", { bubbles: true }))
  input.dispatchEvent(new FocusEvent("blur"))
  await settle()
  const buttons = Array.from(document.querySelectorAll("button"))
  const add = buttons[buttons.length - 1]
  expect(add.disabled).toBe(false)
  add.click()
  await settle()
}
function mutationFixture(pending: ReturnType<typeof deferred>) {
  const create = vi.fn(() => pending.promise)
  control.mutation = (key: string) => ({
    isPending: false,
    mutateAsync:
      key === "java.validateCustomJavaPath" ? vi.fn(async () => true) : create
  })
  return create
}

it("D4a disposed Java A cannot close later duplicate-name B on successful creation", async () => {
  const pending = deferred()
  const create = mutationFixture(pending)
  await mount()
  await submit("synthetic-java-A")
  const retiredEntry = control.entries[0]
  document.querySelector<HTMLElement>('[data-testid="modal-close"]')!.click()
  control.manager.openModal({ name: "addCustomJava" })
  await vi.waitFor(() => expect(control.entries).toHaveLength(2))
  expect(control.entries[1]).not.toBe(retiredEntry)
  pending.resolve(undefined)
  await settle()
  expect(create).toHaveBeenCalledExactlyOnceWith("synthetic-java-A")
  expect(toast.success).toHaveBeenCalledOnce()
  expect(document.querySelectorAll("input")).toHaveLength(1)
  expect(control.manager.hasOpenModals()).toBe(true)
  expect(control.params.get("m[1]")).toBe("addCustomJava")
})

it("D4a live Java A underneath a later warning cannot close that warning", async () => {
  const pending = deferred()
  mutationFixture(pending)
  await mount()
  await submit("synthetic-java-A")
  control.memory({ synthetic: true })
  await vi.waitFor(() =>
    expect(
      document.querySelector('[data-testid="later-warning"]')
    ).not.toBeNull()
  )
  expect(document.querySelector('[data-testid="later-warning"]')).not.toBeNull()
  pending.resolve(undefined)
  await settle()
  expect(toast.success).toHaveBeenCalledOnce()
  expect(document.querySelector('[data-testid="later-warning"]')).not.toBeNull()
  expect(document.querySelectorAll("input")).toHaveLength(1)
  expect(control.params.get("m[2]")).toBe("insufficientMemory")
})

it("D4a current-top duplicate-name Java success removes only its own entry and rebuilds URL", async () => {
  const pending = deferred()
  const create = mutationFixture(pending)
  await mount()
  control.manager.openModal({ name: "addCustomJava" })
  await vi.waitFor(() =>
    expect(document.querySelectorAll("input")).toHaveLength(2)
  )
  expect(control.entries[1]).not.toBe(control.entries[0])
  await submit("synthetic-java-B")
  pending.resolve(undefined)
  await settle()
  expect(create).toHaveBeenCalledExactlyOnceWith("synthetic-java-B")
  expect(toast.success).toHaveBeenCalledOnce()
  expect(document.querySelectorAll("input")).toHaveLength(1)
  expect(control.params.get("m[1]")).toBe("addCustomJava")
  expect(control.params.has("m[2]")).toBe(false)
})

it("D4a retired rejected Java creation leaves the later entry and emits no success", async () => {
  const pending = deferred()
  mutationFixture(pending)
  await mount()
  await submit("synthetic-java-A")
  control.manager.closeModal()
  control.manager.openModal({ name: "addCustomJava" })
  await settle()
  pending.reject(new Error("synthetic duplicate"))
  await settle()
  expect(toast.success).not.toHaveBeenCalled()
  expect(document.querySelectorAll("input")).toHaveLength(1)
  expect(control.manager.hasOpenModals()).toBe(true)
})

it("D4a entry operation rejects retired and other-top owners and closes only current top", async () => {
  mutationFixture(deferred())
  await mount()
  const first = control.entries[0]
  control.manager.openModal({ name: "addCustomJava" })
  await vi.waitFor(() => expect(control.entries).toHaveLength(2))
  const second = control.entries[1]
  expect(first.closeIfCurrentTop()).toBe(false)
  expect(document.querySelectorAll("input")).toHaveLength(2)
  expect(second.closeIfCurrentTop()).toBe(true)
  expect(document.querySelectorAll("input")).toHaveLength(1)
  expect(second.closeIfCurrentTop()).toBe(false)
  expect(control.params.has("m[2]")).toBe(false)
  expect(first.closeIfCurrentTop()).toBe(true)
  expect(first.closeIfCurrentTop()).toBe(false)
  expect(control.manager.hasOpenModals()).toBe(false)
})

it("D4a disposing the manager owner retires the entry even if its signal still contains it", async () => {
  const pending = deferred()
  mutationFixture(pending)
  await mount()
  await submit("synthetic-java-A")
  const entry = control.entries[0]
  dispose?.()
  dispose = undefined
  expect(entry.closeIfCurrentTop()).toBe(false)
  pending.resolve(undefined)
  await settle()
  expect(toast.success).toHaveBeenCalledOnce()
  expect(control.params.get("m[1]")).toBe("addCustomJava")
})
