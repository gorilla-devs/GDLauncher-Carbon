import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import { For } from "solid-js"
const c = vi.hoisted(() => ({
  route: null as any,
  tasks: null as any,
  global: null as any,
  actions: new Map<Element, any>(),
  queries: [] as any[],
  navigate: vi.fn(),
  mutation: vi.fn(),
  modal: vi.fn(),
  layout: null as any,
  events: [] as string[]
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/task26/ui")
  return {
    ...ui,
    Button: (p: any) => {
      const el = ui.Button(p)
      c.actions.set(el as Element, p.onClick)
      return el
    },
    Progress: (p: any) => <div data-progress={p.value} />,
    ContextMenuPortal: ui.Wrapper,
    ContextMenuSub: ui.Wrapper,
    ContextMenuSubTrigger: ui.Wrapper,
    ContextMenuSubContent: ui.Wrapper,
    toast: { success: vi.fn(), error: vi.fn() }
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => ({ id: "7" }),
  useLocation: () => ({ pathname: "/library/server/7/addons" })
}))
vi.mock("./server.data", () => ({ default: () => c.route }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { mutation: vi.fn() },
      queryClient: { invalidateQueries: vi.fn() }
    }),
    createQuery: (opts: any) => {
      c.queries.push(opts)
      return {
        get data() {
          return c.tasks[opts().queryKey[1]]
        },
        isSuccess: true,
        isLoading: false,
        isError: false,
        refetch: vi.fn()
      }
    },
    createMutation: (opts: any) => ({
      mutate: (v: any) => {
        c.events.push(opts().mutationKey[0])
        c.mutation(opts().mutationKey[0], v)
      }
    })
  }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({
    openModal: (...args: any[]) => c.modal(...args),
    closeModal: () => c.events.push("close")
  })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({
    navigate: (p: string) => {
      c.events.push(p)
      c.navigate(p)
    }
  })
}))
vi.mock("@/utils/instances", () => ({
  getServerImageUrl: () => "/fixture-image",
  getInstanceImageUrl: () => "/fixture-image"
}))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "/fixture-image" }))
vi.mock("@/pages/Library/shared/useEntityGoneGuard", () => ({
  useEntityGoneGuard: vi.fn()
}))
vi.mock("@/pages/Library/shared/DetailPageLayout", () => ({
  default: (p: any) => {
    c.layout = p
    return (
      <div>
        {p.headerInfoContent}
        {p.headerActions}
        {p.stickyRightButton}
        <For each={p.tabs}>
          {(tab) => (
            <button onClick={() => p.onTabClick(tab)}>{tab.label}</button>
          )}
        </For>
      </div>
    )
  }
}))
vi.mock("./Tabs/ConsoleTab", () => ({ isConsoleFullScreen: () => false }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.global
}))
vi.mock("@/components/InstanceTile", () => ({ setClickedInstanceId: vi.fn() }))
vi.mock("@/managers/ModalsManager/modals/InstanceExport", () => ({
  setExportStep: vi.fn(),
  setPayload: vi.fn()
}))
vi.mock(
  "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent",
  () => ({ setCheckedFiles: vi.fn() })
)
vi.mock("@/components/SearchInputContext", () => ({ default: () => ({}) }))
vi.mock("@/components/GdlFeatureContextMenuItem", () => ({
  default: () => null
}))
import Server from "./index"
import Metrics from "./Metrics"
import Repair from "@/managers/ModalsManager/modals/RepairModpack"
import { FloatingFavoritesBar } from "../components/FloatingFavoritesBar"
import { DragProvider } from "../DragContext"
let root: HTMLDivElement,
  dispose: (() => void) | undefined,
  setRoute: any,
  setTask: any,
  setGlobal: any
const details = {
  id: 7,
  name: "Fixture",
  favorite: true,
  state: { status: "stopped", failed_task: null },
  modpackInfo: { platform: "modrinth" },
  modloaderType: null,
  gameVersion: "1.21",
  port: 25565
}
beforeEach(() => {
  vi.clearAllMocks()
  c.queries = []
  c.events = []
  c.actions.clear()
  ;[c.route, setRoute] = createStore<any>({
    serverDetails: { data: details },
    allServers: { data: [details], isFetching: false }
  })
  ;[c.tasks, setTask] = createStore<any>({})
  ;[c.global, setGlobal] = createStore<any>({
    servers: { data: [details] },
    instances: { data: [] }
  })
  HTMLElement.prototype.animate = vi.fn(() => ({ cancel: vi.fn() })) as any
  root = document.createElement("div")
  document.body.append(root)
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn(() => 1)
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
  vi.unstubAllGlobals()
})
function buttons() {
  return [...root.querySelectorAll<HTMLButtonElement>("button")].filter(
    (el) =>
      c.actions.get(el) &&
      !el.querySelector(".i-hugeicons\\:star") &&
      !el.querySelector(".i-hugeicons\\:refresh")
  )
}
function invoke() {
  buttons().forEach((el) => c.actions.get(el)())
}
const failed = {
  id: 91,
  progress: {
    type: "Failed",
    value: { cause: [{ display: "Synthetic repair failed" }] }
  },
  active_subtasks: [],
  downloaded: "2",
  download_total: "4"
}
test("ordinary Repair confirmation leaves detail for Library before reinstall admission", () => {
  dispose = render(
    () => <Repair title="Repair" data={{ id: 7, isServer: true }} />,
    root
  )
  const confirm = [...root.querySelectorAll("button")].find((el) =>
    el.textContent?.includes("_trn_repair_confirm")
  )!
  confirm.click()
  expect(c.events).toEqual([
    "close",
    "/library?mode=servers",
    "server.reinstallServer"
  ])
  expect(c.mutation).toHaveBeenCalledWith("server.reinstallServer", 7)
})
test("direct reactive Installing details show owned progress; failure alone never enables lifecycle; terminal stopped success/failure remains actionable", () => {
  dispose = render(() => <Server />, root)
  setRoute("serverDetails", "data", "state", {
    status: "installing",
    task_id: 91
  })
  setTask(91, { ...failed, progress: { type: "Known", value: 0.4 } })
  expect(root.textContent).toContain("_trn_server_task_install")
  expect(root.querySelector('[data-progress="40"]')).not.toBeNull()
  expect(buttons()).toHaveLength(2)
  expect(buttons().every((el) => el.disabled)).toBe(true)
  invoke()
  expect(c.mutation).not.toHaveBeenCalled()
  setTask(91, failed)
  expect(root.textContent).toContain("Synthetic repair failed")
  invoke()
  expect(c.mutation).not.toHaveBeenCalled()
  setRoute("serverDetails", "data", "state", {
    status: "stopped",
    failed_task: 91
  })
  expect(root.textContent).toContain("Synthetic repair failed")
  expect(buttons().every((el) => !el.disabled)).toBe(true)
  invoke()
  expect(c.mutation).toHaveBeenCalledTimes(2)
  setRoute("serverDetails", "data", "state", {
    status: "stopped",
    failed_task: null
  })
  expect(root.textContent).not.toContain("Synthetic repair failed")
  expect(root.textContent).toContain("_trn_server_action_start")
  expect(c.queries.some((opts) => opts().queryKey[0] === "vtask.getTask")).toBe(
    true
  )
})
test.each(["starting", "stopping", "deleting", "missing"])(
  "%s presentation and direct handlers cannot dispatch lifecycle actions",
  (status) => {
    dispose = render(() => <Server />, root)
    if (status === "missing") setRoute("serverDetails", "data", undefined)
    else setRoute("serverDetails", "data", "state", { status })
    expect(buttons()).toHaveLength(2)
    expect(buttons().every((el) => el.disabled)).toBe(true)
    invoke()
    expect(c.mutation).not.toHaveBeenCalled()
  }
)
test.each(["stopped", "running"])(
  "%s details permit the correct lifecycle action and preserve vanilla Addons",
  (status) => {
    setRoute("serverDetails", "data", "state", { status, failed_task: null })
    dispose = render(() => <Server />, root)
    invoke()
    expect(c.mutation.mock.calls.map((call) => call[0])).toEqual([
      status === "stopped" ? "server.startServer" : "server.stopServer",
      status === "stopped" ? "server.startServer" : "server.stopServer"
    ])
    expect(root.textContent).toContain("_trn_server_tab_addons")
  }
)
test.each([
  [
    { status: "recoveredRunning", process_id: 4242, graceful_stop: true },
    "stop"
  ],
  [
    { status: "recoveredRunning", process_id: 4242, graceful_stop: false },
    "forceStop"
  ],
  [{ status: "recoveredUnverified", process_id: 4242 }, "clearRecord"],
  [{ status: "recoveredUnverified", process_id: null }, "clearRecord"]
] as const)(
  "a server left by an earlier session (%o) routes both lifecycle buttons to %s, never Start",
  (state, action) => {
    setRoute("serverDetails", "data", "state", state)
    dispose = render(() => <Server />, root)
    expect(buttons()).toHaveLength(2)
    expect(buttons().every((el) => !el.disabled)).toBe(true)
    invoke()
    const mutations = c.mutation.mock.calls.map((call) => call[0])
    if (action === "stop") {
      // macOS and Linux: the server saves and stops, nothing to confirm.
      expect(mutations).toEqual([
        "server.stopRecoveredServer",
        "server.stopRecoveredServer"
      ])
      expect(c.modal).not.toHaveBeenCalled()
    } else {
      // A Force stop or clearing the record is confirmed first.
      expect(mutations).toEqual([])
      expect(c.modal).toHaveBeenCalledTimes(2)
      expect(c.modal).toHaveBeenCalledWith(
        { name: "recoveredServerProcess" },
        {
          id: 7,
          name: "Fixture",
          action,
          processId: state.process_id
        }
      )
    }
    expect(root.textContent).toContain(
      action === "clearRecord"
        ? "_trn_server_action_clear_record"
        : action === "forceStop"
          ? "_trn_server_action_force_stop"
          : "_trn_server_action_stop"
    )
    // Windows offers only Force stop, so its notice also names the in-game
    // /stop, which saves first.
    const notice = root.querySelector(
      '[data-testid="server-recovered-notice"]'
    )!.textContent
    expect(notice).toBe(
      action === "forceStop"
        ? "instances:_trn_server_recovered_running_notice_force"
        : action === "stop"
          ? "instances:_trn_server_recovered_running_notice"
          : state.process_id === null
            ? "instances:_trn_server_recovered_unreadable_notice"
            : "instances:_trn_server_recovered_unverified_notice"
    )
    // Clear record is not the trash icon Delete uses.
    expect(root.querySelector(".i-hugeicons\\:delete-02")).toBeNull()
  }
)
test("a server running from an earlier session explains its missing metrics instead of offering Start", () => {
  dispose = render(
    () => <Metrics serverId={7} isRunning={false} isRecovered xmx={2048} />,
    root
  )
  expect(root.textContent).toContain("_trn_server_metrics_recovered_hint")
  expect(root.textContent).not.toContain("_trn_server_metrics_start_hint")
})
test.each([
  [
    { status: "recoveredRunning", process_id: 4242, graceful_stop: true },
    "_trn_server_status_recovered_running",
    "stop",
    "stop",
    "_trn_stop"
  ],
  [
    { status: "recoveredRunning", process_id: 4242, graceful_stop: false },
    "_trn_server_status_recovered_running",
    "stop",
    "forceStop",
    "_trn_server_action_force_stop"
  ],
  [
    { status: "recoveredUnverified", process_id: 4242 },
    "_trn_server_status_recovered_unverified",
    "eraser",
    "clearRecord",
    "_trn_server_action_clear_record"
  ]
] as const)(
  "the Favorites entry of a server left by an earlier session (%o) shows its translated status, and its Stop and menu item act as the tile does",
  (state, label, icon, action, item) => {
    setGlobal("servers", "data", 0, "state", state)
    dispose = render(
      () => (
        <DragProvider>
          <FloatingFavoritesBar
            favoriteIds={[7]}
            isSelectionActive={false}
            libraryMode="servers"
          />
        </DragProvider>
      ),
      root
    )
    const status = document.querySelector('[data-testid="dock-server-status"]')!
    expect(status.textContent).toBe(`instances:${label}`)
    // The translated status keeps its own case ("Running from a previous
    // session", not "Running From A Previous Session").
    expect(status.classList.contains("capitalize")).toBe(false)
    const avatar = document.querySelector<HTMLElement>(
      ".group.relative.h-12.w-12"
    )!
    const overlay = avatar.querySelector<HTMLElement>("div.absolute.inset-0")!
    expect(overlay.querySelector(`.i-hugeicons\\:${icon}`)).not.toBeNull()
    const menuItem = [...document.querySelectorAll("button")].find((el) =>
      el.textContent?.includes(`instances:${item}`)
    )!
    expect(menuItem).toBeDefined()
    overlay.click()
    menuItem.click()
    if (action === "stop") {
      // macOS and Linux: the server saves and stops, nothing to confirm.
      expect(c.mutation.mock.calls).toEqual([
        ["server.stopRecoveredServer", 7],
        ["server.stopRecoveredServer", 7]
      ])
      expect(c.modal).not.toHaveBeenCalled()
    } else {
      // A Force stop or clearing the record is confirmed first.
      expect(c.mutation).not.toHaveBeenCalled()
      expect(c.modal).toHaveBeenCalledTimes(2)
      expect(c.modal).toHaveBeenCalledWith(
        { name: "recoveredServerProcess" },
        { id: 7, name: "Fixture", action, processId: 4242 }
      )
    }
    expect(c.navigate).not.toHaveBeenCalled()
  }
)
test.each([
  [{ status: "stopped", failed_task: null }, "_trn_server_status_stopped"],
  [{ status: "installing", task_id: 91 }, "_trn_server_status_installing"],
  [{ status: "starting" }, "_trn_server_status_starting"],
  [
    { status: "running", uptime_seconds: 5, process_id: 4242 },
    "_trn_server_status_running"
  ],
  [{ status: "stopping" }, "_trn_server_status_stopping"],
  [{ status: "deleting" }, "_trn_server_status_deleting"]
] as const)(
  "the Favorites entry of a server (%o) shows its translated status, never the raw one",
  (state, label) => {
    setGlobal("servers", "data", 0, "state", state)
    dispose = render(
      () => (
        <DragProvider>
          <FloatingFavoritesBar
            favoriteIds={[7]}
            isSelectionActive={false}
            libraryMode="servers"
          />
        </DragProvider>
      ),
      root
    )
    const status = document.querySelector('[data-testid="dock-server-status"]')!
    expect(status.textContent).toBe(`instances:${label}`)
  }
)
test("Favorites Installing entry and play callbacks refuse, then stopped entry works", () => {
  setGlobal("servers", "data", 0, "state", {
    status: "installing",
    task_id: 91
  })
  dispose = render(
    () => (
      <DragProvider>
        <FloatingFavoritesBar
          favoriteIds={[7]}
          isSelectionActive={false}
          libraryMode="servers"
        />
      </DragProvider>
    ),
    root
  )
  const avatar = document.querySelector<HTMLElement>(
    ".group.relative.h-12.w-12"
  )!
  expect(avatar).not.toBeNull()
  avatar.click()
  expect(c.navigate).not.toHaveBeenCalled()
  const overlay = avatar.querySelector<HTMLElement>("div.absolute.inset-0")!
  overlay.click()
  expect(c.mutation).not.toHaveBeenCalled()
  setGlobal("servers", "data", 0, "state", {
    status: "stopped",
    failed_task: null
  })
  avatar.click()
  expect(c.navigate).toHaveBeenCalledWith("/library/server/7")
})

test("unknown task progress remains Installing and absent/stale/failed task responses never grant Start", () => {
  setRoute("serverDetails", "data", "state", {
    status: "installing",
    task_id: 91
  })
  dispose = render(() => <Server />, root)
  for (const task of [
    null,
    { ...failed, id: 90 },
    { ...failed, progress: { type: "Indeterminate" } }
  ]) {
    setTask(91, task)
    expect(root.textContent).toContain("_trn_server_task_install")
    expect(buttons().every((el) => el.disabled)).toBe(true)
    invoke()
  }
  expect(c.mutation).not.toHaveBeenCalled()
  expect(
    c.queries.find((opts) => opts().queryKey[0] === "vtask.getTask")()
      .refetchInterval
  ).toBe(500)
  setRoute("serverDetails", "data", "state", {
    status: "stopped",
    failed_task: null
  })
  expect(
    c.queries.find((opts) => opts().queryKey[0] === "vtask.getTask")().enabled
  ).toBe(false)
})
