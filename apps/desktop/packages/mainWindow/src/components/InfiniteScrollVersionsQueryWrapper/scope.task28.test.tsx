import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import Wrapper, { useInfiniteVersionsQuery } from "./index"

const c = vi.hoisted(() => ({
  params: undefined as any,
  query: vi.fn(),
  options: undefined as any,
  context: undefined as any,
  selectedInstance: vi.fn(),
  selectedServer: vi.fn()
}))
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [c.params, vi.fn()]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { query: c.query },
      queryClient: { removeQueries: vi.fn() }
    })
  }
}))
vi.mock("../SearchInputContext", () => ({
  default: () => ({
    setSelectedInstanceId: c.selectedInstance,
    setSelectedServerId: c.selectedServer
  })
}))
vi.mock("@tanstack/solid-query", () => ({
  createInfiniteQuery: (options: any) => {
    c.options = options
    return { isLoading: false, refetch: vi.fn() }
  }
}))
let dispose: () => void
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.query.mockReset()
  c.selectedInstance.mockClear()
  c.selectedServer.mockClear()
})
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
function fixture(params: any, type: any = "mod") {
  const [p, setP] = createSignal(params),
    [addon, setAddon] = createSignal(type)
  c.params = new Proxy({}, { get: (_, key) => p()[key as any] })
  const root = document.createElement("div")
  document.body.append(root)
  function Child() {
    c.context = useInfiniteVersionsQuery()
    return <span />
  }
  dispose = render(
    () => (
      <Wrapper modId="project" modplatform="modrinth" addonType={addon()}>
        <Child />
      </Wrapper>
    ),
    root
  )
  return { setP, setAddon }
}
it("GPT-0038 resolves server defaults, disables pending targets and ignores late old resolution", async () => {
  const resolves = new Map<number, (v: any) => void>()
  c.query.mockImplementation(([key, id]: any) =>
    key === "server.getServerDetails"
      ? new Promise((r) => resolves.set(id, r))
      : Promise.resolve(key.endsWith("getProject") ? {} : [])
  )
  const { setP, setAddon } = fixture({ serverId: "1" })
  expect(c.query).toHaveBeenCalledWith(["server.getServerDetails", 1])
  expect(c.options().enabled).toBe(false)
  setP({ serverId: "2" })
  expect(c.options().enabled).toBe(false)
  resolves.get(2)!({ modloaderType: "forge", gameVersion: "1.20.1" })
  await flush()
  expect(c.options().enabled).toBe(true)
  await c.options().queryFn({ pageParam: 0, queryKey: c.options().queryKey })
  expect(c.query).toHaveBeenLastCalledWith([
    "modplatforms.modrinth.getProjectVersions",
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["forge"] }
  ])
  const key = c.options().queryKey
  resolves.get(1)!({ modloaderType: "fabric", gameVersion: "1.21" })
  await flush()
  expect(c.options().queryKey).toEqual(key)
  c.context.setOverrideEnabled(true)
  c.context.setQuery({ gameVersion: null, modLoaderType: null })
  setAddon(undefined)
  // Task35 qualifies actual addon-kind resolution; unchanged defaults keep
  // the deliberate override after this new current lookup settles.
  resolves.get(2)!({ modloaderType: "forge", gameVersion: "1.20.1" })
  await flush()
  await c.options().queryFn({ pageParam: 0, queryKey: c.options().queryKey })
  expect(c.query).toHaveBeenLastCalledWith([
    "modplatforms.modrinth.getProjectVersions",
    { project_id: "project", game_versions: undefined, loaders: undefined }
  ])
})
it.each([{ instanceId: "3", serverId: "4" }, { instanceId: "3" }])(
  "instance precedence and vanilla game-only scope %j",
  async (params) => {
    c.query.mockResolvedValue({ version: "1.20.1", modloaders: [] })
    fixture(params)
    await flush()
    expect(c.query).toHaveBeenCalledWith(["instance.getInstanceDetails", 3])
    expect(c.query).not.toHaveBeenCalledWith(["server.getServerDetails", 4])
    expect(c.context.query.gameVersion).toBe("1.20.1")
    expect(c.context.query.modLoaderType).toBeUndefined()
  }
)
it("no target is unfiltered and loaderless server addons keep the game version", async () => {
  c.query.mockResolvedValue({ modloaderType: "forge", gameVersion: "1.20.1" })
  const { setP } = fixture({}, "resourcePack")
  await flush()
  expect(c.options().enabled).toBe(true)
  expect(c.context.query.gameVersion).toBeUndefined()
  setP({ serverId: "8" })
  await flush()
  expect(c.context.query.gameVersion).toBe("1.20.1")
  expect(c.context.query.modLoaderType).toBeUndefined()
})
