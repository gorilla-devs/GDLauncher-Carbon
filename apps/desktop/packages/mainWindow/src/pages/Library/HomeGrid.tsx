/**
 * HomeGrid - Library View Orchestrator
 *
 * Slim orchestrator component (~250 lines) that coordinates:
 * - Data hooks (useLibraryData, useLibrarySelection, useLibraryDragDrop)
 * - View switching (FoldersView vs AccordionView)
 * - Animation hooks (useFLIPAnimation, useEntranceAnimation)
 * - Global UI (header, context menu, selection bar, drag ghost)
 */

import {
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
  onMount
} from "solid-js"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
  Skeleton
} from "@gd/ui"
import { Trans } from "@gd/i18n"
import { useDragSelect } from "@/hooks/useDragSelect"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import UnstableCard from "@/components/UnstableCard"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import DragGhost from "@/components/DragGhost"
import {
  setClickedFolderId,
  setVisibleFolderIndices,
  injectFolderTransitionCSS,
  removeFolderTransitionCSS
} from "./utils/folderViewTransition"
import { parseInstanceIds, parseFolderIds } from "./utils/selectionIds"
import "@/components/Library/folderTransitions.css"

import { setClickedInstanceId } from "@/components/InstanceTile"
import { DragProvider, useDragContext } from "./DragContext"
import { SelectionActionBar } from "./components/SelectionActionBar"
import { FloatingFavoritesBar } from "./components/FloatingFavoritesBar"
import { LibraryHeader } from "./components/LibraryHeader"
import {
  useLibraryData,
  useLibrarySelection,
  useFLIPAnimation,
  useEntranceAnimation,
  useLibraryDragDrop
} from "./hooks"
import FoldersView from "./views/FoldersView"
import AccordionView from "./views/AccordionView"
import { LibraryItem } from "./types"

const HomeGrid = () => (
  <DragProvider>
    <HomeGridInner />
  </DragProvider>
)

const HomeGridInner = () => {
  const globalStore = useGlobalStore()
  const modals = useModal()
  const dragContext = useDragContext()

  // UI State
  const [filter, setFilter] = createSignal("")
  const [tileSize, setTileSize] = createSignal(2)
  const [openFolderId, setOpenFolderId] = createSignal<number | null>(null)

  // Refs for drag selection - keyed by type-prefixed string ID (e.g., "instance-5", "folder-3")
  const tileRefs = new Map<string, HTMLDivElement>()

  // Hooks
  // Don't destructure libraryItems/virtualGroups/favoriteIds — they are store
  // properties returned via getters. Destructuring reads the proxy once; if
  // reconcile replaces it the captured reference goes stale. Access them
  // through `data.*` so every read hits the getter in a reactive context.
  const data = useLibraryData(filter)
  const { viewMode, isFoldersView, defaultGroupId, isLoading, isEmpty } = data

  const showFoldersView = createMemo(() => isFoldersView() && !filter().trim())

  const selection = useLibrarySelection()

  const entranceAnimation = useEntranceAnimation()

  const [autoAnimateEnabled, setAutoAnimateEnabled] = createSignal(false)

  const flipAnimation = useFLIPAnimation({
    reducedMotion: () => globalStore.settings.data?.reducedMotion ?? false,
    onCleanup: () => {
      if (!dragContext.justDropped()) {
        setAutoAnimateEnabled(true)
      }
    }
  })

  const dragDrop = useLibraryDragDrop({
    defaultGroupId,
    selection,
    flipAnimation,
    get libraryItems() { return data.libraryItems },
    onBeforeDrop: () => setAutoAnimateEnabled(false)
  })

  // Sync tile size from settings
  createEffect(() => {
    if (globalStore.settings.data?.instancesTileSize) {
      setTileSize(globalStore.settings.data.instancesTileSize)
    }
  })

  // Enable auto-animate only after items are rendered and settled
  createEffect(() => {
    const items = data.libraryItems
    const reducedMotion = globalStore.settings.data?.reducedMotion ?? false
    if (items.length > 0 && !reducedMotion && !dragContext.justDropped()) {
      // Wait for DOM to settle before enabling auto-animate
      requestAnimationFrame(() => {
        setAutoAnimateEnabled(true)
      })
    } else if (reducedMotion) {
      setAutoAnimateEnabled(false)
    }
  })

  // Reset animation state when switching view modes
  createEffect(
    on(
      isFoldersView,
      () => {
        entranceAnimation.reset()
      },
      { defer: true }
    )
  )

  // Close open folder when search activates
  createEffect(() => {
    if (filter().trim()) {
      setOpenFolderId(null)
    }
  })

  // FLIP animation effect - runs after libraryItems changes
  createEffect(() => {
    const items = data.libraryItems
    if (items.length > 0 && flipAnimation.isAnimating()) {
      flipAnimation.animateIfOrderChanged(items.map((item) => item.id))
    }
  })

  // Register drop handler and clear stale view-transition state
  onMount(() => {
    dragContext.setOnDrop(dragDrop.handleDrop)
    setClickedInstanceId(undefined)
  })

  onCleanup(() => {
    dragContext.setOnDrop(null)
  })

  // Drag selection - returns rects keyed by type-prefixed string IDs
  const getItemRects = (): Map<string, DOMRect> => {
    const rects = new Map<string, DOMRect>()
    tileRefs.forEach((el, id) => {
      rects.set(id, el.getBoundingClientRect())
    })
    return rects
  }

  const dragSelect = useDragSelect({
    containerRef: () =>
      document.getElementById("gdl-content-wrapper") ?? undefined,
    getItemRects,
    onSelectionChange: (ids) => selection.selectAll(ids),
    getExistingSelection: () => selection.selectedIds()
  })

  const shouldIgnoreClick = (e: MouseEvent): boolean => {
    const target = e.target as HTMLElement
    return (
      !dragContext.dragSelectEnabled() ||
      openFolderId() !== null ||
      target.closest("[data-instance-tile]") !== null ||
      target.closest("[data-folder-tile]") !== null ||
      target.closest("input") !== null ||
      target.closest("button") !== null ||
      target.closest("[data-kb-menu]") !== null ||
      target.closest("[role='menu']") !== null
    )
  }

  // Escape key handler
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      if (dragContext.isDragging()) {
        dragContext.cancelDrag()
      } else if (selection.selectedIds().size > 0) {
        selection.clearSelection()
      }
    }
  }

  onMount(() => document.addEventListener("keydown", handleKeyDown))
  onCleanup(() => document.removeEventListener("keydown", handleKeyDown))

  // Folder toggle with view transition
  const toggleFolder = async (folderId: number) => {
    const overlay = document.getElementById("overlay")
    const shouldTransition =
      !globalStore.settings.data?.reducedMotion && document.startViewTransition

    if (shouldTransition) {
      const folder = data.libraryItems.find(
        (i) => i.type === "folder" && i.data.id === folderId
      )
      const instanceCount =
        folder?.type === "folder" ? folder.data.instances.length : 0
      const maxVisibleOnOpen = Math.min(instanceCount, 4)
      const visibleIndices = Array.from(
        { length: maxVisibleOnOpen },
        (_, i) => i
      )

      setVisibleFolderIndices(visibleIndices)
      injectFolderTransitionCSS(visibleIndices, "open")
      setClickedFolderId(folderId)

      // Show overlay BEFORE view transition captures states
      // Reset opacity in case it was left at "0" by ModalsManager.closeModal()
      if (overlay) {
        overlay.style.display = "flex"
        overlay.style.opacity = "1"
        overlay.style.transition = ""
      }

      await new Promise<void>((resolve) => queueMicrotask(() => resolve()))

      const transition = document.startViewTransition(() => {
        setOpenFolderId((prev) => (prev === folderId ? null : folderId))
      })
      transition.finished.then(() => {
        setClickedFolderId(null)
        setVisibleFolderIndices([])
        removeFolderTransitionCSS()
        // Hide overlay if folder was closed
        if (openFolderId() === null && overlay) {
          overlay.style.display = "none"
        }
      })
    } else {
      if (overlay) {
        overlay.style.display = folderId ? "flex" : "none"
        overlay.style.opacity = folderId ? "1" : ""
        overlay.style.transition = ""
      }
      setOpenFolderId((prev) => (prev === folderId ? null : folderId))
    }
  }

  const handleBatchDelete = () => {
    const selectedStringIds = selection.selectedIds()

    // Parse string IDs to separate instances from folders
    const selectedInstanceIds = parseInstanceIds(selectedStringIds)
    const selectedFolderIds = parseFolderIds(selectedStringIds)

    // Get full data for instances
    const selectedInstancesList = (globalStore.instances.data || []).filter(
      (instance) => selectedInstanceIds.includes(instance.id)
    )

    // Get full data for folders
    const selectedFoldersList = (globalStore.instanceGroups.data || []).filter(
      (group) => selectedFolderIds.includes(group.id)
    )

    // Open appropriate modal based on what's selected
    if (selectedInstancesList.length > 0 && selectedFoldersList.length === 0) {
      // Only instances selected
      modals?.openModal(
        { name: "confirmBatchInstanceDeletion" },
        {
          instances: selectedInstancesList,
          onComplete: selection.clearSelection
        }
      )
    } else if (
      selectedFoldersList.length > 0 &&
      selectedInstancesList.length === 0
    ) {
      // Only folders selected
      modals?.openModal(
        { name: "confirmBatchFolderDeletion" },
        { folders: selectedFoldersList, onComplete: selection.clearSelection }
      )
    } else {
      // Mixed selection - open mixed deletion modal
      modals?.openModal(
        { name: "confirmBatchMixedDeletion" },
        {
          instances: selectedInstancesList,
          folders: selectedFoldersList,
          onComplete: selection.clearSelection
        }
      )
    }
  }

  return (
    <div
      class="p-6"
      onMouseDown={(e) => {
        if (!shouldIgnoreClick(e)) {
          dragSelect.handlers.handleMouseDown(e)
        }
      }}
    >
      <UnstableCard />
      <Switch>
        <Match when={isLoading()}>
          <Skeleton.instances />
        </Match>
        <Match when={isEmpty()}>
          <div class="mt-12 flex h-full w-full flex-col items-center justify-center gap-6">
            <PlaceholderGorilla
              size={14}
              variant="Welcoming Gorilla - Open Arms"
            />
            <p class="text-lightSlate-700 max-w-100 text-center">
              <Trans key="instances:_trn_no_instances_text" />
            </p>
          </div>
        </Match>
        <Match when={!isLoading() && !isEmpty()}>
          <div>
            <LibraryHeader
              filter={filter}
              setFilter={setFilter}
              tileSize={tileSize}
              setTileSize={setTileSize}
              viewMode={viewMode}
            />
            <ContextMenu>
              <ContextMenuTrigger>
                <div
                  class="mt-4"
                  onClick={() => {
                    if (
                      !dragContext.isDragging() &&
                      !dragContext.justDropped()
                    ) {
                      setOpenFolderId(null)
                    }
                  }}
                >
                  <Switch>
                    <Match when={showFoldersView()}>
                      <FoldersView
                        libraryItems={data.libraryItems}
                        defaultGroupId={defaultGroupId()}
                        tileSize={tileSize}
                        selection={selection}
                        openFolderId={openFolderId}
                        setOpenFolderId={setOpenFolderId}
                        onToggleFolder={toggleFolder}
                        onDragStart={(type, ids, e) =>
                          dragContext.startDrag(type, ids, e)
                        }
                        justDropped={dragContext.justDropped}
                        flipAnimation={flipAnimation}
                        entranceAnimation={entranceAnimation}
                        autoAnimateEnabled={autoAnimateEnabled}
                        tileRefs={tileRefs}
                        newlyCreatedFolderId={dragDrop.newlyCreatedFolderId}
                        clearNewlyCreatedFolderId={
                          dragDrop.clearNewlyCreatedFolderId
                        }
                      />
                    </Match>
                    <Match when={!showFoldersView()}>
                      <AccordionView
                        virtualGroups={data.virtualGroups}
                        tileSize={tileSize}
                        selection={selection}
                        onDragStart={(type, ids, e) =>
                          dragContext.startDrag(type, ids, e)
                        }
                        justDropped={dragContext.justDropped}
                        animatedInstanceIds={entranceAnimation.animatedIds}
                        initialAnimationComplete={entranceAnimation}
                        tileRefs={tileRefs}
                      />
                    </Match>
                  </Switch>
                </div>
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuGroup>
                  <ContextMenuGroupLabel>
                    <Trans key="library:_trn_add_new_instance" />
                  </ContextMenuGroupLabel>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() =>
                      modals?.openModal({ name: "instanceCreation" })
                    }
                  >
                    <div class="i-hugeicons:file-add h-4 w-4" />
                    <Trans key="library:_trn_create_new_instance" />
                  </ContextMenuItem>
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() =>
                      modals?.openModal(
                        { name: "instanceCreation" },
                        { import: true }
                      )
                    }
                  >
                    <div class="i-hugeicons:download-02 h-4 w-4" />
                    <Trans key="library:_trn_import_instance" />
                  </ContextMenuItem>
                </ContextMenuGroup>
              </ContextMenuContent>
            </ContextMenu>
          </div>
        </Match>
      </Switch>

      <SelectionActionBar
        selectedCount={() => selection.selectedIds().size}
        onClearSelection={selection.clearSelection}
        onDelete={handleBatchDelete}
      />

      <FloatingFavoritesBar
        favoriteIds={data.favoriteIds}
        isSelectionActive={selection.selectedIds().size > 0}
      />

      <Show when={dragSelect.selectionRect()}>
        {(rect) => (
          <div
            class="border-primary-500 bg-primary-500/20 pointer-events-none fixed z-50 border-2"
            style={{
              left: `${rect().left}px`,
              top: `${rect().top}px`,
              width: `${rect().width}px`,
              height: `${rect().height}px`
            }}
          />
        )}
      </Show>

      <DragGhost
        instances={globalStore.instances.data || []}
        groups={data.libraryItems
          .filter(
            (item): item is LibraryItem & { type: "folder" } =>
              item.type === "folder"
          )
          .map((item) => ({
            id: item.data.id,
            name: item.data.name,
            instances: item.data.instances
          }))}
        tileSize={tileSize() as 1 | 2 | 3 | 4 | 5}
      />
    </div>
  )
}

export default HomeGrid
