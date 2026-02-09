/**
 * FoldersView Component
 *
 * iOS-style folder view for the library.
 * Displays favorites row, main grid with folders + ungrouped instances,
 * and expanded folder overlay.
 */

import { createEffect, createMemo, Show, Accessor } from "solid-js"
import { createAutoAnimate } from "@formkit/auto-animate/solid"
import { useDragContext, DragType } from "../../DragContext"
import ExpandedFolderContent from "@/components/Library/ExpandedFolderContent"
import { FavoritesRow } from "./FavoritesRow"
import { LibraryGrid } from "./LibraryGrid"
import { LibraryItem, SelectionState, FLIPAnimation } from "../../types"
import { EntranceAnimationReturn } from "../../hooks/useFLIPAnimation"
import { parseInstanceIds } from "../../utils/selectionIds"

interface FoldersViewProps {
  libraryItems: LibraryItem[]
  favoriteIds: number[]
  defaultGroupId: number | null
  tileSize: Accessor<number>
  selection: SelectionState
  openFolderId: Accessor<number | null>
  setOpenFolderId: (id: number | null) => void
  onToggleFolder: (folderId: number) => Promise<void>
  onDragStart: (type: DragType, ids: number[], e: PointerEvent) => void
  justDropped: Accessor<boolean>
  flipAnimation: FLIPAnimation
  entranceAnimation: EntranceAnimationReturn
  autoAnimateEnabled: Accessor<boolean>
  tileRefs: Map<string, HTMLDivElement>
  newlyCreatedFolderId: Accessor<number | null>
  clearNewlyCreatedFolderId: () => void
}

export default function FoldersView(props: FoldersViewProps) {
  const dragContext = useDragContext()

  // Auto-animate refs for grid containers
  const [favoritesGridRef, setFavoritesGridEnabled] = createAutoAnimate({
    duration: 200,
    easing: "ease-out"
  })
  const [mainGridRef, setMainGridEnabled] = createAutoAnimate({
    duration: 200,
    easing: "ease-out"
  })

  // Sync auto-animate with prop
  const updateAutoAnimate = (enabled: boolean) => {
    setFavoritesGridEnabled(enabled)
    setMainGridEnabled(enabled)
  }

  // Update when prop changes
  createEffect(() => {
    updateAutoAnimate(props.autoAnimateEnabled())
  })

  // Get the currently open folder data
  const getOpenFolder = createMemo(() => {
    const id = props.openFolderId()
    if (id === null) return null
    const item = props.libraryItems.find(
      (i) => i.type === "folder" && i.data.id === id
    )
    return item?.type === "folder" ? item.data : null
  })

  return (
    <>
      {/* Favorites Row */}
      <FavoritesRow
        favoriteIds={props.favoriteIds}
        isDragActive={dragContext.isDragging()}
        justDropped={props.justDropped}
        gridRef={favoritesGridRef}
      />

      {/* Main Grid: Ungrouped instances + Folder tiles */}
      <LibraryGrid
        libraryItems={props.libraryItems}
        tileSize={props.tileSize}
        defaultGroupId={props.defaultGroupId}
        openFolderId={props.openFolderId}
        onToggleFolder={(id) => props.onToggleFolder(id)}
        selection={props.selection}
        onDragStart={props.onDragStart}
        justDropped={props.justDropped}
        flipAnimation={props.flipAnimation}
        entranceAnimation={props.entranceAnimation}
        gridRef={mainGridRef}
        tileRefs={props.tileRefs}
        newlyCreatedFolderId={props.newlyCreatedFolderId}
        clearNewlyCreatedFolderId={props.clearNewlyCreatedFolderId}
      />

      {/* Expanded Folder Overlay */}
      <Show when={getOpenFolder()}>
        {(folder) => (
          <ExpandedFolderContent
            group={folder()}
            onClose={() => props.setOpenFolderId(null)}
            tileSize={props.tileSize() as 1 | 2 | 3 | 4 | 5}
            isDefaultGroup={false}
            selectedIds={props.selection.selectedIds()}
            onToggleSelection={props.selection.toggleSelection}
            onSetSelection={(ids) => props.selection.selectAll(ids)}
            onDragStart={(instanceId, isInstanceSelected, e) => {
              const ids = isInstanceSelected
                ? parseInstanceIds(props.selection.selectedIds())
                : [instanceId]
              dragContext.startDrag("instance", ids, e)
            }}
          />
        )}
      </Show>
    </>
  )
}
