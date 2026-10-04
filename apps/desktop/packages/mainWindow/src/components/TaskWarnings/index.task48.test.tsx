import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { FETaskWarnings } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  data: (() => undefined) as () => FETaskWarnings[] | undefined,
  queryKey: null as unknown,
  acknowledge: vi.fn(),
  warning: vi.fn()
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
vi.mock("@gd/ui", () => ({ toast: { warning: io.warning } }))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) => {
      if (!options) return key
      const { interpolation: _, ...rest } = options
      return `${key} ${JSON.stringify(rest)}`
    }
  ]
}))

import TaskWarnings from "./index"

const entry = (id: number): FETaskWarnings => ({
  id,
  name: {
    translation: "InstanceTaskInstallMod",
    args: { mod_name: "Iris", instance_name: "Fabric 1.20.6" }
  },
  subject: null,
  warnings: [
    {
      type: "SkippedDependency",
      value: {
        name: "Sodium",
        platform: "Modrinth",
        project_id: "AANobbMI",
        reason: "NoCompatibleVersion"
      }
    }
  ]
})

let dispose: (() => void) | undefined
let setData: (value: FETaskWarnings[] | undefined) => void
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.acknowledge.mockResolvedValue(null)
  const [data, set] = createSignal<FETaskWarnings[] | undefined>(undefined)
  io.data = data
  setData = (value) => set(() => value)
  dispose = render(() => <TaskWarnings />, document.createElement("div"))
})
afterEach(() => {
  dispose?.()
  dispose = undefined
})

it("reads the pending warnings and shows nothing while there are none", async () => {
  expect(io.queryKey).toEqual(["vtask.getTaskWarnings"])
  setData([])
  await flush()
  expect(io.warning).not.toHaveBeenCalled()
  expect(io.acknowledge).not.toHaveBeenCalled()
})

it("shows a task's warnings once, as a sticky toast, then acknowledges them", async () => {
  setData([entry(7)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(1)
  const [title, options] = io.warning.mock.calls[0]
  expect(title).toBe(
    'notifications:_trn_addon_dependencies_skipped_in_instance {"title":"Iris","instance":"Fabric 1.20.6"}'
  )
  expect(options.duration).toBe(Infinity)
  const description = options.description as HTMLElement
  expect(description.textContent).toContain(
    "notifications:_trn_addon_dependency_skipped_no_compatible_version"
  )
  expect(description.textContent).toContain(
    "notifications:_trn_addon_dependencies_skipped_hint"
  )
  expect(io.acknowledge).toHaveBeenCalledTimes(1)
  expect(io.acknowledge).toHaveBeenCalledWith([
    "vtask.acknowledgeTaskWarnings",
    7
  ])
  // The acknowledgement invalidation refetches the same entry before the
  // backend drops it: it is not shown again.
  setData([entry(7)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(1)
  expect(io.acknowledge).toHaveBeenCalledTimes(1)
  // A later task's warnings are shown on their own.
  setData([entry(7), entry(8)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(2)
  expect(io.acknowledge).toHaveBeenLastCalledWith([
    "vtask.acknowledgeTaskWarnings",
    8
  ])
})

it("does not toast again when the acknowledgement fails", async () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {})
  io.acknowledge.mockRejectedValueOnce(new Error("socket closed"))
  setData([entry(9)])
  await flush()
  expect(error).toHaveBeenCalledWith(
    "Could not acknowledge task warnings",
    expect.any(Error)
  )
  setData([entry(9)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(1)
  error.mockRestore()
})
