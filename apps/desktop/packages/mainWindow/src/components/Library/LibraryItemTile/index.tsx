import { createEffect, createMemo, onCleanup, onMount, Show } from "solid-js"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance } from "@gd/core_module/bindings"
import InstanceTile from "@/components/InstanceTile"
import FolderTile from "@/components/Library/FolderTile"
import { useGlobalStore } from "@/components/GlobalStoreContext"

export type LibraryItem =
  | { id: string; type: "instance"; data: ListInstance }
  | {
      id: string
      type: "folder"
      data: {
        id: number
        name: string
        libraryPosition: number | null
        instances: ListInstance[]
      }
    }

interface LibraryItemTileProps {
  item: LibraryItem
  itemIndex: () => number
  instancesTileSize: () => number
  defaultGroupId: () => number | null
  openFolderId: () => number | null
  toggleFolder: (id: number) => void
  isSelected: (id: number) => boolean
  toggleSelection: (id: number) => void
  selectedIds: () => Set<number>
  onDragStart: (type: "instance" | "group", ids: number[], e: PointerEvent) => void
  justDropped: () => boolean
  tileRefs: Map<number, HTMLDivElement>
  libraryItemRefs: Map<string, HTMLDivElement>
  animatedLibraryItemIds: Set<string>
  libraryInitialAnimationComplete: { value: boolean }
  libraryItemsLength: () => number
}

const LibraryItemTile = (props: LibraryItemTileProps) => {
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  let ref: HTMLDivElement | undefined

  // Item identity - these never change for a given item
  const isFolder = props.item.type === "folder"
  const isInstance = props.item.type === "instance"

  // Use the top-level id for animation tracking (matches reconcile key)
  const itemKey = props.item.id

  const itemId = props.item.data.id

  // Type-safe data accessors
  const instanceData = () =>
    isInstance
      ? (props.item as { type: "instance"; data: ListInstance }).data
      : null

  // Drop indicator logic for folders
  const showFolderDropIndicator = createMemo(() => {
    if (!isFolder) return false
    const target = dragContext.dropTarget()
    return (
      dragContext.isDragging() &&
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
      dragContext.isDragging() &&
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
      dragContext.isDragging() &&
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
      dragContext.isDragging() &&
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
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      target?.type === "createFolder" &&
      target.instanceId === itemId
    )
  })

  const isBeingDragged = createMemo(() => {
    if (!isInstance) return false
    return (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      dragContext.draggedIds().includes(itemId)
    )
  })

  // Combined drop indicator for the visual element
  const showDropIndicator = createMemo(
    () =>
      showFolderDropIndicator() ||
      showInstanceAtFolderDropIndicator() ||
      showInstanceDropIndicator() ||
      showGroupDropIndicator()
  )

  // Register drop zones for folders (group reordering)
  createEffect(() => {
    if (!isFolder || !ref) return

    if (dragContext.isDragging() && dragContext.dragType() === "group") {
      // Don't register drop zone for the folder being dragged
      if (dragContext.draggedIds().includes(itemId)) {
        dragContext.unregisterDropZone(`before-group-${itemId}`)
        return
      }

      const rect = ref.getBoundingClientRect()
      // Register drop zone on left edge
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width / 3 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-group-${itemId}`,
        rect: dropRect,
        target: { type: "beforeGroup", groupId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-group-${itemId}`)
    }
  })

  // Register drop zone for instance positioning before folders
  createEffect(() => {
    if (!isFolder || !ref) return

    if (dragContext.isDragging() && dragContext.dragType() === "instance") {
      const rect = ref.getBoundingClientRect()
      // Register drop zone on left edge
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width / 3 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-instance-at-folder-${itemId}`,
        rect: dropRect,
        target: { type: "beforeInstanceAtFolder", folderId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-instance-at-folder-${itemId}`)
    }
  })

  // Register drop zones for ungrouped instances
  createEffect(() => {
    if (!isInstance || !ref) return

    if (dragContext.isDragging() && dragContext.dragType() === "instance") {
      // Don't register drop zone for dragged instances
      if (dragContext.draggedIds().includes(itemId)) {
        dragContext.unregisterDropZone(`before-instance-${itemId}`)
        dragContext.unregisterDropZone(`create-folder-${itemId}`)
        return
      }

      const rect = ref.getBoundingClientRect()

      // Register "before instance" drop zone (left edge)
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width / 4 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-instance-${itemId}`,
        rect: dropRect,
        target: {
          type: "beforeInstance",
          instanceId: itemId,
          groupId: props.defaultGroupId()!
        }
      })

      // Register "create folder" drop zone (right 75% of tile, excluding beforeInstance zone)
      const createFolderRect = new DOMRect(
        rect.left + rect.width * 0.25, // Start where beforeInstance ends
        rect.top,
        rect.width * 0.75, // Cover remaining 75%
        rect.height
      )
      dragContext.registerDropZone({
        id: `create-folder-${itemId}`,
        rect: createFolderRect,
        target: { type: "createFolder", instanceId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-instance-${itemId}`)
      dragContext.unregisterDropZone(`create-folder-${itemId}`)
    }
  })

  // Register drop zone for group positioning when dragging groups (at instance position)
  createEffect(() => {
    if (!isInstance || !ref) return

    if (dragContext.isDragging() && dragContext.dragType() === "group") {
      const rect = ref.getBoundingClientRect()
      // Register drop zone on left edge
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width / 3 + 8,
        rect.height
      )
      dragContext.registerDropZone({
        id: `before-group-at-instance-${itemId}`,
        rect: dropRect,
        target: { type: "beforeGroupAtInstance", beforeInstanceId: itemId }
      })
    } else {
      dragContext.unregisterDropZone(`before-group-at-instance-${itemId}`)
    }
  })

  // Cleanup
  onCleanup(() => {
    if (isFolder) {
      dragContext.unregisterDropZone(`before-group-${itemId}`)
      dragContext.unregisterDropZone(`before-instance-at-folder-${itemId}`)
    }
    if (isInstance) {
      dragContext.unregisterDropZone(`before-instance-${itemId}`)
      dragContext.unregisterDropZone(`create-folder-${itemId}`)
      dragContext.unregisterDropZone(`before-group-at-instance-${itemId}`)
      props.tileRefs.delete(itemId)
    }
  })

  // Staggered appear animation
  onMount(() => {
    // Register ref for FLIP animation
    if (ref) {
      props.libraryItemRefs.set(itemKey, ref)
    }

    const shouldAnimate =
      !globalStore.settings.data?.reducedMotion &&
      !props.animatedLibraryItemIds.has(itemKey) &&
      !props.libraryInitialAnimationComplete.value

    if (ref && shouldAnimate) {
      props.animatedLibraryItemIds.add(itemKey)
      const delay = 100 + props.itemIndex() * 40
      ref.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: 250,
        delay,
        easing: "linear",
        fill: "forwards"
      })
    }

    if (isInstance && ref) {
      props.tileRefs.set(itemId, ref)
    }

    // Mark initial animation complete on last item
    if (props.itemIndex() === props.libraryItemsLength() - 1) {
      requestAnimationFrame(() => {
        props.libraryInitialAnimationComplete.value = true
      })
    }
  })

  // Cleanup refs
  onCleanup(() => {
    props.libraryItemRefs.delete(itemKey)
  })

  return (
    <div
      ref={ref}
      data-library-item
      data-instance-tile={isInstance || undefined}
      class="relative"
      classList={{
        "opacity-0":
          !globalStore.settings.data?.reducedMotion &&
          !props.animatedLibraryItemIds.has(itemKey) &&
          !props.libraryInitialAnimationComplete.value
      }}
      onClick={(e) => e.stopPropagation()}
    >
      {/* Drop indicator */}
      <Show when={showDropIndicator()}>
        <div class="absolute -left-2.5 top-0 bottom-0 w-1.5 z-50 flex flex-col items-center">
          {/* Top cap */}
          <div class="w-3 h-3 rounded-full bg-primary-500 -mt-1.5 shadow-lg shadow-primary-500/50" />
          {/* Line with glow */}
          <div class="flex-1 w-1 bg-gradient-to-b from-primary-500 via-primary-400 to-primary-500 rounded-full shadow-lg shadow-primary-500/40" />
          {/* Bottom cap */}
          <div class="w-3 h-3 rounded-full bg-primary-500 -mb-1.5 shadow-lg shadow-primary-500/50" />
        </div>
      </Show>

      {/* Create folder indicator */}
      <Show when={showCreateFolderIndicator()}>
        <div class="absolute inset-0 border-2 border-primary-500 rounded-lg bg-primary-500/20 pointer-events-none z-40 flex items-center justify-center">
          <div class="i-hugeicons:folder-add text-primary-400 text-2xl" />
        </div>
      </Show>

      {/* Folder rendering - condition never changes for a given item */}
      <Show when={isFolder}>
        <FolderTile
          groupId={itemId}
          isOpen={props.openFolderId() === itemId}
          onToggle={() => props.toggleFolder(itemId)}
          size={props.instancesTileSize() as 1 | 2 | 3 | 4 | 5}
        />
      </Show>

      {/* Instance rendering - condition never changes for a given item */}
      <Show when={isInstance && instanceData()}>
        {(instance) => (
          <InstanceTile
            instance={instance()}
            identifier={`ungrouped-${itemId}`}
            size={props.instancesTileSize() as 1 | 2 | 3 | 4 | 5}
            isMultiSelected={props.isSelected(itemId)}
            onToggleSelection={() => props.toggleSelection(itemId)}
            isDragging={isBeingDragged()}
            isDragActive={dragContext.isDragging()}
            groupId={props.defaultGroupId() ?? undefined}
            onDragStart={(e) => {
              const ids = props.isSelected(itemId)
                ? Array.from(props.selectedIds())
                : [itemId]
              props.onDragStart("instance", ids, e)
            }}
            preventClick={() => props.justDropped()}
          />
        )}
      </Show>
    </div>
  )
}

export default LibraryItemTile
