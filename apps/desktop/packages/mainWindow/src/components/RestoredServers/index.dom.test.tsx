import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  data: (() => undefined) as () => number | undefined,
  queryKey: null as unknown,
  acknowledge: vi.fn(),
  info: vi.fn()
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
vi.mock("@gd/ui", () => ({ toast: { info: io.info } }))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) =>
      `${key} ${JSON.stringify(options)}`
  ]
}))

import RestoredServers from "./index"

let dispose: (() => void) | undefined
let setData: (value: number | undefined) => void
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.acknowledge.mockResolvedValue(null)
  const [data, set] = createSignal<number | undefined>(undefined)
  io.data = data
  setData = (value) => set(() => value)
  dispose = render(() => <RestoredServers />, document.createElement("div"))
})
afterEach(() => {
  dispose?.()
  dispose = undefined
})

it("says nothing while no server was restored", async () => {
  expect(io.queryKey).toEqual(["server.getRestoredServers"])
  setData(0)
  await flush()
  expect(io.info).not.toHaveBeenCalled()
  expect(io.acknowledge).not.toHaveBeenCalled()
})

it("tells the user once how many servers came back, then acknowledges", async () => {
  setData(2)
  await flush()
  expect(io.info).toHaveBeenCalledTimes(1)
  const [title, options] = io.info.mock.calls[0]
  expect(title).toBe('notifications:_trn_servers_restored {"count":2}')
  expect(options.description).toBe(
    'notifications:_trn_servers_restored_hint {"count":2}'
  )
  expect(io.acknowledge).toHaveBeenCalledWith([
    "server.acknowledgeRestoredServers"
  ])

  // A refetch that still reads the same count before the acknowledgement
  // lands does not repeat the toast; the acknowledged 0 ends it.
  setData(2)
  await flush()
  setData(0)
  await flush()
  expect(io.info).toHaveBeenCalledTimes(1)
})

it("a failed acknowledgement is logged and the toast is not repeated", async () => {
  const logged = vi.spyOn(console, "error").mockImplementation(() => {})
  io.acknowledge.mockRejectedValue(new Error("core gone"))
  setData(1)
  await flush()
  expect(io.info).toHaveBeenCalledTimes(1)
  expect(logged).toHaveBeenCalled()
  logged.mockRestore()
})
