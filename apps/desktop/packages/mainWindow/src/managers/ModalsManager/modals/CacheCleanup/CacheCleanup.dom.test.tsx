import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import CacheCleanup from "./index"
const control = vi.hoisted(() => ({
  blockers: (() => undefined) as () => any,
  query: undefined as any,
  calls: [] as any[],
  invalidate: vi.fn(),
  dismiss: vi.fn(async () => {}),
  start: undefined as any
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { invalidateQueries: control.invalidate },
  rspc: {
    useContext: () => ({ client: { mutation: control.dismiss } }),
    createQuery: (options: any) => {
      const key = options().queryKey[0]
      if (key === "settings.getCacheCleanupBlockers")
        return {
          get data() {
            return control.blockers()
          },
          refetch: async () => ({ data: control.blockers() })
        }
      return key === "settings.getCacheSizes"
        ? { data: { gdlauncher: 10, minecraft: 20 } }
        : control.query(options)
    },
    createMutation: (options: any) => {
      const opts = options()
      control.start = opts
      return {
        isPending: false,
        mutate: (input: any) => {
          control.calls.push(input)
          opts.onSuccess(control.calls.length)
        }
      }
    }
  }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: vi.fn() })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", () => ({
  PRESS_CLASSES: "",
  Checkbox: () => null,
  Progress: () => null,
  Button: (p: any) => (
    <button disabled={p.disabled} onClick={p.onClick}>
      {p.children}
    </button>
  )
}))
let dispose: (() => void) | undefined
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  control.calls = []
  control.blockers = () => undefined
  control.invalidate.mockClear()
  control.dismiss.mockClear()
})
function fixture() {
  const [task, setTask] = createSignal<any>({
    progress: { type: "Known", value: 0 }
  })
  const [outcome, setOutcome] = createSignal<any>({
    type: "Running",
    value: task()
  })
  const [retryOutcome, setRetryOutcome] = createSignal<any>({
    type: "Running",
    value: task()
  })
  control.query = (options: any) => ({
    get data() {
      const [key, id] = options().queryKey
      return key === "vtask.getTaskOutcome"
        ? id === 2
          ? retryOutcome()
          : outcome()
        : task()
    }
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <CacheCleanup {...({} as any)} />, root)
  return { root, setTask, setOutcome, setRetryOutcome }
}
const button = (root: HTMLElement, text: string) =>
  Array.from(root.querySelectorAll("button")).find((b) =>
    b.textContent?.includes(text)
  )
it("failed cleanup shows affected path and a scoped Retry, invalidating partial changes", async () => {
  const { root, setTask, setOutcome, setRetryOutcome } = fixture()
  button(root, "start")!.click()
  await flush()
  const failed = {
    progress: {
      type: "Failed",
      value: {
        cause: [
          { display: "failure" },
          { display: "denied /fixture/kept.cache" }
        ]
      }
    }
  }
  setTask(failed)
  setOutcome({ type: "Failed", value: failed })
  await flush()
  expect(root.textContent).toContain("/fixture/kept.cache")
  expect(button(root, "_trn_cache_cleanup.retry")).toBeDefined()
  expect(control.invalidate).toHaveBeenCalled()
  button(root, "_trn_cache_cleanup.retry")!.click()
  await flush()
  expect(control.dismiss).toHaveBeenCalledWith(["vtask.dismissTask", 1])
  expect(control.calls).toEqual([
    { gdlauncher: true, minecraft: false },
    { gdlauncher: true, minecraft: false }
  ])
  setTask(null)
  setOutcome({ type: "Dismissed" })
  await flush()
  expect(root.textContent).not.toContain("done_title")
  setRetryOutcome({ type: "Succeeded" })
  await flush()
  expect(root.textContent).toContain("done_title")
})
it("a refused start shows the core's sentence, not the rspc payload", async () => {
  const { root } = fixture()
  const payload = JSON.stringify({
    code: 500,
    message: JSON.stringify({
      cause: [{ display: "A game is running", debug: "Busy" }],
      backtrace: "   0: carbon_app::managers::settings"
    })
  })
  control.start.onError(new Error(payload))
  await flush()
  expect(root.textContent).toContain("A game is running")
  expect(root.textContent).not.toContain("{")
  expect(root.textContent).not.toContain("backtrace")
})

const scope = (root: HTMLElement, name: "gdlauncher" | "minecraft") =>
  root.querySelector<HTMLElement>(
    `[data-testid="cache-cleanup-scope-${name}"]`
  )!
const INSTALL = "app:_trn_shutdown_prompt.blocker_install"
const GAME = "modals:_trn_cache_cleanup.in_use_game"

it("offers no cache while a download or install uses both, and says why", async () => {
  control.blockers = () => ({ gdlauncher: "install", minecraft: "install" })
  const { root } = fixture()

  for (const name of ["gdlauncher", "minecraft"] as const) {
    expect(scope(root, name).dataset.disabled).toBe("true")
    expect(scope(root, name).dataset.checked).toBe("false")
    expect(scope(root, name).textContent).toContain(INSTALL)
  }
  expect(root.textContent).not.toContain("gdlauncher_desc")
  expect(button(root, "start")!.disabled).toBe(true)
  button(root, "start")!.click()
  expect(control.calls).toEqual([])
})

it("cleans up only the cache nothing uses while a game runs", async () => {
  control.blockers = () => ({ gdlauncher: null, minecraft: "game" })
  const { root } = fixture()

  expect(scope(root, "gdlauncher").dataset.disabled).toBe("false")
  expect(scope(root, "gdlauncher").textContent).toContain("gdlauncher_desc")
  expect(scope(root, "minecraft").dataset.disabled).toBe("true")
  expect(scope(root, "minecraft").textContent).toContain(GAME)
  button(root, "start")!.click()
  expect(control.calls).toEqual([{ gdlauncher: true, minecraft: false }])
})

it("offers a cache again, ticked as it was, once nothing uses it", async () => {
  const [blockers, setBlockers] = createSignal<any>({
    gdlauncher: "task",
    minecraft: null
  })
  control.blockers = blockers
  const { root } = fixture()
  expect(scope(root, "gdlauncher").dataset.checked).toBe("false")
  expect(button(root, "start")!.disabled).toBe(true)

  setBlockers({ gdlauncher: null, minecraft: null })
  await flush()

  expect(scope(root, "gdlauncher").dataset.disabled).toBe("false")
  expect(scope(root, "gdlauncher").dataset.checked).toBe("true")
  expect(scope(root, "gdlauncher").textContent).toContain("gdlauncher_desc")
  expect(button(root, "start")!.disabled).toBe(false)
})

it("goes back to the choice when the core refuses a cache that came into use", async () => {
  const [blockers, setBlockers] = createSignal<any>(undefined)
  control.blockers = blockers
  const { root } = fixture()
  setBlockers({ gdlauncher: "install", minecraft: null })
  control.start.onError(new Error("refused"))
  await flush()

  expect(root.textContent).not.toContain("failed_title")
  expect(scope(root, "gdlauncher").textContent).toContain(INSTALL)
})
