import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"

// The real @gd/ui dropdown (Kobalte), so the menu's own open and close on a
// pick is what the test sees.
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
vi.mock("@/components/VirtuaWrapper", () => ({ VList: () => null }))
import { InstanceDropdown } from "./InstanceDropdown"

const instance = {
  id: 4,
  name: "Addon test",
  gameVersion: "1.21.1",
  modloader: "vanilla",
  locked: false,
  iconRevision: null,
  iconUrl: null
}
let dispose: (() => void) | undefined
beforeEach(() => {
  c.query.mockReset()
  vi.spyOn(window, "scrollTo").mockImplementation(() => {})
})
afterEach(() => {
  dispose?.()
  document.body.replaceChildren()
  vi.restoreAllMocks()
})
const flush = async () => {
  for (let i = 0; i < 8; i++) {
    await Promise.resolve()
    await new Promise((r) => setTimeout(r, 0))
  }
}
function mount(type: string, instances = [instance]) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } }
  })
  const selectInstance = vi.fn()
  const selectWorld = vi.fn()
  const [loading, setLoading] = createSignal(new Map<number, boolean>())
  const [query, setQuery] = createSignal("")
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <InstanceDropdown
          addon={{ id: "P", platform: "modrinth", type } as any}
          filteredInstances={() => instances}
          shouldVirtualize={() => false}
          searchQuery={query}
          setSearchQuery={setQuery}
          hoveredInstanceId={() => null}
          setHoveredInstanceId={() => {}}
          instanceLoadingStates={loading}
          clearInstanceLoadingState={() => {}}
          handleInstanceSelection={(id) => {
            selectInstance(id)
            setLoading(new Map([[id, true]]))
          }}
          handleWorldSelection={selectWorld}
        />
      </QueryClientProvider>
    ),
    root
  )
  return { root, selectInstance, selectWorld }
}
async function open(root: HTMLElement) {
  const trigger = root.querySelector<HTMLElement>("button")!
  trigger.dispatchEvent(
    new MouseEvent("pointerdown", { button: 0, bubbles: true })
  )
  trigger.click()
  await flush()
}
// Kobalte selects a menu item on pointer up.
const pick = (item: HTMLElement) =>
  item.dispatchEvent(new MouseEvent("pointerup", { button: 0, bubbles: true }))
// The trigger says whether the menu is open; jsdom keeps closed content mounted.
const isOpen = () =>
  document.querySelector("[aria-haspopup]")?.getAttribute("aria-expanded") ===
  "true"
const instanceItem = () =>
  [...document.querySelectorAll<HTMLElement>('[role="menuitemradio"]')].find(
    (item) => item.textContent?.includes("Addon test")
  )

it.each(["mod", "resourcePack", "shader", "world"])(
  "a %s pick keeps the menu open with the row's progress",
  async (type) => {
    c.query.mockImplementation(async () => [])
    const d = mount(type)
    await open(d.root)
    expect(isOpen()).toBe(true)
    pick(instanceItem()!)
    await flush()
    expect(d.selectInstance).toHaveBeenCalledWith(4)
    expect(isOpen()).toBe(true)
    expect(instanceItem()?.textContent).toContain(
      "instances:_trn_status_installing"
    )
  }
)
it("a data pack for an instance with no readable world installs and keeps the menu open", async () => {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds"
      ? [{ name: "Broken", addon_id: "b", identity: null, datapacks: [] }]
      : []
  )
  const d = mount("datapack")
  await open(d.root)
  pick(instanceItem()!)
  await flush()
  expect(d.selectInstance).toHaveBeenCalledWith(4)
  expect(isOpen()).toBe(true)
  expect(instanceItem()?.textContent).toContain(
    "instances:_trn_status_downloading"
  )
})
it("the menu opens on its trigger and closes on Escape, as before", async () => {
  c.query.mockImplementation(async () => [])
  const d = mount("mod")
  await open(d.root)
  expect(isOpen()).toBe(true)
  document.activeElement!.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
  )
  await flush()
  expect(isOpen()).toBe(false)
})
const heldCopy = {
  id: "copy",
  addon_type: "datapacks",
  modrinth: { project_id: "P", version_id: "V1" },
  curseforge: null
}
const world = (name: string, datapacks: any[] = []) => ({
  name,
  addon_id: name,
  identity: `id-${name}`,
  datapacks
})
function serve(worlds: any[], mods: any[]) {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds"
      ? worlds
      : key === "instance.getInstanceMods"
        ? mods
        : []
  )
}
const items = () =>
  [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].map((item) =>
    item.textContent?.trim()
  )
it.each([
  ["no world", []],
  ["only worlds that hold it too", [world("My World", [heldCopy])]]
])(
  "an instance holding the pack with %s is greyed with its tick, with no step",
  async (_, worlds) => {
    serve(worlds, [heldCopy])
    const d = mount("datapack")
    await open(d.root)
    const row = instanceItem()!
    expect(row.hasAttribute("data-disabled")).toBe(true)
    expect(row.querySelector(".i-hugeicons\\:tick-02")).not.toBeNull()
    // No world is left to add it to, so the row does not say to add it.
    expect(row.textContent).toContain("instances:_trn_status_installed")
    expect(row.textContent).not.toContain("content:_trn_datapack_downloaded")
    pick(row)
    await flush()
    expect(d.selectInstance).not.toHaveBeenCalled()
    expect(items()).toEqual([])
  }
)
it("an instance holding the pack with a world that can take it asks where, Instance ticked", async () => {
  serve([world("My World")], [heldCopy])
  const d = mount("datapack")
  await open(d.root)
  const row = instanceItem()!
  expect(row.hasAttribute("data-disabled")).toBe(false)
  pick(row)
  await flush()
  expect(items()).toEqual(["My World", "content:_trn_worlds.instance_copy"])
  const instanceOption = [
    ...document.querySelectorAll<HTMLElement>('[role="menuitem"]')
  ].at(-1)!
  expect(instanceOption.hasAttribute("data-disabled")).toBe(true)
  expect(isOpen()).toBe(true)
})
it("a world pick installs there and keeps the menu open on the list", async () => {
  const target = world("My World")
  serve([target], [])
  const d = mount("datapack")
  await open(d.root)
  pick(instanceItem()!)
  await flush()
  pick(
    [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
      (item) => item.textContent?.includes("My World")
    )!
  )
  await flush()
  expect(d.selectWorld).toHaveBeenCalledWith(4, target)
  expect(d.selectInstance).not.toHaveBeenCalled()
  expect(isOpen()).toBe(true)
  expect(instanceItem()).toBeDefined()
})
const others = [5, 6, 7].map((id) => ({
  ...instance,
  id,
  name: `Other ${id}`
}))
const worldReads = () =>
  c.query.mock.calls
    .map(([key]) => key)
    .filter((key) => key[0] === "instance.getWorlds")
async function close() {
  document.activeElement!.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
  )
  await flush()
}
it("opening the picker reads no worlds when no instance holds the pack", async () => {
  serve([world("My World")], [])
  const d = mount("datapack", [instance, ...others])
  await open(d.root)
  expect(isOpen()).toBe(true)
  expect(worldReads()).toEqual([])
})
it("opening the picker reads worlds only for the instance holding the pack, once while fresh", async () => {
  c.query.mockImplementation(async ([key, id]: any) =>
    key === "instance.getWorlds"
      ? [world("My World")]
      : key === "instance.getInstanceMods" && id === 6
        ? [heldCopy]
        : []
  )
  const d = mount("datapack", [instance, ...others])
  await open(d.root)
  expect(worldReads()).toEqual([["instance.getWorlds", 6]])
  await close()
  expect(isOpen()).toBe(false)
  await open(d.root)
  expect(isOpen()).toBe(true)
  expect(worldReads()).toEqual([["instance.getWorlds", 6]])
})
it("picking an instance that does not hold the pack reads its worlds then", async () => {
  serve([world("My World")], [])
  const d = mount("datapack", [instance, ...others])
  await open(d.root)
  expect(worldReads()).toEqual([])
  pick(instanceItem()!)
  await flush()
  expect(worldReads()).toEqual([["instance.getWorlds", 4]])
  expect(items()).toEqual(["My World", "content:_trn_worlds.instance_copy"])
})
