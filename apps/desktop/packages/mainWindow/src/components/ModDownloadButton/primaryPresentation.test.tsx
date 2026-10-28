import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"

// Only transport, toast and final rendering are adapted. Mutation/query
// observers and the button's production installation/progress hooks are real.
const c = vi.hoisted(() => ({
  query: vi.fn(),
  mutate: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  download: undefined as any,
  loading: undefined as any,
  progress: undefined as any
}))
vi.mock("@/utils/rspcClient", async () => {
  const { createQuery, createMutation } = await import("@tanstack/solid-query")
  return {
    rspc: {
      useContext: () => ({ client: { query: c.query } }),
      createQuery: (options: any) =>
        createQuery(() => ({
          ...options(),
          queryFn: () => c.query(options().queryKey)
        })),
      createMutation: (options: any) =>
        createMutation(() => {
          const key = options().mutationKey
          return {
            mutationKey: [key],
            mutationFn: (input: any) => c.mutate(key, input)
          }
        })
    }
  }
})
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@gd/ui", () => ({ toast: { error: c.error, success: c.success } }))
vi.mock("@gd/i18n", () => {
  const t = (key: string, vars?: any) =>
    vars?.title ? `${key}:${vars.title}` : key
  return { useTransContext: () => [t], i18n: { t } }
})
vi.mock("./hooks/useInstanceSearch", () => ({
  useInstanceSearch: () => ({
    searchQuery: () => "",
    setSearchQuery: vi.fn(),
    hoveredInstanceId: () => null,
    setHoveredInstanceId: vi.fn(),
    filteredInstances: () => [],
    shouldVirtualize: () => false
  })
}))
vi.mock("./components/InstanceDropdown", () => ({
  InstanceDropdown: () => null
}))
vi.mock("./components/InstallButton", () => ({
  InstallButton: (p: any) => {
    c.download = p.onDownload
    c.loading = p.loading
    c.progress = p.progress
    return <div>primary adapter</div>
  }
}))
import ModDownloadButton from "./index"

const cleanups: (() => void)[] = []
const running = {
  type: "Running",
  value: { progress: { type: "Known", value: 0.2 } }
}
const addon = (id: string, type = "mod") => ({
  id,
  title: `mod-${id}`,
  platform: "modrinth",
  type
})
const failed = {
  type: "Running",
  value: {
    progress: {
      type: "Failed",
      value: { cause: [{ display: "local task failed" }] }
    }
  }
}
beforeEach(() => {
  vi.clearAllMocks()
  c.download = c.loading = c.progress = undefined
})
afterEach(() => {
  cleanups.splice(0).forEach((fn) => fn())
  document.body.replaceChildren()
})
async function tick() {
  for (let i = 0; i < 16; i++) await Promise.resolve()
  await new Promise((resolve) => setTimeout(resolve, 0))
}
function pending() {
  let resolve!: (id: number) => void
  let reject!: (error: Error) => void
  const promise = new Promise<number>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
function owner(
  options: {
    type?: string
    fileId?: string
    server?: boolean
    mods?: any[]
    terminal?: any
  } = {}
) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { staleTime: Infinity, retry: false },
      mutations: { retry: false }
    }
  })
  client.setQueryData(["vtask.getTaskOutcome", 61], options.terminal ?? running)
  client.setQueryData(["vtask.getTaskOutcome", 62], running)
  c.query.mockResolvedValue(running)
  const [project, setProject] = createSignal(addon("A", options.type))
  const [target, setTarget] = createSignal(7)
  const [server, setServer] = createSignal<number | undefined>(
    options.server ? 9 : undefined
  )
  const [fileId, setFileId] = createSignal<string | undefined>(options.fileId)
  const [mods, setMods] = createSignal(options.mods ?? [])
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <ModDownloadButton
          addon={project() as any}
          fileId={fileId()}
          selectedInstanceId={target()}
          selectedServerId={server()}
          selectedInstanceMods={mods()}
          selectedServerAddons={[]}
        />
      </QueryClientProvider>
    ),
    root
  )
  let disposed = false
  const retire = () => {
    if (!disposed) {
      disposed = true
      dispose()
    }
  }
  cleanups.push(() => {
    retire()
    client.clear()
  })
  return {
    client,
    setProject,
    setTarget,
    setServer,
    setFileId,
    setMods,
    dispose: retire
  }
}
async function selectB(o: ReturnType<typeof owner>) {
  o.setProject(addon("B"))
  o.setTarget(8)
  o.setServer(undefined)
  o.setMods([])
  await tick()
}
async function complete(
  o: ReturnType<typeof owner>,
  id: number,
  outcome: any = { type: "Succeeded" }
) {
  o.client.setQueryData(["vtask.getTaskOutcome", id], outcome)
  await tick()
}
it("one accepted A task keeps its clicked title after selection changes", async () => {
  c.mutate.mockResolvedValue(61)
  const o = owner()
  await tick()
  await c.download()
  await tick()
  expect(c.mutate).toHaveBeenCalledWith("instance.installLatestMod", {
    instance_id: 7,
    mod_source: { Modrinth: "A" }
  })
  await selectB(o)
  await complete(o, 61)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:mod-A",
    {
      duration: 2000
    }
  )
  expect(c.loading()).toBe(false)
})
it("accepted B replaces observed A and keeps B presentation under real mutation observation", async () => {
  c.mutate.mockResolvedValueOnce(61).mockResolvedValueOnce(62)
  const o = owner()
  await tick()
  await c.download()
  await tick()
  await selectB(o)
  await c.download()
  await tick()
  expect(c.mutate).toHaveBeenLastCalledWith("instance.installLatestMod", {
    instance_id: 8,
    mod_source: { Modrinth: "B" }
  })
  await complete(o, 61)
  expect(c.success).not.toHaveBeenCalled()
  await complete(o, 62)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:mod-B",
    {
      duration: 2000
    }
  )
})
it("A completion while latest B awaits acceptance keeps A title and B pending loading", async () => {
  const b = pending()
  c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
  const o = owner()
  await tick()
  await c.download()
  await tick()
  await selectB(o)
  const clickedB = c.download()
  await tick()
  expect(c.mutate).toHaveBeenLastCalledWith("instance.installLatestMod", {
    instance_id: 8,
    mod_source: { Modrinth: "B" }
  })
  await complete(o, 61)
  expect(c.success.mock.calls).toEqual([
    ["notifications:_trn_addon_installed:mod-A", { duration: 2000 }]
  ])
  expect(c.loading()).toBe(true)
  expect(c.progress()).toBe(null)
  b.resolve(62)
  await clickedB
  await tick()
  expect(c.loading()).toBe(true)
  expect(c.progress()).toBe(20)
  await complete(o, 62)
  expect(c.loading()).toBe(false)
  expect(c.success).toHaveBeenLastCalledWith(
    "notifications:_trn_addon_installed:mod-B",
    {
      duration: 2000
    }
  )
})
it("specific version B pending cannot replace accepted A presentation", async () => {
  const b = pending()
  c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
  const o = owner({ fileId: "v-A" })
  await tick()
  await c.download()
  await tick()
  await selectB(o)
  o.setFileId("v-B")
  await tick()
  const clickedB = c.download()
  await tick()
  expect(c.mutate).toHaveBeenLastCalledWith("instance.installMod", {
    instance_id: 8,
    mod_source: { Modrinth: { project_id: "B", version_id: "v-B" } },
    install_deps: true,
    replaces_mod: null
  })
  await complete(o, 61)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:mod-A",
    {
      duration: 2000
    }
  )
  expect(c.loading()).toBe(true)
  b.resolve(62)
  await clickedB
  await tick()
  expect(c.progress()).toBe(20)
})
it.each([
  ["Failed", failed, true],
  ["Unavailable", { type: "Unavailable" }, true],
  ["Dismissed", { type: "Dismissed" }, false]
])(
  "accepted A %s while B pending preserves outcome distinction and B loading",
  async (_name, outcome, error) => {
    const b = pending()
    c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
    const o = owner()
    await tick()
    await c.download()
    await tick()
    await selectB(o)
    const clickedB = c.download()
    await tick()
    await complete(o, 61, outcome)
    expect(c.success).not.toHaveBeenCalled()
    if (error)
      expect(c.error).toHaveBeenCalledWith(
        "notifications:_trn_addon_install_failed:mod-A",
        expect.objectContaining({ description: expect.any(String) })
      )
    else expect(c.error).not.toHaveBeenCalled()
    expect(c.loading()).toBe(true)
    b.resolve(62)
    await clickedB
    await tick()
    expect(c.progress()).toBe(20)
  }
)
it("accepted datapack type survives a pending mod request", async () => {
  const b = pending()
  c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
  const o = owner({ type: "datapack" })
  await tick()
  await c.download()
  await tick()
  await selectB(o)
  const clickedB = c.download()
  await tick()
  await complete(o, 61)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_datapack_downloaded:mod-A",
    { duration: 2000 }
  )
  b.resolve(62)
  await clickedB
})
it("accepted world version copy intent survives a pending ordinary mod request", async () => {
  const b = pending()
  c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
  const o = owner({
    type: "world",
    fileId: "v-A",
    mods: [
      {
        id: "old-world",
        addon_type: "worlds",
        modrinth: { project_id: "A", version_id: "old-version" }
      }
    ]
  })
  await tick()
  await c.download()
  await tick()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.installMod",
    expect.objectContaining({ replaces_mod: "old-world" })
  )
  await selectB(o)
  o.setFileId(undefined)
  await tick()
  const clickedB = c.download()
  await tick()
  await complete(o, 61)
  expect(c.success).toHaveBeenCalledWith(
    "content:_trn_world_version_copy_complete",
    { duration: 2000 }
  )
  b.resolve(62)
  await clickedB
})
it("accepted server datapack uses server completion while an instance mod awaits acceptance", async () => {
  const b = pending()
  c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
  const o = owner({ type: "datapack", server: true })
  await tick()
  await c.download()
  await tick()
  expect(c.mutate).toHaveBeenCalledWith("server.installLatestServerMod", {
    serverId: 9,
    modSource: { Modrinth: "A" }
  })
  c.success.mockClear()
  await selectB(o)
  const clickedB = c.download()
  await tick()
  await complete(o, 61)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:mod-A",
    {
      duration: 2000
    }
  )
  expect(c.loading()).toBe(true)
  b.resolve(62)
  await clickedB
})
it("specific server acceptance captures clicked title and source despite later selection", async () => {
  const a = pending()
  c.mutate.mockReturnValue(a.promise)
  const o = owner({ server: true, fileId: "v-A" })
  await tick()
  const clickedA = c.download()
  await tick()
  await selectB(o)
  expect(c.mutate).toHaveBeenCalledWith("server.installServerMod", {
    serverId: 9,
    modSource: { Modrinth: { project_id: "A", version_id: "v-A" } }
  })
  a.resolve(61)
  await clickedA
  await tick()
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_downloading_to_server:mod-A",
    { duration: 2000 }
  )
  await complete(o, 61)
  expect(c.success).toHaveBeenLastCalledWith(
    "notifications:_trn_addon_installed:mod-A",
    {
      duration: 2000
    }
  )
})
it.each(["latest", "specific"])(
  "late older %s acceptance cannot steal a later accepted B task",
  async (kind) => {
    const a = pending()
    c.mutate.mockReturnValueOnce(a.promise).mockResolvedValueOnce(62)
    const o = owner({ fileId: kind === "specific" ? "v-A" : undefined })
    await tick()
    const clickedA = c.download()
    await tick()
    await selectB(o)
    o.setFileId(undefined)
    await tick()
    await c.download()
    await tick()
    a.resolve(61)
    await clickedA
    await tick()
    await complete(o, 61)
    expect(c.success).not.toHaveBeenCalled()
    expect(c.loading()).toBe(true)
    expect(c.progress()).toBe(20)
    await complete(o, 62)
    expect(c.success).toHaveBeenCalledWith(
      "notifications:_trn_addon_installed:mod-B",
      {
        duration: 2000
      }
    )
  }
)
it("old acceptance while newer B is pending does not publish an obsolete A observer", async () => {
  const a = pending(),
    b = pending()
  c.mutate.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise)
  const o = owner()
  await tick()
  const clickedA = c.download()
  await tick()
  await selectB(o)
  const clickedB = c.download()
  await tick()
  a.resolve(61)
  await clickedA
  await tick()
  await complete(o, 61)
  expect(c.success).not.toHaveBeenCalled()
  expect(c.loading()).toBe(true)
  b.resolve(62)
  await clickedB
  await tick()
  await complete(o, 62)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:mod-B",
    {
      duration: 2000
    }
  )
})
it("older rejection does not clear newer pending B loading", async () => {
  const a = pending(),
    b = pending()
  c.mutate.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise)
  const o = owner()
  await tick()
  const clickedA = c.download()
  await tick()
  await selectB(o)
  const clickedB = c.download()
  await tick()
  a.reject(new Error("local admission rejected"))
  await clickedA
  await tick()
  expect(c.loading()).toBe(true)
  b.resolve(62)
  await clickedB
  await tick()
  expect(c.progress()).toBe(20)
})
it("current B rejection leaves previously accepted A progress and outcome observable", async () => {
  const b = pending()
  c.mutate.mockResolvedValueOnce(61).mockReturnValueOnce(b.promise)
  const o = owner()
  await tick()
  await c.download()
  await tick()
  await selectB(o)
  const clickedB = c.download()
  await tick()
  b.reject(new Error("local admission rejected"))
  await clickedB
  await tick()
  expect(c.loading()).toBe(true)
  expect(c.progress()).toBe(20)
  await complete(o, 61)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:mod-A",
    {
      duration: 2000
    }
  )
  expect(c.loading()).toBe(false)
})
it("disposal retires awaited acceptance publication without cancelling entered mutation", async () => {
  const a = pending()
  c.mutate.mockReturnValue(a.promise)
  const o = owner({ terminal: { type: "Succeeded" } })
  await tick()
  const clickedA = c.download()
  await tick()
  expect(c.mutate).toHaveBeenCalledTimes(1)
  o.dispose()
  a.resolve(61)
  await clickedA
  await tick()
  expect(c.success).not.toHaveBeenCalled()
  expect(c.error).not.toHaveBeenCalled()
  expect(
    o.client
      .getQueryCache()
      .find({ queryKey: ["vtask.getTaskOutcome", 61] })
      ?.getObserversCount()
  ).toBe(0)
})
it("immediate cached success after latest acceptance completes once and leaves loading idle", async () => {
  c.mutate.mockResolvedValue(61)
  owner({ terminal: { type: "Succeeded" } })
  await tick()
  await c.download()
  await tick()
  expect(c.success.mock.calls).toEqual([
    ["notifications:_trn_addon_installed:mod-A", { duration: 2000 }]
  ])
  expect(c.loading()).toBe(false)
  expect(c.progress()).toBe(null)
})
it("immediate cached failure after specific acceptance leaves loading idle", async () => {
  c.mutate.mockResolvedValue(61)
  owner({ fileId: "v-A", terminal: failed })
  await tick()
  await c.download()
  await tick()
  expect(c.error).toHaveBeenCalledWith(
    "notifications:_trn_addon_install_failed:mod-A",
    { description: "local task failed" }
  )
  expect(c.success).not.toHaveBeenCalled()
  expect(c.loading()).toBe(false)
})
it("immediate server success presents downloading before completion and leaves loading idle", async () => {
  c.mutate.mockResolvedValue(61)
  owner({ server: true, terminal: { type: "Succeeded" } })
  await tick()
  await c.download()
  await tick()
  expect(c.success.mock.calls).toEqual([
    [
      "notifications:_trn_addon_downloading_to_server:mod-A",
      { duration: 2000 }
    ],
    ["notifications:_trn_addon_installed:mod-A", { duration: 2000 }]
  ])
  expect(c.loading()).toBe(false)
  expect(c.progress()).toBe(null)
})
it.each([
  ["Dismissed", { type: "Dismissed" }, false],
  ["Unavailable", { type: "Unavailable" }, true]
])(
  "immediate %s after acceptance retires loading with correct feedback distinction",
  async (_name, terminal, error) => {
    c.mutate.mockResolvedValue(61)
    owner({ terminal })
    await tick()
    await c.download()
    await tick()
    expect(c.loading()).toBe(false)
    expect(c.success).not.toHaveBeenCalled()
    expect(c.error).toHaveBeenCalledTimes(error ? 1 : 0)
  }
)
