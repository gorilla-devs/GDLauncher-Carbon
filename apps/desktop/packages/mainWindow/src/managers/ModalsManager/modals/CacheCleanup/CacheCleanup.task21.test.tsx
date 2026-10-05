import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import CacheCleanup from "./index"
const control = vi.hoisted(() => ({
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
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
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
