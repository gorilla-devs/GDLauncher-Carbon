import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal, Show } from "solid-js"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import {
  MemoryRouter,
  Route,
  createMemoryHistory,
  useLocation,
  useNavigate
} from "@solidjs/router"
import { getSearchResults } from "@/utils/platformSearch"
import { SearchInputContext } from "@/components/SearchInputContext"
import { AddonTypeDropdown } from "@/components/AddonTypeDropdown"
import { Grid } from "./Grid"
import { List } from "./List"

const io = vi.hoisted(() => ({ query: vi.fn(), navigation: vi.fn() }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { query: io.query } }),
    createQuery: (options: any) =>
      createQuery(() => {
        const opts = options()
        return {
          ...opts,
          queryFn: opts.queryFn ?? (() => io.query(opts.queryKey))
        }
      }),
    createMutation: () => ({ mutate: vi.fn() })
  }
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => {
    const navigate = useNavigate()
    return {
      navigate: (path: string, options?: any) => {
        io.navigation(path, options)
        navigate(path, options)
      }
    }
  }
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string) =>
      ({
        "search:_trn_resource_packs": "Resource packs",
        "search:_trn_modpacks": "Modpacks",
        "search:_trn_mods": "Mods",
        "search:_trn_shaders": "Shaders",
        "search:_trn_data_packs": "Data packs",
        "search:_trn_worlds": "Worlds"
      })[key] ?? key
  ],
  Trans: (p: any) => <span>{p.key}</span>
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../packages/ui/src/DropdownMenu")),
  Skeleton: Object.assign(() => <span />, { searchList: () => <span /> })
}))
// Empty-result route tests exclude install/image/share rendering and native virtualization.
vi.mock("./GridItem", () => ({ GridItem: () => <span /> }))
vi.mock("./ListItem", () => ({ ListItem: () => <span /> }))
vi.mock("@/components/SharePreviewContent", () => ({ default: () => <span /> }))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => <span />
}))
vi.mock("@/components/VirtuaWrapper", () => ({ VList: () => <span /> }))

let dispose: (() => void) | undefined
let client: QueryClient
const flush = async () => {
  for (let i = 0; i < 15; i++) {
    await Promise.resolve()
    await vi.advanceTimersByTimeAsync(0)
  }
}
beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  vi.spyOn(window, "scrollTo").mockImplementation(() => {})
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    }
  )
  io.query.mockImplementation(async ([key]: any) => {
    if (key === "modplatforms.unifiedSearch")
      return { data: [], pagination: null }
    if (key === "settings.getSearchSidebarDocked") return true
    if (key === "instance.getInstanceDetails")
      return { modloaders: [{ type_: "fabric" }], version: "1.21" }
    if (key === "server.getServerDetails")
      return { modloaderType: null, gameVersion: "1.21" }
    if (key === "instance.getInstanceMods" || key === "server.getServerAddons")
      return []
    throw new Error(`Unexpected fixture query ${key}`)
  })
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  client?.clear()
  vi.clearAllTimers()
  vi.useRealTimers()
  document.body.textContent = ""
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})
function fixture(
  mode: "grid" | "list",
  initial: string,
  previous: any = "modpack"
) {
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  let search!: ReturnType<typeof getSearchResults>,
    navigate!: ReturnType<typeof useNavigate>,
    location!: ReturnType<typeof useLocation>,
    mountView!: () => void
  const Content = () => {
    navigate = useNavigate()
    location = useLocation()
    search = getSearchResults()
    const [ready, setReady] = createSignal(false)
    mountView = () => setReady(true)
    search.setViewMode("list")
    return (
      <SearchInputContext.Provider value={search}>
        <AddonTypeDropdown />
        <Show when={ready()}>{mode === "grid" ? <Grid /> : <List />}</Show>
      </SearchInputContext.Provider>
    )
  }
  const root = document.createElement("div")
  document.body.append(root)
  const history = createMemoryHistory()
  history.set({ value: initial, replace: true })
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/search/:type?" component={Content} />
        </MemoryRouter>
      </QueryClientProvider>
    ),
    root
  )
  search.setSearchQuery((p) => ({ ...p, projectType: previous }))
  mountView()
  return {
    root,
    get search() {
      return search
    },
    get navigate() {
      return navigate
    },
    get location() {
      return location
    }
  }
}
const payloads = () =>
  io.query.mock.calls
    .filter(([args]) => args[0] === "modplatforms.unifiedSearch")
    .map(([args]) => args[1])
const label = (root: HTMLElement) =>
  root.querySelector('[data-testid="addon-type-dropdown-trigger"]')?.textContent
async function expectType(
  f: ReturnType<typeof fixture>,
  type: string,
  text: string
) {
  await flush()
  await vi.advanceTimersByTimeAsync(500)
  await flush()
  expect(f.search.searchQuery().projectType).toBe(type)
  expect(label(f.root)).toBe(text)
  for (const platform of ["curseforge", "modrinth"])
    expect(
      payloads()
        .filter((p) => p.searchApi === platform)
        .at(-1)?.projectType
    ).toBe(type)
}

it.each(["grid", "list"] as const)(
  "%s canonical Resource packs wins over prior query and reaches both provider boundaries",
  async (mode) => {
    const f = fixture(mode, "/search/resourcePack?instanceId=7", "modpack")
    await expectType(f, "resourcePack", "Resource packs")
    expect(f.location.pathname).toBe("/search/resourcePack")
    expect(f.location.search).toBe("?instanceId=7")
    expect(
      payloads().every((p) =>
        ["modpack", "resourcePack"].includes(p.projectType)
      )
    ).toBe(true)
  }
)
it.each(["grid", "list"] as const)(
  "%s legacy Resource packs is canonical in URL, label, query and provider payloads",
  async (mode) => {
    const f = fixture(
      mode,
      "/search/resource-pack?instanceId=7&keep=yes",
      "modpack"
    )
    await flush()
    await vi.advanceTimersByTimeAsync(500)
    await flush()
    expect.soft(f.search.searchQuery().projectType).toBe("resourcePack")
    expect.soft(label(f.root)).toBe("Resource packs")
    for (const platform of ["curseforge", "modrinth"])
      expect
        .soft(
          payloads()
            .filter((p) => p.searchApi === platform)
            .at(-1)?.projectType
        )
        .toBe("resourcePack")
    expect.soft(f.location.pathname).toBe("/search/resourcePack")
    expect.soft(f.location.search).toBe("?instanceId=7&keep=yes")
    expect
      .soft(io.navigation)
      .toHaveBeenCalledWith("/search/resourcePack?instanceId=7&keep=yes", {
        replace: true
      })
    expect
      .soft(payloads().some((p) => p.projectType === "resource-pack"))
      .toBe(false)
  }
)
it.each(
  ["grid", "list"].flatMap((mode) =>
    ["bogus", "toString", "constructor", "__proto__", "plugin", "unknown"].map(
      (type) => [mode, type] as const
    )
  )
)(
  "%s invalid %s visibly falls back without malformed request state",
  async (mode, type) => {
    const f = fixture(
      mode as "grid" | "list",
      `/search/${type}?keep=yes`,
      "shader"
    )
    await expectType(f, "shader", "Shaders")
    expect(f.location.pathname).toBe("/search/shader")
    expect(f.location.search).toBe("?keep=yes")
    expect(
      payloads().every((p) => ["modpack", "shader"].includes(p.projectType))
    ).toBe(true)
  }
)
it.each(["grid", "list"] as const)(
  "%s missing standalone segment retains valid previous query",
  async (mode) => {
    const f = fixture(mode, "/search", "resourcePack")
    await expectType(f, "resourcePack", "Resource packs")
    expect(f.location.pathname).toBe("/search")
  }
)
it.each(["grid", "list"] as const)(
  "%s missing target segment resyncs as delayed modded metadata resolves",
  async (mode) => {
    let finish!: (data: any) => void
    io.query.mockImplementation(async ([key]: any) => {
      if (key === "instance.getInstanceDetails")
        return new Promise((resolve) => {
          finish = resolve
        })
      if (key === "settings.getSearchSidebarDocked") return true
      if (key === "modplatforms.unifiedSearch")
        return { data: [], pagination: null }
      return []
    })
    const f = fixture(mode, "/search?instanceId=7", "modpack")
    await expectType(f, "shader", "Shaders")
    finish({ modloaders: [{ type_: "fabric" }], version: "1.21" })
    await expectType(f, "mod", "Mods")
    expect(f.location.pathname).toBe("/search")
  }
)
it.each(["grid", "list"] as const)(
  "%s explicit mod keeps precedence while instance metadata is pending",
  async (mode) => {
    const ordinary = io.query.getMockImplementation()!
    io.query.mockImplementation((args: any) =>
      args[0] === "instance.getInstanceDetails"
        ? new Promise(() => {})
        : ordinary(args)
    )
    const f = fixture(mode, "/search/mod?instanceId=7")
    await flush()
    await vi.advanceTimersByTimeAsync(500)
    await flush()
    expect(f.search.searchQuery().projectType).toBe("mod")
    expect(
      payloads()
        .filter((p) => p.projectType === "mod")
        .map((p) => p.searchApi)
    ).toEqual(expect.arrayContaining(["curseforge", "modrinth"]))
    expect(f.location.pathname).toBe("/search/mod")
  }
)
it.each(["grid", "list"] as const)(
  "%s server defaults and dropdown retain datapack-only vanilla capability",
  async (mode) => {
    const f = fixture(mode, "/search?serverId=42", "resourcePack")
    await expectType(f, "datapack", "Data packs")
    expect(f.search.allowedAddonTypes()).toEqual(["datapack"])
    client.setQueryData(["server.getServerDetails", 42], {
      modloaderType: "fabric",
      gameVersion: "1.21"
    })
    await expectType(f, "mod", "Mods")
    expect(f.search.allowedAddonTypes()).toEqual(["mod", "datapack"])
  }
)
it.each(["grid", "list"] as const)(
  "%s actual dropdown selection emits resourcePack and preserves target query",
  async (mode) => {
    const f = fixture(mode, "/search/mod?instanceId=7&keep=yes")
    await flush()
    const trigger = f.root.querySelector<HTMLElement>(
      '[data-testid="addon-type-dropdown-trigger"]'
    )!
    trigger.dispatchEvent(
      new MouseEvent("pointerdown", { button: 0, bubbles: true })
    )
    trigger.click()
    await flush()
    document
      .querySelector<HTMLElement>('[data-addon-type="resourcePack"]')!
      .click()
    await expectType(f, "resourcePack", "Resource packs")
    expect(f.location.pathname).toBe("/search/resourcePack")
    expect(f.location.search).toBe("?instanceId=7&keep=yes")
  }
)
it.each(["grid", "list"] as const)(
  "%s reactive explicit route changes and target defaults stay in sync",
  async (mode) => {
    const f = fixture(mode, "/search/mod?instanceId=7")
    await expectType(f, "mod", "Mods")
    f.navigate("/search/resource-pack?instanceId=7&keep=yes")
    await expectType(f, "resourcePack", "Resource packs")
    expect(f.location.pathname).toBe("/search/resourcePack")
    f.navigate("/search?serverId=42")
    await expectType(f, "datapack", "Data packs")
    expect(f.search.allowedAddonTypes()).toEqual(["datapack"])
    expect(
      payloads().every((p) =>
        ["modpack", "mod", "resourcePack", "datapack"].includes(p.projectType)
      )
    ).toBe(true)
  }
)
