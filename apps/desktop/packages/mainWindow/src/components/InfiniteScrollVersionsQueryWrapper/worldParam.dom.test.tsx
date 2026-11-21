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
  useSearchParams
} from "@solidjs/router"
import { getSearchResults } from "@/utils/platformSearch"
import { SearchInputContext } from "@/components/SearchInputContext"
import Wrapper, { useInfiniteVersionsQuery } from "./index"

// The real search context and the real versions wrapper on a real router, as
// the addon page mounts them. Only the transport is stubbed.
const c = vi.hoisted(() => ({ query: vi.fn() }))
vi.mock("@/utils/rspcClient", () => ({
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
    createMutation: () => ({ mutate: vi.fn() })
  }
}))
vi.mock("@gd/i18n", () => ({ i18n: { t: (key: string) => key } }))

let dispose: (() => void) | undefined
let client: QueryClient
afterEach(() => {
  dispose?.()
  client?.clear()
  document.body.textContent = ""
  c.query.mockReset()
})
const flush = async () => {
  for (let i = 0; i < 25; i++) {
    await Promise.resolve()
    await new Promise((r) => setTimeout(r, 0))
  }
}
function open(url: string) {
  c.query.mockImplementation(async ([key, input]: any) => {
    if (key === "modplatforms.unifiedSearch")
      return { data: [], pagination: null }
    if (key === "settings.getSearchSidebarDocked") return true
    if (key === "instance.getInstanceDetails")
      return {
        name: `Instance ${input}`,
        modloaders: [{ type_: "fabric" }],
        version: "1.20.1"
      }
    if (key === "modplatforms.modrinth.getProject") return {}
    return []
  })
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  let params!: any, versions!: ReturnType<typeof useInfiniteVersionsQuery>
  const Child = () => {
    versions = useInfiniteVersionsQuery()
    return null
  }
  const Page = () => {
    ;[params] = useSearchParams()
    return (
      <SearchInputContext.Provider value={getSearchResults()}>
        <Wrapper modId="LPjGiSO4" modplatform="modrinth" addonType="mod">
          <Child />
        </Wrapper>
      </SearchInputContext.Provider>
    )
  }
  const history = createMemoryHistory()
  history.set({ value: url, replace: true })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/addon/:id/:platform/versions" component={Page} />
        </MemoryRouter>
      </QueryClientProvider>
    ),
    root
  )
  return {
    get params() {
      return params
    },
    get versions() {
      return versions
    }
  }
}

it("an addon page opened for a world of an instance keeps the world in its link", async () => {
  const page = open(
    "/addon/LPjGiSO4/modrinth/versions?instanceId=1&world=New%20World"
  )
  await flush()
  expect(page.versions.targetReady).toBe(true)
  expect(page.params.instanceId).toBe("1")
  expect(page.params.world).toBe("New World")
})
