import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createRoot, createSignal } from "solid-js"
import { createStore } from "solid-js/store"

const control = vi.hoisted(() => ({
  store: null as any,
  open: vi.fn(),
  mutation: vi.fn(),
  close: vi.fn(),
  navigate: vi.fn(),
  drag: vi.fn(),
  header: null as any,
  error: vi.fn(),
  dragState: null as any,
  buttonActions: new Map<any, () => unknown>()
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/task25/ui")
  return {
    ...ui,
    Button: (props: any) => {
      const element = ui.Button(props)
      control.buttonActions.set(element, props.onClick)
      return element
    },
    toast: { error: control.error }
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => (
    <span>
      {props.key}
      {JSON.stringify(props.options ?? {})}
    </span>
  ),
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => control.store
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({
    openModal: control.open,
    closeModal: control.close,
    hasOpenModals: () => false
  })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: control.navigate })
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { cancelQueries: vi.fn() },
  rspc: {
    createQuery: () => ({ data: 1 }),
    createMutation: (opts: any) => ({
      mutate: (value: any) => control.mutation(opts().mutationKey[0], value),
      mutateAsync: (value: any) =>
        control.mutation(opts().mutationKey[0], value)
    })
  }
}))
vi.mock("@/pages/Library/DragContext", () => ({
  useDragContext: () =>
    new Proxy(
      { startDrag: control.drag },
      {
        get: (obj: any, key) =>
          control.dragState?.[key] ??
          obj[key] ??
          (() => (key === "draggedIds" ? [] : false))
      }
    )
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: (id: number, rev: number) =>
    `/instance-icon/${id}/${rev}`,
  getServerImageUrl: (id: number, rev: number) => `/server-icon/${id}/${rev}`
}))
vi.mock("@/components/Instance/SelectionBorder", () => ({
  default: () => <div />
}))
vi.mock("@/components/InstanceTile", () => ({ default: () => <div /> }))
vi.mock("@/components/Server/Tile", () => ({ default: () => <div /> }))
vi.mock("@/pages/Library/components/DropPreviewTile", () => ({
  default: () => <div />
}))
vi.mock("@/pages/Library/components/DropOverlayIndicator", () => ({
  DropOverlayIndicator: () => <div />
}))
vi.mock("@/pages/Library/components/EndOfGroupDropZone", () => ({
  EndOfGroupDropZone: () => <div />,
  EndOfGroupsDropZone: () => <div />
}))
vi.mock("@/pages/Library/hooks/useDragLayoutAnimation", () => ({
  useDragLayoutAnimation: vi.fn()
}))
vi.mock("@/pages/Library/hooks", () => ({
  useDropIndicators: () => ({
    showDropIndicator: () => false,
    shouldCollapseTile: () => false,
    showCreateFolderIndicator: () => false
  }),
  useDropZoneRegistration: vi.fn(),
  useLibraryItemAnimation: vi.fn()
}))
vi.mock("@/hooks/useDragSelect", () => ({
  useDragSelect: () => ({
    selectionRect: () => null,
    handlers: { handleMouseDown: vi.fn() }
  })
}))
vi.mock("@/utils/sidebar", () => ({
  getModloaderIcon: () => "local-modloader"
}))
vi.mock("./ExpandedFolderContent/FolderHeader", async (importOriginal) => {
  const original = await importOriginal<any>()
  return {
    FolderHeader: (props: any) => {
      control.header = props
      return <original.FolderHeader {...props} />
    }
  }
})
vi.mock("@/utils/adhelper", () => ({ default: { width: 0 } }))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (props: any) => <div>{props.children}</div>
}))

import FolderTile from "./FolderTile"
import ExpandedFolderContent, {
  openFolderCreation
} from "./ExpandedFolderContent"
import DragGhost from "@/components/DragGhost"
import ConfirmBatchFolderDeletion, {
  deleteFolderTargets
} from "@/managers/ModalsManager/modals/ConfirmBatchFolderDeletion"
import ConfirmBatchServerDeletion from "@/managers/ModalsManager/modals/ConfirmBatchServerDeletion"
import { LibraryGrid } from "@/pages/Library/views/FoldersView/LibraryGrid"
import { useLibraryData } from "@/pages/Library/hooks/useLibraryData"
import { computeServerLibraryItems } from "@/pages/Library/utils/serverGrouping"

let root: HTMLDivElement
let overlay: HTMLDivElement
let dispose: () => void
const client = {
  id: 9,
  group_id: 2,
  name: "Client nine",
  icon_revision: 3,
  index: 0,
  library_position: 0,
  favorite: false,
  status: { status: "valid", value: { state: { state: "inactive" } } }
}
const servers = [
  {
    id: 7,
    groupId: 2,
    name: "Server seven",
    iconRevision: 4,
    index: 0,
    libraryPosition: 0
  },
  {
    id: 8,
    groupId: 2,
    name: "Server eight",
    iconRevision: 5,
    index: 1,
    libraryPosition: 1
  }
]
beforeEach(() => {
  vi.clearAllMocks()
  control.buttonActions.clear()
  control.header = null
  control.dragState = null
  control.mutation.mockResolvedValue(undefined)
  const [store] = createStore({
    instanceGroups: {
      data: [{ id: 2, name: "Survival", library_position: 2 }]
    },
    serverGroups: {
      data: [{ id: 2, name: "Server hosting", libraryPosition: 2 }]
    },
    instances: { data: [client], isLoading: false },
    servers: { data: servers },
    settings: { data: { reducedMotion: true, instancesGroupBy: null } }
  })
  control.store = store
  root = document.createElement("div")
  document.body.append(root)
  overlay = document.createElement("div")
  overlay.id = "overlay"
  document.body.append(overlay)
})
afterEach(() => {
  dispose?.()
  root.remove()
  overlay.remove()
})
function button(key: string, container = root) {
  return [...container.querySelectorAll("button")].find((item) =>
    item.textContent?.includes(key)
  )!
}
function tile(kind: "instance" | "server") {
  dispose = render(
    () => (
      <FolderTile
        {...({
          groupId: 2,
          kind,
          isOpen: false,
          onToggle: vi.fn(),
          size: 2
        } as any)}
      />
    ),
    root
  )
}

test("equal-ID instance folder tile renders client name, client preview and instance-group drag", () => {
  tile("instance")
  expect(root.querySelector("h4")?.textContent).toBe("Survival")
  expect(root.querySelector("img")?.getAttribute("src")).toBe(
    "/instance-icon/9/3"
  )
  root
    .querySelector("[data-folder-tile]")!
    .dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, button: 0 }))
  expect(control.drag.mock.calls[0][0]).toBe("group")
})
test("equal-ID server folder tile renders server preview, carries its namespace and withholds delete-all", () => {
  tile("server")
  expect(root.querySelector("h4")?.textContent).toBe("Server hosting")
  expect(root.querySelector("img")?.getAttribute("src")).toBe(
    "/server-icon/7/4"
  )
  expect(button("_trn_delete_folder_all")).toBeUndefined()
  button("_trn_unlink_folder").click()
  expect(control.open).toHaveBeenCalledWith(
    { name: "confirmBatchFolderDeletion" },
    expect.objectContaining({
      kind: "server",
      folders: [expect.objectContaining({ id: 2, name: "Server hosting" })]
    })
  )
})
test("server unlink confirmation dispatches only server namespace with naturally colliding IDs", async () => {
  dispose = render(
    () => (
      <ConfirmBatchFolderDeletion
        {...({
          data: { kind: "server", folders: control.store.serverGroups.data }
        } as any)}
      />
    ),
    root
  )
  button("_trn_unlink_folders_button").click()
  await Promise.resolve()
  await Promise.resolve()
  expect(control.mutation).toHaveBeenCalledExactlyOnceWith(
    "server.deleteServerGroup",
    2
  )
  expect(control.navigate).toHaveBeenCalledWith("/library?mode=servers")
  expect(control.store.instances.data[0]).toMatchObject(client)
})
test("server confirmation counts current full server membership and exposes no content-deletion choice", () => {
  dispose = render(
    () => (
      <ConfirmBatchFolderDeletion
        {...({
          data: { kind: "server", folders: control.store.serverGroups.data }
        } as any)}
      />
    ),
    root
  )
  expect(root.textContent).toContain('"count":2')
  expect(button("_trn_delete_folders_with_instances_button")).toBeUndefined()
})
test.each(["unlink", "deleteAll"])(
  "client confirmation preserves %s endpoint",
  async (mode) => {
    dispose = render(
      () => (
        <ConfirmBatchFolderDeletion
          {...({
            data: {
              kind: "instance",
              folders: control.store.instanceGroups.data
            }
          } as any)}
        />
      ),
      root
    )
    button(
      mode === "unlink"
        ? "_trn_unlink_folders_button"
        : "_trn_delete_folders_with_instances_button"
    ).click()
    await Promise.resolve()
    await Promise.resolve()
    expect(control.mutation).toHaveBeenCalledExactlyOnceWith(
      mode === "unlink"
        ? "instance.deleteGroup"
        : "instance.deleteGroupWithInstances",
      2
    )
  }
)
test("both actual producers carry homogeneous folder identity through the grid to each tile", () => {
  let end!: () => void
  const data = createRoot((stop) => {
    end = stop
    return useLibraryData(() => "")
  })
  const item = data.libraryItems[0]
  const serverItems = computeServerLibraryItems(
    servers as any,
    control.store.serverGroups.data,
    "",
    1
  )
  expect(item.type === "folder" && (item.data as any).kind).toBe("instance")
  expect(
    serverItems[0].type === "folder" && (serverItems[0].data as any).kind
  ).toBe("server")
  const selection = {
    selectedIds: () => new Set<string>(),
    isSelected: () => false,
    toggleSelection: vi.fn()
  }
  dispose = render(
    () => (
      <LibraryGrid
        {...({
          libraryItems: [item, serverItems[0]],
          tileSize: () => 2,
          defaultGroupId: 1,
          openFolderId: () => null,
          onToggleFolder: vi.fn(),
          selection,
          onDragStart: vi.fn(),
          justDropped: () => false,
          flipAnimation: {},
          entranceAnimation: { animatedIds: new Set(), initialComplete: true },
          tileRefs: new Map(),
          newlyCreatedFolderId: () => null,
          clearNewlyCreatedFolderId: vi.fn()
        } as any)}
      />
    ),
    root
  )
  expect(
    [...root.querySelectorAll("h4")].map((heading) => heading.textContent)
  ).toEqual(["Survival", "Server hosting"])
  end()
})
test.each(["instance", "server"] as const)(
  "expanded %s folder renames in its namespace and only clients expose arrange",
  (kind) => {
    dispose = render(
      () => (
        <ExpandedFolderContent
          {...({
            group: {
              id: 2,
              name: kind === "server" ? "Server hosting" : "Survival",
              kind,
              instances: []
            },
            isServerMode: kind === "server",
            onClose: vi.fn(),
            tileSize: 2,
            isDefaultGroup: false,
            selectedIds: new Set(),
            onToggleSelection: vi.fn(),
            onSetSelection: vi.fn(),
            onDragStart: vi.fn()
          } as any)}
        />
      ),
      root
    )
    button("", overlay) // Keep the mounted real header available below.
    const rename = overlay.querySelector<HTMLButtonElement>(
      'button[title="instances:_trn_rename_group"]'
    )!
    rename.click()
    const input = overlay.querySelector("input")!
    input.value = "Renamed folder"
    input.dispatchEvent(new Event("input", { bubbles: true }))
    input.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
    )
    expect(control.mutation).toHaveBeenCalledWith(
      kind === "server" ? "server.renameServerGroup" : "instance.renameGroup",
      { group: 2, name: "Renamed folder" }
    )
    expect(
      !!overlay.querySelector('button[title="instances:_trn_rearrange"]')
    ).toBe(kind === "instance")
    if (kind === "instance") {
      button("ui:_trn_by_name", overlay).click()
      expect(control.mutation).toHaveBeenCalledWith("instance.arrangeGroup", {
        group: 2,
        sortBy: "name"
      })
    }
  }
)

test("D10 mixed partial retry acknowledges member successes separately from equal-ID folders", async () => {
  const completedServers = new Set<number>()
  let rejectSecond = true
  control.mutation.mockImplementation(async (key: string, id: number) => {
    if (key === "server.deleteServer") {
      if (completedServers.has(id)) throw new Error("Server not found")
      if (id === 8 && rejectSecond) throw new Error("Busy server")
      completedServers.add(id)
    }
  })
  const done = vi.fn()
  dispose = render(
    () => (
      <ConfirmBatchServerDeletion
        {...({
          data: {
            servers,
            folders: [{ id: 7, name: "Equal-ID folder" }],
            onComplete: done
          }
        } as any)}
      />
    ),
    root
  )
  button("_trn_delete_servers_button").click()
  await vi.waitFor(() =>
    expect(root.querySelector('[role="alert"]')).not.toBeNull()
  )
  expect(control.mutation.mock.calls).toEqual([
    ["server.deleteServer", 7],
    ["server.deleteServer", 8]
  ])
  expect(control.close).not.toHaveBeenCalled()
  expect(done).not.toHaveBeenCalled()
  rejectSecond = false
  button("_trn_delete_servers_button").click()
  await vi.waitFor(() => expect(control.close).toHaveBeenCalledTimes(1))
  expect(control.mutation.mock.calls).toEqual([
    ["server.deleteServer", 7],
    ["server.deleteServer", 8],
    ["server.deleteServer", 8],
    ["server.deleteServerGroup", 7]
  ])
  expect(done).toHaveBeenCalledTimes(1)
  expect(control.navigate).toHaveBeenCalledWith("/library?mode=servers")
})

test.each([
  ["server", "unlink"],
  ["instance", "unlink"],
  ["instance", "deleteAll"]
] as const)(
  "D10 %s folder %s retry processes only pending folders",
  async (kind, mode) => {
    const completed = new Set<number>()
    let rejectSecond = true
    const endpoint =
      kind === "server"
        ? "server.deleteServerGroup"
        : mode === "unlink"
          ? "instance.deleteGroup"
          : "instance.deleteGroupWithInstances"
    control.mutation.mockImplementation(async (_key: string, id: number) => {
      if (completed.has(id)) throw new Error("Folder not found")
      if (id === 3 && rejectSecond) throw new Error("Busy folder")
      completed.add(id)
    })
    dispose = render(
      () => (
        <ConfirmBatchFolderDeletion
          {...({
            data: {
              kind,
              folders: [
                { id: 2, name: "First" },
                { id: 3, name: "Second" }
              ]
            }
          } as any)}
        />
      ),
      root
    )
    const key =
      mode === "unlink"
        ? "_trn_unlink_folders_button"
        : "_trn_delete_folders_with_instances_button"
    button(key).click()
    await vi.waitFor(() =>
      expect(root.querySelector('[role="alert"]')).not.toBeNull()
    )
    expect(control.close).not.toHaveBeenCalled()
    rejectSecond = false
    button(key).click()
    await vi.waitFor(() => expect(control.close).toHaveBeenCalledTimes(1))
    expect(control.mutation.mock.calls).toEqual([
      [endpoint, 2],
      [endpoint, 3],
      [endpoint, 3]
    ])
  }
)

test("unsupported server delete-all and absent namespace are rejected before any direct mutation", async () => {
  const actions = {
    unlinkInstance: vi.fn(),
    deleteInstanceContents: vi.fn(),
    unlinkServer: vi.fn()
  }
  await expect(
    deleteFolderTargets("server", "deleteAll", [{ id: 2 }], actions)
  ).rejects.toThrow("not supported")
  await expect(
    deleteFolderTargets(undefined, "unlink", [{ id: 2 }], actions)
  ).rejects.toThrow("kind is required")
  for (const action of Object.values(actions))
    expect(action).not.toHaveBeenCalled()
})

test("server expanded direct arrange and import handlers refuse instance-only operations", () => {
  dispose = render(
    () => (
      <ExpandedFolderContent
        {...({
          group: { id: 2, kind: "server", name: "Hosting", instances: [] },
          onClose: vi.fn(),
          tileSize: 2,
          isDefaultGroup: false,
          selectedIds: new Set(),
          onSetSelection: vi.fn()
        } as any)}
      />
    ),
    root
  )
  control.header.onSort("name")
  openFolderCreation("server", 2, true, control.open)
  expect(control.mutation).not.toHaveBeenCalled()
  expect(control.open).not.toHaveBeenCalled()
})

test.each(["instance", "server"] as const)(
  "%s folder ghost selects the explicit namespace when group IDs collide",
  (kind) => {
    control.dragState = {
      dragType: () => (kind === "server" ? "serverGroup" : "group"),
      draggedIds: () => [2],
      isDragging: () => true,
      dragDetached: () => true,
      dropAnimating: () => null,
      dropPreviewRect: () => null,
      dropTarget: () => null,
      ghostPosition: () => ({ x: 100, y: 100 })
    }
    dispose = render(
      () => (
        <DragGhost
          {...({
            instances: [client],
            servers,
            groups: [
              {
                id: 2,
                name: "Survival",
                kind: "instance",
                instances: [client]
              },
              { id: 2, name: "Hosting", kind: "server", instances: servers }
            ],
            tileSize: 2
          } as any)}
        />
      ),
      root
    )
    const ghost = document.querySelector('[class*="z-[10002]"]')!
    expect(ghost.querySelector("h4")?.textContent).toBe(
      kind === "server" ? "Hosting" : "Survival"
    )
    expect(ghost.querySelector("img")?.getAttribute("src")).toBe(
      kind === "server" ? "/server-icon/7/4" : "/instance-icon/9/3"
    )
  }
)

test.each(["instance", "server"] as const)(
  "expanded %s folder registers only matching member drags in its namespace",
  (kind) => {
    const [dragType, setDragType] = createSignal(
      kind === "server" ? "instance" : "server"
    )
    const register = vi.fn(),
      setScope = vi.fn()
    control.dragState = {
      dragType,
      isDragging: () => true,
      draggedIds: () => [],
      registerDropZone: register,
      unregisterDropZone: vi.fn(),
      setActiveScope: setScope
    }
    dispose = render(
      () => (
        <ExpandedFolderContent
          {...({
            group: {
              id: 2,
              kind,
              name: "Folder",
              instances: kind === "server" ? servers : [client]
            },
            onClose: vi.fn(),
            tileSize: 2,
            isDefaultGroup: false,
            selectedIds: new Set(),
            onSetSelection: vi.fn()
          } as any)}
        />
      ),
      root
    )
    const scope = kind === "server" ? "server-folder-2" : "folder-2"
    expect(setScope).toHaveBeenCalledWith(scope)
    expect(register).not.toHaveBeenCalled()
    setDragType(kind)
    expect(register).toHaveBeenCalled()
    expect(register.mock.calls.every(([zone]) => zone.scope === scope)).toBe(
      true
    )
    expect(
      register.mock.calls.some(
        ([zone]) => zone.target.type === "beforeInstance"
      )
    ).toBe(true)
  }
)

test("mixed folder failure retry preserves both successful server and folder acknowledgements", async () => {
  let rejectSecond = true
  const done = new Set<string>()
  control.mutation.mockImplementation(async (key: string, id: number) => {
    const target = `${key}:${id}`
    if (done.has(target)) throw new Error("Target not found")
    if (key === "server.deleteServerGroup" && id === 8 && rejectSecond)
      throw new Error("Busy folder")
    done.add(target)
  })
  dispose = render(
    () => (
      <ConfirmBatchServerDeletion
        {...({
          data: {
            servers: [servers[0]],
            folders: [
              { id: 7, name: "Equal-ID folder" },
              { id: 8, name: "Pending" }
            ]
          }
        } as any)}
      />
    ),
    root
  )
  button("_trn_delete_servers_button").click()
  await vi.waitFor(() =>
    expect(root.querySelector('[role="alert"]')).not.toBeNull()
  )
  expect(control.mutation.mock.calls).toEqual([
    ["server.deleteServer", 7],
    ["server.deleteServerGroup", 7],
    ["server.deleteServerGroup", 8]
  ])
  rejectSecond = false
  button("_trn_delete_servers_button").click()
  await vi.waitFor(() => expect(control.close).toHaveBeenCalledTimes(1))
  expect(control.mutation.mock.calls).toEqual([
    ["server.deleteServer", 7],
    ["server.deleteServerGroup", 7],
    ["server.deleteServerGroup", 8],
    ["server.deleteServerGroup", 8]
  ])
})

test.each([
  {
    mode: "unlink",
    chosen: "_trn_unlink_folders_button",
    alternate: "_trn_delete_folders_with_instances_button",
    endpoint: "instance.deleteGroup"
  },
  {
    mode: "deleteAll",
    chosen: "_trn_delete_folders_with_instances_button",
    alternate: "_trn_unlink_folders_button",
    endpoint: "instance.deleteGroupWithInstances"
  }
] as const)(
  "R1 partial $mode refuses changing the captured action and retries only pending folders",
  async ({ chosen, alternate, endpoint }) => {
    let rejectSecond = true
    const completed = new Set<number>()
    control.mutation.mockImplementation(async (_key: string, id: number) => {
      if (completed.has(id)) throw new Error("Folder not found")
      if (id === 3 && rejectSecond) throw new Error("Busy folder")
      completed.add(id)
    })
    const done = vi.fn()
    dispose = render(
      () => (
        <ConfirmBatchFolderDeletion
          {...({
            data: {
              kind: "instance",
              folders: [
                { id: 2, name: "First" },
                { id: 3, name: "Second" }
              ],
              onComplete: done
            }
          } as any)}
        />
      ),
      root
    )
    expect(button(chosen).disabled).toBe(false)
    expect(button(alternate).disabled).toBe(false)
    button(chosen).click()
    await vi.waitFor(() =>
      expect(root.querySelector('[role="alert"]')).not.toBeNull()
    )
    const pendingAlert = root.querySelector('[role="alert"]')!
    expect(control.mutation.mock.calls).toEqual([
      [endpoint, 2],
      [endpoint, 3]
    ])
    expect(control.close).not.toHaveBeenCalled()
    expect(done).not.toHaveBeenCalled()
    expect(control.navigate).not.toHaveBeenCalled()
    rejectSecond = false
    const alternateButton = button(alternate)
    // Call the production callback independently of the DOM disabled guard.
    await control.buttonActions.get(alternateButton)!()
    expect(control.mutation.mock.calls).toEqual([
      [endpoint, 2],
      [endpoint, 3]
    ])
    expect(root.querySelector('[role="alert"]')).toBe(pendingAlert)
    expect(control.error).toHaveBeenCalledTimes(1)
    expect(control.close).not.toHaveBeenCalled()
    expect(done).not.toHaveBeenCalled()
    expect(control.navigate).not.toHaveBeenCalled()
    expect(alternateButton.disabled).toBe(true)
    alternateButton.click()
    expect(control.mutation.mock.calls).toEqual([
      [endpoint, 2],
      [endpoint, 3]
    ])
    expect(button(chosen).disabled).toBe(false)
    button(chosen).click()
    await vi.waitFor(() => expect(control.close).toHaveBeenCalledTimes(1))
    expect(control.mutation.mock.calls).toEqual([
      [endpoint, 2],
      [endpoint, 3],
      [endpoint, 3]
    ])
    expect(done).toHaveBeenCalledTimes(1)
    expect(control.navigate).toHaveBeenCalledExactlyOnceWith("/library")
  }
)

test("R1 a partial pinned operation still permits cancellation without completing pending targets", async () => {
  control.mutation.mockImplementation(async (_key: string, id: number) => {
    if (id === 3) throw new Error("Busy folder")
  })
  const done = vi.fn()
  dispose = render(
    () => (
      <ConfirmBatchFolderDeletion
        {...({
          data: {
            kind: "instance",
            folders: [
              { id: 2, name: "First" },
              { id: 3, name: "Second" }
            ],
            onComplete: done
          }
        } as any)}
      />
    ),
    root
  )
  button("_trn_unlink_folders_button").click()
  await vi.waitFor(() =>
    expect(root.querySelector('[role="alert"]')).not.toBeNull()
  )
  const cancel = button("_trn_instance_confirm_deletion.cancel")
  expect(cancel.disabled).toBe(false)
  cancel.click()
  expect(control.close).toHaveBeenCalledTimes(1)
  expect(done).not.toHaveBeenCalled()
  expect(control.navigate).not.toHaveBeenCalled()
  expect(control.mutation.mock.calls).toEqual([
    ["instance.deleteGroup", 2],
    ["instance.deleteGroup", 3]
  ])
})
