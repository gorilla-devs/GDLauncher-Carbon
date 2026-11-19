import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { FEHookRun } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  data: (() => undefined) as () => FEHookRun[] | undefined,
  queryKey: null as unknown,
  acknowledge: vi.fn(),
  loading: vi.fn(),
  success: vi.fn(),
  error: vi.fn()
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { mutation: io.acknowledge } }),
    createQuery: (options: () => { queryKey: unknown }) => {
      io.queryKey = options().queryKey
      return {
        get data() {
          return io.data()
        }
      }
    }
  }
}))
vi.mock("@gd/ui", () => ({
  toast: { loading: io.loading, success: io.success, error: io.error }
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) => {
      if (!options) return key
      const { interpolation: _, ...rest } = options
      return `${key} ${JSON.stringify(rest)}`
    }
  ]
}))

import HookRuns from "./index"

const run = (
  id: number,
  state: FEHookRun["state"],
  kind: FEHookRun["kind"] = "PostExit"
): FEHookRun => ({ id, instance: 12, name: "Vanilla 1.21", kind, state })

let dispose: (() => void) | undefined
let setData: (value: FEHookRun[] | undefined) => void
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.acknowledge.mockResolvedValue(null)
  const [data, set] = createSignal<FEHookRun[] | undefined>(undefined)
  io.data = data
  setData = (value) => set(() => value)
  dispose = render(() => <HookRuns />, document.createElement("div"))
})
afterEach(() => {
  dispose?.()
  dispose = undefined
})

it("shows one notice per run, from running to how it ended", async () => {
  expect(io.queryKey).toEqual(["instance.getHookRuns"])
  setData([run(4, { type: "Running" })])
  await flush()
  expect(io.loading).toHaveBeenCalledTimes(1)
  expect(io.loading).toHaveBeenCalledWith(
    'notifications:_trn_hook_running_post {"instance":"Vanilla 1.21"}',
    { id: "hook-run-4", duration: Infinity }
  )
  // Still running: nothing new, and nothing acknowledged yet.
  setData([run(4, { type: "Running" })])
  await flush()
  expect(io.loading).toHaveBeenCalledTimes(1)
  expect(io.acknowledge).not.toHaveBeenCalled()

  // The same notice turns into the outcome, and goes by itself.
  setData([run(4, { type: "Finished" })])
  await flush()
  expect(io.success).toHaveBeenCalledWith(
    'notifications:_trn_hook_finished_post {"instance":"Vanilla 1.21"}',
    { id: "hook-run-4", duration: 4000 }
  )
  expect(io.acknowledge).toHaveBeenCalledWith([
    "instance.acknowledgeHookRun",
    4
  ])
  setData([run(4, { type: "Finished" })])
  await flush()
  expect(io.success).toHaveBeenCalledTimes(1)
  expect(io.acknowledge).toHaveBeenCalledTimes(1)
})

it("keeps a failed hook's notice, with the last thing the script printed", async () => {
  // A hook quicker than the app: the first it hears of it is the failure.
  setData([
    run(
      9,
      { type: "Failed", value: { summary: "rclone: command not found" } },
      "PreLaunch"
    )
  ])
  await flush()
  expect(io.loading).not.toHaveBeenCalled()
  expect(io.error).toHaveBeenCalledWith(
    'notifications:_trn_hook_failed_pre {"instance":"Vanilla 1.21"}',
    {
      id: "hook-run-9",
      duration: Infinity,
      description: "rclone: command not found"
    }
  )
  expect(io.acknowledge).toHaveBeenCalledWith([
    "instance.acknowledgeHookRun",
    9
  ])
})
