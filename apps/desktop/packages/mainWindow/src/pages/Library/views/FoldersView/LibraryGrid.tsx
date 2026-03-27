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
import { useDragLayoutAnimation } from "../../hooks/useDragLayoutAnimation"

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
  tileRefs: Map<string, HTMLDivElement>
  newlyCreatedFolderId: Accessor<number | null>
  clearNewlyCreatedFolderId: () => void
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: (id: string) => void
}

export function LibraryGrid(props: LibraryGridProps) {
  const dragContext = useDragContext()

  // Wrap entranceAnimation in refs object for LibraryItemTile
  const libraryItemRefs = new Map<string, HTMLDivElement>()

  let gridEl: HTMLDivElement | undefined
  useDragLayoutAnimation(() => gridEl)

  return (
    <div
      ref={gridEl}
      class={`relative content-start ${TILE_SIZES[props.tileSize() as TileSize]?.gapY ?? "gap-y-6"}`}
      style={{
        display: "grid",
        "grid-template-columns": `repeat(auto-fill, ${TILE_SIZES[props.tileSize() as TileSize]?.widthPx ?? 184}px)`,
        "justify-content": "space-between",
        "column-gap": "16px"
      }}
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
            selectedCount={props.selectedCount}
            onBatchDelete={props.onBatchDelete}
            onSelectExclusive={props.onSelectExclusive}
          />
        )}
      </For>

      {/* End of main grid drop zone for instances */}
      <Show
        when={
          (dragContext.isDragging() || dragContext.justDropped()) &&
          (dragContext.dragType() === "instance" || dragContext.dragType() === "server") &&
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
          (dragContext.dragType() === "group" || dragContext.dragType() === "serverGroup")
        }
      >
        <EndOfGroupsDropZone tileSize={props.tileSize} />
      </Show>
    </div>
  )
}
