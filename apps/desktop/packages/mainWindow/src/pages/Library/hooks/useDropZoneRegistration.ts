/**
 * useDropZoneRegistration Hook
 *
 * Extracts all drop zone registration effects from LibraryItemTile.
 * Handles registering/unregistering drop zones during drag operations.
 */

import { Accessor, createEffect, onCleanup } from "solid-js"
import { useDragContext } from "../DragContext"

type DragContextValue = ReturnType<typeof useDragContext>

export interface UseDropZoneRegistrationOptions {
  /** The ID of the item (instance or folder) */
  itemId: number
  /** The type of item */
  itemType: "instance" | "folder"
  /** Ref accessor for the DOM element */
  ref: Accessor<HTMLDivElement | undefined>
  /** Drag context value */
  dragContext: DragContextValue
  /** Default group ID for ungrouped instances */
  defaultGroupId: Accessor<number | null>
  /** ID of the previous item in the list (for detecting no-op drops) */
  previousItemId?: Accessor<number | null>
}

/**
 * Hook for managing drop zone registration during drag operations.
 */
export function useDropZoneRegistration(
  options: UseDropZoneRegistrationOptions
): void {
  const { itemId, itemType, ref, dragContext, defaultGroupId, previousItemId } =
    options
  const isFolder = itemType === "folder"
  const isInstance = itemType === "instance"

  // Register drop zones for folders (group reordering)
  createEffect(() => {
    const el = ref()
    if (!isFolder || !el) return

    if (dragContext.isDragging() && dragContext.dragType() === "group") {
      // Don't register drop zone for the folder being dragged
      if (dragContext.draggedIds().includes(itemId)) {
        dragContext.unregisterDropZone(`before-group-${itemId}`)
        return
      }

      const rect = el.getBoundingClientRect()
      // Register drop zone on left edge
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width * 0.4 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-group-${itemId}`,
        rect: dropRect,
        element: el,
        rectTransform: (r) =>
          new DOMRect(r.left - 8, r.top, r.width * 0.4 + 8, r.height),
        target: { type: "beforeGroup", groupId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-group-${itemId}`)
    }
  })

  // Register drop zone for instance positioning before folders
  createEffect(() => {
    const el = ref()
    if (!isFolder || !el) return

    if (dragContext.isDragging() && dragContext.dragType() === "instance") {
      const rect = el.getBoundingClientRect()
      // Register drop zone on left edge
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width * 0.4 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-instance-at-folder-${itemId}`,
        rect: dropRect,
        element: el,
        rectTransform: (r) =>
          new DOMRect(r.left - 8, r.top, r.width * 0.4 + 8, r.height),
        target: { type: "beforeInstanceAtFolder", folderId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-instance-at-folder-${itemId}`)
    }
  })

  // Register drop zones for ungrouped instances
  createEffect(() => {
    const el = ref()
    if (!isInstance || !el) return

    if (dragContext.isDragging() && dragContext.dragType() === "instance") {
      // Don't register drop zone for dragged instances
      if (dragContext.draggedIds().includes(itemId)) {
        dragContext.unregisterDropZone(`before-instance-${itemId}`)
        dragContext.unregisterDropZone(`create-folder-${itemId}`)
        return
      }

      // Don't register "beforeInstance" zone if the previous item is being dragged
      // (dropping there would be a no-op - the item is already in that position)
      const prevId = previousItemId?.()
      const prevItemIsBeingDragged =
        prevId !== null &&
        prevId !== undefined &&
        dragContext.draggedIds().includes(prevId)

      if (prevItemIsBeingDragged) {
        dragContext.unregisterDropZone(`before-instance-${itemId}`)
      } else {
        const rect = el.getBoundingClientRect()
        // Register "before instance" drop zone (left edge)
        const dropRect = new DOMRect(
          rect.left - 8,
          rect.top,
          rect.width * 0.4 + 8,
          rect.height
        )
        dragContext.registerDropZone({
          id: `before-instance-${itemId}`,
          rect: dropRect,
          element: el,
          rectTransform: (r) =>
            new DOMRect(r.left - 8, r.top, r.width * 0.4 + 8, r.height),
          target: {
            type: "beforeInstance",
            instanceId: itemId,
            groupId: defaultGroupId()!
          }
        })
      }

      // Register "create folder" drop zone (right 60% of tile, excluding beforeInstance zone)
      // This is still valid even if previous item is being dragged
      const rect = el.getBoundingClientRect()
      const createFolderRect = new DOMRect(
        rect.left + rect.width * 0.4, // Start where beforeInstance ends
        rect.top,
        rect.width * 0.6, // Cover remaining 60%
        rect.height
      )
      dragContext.registerDropZone({
        id: `create-folder-${itemId}`,
        rect: createFolderRect,
        element: el,
        rectTransform: (r) =>
          new DOMRect(r.left + r.width * 0.4, r.top, r.width * 0.6, r.height),
        target: { type: "createFolder", instanceId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-instance-${itemId}`)
      dragContext.unregisterDropZone(`create-folder-${itemId}`)
    }
  })

  // Register drop zone for group positioning when dragging groups (at instance position)
  createEffect(() => {
    const el = ref()
    if (!isInstance || !el) return

    if (dragContext.isDragging() && dragContext.dragType() === "group") {
      const rect = el.getBoundingClientRect()
      // Register drop zone on left edge
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width * 0.4 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-group-at-instance-${itemId}`,
        rect: dropRect,
        element: el,
        rectTransform: (r) =>
          new DOMRect(r.left - 8, r.top, r.width * 0.4 + 8, r.height),
        target: { type: "beforeGroupAtInstance", beforeInstanceId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-group-at-instance-${itemId}`)
    }
  })

  // Cleanup all drop zones on unmount
  onCleanup(() => {
    if (isFolder) {
      dragContext.unregisterDropZone(`before-group-${itemId}`)
      dragContext.unregisterDropZone(`before-instance-at-folder-${itemId}`)
    }
    if (isInstance) {
      dragContext.unregisterDropZone(`before-instance-${itemId}`)
      dragContext.unregisterDropZone(`create-folder-${itemId}`)
      dragContext.unregisterDropZone(`before-group-at-instance-${itemId}`)
    }
  })
}
