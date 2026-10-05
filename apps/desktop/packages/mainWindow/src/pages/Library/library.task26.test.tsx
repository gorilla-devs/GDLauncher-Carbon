import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import { createStore } from "solid-js/store"
const c = vi.hoisted(() => ({
  store: null as any,
  drag: null as any,
  mutation: vi.fn(),
  actions: new Map<any, any>(),
  header: vi.fn(),
  animations: [] as any[],
  frames: [] as any[]
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/task26/ui")
  return {
    ...ui,
    DropdownMenuItem: (p: any) => {
      const el = ui.Button(p)
      c.actions.set(el, p.onClick)
      return el
    }
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.store
}))
vi.mock("@/pages/Library/DragContext", () => ({ useDragContext: () => c.drag }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({ data: 1 }),
    createMutation: (opts: any) => ({
      mutate: (v: any) => c.mutation(opts().mutationKey[0], v),
      mutateAsync: (v: any) => c.mutation(opts().mutationKey[0], v)
    })
  }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/components/FeatureStatusBadge", () => ({ default: () => <div /> }))
vi.mock("@/components/InstanceTile", () => ({ default: () => <div /> }))
vi.mock("@/components/Server/Tile", () => ({ default: () => <div /> }))
vi.mock("@/components/Library/FolderTile", () => ({
  default: (p: any) => <div data-folder-kind={p.kind}>{p.groupId}</div>
}))
vi.mock("@/pages/Library/components/DropOverlayIndicator", () => ({
  DropOverlayIndicator: () => <div />
}))
vi.mock("@/pages/Library/hooks/useDragLayoutAnimation", () => ({
  useDragLayoutAnimation: vi.fn()
}))
vi.mock("@/pages/Library/hooks", async () => ({
  ...(await import("./hooks/useDropIndicators")),
  ...(await import("./hooks/useDropZoneRegistration")),
  useLibraryItemAnimation: vi.fn()
}))
vi.mock("@/components/Library/GroupHeader", () => ({
  default: (p: any) => {
    c.header(p)
    return <div />
  }
}))
vi.mock("@/pages/Library/components/DropPreviewTile", () => ({
  default: () => <div data-drop-preview />
}))
import { useDropZoneRegistration } from "./hooks/useDropZoneRegistration"
import { useDropIndicators } from "./hooks/useDropIndicators"
import { useLibraryDragDrop } from "./hooks/useLibraryDragDrop"
import { LibraryGrid } from "./views/FoldersView/LibraryGrid"
import { EndOfGroupsDropZone } from "./components/EndOfGroupDropZone"
import { LibraryHeader } from "./components/LibraryHeader"
import AccordionView from "./views/AccordionView"
import { useLibraryData } from "./hooks/useLibraryData"
import {
  computeServerLibraryItems,
  computeServerVirtualGroups
} from "./utils/serverGrouping"
let root: HTMLDivElement, dispose: (() => void) | undefined, setStore: any
const selection = {
  selectedIds: () => new Set<string>(),
  isSelected: () => false,
  toggleSelection: vi.fn(),
  clearSelection: vi.fn(),
  selectAll: vi.fn()
}
function client(id: number, version = "1.21") {
  return {
    id,
    group_id: 1,
    name: `Client ${id}`,
    index: id,
    library_position: id + 100,
    favorite: false,
    status: {
      status: "valid",
      value: {
        mc_version: version,
        modloader: null,
        modpack: null,
        state: { state: "inactive" }
      }
    }
  }
}
function server(id: number, version = "1.21") {
  return {
    id,
    groupId: 1,
    name: `Server ${id}`,
    index: id,
    libraryPosition: id + 100,
    gameVersion: version
  }
}
function drag(type: string) {
  const [state, set] = createStore({
    type,
    active: true,
    ids: [3],
    target: null as any
  })
  const zones = new Map<string, any>()
  c.drag = {
    isDragging: () => state.active,
    justDropped: () => false,
    dragDetached: () => true,
    dragType: () => state.type,
    draggedIds: () => state.ids,
    dropTarget: () => state.target,
    registerDropZone: (z: any) => zones.set(z.id, z),
    unregisterDropZone: (id: string) => zones.delete(id)
  }
  return { state, set, zones }
}
beforeEach(() => {
  vi.clearAllMocks()
  c.actions.clear()
  c.animations = []
  c.frames = []
  const store = createStore<any>({
    settings: {
      data: {
        instancesGroupBy: null,
        instancesSortBy: "name",
        instancesSortByAsc: true,
        reducedMotion: false
      }
    },
    instanceGroups: {
      data: [
        { id: 1, name: "localize➽default", library_position: 0 },
        { id: 2, name: "Clients", library_position: 10 }
      ]
    },
    serverGroups: {
      data: [
        { id: 1, name: "localize➽default", libraryPosition: 0 },
        { id: 2, name: "Servers", libraryPosition: 10 }
      ]
    },
    instances: { data: [client(7)], isLoading: false },
    servers: { data: [server(7)] }
  })
  c.store = store[0]
  setStore = store[1]
  drag("serverGroup")
  root = document.createElement("div")
  document.body.append(root)
  vi.stubGlobal("requestAnimationFrame", (fn: any) => {
    c.frames.push(fn)
    return c.frames.length
  })
  HTMLDivElement.prototype.animate = function (_frames: any, options: any) {
    const animation = { onfinish: null, cancel: vi.fn() }
    c.animations.push({ element: this, options, animation })
    return animation as any
  }
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
  vi.unstubAllGlobals()
  delete (HTMLDivElement.prototype as any).animate
})
test.each(["group", "serverGroup"])(
  "%s registers folder targets with matching feedback, collapse and complete cleanup",
  (type) => {
    const { set, zones } = drag(type)
    const kind = type === "group" ? "instance" : "server"
    let indicators: any
    dispose = render(() => {
      const el = document.createElement("div")
      el.getBoundingClientRect = () => new DOMRect(10, 20, 100, 120)
      useDropZoneRegistration({
        itemId: 2,
        itemType: "folder",
        itemKind: kind,
        ref: () => el,
        dragContext: c.drag,
        defaultGroupId: () => 1
      } as any)
      indicators = useDropIndicators({
        itemId: 2,
        itemType: "folder",
        itemKind: kind,
        dragContext: c.drag
      } as any)
      return <EndOfGroupsDropZone {...({ tileSize: () => 2, kind } as any)} />
    }, root)
    expect([...zones.values()].map((z) => z.target.type).sort()).toEqual([
      "beforeGroup",
      "endOfGroups"
    ])
    set("target", { type: "beforeGroup", groupId: 2 })
    expect(indicators.showFolderDropIndicator()).toBe(true)
    set("ids", [2])
    expect(indicators.shouldCollapseTile()).toBe(true)
    expect(zones.has("before-group-2")).toBe(false)
    set("active", false)
    expect(zones.size).toBe(0)
    dispose()
    dispose = undefined
    expect(zones.size).toBe(0)
  }
)
test("equal numeric folder IDs never register or collapse in the other namespace", () => {
  const { zones } = drag("serverGroup")
  let indicators: any
  dispose = render(() => {
    const el = document.createElement("div")
    useDropZoneRegistration({
      itemId: 3,
      itemType: "folder",
      itemKind: "instance",
      ref: () => el,
      dragContext: c.drag,
      defaultGroupId: () => 1
    } as any)
    indicators = useDropIndicators({
      itemId: 3,
      itemType: "folder",
      itemKind: "instance",
      dragContext: c.drag
    } as any)
    return el
  }, root)
  expect(zones.size).toBe(0)
  expect(indicators.shouldCollapseTile()).toBe(false)
})
test.each(["group", "serverGroup"])(
  "%s member-area gesture admits only the first boundary and dispatches the folder tail",
  (type) => {
    const { zones, set } = drag(type)
    const memberType = type === "group" ? "instance" : "server"
    let a: any, b: any, handler: any
    dispose = render(() => {
      for (const [id, first] of [
        [7, true],
        [8, false]
      ] as const) {
        const el = document.createElement("div")
        useDropZoneRegistration({
          itemId: id,
          itemType: memberType,
          itemKind: memberType,
          isFirstUngrouped: () => first,
          ref: () => el,
          dragContext: c.drag,
          defaultGroupId: () => 1
        } as any)
        const state = useDropIndicators({
          itemId: id,
          itemType: memberType,
          itemKind: memberType,
          isFirstUngrouped: () => first,
          dragContext: c.drag
        } as any)
        if (first) a = state
        else b = state
      }
      handler = useLibraryDragDrop({
        defaultGroupId: () => 1,
        selection,
        flipAnimation: { capturePositions: vi.fn() } as any,
        libraryItems: []
      })
      return <div />
    }, root)
    expect([...zones.values()].map((z) => z.target)).toEqual([
      { type: "beforeGroupAtInstance", beforeInstanceId: 7 }
    ])
    set("target", { type: "beforeGroupAtInstance", beforeInstanceId: 7 })
    expect(a.showGroupDropIndicator()).toBe(true)
    expect(b.showGroupDropIndicator()).toBe(false)
    handler.handleDrop(
      { type: "beforeGroupAtInstance", beforeInstanceId: 8 },
      [3],
      type,
      null
    )
    expect(c.mutation).toHaveBeenCalledExactlyOnceWith(
      type === "group" ? "instance.moveGroup" : "server.moveServerGroup",
      { group: 3, target: type === "group" ? "EndOfLibrary" : "endOfLibrary" }
    )
    handler.handleDrop({ type: "beforeGroup", groupId: 3 }, [3], type, null)
    handler.handleDrop(null, [3], type, null)
    expect(c.mutation).toHaveBeenCalledTimes(1)
  }
)
test("sparse root positions render folders first independently from member indices", () => {
  let data: any
  setStore("instances", "data", [
    { ...client(7), index: 90, library_position: 20 },
    { ...client(8), index: 2, library_position: 70 }
  ])
  setStore("instanceGroups", "data", [
    { id: 1, name: "localize➽default", library_position: 0 },
    { id: 2, name: "a", library_position: 10 },
    { id: 3, name: "tail", library_position: 1000 }
  ])
  dispose = render(() => {
    data = useLibraryData(() => "")
    return <div />
  }, root)
  expect(data.libraryItems.map((i: any) => i.id)).toEqual([
    "folder-2",
    "folder-3",
    "instance-7",
    "instance-8"
  ])
  const items = computeServerLibraryItems(
    [
      { ...server(7), index: 90, libraryPosition: 20 },
      { ...server(8), index: 2, libraryPosition: 70 }
    ] as any,
    [
      { id: 1, name: "localize➽default", libraryPosition: 0 },
      { id: 2, name: "a", libraryPosition: 10 },
      { id: 3, name: "tail", libraryPosition: 1000 }
    ] as any,
    "",
    1
  )
  expect(items.map((i) => i.id)).toEqual([
    "server-folder-2",
    "server-folder-3",
    "server-7",
    "server-8"
  ])
})
function accordion(
  mode: "instances" | "servers",
  count = 300,
  reduced = false,
  complete = false,
  animated = new Set<string | number>()
) {
  setStore("settings", "data", "reducedMotion", reduced)
  const groups = [
    {
      id: "game-version-1",
      name: "one",
      instances: Array.from({ length: count }, (_, i) =>
        mode === "servers" ? server(i + 1) : client(i + 1)
      )
    },
    {
      id: "game-version-2",
      name: "two",
      instances: [mode === "servers" ? server(count + 1) : client(count + 1)]
    }
  ]
  const state = { value: complete },
    refs = new Map<string, HTMLDivElement>()
  dispose = render(
    () => (
      <AccordionView
        {...({
          virtualGroups: groups,
          tileSize: () => 2,
          selection,
          onDragStart: vi.fn(),
          justDropped: () => false,
          animatedInstanceIds: animated,
          initialAnimationComplete: state,
          tileRefs: refs,
          libraryMode: mode
        } as any)}
      />
    ),
    root
  )
  return { groups, state, refs, animated }
}
test.each(["instances", "servers"] as const)(
  "GPT0122 %s entrances use a shared <=500ms budget and immediate visible fallback",
  (mode) => {
    const { refs, animated } = accordion(mode)
    expect(c.animations.length).toBeGreaterThan(0)
    expect(
      Math.max(...c.animations.map((a) => a.options.delay))
    ).toBeLessThanOrEqual(500)
    expect(c.animations[0].options).toMatchObject({ delay: 100, duration: 250 })
    expect(refs.size).toBe(301)
    expect(animated.size).toBe(301)
    expect(
      [...refs.values()].slice(20).every((el) => el.style.opacity === "1")
    ).toBe(true)
  }
)
test.each(["instances", "servers"] as const)(
  "%s reduced motion and completed remount never animate and become visible immediately",
  (mode) => {
    const { refs } = accordion(mode, 2, true)
    expect(c.animations).toHaveLength(0)
    expect([...refs.values()].every((el) => el.style.opacity === "1")).toBe(
      true
    )
    dispose!()
    dispose = undefined
    root.replaceChildren()
    const next = accordion(mode, 2, false, true)
    expect(c.animations).toHaveLength(0)
    expect(
      [...next.refs.values()].every((el) => el.style.opacity === "1")
    ).toBe(true)
  }
)
test("unavailable WAAPI keeps loaded content visible and cleanup removes owned refs", () => {
  delete (HTMLDivElement.prototype as any).animate
  const { refs, animated } = accordion("instances", 2)
  expect([...refs.values()].every((el) => el.style.opacity === "1")).toBe(true)
  expect(animated.size).toBe(3)
  dispose!()
  dispose = undefined
  expect(refs.size).toBe(0)
})
test("all actual virtual producer modes feed string groups through their matching AccordionView caller, leaving numeric headers dormant", () => {
  for (const groupBy of [null, "gameVersion", "modloader", "modplatform"]) {
    setStore("settings", "data", "instancesGroupBy", groupBy)
    let data: any
    dispose = render(() => {
      data = useLibraryData(() => "Client")
      return <div />
    }, root)
    const clientGroups = [...data.virtualGroups]
    expect(clientGroups.length).toBeGreaterThan(0)
    expect(clientGroups.every((g: any) => typeof g.id === "string")).toBe(true)
    const serverGroups = computeServerVirtualGroups(
      [server(7)] as any,
      c.store.settings.data,
      "Server",
      "Search"
    )
    expect(serverGroups.length).toBeGreaterThan(0)
    expect(serverGroups.every((g) => typeof g.id === "string")).toBe(true)
    dispose()
    dispose = undefined
    root.replaceChildren()
    for (const [mode, groups] of [
      ["instances", clientGroups],
      ["servers", serverGroups]
    ]) {
      dispose = render(
        () => (
          <AccordionView
            {...({
              virtualGroups: groups,
              tileSize: () => 2,
              selection,
              onDragStart: vi.fn(),
              justDropped: () => false,
              animatedInstanceIds: new Set(),
              initialAnimationComplete: { value: true },
              tileRefs: new Map(),
              libraryMode: mode
            } as any)}
          />
        ),
        root
      )
      expect(c.header).not.toHaveBeenCalled()
      expect(
        [...root.querySelectorAll("[data-custom-header]")].every(
          (el) => el.getAttribute("data-custom-header") === "false"
        )
      ).toBe(true)
      dispose()
      dispose = undefined
      root.replaceChildren()
    }
  }
})

test("GPT0175 server folders advertise only By name and stale direct client criteria cannot mutate", () => {
  const [mode, setMode] = createSignal("instances")
  const [view, setView] = createSignal("folders")
  dispose = render(
    () => (
      <LibraryHeader
        {...({
          libraryMode: mode,
          setLibraryMode: setMode,
          viewMode: view,
          tileSize: () => 2,
          setTileSize: vi.fn(),
          filter: () => "",
          setFilter: vi.fn()
        } as any)}
      />
    ),
    root
  )
  const find = (key: string) =>
    [...root.querySelectorAll("button")].find((el) =>
      el.textContent?.includes(key)
    )!
  const stale = ["date_created", "last_played", "most_played"].map((key) =>
    c.actions.get(find(`_trn_by_${key}`))
  )
  expect(stale.every(Boolean)).toBe(true)
  setMode("servers")
  expect(find("_trn_by_date_created")).toBeUndefined()
  expect(find("_trn_by_last_played")).toBeUndefined()
  expect(find("_trn_by_most_played")).toBeUndefined()
  stale.forEach((action) => action())
  expect(c.mutation).not.toHaveBeenCalled()
  find("_trn_by_name").click()
  expect(c.mutation).toHaveBeenLastCalledWith(
    "server.arrangeServerLibrary",
    undefined
  )
  setMode("instances")
  for (const key of ["date_created", "last_played", "most_played"]) {
    find(`_trn_by_${key}`).click()
  }
  expect(c.mutation.mock.calls.slice(-3).map((x) => x[1])).toEqual([
    "dateCreated",
    "lastPlayed",
    "mostPlayed"
  ])
  setMode("servers")
  setView("accordion")
  expect(root.textContent).toContain("ui:_trn_created")
})

test.each(["instances", "servers"] as const)(
  "actual %s grid forwards identity/boundary and places the tail zone before members",
  (mode) => {
    const kind = mode === "servers" ? "server" : "instance",
      type = mode === "servers" ? "serverGroup" : "group"
    const { zones } = drag(type)
    setStore("instanceGroups", "data", [
      { id: 1, name: "default", library_position: 0 },
      { id: 2, name: "a", library_position: 10 },
      { id: 3, name: "tail", library_position: 1000 }
    ])
    setStore("instances", "data", [
      { ...client(7), index: 90, library_position: 20 },
      { ...client(8), index: 2, library_position: 70 }
    ])
    const servers = [
        { ...server(7), index: 90, libraryPosition: 20 },
        { ...server(8), index: 2, libraryPosition: 70 }
      ],
      groups = [
        { id: 1, name: "default", libraryPosition: 0 },
        { id: 2, name: "a", libraryPosition: 10 },
        { id: 3, name: "tail", libraryPosition: 1000 }
      ]
    dispose = render(() => {
      const data = useLibraryData(() => "")
      return (
        <LibraryGrid
          {...({
            libraryMode: mode,
            libraryItems:
              mode === "servers"
                ? computeServerLibraryItems(
                    servers as any,
                    groups as any,
                    "",
                    1
                  )
                : data.libraryItems,
            tileSize: () => 2,
            defaultGroupId: 1,
            openFolderId: () => null,
            onToggleFolder: vi.fn(),
            selection,
            onDragStart: vi.fn(),
            justDropped: () => false,
            flipAnimation: {},
            entranceAnimation: {
              animatedIds: new Set(),
              initialComplete: true
            },
            tileRefs: new Map(),
            newlyCreatedFolderId: () => null,
            clearNewlyCreatedFolderId: vi.fn()
          } as any)}
        />
      )
    }, root)
    expect(
      [...root.querySelectorAll("[data-folder-kind]")].map((el) =>
        el.getAttribute("data-folder-kind")
      )
    ).toEqual([kind, kind])
    expect(
      [...zones.values()]
        .filter((z) => z.target.type === "beforeGroupAtInstance")
        .map((z) => z.target.beforeInstanceId)
    ).toEqual([7])
    const tiles = root.querySelectorAll("[data-library-item]"),
      tail = zones.get("end-of-groups").element
    expect(
      tiles[1].compareDocumentPosition(tail) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy()
    expect(
      tail.compareDocumentPosition(tiles[2]) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy()
    dispose()
    dispose = undefined
    expect(zones.size).toBe(0)
  }
)
test("skipped entrances finish bookkeeping and completed stable identities stay visible on remount", () => {
  const first = accordion("servers", 2, true)
  expect(first.state.value).toBe(false)
  c.frames.forEach((fn) => fn(0))
  expect(first.state.value).toBe(true)
  dispose!()
  dispose = undefined
  expect(first.refs.size).toBe(0)
  root.replaceChildren()
  const second = accordion("servers", 2, false, false, first.animated)
  expect(c.animations).toHaveLength(0)
  expect(
    [...second.refs.values()].every((el) => el.style.opacity === "1")
  ).toBe(true)
})
