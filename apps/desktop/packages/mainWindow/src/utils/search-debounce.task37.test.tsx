import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router"
import { getSearchResults } from "./platformSearch"

// Only the RPC transport is replaced; both real query wrappers keep reactive keys.
const io = vi.hoisted(() => ({ query: vi.fn() }))
vi.mock("./rspcClient", () => ({
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
    createMutation: () => ({ mutate: () => {} })
  }
}))

let dispose: (() => void) | undefined
let client: QueryClient
async function flush() {
  for (let i = 0; i < 12; i++) {
    await Promise.resolve()
    await vi.advanceTimersByTimeAsync(0)
  }
}
function fixture(text = "") {
  vi.useFakeTimers()
  io.query.mockImplementation(async ([key]: any) => {
    if (key === "modplatforms.unifiedSearch")
      return { data: [], pagination: null }
    if (key === "modplatforms.unifiedGetProjectsByIds")
      return { results: [], errors: [] }
    if (key === "settings.getSearchSidebarDocked") return true
    return null
  })
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  let search!: ReturnType<typeof getSearchResults>
  const Content = () => {
    search = getSearchResults()
    return <div />
  }
  const root = document.createElement("div")
  document.body.append(root)
  const history = createMemoryHistory()
  history.set({ value: `/?q=${encodeURIComponent(text)}`, replace: true })
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/" component={Content} />
        </MemoryRouter>
      </QueryClientProvider>
    ),
    root
  )
  return () => search
}
const calls = (platform: string) =>
  io.query.mock.calls
    .filter(
      ([args]) =>
        args[0] === "modplatforms.unifiedSearch" &&
        args[1].searchApi === platform
    )
    .map(([args]) => ({ text: args[1].searchQuery, size: args[1].pageSize }))
afterEach(() => {
  dispose?.()
  dispose = undefined
  client?.clear()
  vi.clearAllTimers()
  vi.useRealTimers()
  io.query.mockReset()
  document.body.textContent = ""
})

it("GPT0181 first input leads before 500ms and a typing burst publishes only its final trailing query", async () => {
  const search = fixture()
  await flush()
  expect(calls("curseforge")).toEqual([{ text: "", size: 15 }])
  for (const text of ["s", "so", "sod"]) {
    search().setSearchQuery((p) => ({ ...p, searchQuery: text }))
    expect(search().searchQuery().searchQuery).toBe(text)
    await flush()
    await vi.advanceTimersByTimeAsync(50)
  }
  for (const platform of ["curseforge", "modrinth"]) {
    expect(calls(platform)).toEqual([
      { text: "", size: 15 },
      { text: "s", size: 15 }
    ])
  }
  await vi.advanceTimersByTimeAsync(449)
  await flush()
  expect(calls("curseforge")).toHaveLength(2)
  await vi.advanceTimersByTimeAsync(1)
  await flush()
  for (const platform of ["curseforge", "modrinth"]) {
    expect(calls(platform)).toEqual([
      { text: "", size: 15 },
      { text: "s", size: 15 },
      { text: "sod", size: 15 }
    ])
  }
  search().setSearchQuery((p) => ({ ...p, searchQuery: "new" }))
  await flush()
  expect(calls("curseforge").at(-1)?.text).toBe("new")
})

it("one change has no duplicate trailing request and owner disposal cancels the burst", async () => {
  const search = fixture("seed")
  await flush()
  expect(calls("modrinth")).toEqual([{ text: "seed", size: 15 }])
  search().setSearchQuery((p) => ({ ...p, searchQuery: "single" }))
  await flush()
  await vi.advanceTimersByTimeAsync(500)
  await flush()
  expect(calls("modrinth")).toEqual([
    { text: "seed", size: 15 },
    { text: "single", size: 15 }
  ])
  search().setSearchQuery((p) => ({ ...p, searchQuery: "lead" }))
  await flush()
  search().setSearchQuery((p) => ({ ...p, searchQuery: "pending" }))
  await flush()
  dispose!()
  dispose = undefined
  client.clear()
  const before = io.query.mock.calls.length
  expect(vi.getTimerCount()).toBe(0)
  await vi.advanceTimersByTimeAsync(1000)
  expect(io.query.mock.calls).toHaveLength(before)
})

it.each(["curseforge", "modrinth"] as const)(
  "%s filter keeps twenty-row requests and direct input stays synchronous",
  async (platform) => {
    const search = fixture("seed")
    await flush()
    search().setSearchQuery((p) => ({ ...p, searchApi: platform }))
    await flush()
    expect(calls(platform)).toEqual([
      { text: "seed", size: 15 },
      { text: "seed", size: 20 }
    ])
    expect(
      calls(platform === "curseforge" ? "modrinth" : "curseforge")
    ).toEqual([{ text: "seed", size: 15 }])
    search().setSearchQuery((p) => ({ ...p, searchQuery: "#123456" }))
    await flush()
    expect(search().isDirectMode()).toBe(true)
    expect(
      io.query.mock.calls.some(
        ([args]) => args[0] === "modplatforms.unifiedGetProjectsByIds"
      )
    ).toBe(true)
  }
)

it("initial share seed enables share preview and suppresses both platform and direct batch requests", async () => {
  const search = fixture("gdlauncher://share/FIXTURE1")
  await flush()
  expect(search().isShareMode()).toBe(true)
  expect(search().shareCode()).toBe("FIXTURE1")
  expect(calls("curseforge")).toEqual([])
  expect(calls("modrinth")).toEqual([])
  expect(
    io.query.mock.calls.some(
      ([args]) => args[0] === "modplatforms.unifiedGetProjectsByIds"
    )
  ).toBe(false)
})
