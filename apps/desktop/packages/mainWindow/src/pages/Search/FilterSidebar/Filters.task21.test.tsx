import { afterEach, expect, it, vi } from "vitest"
import { createSignal, For } from "solid-js"
import { render } from "solid-js/web"
import { GameVersionsFilter } from "./GameVersionsFilter"
import GameVersionsDisplay from "../FiltersDisplay/GameVersionsDisplay"
import { CategoriesFilter } from "./CategoriesFilter"
import { SearchCategoryDropdown } from "@/components/NavSearchInputFilters"
import { createPlatformSearchQuery } from "@/utils/platformSearchQuery"
const control = vi.hoisted(() => ({
  search: undefined as any,
  category: undefined as any,
  refetch: vi.fn()
}))
vi.mock("@/components/SearchInputContext", () => ({
  default: () => control.search
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    minecraftVersions: { data: [{ id: "1.20.1" }, { id: "1.21.1" }] }
  })
}))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/helpers", () => ({ capitalize: (s: string) => s }))
vi.mock("@/utils/sidebar", () => ({ ModloaderIcon: () => null }))
vi.mock("./shared", () => ({ FilterWarning: () => null }))
vi.mock("../FiltersDisplay/FilterBadge", () => ({
  FilterBadge: (p: any) => (
    <button data-badge onClick={p.onClick}>
      {p.children}
    </button>
  )
}))
vi.mock("./SearchableCheckboxList", () => ({
  SearchableCheckboxList: (p: any) => (
    <div>
      <For each={p.items}>
        {(item: any) => (
          <button
            data-choice={item.value}
            data-checked={p.selectedValues().includes(item.value)}
            onClick={() =>
              p.onToggle(item.value, !p.selectedValues().includes(item.value))
            }
          >
            {item.label}
          </button>
        )}
      </For>
    </div>
  )
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: { createQuery: (options: any) => control.category(options()) }
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(s: string) => s]
}))
vi.mock("@gd/ui", () => {
  const passthrough = (p: any) => <div>{p.children}</div>
  return {
    Collapsable: passthrough,
    DropdownMenuCheckboxItem: (p: any) => (
      <button
        disabled={p.disabled}
        data-category
        onClick={() => p.onChange(!p.checked)}
      >
        {p.children}
      </button>
    ),
    DropdownMenuGroup: passthrough,
    DropdownMenuLabel: passthrough,
    DropdownMenuPortal: passthrough,
    DropdownMenuRadioGroup: passthrough,
    DropdownMenuRadioItem: passthrough,
    DropdownMenuSeparator: passthrough,
    DropdownMenuSub: passthrough,
    DropdownMenuSubContent: passthrough,
    DropdownMenuSubTrigger: passthrough,
    Input: () => null
  }
})
let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  control.refetch.mockClear()
})
const flush = async () => {
  for (let i = 0; i < 4; i++) await Promise.resolve()
}
function mountVersion(api: any) {
  const [get, set] = createPlatformSearchQuery({
    searchApi: api,
    gameVersions: null
  } as any)
  control.search = {
    searchQuery: get,
    setSearchQuery: set,
    selectedInstanceId: () => undefined
  }
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <>
        <GameVersionsFilter />
        <GameVersionsDisplay />
      </>
    ),
    root
  )
  return { root, get, set }
}
it("actual CF checkbox and badges replace previous version; Modrinth preserves OR", async () => {
  const { root, get, set } = mountVersion("curseforge")
  root.querySelector<HTMLButtonElement>('[data-choice="1.20.1"]')!.click()
  root.querySelector<HTMLButtonElement>('[data-choice="1.21.1"]')!.click()
  await flush()
  expect(get().gameVersions).toEqual(["1.21.1"])
  expect(
    [...root.querySelectorAll("[data-badge]")].map((b) => b.textContent)
  ).toEqual(["1.21.1"])
  expect(root.textContent).toContain("search:_trn_curseforge_single_version")
  set((prev) => ({ ...prev, searchApi: "modrinth" }))
  root.querySelector<HTMLButtonElement>('[data-choice="1.20.1"]')!.click()
  await flush()
  expect(get().gameVersions).toEqual(["1.21.1", "1.20.1"])
  expect(root.querySelectorAll("[data-badge]")).toHaveLength(2)
  set((prev) => ({ ...prev, searchApi: null }))
  await flush()
  expect(root.querySelectorAll("[data-badge]")).toHaveLength(1)
})
it.each(["active", "legacy"])(
  "%s category consumer recovers deferred query, keeps healthy data on failed refetch and exposes Retry",
  async (kind) => {
    const [get, set] = createPlatformSearchQuery({
      searchApi: "curseforge",
      categories: null,
      projectType: "modpack"
    } as any)
    control.search = {
      searchQuery: get,
      setSearchQuery: set,
      selectedInstanceId: () => undefined
    }
    const [raw, setRaw] = createSignal<any>(undefined)
    control.category = (options: any) => {
      let previous: any
      return {
        get data() {
          const incoming = raw()
          if (incoming) {
            previous = options.structuralSharing
              ? options.structuralSharing(previous, incoming)
              : incoming
          }
          return previous
        },
        refetch: control.refetch
      }
    }
    const root = document.createElement("div")
    document.body.append(root)
    dispose = render(
      () =>
        kind === "active" ? <CategoriesFilter /> : <SearchCategoryDropdown />,
      root
    )
    const cf: any = {
      id: 1,
      name: "healthy CF",
      projectType: "modpack",
      icon: null
    }
    setRaw({
      curseforge: { 1: cf },
      modrinth: {},
      curseforgeError: null,
      modrinthError: "MR offline"
    })
    await flush()
    expect(root.textContent).toContain("healthy CF")
    const healthy = [...root.querySelectorAll("button")].find(
      (b) => b.textContent === "healthy CF"
    )!
    healthy.click()
    expect(get().categories).toEqual([1])
    setRaw({
      curseforge: {},
      modrinth: {},
      curseforgeError: "CF offline",
      modrinthError: "MR offline"
    })
    await flush()
    expect(root.textContent).toContain("healthy CF")
    expect(root.textContent).toContain("CF offline")
    const retry = [...root.querySelectorAll('[role="alert"]')]
      .find((alert) => alert.textContent?.includes("CF offline"))!
      .querySelector("button")!
    expect(retry.textContent).toContain("general:_trn_retry")
    retry.click()
    expect(control.refetch).toHaveBeenCalled()
    setRaw({
      curseforge: { 1: cf },
      modrinth: {},
      curseforgeError: null,
      modrinthError: null
    })
    await flush()
    expect(root.textContent).not.toContain("CF offline")
    expect(get().categories).toEqual([1])
  }
)
