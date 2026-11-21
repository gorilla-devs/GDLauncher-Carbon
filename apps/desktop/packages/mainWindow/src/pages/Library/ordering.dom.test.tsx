import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
const c = vi.hoisted(() => ({
  store: null as any,
  drag: null as any,
  mutate: vi.fn()
}))
vi.mock("@gd/i18n", () => ({ useTransContext: () => [(k: string) => k] }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.store
}))
vi.mock("./DragContext", () => ({ useDragContext: () => c.drag }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({ data: 1 }),
    createMutation: (f: any) => ({
      mutate: (v: any) => c.mutate(f().mutationKey[0], v)
    })
  }
}))
import { useDropZoneRegistration } from "./hooks/useDropZoneRegistration"
import { useDropIndicators } from "./hooks/useDropIndicators"
import { useLibraryDragDrop } from "./hooks/useLibraryDragDrop"
import { useLibraryData } from "./hooks/useLibraryData"
import { computeServerLibraryItems } from "./utils/serverGrouping"
import DropPreviewTile from "./components/DropPreviewTile"
import LibraryItemTile from "@/components/Library/LibraryItemTile"
vi.mock("@/components/Library/FolderTile", () => ({ default: () => <div /> }))
vi.mock("@/components/InstanceTile", () => ({ default: () => <div /> }))
vi.mock("@/components/Server/Tile", () => ({ default: () => <div /> }))
vi.mock("@/pages/Library/components/DropOverlayIndicator", () => ({
  DropOverlayIndicator: () => <div />
}))
vi.mock("@/pages/Library/hooks", async () => ({
  ...(await import("./hooks/useDropIndicators")),
  ...(await import("./hooks/useDropZoneRegistration")),
  useLibraryItemAnimation: vi.fn()
}))
let root: HTMLDivElement, dispose: (() => void) | undefined, setStore: any
beforeEach(() => {
  vi.clearAllMocks()
  const [store, set] = createStore({
    settings: { data: { instancesGroupBy: null } },
    instances: {
      data: [{ id: 9, name: "A", group_id: 1, index: 90, library_position: 9 }]
    },
    instanceGroups: {
      data: [
        { id: 1, name: "default", library_position: 0 },
        { id: 5, name: "F", library_position: 5 }
      ]
    },
    servers: { data: [{ id: 9, name: "A", favorite: false }] }
  })
  c.store = store
  setStore = set
  root = document.createElement("div")
  document.body.append(root)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
})
function drag(kind: string) {
  const [state, set] = createStore({
    active: true,
    dropped: false,
    type: kind,
    target: null as any
  })
  const zones = new Map<string, any>()
  c.drag = {
    isDragging: () => state.active,
    justDropped: () => state.dropped,
    dragDetached: () => true,
    dragType: () => state.type,
    draggedIds: () => [9],
    dropTarget: () => state.target,
    registerDropZone: (z: any) => zones.set(z.id, z),
    unregisterDropZone: (id: string) => zones.delete(id),
    setDropPreviewRect: vi.fn()
  }
  return { zones, set }
}
function setup(kind: "instance" | "server") {
  const ctx = drag(kind)
  let indicator: any, handler: any, data: any
  const selection = { clearSelection: vi.fn() } as any
  dispose = render(() => {
    const el = document.createElement("div")
    el.getBoundingClientRect = () => new DOMRect(10, 20, 100, 120)
    useDropZoneRegistration({
      itemId: 5,
      itemType: "folder",
      itemKind: kind,
      ref: () => el,
      dragContext: c.drag,
      defaultGroupId: () => 1
    })
    indicator = useDropIndicators({
      itemId: 5,
      itemType: "folder",
      itemKind: kind,
      dragContext: c.drag
    })
    data = useLibraryData(() => "")
    handler = useLibraryDragDrop({
      defaultGroupId: () => 1,
      selection,
      flipAnimation: { capturePositions: vi.fn() } as any,
      libraryItems: []
    })
    return el
  }, root)
  return { ...ctx, indicator, handler, data, selection }
}
test.each(["instance", "server"] as const)(
  "%s omits unsupported item-before-folder registration with actual geometry",
  (kind) => {
    const { zones } = setup(kind)
    expect(
      [...zones.values()].some(
        (z) => z.target.type === "beforeInstanceAtFolder"
      )
    ).toBe(false)
  }
)
test.each(["instance", "server"] as const)(
  "%s suppresses stale before-folder indicators during drag and justDropped",
  (kind) => {
    const { set, indicator } = setup(kind)
    set("target", { type: "beforeInstanceAtFolder", folderId: 5 })
    expect(indicator.showDropIndicator()).toBe(false)
    expect(indicator.activeDropTarget()).toBeNull()
    set("active", false)
    set("dropped", true)
    expect(indicator.showDropIndicator()).toBe(false)
  }
)
test.each(["instance", "server"] as const)(
  "%s stale before-folder dispatch cannot submit a move",
  (kind) => {
    const { handler, selection } = setup(kind)
    handler.handleDrop(
      { type: "beforeInstanceAtFolder", folderId: 5 },
      [9, 10],
      kind,
      null
    )
    expect(c.mutate).not.toHaveBeenCalled()
    expect(selection.clearSelection).not.toHaveBeenCalled()
  }
)
test("preview refuses unsupported self-registration and cleans previous supported target on replacement/stop", () => {
  const { zones, set } = drag("instance")
  const [p, setP] = createStore({
    target: { type: "beforeInstance", instanceId: 9, groupId: 1 } as any
  })
  dispose = render(
    () => <DropPreviewTile tileSize={() => 2} dropTarget={p.target} />,
    root
  )
  expect(zones.size).toBe(1)
  setP("target", { type: "beforeInstanceAtFolder", folderId: 5 })
  expect(zones.size).toBe(0)
  expect(c.drag.setDropPreviewRect).toHaveBeenLastCalledWith(null)
  setP("target", { type: "beforeInstance", instanceId: 10, groupId: 1 })
  expect(zones.size).toBe(1)
  set("active", false)
  expect(zones.size).toBe(0)
  dispose()
  dispose = undefined
  expect(zones.size).toBe(0)
})
test.each(["instance", "server"] as const)(
  "%s preserves same-kind multiselect/member/default/favorite dispatch and folders-first projection",
  (kind) => {
    const { handler, data } = setup(kind)
    const key =
      kind === "instance" ? "instance.moveInstance" : "server.moveServer"
    const target =
      kind === "instance" ? { BeforeInstance: 10 } : { beforeServer: 10 }
    handler.handleDrop(
      { type: "beforeInstance", instanceId: 10, groupId: 1 },
      [9, 10, 11],
      kind,
      null
    )
    expect(c.mutate.mock.calls).toEqual([
      [key, { [kind]: 9, target }],
      [key, { [kind]: 11, target }]
    ])
    for (const type of [
      "dropOnFolder",
      "folderContentArea",
      "endOfGroup",
      "ungrouped"
    ]) {
      handler.handleDrop({ type, groupId: 5 } as any, [9], kind, null)
      expect(c.mutate).toHaveBeenLastCalledWith(key, {
        [kind]: 9,
        target:
          kind === "instance"
            ? { EndOfGroup: type === "ungrouped" ? 1 : 5 }
            : { endOfGroup: type === "ungrouped" ? 1 : 5 }
      })
    }
    handler.handleDrop({ type: "favorites" }, [9], kind, null)
    expect(c.mutate).toHaveBeenLastCalledWith(
      `${kind}.setFavorite`,
      kind === "instance"
        ? { instance: 9, favorite: true }
        : { id: 9, favorite: true }
    )
    handler.handleDrop(null, [9], kind, "favorites")
    expect(c.mutate).toHaveBeenLastCalledWith(
      `${kind}.setFavorite`,
      kind === "instance"
        ? { instance: 9, favorite: false }
        : { id: 9, favorite: false }
    )
    setStore("instances", "data", [
      { id: 9, name: "A", group_id: 1, index: 90, library_position: 5 },
      { id: 10, name: "B", group_id: 1, index: 2, library_position: 9 }
    ])
    setStore("instanceGroups", "data", 1, "library_position", 6)
    expect(data.libraryItems.map((x: any) => x.id)).toEqual([
      "folder-5",
      "instance-9",
      "instance-10"
    ])
    setStore("instances", "data", [
      { id: 9, name: "A", group_id: 1, index: 90, library_position: 9 },
      { id: 10, name: "B", group_id: 1, index: 2, library_position: 5 }
    ])
    expect(data.libraryItems.map((x: any) => x.id)).toEqual([
      "folder-5",
      "instance-10",
      "instance-9"
    ])
    const servers = [
      { id: 9, name: "A", groupId: 1, index: 90, libraryPosition: 9 },
      { id: 10, name: "B", groupId: 1, index: 2, libraryPosition: 5 }
    ]
    const groups = [
      { id: 1, name: "default", libraryPosition: 0 },
      { id: 5, name: "F", libraryPosition: 6 }
    ]
    expect(
      computeServerLibraryItems(servers as any, groups, "", 1).map((x) => x.id)
    ).toEqual(["server-folder-5", "server-10", "server-9"])
  }
)

test.each([
  "beforeInstance",
  "endOfGroup",
  "endOfGroups",
  "beforeGroupAtInstance"
])(
  "supported %s preview remains registered through its own lifetime",
  (type) => {
    const { zones, set } = drag("instance")
    const target = {
      type,
      instanceId: 10,
      groupId: 5,
      beforeInstanceId: 10
    } as any
    dispose = render(
      () => <DropPreviewTile tileSize={() => 2} dropTarget={target} />,
      root
    )
    expect(zones.size).toBe(1)
    expect([...zones.values()][0].target).toBe(target)
    set("active", false)
    expect(zones.size).toBe(0)
    expect(c.drag.setDropPreviewRect).toHaveBeenLastCalledWith(null)
  }
)

test.each(["instance", "server"] as const)(
  "actual %s folder tile never mounts a misleading stale preview",
  (kind) => {
    const { set } = drag(kind)
    dispose = render(
      () => (
        <LibraryItemTile
          {...({
            item: {
              id: `${kind}-folder-5`,
              type: "folder",
              data: {
                id: 5,
                kind,
                name: "F",
                instances: [],
                libraryPosition: 5
              }
            },
            itemIndex: () => 0,
            tileSize: () => 2,
            defaultGroupId: () => 1,
            openFolderId: () => null,
            toggleFolder: vi.fn(),
            isSelected: () => false,
            toggleSelection: vi.fn(),
            selectedIds: () => new Set(),
            onDragStart: vi.fn(),
            justDropped: () => false,
            tileRefs: new Map(),
            libraryItemRefs: new Map(),
            animatedLibraryItemIds: new Set(),
            libraryInitialAnimationComplete: { value: true },
            libraryItemsLength: () => 1
          } as any)}
        />
      ),
      root
    )
    set("target", { type: "beforeInstanceAtFolder", folderId: 5 })
    expect(root.querySelector("[data-drop-preview]")).toBeNull()
  }
)
