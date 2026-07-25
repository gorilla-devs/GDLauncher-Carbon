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
import { useSearchParams } from "@solidjs/router"
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
import { useGDNavigate } from "@/managers/NavigationManager"
import UnstableCard from "@/components/UnstableCard"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import DragGhost from "@/components/DragGhost"
import {
  setClickedFolderId,
  setVisibleFolderIndices,
  injectFolderTransitionCSS,
  removeFolderTransitionCSS
} from "./utils/folderViewTransition"
import {
  parseInstanceIds,
  parseFolderIds,
  parseServerIds
} from "./utils/selectionIds"
import "@/components/Library/folderTransitions.css"
import "./styles/modeTransitions.css"

import {
  clickedInstanceId,
  setClickedInstanceId
} from "@/components/InstanceTile"
import { setClickedServerId } from "@/components/Server/Tile"
import { DragProvider, useDragContext } from "./DragContext"
import { FloatingFavoritesBar } from "./components/FloatingFavoritesBar"
import { LibraryHeader } from "./components/LibraryHeader"
import {
  useLibraryData,
  useServerData,
  useLibrarySelection,
  useFLIPAnimation,
  useEntranceAnimation,
  useLibraryDragDrop
} from "./hooks"
import FoldersView from "./views/FoldersView"
import AccordionView from "./views/AccordionView"
import { LibraryItem, LibraryMode } from "./types"
import { TILE_SIZES, TileSize } from "./constants"

const HomeGrid = () => (
  <DragProvider>
    <HomeGridInner />
  </DragProvider>
)

const HomeGridInner = () => {
  const globalStore = useGlobalStore()
  const modals = useModal()
  const dragContext = useDragContext()
  const navigator = useGDNavigate()

  const [searchParams, setSearchParams] = useSearchParams()

  // UI State
  const [filter, setFilter] = createSignal("")
  const [tileSize, setTileSize] = createSignal(2)
  const [openFolderId, setOpenFolderId] = createSignal<number | null>(null)
  const libraryMode = (): LibraryMode =>
    searchParams.mode === "servers" ? "servers" : "instances"
  // Skeleton crossfade: when loading ends, keep the skeleton overlay alive
  // briefly and fade it out on top of the real content so tiles appear to
  // swap in place rather than flicker through an empty state.
  const [skeletonVisible, setSkeletonVisible] = createSignal(false)
  const [skeletonFading, setSkeletonFading] = createSignal(false)

  // Refs for drag selection - keyed by type-prefixed string ID (e.g., "instance-5", "folder-3")
  const tileRefs = new Map<string, HTMLDivElement>()
  let contentRef: HTMLDivElement | undefined

  // Hooks
  // Don't destructure libraryItems/virtualGroups/favoriteIds — they are store
  // properties returned via getters. Destructuring reads the proxy once; if
  // reconcile replaces it the captured reference goes stale. Access them
  // through `data.*` so every read hits the getter in a reactive context.
  const instanceData = useLibraryData(filter)
  const serverData = useServerData(filter)

  // Active data based on library mode
  const data = createMemo(() =>
    libraryMode() === "instances" ? instanceData : serverData
  )
  const viewMode = createMemo(() => data().viewMode())
  const isFoldersView = createMemo(() => data().isFoldersView())
  const defaultGroupId = createMemo(() => data().defaultGroupId())
  const isLoading = createMemo(() => data().isLoading())
  const isEmpty = createMemo(() => data().isEmpty())

  const showFoldersView = createMemo(() => isFoldersView() && !filter().trim())

  const selection = useLibrarySelection()

  const entranceAnimation = useEntranceAnimation()

  const flipAnimation = useFLIPAnimation({
    reducedMotion: () => globalStore.settings.data?.reducedMotion ?? false
  })

  const dragDrop = useLibraryDragDrop({
    defaultGroupId,
    selection,
    flipAnimation,
    get libraryItems() {
      return data().libraryItems
    }
  })

  // Sync tile size from settings (shared across instances and servers)
  createEffect(() => {
    libraryMode()
    if (globalStore.settings.data?.instancesTileSize) {
      setTileSize(globalStore.settings.data.instancesTileSize)
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

  // Drive skeleton crossfade in response to isLoading flipping.
  createEffect(() => {
    if (isLoading()) {
      setSkeletonVisible(true)
      setSkeletonFading(false)
    } else if (skeletonVisible()) {
      // Real content just mounted — fade skeleton out on top of it,
      // then unmount once the transition finishes.
      setSkeletonFading(true)
      const id = setTimeout(() => {
        setSkeletonVisible(false)
        setSkeletonFading(false)
      }, 220)
      onCleanup(() => clearTimeout(id))
    }
  })

  // FLIP animation effect - runs after libraryItems changes
  createEffect(() => {
    const items = data().libraryItems
    if (items.length > 0 && flipAnimation.isAnimating()) {
      flipAnimation.animateIfOrderChanged(items.map((item) => item.id))
    }
  })

  // Register drop handler and clear stale view-transition state
  onMount(() => {
    dragContext.setOnDrop(dragDrop.handleDrop)

    // If returning directly from an instance page that was inside a folder,
    // re-open that folder so the tile exists in the DOM for the view-transition.
    // Only applies when the previous page was an instance page (/library/:id),
    // not when arriving from an unrelated page (e.g. /settings).
    const clicked = clickedInstanceId()
    const prevPath = navigator.lastPathVisited().path
    const isFromInstancePage = /^\/library\/\d+/.test(prevPath)
    if (clicked?.startsWith("folder-") && isFromInstancePage) {
      const groupId = parseInt(clicked.split("-")[1], 10)
      if (!isNaN(groupId)) {
        setOpenFolderId(groupId)
        const overlay = document.getElementById("overlay")
        if (overlay) {
          overlay.style.display = "flex"
          overlay.style.opacity = "1"
          overlay.style.transition = ""
        }
      }
    }

    // Defer clearing so the view-transition snapshot captures the
    // tile with its view-transition-name before it's removed.
    requestAnimationFrame(() => {
      setClickedInstanceId(undefined)
      setClickedServerId(undefined)
    })
  })

  onCleanup(() => {
    dragContext.setOnDrop(null)
  })

  // Drag selection - returns rects keyed by type-prefixed string IDs
  // Excludes instances that are queued or downloading (preparing) since they
  // shouldn't be selectable.
  const getItemRects = (): Map<string, DOMRect> => {
    const nonSelectable = new Set<string>()
    for (const item of data().libraryItems) {
      if (item.type === "instance" && item.data.status.status === "valid") {
        const s = item.data.status.value.state.state
        if (s === "queued" || s === "preparing") {
          nonSelectable.add(item.id)
        }
      }
    }
    const rects = new Map<string, DOMRect>()
    tileRefs.forEach((el, id) => {
      if (!nonSelectable.has(id)) {
        rects.set(id, el.getBoundingClientRect())
      }
    })
    return rects
  }

  const dragSelect = useDragSelect({
    containerRef: () =>
      document.getElementById("gdl-content-wrapper") ?? undefined,
    getItemRects,
    onSelectionChange: (ids) => selection.selectAll(ids),
    getExistingSelection: () => selection.selectedIds(),
    getTopBoundary: () => {
      const header = document.querySelector<HTMLElement>(
        "[data-library-header]"
      )
      return header?.getBoundingClientRect().bottom
    }
  })

  const shouldIgnoreClick = (e: MouseEvent): boolean => {
    const target = e.target as HTMLElement
    return (
      !dragContext.dragSelectEnabled() ||
      openFolderId() !== null ||
      target.closest("[data-instance-tile]") !== null ||
      target.closest("[data-server-tile]") !== null ||
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
      const isClosing = openFolderId() === folderId
      const folder = data().libraryItems.find(
        (i) => i.type === "folder" && i.data.id === folderId
      )
      const instanceCount =
        folder?.type === "folder" ? folder.data.instances.length : 0
      const maxVisible = Math.min(instanceCount, 4)
      const visibleIndices = Array.from({ length: maxVisible }, (_, i) => i)

      setVisibleFolderIndices(visibleIndices)
      injectFolderTransitionCSS(visibleIndices, isClosing ? "close" : "open")
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

  // Library mode switch with slide transition
  const handleModeSwitch = async (newMode: LibraryMode) => {
    if (libraryMode() === newMode) return

    const reducedMotion = globalStore.settings.data?.reducedMotion
    const modeParam = newMode === "servers" ? "servers" : undefined

    if (!reducedMotion && document.startViewTransition) {
      const direction = newMode === "servers" ? "forward" : "backward"

      // Put the view-transition-name + class on the scroll container
      // instead of contentRef. contentRef's border box spans the full tile
      // list (scrolling happens on the ancestor), so its snapshot paints
      // tiles that were scrolled out of view on top of the surrounding
      // layout during the slide. The scroll container, by contrast, is
      // already viewport-sized, so its snapshot naturally matches what the
      // user was seeing — no clip-path gymnastics needed.
      //
      // The sticky LibraryHeader sits inside the scroll container, so we
      // also give it its own view-transition-name. That lifts it out of
      // the library-content snapshot into its own pseudo group, and the
      // `animation: none` rule in modeTransitions.css keeps it visually
      // fixed while the content slides behind it.
      const scrollEl = document.getElementById("gdl-content-wrapper")
      let headerEl: HTMLElement | null = null
      if (contentRef) {
        let sib = contentRef.previousElementSibling as HTMLElement | null
        while (sib) {
          if (getComputedStyle(sib).position === "sticky") {
            headerEl = sib
            break
          }
          sib = sib.previousElementSibling as HTMLElement | null
        }
      }
      if (scrollEl) {
        scrollEl.style.viewTransitionName = "library-content"
        scrollEl.style.setProperty("view-transition-class", direction)
        // Hide the scrollbar while the snapshot is captured so it isn't
        // baked into the library-content pseudo and carried along by the
        // slide. scrollbar-gutter: stable on the container (see
        // ContentWrapper) reserves the space so there's no layout shift.
        scrollEl.classList.add("library-mode-switching")
      }
      if (headerEl) {
        headerEl.style.viewTransitionName = "library-header"
      }

      const transition = document.startViewTransition(() => {
        setSearchParams({ mode: modeParam }, { replace: true })
      })

      const finish = () => {
        if (scrollEl) {
          scrollEl.style.viewTransitionName = ""
          scrollEl.style.removeProperty("view-transition-class")
          scrollEl.classList.remove("library-mode-switching")
        }
        if (headerEl) headerEl.style.viewTransitionName = ""
      }
      transition.finished.then(finish).catch(finish)
    } else {
      setSearchParams({ mode: modeParam }, { replace: true })
    }
  }

  const handleSelectExclusive = (id: string) => {
    selection.clearSelection()
    selection.toggleSelection(id)
  }

  const handleBatchDelete = () => {
    const selectedStringIds = selection.selectedIds()

    if (libraryMode() === "servers") {
      const selectedServerIds = parseServerIds(selectedStringIds)
      const selectedServersList = (globalStore.servers.data || []).filter(
        (server) => selectedServerIds.includes(server.id)
      )

      if (selectedServersList.length === 0) {
        selection.clearSelection()
        return
      }

      modals?.openModal(
        { name: "confirmBatchServerDeletion" },
        {
          servers: selectedServersList,
          onComplete: selection.clearSelection
        }
      )
      return
    }

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
      data-testid="library-root"
      class="box-border flex flex-1 flex-col p-6"
      onMouseDown={(e) => {
        if (!shouldIgnoreClick(e)) {
          dragSelect.handlers.handleMouseDown(e)
        }
      }}
    >
      <UnstableCard />
      <LibraryHeader
        filter={filter}
        setFilter={setFilter}
        tileSize={tileSize}
        setTileSize={setTileSize}
        viewMode={viewMode}
        libraryMode={libraryMode}
        setLibraryMode={handleModeSwitch}
      />
      <div ref={contentRef} class="relative flex flex-1 flex-col">
        <Show when={skeletonVisible()}>
          <div
            class="pointer-events-none transition-opacity duration-200 ease-out motion-reduce:transition-none"
            classList={{
              "absolute inset-0 z-10 opacity-0": skeletonFading(),
              "opacity-100": !skeletonFading()
            }}
          >
            <Skeleton.instances
              tileWidthPx={TILE_SIZES[tileSize() as TileSize]?.widthPx ?? 184}
              rowGapPx={
                { 1: 16, 2: 24, 3: 32, 4: 40, 5: 48 }[tileSize() as TileSize] ??
                24
              }
            />
          </div>
        </Show>
        <Show when={!isLoading()}>
          <ContextMenu>
            <ContextMenuTrigger class="flex flex-1 flex-col">
              <Show
                when={!isEmpty()}
                fallback={
                  <div class="mt-12 flex h-full w-full flex-col items-center justify-center gap-6">
                    <PlaceholderGorilla
                      size={14}
                      variant="Welcoming Gorilla - Open Arms"
                    />
                    <p class="text-lightSlate-700 max-w-100 text-center">
                      <Trans key="instances:_trn_no_instances_text" />
                    </p>
                  </div>
                }
              >
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
                        libraryItems={data().libraryItems}
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
                        tileRefs={tileRefs}
                        newlyCreatedFolderId={dragDrop.newlyCreatedFolderId}
                        clearNewlyCreatedFolderId={
                          dragDrop.clearNewlyCreatedFolderId
                        }
                        selectedCount={selection.selectedIds().size}
                        onBatchDelete={handleBatchDelete}
                        onSelectExclusive={handleSelectExclusive}
                        libraryMode={libraryMode()}
                      />
                    </Match>
                    <Match when={!showFoldersView()}>
                      <AccordionView
                        virtualGroups={data().virtualGroups}
                        libraryMode={libraryMode()}
                        tileSize={tileSize}
                        selection={selection}
                        onDragStart={(type, ids, e) =>
                          dragContext.startDrag(type, ids, e)
                        }
                        justDropped={dragContext.justDropped}
                        animatedInstanceIds={entranceAnimation.animatedIds}
                        initialAnimationComplete={entranceAnimation}
                        tileRefs={tileRefs}
                        selectedCount={selection.selectedIds().size}
                        onBatchDelete={handleBatchDelete}
                        onSelectExclusive={handleSelectExclusive}
                      />
                    </Match>
                  </Switch>
                </div>
              </Show>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <Switch>
                <Match when={libraryMode() === "instances"}>
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
                </Match>
                <Match when={libraryMode() === "servers"}>
                  <ContextMenuGroup>
                    <ContextMenuGroupLabel>
                      <Trans key="instances:_trn_server_create_title" />
                    </ContextMenuGroupLabel>
                    <ContextMenuSeparator />
                    <ContextMenuItem
                      class="flex items-center gap-2"
                      onClick={() =>
                        modals?.openModal({ name: "serverCreation" })
                      }
                    >
                      <div class="i-hugeicons:server h-4 w-4" />
                      <Trans key="instances:_trn_server_create" />
                    </ContextMenuItem>
                  </ContextMenuGroup>
                </Match>
              </Switch>
            </ContextMenuContent>
          </ContextMenu>
        </Show>
      </div>

      <FloatingFavoritesBar
        favoriteIds={data().favoriteIds}
        isSelectionActive={selection.selectedIds().size > 0}
        libraryMode={libraryMode()}
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
        servers={globalStore.servers.data || []}
        groups={data()
          .libraryItems.filter(
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
