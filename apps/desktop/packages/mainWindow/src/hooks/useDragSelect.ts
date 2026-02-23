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
  onSelectionChange: (selectedIds: string[]) => void
  getItemRects: () => Map<string, DOMRect>
  getExistingSelection?: () => Set<string>
  minDragDistance?: number
}

const AUTO_SCROLL_EDGE_PX = 40
const AUTO_SCROLL_MAX_SPEED = 15

export function useDragSelect(options: UseDragSelectOptions) {
  const minDragDistance = options.minDragDistance ?? 5
  const [dragState, setDragState] = createSignal<DragState | null>(null)
  const [hasMovedEnough, setHasMovedEnough] = createSignal(false)

  // Shift-additive state
  let shiftHeld = false
  let baseSelection = new Set<string>()

  // Auto-scroll state
  let startScrollTop = 0
  let autoScrollRAF: number | null = null
  let lastMouseY = 0

  const resolveScrollContainer = (): HTMLElement | null => {
    const explicit = options.containerRef()
    if (explicit) return explicit
    return document.getElementById("gdl-content-wrapper")
  }

  const computeSelectionRect = (
    state: DragState,
    scrollDelta: number
  ): SelectionRect | null => {
    if (!state.isDragging) return null

    // Adjust startY by scroll delta so the logical rect grows as user scrolls
    const adjustedStartY = state.startY - scrollDelta

    return {
      left: Math.min(state.startX, state.currentX),
      top: Math.min(adjustedStartY, state.currentY),
      width: Math.abs(state.currentX - state.startX),
      height: Math.abs(state.currentY - adjustedStartY)
    }
  }

  const selectionRect = (): SelectionRect | null => {
    const state = dragState()
    if (!state || !state.isDragging || !hasMovedEnough()) return null

    const container = resolveScrollContainer()
    const scrollDelta = container ? container.scrollTop - startScrollTop : 0

    const raw = computeSelectionRect(state, scrollDelta)
    if (!raw) return null

    // Clip to container bounds so the rect and hit-testing stay inside
    if (container) {
      const bounds = container.getBoundingClientRect()
      const left = Math.max(raw.left, bounds.left)
      const top = Math.max(raw.top, bounds.top)
      const right = Math.min(raw.left + raw.width, bounds.right)
      const bottom = Math.min(raw.top + raw.height, bounds.bottom)

      if (right <= left || bottom <= top) return null

      return { left, top, width: right - left, height: bottom - top }
    }

    return raw
  }

  const rectsIntersect = (a: SelectionRect, b: DOMRect): boolean => {
    return !(
      a.left > b.right ||
      a.left + a.width < b.left ||
      a.top > b.bottom ||
      a.top + a.height < b.top
    )
  }

  const getSelectedIds = (rect: SelectionRect): string[] => {
    const itemRects = options.getItemRects()
    const dragSelected: string[] = []

    itemRects.forEach((itemRect, id) => {
      if (rectsIntersect(rect, itemRect)) {
        dragSelected.push(id)
      }
    })

    // Union with base selection when shift was held
    if (baseSelection.size > 0) {
      const merged = new Set(baseSelection)
      for (const id of dragSelected) {
        merged.add(id)
      }
      return Array.from(merged)
    }

    return dragSelected
  }

  const updateSelection = () => {
    const rect = selectionRect()
    if (rect) {
      const selectedIds = getSelectedIds(rect)
      options.onSelectionChange(selectedIds)
    }
  }

  const stopAutoScroll = () => {
    if (autoScrollRAF !== null) {
      cancelAnimationFrame(autoScrollRAF)
      autoScrollRAF = null
    }
  }

  const autoScrollLoop = () => {
    const container = resolveScrollContainer()
    if (!container) {
      autoScrollRAF = null
      return
    }

    const containerRect = container.getBoundingClientRect()
    const mouseY = lastMouseY

    let scrollAmount = 0

    if (mouseY < containerRect.top + AUTO_SCROLL_EDGE_PX) {
      // Near or above top edge — scroll up
      const distance = containerRect.top + AUTO_SCROLL_EDGE_PX - mouseY
      scrollAmount = -Math.min(
        Math.ceil(distance / AUTO_SCROLL_EDGE_PX * AUTO_SCROLL_MAX_SPEED),
        AUTO_SCROLL_MAX_SPEED
      )
    } else if (mouseY > containerRect.bottom - AUTO_SCROLL_EDGE_PX) {
      // Near or below bottom edge — scroll down
      const distance = mouseY - (containerRect.bottom - AUTO_SCROLL_EDGE_PX)
      scrollAmount = Math.min(
        Math.ceil(distance / AUTO_SCROLL_EDGE_PX * AUTO_SCROLL_MAX_SPEED),
        AUTO_SCROLL_MAX_SPEED
      )
    }

    if (scrollAmount !== 0) {
      container.scrollTop += scrollAmount
      // Re-evaluate selection after scrolling
      updateSelection()
    }

    autoScrollRAF = requestAnimationFrame(autoScrollLoop)
  }

  const startAutoScroll = () => {
    if (autoScrollRAF !== null) return
    autoScrollRAF = requestAnimationFrame(autoScrollLoop)
  }

  const handleMouseDown = (e: MouseEvent) => {
    e.preventDefault()

    // Snapshot shift state and existing selection
    shiftHeld = e.shiftKey
    if (shiftHeld && options.getExistingSelection) {
      baseSelection = new Set(options.getExistingSelection())
    } else {
      baseSelection = new Set<string>()
    }

    // Record scroll position at drag start
    const container = resolveScrollContainer()
    startScrollTop = container ? container.scrollTop : 0

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

    lastMouseY = e.clientY

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
      updateSelection()
      startAutoScroll()
    }
  }

  const handleMouseUp = () => {
    const state = dragState()
    const movedEnough = hasMovedEnough()

    document.removeEventListener("mousemove", handleMouseMove)
    document.removeEventListener("mouseup", handleMouseUp)
    stopAutoScroll()

    if (state && movedEnough) {
      updateSelection()
    } else if (state && !movedEnough) {
      // Click without drag
      if (shiftHeld) {
        // Shift+click without drag: preserve existing selection
      } else {
        options.onSelectionChange([])
      }
    }

    setDragState(null)
    setHasMovedEnough(false)
    shiftHeld = false
    baseSelection = new Set<string>()
  }

  onCleanup(() => {
    document.removeEventListener("mousemove", handleMouseMove)
    document.removeEventListener("mouseup", handleMouseUp)
    stopAutoScroll()
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
