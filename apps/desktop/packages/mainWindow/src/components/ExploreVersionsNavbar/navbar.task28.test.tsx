import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import Wrapper, {
  useInfiniteVersionsQuery
} from "../InfiniteScrollVersionsQueryWrapper"
import Navbar from "./index"

const c = vi.hoisted(() => ({
  params: undefined as any,
  query: vi.fn(),
  options: undefined as any,
  context: undefined as any,
  selects: [] as any[],
  override: undefined as any,
  refetch: vi.fn(),
  removeQueries: vi.fn()
}))
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [c.params, vi.fn()]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { query: c.query },
      queryClient: { removeQueries: c.removeQueries }
    }),
    createQuery: () => ({ data: undefined })
  }
}))
vi.mock("@/components/SearchInputContext", () => ({
  default: () => ({
    setSelectedInstanceId: vi.fn(),
    setSelectedServerId: vi.fn()
  })
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    minecraftVersions: {
      data: [
        { id: "1.20.1", type: "release" },
        { id: "1.21", type: "release" }
      ]
    }
  })
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "",
  getServerImageUrl: () => ""
}))
vi.mock("@tanstack/solid-query", () => ({
  createInfiniteQuery: (options: any) => {
    c.options = options
    return { isLoading: false, refetch: c.refetch }
  }
}))
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@gd/ui", () => ({
  Checkbox: (p: any) => {
    c.override = p
    return (
      <button
        data-override
        disabled={p.disabled}
        onClick={() => p.onChange(!p.checked)}
      >
        Override
      </button>
    )
  },
  Select: (p: any) => {
    c.selects.push(p)
    return (
      <div data-select data-value={p.value}>
        {p.children}
      </div>
    )
  },
  SelectTrigger: (p: any) => <div>{p.children}</div>,
  SelectContent: () => null,
  SelectItem: () => null,
  SelectValue: () => null
}))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  c.query.mockReset()
  c.refetch.mockReset()
  c.removeQueries.mockReset()
  c.selects = []
  c.override = undefined
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
function fixture(
  params: any = { serverId: "7" },
  type: any = "mod",
  loaders: any = ["fabric"],
  games = ["1.21"]
) {
  const [selection, setSelection] = createSignal(params)
  const [addon, setAddon] = createSignal(type)
  const [supportedLoaders, setSupportedLoaders] = createSignal(loaders)
  const pending = new Map<string, (v: any) => void>()
  c.params = new Proxy({}, { get: (_, key) => selection()[key as any] })
  c.query.mockImplementation(([key, id]: any) =>
    key === "server.getServerDetails" || key === "instance.getInstanceDetails"
      ? new Promise((resolve) => pending.set(`${key}:${id}`, resolve))
      : Promise.resolve(key.endsWith("getProject") ? {} : [])
  )
  const root = document.createElement("div")
  document.body.append(root)
  function Content() {
    c.context = useInfiniteVersionsQuery()
    return (
      <Navbar
        modplatform="modrinth"
        type="mod"
        addonType={addon()}
        supportedModloaders={supportedLoaders()}
        supportedGameVersions={games}
      />
    )
  }
  dispose = render(
    () => (
      <Wrapper
        modId="synthetic-project"
        modplatform="modrinth"
        addonType={addon()}
      >
        <Content />
      </Wrapper>
    ),
    root
  )
  const server = (
    id = 7,
    details: any = {
      name: `Server ${id}`,
      modloaderType: "forge",
      gameVersion: "1.20.1"
    }
  ) => pending.get(`server.getServerDetails:${id}`)!(details)
  const instance = (
    id = 3,
    details: any = {
      name: `Instance ${id}`,
      modloaders: [{ type_: "forge" }],
      version: "1.20.1"
    }
  ) => pending.get(`instance.getInstanceDetails:${id}`)!(details)
  return {
    root,
    server,
    instance,
    setSelection,
    setAddon,
    setSupportedLoaders,
    serverLookup: (id: number) => pending.get(`server.getServerDetails:${id}`)!
  }
}
async function request() {
  const options = c.options()
  return options.queryFn({ pageParam: 0, queryKey: options.queryKey })
}
const scoped = { modLoaderType: "forge", gameVersion: "1.20.1" }

it("R1 actual navbar retains and displays unsupported server defaults with compatible-empty results", async () => {
  const { root, server } = fixture()
  server()
  await flush()
  expect(c.options().queryKey[3]).toEqual(scoped)
  expect(c.selects[0].options).toContain("1.20.1")
  expect(c.selects[1].options).toContain("forge")
  expect(c.selects.every((p) => p.disabled)).toBe(true)
  expect(root.textContent).toContain("Server 7")
  expect(root.textContent).not.toContain("no_instance_selected")
  const result = await request()
  expect(result.data).toEqual([])
  expect(c.query).toHaveBeenLastCalledWith([
    "modplatforms.modrinth.getProjectVersions",
    {
      project_id: "synthetic-project",
      game_versions: ["1.20.1"],
      loaders: ["forge"]
    }
  ])
})
it("R2 pending selector/direct/Override callbacks cannot admit or refetch an unresolved target", async () => {
  const { server } = fixture({ serverId: "7" }, "mod", ["forge"], ["1.20.1"])
  expect(c.options().enabled).toBe(false)
  c.selects[0].onChange(null)
  c.context.setQuery({ gameVersion: null })
  c.override?.onChange(true)
  expect(c.options().enabled).toBe(false)
  expect(c.options().queryKey[3]).toBeUndefined()
  expect(c.refetch).not.toHaveBeenCalled()
  expect(c.removeQueries).not.toHaveBeenCalled()
  expect(c.selects.every((p) => p.disabled)).toBe(true)
  expect(c.override.disabled).toBe(true)
  server()
  await flush()
  expect(c.options().queryKey[3]).toEqual(scoped)
})
it("R1 explicit Override permits deliberate clears; disabling it restores current defaults", async () => {
  const { server, setAddon, setSupportedLoaders } = fixture(
    { serverId: "7" },
    "mod",
    ["forge"],
    ["1.20.1"]
  )
  server()
  await flush()
  expect(c.override.checked).toBe(false)
  c.selects[0].onChange("")
  expect(c.options().queryKey[3]).toEqual(scoped)
  c.override.onChange(true)
  c.selects[0].onChange("")
  c.selects[1].onChange("")
  await flush()
  expect(c.options().queryKey[3]).toEqual({
    gameVersion: null,
    modLoaderType: null
  })
  setAddon(undefined)
  setSupportedLoaders(["forge"])
  server()
  await flush()
  expect(c.override.checked).toBe(true)
  expect(c.options().queryKey[3]).toEqual({
    gameVersion: null,
    modLoaderType: null
  })
  await request()
  expect(c.query).toHaveBeenLastCalledWith([
    "modplatforms.modrinth.getProjectVersions",
    {
      project_id: "synthetic-project",
      game_versions: undefined,
      loaders: undefined
    }
  ])
  c.override.onChange(false)
  await flush()
  expect(c.options().queryKey[3]).toEqual(scoped)
  expect(c.override.checked).toBe(false)
  expect(c.selects.every((p) => p.disabled)).toBe(true)
})
it("R2 A-to-B resets Override, blocks pending/stale handlers and ignores late A lookup", async () => {
  const { server, setSelection } = fixture(
    { serverId: "7" },
    "mod",
    ["forge"],
    ["1.20.1"]
  )
  const oldPendingSetter = c.context.setQuery
  setSelection({ serverId: "8" })
  expect(c.options().enabled).toBe(false)
  c.context.setQuery({ gameVersion: "old-A" })
  server(8, { name: "Server B", modloaderType: "fabric", gameVersion: "1.21" })
  await flush()
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: "fabric",
    gameVersion: "1.21"
  })
  c.override.onChange(true)
  const bSetter = c.context.setQuery
  bSetter({ gameVersion: null })
  oldPendingSetter({ gameVersion: "old-A" })
  server(7)
  await flush()
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: "fabric",
    gameVersion: null
  })
  c.override.onChange(false)
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: "fabric",
    gameVersion: "1.21"
  })
  c.override.onChange(true)
  const bSelector = c.selects[0].onChange,
    bOverride = c.override.onChange
  setSelection({ serverId: "9" })
  c.context.setQuery({ gameVersion: "old-B" })
  expect(c.options().enabled).toBe(false)
  server(9)
  await flush()
  expect(c.override.checked).toBe(false)
  expect(c.options().queryKey[3]).toEqual(scoped)
  bSetter({ gameVersion: "obsolete-B" })
  bSelector("")
  bOverride(true)
  expect(c.override.checked).toBe(false)
  expect(c.options().queryKey[3]).toEqual(scoped)
})
it("instance precedence and vanilla game-only scope use the wrapper target without duplicate navbar lookup", async () => {
  const { root, instance } = fixture({ instanceId: "3", serverId: "7" })
  instance(3, { name: "Vanilla instance", modloaders: [], version: "1.20.1" })
  await flush()
  expect(
    c.query.mock.calls.filter(
      ([args]) => args[0] === "instance.getInstanceDetails"
    )
  ).toHaveLength(1)
  expect(c.query).not.toHaveBeenCalledWith(["server.getServerDetails", 7])
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: undefined,
    gameVersion: "1.20.1"
  })
  expect(root.textContent).toContain("Vanilla instance")
})
it("loaderless target kinds preserve game-only default and no-target browsing permits direct filtering", async () => {
  const { server, setSelection } = fixture({ serverId: "7" }, "resourcePack")
  server()
  await flush()
  expect(c.selects).toHaveLength(1)
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: undefined,
    gameVersion: "1.20.1"
  })
  setSelection({})
  await flush()
  expect(c.options().enabled).toBe(true)
  expect(c.selects[0].disabled).toBe(false)
  c.selects[0].onChange("1.21")
  expect(c.options().queryKey[3].gameVersion).toBe("1.21")
})
it("disposed direct handlers cannot publish or refetch", async () => {
  const { server } = fixture()
  server()
  await flush()
  c.context.setOverrideEnabled(true)
  const setter = c.context.setQuery
  const override = c.context.setOverrideEnabled
  dispose!()
  dispose = undefined
  c.refetch.mockClear()
  c.removeQueries.mockClear()
  setter({ gameVersion: null })
  override(false)
  expect(c.refetch).not.toHaveBeenCalled()
  expect(c.removeQueries).not.toHaveBeenCalled()
})
it("keyed filter publication avoids cache removal and a changed target blocks old handlers", async () => {
  const { server, setSelection } = fixture()
  server()
  await flush()
  c.context.setOverrideEnabled(true)
  const setter = c.context.setQuery
  c.context.setQuery({ gameVersion: null })
  expect(c.removeQueries).not.toHaveBeenCalled()
  expect(c.refetch).not.toHaveBeenCalled()
  setSelection({ serverId: "8" })
  expect(c.options().enabled).toBe(false)
  expect(c.options().queryKey[3]).toBeUndefined()
  expect(c.refetch).not.toHaveBeenCalled()
  c.context.setQuery({ gameVersion: "obsolete" })
  setter({ gameVersion: "obsolete" })
  expect(c.refetch).not.toHaveBeenCalled()
  server(8)
  await flush()
  expect(c.options().queryKey[3]).toEqual(scoped)
})
it("R4 returning from pending B to settled A restores defaults and retires old Override", async () => {
  const { server, setSelection } = fixture()
  server()
  await flush()
  expect(c.options().queryKey[3]).toEqual(scoped)
  c.override.onChange(true)
  c.selects[0].onChange("")
  c.selects[1].onChange("")
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: null,
    gameVersion: null
  })
  const aSetter = c.context.setQuery,
    aSelector = c.selects[0].onChange,
    aOverride = c.override.onChange
  setSelection({ serverId: "8" })
  expect(c.options().enabled).toBe(false)
  expect(c.options().queryKey[3]).toBeUndefined()
  const bSetter = c.context.setQuery
  setSelection({ serverId: "7" })
  expect(c.options().enabled).toBe(false)
  server()
  await flush()
  expect(c.override.checked).toBe(false)
  expect(c.context.targetReady).toBe(true)
  expect(c.options().queryKey[3]).toEqual(scoped)
  expect(c.selects.every((p) => p.disabled)).toBe(true)
  c.refetch.mockClear()
  c.removeQueries.mockClear()
  aSetter({ gameVersion: null })
  aSelector("")
  aOverride(true)
  bSetter({ gameVersion: "obsolete-B" })
  server(8, { name: "Server B", modloaderType: "fabric", gameVersion: "1.21" })
  await flush()
  expect(c.context.target.id).toBe(7)
  expect(c.override.checked).toBe(false)
  expect(c.options().queryKey[3]).toEqual(scoped)
  expect(c.refetch).not.toHaveBeenCalled()
  expect(c.removeQueries).not.toHaveBeenCalled()
})
it("R5 pending B retains owed defaults through mod-resourcePack-mod resolution changes", async () => {
  const { server, serverLookup, setSelection, setAddon } = fixture()
  server()
  await flush()
  expect(c.options().queryKey[3]).toEqual(scoped)
  c.override.onChange(true)
  c.selects[0].onChange("")
  c.selects[1].onChange("")
  expect(c.options().queryKey[3]).toEqual({
    modLoaderType: null,
    gameVersion: null
  })
  const aSetter = c.context.setQuery
  setSelection({ serverId: "8" })
  const firstModLookup = serverLookup(8),
    pendingBSetter = c.context.setQuery
  expect(c.options().enabled).toBe(false)
  setAddon("resourcePack")
  const loaderlessLookup = serverLookup(8)
  expect(c.options().enabled).toBe(false)
  setAddon("mod")
  const currentModLookup = serverLookup(8)
  expect(c.options().enabled).toBe(false)
  expect(firstModLookup).not.toBe(loaderlessLookup)
  expect(currentModLookup).not.toBe(firstModLookup)
  expect(currentModLookup).not.toBe(loaderlessLookup)
  c.refetch.mockClear()
  c.removeQueries.mockClear()
  currentModLookup({
    name: "Current B",
    modloaderType: "forge",
    gameVersion: "1.20.1"
  })
  await flush()
  expect(c.context.target.id).toBe(8)
  expect(c.context.targetReady).toBe(true)
  expect(c.override.checked).toBe(false)
  firstModLookup({
    name: "Old B mod",
    modloaderType: "fabric",
    gameVersion: "1.21"
  })
  loaderlessLookup({
    name: "Old B loaderless",
    modloaderType: "fabric",
    gameVersion: "1.21"
  })
  await flush()
  expect(c.context.target.name).toBe("Current B")
  expect(c.context.defaults).toEqual(scoped)
  expect(c.options().queryKey[3]).toEqual(scoped)
  expect(c.selects.every((p) => p.disabled)).toBe(true)
  aSetter({ gameVersion: "obsolete A" })
  pendingBSetter({ gameVersion: "obsolete B" })
  expect(c.options().queryKey[3]).toEqual(scoped)
  expect(c.refetch).not.toHaveBeenCalled()
  expect(c.removeQueries).not.toHaveBeenCalled()
})
