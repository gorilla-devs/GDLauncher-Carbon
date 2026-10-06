import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import {
  QueryClient,
  QueryClientProvider,
  useQueryClient
} from "@tanstack/solid-query"
import { AddonContext, StickyHeaderHeightContext } from "@/pages/AddonViewPage"
import Wrapper from "@/components/InfiniteScrollVersionsQueryWrapper"
import Versions from "./index"

const c = vi.hoisted(() => ({ query: vi.fn(), params: {} as any }))
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
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      client: { query: c.query },
      queryClient: useQueryClient()
    }),
    createQuery: () => ({ data: undefined })
  }
}))
vi.mock("@tanstack/solid-virtual", () => ({
  createVirtualizer: () => ({
    getVirtualItems: () => [],
    getTotalSize: () => 0
  })
}))
vi.mock("./VersionRow", () => ({ default: () => null }))
vi.mock("@gd/i18n", () => ({
  // The key, then the interpolated values, so a test can read both.
  Trans: (p: any) => (
    <span>
      {[
        p.key,
        ...Object.entries(p.options ?? {})
          .filter(([name]) => name !== "interpolation")
          .map(([, value]) => String(value))
      ].join("|")}
    </span>
  )
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../../packages/ui/src/Button")),
  Spinner: () => <span />,
  Skeleton: { addonVersionsTable: () => <div data-skeleton /> }
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
const versionCalls = () =>
  c.query.mock.calls
    .filter(([a]) => a[0] === "modplatforms.modrinth.getProjectVersions")
    .map(([a]) => a[1])
function mount(params: any) {
  c.params = params
  client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, gcTime: Infinity }
    }
  })
  const project = {
    data: { id: "project", type: "mod", platform: "modrinth" },
    error: null,
    isPending: false,
    refetch: vi.fn()
  }
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <AddonContext.Provider value={project as any}>
          <StickyHeaderHeightContext.Provider value={() => 0}>
            <Wrapper
              modId="project"
              modplatform="modrinth"
              addonType="mod"
              supportedModloaders={["fabric"]}
              supportedGameVersions={["1.21"]}
            >
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

it("W02#1 an empty server list says what it was filtered by and Show all versions lists every version", async () => {
  c.query.mockImplementation(([key]: any) =>
    Promise.resolve(
      key === "server.getServerDetails"
        ? {
            name: "Survival",
            modloaderType: "neoforge",
            gameVersion: "1.20.1"
          }
        : key.endsWith("getProject")
          ? {}
          : []
    )
  )
  const root = mount({ serverId: "7" })
  await settle()
  const notice = root.querySelector(
    '[data-testid="addon-versions-target-empty"]'
  )!
  expect(notice).not.toBeNull()
  expect(notice.textContent).toContain(
    "content:_trn_no_versions_for_target|NeoForge 1.20.1"
  )
  expect(notice.textContent).toContain(
    "content:_trn_no_versions_for_target_hint|Survival"
  )
  expect(root.textContent).not.toContain("content:_trn_no_versions_found")
  const showAll = [...notice.querySelectorAll("button")].find((b) =>
    b.textContent?.includes("content:_trn_show_all_versions")
  )!
  showAll.click()
  await settle()
  expect(versionCalls()).toEqual([
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["neoforge"] },
    { project_id: "project", game_versions: undefined, loaders: undefined }
  ])
  // Still empty, but now unfiltered: nothing left to explain.
  expect(
    root.querySelector('[data-testid="addon-versions-target-empty"]')
  ).toBeNull()
  expect(root.textContent).toContain("content:_trn_no_versions_found")
})

it("W02#1 an empty list without a target keeps the plain message", async () => {
  c.query.mockImplementation(([key]: any) =>
    Promise.resolve(key.endsWith("getProject") ? {} : [])
  )
  const root = mount({})
  await settle()
  expect(root.textContent).toContain("content:_trn_no_versions_found")
  expect(
    root.querySelector('[data-testid="addon-versions-target-empty"]')
  ).toBeNull()
})

it("W02#4 a failed target lookup names its reason under the error", async () => {
  c.query.mockImplementation(([key]: any) =>
    key === "instance.getInstanceDetails"
      ? Promise.reject(new Error("Instance 7 was not found"))
      : Promise.resolve(key.endsWith("getProject") ? {} : [])
  )
  const root = mount({ instanceId: "7" })
  await settle()
  const alert = root.querySelector('[role="alert"]')!
  expect(alert.textContent).toContain("content:_trn_error_loading_versions")
  expect(alert.textContent).toContain("Instance 7 was not found")
  expect(versionCalls()).toEqual([])
})
