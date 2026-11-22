import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { FilterSidebar } from "./index"
import { CollapsedSidebar } from "./CollapsedSidebar"

const c = vi.hoisted(() => ({
  search: undefined as any,
  navigate: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => <button onClick={p.onClick}>{p.children}</button>
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/components/SearchInputContext", () => ({ default: () => c.search }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({ settings: { data: { reducedMotion: true } } })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "",
  getServerImageUrl: () => ""
}))
vi.mock("./PlatformFilter", () => ({ PlatformFilter: () => null }))
vi.mock("./CategoriesFilter", () => ({ CategoriesFilter: () => null }))
vi.mock("./ModloadersFilter", () => ({ ModloadersFilter: () => null }))
vi.mock("./GameVersionsFilter", () => ({ GameVersionsFilter: () => null }))
vi.mock("./EnvironmentFilter", () => ({ EnvironmentFilter: () => null }))
vi.mock("./SortFilter", () => ({ SortFilter: () => null }))
vi.mock("../ViewModeToggle", () => ({ ViewModeToggle: () => null }))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.navigate.mockReset()
})
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
function context(
  worldName: string | undefined,
  world?: { name: string; title?: string | null; identity: string | null }
) {
  c.search = {
    searchQuery: () => ({}),
    setSearchQuery: vi.fn(),
    sidebarExpanded: () => true,
    setSidebarExpanded: vi.fn(),
    sidebarReady: () => true,
    sidebarApplies: () => true,
    selectedInstanceId: () => 3,
    selectedInstance: { data: { name: "Addon test", iconRevision: null } },
    selectedServerId: () => undefined,
    setSelectedInstanceId: vi.fn(),
    selectedWorld: {
      get name() {
        return worldName
      },
      get data() {
        return world
      }
    }
  }
}
const backButtons = (root: HTMLElement) =>
  [...root.querySelectorAll(".i-hugeicons\\:arrow-left-01")].map(
    (icon) => icon.closest("button")!
  )
// The strip's icon comes first, then the docked panel's header.
const backButton = (root: HTMLElement) => backButtons(root).at(-1)!

it("names the world with its instance, and goes back to the Worlds tab", () => {
  context("My World", { name: "My World", identity: "id" })
  const root = mount(() => <FilterSidebar />)
  const header = backButton(root).parentElement!
  expect(header.textContent).toContain("My World")
  expect(header.textContent).toContain("Addon test")
  expect(header.textContent).not.toContain("search:_trn_context_instance")
  backButton(root).click()
  expect(c.navigate).toHaveBeenCalledWith("/library/3/worlds")
})
it("names the world by its in-game name, not its folder", () => {
  context("New World (2)", {
    name: "New World (2)",
    title: "Skyblock with friends",
    identity: "id"
  })
  const root = mount(() => <FilterSidebar />)
  const header = backButton(root).parentElement!
  expect(header.textContent).toContain("Skyblock with friends")
  expect(header.textContent).not.toContain("New World (2)")
})
it.each([
  ["missing", undefined],
  ["unreadable", { name: "My World", identity: null }]
])("a %s world shows the plain state of a missing target", (_, world) => {
  context("My World", world)
  const root = mount(() => <FilterSidebar />)
  const header = backButton(root).parentElement!
  expect(header.textContent).toContain("...")
  expect(header.textContent).toContain("Addon test")
})
it("an instance target keeps its header and goes back to Addons", () => {
  context(undefined)
  const root = mount(() => <FilterSidebar />)
  const header = backButton(root).parentElement!
  expect(header.textContent).toContain("Addon test")
  expect(header.textContent).toContain("search:_trn_context_instance")
  backButton(root).click()
  expect(c.navigate).toHaveBeenCalledWith("/library/3/addons")
})
it.each([
  ["My World", "/library/3/worlds"],
  [undefined, "/library/3/addons"]
])("the collapsed back button for world %s goes to %s", (name, path) => {
  context(name, name ? { name, identity: "id" } : undefined)
  const root = mount(() => <CollapsedSidebar />)
  backButton(root).click()
  expect(c.navigate).toHaveBeenCalledWith(path)
})
