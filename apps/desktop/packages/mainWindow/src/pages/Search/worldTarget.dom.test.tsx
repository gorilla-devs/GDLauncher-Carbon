import { afterEach, expect, it, vi } from "vitest"
import { For } from "solid-js"
import { render } from "solid-js/web"
import { List } from "./List"
import { Grid } from "./Grid"
import ModDownloadButton from "@/components/ModDownloadButton"
import AddonPage from "@/pages/AddonViewPage"
import RowContainer from "@/components/Browser/RowContainer"

// The real search list, grid, addon page, version row and download button
// with a world as the target. Only transport, router and leaf UI are stubbed.
const control = vi.hoisted(() => ({
  context: undefined as any,
  mods: [] as any[],
  worlds: [] as any[],
  project: undefined as any,
  params: {} as any,
  searchParams: {} as any,
  navigate: vi.fn(),
  mutations: [] as { key: string; input: any }[]
}))
vi.mock("@/components/SearchInputContext", () => ({
  default: () => control.context
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    categories: { data: { curseforge: {}, modrinth: {} }, isLoading: false },
    instances: { data: [] }
  })
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => control.params,
  useSearchParams: () => [control.searchParams],
  useLocation: () => ({ pathname: "/addon/P/modrinth" })
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { invalidateQueries: vi.fn() },
  rspc: {
    useContext: () => ({
      client: {
        query: vi.fn(() => {
          throw new Error("Unexpected direct query")
        })
      }
    }),
    createQuery: (options: any) => {
      const key = options().queryKey[0]
      const data =
        key === "instance.getInstanceMods"
          ? control.mods
          : key === "instance.getWorlds"
            ? control.worlds
            : key === "modplatforms.unifiedGetProject"
              ? control.project
              : undefined
      return { data, isLoading: false, refetch: vi.fn() }
    },
    createMutation: (options: any) => ({
      isPending: false,
      isSuccess: false,
      mutateAsync: async (input: any) => {
        control.mutations.push({ key: options().mutationKey, input })
        return 7
      }
    })
  }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: control.navigate, prev: vi.fn() })
}))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => (
    <div>
      <For each={p.data}>{(item) => p.children(item)}</For>
    </div>
  )
}))
vi.mock("@/components/SharePreviewContent", () => ({ default: () => null }))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => null
}))
vi.mock("./DynamicBadgeContainer", () => ({ default: () => null }))
vi.mock("@/components/ModDownloadButton/components/InstanceDropdown", () => ({
  InstanceDropdown: () => <span>instance dropdown</span>
}))
vi.mock("@/components/ExploreVersionsNavbar", () => ({ default: () => null }))
vi.mock("@/components/AuthorAvatars", () => ({ default: () => null }))
vi.mock("@/components/ContentWrapper", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/components/InfiniteScrollVersionsQueryWrapper", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "fixture",
  CategoryIcon: () => null
}))
vi.mock("@/utils/sidebar", () => ({ ModloaderIcon: () => null }))
vi.mock("@/utils/helpers", () => ({
  formatDownloadCount: String,
  getModloaderDisplayName: String
}))
vi.mock("@/utils/date", () => ({ safeFormat: String }))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", async () => {
  const { createContext, useContext, Show } = await import("solid-js")
  const passthrough = (p: any) => <div>{p.children}</div>
  // A menu that opens and closes as its owner says.
  const Menu = createContext<any>()
  return {
    DropdownMenu: (p: any) => (
      <Menu.Provider value={p}>{p.children}</Menu.Provider>
    ),
    DropdownMenuTrigger: (p: any) => {
      const menu = useContext(Menu)
      return (
        <div data-menu-trigger onClick={() => menu.onOpenChange(!menu.open)}>
          {p.children}
        </div>
      )
    },
    DropdownMenuContent: (p: any) => {
      const menu = useContext(Menu)
      return <Show when={menu.open}>{p.children}</Show>
    },
    DropdownMenuItem: (p: any) => (
      <button data-menu-item disabled={p.disabled} onClick={p.onSelect}>
        {p.children}
      </button>
    ),
    DropdownMenuSeparator: () => <hr />,
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
    TooltipContent: passthrough,
    Tabs: passthrough,
    TabsList: passthrough,
    TabsTrigger: passthrough,
    TabsIndicator: () => null,
    Badge: passthrough,
    AuthorsSkeleton: () => null,
    Skeleton: Object.assign(() => null, { searchList: () => null }),
    Spinner: () => null,
    Progress: () => null,
    toast: { success: vi.fn(), error: vi.fn() }
  }
})
vi.stubGlobal(
  "ResizeObserver",
  class {
    observe() {}
    disconnect() {}
  }
)
let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  control.mutations = []
  control.mods = []
  control.worlds = []
  control.params = {}
  control.searchParams = {}
  control.project = undefined
  control.context = undefined
  control.navigate.mockReset()
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
const result = (version?: string, type = "datapack") => ({
  id: "P",
  platform: "modrinth",
  type,
  slug: "pack",
  title: "Pack",
  description: "fixture",
  categories: [],
  authors: [],
  loaders: [],
  minecraftVersions: [],
  downloadsCount: 1,
  requestedVersionId: version,
  imageUrl: null,
  highResImageUrl: null,
  mainFileId: "latest",
  serverPackFileId: null
})
const instancePack = (version = "V1") => ({
  id: "instance-copy",
  addon_type: "datapacks",
  enabled: true,
  modrinth: { project_id: "P", version_id: version },
  curseforge: null
})
const worldPack = (version = "V1", id = "world-copy") => ({
  id,
  filename: `${id}.zip`,
  file_size: 1,
  is_folder: false,
  name: null,
  version: null,
  has_update: false,
  modrinth: { project_id: "P", version_id: version },
  curseforge: null
})
const world = (
  name: string,
  datapacks: any[] = [],
  identity: string | null = `id-${name}`
) => ({ name, addon_id: `addon-${name}`, identity, datapacks })
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
// What the platforms really return: Modrinth types a data pack "mod" and
// lists "datapack" among its loaders, here beside the loaders of its mod jar.
const hybrid = {
  type: "mod",
  loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
}
const plainMod = { type: "mod", loaders: ["fabric"] }
function search(
  view: string,
  requested: (string | undefined)[],
  target: string | null = "My World",
  shape: object = {}
) {
  const worldName = target ?? undefined
  control.params = { type: "datapack" }
  control.context = {
    allRows: () =>
      requested.map((v) => ({
        type: "value",
        value: { ...result(v), ...shape }
      })),
    selectedInstanceId: () => 1,
    selectedInstanceMods: { data: control.mods },
    selectedServerId: () => undefined,
    selectedServerAddons: { data: [] },
    selectedWorld: {
      get name() {
        return worldName
      },
      get data() {
        return control.worlds.find((w) => w.name === worldName)
      }
    },
    defaultAddonType: () => "datapack",
    searchQuery: () => ({ projectType: "datapack" }),
    setSearchQuery: vi.fn(),
    directErrors: () => [],
    viewMode: () => view,
    isShareMode: () => false,
    isInitialLoading: () => false,
    setRef: vi.fn(),
    lastScrollOffset: () => 0,
    virtualOnScrollHandler: vi.fn()
  }
  return mount(() => (view === "list" ? <List /> : <Grid />))
}
const installButtons = (root: HTMLElement) =>
  [...root.querySelectorAll<HTMLButtonElement>("button")].filter((b) =>
    b.querySelector(".i-hugeicons\\:download-02")
  )

it.each(["list", "grid"])(
  "%s Download installs into the world with its identity, whatever the instance holds",
  async (view) => {
    control.mods = [instancePack()]
    control.worlds = [world("Other", [worldPack()]), world("My World")]
    const root = search(view, [undefined])
    expect(root.textContent).not.toContain("status_installed")
    const [button] = installButtons(root)
    expect(button.disabled).toBe(false)
    button.click()
    await flush()
    expect(control.mutations).toEqual([
      {
        key: "instance.installWorldDatapack",
        input: {
          instance_id: 1,
          world: "My World",
          identity: "id-My World",
          source: { Modrinth: { project_id: "P", version_id: null } },
          replaces: null
        }
      }
    ])
  }
)
it.each(["list", "grid"])(
  "%s shows a pack the world holds as installed even when the instance has none",
  (view) => {
    control.worlds = [world("My World", [worldPack()])]
    const root = search(view, [undefined])
    expect(installButtons(root)).toHaveLength(0)
    expect(root.textContent).toContain("status_installed")
  }
)
it("the exact version counts as installed for the world holding it and not for another world", () => {
  control.worlds = [world("My World", [worldPack("V1")]), world("Other")]
  let root = search("list", ["V1"], "My World")
  expect(installButtons(root)).toHaveLength(0)
  dispose?.()
  document.body.textContent = ""
  root = search("list", ["V1"], "Other")
  expect(installButtons(root)).toHaveLength(1)
})
it("Download of another version of a pack the world holds replaces that copy", async () => {
  control.worlds = [world("My World", [worldPack("V1")])]
  const root = search("list", ["V2"])
  expect(root.textContent).toContain("switch_version")
  installButtons(root)[0].click()
  await flush()
  expect(control.mutations[0]).toEqual({
    key: "instance.installWorldDatapack",
    input: {
      instance_id: 1,
      world: "My World",
      identity: "id-My World",
      source: { Modrinth: { project_id: "P", version_id: "V2" } },
      replaces: "world-copy"
    }
  })
})
it.each([
  ["a world that no longer exists", [world("Other")]],
  ["an unreadable world", [world("My World", [], null)]]
])("%s disables the data pack button and installs nothing", async (_, list) => {
  control.mods = []
  control.worlds = list
  const root = search("list", [undefined])
  const [button] = installButtons(root)
  expect(button.disabled).toBe(true)
  button.click()
  await flush()
  expect(control.mutations).toEqual([])
})
// CurseForge types a data pack "datapack" and lists no loader for it.
const curseforgePack = {
  id: "12",
  platform: "curseforge",
  type: "datapack",
  loaders: []
}
const menuItem = (root: HTMLElement, text: string) =>
  [...root.querySelectorAll<HTMLButtonElement>("[data-menu-item]")].find(
    (item) => item.textContent?.includes(text)
  )!
it.each(["list", "grid"])(
  "%s Download with an instance only asks where, and Download to instance installs into the instance",
  async (view) => {
    control.worlds = [world("My World")]
    const root = search(view, [undefined], null, curseforgePack)
    installButtons(root)[0].click()
    await flush()
    expect(control.mutations).toEqual([])
    menuItem(root, "download_to_instance").click()
    await flush()
    expect(control.mutations).toEqual([
      {
        key: "instance.installLatestMod",
        input: { instance_id: 1, mod_source: { Curseforge: 12 } }
      }
    ])
  }
)
it.each(["list", "grid"])(
  "%s Download with an instance only installs into the world picked there",
  async (view) => {
    control.worlds = [world("My World")]
    const root = search(view, [undefined], null, hybrid)
    installButtons(root)[0].click()
    await flush()
    menuItem(root, "My World").click()
    await flush()
    expect(control.mutations).toEqual([
      {
        key: "instance.installWorldDatapack",
        input: {
          instance_id: 1,
          world: "My World",
          identity: "id-My World",
          source: { Modrinth: { project_id: "P", version_id: null } },
          replaces: null
        }
      }
    ])
  }
)
it("with an instance only and no world, a data pack still downloads into the instance at once", async () => {
  const root = search("list", [undefined], null, curseforgePack)
  installButtons(root)[0].click()
  await flush()
  expect(control.mutations).toEqual([
    {
      key: "instance.installLatestMod",
      input: { instance_id: 1, mod_source: { Curseforge: 12 } }
    }
  ])
})
it("opening a data pack from a world search keeps the world on the addon page", () => {
  control.worlds = [world("My World")]
  const root = search("list", [undefined])
  root.querySelector<HTMLElement>('[data-testid="search-result-row"]')!.click()
  expect(control.navigate).toHaveBeenCalledWith(
    "/addon/P/modrinth?instanceId=1&world=My%20World"
  )
})
it.each(["mod", "resourcePack", "shader", "world"])(
  "a %s ignores the world and installs into the instance as before",
  async (type) => {
    control.worlds = [world("My World")]
    mount(() => (
      <ModDownloadButton
        addon={result(undefined, type) as any}
        selectedInstanceId={1}
        selectedInstanceMods={[]}
        selectedWorld="My World"
      />
    ))
    document.querySelector<HTMLButtonElement>("button")!.click()
    await flush()
    expect(control.mutations.map((m) => m.key)).toEqual([
      "instance.installLatestMod"
    ])
  }
)
it("a world without an instance is ignored: the instance picker is shown", () => {
  control.worlds = [world("My World")]
  const root = mount(() => (
    <ModDownloadButton
      addon={result() as any}
      selectedInstanceId={NaN}
      selectedWorld="My World"
    />
  ))
  expect(root.textContent).toContain("instance dropdown")
})
it("a server target is unchanged by a world", async () => {
  control.worlds = [world("My World")]
  mount(() => (
    <ModDownloadButton
      addon={result() as any}
      selectedServerId={9}
      selectedServerAddons={[]}
      selectedWorld="My World"
    />
  ))
  document.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations.map((m) => m.key)).toEqual([
    "server.installLatestServerMod"
  ])
})
it("the versions page installs the chosen version into the world, replacing its copy", async () => {
  control.worlds = [world("My World", [worldPack("V1")])]
  const root = mount(() => (
    <RowContainer
      project={result() as any}
      modVersion={
        {
          fileId: "V3",
          name: "V3",
          gameVersions: [],
          datePublished: "fixture",
          downloads: 1,
          releaseType: "release"
        } as any
      }
      instanceId={1}
      instanceMods={[]}
      world="My World"
      type="datapack"
      installedFile={undefined}
      loading={false}
      disabled={false}
    />
  ))
  root.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations[0]).toEqual({
    key: "instance.installWorldDatapack",
    input: {
      instance_id: 1,
      world: "My World",
      identity: "id-My World",
      source: { Modrinth: { project_id: "P", version_id: "V3" } },
      replaces: "world-copy"
    }
  })
})
it("the addon page header installs into the world named in the link", async () => {
  control.params = { id: "P", platform: "modrinth" }
  control.searchParams = { instanceId: "1", world: "My World" }
  control.project = result()
  control.mods = [instancePack()]
  control.worlds = [world("My World")]
  const root = mount(() => <AddonPage />)
  const header = root.querySelector<HTMLButtonElement>(
    '[data-testid="addon-install-button"]'
  )!
  expect(header.getAttribute("data-variant")).toBe("primary")
  header.click()
  await flush()
  expect(control.mutations.map((m) => [m.key, m.input.world])).toEqual([
    ["instance.installWorldDatapack", "My World"]
  ])
})
it.each(["list", "grid"])(
  "%s Download on a Modrinth data pack that also ships as a mod installs into the world, never into the instance",
  async (view) => {
    control.worlds = [world("My World")]
    const root = search(view, [undefined], "My World", hybrid)
    installButtons(root)[0].click()
    await flush()
    expect(control.mutations).toEqual([
      {
        key: "instance.installWorldDatapack",
        input: {
          instance_id: 1,
          world: "My World",
          identity: "id-My World",
          source: { Modrinth: { project_id: "P", version_id: null } },
          replaces: null
        }
      }
    ])
  }
)
it.each(["list", "grid"])(
  "%s shows a Modrinth data pack typed as a mod as installed from the world's packs",
  (view) => {
    control.worlds = [world("My World", [worldPack()])]
    const root = search(view, [undefined], "My World", hybrid)
    expect(installButtons(root)).toHaveLength(0)
    expect(root.textContent).toContain("status_installed")
  }
)
it.each(["list", "grid"])(
  "%s opens a Modrinth data pack typed as a mod with the world in the addon page link",
  (view) => {
    control.worlds = [world("My World")]
    const root = search(view, [undefined], "My World", hybrid)
    root
      .querySelector<HTMLElement>('[data-testid="search-result-row"]')!
      .click()
    expect(control.navigate).toHaveBeenCalledWith(
      "/addon/P/modrinth?instanceId=1&world=My%20World"
    )
  }
)
it("a plain mod beside a world installs into the instance and opens without the world", async () => {
  control.worlds = [world("My World")]
  const root = search("list", [undefined], "My World", plainMod)
  installButtons(root)[0].click()
  await flush()
  expect(control.mutations.map((m) => m.key)).toEqual([
    "instance.installLatestMod"
  ])
  root.querySelector<HTMLElement>('[data-testid="search-result-row"]')!.click()
  expect(control.navigate).toHaveBeenCalledWith(
    "/addon/P/modrinth?instanceId=1"
  )
})
it("the versions page installs the chosen build of a Modrinth data pack typed as a mod into the world, replacing its copy", async () => {
  control.worlds = [world("My World", [worldPack("V1")])]
  const root = mount(() => (
    <RowContainer
      project={{ ...result(), ...hybrid } as any}
      modVersion={
        {
          fileId: "V3",
          name: "V3",
          gameVersions: [],
          datePublished: "fixture",
          downloads: 1,
          releaseType: "release"
        } as any
      }
      instanceId={1}
      instanceMods={[]}
      world="My World"
      type="mod"
      installedFile={undefined}
      loading={false}
      disabled={false}
    />
  ))
  root.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations).toEqual([
    {
      key: "instance.installWorldDatapack",
      input: {
        instance_id: 1,
        world: "My World",
        identity: "id-My World",
        source: { Modrinth: { project_id: "P", version_id: "V3" } },
        replaces: "world-copy"
      }
    }
  ])
})
it("the addon page header installs a Modrinth data pack typed as a mod into the world named in the link", async () => {
  control.params = { id: "P", platform: "modrinth" }
  control.searchParams = { instanceId: "1", world: "My World" }
  control.project = { ...result(), ...hybrid }
  control.worlds = [world("My World")]
  const root = mount(() => <AddonPage />)
  root
    .querySelector<HTMLButtonElement>('[data-testid="addon-install-button"]')!
    .click()
  await flush()
  expect(control.mutations.map((m) => [m.key, m.input.world])).toEqual([
    ["instance.installWorldDatapack", "My World"]
  ])
})
