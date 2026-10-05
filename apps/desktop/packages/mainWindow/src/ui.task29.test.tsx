import { afterEach, expect, it, vi } from "vitest"
import { createContext, createSignal } from "solid-js"
import { render } from "solid-js/web"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"
import { deferred, settle, toast } from "./task29.fixture"
import PageView from "./pages/News/PageView"
import PatchesTab from "./pages/News/PatchesTab"
import Java from "./pages/Settings/Java"
import AddCustomJava from "./managers/ModalsManager/modals/Java/AddCustomJava"
import Versions from "./pages/AddonViewPage/Versions"
import { AddonContext, StickyHeaderHeightContext } from "./pages/AddonViewPage"

const control = vi.hoisted(() => ({
  articles: [] as any[],
  java: undefined as any,
  mutation: undefined as any,
  close: vi.fn(),
  navigate: vi.fn(),
  list: undefined as any
}))
vi.mock("@gd/ui", async () => import("./task29.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => ({ id: "article" }),
  useSearchParams: () => [{ instanceId: "12", serverId: "34" }]
}))
vi.mock("@/utils/news", () => ({
  useNews: () => ({ isPending: false, data: [] }),
  usePatchNotes: () => ({ isPending: false, data: control.articles })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: control.navigate })
}))
vi.mock("@/pages/Settings/settings.java.data", () => ({
  default: () => control.java
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: control.close, openModal: vi.fn() }),
  useModalStackEntry: () => ({ closeIfCurrentTop: control.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (props: any) => <div>{props.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (options: any) => ({
      data:
        options().queryKey[0] === "instance.getInstanceMods"
          ? []
          : { javaCustomArgs: "", autoManageJavaSystemProfiles: true }
    }),
    createMutation: (options: any) => control.mutation(options().mutationKey[0])
  }
}))
vi.mock("@/components/InfiniteScrollVersionsQueryWrapper", () => ({
  useInfiniteVersionsQuery: () => control.list
}))
vi.mock("@/pages/AddonViewPage", () => ({
  AddonContext: createContext(),
  StickyHeaderHeightContext: createContext(() => 0)
}))
vi.mock("@tanstack/solid-virtual", () => ({
  createVirtualizer: (options: any) => ({
    getTotalSize: () => options.count * 70,
    getVirtualItems: () =>
      Array.from({ length: options.count }, (_, index) => ({
        index,
        start: index * 70,
        size: 70
      }))
  })
}))
vi.mock("./pages/AddonViewPage/Versions/VersionRow", () => ({
  default: (props: any) => (
    <span>
      {props.modVersion.fileId}:{props.instanceId}:{props.serverId}
    </span>
  )
}))

let dispose: (() => void) | undefined
let client: QueryClient | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  client?.clear()
  client = undefined
  document.body.textContent = ""
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
const button = (root: HTMLElement, key: string) =>
  Array.from(root.querySelectorAll("button")).find((item) =>
    item.textContent?.includes(key)
  )!
function articleFixture() {
  control.articles = [
    {
      id: "article",
      type: "patch",
      contentPath: "https://synthetic.invalid/patch",
      title: "Article",
      date: "2026-01-01",
      url: "https://synthetic.invalid/article"
    }
  ]
  client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } }
  })
  return () => (
    <QueryClientProvider client={client!}>
      <PageView />
    </QueryClientProvider>
  )
}
for (const kind of ["transport", "json", "http"] as const) {
  it(`0176 ${kind} failure settles into Retry and Retry fetches real query content`, async () => {
    const pending = deferred()
    const fetcher = vi.fn(() => pending.promise)
    vi.stubGlobal("fetch", fetcher)
    const root = mount(articleFixture())
    expect(root.querySelector('[data-testid="spinner"]')).not.toBeNull()
    if (kind === "transport") pending.reject(new Error("synthetic offline"))
    else
      pending.resolve({
        ok: kind !== "http",
        status: 503,
        json: async () => {
          if (kind === "json") throw new Error("synthetic malformed JSON")
          return { body: "bad status body" }
        }
      })
    await settle()
    expect(
      client!.getQueryCache().find({
        queryKey: ["patchContent", "https://synthetic.invalid/patch"]
      })!.state.status
    ).toBe("error")
    expect(button(root, "retry")).toBeDefined()
    expect(root.querySelector('[data-testid="spinner"]')).toBeNull()
    fetcher.mockImplementationOnce(async () => ({
      ok: true,
      json: async () => ({ body: "<p>Recovered</p>" })
    }))
    button(root, "retry").click()
    await settle()
    expect(root.textContent).toContain("Recovered")
    expect(fetcher).toHaveBeenCalledTimes(2)
    expect(button(root, "Read Full Patch Notes")).toBeDefined()
  })
}
it("a failed patch load is shown in place only, without the global toast", async () => {
  const fetcher = vi.fn(async () => ({
    ok: false,
    status: 404,
    json: async () => ({})
  }))
  vi.stubGlobal("fetch", fetcher)
  const root = mount(articleFixture())
  await settle()
  const query = client!.getQueryCache().find({
    queryKey: ["patchContent", "https://synthetic.invalid/patch"]
  })!
  expect(query.state.status).toBe("error")
  expect(root.querySelector('[role="alert"]')).not.toBeNull()
  // The app's QueryCache leaves a query with this meta to its own error UI.
  expect(query.meta?.localErrorToast).toBe(true)
})
it("0176 empty success reopens from the same real cache without loading or fetching", async () => {
  const fetcher = vi.fn(async () => ({
    ok: true,
    json: async () => ({ body: "" })
  }))
  vi.stubGlobal("fetch", fetcher)
  const component = articleFixture()
  let root = mount(component)
  await settle()
  expect(root.querySelector('[data-testid="spinner"]')).toBeNull()
  expect(root.textContent).toContain("no_content")
  dispose!()
  dispose = undefined
  root.remove()
  root = mount(component)
  await settle()
  expect(fetcher).toHaveBeenCalledTimes(1)
  expect(root.textContent).toContain("no_content")
})
it("0176 cached body and empty success survive background failure with recovery", async () => {
  for (const body of ["<p>Cached body</p>", ""]) {
    const fetcher = vi.fn(async () => ({
      ok: true,
      json: async () => ({ body })
    }))
    vi.stubGlobal("fetch", fetcher)
    const root = mount(articleFixture())
    await settle()
    fetcher.mockRejectedValueOnce(new Error("synthetic background failure"))
    await client!.refetchQueries()
    await settle()
    expect(root.textContent).toContain(body ? "Cached body" : "no_content")
    expect(root.querySelector('[data-testid="spinner"]')).toBeNull()
    expect(button(root, "retry")).toBeDefined()
    dispose!()
    dispose = undefined
    client!.clear()
    root.remove()
  }
})

const patches = [
  { id: "26.3-pre-3", version: "26.3-pre-3", versionType: "snapshot" },
  { id: "26.3-pre-2", version: "26.3-pre-2", versionType: "snapshot" },
  { id: "1.20-pre1", versionType: "snapshot" },
  { id: "explicit", versionType: "pre-release" },
  { id: "weekly", version: "26w12a", versionType: "snapshot" },
  { id: "release", version: "1.20", versionType: "release" },
  { id: "rc", version: "1.20-rc1", versionType: "snapshot" },
  { id: "prose", version: "the 1.20-pre1 preview", versionType: "snapshot" }
].map((item) => ({ ...item, title: item.id, date: "2026-01-01" }))
it("0178 actual filter includes numbered prereleases and excludes RC/prose", () => {
  control.articles = patches
  const root = mount(() => <PatchesTab />)
  const ids = () =>
    Array.from(root.querySelectorAll("h3")).map((item) => item.textContent)
  expect(ids()).toHaveLength(8)
  button(root, "prereleases").click()
  expect(ids()).toEqual(["26.3-pre-3", "26.3-pre-2", "1.20-pre1", "explicit"])
  button(root, "snapshots").click()
  expect(ids()).toEqual([
    "26.3-pre-3",
    "26.3-pre-2",
    "1.20-pre1",
    "weekly",
    "rc",
    "prose"
  ])
  button(root, "releases").click()
  expect(ids()).toEqual(["release"])
  root.querySelector("h3")!.parentElement!.parentElement!.parentElement!.click()
  expect(control.navigate).toHaveBeenCalledWith("/news/release")
})

function javaFixture(listFirst = false, profilesReady = true) {
  const [javas, setJavas] = createSignal<any>(
    listFirst
      ? {
          21: [
            {
              id: "synthetic-java-21",
              version: "21",
              path: "synthetic",
              type: "custom"
            }
          ]
        }
      : undefined
  )
  const [profiles, setProfiles] = createSignal<any>(
    profilesReady
      ? [
          { name: "custom", javaId: "synthetic-java-21", isSystem: false },
          { name: "none", javaId: null, isSystem: false },
          { name: "managed", javaId: "synthetic-java-21", isSystem: true }
        ]
      : undefined
  )
  const update = vi.fn()
  control.mutation = () => ({ mutate: update })
  control.java = {
    availableJavas: {
      get data() {
        return javas()
      }
    },
    javaProfiles: {
      get data() {
        return profiles()
      }
    },
    totalRam: { data: 8192 },
    javaVendors: { data: [] }
  }
  const root = mount(() => <Java />)
  const row = (name: string) =>
    Array.from(root.querySelectorAll("h3"))
      .find((item) => item.textContent === name)!
      .parentElement!.querySelector("select")!
  return { root, setJavas, setProfiles, update, row }
}
it("0177 unchanged mounted row reacts to late Java discovery without persisting null", () => {
  const { setJavas, update, row } = javaFixture()
  const original = row("custom")
  expect(original.value).toBe("unassigned")
  setJavas({
    21: [
      {
        id: "synthetic-java-21",
        version: "21",
        path: "synthetic",
        type: "custom"
      }
    ]
  })
  expect(row("custom")).toBe(original)
  expect(original.value).toBe("synthetic-java-21")
  expect(row("none").value).toBe("unassigned")
  expect(update).not.toHaveBeenCalled()
})
it("0177 Java-first, profile replacement, system guard and manual payload remain supported", () => {
  const { setProfiles, update, row } = javaFixture(true)
  expect(row("custom").value).toBe("synthetic-java-21")
  expect(row("managed").disabled).toBe(true)
  const custom = row("custom")
  custom.value = "unassigned"
  custom.dispatchEvent(new Event("change", { bubbles: true }))
  expect(update).toHaveBeenCalledWith({ profileName: "custom", javaId: null })
  setProfiles([{ name: "custom", javaId: null, isSystem: false }])
  expect(row("custom").value).toBe("unassigned")
})
it("0177 available list first then profile completion selects assigned Java; missing stays unassigned", () => {
  const { root, setProfiles, update, row } = javaFixture(true, false)
  expect(root.querySelector("select")).toBeNull()
  setProfiles([
    { name: "late", javaId: "synthetic-java-21", isSystem: false },
    { name: "missing", javaId: "missing-java", isSystem: false }
  ])
  expect(row("late").value).toBe("synthetic-java-21")
  expect(row("missing").value).toBe("unassigned")
  expect(update).not.toHaveBeenCalled()
})

it("0192 rejected creation retains path and never announces or closes early; success closes once", async () => {
  const first = deferred()
  const second = deferred()
  const create = vi
    .fn()
    .mockReturnValueOnce(first.promise)
    .mockReturnValueOnce(second.promise)
  control.mutation = (key: string) =>
    key === "java.validateCustomJavaPath"
      ? { isPending: false, mutateAsync: async () => true }
      : { isPending: false, mutate: create, mutateAsync: create }
  const root = mount(() => <AddCustomJava title="Synthetic Java" />)
  const input = root.querySelector("input")!
  input.value = "/synthetic/java"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  input.dispatchEvent(new Event("blur"))
  await settle()
  const add = root.querySelector("button")!
  ;(add as any).task29Click()
  ;(add as any).task29Click()
  expect(create).toHaveBeenCalledTimes(1)
  expect(control.close).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  first.reject(new Error("synthetic duplicate"))
  await settle()
  expect(input.value).toBe("/synthetic/java")
  expect(control.close).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  add.click()
  second.resolve(undefined)
  await settle()
  expect(create).toHaveBeenLastCalledWith("/synthetic/java")
  expect(control.close).toHaveBeenCalledTimes(1)
  expect(toast.success).toHaveBeenCalledTimes(1)
})
it("0192 first pending submission cannot close or announce success", async () => {
  const pending = deferred()
  control.mutation = (key: string) =>
    key === "java.validateCustomJavaPath"
      ? { isPending: false, mutateAsync: async () => true }
      : {
          isPending: false,
          mutate: () => pending.promise,
          mutateAsync: () => pending.promise
        }
  const root = mount(() => <AddCustomJava title="Synthetic Java" />)
  const input = root.querySelector("input")!
  input.value = "/synthetic/java"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  input.dispatchEvent(new Event("blur"))
  await settle()
  root.querySelector("button")!.click()
  expect(control.close).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  pending.resolve(undefined)
  await settle()
})
it("0192 invalid path keeps Add disabled", async () => {
  control.mutation = () => ({
    isPending: false,
    mutateAsync: async () => false
  })
  const root = mount(() => <AddCustomJava title="Synthetic Java" />)
  const input = root.querySelector("input")!
  input.value = "/synthetic/invalid"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  input.dispatchEvent(new Event("blur"))
  await settle()
  expect(root.querySelector("button")!.disabled).toBe(true)
})

function versionsFixture(
  projectError: boolean,
  listError: boolean,
  cached = false,
  pending = false
) {
  const [project, setProject] = createSignal<any>({
    data: cached ? { type: "mod" } : undefined,
    error: projectError ? new Error("project") : null,
    isPending: pending && !projectError
  })
  const [list, setList] = createSignal<any>({
    rows: cached
      ? [
          {
            fileId: "build",
            datePublished: "2026-01-01",
            gameVersions: ["1.20"]
          }
        ]
      : [],
    error: listError ? new Error("versions") : null,
    loading: pending
  })
  const projectRefetch = vi.fn(async () => {
    setProject({ data: { type: "mod" }, error: null, isPending: false })
  })
  const listRefetch = vi.fn(async () => {
    setList({
      rows: [
        { fileId: "build", datePublished: "2026-01-01", gameVersions: ["1.20"] }
      ],
      error: null,
      loading: false
    })
  })
  const mod = {
    get data() {
      return project().data
    },
    get error() {
      return project().error
    },
    get isPending() {
      return project().isPending
    },
    refetch: projectRefetch
  }
  control.list = {
    targetReady: true,
    allRows: () => list().rows,
    get isLoading() {
      return list().loading
    },
    infiniteQuery: {
      get error() {
        return list().error
      },
      refetch: listRefetch
    }
  }
  const root = mount(() => (
    <AddonContext.Provider value={mod as any}>
      <StickyHeaderHeightContext.Provider value={() => 0}>
        <Versions />
      </StickyHeaderHeightContext.Provider>
    </AddonContext.Provider>
  ))
  return { root, projectRefetch, listRefetch }
}
for (const [projectError, listError] of [
  [true, true],
  [true, false],
  [false, true]
]) {
  it(`0253 settled project=${projectError} list=${listError} errors recover both queries`, async () => {
    const { root, projectRefetch, listRefetch } = versionsFixture(
      projectError,
      listError
    )
    expect(root.querySelector('[data-testid="skeleton"]')).toBeNull()
    expect(root.textContent).toContain("error_loading_versions")
    button(root, "retry").click()
    await settle()
    expect(projectRefetch).toHaveBeenCalledTimes(1)
    expect(listRefetch).toHaveBeenCalledTimes(1)
    expect(root.querySelector('[data-file-id="build"]')!.textContent).toContain(
      "build:12:34"
    )
  })
}
it("0253 true pending scope retains skeleton", () => {
  const { root } = versionsFixture(false, false, false, true)
  expect(root.querySelector('[data-testid="skeleton"]')).not.toBeNull()
})
it("0253 cached rows survive background errors and offer nonblocking retry", () => {
  const { root } = versionsFixture(true, true, true)
  expect(root.querySelector('[data-file-id="build"]')).not.toBeNull()
  expect(root.querySelector('[data-testid="skeleton"]')).toBeNull()
  expect(button(root, "retry")).toBeDefined()
})
it("0253 pending target still retries project without forcing an unadmitted list", async () => {
  const { root, projectRefetch, listRefetch } = versionsFixture(
    true,
    false,
    false,
    true
  )
  control.list.targetReady = false
  button(root, "retry").click()
  await settle()
  expect(projectRefetch).toHaveBeenCalledTimes(1)
  expect(listRefetch).not.toHaveBeenCalled()
})
it("0253 manual list dispatch rechecks current target after project recovery starts", async () => {
  const { root, projectRefetch, listRefetch } = versionsFixture(true, true)
  projectRefetch.mockImplementationOnce(async () => {
    control.list.targetReady = false
  })
  button(root, "retry").click()
  await settle()
  expect(listRefetch).not.toHaveBeenCalled()
  control.list.targetReady = true
  button(root, "retry").click()
  await settle()
  expect(listRefetch).toHaveBeenCalledTimes(1)
})
it("0253 cached project with list error shows recovery and successful empty remains distinct", async () => {
  const { root, projectRefetch } = versionsFixture(false, true)
  await projectRefetch()
  expect(root.textContent).toContain("error_loading_versions")
  expect(root.querySelector('[data-testid="skeleton"]')).toBeNull()
  dispose!()
  dispose = undefined
  root.remove()
  const empty = versionsFixture(false, false)
  await empty.projectRefetch()
  expect(empty.root.textContent).toContain("no_versions_found")
  expect(empty.root.querySelector('[data-testid="skeleton"]')).toBeNull()
})
it("0253 rejected retry settles without leaking event errors or hiding cached rows", async () => {
  const { root, projectRefetch, listRefetch } = versionsFixture(
    true,
    true,
    true
  )
  projectRefetch.mockRejectedValueOnce(new Error("synthetic project retry"))
  listRefetch.mockRejectedValueOnce(new Error("synthetic versions retry"))
  button(root, "retry").click()
  await settle()
  expect(root.querySelector('[data-file-id="build"]')).not.toBeNull()
  expect(button(root, "retry").disabled).toBe(false)
})
