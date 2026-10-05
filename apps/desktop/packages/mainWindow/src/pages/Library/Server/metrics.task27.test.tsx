import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
vi.mock("@/utils/rspcClient", () => ({
  apiWsUrl: (p: string) => `ws://fixture.invalid${p}`
}))
vi.mock("@gd/ui", () => ({ Progress: () => <div /> }))
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
import Metrics from "./Metrics"
class Socket {
  static sockets: Socket[] = []
  CONNECTING = 0
  OPEN = 1
  readyState = 0
  onmessage: any
  onerror: any
  close = vi.fn()
  constructor(public url: string) {
    Socket.sockets.push(this)
  }
  message(cpu: number) {
    this.onmessage?.({
      data: JSON.stringify({ cpuPercent: cpu, memoryMb: 10, uptimeSeconds: 5 })
    })
  }
}
let root: HTMLDivElement, dispose: (() => void) | undefined
function mount() {
  Socket.sockets = []
  vi.stubGlobal("WebSocket", Socket)
  const [props, set] = createStore({ serverId: 1, isRunning: true, xmx: 1024 })
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Metrics {...props} />, root)
  return { set, a: Socket.sockets[0] }
}
afterEach(() => {
  dispose?.()
  dispose = undefined
  root?.remove()
  vi.unstubAllGlobals()
})
test("CONNECTING A retires before close, identity resets, and queued message/error cannot overwrite B", () => {
  const { set, a } = mount()
  a.message(11)
  const lateMessage = a.onmessage,
    lateError = a.onerror
  a.close.mockImplementation(() => lateMessage({ data: "not-json-obsolete" }))
  set("serverId", 2)
  expect(a.close).toHaveBeenCalledOnce()
  expect(root.textContent).toContain("0.0%")
  Socket.sockets[1].message(42)
  lateMessage({ data: JSON.stringify({ cpuPercent: 99 }) })
  lateError()
  expect(root.textContent).toContain("42.0%")
  expect(a.onmessage).toBeNull()
  expect(a.onerror).toBeNull()
})
test("OPEN retires, stopping resets metrics, and disposal makes queued callbacks inert", () => {
  const { set, a } = mount()
  a.readyState = a.OPEN
  a.message(33)
  const late = a.onmessage
  set("isRunning", false)
  expect(a.close).toHaveBeenCalledOnce()
  set("isRunning", true)
  expect(root.textContent).toContain("0.0%")
  const b = Socket.sockets[1],
    last = b.onmessage
  dispose!()
  dispose = undefined
  expect(b.close).toHaveBeenCalledOnce()
  expect(() => {
    late({ data: "obsolete" })
    last({ data: "obsolete" })
  }).not.toThrow()
})

test("ordinary current errors reset, current messages recover, and a remount has separate metric ownership", () => {
  const { a } = mount()
  a.message(20)
  a.onerror()
  expect(root.textContent).toContain("0.0%")
  a.message(21)
  const queued = a.onmessage
  dispose!()
  dispose = undefined
  root.remove()
  const { a: b } = mount()
  b.message(55)
  queued({ data: JSON.stringify({ cpuPercent: 99 }) })
  expect(root.textContent).toContain("55.0%")
})
