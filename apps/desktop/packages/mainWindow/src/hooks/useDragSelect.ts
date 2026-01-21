import { createSignal, onCleanup } from "solid-js"

interface DragState {
  isDragging: boolean
  startX: number
  startY: number
  currentX: number
  currentY: number
}

export interface SelectionRect {
  left: number
  top: number
  width: number
  height: number
}

interface UseDragSelectOptions {
  containerRef: () => HTMLElement | undefined
  onSelectionChange: (selectedIds: number[]) => void
  getItemRects: () => Map<number, DOMRect>
  minDragDistance?: number
}

export function useDragSelect(options: UseDragSelectOptions) {
  const minDragDistance = options.minDragDistance ?? 5
  const [dragState, setDragState] = createSignal<DragState | null>(null)
  const [hasMovedEnough, setHasMovedEnough] = createSignal(false)

  const selectionRect = (): SelectionRect | null => {
    const state = dragState()
    if (!state || !state.isDragging || !hasMovedEnough()) return null

    return {
      left: Math.min(state.startX, state.currentX),
      top: Math.min(state.startY, state.currentY),
      width: Math.abs(state.currentX - state.startX),
      height: Math.abs(state.currentY - state.startY)
    }
  }

  const rectsIntersect = (a: SelectionRect, b: DOMRect): boolean => {
    return !(
      a.left > b.right ||
      a.left + a.width < b.left ||
      a.top > b.bottom ||
      a.top + a.height < b.top
    )
  }

  const getSelectedIds = (rect: SelectionRect): number[] => {
    const itemRects = options.getItemRects()
    const selected: number[] = []

    itemRects.forEach((itemRect, id) => {
      if (rectsIntersect(rect, itemRect)) {
        selected.push(id)
      }
    })

    return selected
  }

  const handleMouseDown = (e: MouseEvent) => {
    e.preventDefault()

    setDragState({
      isDragging: true,
      startX: e.clientX,
      startY: e.clientY,
      currentX: e.clientX,
      currentY: e.clientY
    })
    setHasMovedEnough(false)

    document.addEventListener("mousemove", handleMouseMove)
    document.addEventListener("mouseup", handleMouseUp)
  }

  const handleMouseMove = (e: MouseEvent) => {
    const state = dragState()
    if (!state || !state.isDragging) return

    const dx = Math.abs(e.clientX - state.startX)
    const dy = Math.abs(e.clientY - state.startY)

    if (!hasMovedEnough() && (dx >= minDragDistance || dy >= minDragDistance)) {
      setHasMovedEnough(true)
    }

    setDragState({
      ...state,
      currentX: e.clientX,
      currentY: e.clientY
    })

    if (hasMovedEnough()) {
      const rect = selectionRect()
      if (rect) {
        const selectedIds = getSelectedIds(rect)
        options.onSelectionChange(selectedIds)
      }
    }
  }

  const handleMouseUp = () => {
    const state = dragState()
    const movedEnough = hasMovedEnough()

    document.removeEventListener("mousemove", handleMouseMove)
    document.removeEventListener("mouseup", handleMouseUp)

    if (state && movedEnough) {
      const rect = selectionRect()
      if (rect) {
        const selectedIds = getSelectedIds(rect)
        options.onSelectionChange(selectedIds)
      }
    } else if (state && !movedEnough) {
      options.onSelectionChange([])
    }

    setDragState(null)
    setHasMovedEnough(false)
  }

  onCleanup(() => {
    document.removeEventListener("mousemove", handleMouseMove)
    document.removeEventListener("mouseup", handleMouseUp)
  })

  return {
    dragState,
    selectionRect,
    hasMovedEnough,
    handlers: {
      handleMouseDown
    }
  }
}
