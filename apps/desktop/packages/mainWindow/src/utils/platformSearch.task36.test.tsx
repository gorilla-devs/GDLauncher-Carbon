import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import {
  MemoryRouter,
  Route,
  createMemoryHistory,
  useNavigate,
  useSearchParams
} from "@solidjs/router"
import { getSearchResults, nextSearchOffset } from "./platformSearch"
import { parseSearchQuery } from "./searchQueryParser"
import { ListItem } from "../pages/Search/ListItem"
import { GridItem } from "../pages/Search/GridItem"
import ServerPackDownloadButton from "../components/ServerPackDownloadButton"
import AddonPage from "../pages/AddonViewPage"

const c = vi.hoisted(() => ({
  query: vi.fn(),
  creates: [] as any[],
  callbacks: {} as Record<string, any>
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { query: c.query } }),
    createQuery: (options: any) =>
      createQuery(() => {
        const opts = options()
        return {
          ...opts,
          queryFn: opts.queryFn ?? (() => c.query(opts.queryKey))
        }
      }),
    createMutation: (options: any) => {
      const opts = options()
      c.callbacks[opts.mutationKey[0]] = opts
      return {
        mutate: (input: any) =>
          c.creates.push({ key: opts.mutationKey[0], input })
      }
    }
  }
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    categories: { data: { curseforge: {}, modrinth: {} }, isLoading: false }
  })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key],
  i18n: { t: (key: string) => key }
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../packages/ui/src/Button")),
  Spinner: () => <span>loading</span>,
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div>{p.children}</div>,
  Badge: (p: any) => <span>{p.children}</span>,
  toast: { success: vi.fn(), error: vi.fn() },
  AuthorsSkeleton: () => <span />,
  Skeleton: () => <span />,
  Tabs: (p: any) => <div>{p.children}</div>,
  TabsList: (p: any) => <div>{p.children}</div>,
  TabsTrigger: (p: any) => <div>{p.children}</div>,
  TabsIndicator: () => <span />
}))
vi.mock("../pages/Search/DynamicBadgeContainer", () => ({
  default: () => <span />
}))
// Nonpack installation uses separate owned-target controls; no real installer is imported here.
vi.mock("@/components/ModDownloadButton", () => ({
  default: (p: any) => <button data-file={p.fileId}>addon</button>
}))
vi.mock("@/components/InfiniteScrollVersionsQueryWrapper", () => ({
  default: (p: any) => <>{p.children}</>
}))
vi.mock("@/components/ExploreVersionsNavbar", () => ({
  default: () => <span />
}))
vi.mock("@/components/ContentWrapper", () => ({
  default: (p: any) => <div id="gdl-content-wrapper">{p.children}</div>
}))
vi.mock("@/components/AuthorAvatars", () => ({ default: () => <span /> }))
vi.mock("@/utils/sidebar", () => ({ ModloaderIcon: () => <span /> }))

let dispose: (() => void) | undefined
let client: QueryClient
const flush = async () => {
  for (let i = 0; i < 25; i++) {
    await Promise.resolve()
    await new Promise((r) => setTimeout(r, 0))
  }
}
afterEach(() => {
  dispose?.()
  client?.clear()
  document.body.textContent = ""
  c.query.mockReset()
  c.creates = []
  c.callbacks = {}
  vi.unstubAllGlobals()
})
const project = (platform = "curseforge", id = "123456"): any => ({
  platform,
  id,
  title: "Fixture",
  type: "modpack",
  mainFileId: "222222",
  serverPackFileId: "999999",
  categories: [],
  downloadsCount: 1,
  description: "fixture",
  imageUrl: null
})
function fixture(initial = "/search", layout?: "list" | "grid") {
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  let search!: ReturnType<typeof getSearchResults>,
    navigate!: ReturnType<typeof useNavigate>,
    params!: any
  const root = document.createElement("div")
  document.body.append(root)
  const history = createMemoryHistory()
  history.set({ value: initial, replace: true })
  const Content = () => {
    navigate = useNavigate()
    ;[params] = useSearchParams()
    search = getSearchResults()
    return (
      <>
        {layout &&
          (() => {
            const result = search.allRows().find((row) => row.value)?.value
            return (
              result &&
              (layout === "list" ? (
                <ListItem
                  result={result}
                  isInstalled={false}
                  onItemClick={() => {}}
                />
              ) : (
                <GridItem
                  result={result}
                  isInstalled={false}
                  onItemClick={() => {}}
                />
              ))
            )
          })()}
      </>
    )
  }
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/search" component={Content} />
        </MemoryRouter>
      </QueryClientProvider>
    ),
    root
  )
  return {
    get search() {
      return search
    },
    get navigate() {
      return navigate
    },
    get params() {
      return params
    },
    root
  }
}
function defaults() {
  c.query.mockImplementation(async ([key, input]: any) => {
    if (key === "modplatforms.unifiedSearch") {
      const data = Array.from({ length: input.pageSize }, (_, i) =>
        project(input.searchApi, String(input.index + i))
      )
      return {
        data,
        pagination: {
          index: input.index,
          pageSize: input.pageSize,
          resultCount: data.length,
          totalCount: 60
        }
      }
    }
    if (key === "modplatforms.unifiedGetProjectsByIds")
      return { results: [project()], errors: [] }
    if (key === "modplatforms.unifiedGetProject")
      return {
        ...project(),
        authors: [],
        loaders: [],
        minecraftVersions: [],
        releaseDate: "2026-01-01",
        lastUpdated: "2026-01-01"
      }
    if (key === "modplatforms.curseforge.getModFile")
      return {
        data: {
          id: input.fileId,
          modId: input.modId,
          isAvailable: true,
          serverPackFileId: 333333
        }
      }
    if (key === "settings.getSearchSidebarDocked") return true
    if (key === "server.getServerDetails")
      return { gameVersion: "1.21", modloaderType: null }
    if (key === "server.getServerAddons") return []
    return null
  })
}
it("initial routed text is already decoded, applies immediately and preserves target parameters", async () => {
  defaults()
  const f = fixture("/search?q=100%25&serverId=42")
  await flush()
  expect(f.search.searchQuery().searchQuery).toBe("100%")
  expect(f.params.q).toBeUndefined()
  expect(f.params.serverId).toBe("42")
})
function addonFixture(
  requested = "111111",
  file?: () => Promise<any>,
  mainCompanion?: string | null
) {
  defaults()
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    }
  )
  if (file) {
    const original = c.query.getMockImplementation()!
    c.query.mockImplementation((args: any) =>
      args[0] === "modplatforms.curseforge.getModFile" ? file() : original(args)
    )
  }
  if (mainCompanion !== undefined) {
    const original = c.query.getMockImplementation()!
    c.query.mockImplementation(async (args: any) => {
      const response = await original(args)
      return args[0] === "modplatforms.unifiedGetProject"
        ? { ...response, serverPackFileId: mainCompanion }
        : response
    })
  }
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  const root = document.createElement("div")
  document.body.append(root)
  const history = createMemoryHistory()
  history.set({
    value: `/addon/123456/curseforge?requestedVersionId=${requested}`,
    replace: true
  })
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/addon/:id/:platform" component={AddonPage} />
        </MemoryRouter>
      </QueryClientProvider>
    ),
    root
  )
  return root
}
it.each([true, false])(
  "selected companion presence=%s governs both real addon-page server controls and client splits",
  async (present) => {
    const root = addonFixture(
      "111111",
      async () => ({
        data: {
          id: 111111,
          modId: 123456,
          isAvailable: true,
          serverPackFileId: present ? 333333 : null
        }
      }),
      present ? null : "999999"
    )
    await flush()
    const serverButtons = [
      ...root.querySelectorAll<HTMLButtonElement>("button")
    ].filter((b) => b.querySelector(".i-hugeicons\\:server-stack-01"))
    const clients = [
      ...root.querySelectorAll<HTMLButtonElement>("button")
    ].filter((b) => b.querySelector(".i-hugeicons\\:download-02"))
    expect(serverButtons).toHaveLength(present ? 2 : 0)
    expect(clients).toHaveLength(2)
    clients.forEach((b) =>
      expect(b.classList.contains("!rounded-r-none")).toBe(present)
    )
    if (present) {
      serverButtons.forEach((b) => b.click())
      await flush()
      expect(
        c.creates
          .filter((row) => row.key === "server.createServerFromModpack")
          .map((row) => row.input.modpackSource.curseforge)
      ).toEqual([
        { project_id: 123456, file_id: 111111, server_pack_file_id: 333333 },
        { project_id: 123456, file_id: 111111, server_pack_file_id: 333333 }
      ])
    }
  }
)
it("ordinary project-only addon page keeps main companion and client/server no-file-query behavior", async () => {
  const root = addonFixture("")
  await flush()
  const servers = [
    ...root.querySelectorAll<HTMLButtonElement>("button")
  ].filter((b) => b.querySelector(".i-hugeicons\\:server-stack-01"))
  expect(servers).toHaveLength(2)
  servers[0].click()
  await flush()
  expect(
    c.creates.find((row) => row.key === "server.createServerFromModpack").input
      .modpackSource.curseforge
  ).toEqual({
    project_id: 123456,
    file_id: 222222,
    server_pack_file_id: 999999
  })
  expect(
    c.query.mock.calls.some(
      ([args]) => args[0] === "modplatforms.curseforge.getModFile"
    )
  ).toBe(false)
})
it("pending/foreign selected companion remains hidden until actual file-query Retry validates its owner", async () => {
  let finish!: (file: any) => void,
    requested = 0
  const root = addonFixture(
    "111111",
    () =>
      ++requested === 1
        ? new Promise((resolve) => {
            finish = resolve
          })
        : Promise.resolve({
            data: {
              id: 111111,
              modId: 123456,
              isAvailable: true,
              serverPackFileId: 333333
            }
          }),
    "999999"
  )
  await flush()
  expect(root.querySelector(".i-hugeicons\\:server-stack-01")).toBeNull()
  expect(root.querySelector("[role=alert]")?.textContent).toContain(
    "search:_trn_selected_version_loading"
  )
  finish({
    data: {
      id: 111111,
      modId: 999999,
      isAvailable: true,
      serverPackFileId: 333333
    }
  })
  await flush()
  expect(root.querySelector(".i-hugeicons\\:server-stack-01")).toBeNull()
  ;[...root.querySelectorAll<HTMLButtonElement>("button")]
    .find((b) => b.textContent === "general:_trn_retry")!
    .click()
  await flush()
  expect(root.querySelectorAll(".i-hugeicons\\:server-stack-01")).toHaveLength(
    2
  )
})
it("CurseForge addon-page continuation revalidates the requested file and both primary/sticky client actions retain it", async () => {
  const root = addonFixture()
  await flush()
  root.querySelector("#gdl-content-wrapper")!.dispatchEvent(new Event("scroll"))
  await new Promise((r) => setTimeout(r, 40))
  await flush()
  const buttons = [...root.querySelectorAll("button")].filter((b) =>
    [...b.querySelectorAll("div")].some((icon) =>
      icon.classList.contains("i-hugeicons:download-02")
    )
  )
  expect(buttons).toHaveLength(2)
  buttons.forEach((b) => b.click())
  await flush()
  expect(
    c.creates
      .filter((row) => row.key === "instance.createInstance")
      .map((row) => row.input.version.Modpack.value.file_id)
  ).toEqual([111111, 111111])
})
it("CurseForge addon-page malformed release is an explicit refusal rather than main-file fallback", async () => {
  const root = addonFixture("invalid")
  await flush()
  expect(root.querySelector("[role=alert]")?.textContent).toContain(
    "search:_trn_selected_version_unavailable"
  )
  expect(
    [...root.querySelectorAll("button")].some((b) =>
      b.textContent?.includes("instances:_trn_download")
    )
  ).toBe(false)
  expect(
    c.query.mock.calls.some(
      ([args]) => args[0] === "modplatforms.curseforge.getModFile"
    )
  ).toBe(false)
})
it("CurseForge addon-page failed file lookup gives what to do next, not the rspc payload", async () => {
  const payload = JSON.stringify({
    code: 500,
    message: JSON.stringify({
      cause: [{ display: "CurseForge could not be reached", debug: "Reqwest" }],
      backtrace: "   0: carbon_app::managers::modplatforms"
    })
  })
  const root = addonFixture("111111", async () => {
    throw new Error(payload)
  })
  await flush()
  const alert = root.querySelector("[role=alert]")?.textContent
  expect(alert).toContain("search:_trn_selected_version_failed")
  expect(alert).not.toContain("{")
  expect(alert).toContain("general:_trn_retry")
})
it("CurseForge addon-page wrong-owner refusal retries its file query and retains the recovered exact file", async () => {
  let wrongOwner = true
  const root = addonFixture("111111", async () => ({
    data: {
      id: 111111,
      modId: wrongOwner ? 987654 : 123456,
      isAvailable: true,
      serverPackFileId: 333333
    }
  }))
  await flush()
  expect(root.querySelector("[role=alert]")?.textContent).toContain(
    "search:_trn_selected_version_unavailable"
  )
  expect(c.creates).toEqual([])
  wrongOwner = false
  const retry = [...root.querySelectorAll("button")].find(
    (b) => b.textContent === "general:_trn_retry"
  )!
  retry.click()
  await flush()
  const button = [...root.querySelectorAll("button")].find((b) =>
    b.textContent?.includes("instances:_trn_download")
  )!
  button.click()
  await flush()
  expect(
    c.creates.find((row) => row.key === "instance.createInstance").input.version
      .Modpack.value.file_id
  ).toBe(111111)
  expect(
    c.query.mock.calls.filter(
      ([args]) => args[0] === "modplatforms.curseforge.getModFile"
    )
  ).toHaveLength(2)
})
it("each later routed q updates the persistent search and resets remembered/virtual scroll", async () => {
  defaults()
  const f = fixture()
  await flush()
  f.search.setSearchQuery((p) => ({ ...p, searchQuery: "manual" }))
  f.search.setLastScrollOffset(123)
  const scrollTo = vi.fn()
  f.search.setRef({ scrollTo } as any)
  f.navigate(
    "/search?q=" +
      encodeURIComponent("curseforge://install?addonId=123456&fileId=111111")
  )
  await flush()
  expect(f.search.searchQuery().searchQuery).toBe(
    "curseforge://install?addonId=123456&fileId=111111"
  )
  expect(f.params.q).toBeUndefined()
  expect(f.search.lastScrollOffset()).toBe(0)
  expect(scrollTo).toHaveBeenCalledWith(0)
  f.navigate("/search?q=sodium")
  await flush()
  expect(f.search.searchQuery().searchQuery).toBe("sodium")
  f.search.setSearchQuery((p) => ({ ...p, searchQuery: "manual again" }))
  await flush()
  f.navigate("/search?q=sodium")
  await flush()
  expect(f.search.searchQuery().searchQuery).toBe("sodium")
})
it.each(["curseforge", "modrinth"] as const)(
  "%s platform switch refetches effective twenty-row identity and continues at twenty",
  async (platform) => {
    defaults()
    const f = fixture()
    await flush()
    expect(f.search.hasNextPage()).toBe(true)
    f.search.setSearchQuery((p) => ({ ...p, searchApi: platform }))
    await flush()
    const query =
      platform === "curseforge"
        ? f.search.cfInfiniteResults
        : f.search.mrInfiniteResults
    expect(query.data?.pages[0].data).toHaveLength(20)
    expect(query.hasNextPage).toBe(true)
    await query.fetchNextPage()
    await flush()
    expect(
      c.query.mock.calls.some(
        ([args]) =>
          args[0] === "modplatforms.unifiedSearch" &&
          args[1].searchApi === platform &&
          args[1].index === 20 &&
          args[1].pageSize === 20
      )
    ).toBe(true)
  }
)
it.each(["list", "grid"] as const)(
  "accepted explicit CurseForge release reaches actual %s client download",
  async (layout) => {
    defaults()
    expect(
      parseSearchQuery("curseforge://install?addonId=123456&fileId=111111")
        .items[0]
    ).toMatchObject({ fileId: 111111 })
    const f = fixture(
      "/search?q=" +
        encodeURIComponent("curseforge://install?addonId=123456&fileId=111111"),
      layout
    )
    await flush()
    const button =
      [...f.root.querySelectorAll("button")].find((b) =>
        b.textContent?.includes("instances:_trn_download")
      ) ?? f.root.querySelector("button")!
    button.click()
    await flush()
    expect(
      c.creates.find((row) => row.key === "instance.createInstance").input
        .version.Modpack.value.file_id
    ).toBe(111111)
    expect(f.search.allRows()[0].value?.mainFileId).toBe("222222")
  }
)
it("unavailable requested file refuses direct result without main/latest fallback", async () => {
  defaults()
  const original = c.query.getMockImplementation()!
  c.query.mockImplementation((args: any) =>
    args[0] === "modplatforms.curseforge.getModFile"
      ? Promise.reject(new Error("requested file unavailable"))
      : original(args)
  )
  const f = fixture(
    "/search?q=" +
      encodeURIComponent("curseforge://install?addonId=123456&fileId=111111")
  )
  await flush()
  expect(f.search.allRows().filter((r) => r.value)).toEqual([])
  // The player reads a translated reason, not the lookup's raw error.
  expect(f.search.directErrors()).toEqual([
    "search:_trn_linked_pack_file_failed"
  ])
})
it("explicit server action uses the requested release companion rather than project main companion", async () => {
  defaults()
  const root = document.createElement("div")
  document.body.append(root)
  client = new QueryClient()
  dispose = render(
    () => <ServerPackDownloadButton addon={project()} fileId="111111" />,
    root
  )
  root.querySelector("button")!.click()
  await flush()
  expect(
    c.creates.find((row) => row.key === "server.createServerFromModpack").input
      .modpackSource.curseforge
  ).toEqual({
    project_id: 123456,
    file_id: 111111,
    server_pack_file_id: 333333
  })
})
it.each(["wrong-owner", "unavailable", "missing-companion", "rejected"])(
  "explicit server %s refuses creation and releases its clicked control",
  async (kind) => {
    defaults()
    c.query.mockImplementation(async () => {
      if (kind === "rejected") throw new Error("synthetic file unavailable")
      return {
        data: {
          id: 111111,
          modId: kind === "wrong-owner" ? 987654 : 123456,
          isAvailable: kind !== "unavailable",
          serverPackFileId: kind === "missing-companion" ? null : 333333
        }
      }
    })
    const root = document.createElement("div")
    document.body.append(root)
    client = new QueryClient()
    dispose = render(
      () => <ServerPackDownloadButton addon={project()} fileId="111111" />,
      root
    )
    const button = root.querySelector("button")!
    button.click()
    button.click()
    await flush()
    expect(c.creates).toEqual([])
    expect(button.disabled).toBe(false)
    expect(c.query).toHaveBeenCalledTimes(1)
  }
)
it("project-only search after explicit release keeps main selection without cache contamination", async () => {
  defaults()
  const f = fixture(
    "/search?q=" +
      encodeURIComponent("curseforge://install?addonId=123456&fileId=111111")
  )
  await flush()
  expect(f.search.allRows()[0].value?.requestedVersionId).toBe("111111")
  f.search.setSearchQuery((p) => ({ ...p, searchQuery: "#123456" }))
  await flush()
  expect(f.search.allRows()[0].value?.requestedVersionId).toBeUndefined()
  expect(f.search.allRows()[0].value?.mainFileId).toBe("222222")
})
it("old explicit lookup cannot publish into a replacement search", async () => {
  defaults()
  const original = c.query.getMockImplementation()!
  let resolve!: (value: any) => void
  c.query.mockImplementation((args: any) =>
    args[0] === "modplatforms.curseforge.getModFile"
      ? new Promise((r) => {
          resolve = r
        })
      : original(args)
  )
  const f = fixture(
    "/search?q=" +
      encodeURIComponent("curseforge://install?addonId=123456&fileId=111111")
  )
  await flush()
  f.search.setSearchQuery((p) => ({ ...p, searchQuery: "#123456" }))
  await flush()
  resolve({
    data: {
      id: 111111,
      modId: 123456,
      isAvailable: true,
      serverPackFileId: null
    }
  })
  await flush()
  expect(f.search.allRows()[0].value?.requestedVersionId).toBeUndefined()
})
it("Modrinth carried version result and typed batch provenance remain separate from project main", async () => {
  defaults()
  const original = c.query.getMockImplementation()!
  c.query.mockImplementation((args: any) =>
    args[0] === "modplatforms.unifiedGetProjectsByIds"
      ? Promise.resolve({
          results: [
            { ...project("modrinth", "owner"), requestedVersionId: "chosen" }
          ],
          errors: []
        })
      : original(args)
  )
  const f = fixture(
    "/search?q=" + encodeURIComponent("modrinth://version/chosen")
  )
  await flush()
  expect(
    c.query.mock.calls.find(
      ([args]) => args[0] === "modplatforms.unifiedGetProjectsByIds"
    )![0][1]
  ).toMatchObject({ modrinthVersionIds: ["chosen"], modrinthOnlyIds: [] })
  expect(f.search.allRows()[0].value).toMatchObject({
    id: "owner",
    requestedVersionId: "chosen",
    mainFileId: "222222"
  })
})
it.each([
  [
    { data: [1, 2], pagination: { index: 20, resultCount: 2, totalCount: 50 } },
    20,
    20,
    22
  ],
  [
    { data: [1, 2], pagination: { index: 20, resultCount: 2, totalCount: 22 } },
    20,
    20,
    undefined
  ],
  [
    { data: [], pagination: { index: 20, resultCount: 2, totalCount: 50 } },
    20,
    20,
    undefined
  ],
  [{ data: [1, 2] }, 20, 2, 22],
  [{ data: [1] }, 20, 2, undefined],
  [
    { data: [1], pagination: { index: 20, resultCount: 0, totalCount: 50 } },
    20,
    20,
    undefined
  ]
] as const)(
  "response continuation control %# uses offset/count/total or actual missing-pagination request",
  (page, offset, size, expected) => {
    expect(nextSearchOffset(page as any, offset, size)).toBe(expected)
  }
)
it("provider category/sort invalidation remains independent and combined mode restores fifteen-row cache", async () => {
  defaults()
  const f = fixture()
  await flush()
  c.query.mockClear()
  f.search.setSearchQuery((p) => ({ ...p, categories: [12] }))
  await flush()
  expect(
    c.query.mock.calls
      .filter(([args]) => args[0] === "modplatforms.unifiedSearch")
      .map(([args]) => args[1].searchApi)
  ).toEqual(["curseforge"])
  c.query.mockClear()
  f.search.setSearchQuery((p) => ({
    ...p,
    platformFilters: {
      platform: "curseforge",
      filters: { sort_field: "name", sort_order: "ascending" }
    }
  }))
  await flush()
  expect(
    c.query.mock.calls
      .filter(([args]) => args[0] === "modplatforms.unifiedSearch")
      .map(([args]) => args[1].searchApi)
  ).toEqual(["curseforge"])
  f.search.setSearchQuery((p) => ({
    ...p,
    searchApi: "modrinth",
    categories: null
  }))
  await flush()
  expect(f.search.mrInfiniteResults.data?.pages[0].data).toHaveLength(20)
  f.search.setSearchQuery((p) => ({ ...p, searchApi: null }))
  await flush()
  expect(f.search.mrInfiniteResults.data?.pages[0].data).toHaveLength(15)
  expect(f.search.hasNextPage()).toBe(true)
})
