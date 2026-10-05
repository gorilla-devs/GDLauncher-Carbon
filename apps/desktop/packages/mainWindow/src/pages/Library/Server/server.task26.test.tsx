import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createRoot, For } from "solid-js"
import { createStore } from "solid-js/store"
const c = vi.hoisted(() => ({
  params: null as any,
  route: null as any,
  query: null as any,
  sliders: [] as any[],
  actions: new Map<any, any>(),
  mutation: vi.fn(),
  refetch: vi.fn(),
  navigate: vi.fn(),
  layout: null as any,
  inputs: [] as any[],
  realSlider: false
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/task26/ui")
  const { Slider: RealSlider } =
    await import("../../../../../../../../packages/ui/src/Slider")
  return {
    ...ui,
    Slider: (p: any) => {
      c.sliders.push(p)
      return c.realSlider ? <RealSlider {...p} /> : <div />
    },
    Input: (p: any) => {
      c.inputs.push(p)
      return ui.Input(p)
    },
    Button: (p: any) => {
      const el = ui.Button(p)
      c.actions.set(el, p.onClick)
      return el
    }
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => c.params,
  useLocation: () => ({
    get pathname() {
      return `/library/server/${c.params.id}/addons`
    }
  })
}))
vi.mock("./server.data", () => ({ default: () => c.route }))
vi.mock("@/utils/rspcClient", () => ({
  apiWsUrl: (p: string) => `ws://fixture.invalid${p}`,
  rspc: {
    createQuery: (opts: any) => {
      const id = () => opts().queryKey[1]
      return opts().queryKey[0] === "server.getServerDetails"
        ? c.route.serverDetails
        : {
            get data() {
              return c.query[id()]
            },
            refetch: () => c.refetch(id())
          }
    },
    createMutation: (opts: any) => ({
      isPending: false,
      mutate: (v: any) => c.mutation(opts().mutationKey[0], v),
      mutateAsync: (v: any) => c.mutation(opts().mutationKey[0], v)
    })
  }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@/utils/instances", () => ({
  getServerImageUrl: () => "/fixture-image"
}))
vi.mock("@/pages/Library/shared/useEntityGoneGuard", () => ({
  useEntityGoneGuard: vi.fn()
}))
vi.mock("@/pages/Library/shared/DetailPageLayout", () => ({
  default: (p: any) => {
    c.layout = p
    return (
      <div>
        <For each={p.tabs}>
          {(tab) => (
            <button onClick={() => p.onTabClick(tab)}>{tab.label}</button>
          )}
        </For>
      </div>
    )
  }
}))
vi.mock("./Tabs/ConsoleTab", () => ({
  isConsoleFullScreen: () => false,
  setIsConsoleFullScreen: vi.fn()
}))
vi.mock("@/hooks/useKeyboardShortcut", () => ({ default: vi.fn() }))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => (
    <div data-log-list>
      <For each={p.data}>{(line: any) => <div>{line.raw}</div>}</For>
    </div>
  )
}))
import Settings from "./Settings"
import Properties from "./Tabs/PropertiesTab"
import Console from "./Console"
import Server from "./index"
import { useServerAddonMutations } from "./Tabs/Addons/hooks/useServerAddonMutations"
import { serverAllowedAddonTypes } from "@/utils/platformSearch"
let root: HTMLDivElement,
  dispose: (() => void) | undefined,
  setParams: any,
  setQuery: any,
  setRoute: any
const details = {
  id: 7,
  name: "Synthetic",
  xmx: 2048,
  xms: 1024,
  extraJavaArgs: "-XX:test",
  autoRestart: false,
  state: { status: "stopped" },
  gameVersion: "1.21",
  modloaderType: null
}
function deferred() {
  let resolve!: (v?: any) => void, reject!: (e: any) => void
  const promise = new Promise<any>((a, b) => {
    resolve = a
    reject = b
  })
  return { promise, resolve, reject }
}
const tick = async () => {
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
}
function button(key: string) {
  return [...root.querySelectorAll("button")].find((p) =>
    p.textContent?.includes(key)
  )!
}
function motd() {
  return [...root.querySelectorAll("input")].find(
    (el) =>
      el.value === "Old" || el.value.startsWith("Saved") || el.value === "Later"
  )!
}
function edit(el: HTMLInputElement, value: string) {
  el.value = value
  el.dispatchEvent(new Event("input", { bubbles: true }))
}
function save() {
  return c.actions.get(button("_trn_server_properties_save"))() as Promise<any>
}
beforeEach(() => {
  vi.clearAllMocks()
  c.sliders = []
  c.inputs = []
  c.realSlider = false
  c.actions.clear()
  c.mutation.mockResolvedValue(undefined)
  ;[c.params, setParams] = createStore({ id: "7" })
  ;[c.query, setQuery] = createStore<any>({
    7: { motd: "Old", pvp: "true" },
    8: { motd: "Server B", pvp: "false" }
  })
  ;[c.route, setRoute] = createStore<any>({
    serverDetails: { data: { ...details } },
    allServers: { data: [details], isFetching: false }
  })
  c.refetch.mockImplementation(async (id) => ({ data: c.query[id] }))
  root = document.createElement("div")
  document.body.append(root)
  vi.stubGlobal("requestAnimationFrame", (fn: any) => {
    fn(0)
    return 1
  })
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})
test("GPT0116 Slider onChange then release persists the displayed clamped heap pair", () => {
  dispose = render(
    () => (
      <Settings
        serverDetails={c.route.serverDetails.data}
        totalRam={undefined}
      />
    ),
    root
  )
  c.sliders[0].onChange(512)
  c.sliders[0].OnRelease(512)
  expect(c.sliders.map((p) => p.value)).toEqual([512, 512])
  expect(c.mutation).toHaveBeenLastCalledWith(
    "server.updateServer",
    expect.objectContaining({ xmx: 512, xms: 512 })
  )
  expect(c.mutation.mock.calls.at(-1)![1]).not.toHaveProperty("extraJavaArgs")
})
test("heap increase preserves local recent minimum despite stale details and args remain omitted", () => {
  dispose = render(
    () => (
      <Settings
        serverDetails={c.route.serverDetails.data}
        totalRam={undefined}
      />
    ),
    root
  )
  c.sliders[1].onChange(768)
  c.sliders[1].OnRelease(768)
  c.sliders[0].onChange(3072)
  c.sliders[0].OnRelease(3072)
  expect(c.mutation).toHaveBeenLastCalledWith(
    "server.updateServer",
    expect.objectContaining({ xmx: 3072, xms: 768 })
  )
  expect(c.mutation.mock.calls.at(-1)![1]).not.toHaveProperty("extraJavaArgs")
})
test("Properties locks every field and Reset through mutation and awaited refetch, blocking overlap", async () => {
  const mutation = deferred(),
    refresh = deferred()
  c.mutation.mockReturnValue(mutation.promise)
  c.refetch.mockReturnValue(refresh.promise)
  dispose = render(() => <Properties />, root)
  edit(motd(), "Saved A")
  const pending = save()
  expect(
    [...root.querySelectorAll("input,select")]
      .filter((p: any) => p.value !== "")
      .every((p: any) => p.disabled)
  ).toBe(true)
  expect(button("_trn_server_properties_reset").disabled).toBe(true)
  const current = c.inputs.find((p) => p.value === "Saved A")
  current.onInput({ currentTarget: { value: "Later" } })
  expect(current.value).toBe("Saved A")
  await save()
  expect(c.mutation).toHaveBeenCalledTimes(1)
  mutation.resolve()
  await tick()
  expect(button("_trn_server_properties_reset").disabled).toBe(true)
  setQuery(7, { motd: "Saved A", pvp: "true" })
  refresh.resolve({ data: { motd: "Saved A", pvp: "true" } })
  await pending
  expect(motd().value).toBe("Saved A")
  expect(button("_trn_server_properties_save")).toBeUndefined()
  expect(c.mutation).toHaveBeenCalledWith("server.updateServerProperties", {
    id: 7,
    properties: { motd: "Saved A" }
  })
})
test("dirty unsolicited refresh does not replace draft; Reset uses acknowledged baseline", () => {
  dispose = render(() => <Properties />, root)
  edit(motd(), "Saved A")
  setQuery(7, { motd: "Outside", pvp: "false" })
  expect(
    [...root.querySelectorAll("input")].some((el) => el.value === "Saved A")
  ).toBe(true)
  expect(button("_trn_server_properties_save")).toBeDefined()
  button("_trn_server_properties_reset").click()
  expect(
    [...root.querySelectorAll("input")].some((el) => el.value === "Old")
  ).toBe(true)
})
test.each(["mutation", "refetch"])(
  "%s failure preserves dirty draft and unlocks editing",
  async (phase) => {
    dispose = render(() => <Properties />, root)
    edit(motd(), "Saved A")
    if (phase === "mutation")
      c.mutation.mockRejectedValue(new Error("synthetic mutation failure"))
    else c.refetch.mockRejectedValue(new Error("synthetic refetch failure"))
    await expect(save()).rejects.toThrow("synthetic")
    expect(motd().value).toBe("Saved A")
    expect(motd().disabled).toBe(false)
    expect(button("_trn_server_properties_save")).toBeDefined()
  }
)
test("late A save cannot overwrite B or unlock B pending work", async () => {
  const a = deferred(),
    b = deferred()
  c.mutation.mockImplementation((_key, v) =>
    v.id === 7 ? a.promise : b.promise
  )
  dispose = render(() => <Properties />, root)
  edit(motd(), "Saved A")
  const pendingA = save()
  setParams("id", "8")
  await tick()
  const inputB = [...root.querySelectorAll("input")].find(
    (el) => el.value === "Server B"
  )!
  expect(inputB).toBeDefined()
  edit(inputB, "Saved B")
  const pendingB = save()
  a.resolve()
  await pendingA
  expect(motd().value).toBe("Saved B")
  expect(button("_trn_server_properties_reset").disabled).toBe(true)
  expect(c.refetch).not.toHaveBeenCalledWith(8)
  setQuery(8, { motd: "Saved B", pvp: "false" })
  b.resolve()
  await pendingB
  expect(motd().value).toBe("Saved B")
  expect(button("_trn_server_properties_save")).toBeUndefined()
})

test("vanilla ordinary navigation exposes Addons and the actual add action is datapack-only", () => {
  dispose = render(() => <Server />, root)
  expect(button("_trn_server_tab_addons")).toBeDefined()
  button("_trn_server_tab_addons").click()
  expect(c.navigate).toHaveBeenCalledWith("/library/server/7/addons")
  expect(c.layout.activeTabId).toBe("addons")
  let end!: () => void
  const hooks = createRoot((d) => {
    end = d
    return useServerAddonMutations(vi.fn(), {} as any, vi.fn())
  })
  hooks.gotoSearchPage()
  end()
  expect(c.navigate).toHaveBeenLastCalledWith("/search/datapack?serverId=7")
  expect(serverAllowedAddonTypes(false)).toEqual(["datapack"])
  setRoute("serverDetails", "data", "modloaderType", "fabric")
  expect(button("_trn_server_tab_addons")).toBeDefined()
  expect(serverAllowedAddonTypes(true)).toEqual(["mod", "datapack"])
})
class Socket {
  static sockets: Socket[] = []
  OPEN = 1
  CONNECTING = 0
  readyState = 1
  onmessage: any
  onerror: any
  onclose: any
  close = vi.fn()
  constructor(public url: string) {
    Socket.sockets.push(this)
  }
  message(v: any) {
    this.onmessage?.({ data: JSON.stringify(v) })
  }
}
function consoleMount(running = true) {
  const [state, set] = createStore({ serverId: 7, isRunning: running })
  Socket.sockets = []
  vi.stubGlobal("WebSocket", Socket)
  dispose = render(() => <Console {...state} />, root)
  return { set, ws: Socket.sockets[0] }
}
test("GPT0117 genuine crash lines stay visible after stop with command admission disabled", () => {
  const { set, ws } = consoleMount()
  ws.message(["[Error] real JVM error", "bad configuration", "exiting"])
  set("isRunning", false)
  expect(root.querySelector("[data-log-list]")?.textContent).toContain(
    "bad configuration"
  )
  expect(Socket.sockets).toHaveLength(1)
  const command = root.querySelector<HTMLInputElement>(
    'input[placeholder*="_trn_start_server_to_send"]'
  )!
  command.value = "say denied"
  command.dispatchEvent(new Event("input", { bubbles: true }))
  command.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
  )
  expect(c.mutation).not.toHaveBeenCalled()
  expect(
    root.querySelectorAll("input")[root.querySelectorAll("input").length - 1]
      .disabled
  ).toBe(true)
})
test("unopened stopped transport errors keep the true empty state; running empty waits", () => {
  const { set, ws } = consoleMount(false)
  ws.message({ error: "no session" })
  ws.onerror()
  expect(root.textContent).toContain("_trn_server_not_running")
  expect(root.querySelector("[data-log-list]")).toBeNull()
  ws.message([])
  set("isRunning", true)
  expect(root.textContent).toContain("_trn_waiting_for_output")
  ws.message("output")
  expect(root.querySelector("[data-log-list]")?.textContent).toContain("output")
})
test("fresh snapshot resets provenance and obsolete identity/reconnect callbacks cannot seed new logs", () => {
  vi.useFakeTimers()
  const { set, ws } = consoleMount()
  ws.message(["old output"])
  const old = ws.onmessage
  ws.onclose()
  vi.advanceTimersByTime(1000)
  const current = Socket.sockets[1]
  current.message([])
  old({ data: JSON.stringify(["obsolete"]) })
  set("isRunning", false)
  expect(root.textContent).toContain("_trn_server_not_running")
  current.message("new output")
  expect(root.querySelector("[data-log-list]")?.textContent).toContain(
    "new output"
  )
  const obsolete = current.onmessage
  set("serverId", 8)
  obsolete({ data: JSON.stringify(["old server"]) })
  expect(root.querySelector("[data-log-list]")).toBeNull()
  expect(root.textContent).toContain("_trn_server_not_running")
})

test("actual Slider document mouseup invokes the Settings clamp before persistence", () => {
  c.realSlider = true
  dispose = render(
    () => (
      <Settings
        serverDetails={c.route.serverDetails.data}
        totalRam={undefined}
      />
    ),
    root
  )
  const track = root.querySelector<HTMLDivElement>(
    "div.absolute.z-10.cursor-pointer"
  )!
  Object.defineProperty(track, "clientWidth", { value: 100 })
  track.getBoundingClientRect = () => new DOMRect(10, 0, 100, 10)
  track.dispatchEvent(
    new MouseEvent("mousedown", { clientX: 10, bubbles: true })
  )
  document.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }))
  expect(c.sliders.map((p) => p.value)).toEqual([512, 512])
  expect(c.mutation).toHaveBeenLastCalledWith(
    "server.updateServer",
    expect.objectContaining({ xmx: 512, xms: 512 })
  )
})
test("resolved failed refetch preserves dirty submitted draft and unlocks", async () => {
  dispose = render(() => <Properties />, root)
  edit(motd(), "Saved A")
  c.refetch.mockResolvedValue({
    isError: true,
    error: new Error("synthetic query failure"),
    data: { motd: "Old", pvp: "true" }
  })
  await expect(save()).rejects.toThrow("synthetic query failure")
  expect(motd().value).toBe("Saved A")
  expect(motd().disabled).toBe(false)
})
