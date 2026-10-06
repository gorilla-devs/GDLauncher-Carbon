import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal, For } from "solid-js"
import { createStore } from "solid-js/store"
import { render } from "solid-js/web"
import { BaseTile } from "./BaseTile"
import FolderTile from "./Library/FolderTile"
import Mod from "@/pages/Library/Instance/Tabs/Mods/Mod"
import { createSelectColumn } from "@/pages/Library/shared/addons/components/columns/selectColumn"
import { SearchableCheckboxList } from "@/pages/Search/FilterSidebar/SearchableCheckboxList"
import SingleCheckBox from "@/managers/ModalsManager/modals/OnBoarding/SingleCheckBox"
import { pointerGesture } from "@/task33.fixture"

const c = vi.hoisted(() => ({
  selected: undefined as any,
  mutate: vi.fn(),
  drag: vi.fn()
}))
vi.mock("@gd/ui", async () => {
  const fixture = await import("@/task33.fixture")
  return {
    ...fixture,
    PRESS_CLASSES_LIGHT: "",
    ContextMenu: fixture.Box,
    ContextMenuContent: fixture.Box,
    ContextMenuGroup: fixture.Box,
    ContextMenuGroupLabel: fixture.Box,
    ContextMenuItem: fixture.Box,
    ContextMenuSeparator: fixture.Box,
    ContextMenuTrigger: fixture.Box,
    Popover: fixture.Box,
    PopoverContent: fixture.Box,
    PopoverTrigger: fixture.Box,
    Switch: () => <span />
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@gd/i18n/helpers", () => ({ getTaskTranslationKey: (k: string) => k }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    instanceGroups: { data: [{ id: 4, name: "Synthetic folder" }] },
    serverGroups: { data: [] },
    instances: { data: [] },
    servers: { data: [] }
  })
}))
vi.mock("@/pages/Library/DragContext", () => ({
  useDragContext: () => ({
    dropTarget: () => undefined,
    dragType: () => undefined,
    isDragging: () => false,
    dragDetached: () => false,
    draggedIds: () => [],
    justDropped: () => false,
    unregisterDropZone: vi.fn(),
    registerDropZone: vi.fn(),
    startDrag: c.drag
  })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => ({ id: "7" }),
  useLocation: () => ({ pathname: "/library/7/mods" })
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { setQueryData: vi.fn() },
  rspc: {
    createQuery: () => ({ data: undefined }),
    createMutation: () => ({ mutate: c.mutate, isPending: false })
  }
}))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "" }))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: () => "",
  getServerImageUrl: () => "",
  getModImageUrl: () => "",
  getModpackPlatformIcon: () => ""
}))
vi.mock("@/components/CopyIcon", () => ({ default: () => null }))
vi.mock("@/pages/Library/components/DropOverlayIndicator", () => ({
  DropOverlayIndicator: () => null
}))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => <For each={p.data}>{p.children}</For>
}))
vi.mock("@/managers/ModalsManager/modals/OnBoarding/SingleEntity", () => ({
  instances: () => c.selected()
}))

let dispose: undefined | (() => void)
beforeEach(() => vi.clearAllMocks())
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
function input(root: HTMLElement) {
  return root.querySelector<HTMLInputElement>('input[type="checkbox"]')!
}
function enter(element: HTMLElement) {
  element.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Enter",
      bubbles: true,
      cancelable: true
    })
  )
}

it("0110 actual BaseTile selection is single-owned and does not activate row/play/drag", () => {
  const [selected, setSelected] = createSignal(false),
    row = vi.fn(),
    play = vi.fn(),
    drag = vi.fn()
  const toggle = vi.fn(() => setSelected((v) => !v))
  const root = mount(() => (
    <BaseTile
      name="Synthetic instance"
      size={1}
      isLoading={false}
      isWaiting={false}
      isRunning={false}
      isBusy={false}
      isDeleting={false}
      isMultiSelected={selected()}
      showCheckbox
      onToggleSelection={toggle}
      isDragging={false}
      isDragActive={false}
      canDrag
      onClick={row}
      onPlay={play}
      onDragStart={drag}
      shouldSetViewTransition={false}
      viewTransitionPrefix="synthetic"
      playButtonContent="Play"
      infoContent="Info"
    />
  ))
  pointerGesture(input(root).closest("label")!)
  expect(toggle).toHaveBeenCalledTimes(1)
  expect(selected()).toBe(true)
  enter(input(root))
  expect(toggle).toHaveBeenCalledTimes(2)
  expect(selected()).toBe(false)
  expect(row).not.toHaveBeenCalled()
  expect(play).not.toHaveBeenCalled()
  expect(drag).not.toHaveBeenCalled()
  root.firstElementChild!.dispatchEvent(
    new MouseEvent("click", { bubbles: true })
  )
  expect(row).toHaveBeenCalledTimes(1)
})
it("0110 actual FolderTile checkbox does not open folder or begin drag", () => {
  const [selected, setSelected] = createSignal(false),
    open = vi.fn()
  const toggle = vi.fn(() => setSelected((v) => !v))
  const root = mount(() => (
    <FolderTile
      groupId={4}
      kind="instance"
      isOpen={false}
      size={1}
      onToggle={open}
      isSelected={selected()}
      onToggleSelection={toggle}
    />
  ))
  pointerGesture(input(root).closest("label")!)
  expect(toggle).toHaveBeenCalledTimes(1)
  expect(selected()).toBe(true)
  enter(input(root))
  expect(toggle).toHaveBeenCalledTimes(2)
  expect(selected()).toBe(false)
  expect(open).not.toHaveBeenCalled()
  expect(c.drag).not.toHaveBeenCalled()
  root.querySelector<HTMLElement>("[data-folder-tile]")!.click()
  expect(open).toHaveBeenCalledTimes(1)
})
it("0110 actual legacy mod row click and checkbox boolean intent each toggle once", () => {
  const [selected, setSelected] = createStore<Record<string, boolean>>({})
  const root = mount(() => (
    <Mod
      mod={
        {
          id: "mod-A",
          filename: "example.jar",
          enabled: true,
          metadata: null,
          curseforge: null,
          modrinth: null,
          has_update: false
        } as any
      }
      selectMods={selected}
      setSelectedMods={setSelected}
      isInstanceLocked={false}
    />
  ))
  input(root).click()
  expect(selected["mod-A"]).toBe(true)
  expect(input(root).checked).toBe(true)
  enter(input(root))
  expect(selected["mod-A"]).toBeUndefined()
  root.firstElementChild!.dispatchEvent(
    new MouseEvent("click", { bubbles: true })
  )
  expect(selected["mod-A"]).toBe(true)
  expect(c.mutate).not.toHaveBeenCalled()
})
it("0110 actual table header indeterminate and cell selection preserve one callback/disabled row", () => {
  const [selected, setSelected] = createSignal(false),
    [allowed, setAllowed] = createSignal(true),
    [count, setCount] = createSignal(1)
  const all = vi.fn(() => setCount(2)),
    rowClick = vi.fn(),
    toggle = vi.fn((value: boolean) => setSelected(value))
  const column = createSelectColumn({
    selectedCount: count,
    totalRows: () => 2,
    onSelectAll: all
  }) as any
  const root = mount(() => (
    <>
      <div>{column.header()}</div>
      <div onClick={rowClick}>
        {column.cell({
          row: {
            id: "row-A",
            original: { filename: "example.jar" },
            getIsSelected: selected,
            getCanSelect: allowed,
            getIsSomeSelected: () => false,
            toggleSelected: toggle
          }
        })}
      </div>
    </>
  ))
  const inputs = root.querySelectorAll<HTMLInputElement>(
    'input[type="checkbox"]'
  )
  expect(inputs[0].indeterminate).toBe(true)
  inputs[0].click()
  expect(all).toHaveBeenCalledTimes(1)
  expect(inputs[0].indeterminate).toBe(false)
  inputs[1].click()
  expect(toggle).toHaveBeenCalledExactlyOnceWith(true)
  expect(rowClick).not.toHaveBeenCalled()
  setAllowed(false)
  inputs[1].click()
  expect(toggle).toHaveBeenCalledTimes(1)
})
it("0110 actual search filter late item replacement uses current label and boolean target", () => {
  const [items, setItems] = createSignal([{ label: "Old", value: "old" }]),
    [selected, setSelected] = createSignal<string[]>([])
  const toggle = vi.fn((id: string | number, checked: boolean) =>
    setSelected(checked ? [String(id)] : [])
  )
  const root = mount(() => (
    <SearchableCheckboxList
      items={items()}
      selectedValues={selected}
      onToggle={toggle}
      showSearch={false}
    />
  ))
  setItems([{ label: "Current", value: "current" }])
  enter(input(root))
  expect(toggle).toHaveBeenCalledExactlyOnceWith("current", true)
  input(root).closest("label")!.click()
  expect(toggle).toHaveBeenLastCalledWith("current", false)
  expect(toggle).toHaveBeenCalledTimes(2)
})
it("0110 actual Task32 import row preserves native checked/click and row ownership", () => {
  const [selected, setSelected] = createSignal<string[]>([])
  c.selected = selected
  const root = mount(() => (
    <SingleCheckBox
      title="Import current"
      filename="Folder"
      selectionId="selection-A"
      setList={setSelected}
    />
  ))
  input(root).click()
  expect(selected()).toEqual(["selection-A"])
  expect(input(root).checked).toBe(true)
  input(root).closest("label")!.click()
  expect(selected()).toEqual([])
  root.firstElementChild!.dispatchEvent(
    new MouseEvent("click", { bubbles: true })
  )
  expect(selected()).toEqual(["selection-A"])
})
it("0110 invalid import row still cannot create a checkbox or accept row selection", () => {
  const [selected, setSelected] = createSignal<string[]>([])
  c.selected = selected
  const root = mount(() => (
    <SingleCheckBox
      title="Invalid"
      invalidReason="Synthetic invalid"
      selectionId="bad"
      setList={setSelected}
    />
  ))
  expect(root.querySelector('input[type="checkbox"]')).toBeNull()
  root.firstElementChild!.dispatchEvent(
    new MouseEvent("click", { bubbles: true })
  )
  expect(selected()).toEqual([])
})
