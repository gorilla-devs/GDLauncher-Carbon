import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"

// Real query/mutation observers and task progress; transport, toast and the
// leaf controls are stubbed.
const c = vi.hoisted(() => ({
  query: vi.fn(),
  mutate: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  download: undefined as any,
  staged: undefined as boolean | undefined,
  worldSelect: undefined as any
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
    vars?.world
      ? `${key}:${vars.name}@${vars.world}`
      : vars?.title
        ? `${key}:${vars.title}`
        : key
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
  InstanceDropdown: (p: any) => {
    c.worldSelect = p.handleWorldSelection
    return null
  }
}))
vi.mock("./components/InstallButton", () => ({
  InstallButton: (p: any) => {
    c.download = p.onDownload
    c.staged = p.staged
    return null
  }
}))
import ModDownloadButton from "./index"

const world = {
  name: "My World",
  addon_id: "w",
  identity: "id-1",
  datapacks: []
}
const running = {
  type: "Running",
  value: { progress: { type: "Known", value: 0.2 } }
}
const pack = { id: "P", title: "Pack", platform: "modrinth", type: "datapack" }
const cleanups: (() => void)[] = []
beforeEach(() => {
  vi.clearAllMocks()
  c.download = c.worldSelect = undefined
})
afterEach(() => {
  cleanups.splice(0).forEach((fn) => fn())
  document.body.replaceChildren()
})
async function tick() {
  for (let i = 0; i < 16; i++) await Promise.resolve()
  await new Promise((resolve) => setTimeout(resolve, 0))
}
function mount(element: () => any) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { staleTime: Infinity, retry: false },
      mutations: { retry: false }
    }
  })
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(
    () => (
      <QueryClientProvider client={client}>{element()}</QueryClientProvider>
    ),
    root
  )
  cleanups.push(() => {
    dispose()
    client.clear()
  })
  return client
}

it("a world install's completion says the pack was added to that world", async () => {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? [world] : running
  )
  c.mutate.mockResolvedValue(61)
  const client = mount(() => (
    <ModDownloadButton
      addon={pack as any}
      selectedInstanceId={7}
      selectedInstanceMods={[]}
      selectedWorld="My World"
    />
  ))
  await tick()
  await c.download()
  await tick()
  expect(c.mutate).toHaveBeenCalledWith("instance.installWorldDatapack", {
    instance_id: 7,
    world: "My World",
    identity: "id-1",
    source: { Modrinth: { project_id: "P", version_id: null } },
    replaces: null
  })
  client.setQueryData(["vtask.getTaskOutcome", 61], { type: "Succeeded" })
  await tick()
  expect(c.success).toHaveBeenCalledWith(
    "content:_trn_worlds.added_to_world:Pack@My World",
    { duration: 2000 }
  )
  expect(c.success).not.toHaveBeenCalledWith(
    expect.stringContaining("notifications:_trn_datapack_downloaded"),
    expect.anything()
  )
})
it("a Modrinth data pack typed as a mod installs into the world and its completion names the world", async () => {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? [world] : running
  )
  c.mutate.mockResolvedValue(63)
  const client = mount(() => (
    <ModDownloadButton
      addon={
        {
          ...pack,
          type: "mod",
          loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
        } as any
      }
      selectedInstanceId={7}
      selectedInstanceMods={[]}
      selectedWorld="My World"
    />
  ))
  await tick()
  expect(c.staged).toBe(false)
  await c.download()
  await tick()
  expect(c.mutate.mock.calls.map(([key]) => key)).toEqual([
    "instance.installWorldDatapack"
  ])
  client.setQueryData(["vtask.getTaskOutcome", 63], { type: "Succeeded" })
  await tick()
  expect(c.success).toHaveBeenCalledWith(
    "content:_trn_worlds.added_to_world:Pack@My World",
    { duration: 2000 }
  )
})
it("a Modrinth data pack typed as a mod going into the instance is not told to be added to a world", async () => {
  c.query.mockImplementation(async () => running)
  mount(() => (
    <ModDownloadButton
      addon={
        {
          ...pack,
          type: "mod",
          loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
        } as any
      }
      selectedInstanceId={7}
      selectedInstanceMods={[]}
    />
  ))
  await tick()
  expect(c.staged).toBe(false)
})
it("a world picked in the instance dropdown installs there and its completion names it", async () => {
  c.query.mockImplementation(async ([key]: any) =>
    key === "vtask.getTaskOutcome" ? { type: "Succeeded" } : undefined
  )
  c.mutate.mockResolvedValue(62)
  mount(() => <ModDownloadButton addon={pack as any} />)
  await tick()
  await c.worldSelect(4, world)
  await tick()
  expect(c.mutate).toHaveBeenCalledWith("instance.installWorldDatapack", {
    instance_id: 4,
    world: "My World",
    identity: "id-1",
    source: { Modrinth: { project_id: "P", version_id: null } },
    replaces: null
  })
  await tick()
  expect(c.success).toHaveBeenCalledWith(
    "content:_trn_worlds.added_to_world:Pack@My World",
    { duration: 2000 }
  )
})
it("a refused world install reports no completion", async () => {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? [world] : running
  )
  c.mutate.mockRejectedValue(new Error("refused"))
  mount(() => (
    <ModDownloadButton
      addon={pack as any}
      selectedInstanceId={7}
      selectedWorld="My World"
    />
  ))
  await tick()
  await c.download()
  await tick()
  expect(c.mutate).toHaveBeenCalledTimes(1)
  expect(c.success).not.toHaveBeenCalled()
})
it.each([
  ["the instance", undefined, true],
  ["a world", "My World", false]
])(
  "the button for a data pack going into %s says to add it to a world: %s",
  async (_, selectedWorld, staged) => {
    c.query.mockImplementation(async ([key]: any) =>
      key === "instance.getWorlds" ? [world] : running
    )
    mount(() => (
      <ModDownloadButton
        addon={pack as any}
        selectedInstanceId={7}
        selectedWorld={selectedWorld}
      />
    ))
    await tick()
    expect(c.staged).toBe(staged)
  }
)
