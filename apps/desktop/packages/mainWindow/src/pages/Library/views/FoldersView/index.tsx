/**
 * FoldersView Component
 *
 * iOS-style folder view for the library.
 * Displays main grid with folders + ungrouped instances,
 * and expanded folder overlay.
 */

import { createMemo, Show, Accessor } from "solid-js"
import { useDragContext, DragType } from "../../DragContext"
import ExpandedFolderContent from "@/components/Library/ExpandedFolderContent"
import { LibraryGrid } from "./LibraryGrid"
import {
  LibraryItem,
  LibraryMode,
  SelectionState,
  FLIPAnimation
} from "../../types"
import { EntranceAnimationReturn } from "../../hooks/useFLIPAnimation"
import { parseInstanceIds, parseServerIds } from "../../utils/selectionIds"

interface FoldersViewProps {
  libraryItems: LibraryItem[]
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
  tileRefs: Map<string, HTMLDivElement>
  newlyCreatedFolderId: Accessor<number | null>
  clearNewlyCreatedFolderId: () => void
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: (id: string) => void
  libraryMode?: LibraryMode
}

export default function FoldersView(props: FoldersViewProps) {
  const dragContext = useDragContext()

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
        tileRefs={props.tileRefs}
        newlyCreatedFolderId={props.newlyCreatedFolderId}
        clearNewlyCreatedFolderId={props.clearNewlyCreatedFolderId}
        selectedCount={props.selectedCount}
        onBatchDelete={props.onBatchDelete}
        onSelectExclusive={props.onSelectExclusive}
      />

      {/* Expanded Folder Overlay */}
      <Show when={getOpenFolder()}>
        {(folder) => (
          <ExpandedFolderContent
            group={folder()}
            onClose={() => props.setOpenFolderId(null)}
            tileSize={props.tileSize() as 1 | 2 | 3 | 4 | 5}
            isDefaultGroup={false}
            isServerMode={props.libraryMode === "servers"}
            selectedIds={props.selection.selectedIds()}
            onToggleSelection={props.selection.toggleSelection}
            onSetSelection={(ids) => props.selection.selectAll(ids)}
            onDragStart={(itemId, isItemSelected, e) => {
              const isServer = props.libraryMode === "servers"
              const ids = isItemSelected
                ? isServer
                  ? parseServerIds(props.selection.selectedIds())
                  : parseInstanceIds(props.selection.selectedIds())
                : [itemId]
              dragContext.startDrag(isServer ? "server" : "instance", ids, e)
            }}
            selectedCount={props.selectedCount}
            onBatchDelete={props.onBatchDelete}
            onSelectExclusive={props.onSelectExclusive}
          />
        )}
      </Show>
    </>
  )
}
