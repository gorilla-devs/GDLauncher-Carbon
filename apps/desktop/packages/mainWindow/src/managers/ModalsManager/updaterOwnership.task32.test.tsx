import { beforeAll, beforeEach, afterEach, it, expect, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { ModalProvider, useModal, useModalStackEntry } from "."
import ModalLayout from "./ModalLayout"
import { useAddonMutations } from "@/pages/Library/Instance/Tabs/Addons/hooks/useAddonMutations"
import { deferred, settle, toast } from "@/task32.fixture"
const c = vi.hoisted(() => ({
  manager: undefined as any,
  entries: [] as any[],
  params: new URLSearchParams(),
  accept: vi.fn(),
  read: vi.fn(),
  protection: (() => true) as () => boolean,
  setParams: vi.fn(),
  launch: undefined as any,
  refresh: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task32.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => (
    <span>
      {p.key}:{JSON.stringify(p.options)}
    </span>
  ),
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => ({ id: "7" }),
  useLocation: () => ({
    get search() {
      return `?${c.params}`
    }
  }),
  useSearchParams: () => [
    undefined,
    (values: Record<string, string | null>) => {
      c.setParams(values)
      for (const [key, value] of Object.entries(values)) {
        if (value === null) c.params.delete(key)
        else c.params.set(key, value)
      }
    }
  ]
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/utils/adhelper", () => ({ default: { width: 0 } }))
vi.mock("@/utils/memoryWarningBridge", () => ({
  listenMemoryWarning: () => () => {}
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
  queryClient: {},
  rspc: {
    createQuery: () => ({ data: false }),
    useContext: () => ({ client: { query: c.read } }),
    createMutation: (f: any) => ({
      mutateAsync: async (value: any) => {
        try {
          return await c.accept(value)
        } catch (error) {
          f().onError?.(error)
          throw error
        }
      }
    })
  }
}))
vi.mock("./modals/ModsUpdater", async (original) => {
  const actual = await original<typeof import("./modals/ModsUpdater")>()
  return {
    default: (p: any) => {
      c.entries.push(useModalStackEntry())
      return (
        <div data-testid={`updater-${p.data.tag}`}>
          <actual.default {...p} />
        </div>
      )
    }
  }
})
vi.mock("./modals/InsufficientMemory", () => ({
  default: () => (
    <div data-testid="protected">
      <ModalLayout title="Synthetic protected" preventClose={c.protection()}>
        <span>Protected replacement</span>
      </ModalLayout>
    </div>
  )
}))
let dispose: (() => void) | undefined
// Load the lazily imported modal once, under the hook timeout, so the first
// test's `vi.waitFor` does not race its cold transform under parallel load.
beforeAll(async () => {
  await import("./modals/ModsUpdater")
})
beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  c.entries = []
  c.params = new URLSearchParams()
  c.manager = undefined
  c.accept.mockReset().mockResolvedValue(11)
  c.read.mockReset().mockResolvedValue({ type: "Succeeded" })
  c.protection = () => true
  c.refresh.mockReset().mockResolvedValue(undefined)
  const overlay = document.createElement("div")
  overlay.id = "overlay"
  document.body.append(overlay)
})
afterEach(async () => {
  dispose?.()
  dispose = undefined
  await vi.advanceTimersByTimeAsync(110)
  document.body.textContent = ""
  vi.useRealTimers()
})
async function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  const Capture = () => {
    c.manager = useModal()
    const hook = useAddonMutations(c.refresh, {} as any, vi.fn())
    c.launch = hook.handleUpdateSelected
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
  await settle()
}
async function updater(tag = "A", onComplete = vi.fn()) {
  c.manager.openModal(
    { name: "modsUpdater" },
    {
      tag,
      instanceId: 7,
      mods: [{ id: tag, filename: `${tag}.jar`, addon_type: "mods" }],
      onComplete
    }
  )
  await vi.waitFor(() =>
    expect(
      document.querySelector(`[data-testid="updater-${tag}"]`)
    ).not.toBeNull()
  )
  return onComplete
}
async function protectedModal() {
  c.manager.openModal({ name: "insufficientMemory" })
  await vi.waitFor(() =>
    expect(document.querySelector('[data-testid="protected"]')).not.toBeNull()
  )
}
it("0243 live updater underneath protected newer modal closes only its own non-top entry and rebuilds URL", async () => {
  const pending = deferred<number>()
  c.accept.mockReturnValue(pending.promise)
  await mount()
  const complete = await updater()
  await protectedModal()
  pending.resolve(11)
  await settle()
  expect(document.querySelector('[data-testid="protected"]')).not.toBeNull()
  expect(document.querySelector('[data-testid="updater-A"]')).toBeNull()
  expect(complete).toHaveBeenCalledTimes(1)
  expect(c.params.get("m[1]")).toBe("insufficientMemory")
  expect(c.params.has("m[2]")).toBe(false)
})
it("0243 callback that disposes updater and opens replacement cannot grant old close or success", async () => {
  const pending = deferred<number>()
  c.accept.mockReturnValue(pending.promise)
  await mount()
  const complete = vi.fn(() => {
    c.manager.closeModal()
    c.manager.openModal({ name: "insufficientMemory" })
  })
  await updater("A", complete)
  pending.resolve(11)
  await settle()
  expect(complete).toHaveBeenCalledTimes(1)
  expect(c.manager.hasOpenModals()).toBe(true)
  expect(c.params.get("m[1]")).toBe("insufficientMemory")
  // The replacement entry is already admitted; its lazy component may still
  // be loading after the callback's promise settles.
  await vi.waitFor(() =>
    expect(document.querySelector('[data-testid="protected"]')).not.toBeNull()
  )
  expect(toast.success).not.toHaveBeenCalled()
})
it("0243 async refetch stays owned until settled and disposed continuation cannot close a replacement", async () => {
  const terminal = deferred<any>()
  const refetch = deferred()
  c.read.mockReturnValue(terminal.promise)
  await mount()
  const complete = vi.fn(() => refetch.promise)
  await updater("A", complete)
  terminal.resolve({ type: "Succeeded" })
  await settle()
  expect(document.querySelector('[data-testid="updater-A"]')).not.toBeNull()
  expect(complete).toHaveBeenCalledTimes(1)
  c.manager.closeModal()
  await protectedModal()
  refetch.resolve(undefined)
  await settle()
  expect(document.querySelector('[data-testid="protected"]')).not.toBeNull()
  expect(toast.success).not.toHaveBeenCalled()
})
it("0243 disposed pending query cannot publish effects onto same-name replacement", async () => {
  const pending = deferred<any>(),
    next = deferred<any>()
  c.read.mockReturnValueOnce(pending.promise).mockReturnValueOnce(next.promise)
  await mount()
  const old = await updater()
  c.manager.closeModal()
  const current = await updater("B")
  pending.resolve({ type: "Succeeded" })
  await settle()
  expect(document.querySelector('[data-testid="updater-B"]')).not.toBeNull()
  expect(old).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  next.resolve({ type: "Succeeded" })
  await settle()
  expect(current).toHaveBeenCalledTimes(1)
  expect(c.manager.hasOpenModals()).toBe(false)
})
it("0243 owning successful programmatic close remains allowed for current top", async () => {
  const pending = deferred<any>()
  c.read.mockReturnValue(pending.promise)
  await mount()
  const complete = await updater()
  pending.resolve({ type: "Succeeded" })
  await settle()
  expect(complete).toHaveBeenCalledTimes(1)
  expect(toast.success).toHaveBeenCalledTimes(1)
  expect(c.manager.hasOpenModals()).toBe(false)
  expect(c.params.has("m[1]")).toBe(false)
})
it("0243 existing static/live Escape/header/backdrop protection remains and live accessor can release", async () => {
  await mount()
  const [blocked, setBlocked] = createSignal(true)
  c.protection = blocked
  await protectedModal()
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
  )
  document.querySelector<HTMLElement>('[data-testid="modal-close"]')!.click()
  expect(c.manager.hasOpenModals()).toBe(true)
  const backdrop = document
    .querySelector('[data-testid="protected"]')!
    .closest(".animate-modalEnter")!.parentElement!
  backdrop.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }))
  expect(c.manager.hasOpenModals()).toBe(true)
  setBlocked(false)
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
  )
  expect(c.manager.hasOpenModals()).toBe(false)
})

it("0243 actual addon callback forwards asynchronous originating refetch ownership", async () => {
  const accepted = deferred<number>(),
    refresh = deferred()
  c.accept.mockReturnValue(accepted.promise)
  c.refresh.mockReturnValue(refresh.promise)
  await mount()
  await c.launch([
    {
      id: "caller",
      filename: "caller.jar",
      has_update: true,
      addon_type: "mods"
    }
  ])
  await vi.waitFor(() => expect(c.entries).toHaveLength(1))
  accepted.resolve(11)
  await settle()
  expect(c.refresh).toHaveBeenCalledTimes(1)
  expect(c.manager.hasOpenModals()).toBe(true)
  refresh.resolve(undefined)
  await settle()
  expect(c.manager.hasOpenModals()).toBe(false)
})
it("0243 rejected owned refetch reports failure without success or unhandled continuation", async () => {
  const accepted = deferred<number>(),
    refresh = deferred()
  c.accept.mockReturnValue(accepted.promise)
  await mount()
  await updater(
    "A",
    vi.fn(() => refresh.promise)
  )
  accepted.resolve(11)
  await settle()
  refresh.reject(Error("owned refresh failed"))
  await settle()
  expect(toast.error).toHaveBeenCalledWith(
    "content:_trn_mods_updater.refresh_error",
    expect.objectContaining({
      // The cause itself, without an "Error:" prefix or an rspc payload.
      description: "owned refresh failed"
    })
  )
  expect(toast.success).not.toHaveBeenCalled()
  expect(c.manager.hasOpenModals()).toBe(true)
})
it("0243 disposed mutation rejection cannot publish stale configured error feedback", async () => {
  const accepted = deferred<number>()
  c.accept.mockReturnValue(accepted.promise)
  await mount()
  await updater()
  c.manager.closeModal()
  await protectedModal()
  accepted.reject(Error("late old mutation failed"))
  await settle()
  expect(toast.error).not.toHaveBeenCalled()
  expect(document.querySelector('[data-testid="protected"]')).not.toBeNull()
})
it("0243 cleanup retires owned polling waiter and vanished entry close remains no-op", async () => {
  c.read.mockResolvedValue({ type: "Pending" })
  await mount()
  await updater()
  await settle()
  const entry = c.entries[0]
  c.manager.closeModal()
  await vi.advanceTimersByTimeAsync(110)
  await settle()
  const reads = c.read.mock.calls.length
  expect(entry.closeOwn()).toBe(false)
  await vi.advanceTimersByTimeAsync(600)
  expect(c.read).toHaveBeenCalledTimes(reads)
  expect(toast.success).not.toHaveBeenCalled()
})
