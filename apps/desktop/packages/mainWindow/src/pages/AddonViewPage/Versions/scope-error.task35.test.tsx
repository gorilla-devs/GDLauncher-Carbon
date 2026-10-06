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

const c = vi.hoisted(() => ({ query: vi.fn(), project: undefined as any }))
vi.mock("@/pages/AddonViewPage", async () => {
  const { createContext } = await import("solid-js")
  return {
    AddonContext: createContext(),
    StickyHeaderHeightContext: createContext(() => 0)
  }
})
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [{ instanceId: "7" }, vi.fn()]
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
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
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
it("current target failure uses actual Versions alert and Retry, then resumes scoped requests once", async () => {
  c.query.mockImplementation(([key]) =>
    key === "instance.getInstanceDetails"
      ? Promise.reject(new Error("synthetic target failure"))
      : Promise.resolve(key.endsWith("getProject") ? {} : [])
  )
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
            <Wrapper modId="project" modplatform="modrinth" addonType="mod">
              <Versions />
            </Wrapper>
          </StickyHeaderHeightContext.Provider>
        </AddonContext.Provider>
      </QueryClientProvider>
    ),
    root
  )
  await settle()
  expect(root.querySelector('[role="alert"]')).not.toBeNull()
  expect(root.querySelector("[data-skeleton]")).toBeNull()
  expect(
    c.query.mock.calls.filter(
      ([a]) => a[0] === "modplatforms.modrinth.getProjectVersions"
    )
  ).toEqual([])
  c.query.mockImplementation(([key]) =>
    Promise.resolve(
      key === "instance.getInstanceDetails"
        ? { version: "1.20.1", modloaders: [{ type_: "fabric" }] }
        : key.endsWith("getProject")
          ? {}
          : []
    )
  )
  const retry = root.querySelector("button")!
  retry.click()
  retry.click()
  await settle()
  expect(root.querySelector('[role="alert"]')).toBeNull()
  expect(project.refetch).toHaveBeenCalledTimes(1)
  expect(
    c.query.mock.calls.filter(([a]) => a[0] === "instance.getInstanceDetails")
  ).toHaveLength(2)
  expect(
    c.query.mock.calls
      .filter(([a]) => a[0] === "modplatforms.modrinth.getProjectVersions")
      .map(([a]) => a[1])
  ).toEqual([
    { project_id: "project", game_versions: ["1.20.1"], loaders: ["fabric"] }
  ])
})
