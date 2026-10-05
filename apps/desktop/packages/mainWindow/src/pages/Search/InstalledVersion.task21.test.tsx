import { afterEach, expect, it, vi } from "vitest"
import { For } from "solid-js"
import { render } from "solid-js/web"
import { List } from "./List"
import { Grid } from "./Grid"
import ModDownloadButton from "@/components/ModDownloadButton"
import AddonPage from "@/pages/AddonViewPage"
import RowContainer from "@/components/Browser/RowContainer"

const control = vi.hoisted(() => ({
  context: undefined as any,
  mods: [] as any[],
  serverAddons: [] as any[],
  project: undefined as any,
  params: {} as any,
  searchParams: {} as any,
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
          : key === "server.getServerAddons"
            ? control.serverAddons
            : key === "modplatforms.unifiedGetProject"
              ? control.project
              : key === "modplatforms.modrinth.getVersion"
                ? {
                    id: control.searchParams.requestedVersionId,
                    project_id: "P"
                  }
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
  useGDNavigate: () => ({ navigate: vi.fn(), prev: vi.fn() })
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
  InstanceDropdown: () => null
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
  control.serverAddons = []
  control.params = {}
  control.searchParams = {}
  control.project = undefined
  control.context = undefined
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
const result = (version?: string, platform = "modrinth", type = "mod") => ({
  id: "P",
  platform,
  type,
  slug: "project",
  title: "Project",
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
const installed = (version = "A", kind = "mods") => ({
  id: "local-A",
  addon_type: kind,
  enabled: false,
  modrinth: { project_id: "P", version_id: version },
  curseforge: null
})
const serverAddon = () => ({
  id: "server-local",
  addonType: "mods",
  displayName: "project",
  filename: "project.jar",
  modrinthProjectId: "P",
  curseforgeProjectId: null,
  enabled: false,
  fileSize: 4,
  hasImage: false
})
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
function search(
  view: string,
  requested: (string | undefined)[],
  server = false,
  mods = [installed()],
  projectId = "P"
) {
  control.params = { type: "mod" }
  control.mods = mods
  control.serverAddons = server ? [serverAddon()] : []
  control.context = {
    allRows: () =>
      requested.map((v) => ({
        type: "value",
        value: { ...result(v), id: projectId }
      })),
    selectedInstanceId: () => (server ? undefined : 1),
    selectedInstanceMods: { data: control.mods },
    selectedServerId: () => (server ? 9 : undefined),
    selectedServerAddons: { data: control.serverAddons },
    defaultAddonType: () => "mod",
    searchQuery: () => ({ projectType: "mod" }),
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
it.each(["list", "grid"])(
  "actual %s retains B and C install actions when only A is installed",
  async (view) => {
    const root = search(view, ["B", "C"])
    const buttons = [...root.querySelectorAll<HTMLButtonElement>("button")]
    expect(buttons).toHaveLength(2)
    expect(root.textContent).not.toContain("status_installed")
    buttons.forEach((button) => button.click())
    await flush()
    expect(
      control.mutations.map((m) => m.input.mod_source.Modrinth.version_id)
    ).toEqual(["B", "C"])
    expect(
      control.mutations.every(
        (m) => m.input.replaces_mod === "local-A" && m.input.install_deps
      )
    ).toBe(true)
  }
)
it.each(["list", "grid"])(
  "actual %s preserves exact-A and ordinary project Installed controls",
  (view) => {
    const root = search(view, ["A", undefined])
    expect(root.querySelectorAll("button")).toHaveLength(0)
    expect(root.textContent?.match(/status_installed/g)).toHaveLength(2)
  }
)
it.each(["list", "grid"])(
  "actual %s permits a selected server version with unknown installed artifact",
  async (view) => {
    const root = search(view, ["B"], true)
    const button = root.querySelector<HTMLButtonElement>("button")
    expect(button).not.toBeNull()
    button!.click()
    await flush()
    expect(control.mutations[0]).toEqual({
      key: "server.installServerMod",
      input: {
        serverId: 9,
        modSource: { Modrinth: { project_id: "P", version_id: "B" } }
      }
    })
  }
)
it("actual nonpack button renders unknown server artifact without fabricated provider metadata", async () => {
  const root = mount(() => (
    <ModDownloadButton
      addon={result("B") as any}
      fileId="B"
      selectedServerId={9}
      selectedServerAddons={[serverAddon()]}
    />
  ))
  expect(root.textContent).toContain("switch_version")
  root.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations[0].input.modSource.Modrinth.version_id).toBe("B")
})
it("actual addon page gives primary and sticky controls the same ordinary server context", () => {
  control.params = { id: "P", platform: "modrinth" }
  control.searchParams = { serverId: "9" }
  control.project = result()
  control.serverAddons = [serverAddon()]
  const root = mount(() => <AddonPage />)
  expect(root.querySelectorAll('[data-variant="green"]')).toHaveLength(2)
})
it("actual addon page coordinates selected-version primary/sticky actions with unknown server metadata", () => {
  control.params = { id: "P", platform: "modrinth" }
  control.searchParams = { serverId: "9", requestedVersionId: "B" }
  control.project = result()
  control.serverAddons = [serverAddon()]
  const root = mount(() => <AddonPage />)
  expect(root.querySelectorAll('[data-variant="green"]')).toHaveLength(0)
  expect(
    root.querySelector('[data-testid="addon-install-button"]')?.textContent
  ).toContain("switch_version")
  // Sticky is icon-only, but its actual InstallButton state must still have the same known project.
  expect(
    root.querySelectorAll("button .i-hugeicons\\:download-02")
  ).toHaveLength(2)
})
it("explicit version row keeps its own C selection over navigation B", async () => {
  control.searchParams = { requestedVersionId: "B" }
  control.mods = [installed()]
  const root = mount(() => (
    <RowContainer
      project={result("B") as any}
      modVersion={
        {
          fileId: "C",
          name: "C",
          gameVersions: [],
          datePublished: "fixture",
          downloads: 1,
          releaseType: "release"
        } as any
      }
      instanceId={1}
      instanceMods={control.mods}
      type="mod"
      installedFile={undefined}
      loading={false}
      disabled={false}
    />
  ))
  root.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations[0].input.mod_source.Modrinth.version_id).toBe("C")
})
it("kind and platform separate installed artifacts while world version copy stays supported", async () => {
  const wrong = {
    ...installed("B", "resourcepacks"),
    modrinth: null,
    curseforge: { project_id: 12, file_id: 1 }
  }
  const root = mount(() => (
    <ModDownloadButton
      addon={result("B") as any}
      fileId="B"
      selectedInstanceId={1}
      selectedInstanceMods={[wrong] as any}
    />
  ))
  root.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations[0].input.replaces_mod).toBeNull()
  dispose?.()
  control.mutations = []
  mount(() => (
    <ModDownloadButton
      addon={result("B", "modrinth", "world") as any}
      fileId="B"
      selectedInstanceId={1}
      selectedInstanceMods={[installed("A", "worlds")] as any}
    />
  ))
  expect(document.body.textContent).toContain("world_version_copy_action")
})
it.each(["list", "grid"])(
  "round01 actual %s separates addon kind even for the same provider/version",
  async (view) => {
    const root = search(view, ["B"], false, [installed("B", "resourcepacks")])
    const button = root.querySelector<HTMLButtonElement>("button")
    expect(button).not.toBeNull()
    button!.click()
    await flush()
    expect(control.mutations[0].input.replaces_mod).toBeNull()
  }
)
it.each(["list", "grid"])(
  "round01 actual %s separates colliding project IDs across platforms",
  async (view) => {
    const wrongPlatform = {
      ...installed(),
      modrinth: null,
      curseforge: { project_id: 12, file_id: 99 }
    }
    const root = search(view, ["B"], false, [wrongPlatform] as any, "12")
    const button = root.querySelector<HTMLButtonElement>("button")
    expect(button).not.toBeNull()
    button!.click()
    await flush()
    expect(control.mutations[0].input).toMatchObject({
      mod_source: { Modrinth: { project_id: "12", version_id: "B" } },
      replaces_mod: null
    })
  }
)
it("round01 actual CurseForge server selection also keeps missing exact metadata unknown", async () => {
  const addon = { ...result(undefined, "curseforge"), id: "12" }
  const cached = {
    ...serverAddon(),
    modrinthProjectId: null,
    curseforgeProjectId: 12
  }
  const root = mount(() => (
    <ModDownloadButton
      addon={addon as any}
      fileId={10}
      selectedServerId={9}
      selectedServerAddons={[cached]}
    />
  ))
  expect(root.textContent).toContain("switch_version")
  root.querySelector<HTMLButtonElement>("button")!.click()
  await flush()
  expect(control.mutations[0].input.modSource.Curseforge).toEqual({
    project_id: 12,
    file_id: 10
  })
})
it("round01 ordinary server slug aliases remain supported without provider metadata", () => {
  const cached = {
    ...serverAddon(),
    displayName: "project-1.0",
    modrinthProjectId: null
  }
  const root = mount(() => (
    <ModDownloadButton
      addon={result() as any}
      selectedServerId={9}
      selectedServerAddons={[cached]}
    />
  ))
  expect(root.querySelectorAll('[data-variant="green"]')).toHaveLength(1)
})
