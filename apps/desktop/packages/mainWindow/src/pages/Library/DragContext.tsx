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
  target: DropTarget | null,
  draggedIds: number[],
  dragType: DragType,
  origin: string | null
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
  dropPreviewRect: Accessor<DOMRect | null>
  dropAnimating: Accessor<{ type: string; targetX: number; targetY: number } | null>
  dragDetached: Accessor<boolean>

  // Actions
  startDrag: (type: DragType, ids: number[], e: PointerEvent, origin?: string) => void
  updateDrag: (e: PointerEvent) => void
  endDrag: () => void
  cancelDrag: () => void
  setDragSelectEnabled: (enabled: boolean) => void
  setActiveScope: (scope: string | null) => void
  setDropPreviewRect: (rect: DOMRect | null) => void

  // Origin tracking
  getDragOrigin: () => string | null

  // Drop zone registration
  registerDropZone: (zone: DropZone) => void
  unregisterDropZone: (id: string) => void
  getDropZones: () => DropZone[]

  // Drop handler
  setOnDrop: (handler: DropHandler | null) => void

  // Layout animation capture
  addLayoutCaptureCallback: (fn: () => void) => void
  removeLayoutCaptureCallback: (fn: () => void) => void
}

const DragContext = createContext<DragContextValue>()

const MIN_DRAG_DISTANCE = 5

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

export function DragProvider(props: { children: JSX.Element }) {
  const [isDragging, setIsDragging] = createSignal(false)
  const [hasDragStarted, setHasDragStarted] = createSignal(false)
  const [dragType, setDragType] = createSignal<DragType | null>(null)
  const [draggedIds, setDraggedIds] = createSignal<number[]>([])
  const [dropTarget, setDropTarget] = createSignal<DropTarget | null>(null, {
    equals: (a, b) => isSameTarget(a, b)
  })
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
  const [dropPreviewRect, setDropPreviewRect] = createSignal<DOMRect | null>(
    null
  )
  const [dropAnimating, setDropAnimating] = createSignal<{
    type: string
    targetX: number
    targetY: number
  } | null>(null)
  const [dragDetached, setDragDetached] = createSignal(false)

  const DETACH_THRESHOLD_SQ = 35 * 35

  // Drop zones registry
  let dropZones: DropZone[] = []
  let onDropHandler: DropHandler | null = null
  let scrollCleanup: (() => void) | null = null
  let dragOrigin: string | null = null

  // Layout animation capture callbacks (multiple grids can register)
  const layoutCaptureCallbacks = new Set<() => void>()

  const addLayoutCaptureCallback = (fn: () => void) => {
    layoutCaptureCallbacks.add(fn)
  }

  const removeLayoutCaptureCallback = (fn: () => void) => {
    layoutCaptureCallbacks.delete(fn)
  }

  // Position-based stabilization: reject target changes when cursor hasn't moved
  let lastStableTarget: DropTarget | null = null
  let lastStablePosition: { x: number; y: number } | null = null
  const POSITION_THRESHOLD = 4

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
    // Force rect refresh on next findDropTarget — layout may have changed
    lastRefreshTime = 0
  }

  const unregisterDropZone = (id: string) => {
    dropZones = dropZones.filter((z) => z.id !== id)
    // Force rect refresh — removing a zone (e.g. DropPreviewTile unmount)
    // causes grid reflow, so cached rects become stale.
    lastRefreshTime = 0
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
    // Always include favorites zone so it's reachable from any context (e.g., inside folders)
    const scope = activeScope()
    const scopedZones =
      scope !== null
        ? dropZones.filter((z) => z.scope === scope || z.target.type === "favorites")
        : dropZones

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

  const startDrag = (type: DragType, ids: number[], e: PointerEvent, origin?: string) => {
    dragOrigin = origin ?? null
    setDragType(type)
    setDraggedIds(ids)
    setStartPosition({ x: e.clientX, y: e.clientY })
    setGhostPosition({ x: e.clientX, y: e.clientY })
    setHasDragStarted(false)
    // Disable drag-select immediately to prevent selection starting before threshold
    setDragSelectEnabled(false)

    // Add document-level listeners
    document.addEventListener("pointermove", handlePointerMove)
    document.addEventListener("pointerup", handlePointerUp)
    document.addEventListener("keydown", handleKeyDown)

    const handleScroll = () => {
      if (hasDragStarted() && dragDetached()) {
        const pos = ghostPosition()
        const target = findDropTarget(pos.x, pos.y)
        const resolved = resolveDropTarget(target, pos.x, pos.y, true)
        if (!isSameTarget(resolved, dropTarget())) {
          layoutCaptureCallbacks.forEach((fn) => fn())
        }
        setDropTarget(resolved)
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

      // Latch dragDetached once cursor moves far enough from start
      if (!dragDetached() && start) {
        const ddx = e.clientX - start.x
        const ddy = e.clientY - start.y
        if (ddx * ddx + ddy * ddy > DETACH_THRESHOLD_SQ) {
          setDragDetached(true)
        }
      }

      // Find drop target only after detach — suppresses all visual changes
      // (ghost, tile hiding, DropPreviewTile, grid reflow) until the tile
      // has been dragged far enough from its original position.
      if (dragDetached()) {
        const target = findDropTarget(e.clientX, e.clientY)
        const resolved = resolveDropTarget(target, e.clientX, e.clientY)
        if (!isSameTarget(resolved, dropTarget())) {
          layoutCaptureCallbacks.forEach((fn) => fn())
        }
        setDropTarget(resolved)
      }
    }
  }

  const executeDrop = () => {
    cleanup()

    if (hasDragStarted()) {
      setJustDropped(true)

      // Use the current dropTarget — this matches what the preview was showing.
      // Don't recalculate, as forceAccept would bypass hysteresis and could
      // resolve a different target than the preview displayed.
      const target = dropTarget()
      const ids = draggedIds()
      const type = dragType()

      if (ids.length > 0 && type && onDropHandler) {
        onDropHandler(target, ids, type, dragOrigin)
      }

      // Animated exit for favorites unfavorite — keep ghost alive during animation
      const isFavUnfavorite =
        dragOrigin === "favorites" &&
        (target === null || target.type !== "favorites")

      if (isFavUnfavorite) {
        // Find the library content area to target the center of the overlay
        const contentEl = document.querySelector<HTMLElement>(
          "[style*='view-transition-name: library-content']"
        )
        const contentRect = contentEl?.getBoundingClientRect()
        const targetX = contentRect
          ? contentRect.left + contentRect.width / 2
          : window.innerWidth / 2
        const targetY = contentRect
          ? contentRect.top + contentRect.height / 2
          : window.innerHeight / 2

        setDropAnimating({ type: "unfavorite", targetX, targetY })
        setHasDragStarted(false)
        setStartPosition(null)
        setDragSelectEnabled(true)
        resetDropTargetHysteresis()
        // isDragging, dragType, draggedIds, dragOrigin stay alive for the ghost
        setTimeout(() => {
          setDropAnimating(null)
          setIsDragging(false)
          setDragDetached(false)
          setDragType(null)
          setDraggedIds([])
          setDropTarget(null)
          setDropPreviewRect(null)
          dragOrigin = null
          requestAnimationFrame(() => {
            setJustDropped(false)
          })
        }, 300)
        return
      }

      // Animate ghost to drop preview position if available
      const previewRect = dropPreviewRect()
      if (previewRect) {
        const targetX = previewRect.left + previewRect.width / 2
        const targetY = previewRect.top + previewRect.height / 2

        setDropAnimating({ type: "settle", targetX, targetY })
        setHasDragStarted(false)
        setStartPosition(null)
        setDragSelectEnabled(true)
        resetDropTargetHysteresis()
        // isDragging, dragType, draggedIds stay alive for the ghost
        setTimeout(() => {
          setDropAnimating(null)
          setIsDragging(false)
          setDragDetached(false)
          setDragType(null)
          setDraggedIds([])
          setDropTarget(null)
          setDropPreviewRect(null)
          dragOrigin = null
          requestAnimationFrame(() => {
            setJustDropped(false)
          })
        }, 300)
        return
      }

      // No preview rect (e.g. drop on favorites, null target) — clear immediately
      setIsDragging(false)
      setTimeout(() => {
        if (!isDragging()) {
          setDragType(null)
          setDraggedIds([])
          setDropTarget(null)
          setDropPreviewRect(null)
        }
        requestAnimationFrame(() => {
          setJustDropped(false)
        })
      }, 100)
    } else {
      // No drag started — clear immediately
      setDragType(null)
      setDraggedIds([])
      setDropTarget(null)
      setDropPreviewRect(null)
    }

    dragOrigin = null
    setHasDragStarted(false)
    setDragDetached(false)
    setStartPosition(null)
    setDragSelectEnabled(true)
    resetDropTargetHysteresis()
  }

  const handlePointerUp = (_e: PointerEvent) => {
    executeDrop()
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
    executeDrop()
  }

  const cancelDrag = () => {
    cleanup()
    dragOrigin = null
    setIsDragging(false)
    setHasDragStarted(false)
    setDragDetached(false)
    setDragType(null)
    setDraggedIds([])
    setDropTarget(null)
    setDropPreviewRect(null)
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
    dropPreviewRect,
    dropAnimating,
    dragDetached,
    startDrag,
    updateDrag,
    endDrag,
    cancelDrag,
    setDragSelectEnabled,
    setActiveScope,
    setDropPreviewRect,
    getDragOrigin: () => dragOrigin,
    registerDropZone,
    unregisterDropZone,
    getDropZones,
    setOnDrop,
    addLayoutCaptureCallback,
    removeLayoutCaptureCallback
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
