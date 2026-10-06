import { afterEach, expect, it, vi } from "vitest"
import { createSignal, For } from "solid-js"
import { render } from "solid-js/web"
import { Radio } from "../../../../../../../../packages/ui/src/Radio"
import { Collapsable } from "../../../../../../../../packages/ui/src/Collapsable"
import { SearchableCheckboxList } from "./SearchableCheckboxList"
import { GameVersionsFilter } from "./GameVersionsFilter"
import { FilterSidebar } from "./index"
import Language from "../../Settings/Language"

const c = vi.hoisted(() => ({
  search: undefined as any,
  global: undefined as any,
  changeLanguage: vi.fn(async () => {}),
  settings: { language: "en" }
}))
// jsdom lacks native inert reflection. Model only its boolean DOM property;
// focus eligibility below is synthetic, not browser navigation proof.
if (!("inert" in HTMLElement.prototype))
  Object.defineProperty(HTMLElement.prototype, "inert", {
    configurable: true,
    get() {
      return this.hasAttribute("inert")
    },
    set(value: boolean) {
      this.toggleAttribute("inert", value)
    }
  })
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../../packages/ui/src/Radio")),
  ...(await import("../../../../../../../../packages/ui/src/Collapsable")),
  ...(await import("../../../../../../../../packages/ui/src/Checkbox")),
  Button: (p: any) => <button onClick={p.onClick}>{p.children}</button>
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key],
  supportedLanguages: { en: "england", de: "germany" }
}))
vi.mock("@gd/i18n/helpers", () => ({
  getLanguageKey: (key: string) => key,
  getLanguageNativeKey: (key: string) => key
}))
vi.mock("@/utils/language", () => ({ default: c.changeLanguage }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: { createQuery: () => ({ data: c.settings }) }
}))
vi.mock("@/components/SearchInputContext", () => ({ default: () => c.search }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.global
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "",
  getServerImageUrl: () => ""
}))
vi.mock("./CategoriesFilter", () => ({ CategoriesFilter: () => null }))
vi.mock("./ModloadersFilter", () => ({ ModloadersFilter: () => null }))
vi.mock("./CollapsedSidebar", () => ({
  CollapsedSidebar: () => <button>collapsed</button>
}))
vi.mock("../ViewModeToggle", () => ({ ViewModeToggle: () => null }))
vi.mock("./shared", () => ({ FilterWarning: () => null }))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => (
    <div data-virtual>
      <For each={p.data}>{p.children}</For>
    </div>
  )
}))
let dispose: () => void
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.changeLanguage.mockClear()
  vi.useRealTimers()
})
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
function searchFixture() {
  const [query, setQuery] = createSignal<any>({
    searchApi: null,
    environment: null,
    gameVersions: null
  })
  const [docked, setDocked] = createSignal(true),
    [ready, setReady] = createSignal(true),
    [versions, setVersions] = createSignal([{ id: "1.20.1" }, { id: "1.20.2" }])
  c.search = {
    searchQuery: query,
    setSearchQuery: setQuery,
    sidebarExpanded: docked,
    setSidebarExpanded: setDocked,
    sidebarReady: ready,
    selectedInstanceId: () => undefined,
    selectedServerId: () => undefined
  }
  c.global = {
    settings: { data: { reducedMotion: true } },
    minecraftVersions: {
      get data() {
        return versions()
      }
    }
  }
  return { query, setQuery, setDocked, setReady, setVersions }
}
function pointer(element: HTMLElement) {
  let captured = false
  Object.assign(element, {
    setPointerCapture: () => (captured = true),
    hasPointerCapture: () => captured,
    releasePointerCapture: () => (captured = false)
  })
  for (const type of ["pointerdown", "pointerup"]) {
    const e = new Event(type, { bubbles: true })
    Object.assign(e, { pointerId: 1 })
    element.dispatchEvent(e)
  }
  element.dispatchEvent(new MouseEvent("click", { bubbles: true }))
}
it("GPT-0127 native radios are focusable with scoped names and exactly one native/pointer callback", () => {
  const calls = vi.fn()
  const [value, setValue] = createSignal<any>("a")
  const root = mount(() => (
    <>
      <Radio.group
        value={value()}
        onChange={(v) => {
          calls(v)
          setValue(v)
        }}
        options={[
          { value: "a", label: "A" },
          { value: "b", label: "B" }
        ]}
      />
      <Radio.group options={[{ value: "a", label: "other" }]} />
    </>
  ))
  const inputs = Array.from(root.querySelectorAll<HTMLInputElement>("input"))
  expect(inputs[0].classList.contains("hidden")).toBe(false)
  expect(inputs[0].tabIndex).toBe(0)
  expect(inputs[0].name).toBeTruthy()
  expect(inputs[1].name).toBe(inputs[0].name)
  expect(inputs[2].name).not.toBe(inputs[0].name)
  inputs[1].click()
  expect(calls).toHaveBeenCalledExactlyOnceWith("b")
  calls.mockClear()
  root.querySelectorAll<HTMLLabelElement>("label")[1].click()
  expect(calls).not.toHaveBeenCalled()
  root.querySelectorAll<HTMLLabelElement>("label")[0].click()
  expect(calls).toHaveBeenCalledExactlyOnceWith("a")
})
it("GPT-0127 allowDeselect preserves selected pointer/Space and button style activation", () => {
  const calls = vi.fn()
  const root = mount(() => (
    <>
      <Radio value="a" checked allowDeselect onChange={calls}>
        A
      </Radio>
      <Radio value="b" checked={false} buttonStyle="button" onChange={calls}>
        B
      </Radio>
    </>
  ))
  const input = root.querySelector("input")!
  root.querySelector("label")!.click()
  expect(calls).toHaveBeenCalledExactlyOnceWith("a")
  calls.mockClear()
  input.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }))
  expect(calls).toHaveBeenCalledExactlyOnceWith("a")
  calls.mockClear()
  root.querySelector("button")!.click()
  expect(calls).toHaveBeenCalledExactlyOnceWith("b")
})
it("GPT-0127 native Language activation reaches its real changeLanguage handler", () => {
  const root = mount(() => <Language />)
  root.querySelectorAll<HTMLInputElement>("input")[1].click()
  expect(c.changeLanguage).toHaveBeenCalledExactlyOnceWith("de")
})
it("GPT-0127 inactive sidebar copies and collapsed descendants are inert", () => {
  const { query, setDocked, setReady } = searchFixture()
  const root = mount(() => <FilterSidebar />)
  const eligible = () =>
    Array.from(
      root.querySelectorAll<HTMLInputElement>("input[type=radio]")
    ).filter((i) => !i.closest("[inert]"))
  expect(eligible()).toHaveLength(4)
  const platform = eligible()[0]
  expect(platform.name).toBe(eligible()[1].name)
  expect(platform.name).not.toBe(eligible()[2].name)
  platform.click()
  expect(query().searchApi).toBe("curseforge")
  setDocked(false)
  expect(eligible()).toHaveLength(0)
  const strip = root.firstElementChild!.firstElementChild!
  strip.dispatchEvent(new MouseEvent("mouseenter", { bubbles: false }))
  expect(eligible().length).toBeGreaterThan(0)
  setReady(false)
  expect(eligible()).toHaveLength(0)
})
it("GPT-0127 Collapsable excludes closed descendants and restores open content", () => {
  let toggle!: () => void
  const root = mount(() => (
    <Collapsable
      defaultOpened={false}
      customHeader={(t) => {
        toggle = t
        return <span />
      }}
    >
      <Radio value="a" checked={false}>
        A
      </Radio>
    </Collapsable>
  ))
  expect(root.querySelector("input")!.closest("[inert]")).not.toBeNull()
  toggle()
  expect(root.querySelector("input")!.closest("[inert]")).toBeNull()
})
it("GPT-0128 direct nonempty immutable replacement follows current labels/icons/checks and toggle IDs", () => {
  const [items, setItems] = createSignal([
    { label: "1.20.1", value: "1.20.1", icon: <i>old</i> },
    { label: "1.20.2", value: "1.20.2" }
  ])
  const [selected, setSelected] = createSignal<string[]>([])
  const toggle = vi.fn()
  const root = mount(() => (
    <SearchableCheckboxList
      items={items()}
      selectedValues={selected}
      onToggle={toggle}
    />
  ))
  expect(root.textContent).toContain("1.20.1")
  setItems([
    { label: "1.21.1", value: "1.21.1", icon: <i>new</i> },
    { label: "1.21.2", value: "1.21.2" }
  ])
  setSelected(["1.21.1"])
  expect(root.textContent).toContain("1.21.1")
  expect(root.textContent).not.toContain("old")
  pointer(root.querySelector<HTMLElement>(".press-effect")!)
  expect(toggle).toHaveBeenCalledExactlyOnceWith("1.21.1", false)
  setItems([])
  expect(root.textContent).toContain("No results")
  setItems([{ label: "fresh", value: "fresh" }])
  expect(root.textContent).toContain("fresh")
})
it("GPT-0128 actual parent game-version handler emits current filtered ID after debounce", () => {
  vi.useFakeTimers()
  const { query, setVersions } = searchFixture()
  setVersions(Array.from({ length: 12 }, (_, i) => ({ id: `1.20.${i}` })))
  const root = mount(() => <GameVersionsFilter />)
  root.querySelector<HTMLElement>(".h-8")!.click()
  const input = root.querySelector("input")!
  input.value = "1.20.1"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  vi.advanceTimersByTime(150)
  setVersions(Array.from({ length: 12 }, (_, i) => ({ id: `1.21.${i}` })))
  input.value = "1.21.1"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  vi.advanceTimersByTime(150)
  pointer(root.querySelector<HTMLElement>(".press-effect.active\\:scale-95")!)
  expect(query().gameVersions).toEqual(["1.21.1"])
})
it("GPT-0128 direct nonempty parent manifest replacement emits the current version", () => {
  const { query, setVersions } = searchFixture()
  const root = mount(() => <GameVersionsFilter />)
  root.querySelector<HTMLElement>(".h-8")!.click()
  expect(
    root.querySelector(".press-effect.active\\:scale-95")!.closest("[inert]")
  ).toBeNull()
  setVersions([{ id: "1.21.1" }, { id: "1.21.2" }])
  expect(root.textContent).toContain("1.21.1")
  pointer(root.querySelector<HTMLElement>(".press-effect.active\\:scale-95")!)
  expect(query().gameVersions).toEqual(["1.21.1"])
})
it("native selected allowDeselect dispatches once with a controlled value", () => {
  const calls = vi.fn()
  const [value, setValue] = createSignal<string | undefined>("a")
  const root = mount(() => (
    <Radio
      name="one"
      value="a"
      checked={value() === "a"}
      allowDeselect
      onChange={() => {
        calls()
        setValue(value() ? undefined : "a")
      }}
    >
      A
    </Radio>
  ))
  const input = root.querySelector("input")!
  input.click()
  expect(calls).toHaveBeenCalledTimes(1)
  expect(value()).toBeUndefined()
  calls.mockClear()
  input.click()
  expect(calls).toHaveBeenCalledTimes(1)
  expect(value()).toBe("a")
})
it("unchanged virtual branch renders through its list adapter", () => {
  const root = mount(() => (
    <SearchableCheckboxList
      items={[
        { label: "one", value: "one" },
        { label: "two", value: "two" }
      ]}
      selectedValues={() => []}
      onToggle={() => {}}
      virtualizeThreshold={1}
    />
  ))
  expect(root.querySelector("[data-virtual]")?.textContent).toContain("two")
})
