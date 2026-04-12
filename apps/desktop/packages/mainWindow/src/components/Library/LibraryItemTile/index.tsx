import { Accessor, Show } from "solid-js"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance, ListServer } from "@gd/core_module/bindings"
import InstanceTile from "@/components/InstanceTile"
import ServerTile from "@/components/Server/Tile"
import FolderTile from "@/components/Library/FolderTile"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { LibraryItem } from "@/pages/Library/types"
import DropPreviewTile from "@/pages/Library/components/DropPreviewTile"
import {
  useDropIndicators,
  useDropZoneRegistration,
  useLibraryItemAnimation
} from "@/pages/Library/hooks"
import { parseInstanceIds, parseServerIds } from "@/pages/Library/utils/selectionIds"
import { DropOverlayIndicator } from "@/pages/Library/components/DropOverlayIndicator"

interface LibraryItemTileProps {
  item: LibraryItem
  itemIndex: () => number
  tileSize: () => number
  defaultGroupId: () => number | null
  openFolderId: () => number | null
  toggleFolder: (id: number) => void
  isSelected: (id: string) => boolean
  toggleSelection: (id: string) => void
  selectedIds: () => Set<string>
  onDragStart: (
    type: "instance" | "group" | "server" | "serverGroup",
    ids: number[],
    e: PointerEvent
  ) => void
  justDropped: () => boolean
  tileRefs: Map<string, HTMLDivElement>
  libraryItemRefs: Map<string, HTMLDivElement>
  animatedLibraryItemIds: Set<string>
  libraryInitialAnimationComplete: { value: boolean }
  libraryItemsLength: () => number
  newlyCreatedFolderId?: Accessor<number | null>
  clearNewlyCreatedFolderId?: () => void
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: (id: string) => void
}

const LibraryItemTile = (props: LibraryItemTileProps) => {
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  let ref: HTMLDivElement | undefined

  // Item identity - these never change for a given item
  const isFolder = props.item.type === "folder"
  const isInstance = props.item.type === "instance"
  const isServer = props.item.type === "server"
  const itemKey = props.item.id
  const itemId = props.item.data.id

  // Type-safe data accessors
  const instanceData = () =>
    isInstance
      ? (props.item as { type: "instance"; data: ListInstance }).data
      : null

  const serverData = () =>
    isServer
      ? (props.item as { type: "server"; data: ListServer }).data
      : null

  const itemType = isFolder ? "folder" as const : isServer ? "server" as const : "instance" as const

  // Extracted hooks for drop indicators
  const dropIndicators = useDropIndicators({
    itemId,
    itemType,
    dragContext
  })

  // Extracted hook for drop zone registration
  useDropZoneRegistration({
    itemId,
    itemType,
    ref: () => ref,
    dragContext,
    defaultGroupId: props.defaultGroupId
  })

  // Extracted hook for animations
  useLibraryItemAnimation({
    itemKey,
    itemId,
    itemType,
    ref: () => ref,
    itemIndex: props.itemIndex,
    reducedMotion: () => globalStore.settings.data?.reducedMotion ?? false,
    animatedIds: props.animatedLibraryItemIds,
    initialComplete: props.libraryInitialAnimationComplete,
    itemsLength: props.libraryItemsLength,
    newlyCreatedFolderId: props.newlyCreatedFolderId,
    clearNewlyCreatedFolderId: props.clearNewlyCreatedFolderId,
    libraryItemRefs: props.libraryItemRefs,
    tileRefs: props.tileRefs,
    tileRefId: props.item.id
  })

  return (
    <>
      {/* Drop preview tile */}
      <Show when={dropIndicators.showDropIndicator()}>
        <DropPreviewTile
          tileSize={props.tileSize}
          dropTarget={dropIndicators.activeDropTarget()!}
        />
      </Show>

      <div
        ref={(el) => {
          ref = el
        }}
        data-library-item
        data-instance-tile={isInstance || undefined}
        data-server-tile={isServer || undefined}
        class="relative"
        classList={{
          hidden: dropIndicators.shouldCollapseTile()
        }}
        style={{ opacity: "0" }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Create folder indicator */}
        <DropOverlayIndicator
          isVisible={dropIndicators.showCreateFolderIndicator()}
          icon="i-hugeicons:folder-add"
          class="z-40 rounded-lg"
        />

        {/* Folder rendering - condition never changes for a given item */}
        <Show when={isFolder}>
          <FolderTile
            groupId={itemId}
            isOpen={props.openFolderId() === itemId}
            onToggle={() => props.toggleFolder(itemId)}
            size={props.tileSize() as 1 | 2 | 3 | 4 | 5}
            isSelected={props.isSelected(props.item.id)}
            onToggleSelection={() => props.toggleSelection(props.item.id)}
            selectedCount={props.selectedCount}
            onBatchDelete={props.onBatchDelete}
            onSelectExclusive={() => props.onSelectExclusive?.(props.item.id)}
          />
        </Show>

        {/* Instance rendering - condition never changes for a given item */}
        <Show when={isInstance && instanceData()}>
          {(instance) => (
            <InstanceTile
              instance={instance()}
              identifier={`ungrouped-${itemId}`}
              size={props.tileSize() as 1 | 2 | 3 | 4 | 5}
              isMultiSelected={props.isSelected(props.item.id)}
              onToggleSelection={() => props.toggleSelection(props.item.id)}
              isDragging={dropIndicators.isBeingDragged()}
              isDragActive={dragContext.isDragging()}
              groupId={props.defaultGroupId() ?? undefined}
              onDragStart={(e) => {
                const ids = props.isSelected(props.item.id)
                  ? parseInstanceIds(props.selectedIds())
                  : [itemId]
                props.onDragStart("instance", ids, e)
              }}
              preventClick={() => props.justDropped()}
              selectedCount={props.selectedCount}
              onBatchDelete={props.onBatchDelete}
              onSelectExclusive={() => props.onSelectExclusive?.(props.item.id)}
            />
          )}
        </Show>

        {/* Server rendering - condition never changes for a given item */}
        <Show when={isServer && serverData()}>
          {(server) => (
            <ServerTile
              server={server()}
              identifier={`server-${itemId}`}
              size={props.tileSize() as 1 | 2 | 3 | 4 | 5}
              isMultiSelected={props.isSelected(props.item.id)}
              onToggleSelection={() => props.toggleSelection(props.item.id)}
              isDragging={dropIndicators.isBeingDragged()}
              isDragActive={dragContext.isDragging()}
              groupId={props.defaultGroupId() ?? undefined}
              onDragStart={(e) => {
                const ids = props.isSelected(props.item.id)
                  ? parseServerIds(props.selectedIds())
                  : [itemId]
                props.onDragStart("server", ids, e)
              }}
              preventClick={props.justDropped()}
              onSelectExclusive={() => props.onSelectExclusive?.(props.item.id)}
            />
          )}
        </Show>
      </div>
    </>
  )
}

export default LibraryItemTile
