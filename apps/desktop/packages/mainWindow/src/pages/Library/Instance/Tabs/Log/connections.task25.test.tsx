import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"

const control = vi.hoisted(() => ({
  sidebar: null as any,
  content: null as any,
  scroll: vi.fn()
}))
vi.mock("@solidjs/router", () => ({ useParams: () => ({ id: "1" }) }))
vi.mock("@/utils/rspcClient.js", () => ({
  apiWsUrl: (path: string) => `ws://injected.invalid${path}`,
  rspc: {
    createQuery: (options: any) => ({
      data: options().queryKey[0] === "instance.getLogs" ? [] : undefined
    })
  }
}))
vi.mock("./LogsSidebar", () => ({
  default: (props: any) => {
    control.sidebar = props
    return <div />
  }
}))
vi.mock("./LogsContent", () => ({
  default: (props: any) => {
    control.content = props
    props.assignVirtualizerRef({
      scrollToIndex: control.scroll,
      viewportSize: 100,
      scrollSize: 100,
      scrollOffset: 0
    })
    return <div data-testid="rows">{JSON.stringify(props.logs)}</div>
  }
}))
import Logs from "."

class Socket {
  static instances: Socket[] = []
  CONNECTING = 0
  OPEN = 1
  CLOSING = 2
  CLOSED = 3
  readyState = 0
  onmessage: ((event: any) => void) | null = null
  close = vi.fn(() => {
    this.readyState = this.CLOSED
  })
  constructor(public url: string) {
    Socket.instances.push(this)
  }
  message(label: string) {
    this.onmessage?.({ data: JSON.stringify([{ message: label }]) })
  }
}
let root: HTMLDivElement
let dispose: () => void
beforeEach(() => {
  Socket.instances = []
  vi.clearAllMocks()
  vi.stubGlobal("WebSocket", Socket)
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Logs />, root)
})
afterEach(() => {
  dispose?.()
  root.remove()
  vi.unstubAllGlobals()
})

test("superseding CONNECTING revokes queued callbacks before cancellation and keeps only the current session", () => {
  control.sidebar.setSelectedLog(1)
  const a = Socket.instances[0]
  const queuedA = a.onmessage!
  a.close.mockImplementation(() => {
    queuedA({ data: JSON.stringify([{ message: "obsolete-during-close" }]) })
    a.readyState = a.CLOSED
  })
  control.sidebar.setSelectedLog(2)
  expect(a.close).toHaveBeenCalledOnce()
  expect(a.onmessage).toBeNull()
  const b = Socket.instances[1]
  b.readyState = b.OPEN
  b.message("current-B")
  queuedA({ data: JSON.stringify([{ message: "obsolete-A" }]) })
  expect(root.querySelector("[data-testid=rows]")?.textContent).toBe(
    '[{"message":"current-B"}]'
  )
  expect(control.scroll).toHaveBeenCalledTimes(1)
  expect(control.content.isLoading).toBe(false)
  b.message("append-B")
  expect(control.content.logs.map((entry: any) => entry.message)).toEqual([
    "current-B",
    "append-B"
  ])
})
test("OPEN cleanup and disposal also detach and reject queued messages", () => {
  control.sidebar.setSelectedLog(1)
  const a = Socket.instances[0]
  a.readyState = a.OPEN
  const queuedA = a.onmessage!
  control.sidebar.setSelectedLog(2)
  expect(a.close).toHaveBeenCalledOnce()
  const b = Socket.instances[1]
  const queuedB = b.onmessage!
  dispose()
  expect(b.close).toHaveBeenCalledOnce()
  expect(a.onmessage).toBeNull()
  expect(b.onmessage).toBeNull()
  queuedA({ data: JSON.stringify([{ message: "disposed-A" }]) })
  queuedB({ data: JSON.stringify([{ message: "disposed-B" }]) })
  expect(control.content.logs).toHaveLength(0)
  expect(control.scroll).not.toHaveBeenCalled()
})
