import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"

const c = vi.hoisted(() => ({ query: vi.fn() }))
vi.mock("@/utils/rspcClient", async () => {
  const { createQuery } = await import("@tanstack/solid-query")
  return {
    rspc: {
      createQuery: (options: any) =>
        createQuery(() => ({
          ...options(),
          queryFn: () => c.query(options().queryKey)
        }))
    }
  }
})
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@gd/ui", () => {
  const Wrapper = (p: any) => <div>{p.children}</div>
  const Item = (p: any) => (
    <button
      data-item
      class={p.class}
      disabled={p.disabled}
      onClick={() => p.onSelect?.()}
    >
      {p.children}
    </button>
  )
  return {
    Button: Wrapper,
    DropdownMenu: Wrapper,
    DropdownMenuTrigger: Wrapper,
    DropdownMenuContent: Wrapper,
    DropdownMenuRadioGroup: Wrapper,
    DropdownMenuRadioItem: Item,
    DropdownMenuItem: Item,
    Input: () => <input />,
    Spinner: () => <span>spinner</span>
  }
})
vi.mock("@/components/VirtuaWrapper", () => ({ VList: () => null }))
import { InstanceDropdown } from "./InstanceDropdown"
import { SearchInputContext } from "@/components/SearchInputContext"

const instance = {
  id: 4,
  name: "Addon test",
  gameVersion: "1.21.1",
  modloader: "vanilla",
  locked: false,
  iconRevision: null,
  iconUrl: null
}
const world = (name: string, identity: string | null = `id-${name}`) => ({
  name,
  addon_id: name,
  identity,
  datapacks: [] as any[]
})
let dispose: (() => void) | undefined
beforeEach(() => {
  c.query.mockReset()
})
afterEach(() => {
  dispose?.()
  document.body.replaceChildren()
})
const flush = async () => {
  for (let i = 0; i < 6; i++) {
    await Promise.resolve()
    await new Promise((r) => setTimeout(r, 0))
  }
}
/** `searched` is the addon type the search the row sits in is showing. */
function mount(type: string, loaders?: string[], searched?: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } }
  })
  const selectInstance = vi.fn()
  const selectWorld = vi.fn()
  const [query, setQuery] = createSignal("")
  const root = document.createElement("div")
  document.body.append(root)
  const search = searched
    ? ({ searchQuery: () => ({ projectType: searched }) } as any)
    : undefined
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <SearchInputContext.Provider value={search}>
          <InstanceDropdown
            addon={{ id: "P", platform: "modrinth", type, loaders } as any}
            filteredInstances={() => [instance]}
            shouldVirtualize={() => false}
            searchQuery={query}
            setSearchQuery={setQuery}
            hoveredInstanceId={() => null}
            setHoveredInstanceId={() => {}}
            instanceLoadingStates={() => new Map()}
            clearInstanceLoadingState={() => {}}
            handleInstanceSelection={selectInstance}
            handleWorldSelection={selectWorld}
          />
        </SearchInputContext.Provider>
      </QueryClientProvider>
    ),
    root
  )
  const items = () =>
    [...root.querySelectorAll<HTMLButtonElement>("[data-item]")].map((b) =>
      b.textContent?.trim()
    )
  const click = (text: string) =>
    [...root.querySelectorAll<HTMLButtonElement>("[data-item]")]
      .find((b) => b.textContent?.includes(text))!
      .click()
  return { root, items, click, selectInstance, selectWorld }
}

it("for a data pack, a picked instance asks where: its readable worlds, then the instance", async () => {
  const worlds = [world("My World"), world("Broken", null), world("Skyblock")]
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? worlds : []
  )
  const d = mount("datapack")
  await flush()
  d.click("Addon test")
  await flush()
  expect(d.selectInstance).not.toHaveBeenCalled()
  expect(d.items()).toEqual([
    "My World",
    "Skyblock",
    "content:_trn_worlds.instance_copy"
  ])
  expect(d.root.textContent).toContain("Addon test")
  d.click("Skyblock")
  expect(d.selectWorld).toHaveBeenCalledWith(4, worlds[2])
  expect(d.selectInstance).not.toHaveBeenCalled()
})
it("for a data pack, picking Instance installs into the instance as before", async () => {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? [world("My World")] : []
  )
  const d = mount("datapack")
  await flush()
  d.click("Addon test")
  await flush()
  d.click("content:_trn_worlds.instance_copy")
  expect(d.selectInstance).toHaveBeenCalledWith(4)
  expect(d.selectWorld).not.toHaveBeenCalled()
})
it("a world already holding the project is shown as installed and cannot be picked", async () => {
  const holder = world("My World")
  holder.datapacks = [
    { id: "copy", modrinth: { project_id: "P", version_id: "V1" } }
  ]
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? [holder, world("Other")] : []
  )
  const d = mount("datapack")
  await flush()
  d.click("Addon test")
  await flush()
  const button = [
    ...d.root.querySelectorAll<HTMLButtonElement>("[data-item]")
  ].find((b) => b.textContent?.includes("My World"))!
  expect(button.disabled).toBe(true)
  expect(button.querySelector(".i-hugeicons\\:tick-02")).not.toBeNull()
})
it.each([
  ["no world", []],
  ["only unreadable worlds", [world("Broken", null)]]
])(
  "an instance with %s installs the data pack at instance level without asking",
  async (_, worlds) => {
    c.query.mockImplementation(async ([key]: any) =>
      key === "instance.getWorlds" ? worlds : []
    )
    const d = mount("datapack")
    await flush()
    d.click("Addon test")
    await flush()
    expect(d.selectInstance).toHaveBeenCalledWith(4)
    expect(d.selectWorld).not.toHaveBeenCalled()
    expect(d.items()).toEqual([expect.stringContaining("Addon test")])
  }
)
it("an instance whose worlds cannot be read installs at instance level", async () => {
  c.query.mockImplementation(async ([key]: any) => {
    if (key === "instance.getWorlds") throw new Error("unreadable")
    return []
  })
  const d = mount("datapack")
  await flush()
  d.click("Addon test")
  await flush()
  expect(d.selectInstance).toHaveBeenCalledWith(4)
})
it.each(["mod", "resourcePack", "shader", "world"])(
  "a %s installs on the instance pick, without asking for a world",
  async (type) => {
    c.query.mockImplementation(async () => [])
    const d = mount(type)
    await flush()
    d.click("Addon test")
    expect(d.selectInstance).toHaveBeenCalledWith(4)
    await flush()
    expect(
      c.query.mock.calls.some(([key]) => key[0] === "instance.getWorlds")
    ).toBe(false)
  }
)
// Modrinth types a data pack "mod" and lists "datapack" among its loaders.
const hybridLoaders = ["datapack", "fabric", "forge", "neoforge", "quilt"]
const serveWorld = () =>
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds" ? [world("My World")] : []
  )
const worldsRead = () =>
  c.query.mock.calls.some(([key]) => key[0] === "instance.getWorlds")
it("a Modrinth data pack that also ships as a mod asks where while data packs are searched", async () => {
  serveWorld()
  const d = mount("mod", hybridLoaders, "datapack")
  await flush()
  d.click("Addon test")
  await flush()
  expect(d.selectInstance).not.toHaveBeenCalled()
  expect(d.items()).toEqual(["My World", "content:_trn_worlds.instance_copy"])
  d.click("My World")
  expect(d.selectWorld).toHaveBeenCalledWith(4, world("My World"))
})
it.each(["mod", undefined])(
  "a Modrinth data pack that also ships as a mod installs its mod in one click when %s is searched",
  async (searched) => {
    serveWorld()
    const d = mount("mod", hybridLoaders, searched)
    await flush()
    d.click("Addon test")
    expect(d.selectInstance).toHaveBeenCalledWith(4)
    await flush()
    expect(worldsRead()).toBe(false)
  }
)
it.each(["datapack", "mod", undefined])(
  "a Modrinth project that is only a data pack asks where when %s is searched",
  async (searched) => {
    serveWorld()
    const d = mount("mod", ["datapack"], searched)
    await flush()
    d.click("Addon test")
    await flush()
    expect(d.selectInstance).not.toHaveBeenCalled()
    expect(d.items()).toEqual(["My World", "content:_trn_worlds.instance_copy"])
  }
)
it("a plain mod never asks where, even while data packs are searched", async () => {
  serveWorld()
  const d = mount("mod", ["fabric"], "datapack")
  await flush()
  d.click("Addon test")
  expect(d.selectInstance).toHaveBeenCalledWith(4)
  await flush()
  expect(worldsRead()).toBe(false)
})
