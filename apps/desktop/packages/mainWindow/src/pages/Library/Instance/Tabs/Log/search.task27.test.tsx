import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { For } from "solid-js"
import { createStore } from "solid-js/store"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"
import { LogEntryLevel, LogEntrySourceKind } from "@/utils/logs"
const c = vi.hoisted(() => ({
  sidebar: null as any,
  entries: null as any,
  snapshots: {} as Record<number, any[]>,
  calls: [] as any[],
  deferred: [] as any[],
  defer: false,
  aborted: 0
}))
vi.mock("@solidjs/router", () => ({ useParams: () => ({ id: "1" }) }))
vi.mock("@/utils/rspcClient.js", async () => {
  const { createQuery } = await import("@tanstack/solid-query")
  return {
    apiWsUrl: (p: string) => `ws://fixture.invalid${p}`,
    rspc: {
      createQuery: (opts: any) => {
        if (opts().queryKey[0] === "instance.getLogs")
          return {
            get data() {
              return c.entries
            }
          }
        if (opts().queryKey[0] === "instance.getInstanceDetails")
          return { data: { state: { state: "inactive" } } }
        return createQuery(() => ({
          ...opts(),
          queryFn: ({ queryKey, signal }: any) => {
            const arg = queryKey[1]
            c.calls.push({ ...arg })
            signal.addEventListener("abort", () => c.aborted++)
            const results = (c.snapshots[arg.log_id] ?? []).flatMap(
              (line, index) => {
                const pos = line.message.indexOf(arg.query)
                return pos >= 0
                  ? [{ entry_index: index, pos, len: arg.query.length }]
                  : []
              }
            )
            if (c.defer)
              return new Promise((resolve) =>
                c.deferred.push(() => resolve(results))
              )
            return Promise.resolve(results)
          }
        }))
      }
    }
  }
})
vi.mock("./LogsSidebar", () => ({
  default: (p: any) => {
    c.sidebar = p
    return <div />
  }
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/task26/ui")
  return {
    ...ui,
    Input: (p: any) => (
      <div>
        {p.icon}
        {ui.Input(p)}
      </div>
    ),
    PopoverTrigger: (p: any) => (
      <div data-finder-toggle onClick={p.onClick}>
        {p.children}
      </div>
    )
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("./components/FullscreenToggle", () => ({ default: () => null }))
vi.mock("./components/LogsOptions", () => ({ default: () => null }))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => {
    p.ref?.({
      scrollToIndex: vi.fn(),
      viewportSize: 100,
      scrollSize: 100,
      scrollOffset: 0
    })
    return <For each={p.data}>{(line, index) => p.children(line, index)}</For>
  }
}))
import Logs from "."
class Socket {
  static sockets: Socket[] = []
  CONNECTING = 0
  OPEN = 1
  readyState = 0
  onmessage: any
  close = vi.fn()
  constructor(public url: string) {
    Socket.sockets.push(this)
  }
  message(text: string) {
    const id = Number(this.url.split("=").at(-1))
    const entry = {
      message: text,
      level: LogEntryLevel.Info,
      sourceKind: LogEntrySourceKind._StdOut,
      logger: "N/A",
      thread: "N/A",
      timestamp: 0
    }
    c.snapshots[id].push(entry)
    this.onmessage?.({ data: JSON.stringify([entry]) })
  }
}
let root: HTMLDivElement,
  dispose: (() => void) | undefined,
  client: QueryClient,
  setEntries: any
async function flush() {
  for (let i = 0; i < 10; i++) await Promise.resolve()
  await vi.advanceTimersByTimeAsync(0)
}
async function advance(ms = 250) {
  await vi.advanceTimersByTimeAsync(ms)
  await flush()
}
function toggle() {
  root.querySelector<HTMLElement>("[data-finder-toggle]")!.click()
}
async function search() {
  toggle()
  const input = root.querySelector("input")!
  input.value = "ERROR"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  await flush()
}
beforeEach(() => {
  vi.useFakeTimers()
  c.calls = []
  c.deferred = []
  c.defer = false
  c.aborted = 0
  c.snapshots = { 1: [], 2: [] }
  Socket.sockets = []
  const [entries, set] = createStore<any[]>([
    { id: 1, active: true },
    { id: 2, active: false }
  ])
  c.entries = entries
  setEntries = set
  vi.stubGlobal("WebSocket", Socket)
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn(() => 1)
  )
  vi.stubGlobal("cancelAnimationFrame", vi.fn())
  root = document.createElement("div")
  document.body.append(root)
  client = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: Infinity,
        refetchOnWindowFocus: false,
        gcTime: Infinity
      }
    }
  })
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <Logs />
      </QueryClientProvider>
    ),
    root
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  client.clear()
  root.remove()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})
test("matching live growth updates actual finder count and mounted highlight without changing query key", async () => {
  await search()
  await advance()
  expect(root.textContent).toContain("_trn_no_results")
  const before = c.calls.length
  const keysBefore = client
    .getQueryCache()
    .findAll()
    .map((q) => q.queryKey)
  Socket.sockets[0].message("new ERROR line")
  expect(root.textContent).toContain("new ERROR line")
  await advance()
  expect(root.textContent).toContain("1 of 1")
  expect(root.querySelector('[id="finder-result-0"]')?.textContent).toBe(
    "ERROR"
  )
  expect(c.calls.length).toBeGreaterThan(before)
  expect(
    client
      .getQueryCache()
      .findAll()
      .map((q) => q.queryKey)
  ).toEqual(keysBefore)
})
test("continuous growth gets a bounded first refresh; deferred work is never repeatedly cancelled and gets one dirty follow-up", async () => {
  await search()
  await advance()
  const before = c.calls.length
  for (let i = 0; i < 5; i++) {
    Socket.sockets[0].message(`ERROR ${i}`)
    await advance(60)
  }
  expect(c.calls.length).toBeGreaterThan(before)
  expect(c.calls.length - before).toBeLessThanOrEqual(2)
  await advance()
  c.defer = true
  Socket.sockets[0].message("ERROR deferred")
  await advance()
  const during = c.calls.length
  for (let i = 0; i < 8; i++) {
    Socket.sockets[0].message(`ERROR pending ${i}`)
    await advance(100)
  }
  expect(c.calls.length).toBe(during)
  expect(c.aborted).toBe(0)
  expect(c.deferred).toHaveLength(1)
  c.defer = false
  c.deferred.shift()()
  await flush()
  await advance()
  expect(c.calls.length).toBe(during + 1)
  expect(root.textContent).toContain("1 of 14")
})
test.each(["hide", "stop", "session", "query", "dispose"])(
  "%s revokes queued growth refresh",
  async (mode) => {
    await search()
    await advance()
    Socket.sockets[0].message("ERROR pending")
    const before = c.calls.length
    if (mode === "hide") toggle()
    else if (mode === "stop") setEntries(0, "active", false)
    else if (mode === "session") c.sidebar.setSelectedLog(2)
    else if (mode === "query") {
      const input = root.querySelector("input")!
      input.value = ""
      input.dispatchEvent(new Event("input", { bubbles: true }))
    } else {
      dispose!()
      dispose = undefined
    }
    await advance(1000)
    expect(
      c.calls
        .slice(before)
        .every(
          (call) =>
            (mode === "session" && call.log_id === 2) ||
            (mode === "stop" && call.log_id === 1)
        )
    ).toBe(true)
    if (mode === "stop") expect(root.textContent).toContain("1 of 1")
  }
)
test("old socket is inert after session switch; finite and empty searches have no growth timers; opening gets current snapshot", async () => {
  await search()
  await advance()
  const late = Socket.sockets[0].onmessage
  c.sidebar.setSelectedLog(2)
  await flush()
  await advance()
  const before = c.calls.length
  late({ data: JSON.stringify([{ message: "obsolete ERROR" }]) })
  await advance()
  expect(c.calls.length).toBe(before)
  expect(root.textContent).not.toContain("obsolete ERROR")
  Socket.sockets[1].message("finite ERROR")
  await advance()
  expect(c.calls.length).toBe(before)
  toggle()
  toggle()
  await flush()
  await advance()
  expect(root.textContent).toContain("1 of 1")
  const input = root.querySelector("input")!
  input.value = ""
  input.dispatchEvent(new Event("input", { bubbles: true }))
  const count = c.calls.length
  Socket.sockets[1].message("ERROR empty")
  await advance()
  expect(c.calls.length).toBe(count)
})

test.each(["hide", "stop", "session", "query", "dispose"])(
  "%s revokes deferred refresh publication",
  async (mode) => {
    await search()
    await advance()
    c.defer = true
    Socket.sockets[0].message("ERROR old")
    await advance()
    expect(c.deferred).toHaveLength(1)
    if (mode === "hide") toggle()
    else if (mode === "stop") setEntries(0, "active", false)
    else if (mode === "session") c.sidebar.setSelectedLog(2)
    else if (mode === "query") {
      const input = root.querySelector("input")!
      input.value = "other"
      input.dispatchEvent(new Event("input", { bubbles: true }))
    } else {
      dispose!()
      dispose = undefined
    }
    if (mode === "stop") Socket.sockets[0].message("ERROR terminal")
    c.defer = false
    c.deferred.shift()()
    await flush()
    await advance(1000)
    if (mode === "stop") {
      expect(root.textContent).toContain("1 of 2")
      expect(root.querySelectorAll('[id^="finder-result-"]')).toHaveLength(2)
      expect(c.aborted).toBe(0)
    } else {
      expect(root.querySelector('[id="finder-result-0"]')).toBeNull()
      if (mode !== "dispose") expect(root.textContent).not.toContain("1 of 1")
    }
  }
)
test("hide and reopen while an old request is pending joins without cancellation then obtains a current snapshot", async () => {
  await search()
  await advance()
  c.defer = true
  Socket.sockets[0].message("ERROR first")
  await advance()
  toggle()
  Socket.sockets[0].message("ERROR second")
  toggle()
  await flush()
  expect(c.deferred).toHaveLength(1)
  expect(c.aborted).toBe(0)
  c.defer = false
  c.deferred.shift()()
  await flush()
  await advance()
  expect(root.textContent).toContain("1 of 2")
})
test("flags select a fresh stable query and stopped logs retain the last authorized results", async () => {
  await search()
  await advance()
  Socket.sockets[0].message("ERROR match")
  await advance()
  expect(root.textContent).toContain("1 of 1")
  const flag = root.querySelector(".i-codicon\\:case-sensitive")!.parentElement!
  flag.click()
  await flush()
  await advance()
  expect(c.calls.at(-1).match_case).toBe(true)
  setEntries(0, "active", false)
  const before = c.calls.length
  await advance(1000)
  expect(c.calls).toHaveLength(before)
  expect(root.textContent).toContain("1 of 1")
})

test("final matching line followed by stop before the batch obtains the finite snapshot", async () => {
  await search()
  await advance()
  expect(root.textContent).toContain("_trn_no_results")
  Socket.sockets[0].message("final ERROR line")
  expect(root.textContent).toContain("final ERROR line")
  setEntries(0, "active", false)
  await advance(1000)
  expect(root.textContent).toContain("1 of 1")
  expect(root.querySelector('[id="finder-result-0"]')?.textContent).toBe(
    "ERROR"
  )
})
