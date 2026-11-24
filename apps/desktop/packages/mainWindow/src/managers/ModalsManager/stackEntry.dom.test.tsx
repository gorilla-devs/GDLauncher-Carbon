import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { ModalProvider, useModal, useModalStackEntry } from "."

const control = vi.hoisted(() => ({
  manager: undefined as any,
  entries: [] as any[],
  params: new URLSearchParams(),
  memory: undefined as any
}))

vi.mock("@gd/ui", async () => import("@/test/uiBoxes.fixture"))
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
vi.mock("@/utils/rspcClient", () => ({ rspc: {} }))
// Any dialog serves: each one opened records the entry the manager gave it.
vi.mock("./modals/WhyAreAdsNeeded", () => ({
  default: () => {
    control.entries.push(useModalStackEntry())
    return <div data-testid="owned-dialog" />
  }
}))
vi.mock("./modals/InsufficientMemory", () => ({
  default: () => <div data-testid="later-warning">Synthetic warning</div>
}))

let dispose: (() => void) | undefined
// The manager loads the modal with `lazy(() => import(...))`. Loading it here,
// under the hook timeout, keeps the first test's short `vi.waitFor` from
// racing a cold transform of the whole modal module graph when the suite
// runs alongside the other projects.
beforeAll(async () => {
  await import("./modals/WhyAreAdsNeeded")
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

const dialogs = () => document.querySelectorAll('[data-testid="owned-dialog"]')
async function open(count: number) {
  control.manager.openModal({ name: "whyAreAdsNeeded" })
  await vi.waitFor(() => expect(control.entries).toHaveLength(count))
}
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
  await open(1)
}

it("an entry closes its dialog only while that dialog is on top, and only once", async () => {
  await mount()
  const first = control.entries[0]
  await open(2)
  const second = control.entries[1]
  expect(second).not.toBe(first)
  expect(first.closeIfCurrentTop()).toBe(false)
  expect(dialogs()).toHaveLength(2)
  expect(second.closeIfCurrentTop()).toBe(true)
  expect(dialogs()).toHaveLength(1)
  expect(second.closeIfCurrentTop()).toBe(false)
  expect(control.params.get("m[1]")).toBe("whyAreAdsNeeded")
  expect(control.params.has("m[2]")).toBe(false)
  expect(first.closeIfCurrentTop()).toBe(true)
  expect(first.closeIfCurrentTop()).toBe(false)
  expect(control.manager.hasOpenModals()).toBe(false)
})

it("a closed dialog's entry cannot close the same dialog opened after it", async () => {
  await mount()
  const retired = control.entries[0]
  control.manager.closeModal()
  await open(2)
  expect(control.entries[1]).not.toBe(retired)
  expect(retired.closeIfCurrentTop()).toBe(false)
  expect(dialogs()).toHaveLength(1)
  expect(control.manager.hasOpenModals()).toBe(true)
  expect(control.params.get("m[1]")).toBe("whyAreAdsNeeded")
})

it("a dialog under a later warning cannot close that warning", async () => {
  await mount()
  control.memory({ synthetic: true })
  await vi.waitFor(() =>
    expect(
      document.querySelector('[data-testid="later-warning"]')
    ).not.toBeNull()
  )
  expect(control.entries[0].closeIfCurrentTop()).toBe(false)
  expect(document.querySelector('[data-testid="later-warning"]')).not.toBeNull()
  expect(dialogs()).toHaveLength(1)
  expect(control.params.get("m[2]")).toBe("insufficientMemory")
})

it("an entry is retired with the dialog manager that gave it", async () => {
  await mount()
  const entry = control.entries[0]
  dispose?.()
  dispose = undefined
  expect(entry.closeIfCurrentTop()).toBe(false)
  expect(control.params.get("m[1]")).toBe("whyAreAdsNeeded")
})
