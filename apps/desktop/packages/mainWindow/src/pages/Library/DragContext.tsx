import {
  createContext,
  useContext,
  createSignal,
  Accessor,
  JSX,
  onCleanup
} from "solid-js"

export type DragType = "instance" | "group"

export type DropTarget =
  | { type: "favorites" }
  | { type: "beforeInstance"; instanceId: number; groupId: number }
  | { type: "endOfGroup"; groupId: number }
  | { type: "beforeGroup"; groupId: number }
  | { type: "endOfGroups" }
  | { type: "dropOnFolder"; groupId: number } // Drop on collapsed folder
  | { type: "createFolder"; instanceId: number } // Drop on ungrouped instance to create folder
  | { type: "ungrouped" } // Return to default group (main grid)
  | { type: "beforeGroupAtInstance"; beforeInstanceId: number } // Position group before an ungrouped instance
  | { type: "endOfLibrary" } // Position group at end of library
  | { type: "beforeInstanceAtFolder"; folderId: number } // Position instance before a folder

export interface DraggedItem {
  type: DragType
  ids: number[]
}

export interface GhostPosition {
  x: number
  y: number
}

export interface DropZone {
  id: string
  rect: DOMRect
  element?: HTMLElement
  rectTransform?: (rect: DOMRect) => DOMRect
  target: DropTarget
  scope?: string // Optional scope for filtering (e.g., "folder-123")
}

type DropHandler = (
  target: DropTarget,
  draggedIds: number[],
  dragType: DragType
) => void

interface DragContextValue {
  // State accessors
  isDragging: Accessor<boolean>
  dragType: Accessor<DragType | null>
  draggedIds: Accessor<number[]>
  dropTarget: Accessor<DropTarget | null>
  ghostPosition: Accessor<GhostPosition>
  dragSelectEnabled: Accessor<boolean>
  justDropped: Accessor<boolean>
  activeScope: Accessor<string | null>

  // Actions
  startDrag: (type: DragType, ids: number[], e: PointerEvent) => void
  updateDrag: (e: PointerEvent) => void
  endDrag: () => void
  cancelDrag: () => void
  setDragSelectEnabled: (enabled: boolean) => void
  setActiveScope: (scope: string | null) => void

  // Drop zone registration
  registerDropZone: (zone: DropZone) => void
  unregisterDropZone: (id: string) => void
  getDropZones: () => DropZone[]

  // Drop handler
  setOnDrop: (handler: DropHandler | null) => void
}

const DragContext = createContext<DragContextValue>()

const MIN_DRAG_DISTANCE = 5

export function DragProvider(props: { children: JSX.Element }) {
  const [isDragging, setIsDragging] = createSignal(false)
  const [hasDragStarted, setHasDragStarted] = createSignal(false)
  const [dragType, setDragType] = createSignal<DragType | null>(null)
  const [draggedIds, setDraggedIds] = createSignal<number[]>([])
  const [dropTarget, setDropTarget] = createSignal<DropTarget | null>(null)
  const [ghostPosition, setGhostPosition] = createSignal<GhostPosition>({
    x: 0,
    y: 0
  })
  const [dragSelectEnabled, setDragSelectEnabled] = createSignal(true)
  const [startPosition, setStartPosition] = createSignal<{
    x: number
    y: number
  } | null>(null)
  const [justDropped, setJustDropped] = createSignal(false)
  const [activeScope, setActiveScope] = createSignal<string | null>(null)

  // Drop zones registry
  let dropZones: DropZone[] = []
  let onDropHandler: DropHandler | null = null
  let scrollCleanup: (() => void) | null = null

  // Position-based stabilization: reject target changes when cursor hasn't moved
  let lastStableTarget: DropTarget | null = null
  let lastStablePosition: { x: number; y: number } | null = null
  const POSITION_THRESHOLD = 10

  // Throttle rect refresh to max 20 refreshes/second
  let lastRefreshTime = 0
  const REFRESH_THROTTLE = 50

  const setOnDrop = (handler: DropHandler | null) => {
    onDropHandler = handler
  }

  const registerDropZone = (zone: DropZone) => {
    // Remove existing zone with same id
    dropZones = dropZones.filter((z) => z.id !== zone.id)
    dropZones.push(zone)
  }

  const unregisterDropZone = (id: string) => {
    dropZones = dropZones.filter((z) => z.id !== id)
  }

  const getDropZones = () => dropZones

  const refreshDropZoneRects = () => {
    const now = performance.now()
    if (now - lastRefreshTime < REFRESH_THROTTLE) return
    lastRefreshTime = now

    for (const zone of dropZones) {
      if (zone.element?.isConnected) {
        const rawRect = zone.element.getBoundingClientRect()
        zone.rect = zone.rectTransform ? zone.rectTransform(rawRect) : rawRect
      }
    }
  }

  const findDropTarget = (x: number, y: number): DropTarget | null => {
    refreshDropZoneRects()

    // Filter zones by active scope - when scope is set, only consider scoped zones
    const scope = activeScope()
    const scopedZones =
      scope !== null ? dropZones.filter((z) => z.scope === scope) : dropZones

    // Sort drop zones by priority (favorites first, then instances, then groups)
    const sortedZones = [...scopedZones].sort((a, b) => {
      const priority: Record<DropTarget["type"], number> = {
        favorites: 0,
        createFolder: 1, // Higher priority than beforeInstance (center of tile)
        beforeInstance: 2,
        beforeGroupAtInstance: 2.5, // Same band as beforeInstance for groups
        beforeInstanceAtFolder: 2.5, // Same band for instances before folders
        dropOnFolder: 3,
        endOfGroup: 4,
        beforeGroup: 5,
        endOfGroups: 5.5,
        endOfLibrary: 5.5,
        ungrouped: 7
      }
      return priority[a.target.type] - priority[b.target.type]
    })

    for (const zone of sortedZones) {
      const rect = zone.rect
      if (
        x >= rect.left &&
        x <= rect.right &&
        y >= rect.top &&
        y <= rect.bottom
      ) {
        return zone.target
      }
    }

    return null
  }

  const isSameTarget = (
    a: DropTarget | null,
    b: DropTarget | null
  ): boolean => {
    if (a === null && b === null) return true
    if (a === null || b === null) return false
    if (a.type !== b.type) return false
    switch (a.type) {
      case "favorites":
      case "endOfGroups":
      case "ungrouped":
      case "endOfLibrary":
        return true
      case "beforeInstance":
        return (
          a.instanceId === (b as typeof a).instanceId &&
          a.groupId === (b as typeof a).groupId
        )
      case "endOfGroup":
      case "dropOnFolder":
      case "beforeGroup":
        return a.groupId === (b as typeof a).groupId
      case "createFolder":
        return a.instanceId === (b as typeof a).instanceId
      case "beforeGroupAtInstance":
        return a.beforeInstanceId === (b as typeof a).beforeInstanceId
      case "beforeInstanceAtFolder":
        return a.folderId === (b as typeof a).folderId
      default:
        return false
    }
  }

  const resolveDropTarget = (
    newTarget: DropTarget | null,
    cursorX: number,
    cursorY: number,
    forceAccept?: boolean
  ): DropTarget | null => {
    // Same target — just update cursor position
    if (isSameTarget(newTarget, lastStableTarget)) {
      lastStablePosition = { x: cursorX, y: cursorY }
      return newTarget
    }

    // Target changed — check if cursor moved enough to accept
    if (
      !forceAccept &&
      lastStableTarget !== null &&
      lastStablePosition !== null
    ) {
      const dx = Math.abs(cursorX - lastStablePosition.x)
      const dy = Math.abs(cursorY - lastStablePosition.y)
      if (dx < POSITION_THRESHOLD && dy < POSITION_THRESHOLD) {
        return lastStableTarget
      }
    }

    // Accept the new target
    lastStableTarget = newTarget
    lastStablePosition = { x: cursorX, y: cursorY }
    return newTarget
  }

  const resetDropTargetHysteresis = () => {
    lastStableTarget = null
    lastStablePosition = null
  }

  const startDrag = (type: DragType, ids: number[], e: PointerEvent) => {
    setDragType(type)
    setDraggedIds(ids)
    setStartPosition({ x: e.clientX, y: e.clientY })
    setGhostPosition({ x: e.clientX, y: e.clientY })
    setHasDragStarted(false)

    // Add document-level listeners
    document.addEventListener("pointermove", handlePointerMove)
    document.addEventListener("pointerup", handlePointerUp)
    document.addEventListener("keydown", handleKeyDown)

    const handleScroll = () => {
      if (hasDragStarted()) {
        const pos = ghostPosition()
        const target = findDropTarget(pos.x, pos.y)
        setDropTarget(resolveDropTarget(target, pos.x, pos.y, true))
      }
    }
    document.addEventListener("scroll", handleScroll, {
      capture: true,
      passive: true
    })
    scrollCleanup = () =>
      document.removeEventListener("scroll", handleScroll, { capture: true })
  }

  const handlePointerMove = (e: PointerEvent) => {
    const start = startPosition()
    if (!start) return

    const dx = Math.abs(e.clientX - start.x)
    const dy = Math.abs(e.clientY - start.y)

    // Check if we've moved enough to start dragging
    if (
      !hasDragStarted() &&
      (dx >= MIN_DRAG_DISTANCE || dy >= MIN_DRAG_DISTANCE)
    ) {
      setHasDragStarted(true)
      setIsDragging(true)
      setDragSelectEnabled(false)
    }

    if (hasDragStarted()) {
      setGhostPosition({ x: e.clientX, y: e.clientY })

      // Find drop target
      const target = findDropTarget(e.clientX, e.clientY)
      setDropTarget(resolveDropTarget(target, e.clientX, e.clientY))
    }
  }

  const handlePointerUp = () => {
    cleanup()

    if (hasDragStarted()) {
      setJustDropped(true)

      const target = dropTarget()
      const ids = draggedIds()
      const type = dragType()

      if (target && ids.length > 0 && type && onDropHandler) {
        onDropHandler(target, ids, type)
      }

      setIsDragging(false)

      // Delay clearing visual state so preview tiles and collapsed tiles persist
      // during the settlement window, allowing data to arrive at the correct
      // DOM positions before the layout shifts.
      setTimeout(() => {
        // Clear visual state first while justDropped still suppresses auto-animate.
        // This way DOM mutations (preview unmount, tile unhide) happen with
        // auto-animate disabled, preventing the folder grid from animating them.
        if (!isDragging()) {
          setDragType(null)
          setDraggedIds([])
          setDropTarget(null)
        }
        // Clear justDropped in the next frame so the DOM has settled before
        // auto-animate re-enables — no mutations left to animate.
        requestAnimationFrame(() => {
          setJustDropped(false)
        })
      }, 100)
    } else {
      // No drag started — clear immediately
      setDragType(null)
      setDraggedIds([])
      setDropTarget(null)
    }

    setHasDragStarted(false)
    setStartPosition(null)
    setDragSelectEnabled(true)
    resetDropTargetHysteresis()
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      cancelDrag()
    }
  }

  const cleanup = () => {
    document.removeEventListener("pointermove", handlePointerMove)
    document.removeEventListener("pointerup", handlePointerUp)
    document.removeEventListener("keydown", handleKeyDown)
    if (scrollCleanup) {
      scrollCleanup()
      scrollCleanup = null
    }
  }

  const updateDrag = (e: PointerEvent) => {
    handlePointerMove(e)
  }

  const endDrag = () => {
    handlePointerUp()
  }

  const cancelDrag = () => {
    cleanup()
    setIsDragging(false)
    setHasDragStarted(false)
    setDragType(null)
    setDraggedIds([])
    setDropTarget(null)
    setStartPosition(null)
    setDragSelectEnabled(true)
    resetDropTargetHysteresis()
  }

  onCleanup(cleanup)

  const value: DragContextValue = {
    isDragging,
    dragType,
    draggedIds,
    dropTarget,
    ghostPosition,
    dragSelectEnabled,
    justDropped,
    activeScope,
    startDrag,
    updateDrag,
    endDrag,
    cancelDrag,
    setDragSelectEnabled,
    setActiveScope,
    registerDropZone,
    unregisterDropZone,
    getDropZones,
    setOnDrop
  }

  return (
    <DragContext.Provider value={value}>{props.children}</DragContext.Provider>
  )
}

export function useDragContext() {
  const context = useContext(DragContext)
  if (!context) {
    throw new Error("useDragContext must be used within a DragProvider")
  }
  return context
}
