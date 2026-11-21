import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createQuery, QueryClientProvider } from "@tanstack/solid-query"
import initRspc, { queryClient, rspc } from "./rspcClient"
vi.mock("@gd/ui", () => ({
  toast: { error: vi.fn(), success: vi.fn(), dismiss: vi.fn() },
  Progress: () => <span />
}))
vi.mock("@gd/i18n", () => ({
  i18n: { language: "english", on: vi.fn(), t: (key: string) => key }
}))
let dispose: (() => void) | undefined, stopSocket: (() => void) | undefined
afterEach(() => {
  stopSocket?.()
  dispose?.()
  queryClient.clear()
  vi.restoreAllMocks()
  vi.useRealTimers()
  vi.unstubAllGlobals()
  document.body.textContent = ""
})
const flush = async () => {
  for (let i = 0; i < 30; i++) await Promise.resolve()
}
it("reconnect settings catch-up preserves the same-channel updater's next periodic check", async () => {
  vi.useFakeTimers()
  let settings = { releaseChannel: "stable", theme: "dark" }
  const reads = vi.fn(async () => settings)
  // Inject the query handler directly: the real rspc HTTP/auth transport is
  // never entered, while the real shared QueryClient and Solid query are used.
  vi.spyOn(rspc, "createQuery").mockImplementation(((opts: any) =>
    createQuery(() => ({ ...opts(), queryFn: reads }))) as any)
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: true }),
    getUpdateState: async () => null,
    onUpdateStateChanged: vi.fn(),
    checkForUpdates: vi.fn()
  })
  const updater = await import("./updater")
  const root = document.createElement("div")
  document.body.append(root)
  const Content = () => {
    void updater.checkForUpdates()
    return <span />
  }
  dispose = render(
    () => (
      <QueryClientProvider client={queryClient}>
        <Content />
      </QueryClientProvider>
    ),
    root
  )
  class Socket extends EventTarget {
    static sockets: Socket[] = []
    constructor(_url: string) {
      super()
      Socket.sockets.push(this)
    }
    close() {
      this.dispatchEvent(new Event("close"))
    }
    open() {
      this.dispatchEvent(new Event("open"))
    }
  }
  vi.stubGlobal("WebSocket", Socket)
  stopSocket = initRspc(12345, "synthetic-token").createInvalidateQuery()
  Socket.sockets[0].open()
  await flush()
  expect(window.checkForUpdates).toHaveBeenCalledTimes(1)
  Socket.sockets[0].close()
  settings = { releaseChannel: "stable", theme: "light" }
  await vi.advanceTimersByTimeAsync(1000)
  Socket.sockets[1].open()
  await flush()
  await vi.advanceTimersByTimeAsync(0)
  expect(reads).toHaveBeenCalledTimes(2)
  expect(window.checkForUpdates).toHaveBeenCalledTimes(1)
  await vi.advanceTimersByTimeAsync(30 * 60 * 1000)
  expect(window.checkForUpdates).toHaveBeenCalledTimes(2)
  expect(window.checkForUpdates).toHaveBeenLastCalledWith("stable")
})
