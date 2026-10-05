import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { createSignal } from "solid-js"
import { createStore } from "solid-js/store"
import { render } from "solid-js/web"

const control = vi.hoisted(() => ({
  store: null as any,
  params: null as any,
  setParams: null as any,
  selection: null as any,
  instanceData: null as any,
  serverData: null as any,
  view: null as any,
  open: vi.fn(),
  cancel: vi.fn()
}))
vi.mock("@gd/ui", () => import("@/test/task25/ui"))
vi.mock("@gd/i18n", () => ({ Trans: (props: any) => <span>{props.key}</span> }))
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [
    control.params,
    (values: any) => control.setParams(values)
  ]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => control.store
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: control.open })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ lastPathVisited: () => ({ path: "/" }) })
}))
vi.mock("@/hooks/useDragSelect", () => ({
  useDragSelect: () => ({ selectionRect: () => null, handlers: {} })
}))
vi.mock("@/components/UnstableCard", () => ({ default: () => <div /> }))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => <div />
}))
vi.mock("@/components/DragGhost", () => ({ default: () => <div /> }))
vi.mock("@/components/InstanceTile", () => ({
  clickedInstanceId: () => undefined,
  setClickedInstanceId: vi.fn()
}))
vi.mock("@/components/Server/Tile", () => ({ setClickedServerId: vi.fn() }))
vi.mock("./DragContext", () => ({
  DragProvider: (props: any) => props.children,
  useDragContext: () =>
    new Proxy(
      { cancelDrag: control.cancel },
      { get: (obj: any, key) => obj[key] ?? (() => false) }
    )
}))
vi.mock("./components/FloatingFavoritesBar", () => ({
  FloatingFavoritesBar: () => <div />
}))
vi.mock("./components/LibraryHeader", () => ({ LibraryHeader: () => <div /> }))
vi.mock("./views/FoldersView", () => ({
  default: (props: any) => {
    control.view = props
    return <div />
  }
}))
vi.mock("./views/AccordionView", () => ({ default: () => <div /> }))
vi.mock("./hooks", () => ({
  useLibraryData: () => control.instanceData,
  useServerData: () => control.serverData,
  useLibrarySelection: () => control.selection,
  useFLIPAnimation: () => ({ isAnimating: () => false }),
  useEntranceAnimation: () => ({ reset: vi.fn() }),
  useLibraryDragDrop: () => ({
    handleDrop: vi.fn(),
    newlyCreatedFolderId: () => null,
    clearNewlyCreatedFolderId: vi.fn()
  })
}))
import HomeGrid from "./HomeGrid"
import { computeServerLibraryItems } from "./utils/serverGrouping"
import {
  parseServerIds,
  parseServerFolderIds,
  parseFolderIds
} from "./utils/selectionIds"
let dispose: () => void
let root: HTMLDivElement
let select: (ids: string[]) => void
beforeEach(() => {
  vi.clearAllMocks()
  vi.stubGlobal("requestAnimationFrame", (callback: any) => {
    callback()
    return 0
  })
  const [store] = createStore({
    settings: { data: { reducedMotion: true } },
    instanceGroups: { data: [{ id: 2, name: "Survival" }] },
    serverGroups: {
      data: [{ id: 2, name: "Server hosting", libraryPosition: 0 }]
    },
    instances: { data: [{ id: 9, group_id: 1, name: "Client nine" }] },
    servers: {
      data: [
        {
          id: 7,
          groupId: 1,
          name: "Server seven",
          index: 0,
          libraryPosition: 0
        }
      ]
    }
  })
  const [params, setParams] = createStore({ mode: "servers" })
  const [selected, setSelected] = createSignal(new Set<string>())
  control.store = store
  control.params = params
  control.setParams = setParams
  select = (ids) => setSelected(new Set(ids))
  control.selection = {
    selectedIds: selected,
    isSelected: (id: string) => selected().has(id),
    clearSelection: () => setSelected(new Set<string>()),
    selectAll: select,
    toggleSelection: vi.fn()
  }
  const data = (items: any) => ({
    libraryItems: items,
    virtualGroups: [],
    favoriteIds: [],
    viewMode: () => "folders",
    isFoldersView: () => true,
    defaultGroupId: () => 1,
    isLoading: () => false,
    isEmpty: () => false
  })
  control.instanceData = data([
    {
      id: "folder-2",
      type: "folder",
      data: { id: 2, name: "Survival", kind: "instance", instances: [] }
    }
  ])
  control.serverData = data(
    computeServerLibraryItems(
      store.servers.data as any,
      store.serverGroups.data,
      "",
      1
    )
  )
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <HomeGrid />, root)
})
afterEach(() => {
  dispose()
  root.remove()
  vi.unstubAllGlobals()
})
function serverFolderId() {
  return control.serverData.libraryItems.find(
    (item: any) => item.type === "folder"
  ).id
}
test("server folder-only multiselection opens a server unlink confirmation", () => {
  select([serverFolderId()])
  control.view.onBatchDelete()
  expect(control.open).toHaveBeenCalledWith(
    { name: "confirmBatchFolderDeletion" },
    expect.objectContaining({
      kind: "server",
      folders: [expect.objectContaining({ id: 2, name: "Server hosting" })]
    })
  )
  expect(control.selection.selectedIds().size).toBe(1)
})
test("mixed server member/folder selection preserves both targets in a server-only confirmation", () => {
  select(["server-7", serverFolderId()])
  control.view.onBatchDelete()
  expect(control.open).toHaveBeenCalledWith(
    { name: "confirmBatchServerDeletion" },
    expect.objectContaining({
      servers: [expect.objectContaining({ id: 7 })],
      folders: [expect.objectContaining({ id: 2, name: "Server hosting" })]
    })
  )
  expect(
    control.open.mock.calls.some(
      ([modal]) => modal.name === "confirmBatchMixedDeletion"
    )
  ).toBe(false)
})
test.each([false, true])(
  "client folder selection preserves its %s mixed-selection branch",
  (mixed) => {
    control.setParams("mode", "instances")
    select(mixed ? ["folder-2", "instance-9"] : ["folder-2"])
    control.view.onBatchDelete()
    expect(control.open).toHaveBeenCalledWith(
      {
        name: mixed ? "confirmBatchMixedDeletion" : "confirmBatchFolderDeletion"
      },
      expect.objectContaining({
        folders: [expect.objectContaining({ id: 2, name: "Survival" })],
        ...(mixed
          ? { instances: [expect.objectContaining({ id: 9 })] }
          : { kind: "instance" })
      })
    )
  }
)
test("switching library namespace retires selection and an open folder", () => {
  select([serverFolderId()])
  control.view.setOpenFolderId(2)
  control.setParams("mode", "instances")
  expect(control.selection.selectedIds().size).toBe(0)
  expect(control.view.openFolderId()).toBeNull()
  expect(control.cancel).toHaveBeenCalled()
})

test("server member and folder parsers preserve separate targets with colliding numeric IDs", () => {
  const ids = [
    "server-2",
    "server-folder-2",
    "folder-2",
    "instance-2",
    "server-folder-x"
  ]
  expect(parseServerIds(ids)).toEqual([2])
  expect(parseServerFolderIds(ids)).toEqual([2])
  expect(parseFolderIds(ids)).toEqual([2])
})
