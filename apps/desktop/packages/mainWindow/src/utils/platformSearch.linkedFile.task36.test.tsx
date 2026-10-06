// curseforge.com's Install buttons send curseforge://install links that pin
// CurseForge's main file (usually the newest Minecraft build). These tests
// drive such a link through the real search, row, instance dropdown, addon
// page and installer hook: the pin is kept only where the file fits the
// instance; elsewhere the instance gets its own version (2.0.40's path).
import { afterEach, expect, it, vi } from "vitest"
import { For } from "solid-js"
import { render } from "solid-js/web"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router"
import { getSearchResults } from "./platformSearch"
import { SearchInputContext } from "@/components/SearchInputContext"
import { List } from "@/pages/Search/List"
import AddonPage from "@/pages/AddonViewPage"
import RowContainer from "@/components/Browser/RowContainer"

const c = vi.hoisted(() => ({
  query: vi.fn(),
  mutations: [] as { key: string; input: any }[],
  instances: [] as any[]
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
      const raw = options().mutationKey
      const key = Array.isArray(raw) ? raw[0] : raw
      const record = async (input: any) => {
        c.mutations.push({ key, input })
        return 61
      }
      return {
        mutate: record,
        mutateAsync: record,
        isPending: false,
        isSuccess: false
      }
    }
  }
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    categories: { data: { curseforge: {}, modrinth: {} }, isLoading: false },
    instances: {
      get data() {
        return c.instances
      }
    }
  })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn(), prev: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@gd/i18n", () => ({
  // Render options so labels can be checked for the names they carry.
  Trans: (p: any) => {
    const { interpolation: _i, ...options } = p.options ?? {}
    return (
      <span data-key={p.key}>
        {p.key}
        {Object.keys(options).length ? JSON.stringify(options) : ""}
      </span>
    )
  },
  useTransContext: () => [(key: string) => key],
  i18n: { t: (key: string) => key }
}))
vi.mock("@gd/ui", () => {
  const passthrough = (p: any) => <div>{p.children}</div>
  return {
    Button: (p: any) => (
      <button
        data-variant={p.variant}
        data-testid={p["data-testid"]}
        disabled={p.disabled}
        onClick={p.onClick}
      >
        {p.children}
      </button>
    ),
    Tooltip: passthrough,
    TooltipTrigger: passthrough,
    TooltipContent: (p: any) => <div data-tooltip>{p.children}</div>,
    Spinner: () => null,
    Progress: () => null,
    Badge: passthrough,
    Skeleton: Object.assign(() => null, {
      searchList: () => null,
      searchListItem: () => null
    }),
    AuthorsSkeleton: () => null,
    Tabs: passthrough,
    TabsList: passthrough,
    TabsTrigger: passthrough,
    TabsIndicator: () => null,
    DropdownMenu: passthrough,
    DropdownMenuTrigger: passthrough,
    DropdownMenuContent: passthrough,
    DropdownMenuRadioGroup: passthrough,
    DropdownMenuRadioItem: (p: any) => (
      <div
        data-testid="instance-option"
        data-value={p.value}
        onClick={() => p.onSelect?.()}
      >
        {p.children}
      </div>
    ),
    Input: (p: any) => <input ref={p.ref} />,
    toast: { success: vi.fn(), error: vi.fn() }
  }
})
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => (
    <div>
      <For each={p.data}>{(item) => p.children(item)}</For>
    </div>
  )
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "fixture"
}))
vi.mock("@/pages/Search/DynamicBadgeContainer", () => ({
  default: () => null
}))
vi.mock("@/components/SharePreviewContent", () => ({ default: () => null }))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => null
}))
vi.mock("@/components/InfiniteScrollVersionsQueryWrapper", () => ({
  default: (p: any) => <>{p.children}</>
}))
vi.mock("@/components/ExploreVersionsNavbar", () => ({ default: () => null }))
vi.mock("@/components/ContentWrapper", () => ({
  default: (p: any) => <div id="gdl-content-wrapper">{p.children}</div>
}))
vi.mock("@/components/AuthorAvatars", () => ({ default: () => null }))
vi.mock("@/utils/sidebar", () => ({ ModloaderIcon: () => null }))

const PROJECT = 238222
const MAIN_FILE = 9207610
const LINK = `curseforge://install?addonId=${PROJECT}&fileId=${MAIN_FILE}`
const mainFile = {
  id: MAIN_FILE,
  modId: PROJECT,
  isAvailable: true,
  serverPackFileId: null,
  displayName: "jei-26.3-neoforge-31.14.0.72.jar",
  fileName: "jei-26.3-neoforge-31.14.0.72.jar",
  gameVersions: ["26.3", "NeoForge", "Client", "Server"]
}
const jei = (type = "mod"): any => ({
  platform: "curseforge",
  id: String(PROJECT),
  slug: "jei",
  title: "Just Enough Items",
  type,
  mainFileId: String(MAIN_FILE),
  serverPackFileId: null,
  categories: [],
  authors: [],
  loaders: [],
  minecraftVersions: [],
  downloadsCount: 1,
  description: "fixture",
  imageUrl: null,
  releaseDate: "2026-01-01",
  lastUpdated: "2026-01-01"
})
const instance = (
  id: number,
  name: string,
  mc_version: string | null,
  modloader: string | null
) => ({
  id,
  name,
  locked: false,
  icon_revision: null,
  status: {
    status: "valid",
    value: {
      mc_version,
      modloader,
      modloader_version: null,
      modpack: null,
      state: "notPersisted"
    }
  }
})
// The jar a compatible install put there earlier (a 1.20.1 Forge build).
const installedJei = {
  id: "local-jei",
  addon_type: "mods",
  enabled: true,
  curseforge: { project_id: PROJECT, file_id: 4712866 },
  modrinth: null
}

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
  c.mutations = []
  c.instances = []
  vi.unstubAllGlobals()
})

// Server 5 runs 1.20.1 Forge, server 6 runs 26.3 NeoForge.
const servers: Record<number, any> = {
  5: { id: 5, gameVersion: "1.20.1", modloaderType: "forge" },
  6: { id: 6, gameVersion: "26.3", modloaderType: "neoforge" }
}
// A JEI jar already on the server; server artifacts carry no version.
const serverJei = {
  id: "server-jei",
  filename: "jei-1.20.1-forge-15.20.0.106.jar",
  displayName: "jei-1.20.1-forge-15.20.0.106",
  enabled: true,
  addonType: "mods",
  fileSize: 1,
  hasImage: false,
  curseforgeProjectId: PROJECT,
  modrinthProjectId: null
}

function backend(
  opts: {
    mods?: any[]
    serverAddons?: any[]
    file?: any
    fileError?: Error
    type?: string
  } = {}
) {
  c.instances = [
    instance(1, "Forge 1.20.1", "1.20.1", "forge"),
    instance(2, "NeoForge 26.3", "26.3", "neoforge")
  ]
  c.query.mockImplementation(async ([key, input]: any) => {
    switch (key) {
      case "modplatforms.unifiedGetProjectsByIds":
        return { results: [jei(opts.type)], errors: [] }
      case "modplatforms.unifiedGetProject":
        return jei(opts.type)
      case "modplatforms.curseforge.getModFile":
        if (opts.fileError) throw opts.fileError
        return { data: opts.file ?? { ...mainFile, id: input.fileId } }
      case "instance.getInstanceMods":
        return opts.mods ?? []
      case "server.getServerDetails":
        return servers[input]
      case "server.getServerAddons":
        return opts.serverAddons ?? []
      case "instance.getInstanceDetails":
        return input === 2
          ? { version: "26.3", modloaders: [{ type_: "neoforge" }] }
          : { version: "1.20.1", modloaders: [{ type_: "forge" }] }
      case "settings.getSearchSidebarDocked":
        return true
      case "vtask.getTaskOutcome":
        return { type: "Succeeded" }
      default:
        return null
    }
  })
}

function mount(path: string, view: "list" | "grid" = "list") {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    }
  )
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  const root = document.createElement("div")
  document.body.append(root)
  const history = createMemoryHistory()
  history.set({ value: path, replace: true })
  const Search = () => {
    const search = getSearchResults()
    search.setViewMode(view)
    return (
      <SearchInputContext.Provider value={search}>
        <List />
      </SearchInputContext.Provider>
    )
  }
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/search" component={Search} />
          <Route path="/addon/:id/:platform" component={AddonPage} />
        </MemoryRouter>
      </QueryClientProvider>
    ),
    root
  )
  return root
}
const searchLink = (extra = "", view: "list" | "grid" = "list") =>
  mount(`/search?${extra}q=${encodeURIComponent(LINK)}`, view)
const option = (root: HTMLElement, id: number) =>
  root.querySelector<HTMLElement>(
    `[data-testid="instance-option"][data-value="${id}"]`
  )!
const installs = (key: string) =>
  c.mutations.filter((m) => m.key === key).map((m) => m.input)

it.each(["list", "grid"] as const)(
  "%s: choosing a 1.20.1 Forge instance for a 26.3 NeoForge link installs that instance's version",
  async (view) => {
    backend()
    const root = searchLink("", view)
    await flush()

    const forge = option(root, 1)
    const mismatch = forge.querySelector('[data-testid="linked-file-mismatch"]')
    expect(mismatch?.textContent).toContain("search:_trn_linked_file_mismatch")
    expect(mismatch?.textContent).toContain("26.3 · NeoForge")
    // Long release lists are shortened; the title spells every one out.
    expect(mismatch?.querySelector("[title]")?.getAttribute("title")).toBe(
      "26.3 · NeoForge"
    )
    expect(
      option(root, 2).querySelector('[data-testid="linked-file-mismatch"]')
    ).toBeNull()

    forge.click()
    await flush()
    expect(installs("instance.installLatestMod")).toEqual([
      { instance_id: 1, mod_source: { Curseforge: PROJECT } }
    ])
    expect(installs("instance.installMod")).toEqual([])
  }
)

it("choosing a 26.3 NeoForge instance keeps the linked file", async () => {
  backend()
  const root = searchLink()
  await flush()
  option(root, 2).click()
  await flush()
  expect(installs("instance.installMod")).toEqual([
    {
      mod_source: { Curseforge: { project_id: PROJECT, file_id: MAIN_FILE } },
      instance_id: 2,
      install_deps: true,
      replaces_mod: null
    }
  ])
  expect(installs("instance.installLatestMod")).toEqual([])
})

it("a Fabric file keeps its pin on a Quilt instance", async () => {
  backend({
    file: { ...mainFile, gameVersions: ["1.20.1", "Fabric"] }
  })
  c.instances = [instance(3, "Quilt 1.20.1", "1.20.1", "quilt")]
  const root = searchLink()
  await flush()
  expect(
    option(root, 3).querySelector('[data-testid="linked-file-mismatch"]')
  ).toBeNull()
  option(root, 3).click()
  await flush()
  expect(installs("instance.installMod")[0].mod_source.Curseforge).toEqual({
    project_id: PROJECT,
    file_id: MAIN_FILE
  })
})

it.each(["list", "grid"] as const)(
  "%s: the row names the linked file instead of its numeric ID",
  async (view) => {
    backend()
    const root = searchLink("", view)
    await flush()
    // GridItem cards are square; ListItem rows are not.
    expect(
      root
        .querySelector('[data-testid="search-result-row"]')!
        .classList.contains("aspect-square")
    ).toBe(view === "grid")
    const label = root.querySelector('[data-testid="linked-file-label"]')!
    expect(label.textContent).toContain("search:_trn_linked_file")
    expect(label.textContent).toContain("jei-26.3-neoforge-31.14.0.72.jar")
    expect(label.textContent).toContain("26.3 · NeoForge")
    expect(label.textContent).not.toContain(String(MAIN_FILE))
    expect(label.textContent).not.toContain("Selected version")
  }
)

it.each(["list", "grid"] as const)(
  "%s: in an instance search, an installed compatible copy shows Installed and is not switched to the linked file",
  async (view) => {
    backend({ mods: [installedJei] })
    const root = searchLink("instanceId=1&", view)
    await flush()
    const row = root.querySelector('[data-testid="search-result-row"]')!
    expect(row.textContent).toContain("instances:_trn_status_installed")
    expect(row.textContent).not.toContain("instances:_trn_switch_version")
    root.querySelectorAll("button").forEach((b) => b.click())
    await flush()
    expect(c.mutations).toEqual([])
  }
)

it("in an instance search that fits the linked file, the installed copy can still switch to it", async () => {
  backend({ mods: [installedJei] })
  const root = searchLink("instanceId=2&")
  await flush()
  const row = root.querySelector('[data-testid="search-result-row"]')!
  expect(row.textContent).toContain("instances:_trn_switch_version")
  row.querySelector("button")!.click()
  await flush()
  expect(installs("instance.installMod")).toEqual([
    {
      mod_source: { Curseforge: { project_id: PROJECT, file_id: MAIN_FILE } },
      instance_id: 2,
      install_deps: true,
      replaces_mod: "local-jei"
    }
  ])
})

const addonPage = (instanceId: number) =>
  mount(
    `/addon/${PROJECT}/curseforge?instanceId=${instanceId}&requestedVersionId=${MAIN_FILE}`
  )
const headerButtons = (root: HTMLElement) =>
  [...root.querySelectorAll<HTMLButtonElement>("button")].filter(
    (b) =>
      b.querySelector(".i-hugeicons\\:download-02") ||
      b.querySelector(".i-hugeicons\\:tick-02")
  )

it("the addon page header applies the same gate to a link's file", async () => {
  backend()
  const root = addonPage(1)
  await flush()
  const buttons = headerButtons(root)
  expect(buttons).toHaveLength(2)
  expect(
    root.querySelector('[data-testid="addon-install-button"]')?.textContent
  ).toContain("instances:_trn_download")
  expect(root.textContent).toContain("search:_trn_linked_file_mismatch")
  buttons[0].click()
  await flush()
  expect(installs("instance.installLatestMod")).toEqual([
    { instance_id: 1, mod_source: { Curseforge: PROJECT } }
  ])
  expect(installs("instance.installMod")).toEqual([])
})

it("the addon page header keeps a link's file that fits the instance", async () => {
  backend()
  const root = addonPage(2)
  await flush()
  expect(root.textContent).not.toContain("search:_trn_linked_file_mismatch")
  headerButtons(root)[0].click()
  await flush()
  expect(installs("instance.installMod")).toEqual([
    {
      mod_source: { Curseforge: { project_id: PROJECT, file_id: MAIN_FILE } },
      instance_id: 2,
      install_deps: true,
      replaces_mod: null
    }
  ])
})

it("the addon page header shows Installed for a compatible copy and does not replace it", async () => {
  backend({ mods: [installedJei] })
  const root = addonPage(1)
  await flush()
  const primary = root.querySelector<HTMLButtonElement>(
    '[data-testid="addon-install-button"]'
  )!
  const sticky = root
    .querySelector("button .i-hugeicons\\:tick-02")
    ?.closest("button")
  expect(primary.dataset.variant).toBe("green")
  expect(primary.textContent).toContain("content:_trn_mod.downloaded")
  expect(sticky?.dataset.variant).toBe("green")
  primary.click()
  sticky!.click()
  await flush()
  expect(c.mutations).toEqual([])
})

it("a Versions-tab pick still installs exactly that file into a mismatched instance", async () => {
  backend()
  client = new QueryClient()
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <RowContainer
          project={jei()}
          modVersion={
            {
              fileId: String(MAIN_FILE),
              name: mainFile.displayName,
              gameVersions: mainFile.gameVersions,
              datePublished: "2026-01-01",
              downloads: 1,
              releaseType: "release"
            } as any
          }
          instanceId={1}
          instanceMods={[]}
          type="mod"
          installedFile={undefined}
          loading={false}
          disabled={false}
        />
      </QueryClientProvider>
    ),
    root
  )
  await flush()
  root.querySelector("button")!.click()
  await flush()
  expect(installs("instance.installMod")).toEqual([
    {
      mod_source: { Curseforge: { project_id: PROJECT, file_id: MAIN_FILE } },
      instance_id: 1,
      install_deps: true,
      replaces_mod: null
    }
  ])
  expect(installs("instance.installLatestMod")).toEqual([])
})

// Server-scoped searches and addon pages apply the same gate, against the
// server's release and loader (2.0.40 installed the server's own version).
const firstRow = (root: HTMLElement) =>
  root.querySelector<HTMLElement>('[data-testid="search-result-row"]')!

it.each(["list", "grid"] as const)(
  "%s: in a server search, a link whose file does not fit the server installs the server's version",
  async (view) => {
    backend()
    const root = searchLink("serverId=5&", view)
    await flush()
    const row = firstRow(root)
    expect(row.textContent).toContain("instances:_trn_download")
    expect(row.textContent).not.toContain("instances:_trn_download_version")
    expect(row.textContent).toContain("search:_trn_linked_file_mismatch")
    row.querySelector("button")!.click()
    await flush()
    expect(installs("server.installLatestServerMod")).toEqual([
      { serverId: 5, modSource: { Curseforge: PROJECT } }
    ])
    expect(installs("server.installServerMod")).toEqual([])
  }
)

it.each(["list", "grid"] as const)(
  "%s: in a server search, a copy already on the server shows Installed for a link whose file does not fit",
  async (view) => {
    backend({ serverAddons: [serverJei] })
    const root = searchLink("serverId=5&", view)
    await flush()
    const row = firstRow(root)
    expect(row.textContent).toContain("instances:_trn_status_installed")
    expect(row.textContent).not.toContain("instances:_trn_switch_version")
    root.querySelectorAll("button").forEach((b) => b.click())
    await flush()
    expect(c.mutations).toEqual([])
  }
)

it("in a server search that fits the linked file, the install keeps it", async () => {
  backend()
  const root = searchLink("serverId=6&")
  await flush()
  const row = firstRow(root)
  expect(row.textContent).toContain("instances:_trn_download_version")
  expect(row.textContent).not.toContain("search:_trn_linked_file_mismatch")
  row.querySelector("button")!.click()
  await flush()
  expect(installs("server.installServerMod")).toEqual([
    {
      serverId: 6,
      modSource: { Curseforge: { project_id: PROJECT, file_id: MAIN_FILE } }
    }
  ])
  expect(installs("server.installLatestServerMod")).toEqual([])
})

// GDLauncher installs a server addon under CurseForge's file name, so a
// server copy named like the linked file is that file (2.0.40 showed it as
// Installed); a copy under another name can still switch to it.
const serverLinkedJei = {
  ...serverJei,
  filename: mainFile.fileName,
  displayName: "jei-26.3-neoforge-31.14.0.72"
}

it.each(["list", "grid"] as const)(
  "%s: in a server search that fits the linked file, a copy of that file shows Installed",
  async (view) => {
    backend({ serverAddons: [serverLinkedJei] })
    const root = searchLink("serverId=6&", view)
    await flush()
    const row = firstRow(root)
    expect(row.textContent).toContain("instances:_trn_status_installed")
    expect(row.textContent).not.toContain("instances:_trn_switch_version")
    root.querySelectorAll("button").forEach((b) => b.click())
    await flush()
    expect(c.mutations).toEqual([])
  }
)

it("in a server search that fits the linked file, a disabled copy of that file shows Installed", async () => {
  backend({
    serverAddons: [
      {
        ...serverLinkedJei,
        filename: `${mainFile.fileName}.disabled`,
        enabled: false
      }
    ]
  })
  const root = searchLink("serverId=6&")
  await flush()
  expect(firstRow(root).textContent).toContain(
    "instances:_trn_status_installed"
  )
})

it("in a server search that fits the linked file, a copy under another name can switch to it", async () => {
  backend({ serverAddons: [serverJei] })
  const root = searchLink("serverId=6&")
  await flush()
  const row = firstRow(root)
  expect(row.textContent).toContain("instances:_trn_switch_version")
  row.querySelector("button")!.click()
  await flush()
  expect(installs("server.installServerMod")).toEqual([
    {
      serverId: 6,
      modSource: { Curseforge: { project_id: PROJECT, file_id: MAIN_FILE } }
    }
  ])
})

it("the addon page header on a server shows Installed for a copy of the linked file and does not reinstall it", async () => {
  backend({ serverAddons: [serverLinkedJei] })
  const root = mount(
    `/addon/${PROJECT}/curseforge?serverId=6&requestedVersionId=${MAIN_FILE}`
  )
  await flush()
  const primary = root.querySelector<HTMLButtonElement>(
    '[data-testid="addon-install-button"]'
  )!
  expect(primary.dataset.variant).toBe("green")
  expect(primary.textContent).not.toContain("instances:_trn_switch_version")
  headerButtons(root).forEach((b) => b.click())
  await flush()
  expect(c.mutations).toEqual([])
})

it("the addon page header applies the same gate on a server", async () => {
  backend()
  const root = mount(
    `/addon/${PROJECT}/curseforge?serverId=5&requestedVersionId=${MAIN_FILE}`
  )
  await flush()
  const primary = root.querySelector<HTMLButtonElement>(
    '[data-testid="addon-install-button"]'
  )!
  expect(primary.textContent).toContain("instances:_trn_download")
  expect(primary.textContent).not.toContain("instances:_trn_download_version")
  expect(root.textContent).toContain("search:_trn_linked_file_mismatch")
  primary.click()
  await flush()
  expect(installs("server.installLatestServerMod")).toEqual([
    { serverId: 5, modSource: { Curseforge: PROJECT } }
  ])
  expect(installs("server.installServerMod")).toEqual([])
})

// A link whose file is gone keeps the project for non-modpacks, unpinned, as
// 2.0.40 showed it; only a modpack link needs its exact file.
const unloadable = [
  ["a failed lookup", { fileError: new Error("synthetic lookup failure") }],
  ["an unavailable file", { file: { ...mainFile, isAvailable: false } }],
  ["another project's file", { file: { ...mainFile, modId: 1 } }]
] as const

it.each(unloadable)(
  "a mod link with %s keeps the project row, unpinned",
  async (_, opts) => {
    backend(opts)
    const root = searchLink()
    await flush()
    expect(
      root.querySelectorAll('[data-testid="search-result-row"]')
    ).toHaveLength(1)
    expect(root.querySelector('[role="alert"]')).toBeNull()
    expect(
      firstRow(root).querySelector('[data-testid="linked-file-unavailable"]')
        ?.textContent
    ).toContain("search:_trn_linked_file_unavailable")
    expect(root.querySelector('[data-testid="linked-file-label"]')).toBeNull()
    expect(
      root.querySelector('[data-testid="linked-file-mismatch"]')
    ).toBeNull()
    option(root, 2).click()
    await flush()
    expect(installs("instance.installLatestMod")).toEqual([
      { instance_id: 2, mod_source: { Curseforge: PROJECT } }
    ])
    expect(installs("instance.installMod")).toEqual([])
  }
)

it("in an instance search, a mod link whose file is gone shows an installed copy as Installed", async () => {
  backend({
    fileError: new Error("synthetic lookup failure"),
    mods: [installedJei]
  })
  const root = searchLink("instanceId=2&")
  await flush()
  expect(firstRow(root).textContent).toContain(
    "instances:_trn_status_installed"
  )
})

it("a modpack link whose file is gone still offers no row", async () => {
  backend({ fileError: new Error("synthetic lookup failure"), type: "modpack" })
  const root = searchLink()
  await flush()
  expect(root.querySelector('[data-testid="search-result-row"]')).toBeNull()
  // The cause is logged; the player gets the translated explanation.
  expect(root.querySelector('[role="alert"]')?.textContent).toContain(
    "search:_trn_linked_pack_file_failed"
  )
})

it("the addon page drops a mod link's file that cannot be loaded and says so", async () => {
  backend({ fileError: new Error("synthetic lookup failure") })
  const root = addonPage(1)
  await flush()
  expect(
    root.querySelector('[data-testid="linked-file-unavailable"]')?.textContent
  ).toContain("search:_trn_linked_file_unavailable")
  expect(root.textContent).not.toContain("Retry selected version")
  root
    .querySelector<HTMLButtonElement>('[data-testid="addon-install-button"]')!
    .click()
  await flush()
  expect(installs("instance.installLatestMod")).toEqual([
    { instance_id: 1, mod_source: { Curseforge: PROJECT } }
  ])
  expect(installs("instance.installMod")).toEqual([])
})

// Modrinth version links keep their pin; the row names the version.
function modrinthVersionLink(version: any) {
  c.query.mockImplementation(async ([key]: any) => {
    switch (key) {
      case "modplatforms.unifiedGetProjectsByIds":
        return {
          results: [
            {
              ...jei(),
              platform: "modrinth",
              id: "u6dRKJwZ",
              requestedVersionId: "IZskON6d"
            }
          ],
          errors: []
        }
      case "modplatforms.modrinth.getVersion":
        if (!version) throw new Error("synthetic lookup failure")
        return version
      case "settings.getSearchSidebarDocked":
        return true
      default:
        return null
    }
  })
  return mount(`/search?q=${encodeURIComponent("modrinth://version/IZskON6d")}`)
}

it("a Modrinth version link names the version instead of its ID", async () => {
  const root = modrinthVersionLink({
    id: "IZskON6d",
    project_id: "u6dRKJwZ",
    name: "JEI 15.20.0.106",
    version_number: "15.20.0.106",
    game_versions: ["1.20.1"],
    loaders: ["forge"]
  })
  await flush()
  const label = root.querySelector('[data-testid="linked-file-label"]')!
  expect(label.textContent).toContain("JEI 15.20.0.106")
  expect(label.textContent).toContain("1.20.1 · Forge")
  expect(label.textContent).not.toContain("IZskON6d")
})

it("the addon page keeps a Modrinth version link's pin when its version cannot be loaded", async () => {
  c.instances = [instance(1, "Forge 1.20.1", "1.20.1", "forge")]
  c.query.mockImplementation(async ([key]: any) => {
    switch (key) {
      case "modplatforms.unifiedGetProject":
        return { ...jei(), platform: "modrinth", id: "u6dRKJwZ" }
      case "modplatforms.modrinth.getVersion":
        throw new Error("synthetic lookup failure")
      case "instance.getInstanceMods":
        return []
      case "settings.getSearchSidebarDocked":
        return true
      default:
        return null
    }
  })
  const root = mount(
    "/addon/u6dRKJwZ/modrinth?instanceId=1&requestedVersionId=IZskON6d"
  )
  await flush()
  // The deliberate pick is not swapped for the latest version: the page says
  // what failed and offers Retry, as before.
  expect(
    root.querySelector('[data-testid="linked-file-unavailable"]')
  ).toBeNull()
  expect(root.querySelector('[data-testid="addon-install-button"]')).toBeNull()
  const alert = root.querySelector('[role="alert"]')!
  // The global toast carries the cause; the page says what to do next.
  expect(alert.textContent).toContain("search:_trn_selected_version_failed")
  expect(alert.textContent).not.toContain("synthetic lookup failure")
  const retry = [...alert.querySelectorAll("button")].find((b) =>
    b.textContent?.includes("general:_trn_retry")
  )
  expect(retry).toBeDefined()
  retry!.click()
  await flush()
  expect(c.mutations).toEqual([])
})

it("a Modrinth version link whose version cannot be loaded shows no raw ID", async () => {
  const root = modrinthVersionLink(undefined)
  await flush()
  const label = root.querySelector('[data-testid="linked-file-label"]')!
  expect(label.textContent).toContain("search:_trn_linked_version")
  expect(label.textContent).not.toContain("IZskON6d")
})
