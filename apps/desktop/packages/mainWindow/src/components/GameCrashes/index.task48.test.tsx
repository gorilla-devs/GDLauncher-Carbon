import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { FEGameCrash } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  data: (() => undefined) as () => FEGameCrash[] | undefined,
  queryKey: null as unknown,
  acknowledge: vi.fn(),
  error: vi.fn(),
  navigate: vi.fn()
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
vi.mock("@gd/ui", () => ({ toast: { error: io.error } }))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) => {
      if (!options) return key
      const { interpolation: _, ...rest } = options
      return `${key} ${JSON.stringify(rest)}`
    }
  ]
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: io.navigate })
}))

import GameCrashes from "./index"

const crash = (id: number): FEGameCrash => ({
  id,
  instance: 12,
  name: "All the Mods 10",
  kind: { type: "Crashed", value: { report: true } }
})

let dispose: (() => void) | undefined
let setData: (value: FEGameCrash[] | undefined) => void
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.acknowledge.mockResolvedValue(null)
  const [data, set] = createSignal<FEGameCrash[] | undefined>(undefined)
  io.data = data
  setData = (value) => set(() => value)
  dispose = render(() => <GameCrashes />, document.createElement("div"))
})
afterEach(() => {
  dispose?.()
  dispose = undefined
})

it("says a game crashed once, stays until closed, and opens that instance's log", async () => {
  expect(io.queryKey).toEqual(["instance.getGameCrashes"])
  setData([])
  await flush()
  expect(io.error).not.toHaveBeenCalled()

  setData([crash(3)])
  await flush()
  expect(io.error).toHaveBeenCalledTimes(1)
  const [title, options] = io.error.mock.calls[0]
  expect(title).toBe(
    'notifications:_trn_game_crashed {"instance":"All the Mods 10"}'
  )
  expect(options.description).toBe("notifications:_trn_game_crashed_report")
  expect(options.duration).toBe(Infinity)
  expect(options.action.label).toBe("notifications:_trn_game_open_log")
  options.action.onClick()
  expect(io.navigate).toHaveBeenCalledWith("/library/12/logs")
  expect(io.acknowledge).toHaveBeenCalledWith([
    "instance.acknowledgeGameCrash",
    3
  ])

  // The refetch after the acknowledgement may still carry it: not shown twice.
  setData([crash(3)])
  await flush()
  expect(io.error).toHaveBeenCalledTimes(1)
  setData([crash(3), crash(4)])
  await flush()
  expect(io.error).toHaveBeenCalledTimes(2)
})
