import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount
} from "solid-js"
import { Portal } from "solid-js/web"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger
} from "@gd/ui"
import { Trans } from "@gd/i18n"
import { ListInstance, ListServer } from "@gd/core_module/bindings"
import InstanceTile from "@/components/InstanceTile"
import ServerTile from "@/components/Server/Tile"
import { useDragContext } from "@/pages/Library/DragContext"
import { useDragSelect } from "@/hooks/useDragSelect"
import { rspc } from "@/utils/rspcClient"
import adSize from "@/utils/adhelper"
import {
  clickedFolderId,
  setClickedFolderId,
  visibleFolderIndices,
  setVisibleFolderIndices,
  injectFolderTransitionCSS,
  removeFolderTransitionCSS
} from "@/pages/Library/utils/folderViewTransition"
import { TILE_SIZES } from "@/pages/Library/constants"
import { useDragLayoutAnimation } from "@/pages/Library/hooks/useDragLayoutAnimation"
import { getInstanceImageUrl } from "@/utils/instances"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import { EndOfGroupDropZone } from "@/pages/Library/components/EndOfGroupDropZone"
import DropPreviewTile from "@/pages/Library/components/DropPreviewTile"
import { FolderHeader } from "./FolderHeader"

interface ExpandedFolderContentProps {
  group: { id: number; name: string; instances: (ListInstance | ListServer)[] }
  onClose: () => void
  tileSize: 1 | 2 | 3 | 4 | 5
  isDefaultGroup: boolean
  isServerMode?: boolean
  selectedIds: Set<string>
  onToggleSelection: (id: string) => void
  onSetSelection: (ids: string[]) => void
  onDragStart: (
    instanceId: number,
    isSelected: boolean,
    e: PointerEvent
  ) => void
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: (id: string) => void
}

// Backdrop drop zone - dropping on it moves instances to root library
const BackdropDropZone = (props: { onClose: () => void; folderId: number }) => {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "ungrouped"
  }

  // Register drop zone when dragging instances or servers
  createEffect(() => {
    const dtype = dragContext.dragType()
    if (
      dragContext.isDragging() &&
      (dtype === "instance" || dtype === "server") &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: "backdrop-ungrouped",
        rect,
        element: ref,
        target: { type: "ungrouped" },
        scope: `folder-${props.folderId}`
      })
    } else {
      dragContext.unregisterDropZone("backdrop-ungrouped")
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone("backdrop-ungrouped")
  })

  return (
    <div
      ref={ref}
      class="pointer-events-auto absolute inset-0 transition-colors duration-200"
      classList={{
        "bg-black/50": !isOver(),
        "bg-primary-500/20 border-2 border-dashed border-primary-500": isOver()
      }}
      onClick={(e) => {
        e.stopPropagation()
        // Don't close folder if we just finished a drag operation
        if (!dragContext.isDragging() && !dragContext.justDropped()) {
          props.onClose()
        }
      }}
    />
  )
}

const ExpandedFolderContent = (props: ExpandedFolderContentProps) => {
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  const [isEditing, setIsEditing] = createSignal(false)
  const [editValue, setEditValue] = createSignal("")
  let inputRef: HTMLInputElement | undefined
  let scrollContainerRef: HTMLDivElement | undefined

  let folderGridEl: HTMLDivElement | undefined
  useDragLayoutAnimation(() => folderGridEl)

  // Capture group values at mount time to avoid stale access during unmount
  // (props.group comes from a <Show> callback and becomes stale when parent unmounts)
  const groupId = props.group.id
  const groupName = props.group.name

  // Set active scope for folder overlay - only scoped zones will be considered during drag
  createEffect(() => {
    dragContext.setActiveScope(`folder-${groupId}`)
    onCleanup(() => {
      dragContext.setActiveScope(null)
    })
  })

  // Create a safe accessor for instances that returns empty array if props become stale
  // This prevents the For loop from throwing when the parent Show unmounts
  const safeInstances = createMemo(() => {
    try {
      return props.group.instances
    } catch {
      // Return empty array if props.group is stale (parent Show unmounting)
      return []
    }
  })

  // Non-reactive Map for storing DOM refs - refs don't need reactivity
  // Keyed by type-prefixed string ID (e.g., "instance-5")
  const folderTileRefs = new Map<string, HTMLDivElement>()

  // Get item rects for drag select - returns rects keyed by string IDs
  // Excludes instances that are queued or downloading (preparing).
  const getItemRects = (): Map<string, DOMRect> => {
    const nonSelectable = new Set<string>()
    const idPrefix = props.isServerMode ? "server" : "instance"
    for (const inst of safeInstances()) {
      if ("status" in inst && inst.status.status === "valid") {
        const s = inst.status.value.state.state
        if (s === "queued" || s === "preparing") {
          nonSelectable.add(`${idPrefix}-${inst.id}`)
        }
      }
    }
    const rects = new Map<string, DOMRect>()
    folderTileRefs.forEach((el, id) => {
      if (el && !nonSelectable.has(id)) rects.set(id, el.getBoundingClientRect())
    })
    return rects
  }

  // Local drag select hook for folder content
  const dragSelect = useDragSelect({
    containerRef: () => scrollContainerRef,
    getItemRects,
    onSelectionChange: (ids) => props.onSetSelection(ids),
    getExistingSelection: () => props.selectedIds
  })

  // Track registered drop zone IDs to clean up properly without accessing stale props
  const registeredDropZoneIds = new Set<string>()

  // Register drop zones for instance/server reordering within folder
  createEffect(() => {
    const dtype = dragContext.dragType()
    if (dragContext.isDragging() && (dtype === "instance" || dtype === "server")) {
      const draggedIds = dragContext.draggedIds()
      // Use safeInstances which handles stale props gracefully
      const instances = safeInstances()
      const idPrefix = props.isServerMode ? "server" : "instance"

      instances.forEach((instance, index) => {
        const zoneId = `before-folder-instance-${instance.id}`
        const instanceStringId = `${idPrefix}-${instance.id}`
        const el = folderTileRefs.get(instanceStringId)
        if (!el) return

        // Don't register drop zone for dragged instances
        if (draggedIds.includes(instance.id)) {
          dragContext.unregisterDropZone(zoneId)
          registeredDropZoneIds.delete(zoneId)
          return
        }

        const rect = el.getBoundingClientRect()
        // Full tile width - no createFolder zones inside folders
        const dropRect = new DOMRect(
          rect.left - 8,
          rect.top,
          rect.width + 16,
          rect.height
        )

        dragContext.registerDropZone({
          id: zoneId,
          rect: dropRect,
          element: el,
          rectTransform: (r) =>
            new DOMRect(r.left - 8, r.top, r.width + 16, r.height),
          target: {
            type: "beforeInstance",
            instanceId: instance.id,
            groupId: groupId
          },
          scope: `folder-${groupId}`
        })
        registeredDropZoneIds.add(zoneId)
      })
    } else {
      // Unregister all when not dragging - use tracked IDs instead of stale props
      registeredDropZoneIds.forEach((zoneId) => {
        dragContext.unregisterDropZone(zoneId)
      })
      registeredDropZoneIds.clear()
    }
  })

  // Register content area as endOfGroup zone to prevent cursor from falling through
  // to the backdrop's "ungrouped" zone when not on a specific instance's beforeInstance zone
  createEffect(() => {
    const dtype = dragContext.dragType()
    if (
      dragContext.isDragging() &&
      (dtype === "instance" || dtype === "server") &&
      scrollContainerRef
    ) {
      const rect = scrollContainerRef.getBoundingClientRect()
      dragContext.registerDropZone({
        id: `folder-content-area-${groupId}`,
        rect,
        element: scrollContainerRef,
        target: { type: "folderContentArea", groupId: groupId },
        scope: `folder-${groupId}`
      })
      registeredDropZoneIds.add(`folder-content-area-${groupId}`)
    } else {
      dragContext.unregisterDropZone(`folder-content-area-${groupId}`)
      registeredDropZoneIds.delete(`folder-content-area-${groupId}`)
    }
  })

  // Cleanup drop zones on unmount - use tracked IDs instead of stale props
  onCleanup(() => {
    registeredDropZoneIds.forEach((zoneId) => {
      dragContext.unregisterDropZone(zoneId)
    })
    registeredDropZoneIds.clear()
  })

  const renameGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.renameGroup"]
  }))

  const arrangeGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.arrangeGroup"]
  }))

  // Detect which instance tiles are currently visible in the scroll container
  const getVisibleInstanceIndices = (): number[] => {
    if (!scrollContainerRef) return []

    const containerRect = scrollContainerRef.getBoundingClientRect()
    const tiles = scrollContainerRef.querySelectorAll("[data-instance-tile]")
    const visibleIndices: number[] = []

    tiles.forEach((tile, index) => {
      const tileRect = tile.getBoundingClientRect()
      // Check if tile is at least partially visible in container
      const isVisible =
        tileRect.bottom > containerRect.top &&
        tileRect.top < containerRect.bottom &&
        tileRect.right > containerRect.left &&
        tileRect.left < containerRect.right

      if (isVisible) {
        visibleIndices.push(index)
      }
    })

    return visibleIndices
  }

  // Handle close with view transition
  const handleClose = async () => {
    const shouldTransition =
      !globalStore.settings.data?.reducedMotion && document.startViewTransition

    if (shouldTransition) {
      const visibleIndices = getVisibleInstanceIndices()
      setVisibleFolderIndices(visibleIndices)
      injectFolderTransitionCSS(visibleIndices, "close")
      setClickedFolderId(groupId)

      // Preload preview images (first 4) into browser cache before transition
      const previewInstances = safeInstances().slice(0, 4)
      await Promise.all(
        previewInstances.map((inst) => {
          if (!inst.icon_revision) return Promise.resolve()
          return new Promise<void>((resolve) => {
            const img = new Image()
            img.onload = () => resolve()
            img.onerror = () => resolve()
            img.src = getInstanceImageUrl(inst.id, inst.icon_revision!)
            setTimeout(resolve, 300) // Don't wait forever
          })
        })
      )

      // Wait for SolidJS to flush DOM updates before capturing OLD snapshot
      await new Promise<void>((resolve) => queueMicrotask(() => resolve()))

      const transition = document.startViewTransition(async () => {
        props.onClose()
        // Wait for SolidJS to render FolderTile's images before capturing NEW snapshot
        await new Promise<void>((resolve) => queueMicrotask(() => resolve()))
      })
      transition.finished.then(() => {
        setClickedFolderId(null)
        setVisibleFolderIndices([])
        removeFolderTransitionCSS()
      })
    } else {
      props.onClose()
    }
  }

  // Focus input when editing starts
  createEffect(() => {
    if (isEditing() && inputRef) {
      inputRef.focus()
      inputRef.select()
    }
  })

  const handleSave = () => {
    const newName = editValue().trim()
    if (newName && newName !== groupName) {
      renameGroupMutation.mutate({
        group: groupId,
        name: newName
      })
    }
    setIsEditing(false)
  }

  const handleCancel = () => {
    setIsEditing(false)
    setEditValue("")
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSave()
    } else if (e.key === "Escape") {
      handleCancel()
    }
  }

  const handleStartEdit = () => {
    if (props.isDefaultGroup) return
    setEditValue(groupName)
    setIsEditing(true)
  }

  // Escape key handler - overlay display is managed by toggleFolder in HomeGrid
  onMount(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !isEditing()) {
        handleClose()
      }
    }
    window.addEventListener("keydown", handleEscape)

    onCleanup(() => {
      window.removeEventListener("keydown", handleEscape)
      // Hide the overlay unless a modal is still open on top of it
      if (!modalsContext?.hasOpenModals()) {
        const overlay = document.getElementById("overlay")
        if (overlay) {
          overlay.style.display = "none"
        }
      }
    })
  })

  return (
    <Portal mount={document.getElementById("overlay")!}>
      {/* Full viewport container */}
      <div class="pointer-events-auto absolute inset-0 z-50 flex h-screen w-screen">
        {/* Centering container - grows to fill available space */}
        <div
          class="relative flex h-full grow items-center justify-center"
          onClick={() => {
            if (!dragContext.isDragging() && !dragContext.justDropped()) {
              handleClose()
            }
          }}
        >
          {/* Backdrop - also serves as drop zone to move instances to root */}
          <BackdropDropZone onClose={handleClose} folderId={groupId} />

          {/* Overlay content - centered via parent flex */}
          <div
            ref={scrollContainerRef}
            class="bg-darkSlate-800 border-darkSlate-600 relative z-10 h-3/5 max-h-[600px] w-3/5 max-w-3xl overflow-auto rounded-lg border p-6 backdrop-blur-sm"
            onClick={(e) => e.stopPropagation()}
            on:mousedown={(e) => {
              // Drag-select handler on scroll container (outside ContextMenuTrigger)
              // to avoid event interference from Kobalte's trigger wrappers
              if (e.button !== 0) return
              const target = e.target as HTMLElement
              if (
                target.closest("[data-instance-tile]") ||
                target.closest("[data-server-tile]") ||
                target.closest("button") ||
                target.closest("input") ||
                target.closest("[data-kb-menu]") ||
                target.closest("[role='menu']")
              ) return
              e.stopPropagation()
              dragSelect.handlers.handleMouseDown(e)
            }}
            style={
              clickedFolderId() === groupId
                ? { "view-transition-name": "folder-tile" }
                : {}
            }
          >
            {/* Header */}
            <FolderHeader
              groupId={groupId}
              groupName={groupName}
              instanceCount={() => safeInstances().length}
              isDefaultGroup={props.isDefaultGroup}
              isEditing={isEditing}
              editValue={editValue}
              onStartEdit={handleStartEdit}
              onSave={handleSave}
              onEditValueChange={setEditValue}
              onKeyDown={handleKeyDown}
              onSort={(sortBy) =>
                arrangeGroupMutation.mutate({ group: groupId, sortBy })
              }
              onClose={handleClose}
              viewTransitionName={
                clickedFolderId() === groupId ? "folder-name" : undefined
              }
              inputRef={(el) => {
                inputRef = el
              }}
            />

            {/* Instance grid with context menu for creating new instances */}
            <ContextMenu>
              <ContextMenuTrigger class="flex-1">
                <div
                  ref={folderGridEl}
                  class={`min-h-[100px] overflow-visible ${TILE_SIZES[props.tileSize]?.gapY ?? "gap-y-6"}`}
                  style={{
                    display: "grid",
                    "grid-template-columns": `repeat(auto-fill, ${TILE_SIZES[props.tileSize]?.widthPx ?? 184}px)`,
                    "justify-content": "space-evenly",
                    "column-gap": "16px"
                  }}
                >
                  <For each={safeInstances()}>
                    {(instance, index) => {
                      const idPrefix = props.isServerMode ? "server" : "instance"
                      const instanceStringId = `${idPrefix}-${instance.id}`
                      const isBeingDragged = () =>
                        (dragContext.isDragging() ||
                          dragContext.justDropped()) &&
                        dragContext.dragDetached() &&
                        (dragContext.dragType() === "instance" || dragContext.dragType() === "server") &&
                        dragContext.draggedIds().includes(instance.id)

                      const isSelected = () =>
                        props.selectedIds.has(instanceStringId)

                      // Compute active drop target for this instance
                      // Use captured groupId to avoid stale props access during unmount
                      const activeDropTarget = createMemo(() => {
                        const target = dragContext.dropTarget()
                        if (!target) return null
                        if (
                          target.type === "beforeInstance" &&
                          (target as { instanceId: number }).instanceId ===
                            instance.id &&
                          (target as { groupId: number }).groupId === groupId
                        ) {
                          return target
                        }
                        return null
                      })

                      // Only visible instances get folder-preview transition names for animation
                      // Use captured groupId to avoid stale props access during unmount
                      const getFolderPreviewStyle = () => {
                        if (
                          clickedFolderId() === groupId &&
                          visibleFolderIndices().includes(index())
                        ) {
                          return {
                            "view-transition-name": `folder-preview-${index()}`
                          }
                        }
                        return {}
                      }

                      return (
                        <>
                          {/* Drop preview tile */}
                          <Show when={activeDropTarget()}>
                            {(target) => (
                              <DropPreviewTile
                                tileSize={() => props.tileSize}
                                dropTarget={target()}
                                scope={`folder-${groupId}`}
                              />
                            )}
                          </Show>
                          {/* Remove dragged tile from DOM so the grid collapses the gap */}
                          <Show when={!isBeingDragged()}>
                            <div
                              data-instance-tile={!props.isServerMode || undefined}
                              data-server-tile={props.isServerMode || undefined}
                              class="relative"
                              style={getFolderPreviewStyle()}
                              ref={(el) => {
                                if (el) folderTileRefs.set(instanceStringId, el)
                                onCleanup(() =>
                                  folderTileRefs.delete(instanceStringId)
                                )
                              }}
                            >
                              <Show
                                when={props.isServerMode}
                                fallback={
                                  <InstanceTile
                                    instance={instance as ListInstance}
                                    identifier={`folder-${groupId}-${instance.id}`}
                                    size={props.tileSize}
                                    isMultiSelected={isSelected()}
                                    onToggleSelection={() =>
                                      props.onToggleSelection(instanceStringId)
                                    }
                                    isDragging={isBeingDragged()}
                                    isDragActive={dragContext.isDragging()}
                                    groupId={groupId}
                                    onDragStart={(e) =>
                                      props.onDragStart(
                                        instance.id,
                                        isSelected(),
                                        e
                                      )
                                    }
                                    preventClick={() => dragContext.justDropped()}
                                    selectedCount={props.selectedCount}
                                    onBatchDelete={props.onBatchDelete}
                                    onSelectExclusive={() => props.onSelectExclusive?.(`instance-${instance.id}`)}
                                  />
                                }
                              >
                                <ServerTile
                                  server={instance as unknown as ListServer}
                                  identifier={`folder-${groupId}-${instance.id}`}
                                  size={props.tileSize}
                                  isMultiSelected={isSelected()}
                                  onToggleSelection={() =>
                                    props.onToggleSelection(instanceStringId)
                                  }
                                  isDragging={isBeingDragged()}
                                  isDragActive={dragContext.isDragging()}
                                  groupId={groupId}
                                  onDragStart={(e) =>
                                    props.onDragStart(
                                      instance.id,
                                      isSelected(),
                                      e
                                    )
                                  }
                                  preventClick={dragContext.justDropped()}
                                />
                              </Show>
                            </div>
                          </Show>
                        </>
                      )
                    }}
                  </For>

                  {/* Empty state */}
                  <Show when={safeInstances().length === 0}>
                    <div class="text-darkSlate-400 w-full py-8 text-center">
                      <Trans key="instances:_trn_drag_instances_to_folder" />
                    </div>
                  </Show>

                  {/* End of folder drop zone */}
                  <Show
                    when={
                      (dragContext.isDragging() || dragContext.justDropped()) &&
                      (dragContext.dragType() === "instance" || dragContext.dragType() === "server")
                    }
                  >
                    <EndOfGroupDropZone
                      groupId={groupId}
                      zoneIdPrefix="end-of-folder"
                      tileSize={() => props.tileSize}
                      scope={`folder-${groupId}`}
                    />
                  </Show>
                </div>
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuItem
                  class="flex items-center gap-2"
                  onClick={() => {
                    modalsContext?.openModal(
                      { name: "instanceCreation" },
                      { groupId: groupId }
                    )
                  }}
                >
                  <div class="i-hugeicons:file-add h-4 w-4" />
                  <Trans key="library:_trn_create_new_instance" />
                </ContextMenuItem>
                <ContextMenuItem
                  class="flex items-center gap-2"
                  onClick={() => {
                    modalsContext?.openModal(
                      { name: "instanceCreation" },
                      { import: true, groupId: groupId }
                    )
                  }}
                >
                  <div class="i-hugeicons:download-02 h-4 w-4" />
                  <Trans key="library:_trn_import_instance" />
                </ContextMenuItem>
              </ContextMenuContent>
            </ContextMenu>
          </div>
        </div>

        {/* Ad sidebar placeholder - matches the ad area width for proper centering */}
        <div
          class="animate-fadeIn pointer-events-auto h-screen bg-black/50"
          style={{ width: `${adSize.width}px` }}
          onClick={() => {
            if (!dragContext.isDragging() && !dragContext.justDropped()) {
              handleClose()
            }
          }}
        />

        {/* Selection marquee for folder drag-select - outside scroll container to avoid offset issues */}
        <Show when={dragSelect.selectionRect()}>
          {(rect) => (
            <div
              class="border-primary-500 bg-primary-500/20 pointer-events-none fixed z-[60] border-2"
              style={{
                left: `${rect().left}px`,
                top: `${rect().top}px`,
                width: `${rect().width}px`,
                height: `${rect().height}px`
              }}
            />
          )}
        </Show>
      </div>
    </Portal>
  )
}

export default ExpandedFolderContent
