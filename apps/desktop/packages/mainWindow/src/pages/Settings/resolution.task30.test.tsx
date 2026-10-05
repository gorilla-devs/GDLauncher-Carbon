import { afterEach, expect, it, vi } from "vitest"
import { createSignal, For } from "solid-js"
import { render } from "solid-js/web"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import General from "./General"

const control = vi.hoisted(() => ({
  route: undefined as any,
  mutation: undefined as any,
  queryClient: undefined as any,
  cancel: vi.fn(async () => {})
}))
vi.mock("@gd/ui", () => {
  const Box = (props: any) => (
    <div>{typeof props.children === "function" ? null : props.children}</div>
  )
  return {
    Button: (props: any) => (
      <button disabled={props.disabled} onClick={props.onClick}>
        {props.children}
      </button>
    ),
    Input: (props: any) => (
      <input type={props.type} value={props.value} onChange={props.onChange} />
    ),
    Select: (props: any) => (
      <select
        value={props.value}
        onChange={(event) => props.onChange(event.currentTarget.value)}
      >
        <For each={props.options}>
          {(value) => <option value={value}>{value}</option>}
        </For>
      </select>
    ),
    Switch: (props: any) => (
      <input
        type="checkbox"
        checked={props.checked}
        onChange={props.onChange}
      />
    ),
    SelectTrigger: Box,
    SelectContent: Box,
    SelectItem: Box,
    SelectValue: Box,
    Skeleton: Box
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("./settings.general.data", () => ({
  default: () => ({ data: control.route })
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (options: any) =>
      options().mutationKey[0] === "settings.setSettings"
        ? control.mutation
        : { mutate: vi.fn() },
    createQuery: (options: any) => ({
      data:
        options().queryKey[0] === "settings.getCacheSizes"
          ? { gdlauncher: 0, minecraft: 0 }
          : false
    }),
    useContext: () => ({
      queryClient: control.queryClient || { cancelQueries: control.cancel }
    })
  }
}))
vi.mock("@/utils/updater", () => ({
  manualCheckForUpdates: vi.fn(),
  isCheckingForUpdates: () => false
}))
vi.mock("@/utils/logos", () => ({ wideLogoUrl: "synthetic" }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))

function deferred() {
  let resolve!: (value?: any) => void
  let reject!: (error: any) => void
  const promise = new Promise<any>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 2))
const custom = (width = 854, height = 480) => ({
  type: "Custom",
  value: [width, height]
})
const standard = (width = 1046, height = 588) => ({
  type: "Standard",
  value: [width, height]
})
const copy = <T,>(value: T): T => JSON.parse(JSON.stringify(value))
const disposers: (() => void)[] = []
let current: ReturnType<typeof fixture> | undefined
afterEach(async () => {
  current?.gates.forEach((gate) => gate.resolve())
  for (let i = 0; i < 4; i++) {
    current?.reads.forEach((gate) => gate.resolve(copy(current!.persisted)))
    await flush()
  }
  disposers.splice(0).forEach((dispose) => dispose())
  control.queryClient?.clear()
  control.queryClient = undefined
  document.body.textContent = ""
  vi.unstubAllGlobals()
  vi.clearAllMocks()
  control.cancel.mockReset().mockResolvedValue(undefined)
})
function fixture(initial: any = custom()) {
  vi.stubGlobal("__APP_VERSION__", "synthetic")
  const base = {
    releaseChannel: "stable",
    concurrentDownloads: 1,
    launcherActionOnGameLaunch: "none"
  }
  const [data, setData] = createSignal({
    ...base,
    gameResolution: copy(initial)
  })
  const f = {
    persisted: copy(initial),
    writes: [] as any[],
    unrelated: [] as any[],
    gates: Array.from({ length: 20 }, deferred),
    reads: [] as ReturnType<typeof deferred>[],
    delayReads: false,
    refresh: (value: any) => setData({ ...base, gameResolution: copy(value) }),
    route: {
      get data() {
        return data()
      },
      refetch: vi.fn(async () => {
        let fresh = copy(f.persisted)
        if (f.delayReads) {
          const gate = deferred()
          f.reads.push(gate)
          fresh = await gate.promise
        }
        setData({ ...base, gameResolution: copy(fresh) })
        return { isSuccess: true, data: data() }
      })
    }
  }
  const save = vi.fn(async (patch: any) => {
    if (!("gameResolution" in patch)) {
      f.unrelated.push(copy(patch))
      return
    }
    const snapshot = copy(patch.gameResolution.Set)
    const index = f.writes.length
    f.writes.push(snapshot)
    // This is the persistence side effect at successful mutation fulfillment,
    // not an already committed save with a delayed response.
    await f.gates[index].promise
    f.persisted = snapshot
  })
  control.route = f.route
  control.mutation = {
    mutateAsync: save,
    mutate: (patch: any) => {
      void save(patch).catch(() => {})
    },
    isPending: false
  }
  current = f
  return f
}
function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(() => <General />, root)
  disposers.push(dispose)
  return { root, dispose }
}
function dimensions(root: HTMLElement) {
  return Array.from(
    root.querySelectorAll<HTMLInputElement>('input[type="number"]')
  )
}
function edit(root: HTMLElement, index: number, value: string) {
  const input = dimensions(root)[index]
  input.value = value
  input.dispatchEvent(new Event("change", { bubbles: true }))
}
function select(root: HTMLElement, value: string) {
  const input = Array.from(root.querySelectorAll("select")).find((element) =>
    Array.from(element.options).some((option) => option.value === "custom")
  )!
  input.value = value
  input.dispatchEvent(new Event("change", { bubbles: true }))
  return input
}

it("0194 rapid width and height intent persists the complete latest tuple without query refresh", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  edit(root, 1, "600")
  f.gates.forEach((gate) => gate.resolve())
  await flush()
  await flush()
  expect(f.persisted).toEqual(custom(1024, 600))
  expect(dimensions(root).map((input) => input.value)).toEqual(["1024", "600"])
})
it("0194 attempted reversed persistence cannot dispatch the second immutable save before the first settles", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  await flush()
  edit(root, 1, "600")
  f.gates[1].resolve()
  await flush()
  expect(f.writes).toEqual([custom(1024, 480)])
  f.gates[0].resolve()
  await flush()
  await flush()
  expect(f.writes).toEqual([custom(1024, 480), custom(1024, 600)])
  expect(f.persisted).toEqual(custom(1024, 600))
})
it("0194 a stale query refresh cannot discard pending width before the height edit", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  f.refresh(custom())
  await flush()
  edit(root, 1, "600")
  f.gates.forEach((gate) => gate.resolve())
  await flush()
  await flush()
  expect(f.persisted).toEqual(custom(1024, 600))
})
it("0194 Custom, Standard and default publish in one intent order", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  select(root, "Standard:1046x588")
  const selector = select(root, "default")
  expect(selector.value).toBe("default")
  expect(dimensions(root)).toHaveLength(0)
  f.gates.forEach((gate) => gate.resolve())
  await flush()
  await flush()
  expect(f.writes).toEqual([custom(1024, 480), standard(), null])
  expect(f.persisted).toBeNull()
})
it("0194 failed first publication does not poison a newer complete save", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  await flush()
  edit(root, 1, "600")
  f.gates[0].reject(new Error("synthetic rejected save"))
  f.gates[1].resolve()
  await flush()
  await flush()
  expect(f.persisted).toEqual(custom(1024, 600))
})
it("0194 failed latest save retains an editable draft and Retry publishes that complete draft", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  await flush()
  f.gates[0].reject(new Error("synthetic rejected save"))
  await flush()
  expect(dimensions(root)[0].value).toBe("1024")
  const retry = Array.from(root.querySelectorAll("button")).find((button) =>
    button.textContent?.includes("retry")
  )
  expect(retry).toBeDefined()
  retry!.click()
  f.gates[1].resolve()
  await flush()
  await flush()
  expect(f.persisted).toEqual(custom(1024, 480))
})
it("0194 navigation shares pending draft and serializes a replacement owner's edit", async () => {
  const f = fixture()
  const first = mount()
  edit(first.root, 0, "1024")
  await flush()
  first.dispose()
  const second = mount()
  expect(dimensions(second.root)[0].value).toBe("1024")
  edit(second.root, 1, "600")
  f.gates[1].resolve()
  await flush()
  expect(f.writes).toHaveLength(1)
  f.gates[0].resolve()
  await flush()
  await flush()
  expect(f.persisted).toEqual(custom(1024, 600))
  expect(dimensions(second.root).map((input) => input.value)).toEqual([
    "1024",
    "600"
  ])
})
it("0194 fresh reconciliation is ordered after persistence and stale query data stays blocked until then", async () => {
  const f = fixture()
  f.delayReads = true
  const { root } = mount()
  edit(root, 0, "1024")
  await flush()
  expect(f.route.refetch).not.toHaveBeenCalled()
  f.gates[0].resolve()
  await flush()
  expect(control.cancel).toHaveBeenCalledWith({
    queryKey: ["settings.getSettings"],
    exact: true
  })
  expect(f.route.refetch).toHaveBeenCalledOnce()
  f.refresh(custom(800, 400))
  await flush()
  expect(dimensions(root)[0].value).toBe("1024")
  f.reads[0].resolve(custom(1025, 481))
  await flush()
  expect(dimensions(root).map((input) => input.value)).toEqual(["1025", "481"])
})
it("0194 disposed owner reconciliation cannot publish over newer owner intent", async () => {
  const f = fixture()
  f.delayReads = true
  const first = mount()
  edit(first.root, 0, "1024")
  f.gates[0].resolve()
  await flush()
  first.dispose()
  const second = mount()
  edit(second.root, 1, "600")
  await flush()
  f.reads[0]?.resolve(custom(1024, 480))
  await flush()
  expect(dimensions(second.root).map((input) => input.value)).toEqual([
    "1024",
    "600"
  ])
  f.gates[1].resolve()
  await flush()
  f.reads.forEach((gate) => gate.resolve(custom(1024, 600)))
  await flush()
  expect(f.persisted).toEqual(custom(1024, 600))
})
it("0194 normal refresh-between-edits and one-coordinate edits retain the other coordinate", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  f.gates[0].resolve()
  await flush()
  f.refresh(custom(1024, 480))
  edit(root, 1, "600")
  f.gates[1].resolve()
  await flush()
  await flush()
  expect(f.persisted).toEqual(custom(1024, 600))
  expect(f.writes[0]).toEqual(custom(1024, 480))
})
it("0194 coherent preset/default/custom controls preserve schema and initial Custom dimensions", async () => {
  const f = fixture(null)
  const { root } = mount()
  expect(dimensions(root)).toHaveLength(0)
  select(root, "custom")
  f.gates[0].resolve()
  await flush()
  expect(dimensions(root).map((input) => input.value)).toEqual(["854", "480"])
  select(root, "Standard:1046x588")
  f.gates[1].resolve()
  await flush()
  expect(f.persisted).toEqual(standard())
})
it("0194 empty invalid and out-of-u16 input never publishes NaN or an invalid tuple", async () => {
  const f = fixture()
  const { root } = mount()
  for (const value of ["", "-1", "65536", "1.5"]) edit(root, 0, value)
  await flush()
  expect(f.writes).toEqual([])
  edit(root, 0, "65535")
  f.gates[0].resolve()
  await flush()
  expect(f.persisted).toEqual(custom(65535, 480))
})
it("0194 unrelated settings writes remain independent of resolution publication", async () => {
  const f = fixture()
  const { root } = mount()
  edit(root, 0, "1024")
  await flush()
  const toggle = root.querySelector<HTMLInputElement>(
    '[data-testid="settings-reduced-motion-toggle"] input'
  )!
  toggle.checked = true
  toggle.dispatchEvent(new Event("change", { bubbles: true }))
  await flush()
  expect(f.unrelated).toEqual([{ reducedMotion: { Set: true } }])
})
it("0194 revoked reconciliation cannot dispatch a stale refetch after query cancellation awaits", async () => {
  const f = fixture()
  const { root } = mount()
  const cancellation = deferred()
  control.cancel.mockImplementationOnce(() => cancellation.promise)
  edit(root, 0, "1024")
  f.gates[0].resolve()
  await flush()
  edit(root, 1, "600")
  cancellation.resolve()
  await flush()
  expect(f.route.refetch).not.toHaveBeenCalled()
  f.gates[1].resolve()
  await flush()
  expect(f.route.refetch).toHaveBeenCalledOnce()
  expect(f.persisted).toEqual(custom(1024, 600))
})
it("0194 failed fresh reconciliation retains the draft and recoverable Retry", async () => {
  const f = fixture()
  const { root } = mount()
  f.route.refetch.mockRejectedValueOnce(new Error("synthetic failed read"))
  edit(root, 0, "1024")
  f.gates[0].resolve()
  await flush()
  expect(dimensions(root)[0].value).toBe("1024")
  const retry = Array.from(root.querySelectorAll("button")).find((button) =>
    button.textContent?.includes("retry")
  )!
  expect(retry.disabled).toBe(false)
  retry.click()
  f.gates[1].resolve()
  await flush()
  expect(f.route.refetch).toHaveBeenCalledTimes(2)
  expect(f.persisted).toEqual(custom(1024, 480))
})
it("0194 installed Query cancels a replacement owner's pre-write pending read before final fresh reconciliation", async () => {
  const f = fixture()
  const first = mount()
  edit(first.root, 0, "1024")
  await flush()
  first.dispose()
  const oldRead = deferred()
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  control.queryClient = client
  let requests = 0
  const queryFn = vi.fn(() =>
    ++requests === 1
      ? oldRead.promise
      : Promise.resolve({ gameResolution: copy(f.persisted) })
  )
  const root = document.createElement("div")
  document.body.append(root)
  const Page = () => {
    control.route = createQuery(() => ({
      queryKey: ["settings.getSettings"],
      queryFn
    }))
    return <General />
  }
  disposers.push(
    render(
      () => (
        <QueryClientProvider client={client}>
          <Page />
        </QueryClientProvider>
      ),
      root
    )
  )
  await flush()
  expect(dimensions(root)[0].value).toBe("1024")
  edit(root, 1, "600")
  f.gates[0].resolve()
  f.gates[1].resolve()
  await flush()
  await flush()
  expect(queryFn).toHaveBeenCalledTimes(2)
  expect(client.getQueryData(["settings.getSettings"])).toEqual({
    gameResolution: custom(1024, 600)
  })
  oldRead.resolve({ gameResolution: custom() })
  await flush()
  expect(dimensions(root).map((input) => input.value)).toEqual(["1024", "600"])
  expect(client.getQueryData(["settings.getSettings"])).toEqual({
    gameResolution: custom(1024, 600)
  })
})
