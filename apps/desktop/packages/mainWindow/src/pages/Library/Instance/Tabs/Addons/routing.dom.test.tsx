import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import type { AddonType, Mod } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  params: null as any,
  state: null as any,
  navigate: vi.fn(),
  mutation: vi.fn()
}))
vi.mock("@solidjs/router", () => ({ useParams: () => io.params }))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: io.navigate })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/pages/Library/Instance/instance.data", () => ({
  default: () => ({
    instanceDetails: {
      get data() {
        return io.state.details
      }
    }
  })
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { ensureQueryData: vi.fn() },
  rspc: {
    useContext: () => ({ client: { query: vi.fn() } }),
    createQuery: (options: any) =>
      options().queryKey[0] === "instance.getInstanceMods"
        ? {
            get data() {
              return io.state.rows
            },
            isLoading: false,
            refetch: vi.fn()
          }
        : { data: true, isLoading: false },
    createMutation: () => ({ mutate: io.mutation, mutateAsync: vi.fn() })
  }
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/utils/instances", () => ({
  getModImageUrl: () => {
    throw new Error("Image route excluded")
  }
}))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => <span />
}))
vi.mock("./components/DatapackStaging", () => ({
  DatapackStaging: () => <span />
}))
vi.mock("@/pages/Library/shared/addons/components/AddonTable", () => ({
  AddonTable: () => <div data-table />
}))
vi.mock("@/pages/Library/shared/addons/components/columns", () => ({
  createAddonColumns: () => []
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../../../../packages/ui/src/Button")),
  ...(await import("../../../../../../../../../../packages/ui/src/Badge")),
  ...(await import("../../../../../../../../../../packages/ui/src/Input")),
  Tooltip: (p: any) => <>{p.children}</>,
  TooltipTrigger: (p: any) => <>{p.children}</>,
  TooltipContent: () => <span />,
  Select: () => <span />,
  toast: { error: vi.fn() }
}))

import Addons from "./index"
import { useAddonMutations } from "./hooks/useAddonMutations"

const kinds: AddonType[] = [
  "mods",
  "shaders",
  "resourcepacks",
  "datapacks",
  "worlds"
]
const routes = ["mod", "shader", "resourcePack", "datapack", "world"]
const addon = (kind: AddonType): Mod => ({
  id: kind,
  addon_type: kind,
  filename: `${kind}.zip`,
  enabled: true,
  metadata: null,
  curseforge: null,
  modrinth: null,
  has_update: false,
  is_duplicate: false,
  file_size: 1
})
let dispose: (() => void) | undefined
let setState: any
let sequence = 70
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
beforeEach(() => {
  vi.clearAllMocks()
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    }
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.unstubAllGlobals()
})
function fixture(
  id = String(++sequence),
  rows = kinds.map(addon),
  details: any = { modloaders: [{}], modpack: null }
) {
  io.params = { id }
  const [state, set] = createStore({ rows, details })
  io.state = state
  setState = set
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Addons />, root)
  return root
}
function filter(root: HTMLElement, kind: AddonType) {
  const badge = [...root.querySelectorAll<HTMLElement>("[class]")].find(
    (el) =>
      el.classList.contains("cursor-pointer") &&
      el.textContent === `content:_trn_tabs.${kind}`
  )
  expect(badge, `filter ${kind}`).toBeDefined()
  badge!.click()
}
function add(root: HTMLElement) {
  return root.querySelector<HTMLButtonElement>(
    '[data-testid="addons-add-button"]'
  )!
}
function hideRows(root: HTMLElement) {
  const input = root.querySelector<HTMLInputElement>("input")!
  input.value = "no matching fixture"
  input.dispatchEvent(new Event("input", { bubbles: true }))
}

it.each(kinds.map((kind, i) => [kind, routes[i]] as const))(
  "actual gotoSearchPage %s emits %s with target",
  (kind, route) => {
    io.params = { id: "7" }
    io.state = { rows: [] }
    const actions = createRoot((end) => {
      dispose = end
      return useAddonMutations(
        vi.fn(),
        {
          optimisticToggleAddon: vi.fn(),
          optimisticDeleteAddon: vi.fn(),
          optimisticDeleteAddons: vi.fn(),
          optimisticUpdateAddon: vi.fn(),
          rollbackToServerState: vi.fn(),
          startUpdatingMod: vi.fn(),
          stopUpdatingMod: vi.fn()
        },
        vi.fn()
      )
    })
    actions.gotoSearchPage(kind)
    expect(io.navigate).toHaveBeenCalledExactlyOnceWith(
      `/search/${route}?instanceId=7`
    )
  }
)
it("optional producer type preserves missing-segment target navigation", () => {
  io.params = { id: "7" }
  io.state = { rows: [] }
  createRoot((end) => {
    dispose = end
    useAddonMutations(vi.fn(), {} as any, vi.fn()).gotoSearchPage()
  })
  expect(io.navigate).toHaveBeenCalledExactlyOnceWith("/search/?instanceId=7")
})
it.each(["toolbar", "empty", "cached"])(
  "mounted %s Add action uses sole stored resourcepacks",
  async (action) => {
    const id = String(++sequence)
    let root = fixture(id)
    await flush()
    for (const kind of kinds.filter((kind) => kind !== "resourcepacks"))
      filter(root, kind)
    await flush()
    if (action === "cached") {
      dispose!()
      root.remove()
      root = fixture(id)
      await flush()
    }
    if (action === "empty") {
      hideRows(root)
      await flush()
      const empty = [
        ...root.querySelectorAll<HTMLButtonElement>("button")
      ].find(
        (button) =>
          button !== add(root) &&
          button.textContent?.includes("content:_trn_add_addons")
      )!
      expect(empty).toBeDefined()
      empty.click()
    } else add(root).click()
    expect(io.navigate).toHaveBeenLastCalledWith(
      `/search/resourcePack?instanceId=${id}`
    )
  }
)
it.each(kinds.map((kind, i) => [kind, routes[i]] as const))(
  "mounted sole %s selects %s",
  async (kind, route) => {
    const id = String(++sequence),
      root = fixture(id)
    await flush()
    for (const other of kinds.filter((value) => value !== kind))
      filter(root, other)
    add(root).click()
    expect(io.navigate).toHaveBeenLastCalledWith(
      `/search/${route}?instanceId=${id}`
    )
  }
)
it("all five stored keys choose mod even when only Resource packs is visible", async () => {
  const id = String(++sequence),
    root = fixture(id, [addon("resourcepacks")])
  await flush()
  add(root).click()
  expect(io.navigate).toHaveBeenLastCalledWith(`/search/mod?instanceId=${id}`)
})
it("pending details choose mods and resolved vanilla chooses shaders", async () => {
  const id = String(++sequence),
    root = fixture(id, [], null)
  await flush()
  add(root).click()
  expect(io.navigate).toHaveBeenLastCalledWith(`/search/mod?instanceId=${id}`)
  setState("details", { modloaders: [], modpack: null })
  await flush()
  add(root).click()
  expect(io.navigate).toHaveBeenLastCalledWith(
    `/search/shader?instanceId=${id}`
  )
})
it("locked instance retains disabled toolbar action", async () => {
  const root = fixture(undefined, kinds.map(addon), {
    modloaders: [{}],
    modpack: { locked: true }
  })
  await flush()
  expect(add(root).disabled).toBe(true)
  add(root).click()
  expect(io.navigate).not.toHaveBeenCalled()
})
