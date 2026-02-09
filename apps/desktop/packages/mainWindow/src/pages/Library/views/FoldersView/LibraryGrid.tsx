/**
 * LibraryGrid Component
 *
 * Main grid of library items (folders + ungrouped instances).
 * Handles the layout and rendering of LibraryItemTile components.
 */

import { For, Show, Accessor } from "solid-js"
import { useDragContext, DragType } from "../../DragContext"
import LibraryItemTile from "@/components/Library/LibraryItemTile"
import {
  EndOfGroupDropZone,
  EndOfGroupsDropZone
} from "../../components/EndOfGroupDropZone"
import { LibraryItem, SelectionState, FLIPAnimation } from "../../types"
import { EntranceAnimationReturn } from "../../hooks/useFLIPAnimation"
import { TILE_SIZES, TileSize } from "../../constants"

interface LibraryGridProps {
  libraryItems: LibraryItem[]
  tileSize: Accessor<number>
  defaultGroupId: number | null
  openFolderId: Accessor<number | null>
  onToggleFolder: (folderId: number) => void
  selection: SelectionState
  onDragStart: (type: DragType, ids: number[], e: PointerEvent) => void
  justDropped: Accessor<boolean>
  flipAnimation: FLIPAnimation
  entranceAnimation: EntranceAnimationReturn
  gridRef?: (el: HTMLDivElement) => void
  tileRefs: Map<string, HTMLDivElement>
  newlyCreatedFolderId: Accessor<number | null>
  clearNewlyCreatedFolderId: () => void
}

export function LibraryGrid(props: LibraryGridProps) {
  const dragContext = useDragContext()

  // Wrap entranceAnimation in refs object for LibraryItemTile
  const libraryItemRefs = new Map<string, HTMLDivElement>()

  return (
    <div
      ref={(el) => {
        props.gridRef?.(el)
      }}
      class={`relative flex flex-wrap content-start gap-x-4 ${TILE_SIZES[props.tileSize() as TileSize]?.gapY ?? "gap-y-6"}`}
    >
      <For each={props.libraryItems}>
        {(item, itemIndex) => (
          <LibraryItemTile
            item={item}
            itemIndex={itemIndex}
            tileSize={props.tileSize}
            defaultGroupId={() => props.defaultGroupId}
            openFolderId={props.openFolderId}
            toggleFolder={props.onToggleFolder}
            isSelected={props.selection.isSelected}
            toggleSelection={props.selection.toggleSelection}
            selectedIds={props.selection.selectedIds}
            onDragStart={(type, ids, e) => props.onDragStart(type, ids, e)}
            justDropped={props.justDropped}
            tileRefs={props.tileRefs}
            libraryItemRefs={libraryItemRefs}
            animatedLibraryItemIds={
              props.entranceAnimation.animatedIds as Set<string>
            }
            libraryInitialAnimationComplete={{
              get value() {
                return props.entranceAnimation.initialComplete
              },
              set value(v) {
                props.entranceAnimation.value = v
              }
            }}
            libraryItemsLength={() => props.libraryItems.length}
            newlyCreatedFolderId={props.newlyCreatedFolderId}
            clearNewlyCreatedFolderId={props.clearNewlyCreatedFolderId}
            previousItemId={() => {
              const idx = itemIndex()
              if (idx === 0) return null
              const prevItem = props.libraryItems[idx - 1]
              // Only return instance IDs (folders use different drop targets)
              return prevItem?.type === "instance" ? prevItem.data.id : null
            }}
          />
        )}
      </For>

      {/* End of main grid drop zone for instances */}
      <Show
        when={
          (dragContext.isDragging() || dragContext.justDropped()) &&
          dragContext.dragType() === "instance" &&
          props.defaultGroupId
        }
      >
        <EndOfGroupDropZone
          groupId={props.defaultGroupId!}
          instanceCount={
            props.libraryItems.filter((i) => i.type === "instance").length
          }
          tileSize={props.tileSize}
        />
      </Show>

      {/* End of groups drop zone for folder reordering */}
      <Show
        when={
          (dragContext.isDragging() || dragContext.justDropped()) &&
          dragContext.dragType() === "group"
        }
      >
        <EndOfGroupsDropZone tileSize={props.tileSize} />
      </Show>
    </div>
  )
}
