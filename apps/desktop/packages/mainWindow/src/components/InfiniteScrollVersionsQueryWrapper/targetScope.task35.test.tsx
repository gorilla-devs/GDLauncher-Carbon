import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import {
  QueryClient,
  QueryClientProvider,
  useQueryClient
} from "@tanstack/solid-query"
import Wrapper, { useInfiniteVersionsQuery } from "./index"

const c = vi.hoisted(() => ({ params: undefined as any, query: vi.fn() }))
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [c.params, vi.fn()]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { query: c.query },
      queryClient: useQueryClient()
    })
  }
}))
vi.mock("../SearchInputContext", () => ({ default: () => undefined }))

const disposals: (() => void)[] = [],
  clients: QueryClient[] = []
afterEach(() => {
  disposals.splice(0).forEach((d) => d())
  clients.splice(0).forEach((q) => q.clear())
  document.body.textContent = ""
  c.query.mockReset()
})
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 5))
}

interface Target {
  loader?: string | null
  gameVersion?: string | null
}
/** Answers target lookups with `target` and every list request with
 *  `versions` (empty unless given). */
function backend(target: Target, versions: any[] = []) {
  c.query.mockImplementation(([key, input]: any) => {
    if (key === "instance.getInstanceDetails")
      return Promise.resolve({
        name: `Instance ${input}`,
        modloaders: target.loader ? [{ type_: target.loader }] : [],
        version: target.gameVersion ?? null
      })
    if (key === "server.getServerDetails")
      return Promise.resolve({
        name: `Server ${input}`,
        modloaderType: target.loader ?? null,
        gameVersion: target.gameVersion ?? ""
      })
    if (key === "modplatforms.modrinth.getProject") return Promise.resolve({})
    if (key === "modplatforms.modrinth.getProjectVersions")
      return Promise.resolve(versions)
    if (key === "modplatforms.curseforge.getMod")
      return Promise.resolve({ data: {} })
    if (key === "modplatforms.curseforge.getModFiles")
      return Promise.resolve({
        data: [],
        pagination: { index: 0, totalCount: 0 }
      })
    throw new Error(`Unexpected synthetic query ${key}`)
  })
}
const calls = (key: string) =>
  c.query.mock.calls.filter(([a]) => a[0] === key).map(([a]) => a[1])
const versionCalls = () => calls("modplatforms.modrinth.getProjectVersions")

function mount(
  params: Record<string, string | undefined>,
  options: {
    addonType?: any
    loaders?: string[]
    gameVersions?: string[]
    platform?: "modrinth" | "curseforge"
  } = {}
) {
  const [type, setType] = createSignal(options.addonType)
  const [loaders, setLoaders] = createSignal(options.loaders)
  const [gameVersions, setGameVersions] = createSignal(options.gameVersions)
  c.params = params
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: Infinity, staleTime: Infinity }
    }
  })
  clients.push(client)
  const root = document.createElement("div")
  document.body.append(root)
  let context!: ReturnType<typeof useInfiniteVersionsQuery>
  function Child() {
    context = useInfiniteVersionsQuery()
    return null
  }
  disposals.push(
    render(
      () => (
        <QueryClientProvider client={client}>
          <Wrapper
            modId={options.platform === "curseforge" ? "123" : "project"}
            modplatform={options.platform ?? "modrinth"}
            addonType={type()}
            supportedModloaders={loaders()}
            supportedGameVersions={gameVersions()}
          >
            <Child />
          </Wrapper>
        </QueryClientProvider>
      ),
      root
    )
  )
  return {
    get context() {
      return context
    },
    setType,
    setLoaders,
    setGameVersions
  }
}

it("W02#1 a Quilt instance lists the Fabric builds of a Fabric-only addon", async () => {
  backend({ loader: "quilt", gameVersion: "1.20.1" })
  mount({ instanceId: "3" }, { addonType: "mod", loaders: ["fabric"] })
  await settle()
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["fabric"] }
  ])
})

it("W02#1 a NeoForge 1.20.1 instance lists the Forge builds of a Forge-only CurseForge addon", async () => {
  backend({ loader: "neoforge", gameVersion: "1.20.1" })
  mount(
    { instanceId: "3" },
    { addonType: "mod", loaders: ["forge"], platform: "curseforge" }
  )
  await settle()
  expect(
    calls("modplatforms.curseforge.getModFiles").map((input) => input.query)
  ).toEqual([
    { index: 0, pageSize: 20, gameVersion: "1.20.1", modLoaderType: "forge" }
  ])
})

it("W02#1 an instance drops a loader or game version the addon declares nothing for, as 2.0.40 did", async () => {
  backend({ loader: "forge", gameVersion: "24w14a" })
  const f = mount(
    { instanceId: "3" },
    { addonType: "mod", loaders: ["fabric"], gameVersions: ["1.20.4"] }
  )
  await settle()
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: undefined, loaders: undefined }
  ])
  // Nothing filters the list, so an empty one has nothing to explain.
  expect(f.context.targetFilters).toBeUndefined()
})

it("W02#1 a server keeps its own loader and version, and Show all versions lists the rest", async () => {
  backend({ loader: "forge", gameVersion: "1.20.1" })
  const f = mount(
    { serverId: "7" },
    { addonType: "mod", loaders: ["fabric"], gameVersions: ["1.21"] }
  )
  await settle()
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["forge"] }
  ])
  expect(f.context.targetFilters).toEqual({
    modLoaderType: "forge",
    gameVersion: "1.20.1"
  })
  f.context.showAllVersions()
  await settle()
  expect(f.context.overrideEnabled).toBe(true)
  expect(f.context.targetFilters).toBeUndefined()
  expect(versionCalls().at(-1)).toEqual({
    project_id: "project",
    game_versions: undefined,
    loaders: undefined
  })
  // Unticking Override goes back to the server's own filters.
  f.context.setOverrideEnabled(false)
  await settle()
  expect(f.context.query).toMatchObject({
    modLoaderType: "forge",
    gameVersion: "1.20.1"
  })
  expect(f.context.targetFilters).toEqual({
    modLoaderType: "forge",
    gameVersion: "1.20.1"
  })
})

it("W02#1 support declared after the target resolved re-scopes the same target once", async () => {
  backend({ loader: "quilt", gameVersion: "1.20.1" })
  const f = mount({ instanceId: "3" })
  await settle()
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["quilt"] }
  ])
  f.setType("mod")
  f.setLoaders(["fabric"])
  f.setGameVersions(["1.20.1"])
  await settle()
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["quilt"] },
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["fabric"] }
  ])
  expect(f.context.targetReady).toBe(true)
  expect(f.context.overrideEnabled).toBe(false)
  // A refetched project hands out equal new arrays: nothing changes.
  f.setLoaders(["fabric"])
  f.setGameVersions(["1.20.1"])
  await settle()
  expect(versionCalls()).toHaveLength(2)
  expect(
    calls("instance.getInstanceDetails"),
    "the target is not looked up again"
  ).toHaveLength(1)
})

it("W02#2 the addon's type arriving after the target resolved does not fetch the list again", async () => {
  for (const params of [{}, { instanceId: "3" }, { serverId: "7" }]) {
    backend({ loader: "fabric", gameVersion: "1.20.1" })
    const f = mount(params)
    await settle()
    expect(versionCalls()).toHaveLength(1)
    f.setType("mod")
    f.setLoaders(["fabric", "quilt"])
    f.setGameVersions(["1.20.1", "1.21"])
    await settle()
    expect(versionCalls(), JSON.stringify(params)).toHaveLength(1)
    expect(f.context.targetReady).toBe(true)
    disposals.splice(0).forEach((d) => d())
    c.query.mockReset()
  }
})

it("W02#4 an instance with no known game version lists every game version instead of an error", async () => {
  backend({ loader: "forge", gameVersion: null })
  const f = mount({ instanceId: "3" }, { addonType: "mod" })
  await settle()
  expect(f.context.targetError).toBeUndefined()
  expect(f.context.targetReady).toBe(true)
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: undefined, loaders: ["forge"] }
  ])
})

it("W02#5 a modpack opened with a server or instance selected lists every pack version", async () => {
  for (const params of [{ serverId: "7" }, { instanceId: "3" }]) {
    backend({ loader: "forge", gameVersion: "1.20.1" })
    const f = mount(params, { addonType: "modpack" })
    await settle()
    expect(calls("server.getServerDetails")).toEqual([])
    expect(calls("instance.getInstanceDetails")).toEqual([])
    expect(versionCalls()).toEqual([
      { project_id: "project", game_versions: undefined, loaders: undefined }
    ])
    expect(f.context.targetFilters).toBeUndefined()
    disposals.splice(0).forEach((d) => d())
    c.query.mockReset()
  }
  // The type arrives after the first resolution: the pack list ends unscoped.
  backend({ loader: "forge", gameVersion: "1.20.1" })
  const f = mount({ serverId: "7" })
  await settle()
  f.setType("modpack")
  await settle()
  expect(versionCalls().at(-1)).toEqual({
    project_id: "project",
    game_versions: undefined,
    loaders: undefined
  })
  expect(f.context.targetReady).toBe(true)
})
