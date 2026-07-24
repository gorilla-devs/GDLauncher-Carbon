import { onCleanup } from "solid-js"
import {
  DragType,
  DropTarget,
  useDragContext
} from "@/pages/Library/DragContext"

interface UseDragSourceOptions {
  type: DragType
  getIds: () => number[]
  disabled?: () => boolean
}

interface UseDragSourceResult {
  isDragging: () => boolean
  handlePointerDown: (e: PointerEvent) => void
}

/**
 * Hook for making an element a drag source
 */
export function useDragSource(
  options: UseDragSourceOptions
): UseDragSourceResult {
  const dragContext = useDragContext()

  const isDragging = () => {
    return (
      dragContext.isDragging() &&
      dragContext.dragType() === options.type &&
      dragContext.draggedIds().some((id) => options.getIds().includes(id))
    )
  }

  const handlePointerDown = (e: PointerEvent) => {
    // Only handle left click
    if (e.button !== 0) return

    if (options.disabled?.()) return

    // Prevent text selection
    e.preventDefault()

    const ids = options.getIds()
    if (ids.length === 0) return

    dragContext.startDrag(options.type, ids, e)
  }

  return {
    isDragging,
    handlePointerDown
  }
}

interface UseDropTargetOptions {
  id: string
  getTarget: () => DropTarget | null
  getRect: () => DOMRect | null
  onDrop?: (draggedIds: number[], dragType: DragType) => void
}

interface UseDropTargetResult {
  isOver: () => boolean
  canDrop: () => boolean
}

/**
 * Hook for making an element a drop target
 */
export function useDropTarget(
  options: UseDropTargetOptions
): UseDropTargetResult {
  const dragContext = useDragContext()

  onCleanup(() => {
    dragContext.unregisterDropZone(options.id)
  })

  const isOver = () => {
    const currentTarget = dragContext.dropTarget()
    const ourTarget = options.getTarget()

    if (!currentTarget || !ourTarget) return false

    // Compare targets based on type and relevant properties
    if (currentTarget.type !== ourTarget.type) return false

    switch (currentTarget.type) {
      case "favorites":
        return ourTarget.type === "favorites"
      case "beforeInstance":
        return (
          ourTarget.type === "beforeInstance" &&
          currentTarget.instanceId === ourTarget.instanceId
        )
      case "endOfGroup":
        return (
          ourTarget.type === "endOfGroup" &&
          currentTarget.groupId === ourTarget.groupId
        )
      case "beforeGroup":
        return (
          ourTarget.type === "beforeGroup" &&
          currentTarget.groupId === ourTarget.groupId
        )
      case "endOfGroups":
        return ourTarget.type === "endOfGroups"
      default:
        return false
    }
  }

  const canDrop = () => {
    return dragContext.isDragging()
  }

  return {
    isOver,
    canDrop
  }
}

/**
 * Utility to calculate drop position indicators for instances
 */
export function calculateInstanceDropPosition(
  instances: { id: number; groupId: number }[],
  mouseY: number,
  getInstanceRect: (id: number) => DOMRect | null
): DropTarget | null {
  for (let i = 0; i < instances.length; i++) {
    const instance = instances[i]
    const rect = getInstanceRect(instance.id)

    if (!rect) continue

    const midY = rect.top + rect.height / 2

    // If mouse is above the midpoint, drop before this instance
    if (mouseY < midY) {
      return {
        type: "beforeInstance",
        instanceId: instance.id,
        groupId: instance.groupId
      }
    }
  }

  // If we're past all instances, return end of last group
  if (instances.length > 0) {
    const lastInstance = instances[instances.length - 1]
    return {
      type: "endOfGroup",
      groupId: lastInstance.groupId
    }
  }

  return null
}

/**
 * Utility to calculate drop position indicators for groups
 */
export function calculateGroupDropPosition(
  groups: { id: number }[],
  mouseY: number,
  getGroupRect: (id: number) => DOMRect | null
): DropTarget | null {
  for (let i = 0; i < groups.length; i++) {
    const group = groups[i]
    const rect = getGroupRect(group.id)

    if (!rect) continue

    // Only consider the header area for group reordering
    const headerHeight = 40 // approximate header height
    const headerBottom = rect.top + headerHeight

    if (mouseY < headerBottom) {
      return {
        type: "beforeGroup",
        groupId: group.id
      }
    }
  }

  // If we're past all groups, return end of groups
  return { type: "endOfGroups" }
}
