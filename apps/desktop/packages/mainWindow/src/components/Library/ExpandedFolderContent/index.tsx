import {
  For,
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount
} from "solid-js"
import { Portal } from "solid-js/web"
import {
  Button,
  Input,
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { ListInstance } from "@gd/core_module/bindings"
import InstanceTile from "@/components/InstanceTile"
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
} from "../FolderTile"
import { getInstanceImageUrl } from "@/utils/instances"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"

interface ExpandedFolderContentProps {
  group: { id: number; name: string; instances: ListInstance[] }
  onClose: () => void
  tileSize: 1 | 2 | 3 | 4 | 5
  isDefaultGroup: boolean
  selectedIds: Set<number>
  onToggleSelection: (id: number) => void
  onSetSelection: (ids: number[]) => void
  onDragStart: (
    instanceId: number,
    isSelected: boolean,
    e: PointerEvent
  ) => void
}

// Backdrop drop zone - dropping on it moves instances to root library
const BackdropDropZone = (props: { onClose: () => void }) => {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "ungrouped"
  }

  // Register drop zone when dragging instances
  createEffect(() => {
    if (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: "backdrop-ungrouped",
        rect,
        target: { type: "ungrouped" }
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
      class="absolute inset-0 transition-colors duration-200"
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

// End of folder drop zone component
const EndOfFolderDropZone = (props: { groupId: number }) => {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "endOfGroup" && target.groupId === props.groupId
  }

  // Register drop zone
  createEffect(() => {
    if (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: `end-of-folder-${props.groupId}`,
        rect,
        target: { type: "endOfGroup", groupId: props.groupId }
      })
    } else {
      dragContext.unregisterDropZone(`end-of-folder-${props.groupId}`)
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone(`end-of-folder-${props.groupId}`)
  })

  return (
    <div
      ref={ref}
      class="relative flex items-center justify-center min-w-16 h-24 rounded-lg transition-all duration-200"
      classList={{
        "border-2 border-dashed border-darkSlate-500": !isOver(),
        "border-2 border-solid border-primary-500 bg-primary-500/10": isOver()
      }}
    >
      <Show when={isOver()}>
        <div class="absolute -left-2.5 top-0 bottom-0 w-1.5 z-50 flex flex-col items-center">
          <div class="w-3 h-3 rounded-full bg-primary-500 -mt-1.5 shadow-lg shadow-primary-500/50" />
          <div class="flex-1 w-1 bg-gradient-to-b from-primary-500 via-primary-400 to-primary-500 rounded-full shadow-lg shadow-primary-500/40" />
          <div class="w-3 h-3 rounded-full bg-primary-500 -mb-1.5 shadow-lg shadow-primary-500/50" />
        </div>
      </Show>
      <div
        class="i-hugeicons:plus text-lg transition-colors"
        classList={{
          "text-darkSlate-500": !isOver(),
          "text-primary-500": isOver()
        }}
      />
    </div>
  )
}

const ExpandedFolderContent = (props: ExpandedFolderContentProps) => {
  const [t] = useTransContext()
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  const [isEditing, setIsEditing] = createSignal(false)
  const [editValue, setEditValue] = createSignal("")
  let inputRef: HTMLInputElement | undefined
  let scrollContainerRef: HTMLDivElement | undefined

  // Tile refs for folder instances (for drag selection)
  const folderTileRefs = new Map<number, HTMLDivElement>()

  // Get item rects for drag select
  const getItemRects = (): Map<number, DOMRect> => {
    const rects = new Map<number, DOMRect>()
    folderTileRefs.forEach((el, id) => {
      if (el) rects.set(id, el.getBoundingClientRect())
    })
    return rects
  }

  // Local drag select hook for folder content
  const dragSelect = useDragSelect({
    containerRef: () => scrollContainerRef,
    getItemRects,
    onSelectionChange: (ids) => props.onSetSelection(ids)
  })

  const renameGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.renameGroup"]
  }))

  const sortGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.sortGroup"]
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
      setClickedFolderId(props.group.id)

      // Preload preview images (first 4) into browser cache before transition
      const previewInstances = props.group.instances.slice(0, 4)
      await Promise.all(
        previewInstances.map((inst) => {
          if (!inst.icon_revision) return Promise.resolve()
          return new Promise<void>((resolve) => {
            const img = new Image()
            img.onload = () => resolve()
            img.onerror = () => resolve()
            img.src = getInstanceImageUrl(inst.id, inst.icon_revision)
            setTimeout(resolve, 150) // Don't wait forever
          })
        })
      )

      // Wait for SolidJS to flush DOM updates before capturing OLD snapshot
      await new Promise((resolve) => queueMicrotask(resolve))

      const transition = document.startViewTransition(async () => {
        props.onClose()
        // Wait for SolidJS to render FolderTile's images before capturing NEW snapshot
        await new Promise((resolve) => queueMicrotask(resolve))
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
    if (newName && newName !== props.group.name) {
      renameGroupMutation.mutate({
        group: props.group.id,
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
    setEditValue(props.group.name)
    setIsEditing(true)
  }

  // Escape key handler and overlay visibility management
  onMount(() => {
    const overlay = document.getElementById("overlay")
    if (overlay) {
      overlay.style.display = "flex"
    }

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !isEditing()) {
        handleClose()
      }
    }
    window.addEventListener("keydown", handleEscape)

    onCleanup(() => {
      window.removeEventListener("keydown", handleEscape)
      if (overlay) {
        overlay.style.display = "none"
      }
    })
  })

  return (
    <Portal mount={document.getElementById("overlay")!}>
      {/* Full viewport container */}
      <div class="absolute inset-0 z-50 flex h-screen w-screen">
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
          <BackdropDropZone onClose={handleClose} />

          {/* Overlay content - centered via parent flex */}
          <div
            ref={scrollContainerRef}
            class="relative z-10 w-[60%] h-[60%] max-w-3xl max-h-[500px] bg-darkSlate-800 backdrop-blur-sm rounded-lg p-6 border border-darkSlate-600 overflow-auto"
            onClick={(e) => e.stopPropagation()}
            style={
              clickedFolderId() === props.group.id
                ? { "view-transition-name": "folder-tile" }
                : {}
            }
          >
            {/* Header */}
            <div class="flex justify-between items-center mb-4">
              <div class="flex items-center gap-2">
                <div class="i-hugeicons:folder-01 text-primary-400" />
                <Show
                  when={!isEditing()}
                  fallback={
                    <Input
                      ref={inputRef}
                      value={editValue()}
                      onInput={(e) => setEditValue(e.currentTarget.value)}
                      onKeyDown={handleKeyDown}
                      onBlur={handleSave}
                      class="h-7 text-base py-0 w-48"
                    />
                  }
                >
                  <h3
                    class="text-lg font-medium text-lightSlate-100 cursor-pointer hover:text-lightSlate-50"
                    classList={{
                      "cursor-default": props.isDefaultGroup
                    }}
                    onDblClick={handleStartEdit}
                  >
                    {props.group.name}
                  </h3>
                </Show>
                <span class="text-sm text-darkSlate-400">
                  ({props.group.instances.length})
                </span>
              </div>
              <div class="flex items-center gap-2">
                {/* Sort dropdown */}
                <DropdownMenu>
                  <DropdownMenuTrigger>
                    <Button
                      variant="ghost"
                      size="small"
                      title={t("instances:_trn_rearrange")}
                    >
                      <div class="i-hugeicons:arrow-up-down w-4 h-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent>
                    <DropdownMenuLabel>
                      <Trans key="instances:_trn_rearrange" />
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => sortGroupMutation.mutate({ group: props.group.id, sortBy: "name" })}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:text h-4 w-4" />
                        <Trans key="ui:_trn_by_name" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => sortGroupMutation.mutate({ group: props.group.id, sortBy: "lastPlayed" })}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:clock-01 h-4 w-4" />
                        <Trans key="ui:_trn_by_last_played" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => sortGroupMutation.mutate({ group: props.group.id, sortBy: "mostPlayed" })}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:time-02 h-4 w-4" />
                        <Trans key="ui:_trn_by_most_played" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => sortGroupMutation.mutate({ group: props.group.id, sortBy: "dateCreated" })}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:calendar-add-01 h-4 w-4" />
                        <Trans key="ui:_trn_by_date_created" />
                      </div>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
                <Show when={!props.isDefaultGroup}>
                  <Button
                    variant="ghost"
                    size="small"
                    onClick={handleStartEdit}
                    title={t("instances:_trn_rename_group")}
                  >
                    <div class="i-hugeicons:pencil-edit-01 w-4 h-4" />
                  </Button>
                </Show>
                <Button variant="ghost" size="small" onClick={handleClose}>
                  <div class="i-hugeicons:cancel-01 w-4 h-4" />
                </Button>
              </div>
            </div>

            {/* Instance grid with context menu for creating new instances */}
            <ContextMenu>
              <ContextMenuTrigger class="flex-1">
                <div
                  class="flex flex-wrap gap-4 min-h-[100px]"
                  classList={{
                    "gap-y-4": props.tileSize === 1,
                    "gap-y-6": props.tileSize === 2,
                    "gap-y-8": props.tileSize === 3,
                    "gap-y-10": props.tileSize === 4,
                    "gap-y-12": props.tileSize === 5
                  }}
                  onMouseDown={(e) => {
                    // Only start drag select on left click in empty space
                    if (e.button !== 0) return
                    const target = e.target as HTMLElement
                    if (target.closest("[data-instance-tile]")) return
                    // Stop propagation to prevent root-level HomeGrid drag-select from triggering
                    e.stopPropagation()
                    dragSelect.handlers.handleMouseDown(e)
                  }}
                >
                  <For each={props.group.instances}>
                    {(instance, index) => {
                      const isBeingDragged = () =>
                        dragContext.isDragging() &&
                        dragContext.dragType() === "instance" &&
                        dragContext.draggedIds().includes(instance.id)

                      const isSelected = () => props.selectedIds.has(instance.id)

                      // Only visible instances get folder-preview transition names for animation
                      const getFolderPreviewStyle = () => {
                        if (clickedFolderId() === props.group.id && visibleFolderIndices().includes(index())) {
                          return {
                            "view-transition-name": `folder-preview-${index()}`
                          }
                        }
                        return {}
                      }

                      return (
                        <div
                          data-instance-tile
                          class="relative"
                          style={getFolderPreviewStyle()}
                          ref={(el) => {
                            if (el) folderTileRefs.set(instance.id, el)
                            onCleanup(() => folderTileRefs.delete(instance.id))
                          }}
                        >
                          <InstanceTile
                            instance={instance}
                            identifier={`folder-${props.group.id}-${instance.id}`}
                            size={props.tileSize as any}
                            isMultiSelected={isSelected()}
                            onToggleSelection={() =>
                              props.onToggleSelection(instance.id)
                            }
                            isDragging={isBeingDragged()}
                            isDragActive={dragContext.isDragging()}
                            groupId={props.group.id}
                            onDragStart={(e) =>
                              props.onDragStart(instance.id, isSelected(), e)
                            }
                            preventClick={() => dragContext.justDropped()}
                          />
                        </div>
                      )
                    }}
                  </For>

                  {/* Empty state */}
                  <Show when={props.group.instances.length === 0}>
                    <div class="text-darkSlate-400 text-center w-full py-8">
                      <Trans key="instances:_trn_drag_instances_to_folder" />
                    </div>
                  </Show>

                  {/* End of folder drop zone */}
                  <Show
                    when={
                      dragContext.isDragging() &&
                      dragContext.dragType() === "instance"
                    }
                  >
                    <EndOfFolderDropZone groupId={props.group.id} />
                  </Show>
                </div>
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuItem
                  class="flex items-center gap-2"
                  onClick={() => {
                    modalsContext?.openModal(
                      { name: "instanceCreation" },
                      { groupId: props.group.id }
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
                      { import: true, groupId: props.group.id }
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
          class="h-screen bg-black/50 animate-fadeIn"
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
              class="fixed pointer-events-none border-2 border-primary-500 bg-primary-500/20 z-[60]"
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
