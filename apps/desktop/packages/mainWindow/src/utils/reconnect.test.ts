import { afterEach, expect, it, vi } from "vitest"
import { QueryObserver } from "@tanstack/query-core"
import initRspc, { queryClient } from "./rspcClient"
import { toast } from "@gd/ui"
vi.mock("@gd/ui", () => ({ toast: { error: vi.fn(), success: vi.fn() } }))
vi.mock("@gd/i18n", () => ({ i18n: { t: (key: string) => key } }))

class Socket extends EventTarget {
  static sockets: Socket[] = []
  close = vi.fn(() => this.dispatchEvent(new Event("close")))
  constructor(_url: string) {
    super()
    Socket.sockets.push(this)
  }
  open() {
    this.dispatchEvent(new Event("open"))
  }
  message(data: string) {
    this.dispatchEvent(new MessageEvent("message", { data }))
  }
}
let cleanup: any
const unsubscribers: (() => void)[] = []
const flush = async () => {
  for (let i = 0; i < 30; i++) await Promise.resolve()
}
afterEach(() => {
  if (typeof cleanup === "function") cleanup()
  cleanup = undefined
  unsubscribers.splice(0).forEach((u) => u())
  queryClient.clear()
  Socket.sockets = []
  vi.useRealTimers()
  vi.unstubAllGlobals()
})
function connect() {
  vi.stubGlobal("WebSocket", Socket)
  cleanup = initRspc(12345, "synthetic-token").createInvalidateQuery()
  return Socket.sockets[0]
}
function observe(key: string, fn: () => Promise<any>) {
  const observer = new QueryObserver(queryClient, {
    queryKey: [key],
    queryFn: fn
  })
  unsubscribers.push(observer.subscribe(() => {}))
  return observer
}
it("successful reconnect catches up active instance/task/settings state and stales inactive entries", async () => {
  vi.useFakeTimers()
  let running = true
  const instances = observe("instance.getAllInstances", async () => ({
    running
  }))
  const tasks = observe("vtask.getTasks", async () => (running ? ["task"] : []))
  const settings = observe("settings.getSettings", async () => ({
    theme: running ? "dark" : "light",
    releaseChannel: "stable"
  }))
  queryClient.setQueryData(["server.getAllServers"], ["stale"])
  const socket = connect()
  socket.open()
  await flush()
  expect(instances.getCurrentResult().data).toEqual({ running: true })
  socket.close()
  running = false
  await vi.advanceTimersByTimeAsync(1000)
  Socket.sockets[1].open()
  await flush()
  expect(instances.getCurrentResult().data).toEqual({ running: false })
  expect(tasks.getCurrentResult().data).toEqual([])
  expect(settings.getCurrentResult().data.theme).toBe("light")
  expect(
    queryClient.getQueryState(["server.getAllServers"])?.isInvalidated
  ).toBe(true)
})
it("first connection relies on initial query; matching message and malformed-frame handling remain", async () => {
  vi.useFakeTimers()
  let value = 1
  const fn = vi.fn(async () => value)
  const observer = observe("instance.getAllInstances", fn)
  const socket = connect()
  socket.open()
  await flush()
  expect(fn).toHaveBeenCalledTimes(1)
  value = 2
  socket.message("{bad")
  socket.message(
    JSON.stringify({ key: "instance.getAllInstances", args: null })
  )
  await flush()
  expect(observer.getCurrentResult().data).toBe(2)
})
it("obsolete opens/close bursts own one retry timer and disposal prevents future connection", async () => {
  vi.useFakeTimers()
  const first = connect()
  first.open()
  first.close()
  first.close()
  await vi.advanceTimersByTimeAsync(1000)
  expect(Socket.sockets).toHaveLength(2)
  first.open()
  first.close()
  Socket.sockets[1].open()
  await vi.advanceTimersByTimeAsync(1000)
  expect(Socket.sockets).toHaveLength(2)
  expect(typeof cleanup).toBe("function")
  cleanup()
  await vi.advanceTimersByTimeAsync(5000)
  expect(Socket.sockets).toHaveLength(2)
})
it("a newer reconnect during an unsettled refresh queues one serialized follow-up", async () => {
  vi.useFakeTimers()
  let value = 1,
    resolve!: (value: number) => void,
    calls = 0,
    active = 0,
    maxActive = 0
  const observer = observe("instance.getAllInstances", async () => {
    calls++
    active++
    maxActive = Math.max(maxActive, active)
    const result =
      calls === 2
        ? await new Promise<number>((r) => {
            resolve = r
          })
        : value
    active--
    return result
  })
  const first = connect()
  first.open()
  await flush()
  first.close()
  value = 2
  await vi.advanceTimersByTimeAsync(1000)
  const second = Socket.sockets[1]
  second.open()
  await flush()
  expect(calls).toBe(2)
  second.close()
  value = 3
  await vi.advanceTimersByTimeAsync(1000)
  Socket.sockets[2].open()
  Socket.sockets[2].open()
  await flush()
  expect(calls).toBe(2)
  resolve(2)
  await flush()
  expect(calls).toBe(3)
  expect(maxActive).toBe(1)
  expect(observer.getCurrentResult().data).toBe(3)
})
it("refresh rejection is handled and the next reconnect can recover", async () => {
  vi.useFakeTimers()
  let fail = false,
    value = 1
  const observer = observe("settings.getSettings", async () => {
    if (fail) throw new Error("synthetic refresh failure")
    return value
  })
  const first = connect()
  first.open()
  await flush()
  first.close()
  fail = true
  await vi.advanceTimersByTimeAsync(1000)
  Socket.sockets[1].open()
  await flush()
  expect(observer.getCurrentResult().isError).toBe(true)
  Socket.sockets[1].close()
  fail = false
  value = 2
  await vi.advanceTimersByTimeAsync(1000)
  Socket.sockets[2].open()
  await flush()
  expect(observer.getCurrentResult().data).toBe(2)
})
it("reconnect excludes provider queries while delete-failure notification remains supported", async () => {
  vi.useFakeTimers()
  const provider = vi.fn(async () => "project")
  observe("modplatforms.unifiedGetProject", provider)
  const first = connect()
  first.open()
  await flush()
  first.close()
  await vi.advanceTimersByTimeAsync(1000)
  Socket.sockets[1].open()
  await flush()
  expect(provider).toHaveBeenCalledTimes(1)
  vi.mocked(toast.error).mockClear()
  Socket.sockets[1].message(
    JSON.stringify({
      key: "instance.deleteInstanceFailed",
      args: { error: "synthetic deletion failure" }
    })
  )
  await flush()
  expect(toast.error).toHaveBeenCalledWith(
    "notifications:_trn_cannot_delete_instance",
    { description: "synthetic deletion failure" }
  )
})
