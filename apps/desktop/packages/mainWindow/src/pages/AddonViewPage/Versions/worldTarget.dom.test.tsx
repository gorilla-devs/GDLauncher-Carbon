import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import {
  QueryClient,
  QueryClientProvider,
  useQueryClient
} from "@tanstack/solid-query"
import { AddonContext, StickyHeaderHeightContext } from "@/pages/AddonViewPage"
import Wrapper from "@/components/InfiniteScrollVersionsQueryWrapper"
import Navbar from "@/components/ExploreVersionsNavbar"
import Versions from "./index"

// The real versions wrapper, list and target bar for a Modrinth data pack
// that also ships as a mod, as the platform really returns it.
const c = vi.hoisted(() => ({ query: vi.fn(), params: {} as any }))
const project = {
  id: "P",
  platform: "modrinth",
  type: "mod",
  loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"],
  minecraftVersions: ["1.20.1"]
}
const build = (id: string, name: string, file: string) => ({
  project_id: "P",
  id,
  name,
  version_type: "release",
  game_versions: ["1.20.1"],
  downloads: 1,
  date_published: "2024-01-01T00:00:00Z",
  files: [{ filename: file, size: 1, hashes: { sha512: "hash" } }],
  status: "listed"
})
const jar = build("JAR", "v1.2.8 ~ Mod 1.20.x", "Nullscape_1.20.x_v1.2.8.jar")
const packs = [
  build("PACK2", "v1.2.8 ~ Datapack 1.20.x", "Nullscape_1.20_v1.2.8.zip"),
  build("PACK1", "v1.2.7 ~ Datapack 1.20.x", "Nullscape_1.20_v1.2.7.zip")
]
const copy = (id: string, version: string, addon_type?: string) => ({
  id,
  addon_type,
  modrinth: { project_id: "P", version_id: version },
  curseforge: null
})
vi.mock("@/pages/AddonViewPage", async () => {
  const { createContext } = await import("solid-js")
  return {
    AddonContext: createContext(),
    StickyHeaderHeightContext: createContext(() => 0)
  }
})
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [c.params, vi.fn()]
}))
vi.mock("@/components/SearchInputContext", () => ({ default: () => undefined }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({ minecraftVersions: { data: [] } })
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "",
  getServerImageUrl: () => ""
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { query: c.query },
      queryClient: useQueryClient()
    }),
    createQuery: (options: any) => ({
      get data() {
        const key = options().queryKey[0]
        // The instance holds the mod jar, its world an older data pack build.
        return key === "instance.getWorlds"
          ? [
              {
                name: "New World",
                addon_id: "w",
                identity: "id",
                datapacks: [copy("world-copy", "PACK1")]
              }
            ]
          : key === "instance.getInstanceMods"
            ? [copy("instance-copy", "JAR", "mods")]
            : undefined
      }
    })
  }
}))
vi.mock("@tanstack/solid-virtual", () => ({
  createVirtualizer: (options: any) => ({
    getVirtualItems: () =>
      Array.from({ length: options.count }, (_, index) => ({
        index,
        size: 70,
        start: index * 70
      })),
    getTotalSize: () => 0
  })
}))
vi.mock("./VersionRow", () => ({
  default: (p: any) => (
    <span data-row-world={p.world ?? ""} data-held={p.installedFile?.id ?? ""}>
      {p.modVersion.fileName}
    </span>
  )
}))
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => <button>{p.children}</button>,
  Spinner: () => <span />,
  Skeleton: { addonVersionsTable: () => <div data-skeleton /> },
  Checkbox: (p: any) => <label>{p.children}</label>,
  Select: () => null,
  SelectTrigger: () => null,
  SelectContent: () => null,
  SelectItem: () => null,
  SelectValue: () => null
}))
let dispose: (() => void) | undefined, client: QueryClient
afterEach(() => {
  dispose?.()
  dispose = undefined
  client?.clear()
  c.query.mockReset()
  document.body.textContent = ""
})
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 5))
}
function open(params: Record<string, string>) {
  c.params = params
  c.query.mockImplementation(([key, input]: any) =>
    Promise.resolve(
      key === "instance.getInstanceDetails"
        ? {
            name: "Addon test",
            modloaders: [{ type_: "fabric" }],
            version: "1.20.1"
          }
        : key === "modplatforms.modrinth.getProjectVersions"
          ? input.loaders?.[0] === "datapack"
            ? packs
            : [jar]
          : {}
    )
  )
  client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, gcTime: Infinity }
    }
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <AddonContext.Provider
          value={
            {
              data: project,
              error: null,
              isPending: false,
              refetch: vi.fn()
            } as any
          }
        >
          <StickyHeaderHeightContext.Provider value={() => 0}>
            <Wrapper
              modId="P"
              modplatform="modrinth"
              addonType="mod"
              supportedModloaders={project.loaders}
              supportedGameVersions={project.minecraftVersions}
            >
              <div data-target-bar>
                <Navbar modplatform="modrinth" type="mod" addonType="mod" />
              </div>
              <Versions />
            </Wrapper>
          </StickyHeaderHeightContext.Provider>
        </AddonContext.Provider>
      </QueryClientProvider>
    ),
    root
  )
  return root
}
const rows = (root: HTMLElement) =>
  [
    ...root.querySelectorAll<HTMLElement>('[data-testid="addon-version-row"]')
  ].map((row) => ({
    file: row.textContent,
    world: row
      .querySelector("[data-row-world]")!
      .getAttribute("data-row-world"),
    marked: row.className.includes("bg-green-500/5")
  }))

it("opened for a world, the list holds the data pack builds, marks the one the world holds and names the world", async () => {
  const root = open({ instanceId: "1", world: "New World" })
  await settle()
  expect(rows(root)).toEqual([
    { file: "Nullscape_1.20_v1.2.8.zip", world: "New World", marked: false },
    { file: "Nullscape_1.20_v1.2.7.zip", world: "New World", marked: true }
  ])
  // A pick replaces the world's copy, never the instance's mod jar.
  expect(root.querySelector("[data-held]")!.getAttribute("data-held")).toBe(
    "world-copy"
  )
  const bar = root.querySelector("[data-target-bar]")!.textContent
  expect(bar).toContain("New World")
  expect(bar).toContain("Addon test")
})

it("opened for the instance alone, the list holds the instance's own builds as before", async () => {
  const root = open({ instanceId: "1" })
  await settle()
  expect(rows(root)).toEqual([
    { file: "Nullscape_1.20.x_v1.2.8.jar", world: "", marked: true }
  ])
  expect(root.querySelector("[data-held]")!.getAttribute("data-held")).toBe(
    "instance-copy"
  )
  const bar = root.querySelector("[data-target-bar]")!.textContent
  expect(bar).toContain("Addon test")
  expect(bar).not.toContain("New World")
})
