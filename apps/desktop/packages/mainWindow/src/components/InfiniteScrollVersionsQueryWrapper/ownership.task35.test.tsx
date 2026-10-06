import { afterEach, expect, it, vi } from "vitest"
import { createSignal, For } from "solid-js"
import { render } from "solid-js/web"
import {
  QueryClient,
  QueryClientProvider,
  useQueryClient
} from "@tanstack/solid-query"
import Wrapper, { useInfiniteVersionsQuery } from "./index"
import Navbar from "../ExploreVersionsNavbar"
import RowContainer from "../Browser/RowContainer"

const c = vi.hoisted(() => ({
  params: undefined as any,
  query: vi.fn(),
  mutations: [] as any[]
}))
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [c.params, vi.fn()]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { query: c.query },
      queryClient: useQueryClient()
    }),
    createQuery: () => ({ data: undefined }),
    createMutation: (options: any) => ({
      mutateAsync: async (input: any) => {
        c.mutations.push({ key: options().mutationKey, input })
        return 1
      }
    })
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
    instances: { data: [] },
    minecraftVersions: {
      data: [
        { id: "1.12.2", type: "release" },
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
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("../ModDownloadButton/components/InstanceDropdown", () => ({
  InstanceDropdown: () => null
}))
vi.mock("../ModpackDownloadButton", () => ({ default: () => null }))
vi.mock("../ServerPackDownloadButton", () => ({ default: () => null }))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../packages/ui/src/Checkbox")),
  ...(await import("../../../../../../../packages/ui/src/Select")),
  ...(await import("../../../../../../../packages/ui/src/Button")),
  Badge: (p: any) => <span>{p.children}</span>,
  Spinner: () => <span>loading</span>,
  Progress: () => null,
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div>{p.children}</div>,
  toast: { success: vi.fn(), error: vi.fn() }
}))
const disposals: (() => void)[] = [],
  clients: QueryClient[] = []
afterEach(() => {
  disposals.splice(0).forEach((d) => d())
  clients.splice(0).forEach((q) => q.clear())
  document.body.textContent = ""
  c.query.mockReset()
  c.mutations = []
})
const settle = async () => {
  for (let i = 0; i < 5; i++) {
    await new Promise((r) => setTimeout(r, 5))
  }
}
function deferred<T = any>() {
  let resolve!: (v: T) => void, reject!: (e: Error) => void
  const promise = new Promise<T>((r, j) => {
    resolve = r
    reject = j
  })
  return { promise, resolve, reject }
}
const version = (id = "chosen") => ({
  id,
  project_id: "project",
  name: "Synthetic version",
  version_type: "release",
  game_versions: ["1.20.1"],
  downloads: 1,
  date_published: "2026-01-01",
  files: [
    { filename: "synthetic.jar", size: 1, hashes: { sha512: "synthetic" } }
  ]
})
function mount(
  params: any = { instanceId: "7" },
  addonType: any = "mod",
  platform: any = "modrinth",
  modId = "project",
  navbar = false,
  supported = { games: ["1.20.1"], loaders: ["fabric"] },
  sharedClient?: QueryClient
) {
  const [selection, setSelection] = createSignal(params),
    [type, setType] = createSignal(addonType),
    [project, setProject] = createSignal(modId),
    [provider, setProvider] = createSignal(platform)
  c.params = new Proxy({}, { get: (_, key) => selection()[key as any] })
  const client =
    sharedClient ??
    new QueryClient({
      defaultOptions: {
        queries: { retry: false, gcTime: Infinity, staleTime: Infinity }
      }
    })
  if (!sharedClient) clients.push(client)
  const root = document.createElement("div")
  document.body.append(root)
  let context!: ReturnType<typeof useInfiniteVersionsQuery>
  function Child() {
    context = useInfiniteVersionsQuery()
    return (
      <>
        {navbar && (
          <Navbar
            modplatform={provider()}
            type="mod"
            addonType={type()}
            supportedGameVersions={supported.games}
            supportedModloaders={supported.loaders as any}
          />
        )}
        <div data-rows>
          <For each={context.allRows()}>
            {(row) => (
              <RowContainer
                modVersion={row}
                project={
                  {
                    id: project(),
                    platform: provider(),
                    type: "mod",
                    title: "Synthetic addon"
                  } as any
                }
                instanceId={
                  context.target.kind === "instance"
                    ? context.target.id
                    : undefined
                }
                serverId={
                  context.target.kind === "server"
                    ? context.target.id
                    : undefined
                }
                installedFile={undefined}
                type="mod"
                loading={false}
                disabled={false}
              />
            )}
          </For>
        </div>
      </>
    )
  }
  const dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <Wrapper modId={project()} modplatform={provider()} addonType={type()}>
          <Child />
        </Wrapper>
      </QueryClientProvider>
    ),
    root
  )
  disposals.push(dispose)
  return {
    root,
    dispose,
    client,
    context,
    setSelection,
    setType,
    setProject,
    setProvider
  }
}
function normalQuery([key, input]: any) {
  if (key === "instance.getInstanceDetails")
    return Promise.resolve({
      name: `Instance ${input}`,
      modloaders: [{ type_: input === 7 ? "forge" : "fabric" }],
      version: input === 7 ? "1.12.2" : "1.20.1"
    })
  if (key === "server.getServerDetails")
    return Promise.resolve({
      name: "Server",
      modloaderType: "forge",
      gameVersion: "1.12.2"
    })
  if (key === "modplatforms.modrinth.getProject") return Promise.resolve({})
  if (key === "modplatforms.modrinth.getProjectVersions")
    return Promise.resolve([])
  throw new Error(`Unexpected synthetic query ${key}`)
}
const versionCalls = () =>
  c.query.mock.calls
    .filter(([args]) => args[0] === "modplatforms.modrinth.getProjectVersions")
    .map(([args]) => args[1])
it("D2 simultaneous wrapper stores remain independent across late/disposed owner resolution", async () => {
  const detailsA = deferred(),
    detailsB = deferred()
  c.query.mockImplementation((args) =>
    args[0] === "instance.getInstanceDetails"
      ? args[1] === 7
        ? detailsA.promise
        : detailsB.promise
      : normalQuery(args)
  )
  const a = mount(),
    b = mount({ instanceId: "8" })
  detailsB.resolve({ modloaders: [{ type_: "fabric" }], version: "1.20.1" })
  await settle()
  expect(b.context.query.modLoaderType).toBe("fabric")
  expect(a.context.query.modLoaderType).not.toBe("fabric")
  a.dispose()
  detailsA.resolve({ modloaders: [{ type_: "forge" }], version: "1.12.2" })
  await settle()
  expect(b.context.query.modLoaderType).toBe("fabric")
  expect(versionCalls()).toEqual([
    { project_id: "project", loaders: ["fabric"], game_versions: ["1.20.1"] }
  ])
})
it("D2 a delayed old project request retains its captured project and filters after owner replacement", async () => {
  const projectA = deferred()
  c.query.mockImplementation((args) =>
    args[0] === "modplatforms.modrinth.getProject" && args[1] === "project"
      ? projectA.promise
      : normalQuery(args)
  )
  const f = mount()
  await settle()
  f.context.setOverrideEnabled(true)
  f.context.setQuery({ gameVersion: "1.21", modLoaderType: "fabric" })
  f.setProject("replacement")
  await settle()
  projectA.resolve({})
  await settle()
  const calls = versionCalls()
  expect(calls).toContainEqual({
    project_id: "project",
    loaders: ["forge"],
    game_versions: ["1.12.2"]
  })
  expect(calls).toContainEqual({
    project_id: "replacement",
    loaders: ["fabric"],
    game_versions: ["1.21"]
  })
  expect(
    calls.some(
      (v) => v.project_id === "replacement" && v.loaders?.[0] === "forge"
    )
  ).toBe(false)
})
it.each(["resolve", "reject"])(
  "D2 project-qualified scope discards old details %s and stale handlers",
  async (outcome) => {
    const old = deferred(),
      current = deferred()
    c.query.mockImplementation((args) =>
      args[0] === "instance.getInstanceDetails"
        ? c.query.mock.calls.filter(
            ([a]) => a[0] === "instance.getInstanceDetails"
          ).length === 1
          ? old.promise
          : current.promise
        : normalQuery(args)
    )
    const f = mount()
    const pendingSetter = f.context.setQuery
    f.setProject("replacement")
    expect(f.context.targetReady).toBe(false)
    expect(
      c.query.mock.calls.filter(([a]) => a[0] === "instance.getInstanceDetails")
    ).toHaveLength(2)
    current.resolve({
      name: "current",
      modloaders: [{ type_: "fabric" }],
      version: "1.20.1"
    })
    await settle()
    f.context.setOverrideEnabled(true)
    pendingSetter({ modLoaderType: "obsolete" })
    if (outcome === "resolve")
      old.resolve({
        name: "old",
        modloaders: [{ type_: "forge" }],
        version: "1.12.2"
      })
    else old.reject(new Error("old synthetic error"))
    await settle()
    expect(f.context.query.modLoaderType).toBe("fabric")
    expect(f.context.target.name).toBe("current")
  }
)
it("D2 filter publication neither evicts another owner nor duplicates the initial request", async () => {
  c.query.mockImplementation(normalQuery)
  const a = mount(),
    b = mount(
      { instanceId: "8" },
      "mod",
      "modrinth",
      "project",
      false,
      undefined,
      a.client
    )
  await settle()
  expect(versionCalls()).toHaveLength(2)
  const bQuery = b.client
    .getQueryCache()
    .getAll()
    .find(
      (q) =>
        q.queryKey[0] === "modplatforms.versions" &&
        (q.queryKey[3] as any)?.modLoaderType === "fabric"
    )!
  const remove = vi.spyOn(a.client, "removeQueries")
  a.context.setOverrideEnabled(true)
  a.context.setQuery({ gameVersion: null })
  await settle()
  expect(remove).not.toHaveBeenCalled()
  expect(b.client.getQueryCache().getAll()).toContain(bQuery)
  expect(versionCalls()).toHaveLength(3)
})
it("D2 CurseForge pagination reads each pageParam and immutable page size while project awaits", async () => {
  const pending: ReturnType<typeof deferred>[] = []
  c.query.mockImplementation(([key, input]) => {
    if (key === "instance.getInstanceDetails") return normalQuery([key, input])
    if (key === "modplatforms.curseforge.getMod") {
      const p = deferred()
      pending.push(p)
      return p.promise
    }
    if (key === "modplatforms.curseforge.getModFiles")
      return Promise.resolve({
        data: [],
        pagination: { index: input.query.index, totalCount: 60 }
      })
    throw new Error(`Unexpected synthetic query ${key}`)
  })
  const f = mount({ instanceId: "7" }, "mod", "curseforge", "123")
  await settle()
  f.context.setOverrideEnabled(true)
  f.context.setQuery({ pageSize: 10 })
  await settle()
  pending[0].resolve({ data: {} })
  await settle()
  expect(
    c.query.mock.calls.find(
      ([a]) => a[0] === "modplatforms.curseforge.getModFiles"
    )![0][1].query
  ).toMatchObject({ index: 0, pageSize: 20 })
  pending.slice(1).forEach((p) => p.resolve({ data: {} }))
  await settle()
  const next = f.context.infiniteQuery.fetchNextPage()
  await settle()
  pending.at(-1)!.resolve({ data: {} })
  await next
  const inputs = c.query.mock.calls
    .filter(([a]) => a[0] === "modplatforms.curseforge.getModFiles")
    .map(([a]) => a[1].query)
  expect(inputs.at(-1)).toMatchObject({ index: 10, pageSize: 10 })
})
it.each(["loader-and-game", "game-only", "compatible", "server"])(
  "GPT0039 Task28 carrier: real navbar and query preserve %s defaults",
  async (control) => {
    c.query.mockImplementation((args) =>
      args[0] === "modplatforms.modrinth.getProjectVersions"
        ? Promise.resolve(
            args[1].game_versions?.[0] === "1.20.1" &&
              args[1].loaders?.[0] === "fabric"
              ? [version()]
              : []
          )
        : normalQuery(args)
    )
    const f = mount(
      control === "server"
        ? { serverId: "7" }
        : { instanceId: control === "compatible" ? "8" : "7" },
      "mod",
      "modrinth",
      "project",
      true,
      {
        games: ["1.20.1"],
        loaders: control === "game-only" ? ["forge"] : ["fabric"]
      }
    )
    await settle()
    const checkbox = f.root.querySelector<HTMLInputElement>(
      'input[type="checkbox"]'
    )!
    expect(checkbox.checked).toBe(false)
    const selects = [
      ...f.root.querySelectorAll('button[aria-haspopup="listbox"]')
    ] as HTMLButtonElement[]
    expect(selects).toHaveLength(2)
    expect(selects.every((s) => s.disabled)).toBe(true)
    expect(selects[0].textContent).toContain(
      control === "compatible" ? "1.20.1" : "1.12.2"
    )
    expect(selects[1].textContent).toContain(
      control === "compatible" ? "fabric" : "forge"
    )
    expect(f.context.allRows()).toHaveLength(control === "compatible" ? 1 : 0)
    expect(versionCalls()).toHaveLength(1)
  }
)
it("GPT0039 actual Override checkbox and selectors expose deliberate row; actual install retains target/version; off restores defaults", async () => {
  c.query.mockImplementation((args) =>
    args[0] === "modplatforms.modrinth.getProjectVersions"
      ? Promise.resolve(
          args[1].game_versions?.[0] === "1.20.1" &&
            args[1].loaders?.[0] === "fabric"
            ? [version()]
            : []
        )
      : normalQuery(args)
  )
  const f = mount({ instanceId: "7" }, "mod", "modrinth", "project", true)
  await settle()
  const checkbox = f.root.querySelector<HTMLInputElement>(
    'input[type="checkbox"]'
  )!
  checkbox.click()
  expect(f.context.overrideEnabled).toBe(true)
  async function choose(index: number, value: string) {
    const trigger = f.root.querySelectorAll('button[aria-haspopup="listbox"]')[
      index
    ] as HTMLButtonElement
    trigger.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        pointerType: "mouse",
        button: 0
      })
    )
    trigger.click()
    await settle()
    const option = [...document.querySelectorAll('[role="option"]')].find((o) =>
      o.textContent?.includes(value)
    ) as HTMLElement
    expect(option).toBeDefined()
    option.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        pointerType: "mouse",
        button: 0
      })
    )
    option.dispatchEvent(
      new PointerEvent("pointerup", {
        bubbles: true,
        pointerType: "mouse",
        button: 0
      })
    )
    option.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 1 }))
    await settle()
  }
  await choose(0, "1.20.1")
  expect(f.context.query.gameVersion).toBe("1.20.1")
  await choose(1, "fabric")
  expect(f.context.query.modLoaderType).toBe("fabric")
  expect(f.context.allRows()).toHaveLength(1)
  const install = [...f.root.querySelectorAll("[data-rows] button")].find((b) =>
    b.textContent?.includes("download_version")
  ) as HTMLButtonElement
  expect(install).toBeDefined()
  install.click()
  await settle()
  expect(c.mutations).toContainEqual({
    key: "instance.installMod",
    input: {
      instance_id: 7,
      mod_source: { Modrinth: { project_id: "project", version_id: "chosen" } },
      install_deps: true,
      replaces_mod: null
    }
  })
  checkbox.click()
  await settle()
  expect(f.context.overrideEnabled).toBe(false)
  expect(f.context.allRows()).toEqual([])
  expect(f.context.query).toMatchObject({
    gameVersion: "1.12.2",
    modLoaderType: "forge"
  })
})
it.each(["resourcePack", "shader", "world", "datapack"])(
  "loaderless %s retains game scope; Modrinth remains one page",
  async (addon) => {
    c.query.mockImplementation(normalQuery)
    const f = mount({ instanceId: "7" }, addon)
    await settle()
    expect(versionCalls()).toEqual([
      { project_id: "project", game_versions: ["1.12.2"], loaders: undefined }
    ])
    await f.context.infiniteQuery.fetchNextPage()
    expect(versionCalls()).toHaveLength(1)
  }
)
it("unscoped browse and vanilla preserve normal no-filter/game-only behavior", async () => {
  c.query.mockImplementation((args) =>
    args[0] === "instance.getInstanceDetails"
      ? Promise.resolve({ version: "1.20.1", modloaders: [] })
      : normalQuery(args)
  )
  const f = mount({})
  await settle()
  expect(f.context.overrideEnabled).toBe(true)
  expect(versionCalls()[0]).toEqual({
    project_id: "project",
    loaders: undefined,
    game_versions: undefined
  })
  f.setSelection({ instanceId: "8" })
  expect(f.context.allRows()).toEqual([])
  await settle()
  expect(versionCalls().at(-1)).toEqual({
    project_id: "project",
    loaders: undefined,
    game_versions: ["1.20.1"]
  })
})
it.each(["instance", "server"])(
  "D1/D2 known %s lookup failure cannot admit unfiltered requests and supports current retry",
  async (kind) => {
    let fail = true
    c.query.mockImplementation((args) =>
      args[0] ===
        `${kind}.get${kind === "instance" ? "Instance" : "Server"}Details` &&
      fail
        ? Promise.reject(new Error("synthetic target lookup failed"))
        : normalQuery(args)
    )
    const f = mount(
      kind === "instance" ? { instanceId: "7" } : { serverId: "7" }
    )
    await settle()
    expect(versionCalls()).toEqual([])
    expect(f.context.targetReady).toBe(false)
    expect(f.context.allRows()).toEqual([])
    expect(f.context.targetError).toBeTruthy()
    expect(f.context.isLoading).toBe(false)
    fail = false
    await f.context.retryTarget()
    await settle()
    expect(f.context.targetReady).toBe(true)
    expect(versionCalls()).toEqual([
      { project_id: "project", game_versions: ["1.12.2"], loaders: ["forge"] }
    ])
  }
)
it.each(["instance", "server"])(
  "D1/D2 missing %s details stays gated with a readable current error",
  async (kind) => {
    c.query.mockImplementation((args) =>
      args[0].startsWith(`${kind}.get`)
        ? Promise.resolve(null)
        : normalQuery(args)
    )
    const f = mount(
      kind === "instance" ? { instanceId: "7" } : { serverId: "7" }
    )
    await settle()
    expect(versionCalls()).toEqual([])
    expect(f.context.targetReady).toBe(false)
    expect(f.context.targetError).toBeInstanceOf(Error)
    expect(f.context.isLoading).toBe(false)
  }
)
it("D1/D2 retry authority expires across replacement, ABA and disposal; old success/error cannot settle current retry", async () => {
  c.query.mockRejectedValueOnce(new Error("synthetic initial failure"))
  const a = deferred(),
    b = deferred()
  c.query
    .mockImplementationOnce(() => a.promise)
    .mockImplementationOnce(() => b.promise)
    .mockImplementation(normalQuery)
  const f = mount()
  await settle()
  const failedRetry = f.context.retryTarget
  const retry = failedRetry()
  const queriesAfterRetry = c.query.mock.calls.length
  await failedRetry()
  expect(c.query.mock.calls).toHaveLength(queriesAfterRetry)
  f.setProject("replacement")
  expect(f.context.targetReady).toBe(false)
  b.resolve({
    name: "Replacement",
    version: "1.20.1",
    modloaders: [{ type_: "fabric" }]
  })
  await settle()
  a.reject(new Error("stale retry failure"))
  await retry
  expect(f.context.targetError).toBeUndefined()
  expect(f.context.target.name).toBe("Replacement")
  f.setProject("project")
  await settle()
  const calls = c.query.mock.calls.length
  await failedRetry()
  expect(c.query.mock.calls).toHaveLength(calls)
  f.dispose()
  await f.context.retryTarget()
  expect(c.query.mock.calls).toHaveLength(calls)
})
it("D1/D2 disposing a failed current owner revokes its retained Retry", async () => {
  c.query.mockRejectedValue(new Error("synthetic current failure"))
  const f = mount()
  await settle()
  const retry = f.context.retryTarget
  const calls = c.query.mock.calls.length
  f.dispose()
  await retry()
  expect(c.query.mock.calls).toHaveLength(calls)
  expect(versionCalls()).toEqual([])
})
it("D2 equivalent null and undefined no-filter edits preserve rows without another request", async () => {
  c.query.mockImplementation((args) =>
    args[0] === "modplatforms.modrinth.getProjectVersions"
      ? Promise.resolve([version()])
      : normalQuery(args)
  )
  const f = mount({})
  await settle()
  expect(f.context.allRows()).toHaveLength(1)
  f.context.setQuery({ gameVersion: null, modLoaderType: null })
  await settle()
  expect(f.context.allRows()).toHaveLength(1)
  expect(versionCalls()).toHaveLength(1)
})
