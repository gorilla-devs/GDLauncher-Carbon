/**
 * useDropIndicators Hook
 *
 * Extracts all drop indicator memos from LibraryItemTile.
 * Provides computed state for showing various drop indicators during drag operations.
 */

import { Accessor, createMemo } from "solid-js"
import { DropTarget, useDragContext } from "../DragContext"

type DragContextValue = ReturnType<typeof useDragContext>

export interface UseDropIndicatorsOptions {
  /** The ID of the item (instance or folder) */
  itemId: number
  /** The type of item */
  itemType: "instance" | "folder"
  /** Drag context value */
  dragContext: DragContextValue
}

export interface DropIndicatorState {
  /** Show folder drop indicator (group before another group) */
  showFolderDropIndicator: Accessor<boolean>
  /** Show instance drop indicator before a folder */
  showInstanceAtFolderDropIndicator: Accessor<boolean>
  /** Show instance drop indicator (instance before another instance) */
  showInstanceDropIndicator: Accessor<boolean>
  /** Show group drop indicator at instance position */
  showGroupDropIndicator: Accessor<boolean>
  /** Show create folder indicator when hovering to create folder */
  showCreateFolderIndicator: Accessor<boolean>
  /** Whether this instance is currently being dragged */
  isBeingDragged: Accessor<boolean>
  /** Whether this item (instance or folder) is being dragged */
  isItemBeingDragged: Accessor<boolean>
  /** Whether to collapse the tile's space when dragged */
  shouldCollapseTile: Accessor<boolean>
  /** The active drop target for this item, if any */
  activeDropTarget: Accessor<DropTarget | null>
  /** Combined indicator: whether any drop indicator should show */
  showDropIndicator: Accessor<boolean>
}

/**
 * Hook for computing drop indicator visibility during drag operations.
 */
export function useDropIndicators(
  options: UseDropIndicatorsOptions
): DropIndicatorState {
  const { itemId, itemType, dragContext } = options
  const isFolder = itemType === "folder"
  const isInstance = itemType === "instance"

  const isDragActive = () =>
    dragContext.isDragging() || dragContext.justDropped()

  // Drop indicator logic for folders
  const showFolderDropIndicator = createMemo(() => {
    if (!isFolder) return false
    const target = dragContext.dropTarget()
    return (
      isDragActive() &&
      dragContext.dragType() === "group" &&
      target?.type === "beforeGroup" &&
      target.groupId === itemId
    )
  })

  // Drop indicator for instances before folders
  const showInstanceAtFolderDropIndicator = createMemo(() => {
    if (!isFolder) return false
    const target = dragContext.dropTarget()
    return (
      isDragActive() &&
      dragContext.dragType() === "instance" &&
      target?.type === "beforeInstanceAtFolder" &&
      target.folderId === itemId
    )
  })

  // Drop indicator logic for instances
  const showInstanceDropIndicator = createMemo(() => {
    if (!isInstance) return false
    const target = dragContext.dropTarget()
    return (
      isDragActive() &&
      dragContext.dragType() === "instance" &&
      target?.type === "beforeInstance" &&
      target.instanceId === itemId
    )
  })

  // Group drop indicator at instance position
  const showGroupDropIndicator = createMemo(() => {
    if (!isInstance) return false
    const target = dragContext.dropTarget()
    return (
      isDragActive() &&
      dragContext.dragType() === "group" &&
      target?.type === "beforeGroupAtInstance" &&
      target.beforeInstanceId === itemId
    )
  })

  // Create folder indicator
  const showCreateFolderIndicator = createMemo(() => {
    if (!isInstance) return false
    const target = dragContext.dropTarget()
    return (
      isDragActive() &&
      dragContext.dragType() === "instance" &&
      target?.type === "createFolder" &&
      target.instanceId === itemId
    )
  })

  const isBeingDragged = createMemo(() => {
    if (!isInstance) return false
    return (
      isDragActive() &&
      dragContext.dragDetached() &&
      dragContext.dragType() === "instance" &&
      dragContext.draggedIds().includes(itemId)
    )
  })

  // Check if THIS item (instance OR folder) is being dragged
  // Only true once cursor has moved far enough from start (dragDetached latch)
  const isItemBeingDragged = createMemo(() => {
    if (!isDragActive() || !dragContext.dragDetached()) return false

    if (isInstance) {
      return (
        dragContext.dragType() === "instance" &&
        dragContext.draggedIds().includes(itemId)
      )
    }

    if (isFolder) {
      return (
        dragContext.dragType() === "group" &&
        dragContext.draggedIds().includes(itemId)
      )
    }

    return false
  })

  // Collapse the tile's space when being dragged
  const shouldCollapseTile = createMemo(() => isItemBeingDragged())

  // Compute the active drop target (if any) for this item
  const activeDropTarget = createMemo((): DropTarget | null => {
    const target = dragContext.dropTarget()
    if (!target) return null

    if (showFolderDropIndicator())
      return { type: "beforeGroup" as const, groupId: itemId }
    if (showInstanceAtFolderDropIndicator())
      return { type: "beforeInstanceAtFolder" as const, folderId: itemId }
    if (showInstanceDropIndicator()) return target
    if (showGroupDropIndicator())
      return {
        type: "beforeGroupAtInstance" as const,
        beforeInstanceId: itemId
      }

    return null
  })

  // Combined drop indicator for the visual element
  const showDropIndicator = createMemo(() => activeDropTarget() !== null)

  return {
    showFolderDropIndicator,
    showInstanceAtFolderDropIndicator,
    showInstanceDropIndicator,
    showGroupDropIndicator,
    showCreateFolderIndicator,
    isBeingDragged,
    isItemBeingDragged,
    shouldCollapseTile,
    activeDropTarget,
    showDropIndicator
  }
}
