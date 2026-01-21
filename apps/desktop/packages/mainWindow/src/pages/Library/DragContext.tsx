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
  | { type: "dropOnFolder"; groupId: number }        // Drop on collapsed folder
  | { type: "createFolder"; instanceId: number }     // Drop on ungrouped instance to create folder
  | { type: "ungrouped" }                            // Return to default group (main grid)

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
  target: DropTarget
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

  // Actions
  startDrag: (type: DragType, ids: number[], e: PointerEvent) => void
  updateDrag: (e: PointerEvent) => void
  endDrag: () => void
  cancelDrag: () => void
  setDragSelectEnabled: (enabled: boolean) => void

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

  // Drop zones registry
  let dropZones: DropZone[] = []
  let onDropHandler: DropHandler | null = null

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

  const findDropTarget = (x: number, y: number): DropTarget | null => {
    // Sort drop zones by priority (favorites first, then instances, then groups)
    const sortedZones = [...dropZones].sort((a, b) => {
      const priority: Record<DropTarget["type"], number> = {
        favorites: 0,
        createFolder: 1,      // Higher priority than beforeInstance (center of tile)
        beforeInstance: 2,
        dropOnFolder: 3,
        endOfGroup: 4,
        beforeGroup: 5,
        endOfGroups: 6,
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
  }

  const handlePointerMove = (e: PointerEvent) => {
    const start = startPosition()
    if (!start) return

    const dx = Math.abs(e.clientX - start.x)
    const dy = Math.abs(e.clientY - start.y)

    // Check if we've moved enough to start dragging
    if (!hasDragStarted() && (dx >= MIN_DRAG_DISTANCE || dy >= MIN_DRAG_DISTANCE)) {
      setHasDragStarted(true)
      setIsDragging(true)
      setDragSelectEnabled(false)
    }

    if (hasDragStarted()) {
      setGhostPosition({ x: e.clientX, y: e.clientY })

      // Find drop target
      const target = findDropTarget(e.clientX, e.clientY)
      setDropTarget(target)
    }
  }

  const handlePointerUp = () => {
    cleanup()

    if (hasDragStarted()) {
      // Set justDropped flag to prevent click event from firing
      setJustDropped(true)
      // Clear after a brief delay (click event fires ~0-50ms after pointerup)
      setTimeout(() => setJustDropped(false), 100)

      // Call drop handler before resetting state
      const target = dropTarget()
      const ids = draggedIds()
      const type = dragType()

      if (target && ids.length > 0 && type && onDropHandler) {
        onDropHandler(target, ids, type)
      }

      // Drag completed
      setIsDragging(false)
    }

    // Reset state
    setHasDragStarted(false)
    setDragType(null)
    setDraggedIds([])
    setDropTarget(null)
    setStartPosition(null)
    setDragSelectEnabled(true)
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
    startDrag,
    updateDrag,
    endDrag,
    cancelDrag,
    setDragSelectEnabled,
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
