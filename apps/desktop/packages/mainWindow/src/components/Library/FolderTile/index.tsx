import {
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  Show,
  For
} from "solid-js"
import { useTransContext } from "@gd/i18n"
import { useDragContext } from "@/pages/Library/DragContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getInstanceImageUrl } from "@/utils/instances"
import { useModal } from "@/managers/ModalsManager"
import {
  Checkbox,
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger
} from "@gd/ui"
import SelectionBorder from "@/components/Instance/SelectionBorder"
import DefaultImg from "/assets/images/default-instance-img.png"
import { TILE_SIZES } from "@/pages/Library/constants"
import {
  clickedFolderId,
  visibleFolderIndices
} from "@/pages/Library/utils/folderViewTransition"
import { DropOverlayIndicator } from "@/pages/Library/components/DropOverlayIndicator"
import type { ListInstance, ListServer } from "@gd/core_module/bindings"

interface FolderTileProps {
  groupId: number
  isOpen: boolean
  onToggle: () => void
  size: 1 | 2 | 3 | 4 | 5
  isSelected?: boolean
  onToggleSelection?: () => void
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: () => void
}

const FolderTile = (props: FolderTileProps) => {
  const [t] = useTransContext()
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  let ref: HTMLDivElement | undefined
  const [isHovering, setIsHovering] = createSignal(false)
  const [isMenuOpen, setIsMenuOpen] = createSignal(false)

  // Detect whether this folder is a server group or instance group
  const isServerGroup = createMemo(
    () =>
      globalStore.serverGroups.data?.some((g) => g.id === props.groupId) ??
      false
  )

  // Look up group from globalStore - creates reactive dependency
  const group = createMemo(() =>
    isServerGroup()
      ? globalStore.serverGroups.data?.find((g) => g.id === props.groupId)
      : globalStore.instanceGroups.data?.find((g) => g.id === props.groupId)
  )

  const groupName = createMemo(() => {
    const g = group()
    if (!g) return ""
    return g.name === "localize➽default" ? t("general:_trn_default") : g.name
  })

  // Get items for this group - REACTIVE DEPENDENCY on globalStore
  const groupInstances = createMemo(() =>
    isServerGroup()
      ? (globalStore.servers.data || []).filter(
          (s) => s.groupId === props.groupId
        )
      : (globalStore.instances.data || []).filter(
          (i) => i.group_id === props.groupId
        )
  )

  // Preview items - now reactive via groupInstances()
  const previewInstances = createMemo(() => groupInstances().slice(0, 4))

  // Check if this folder should have view-transition-name for animation
  // Only set when this folder is clicked AND not open (to avoid duplicate with ExpandedFolderContent)
  const shouldSetViewTransition = () =>
    clickedFolderId() === props.groupId && !props.isOpen

  // Check if this folder is being hovered during drag
  const isOver = createMemo(() => {
    const target = dragContext.dropTarget()
    return target?.type === "dropOnFolder" && target.groupId === props.groupId
  })

  // Check if this folder is being dragged
  const isBeingDragged = createMemo(
    () =>
      dragContext.isDragging() &&
      dragContext.dragDetached() &&
      dragContext.dragType() === "group" &&
      dragContext.draggedIds().includes(props.groupId)
  )

  // Handle drag start for folder reordering
  const handleDragStart = (e: PointerEvent) => {
    e.stopPropagation()
    // Don't call preventDefault - let clicks work normally
    // DragContext's 5px threshold handles click vs drag distinction
    dragContext.startDrag("group", [props.groupId], e)
  }

  // Register as drop target when dragging instances or servers
  createEffect(() => {
    const dtype = dragContext.dragType()
    if (
      dragContext.isDragging() &&
      (dtype === "instance" || dtype === "server") &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      // Shrink drop zone to right 60% (excludes left edge for beforeInstanceAtFolder)
      const dropRect = new DOMRect(
        rect.left + rect.width * 0.4,
        rect.top,
        rect.width * 0.6,
        rect.height
      )
      dragContext.registerDropZone({
        id: `folder-${props.groupId}`,
        rect: dropRect,
        element: ref,
        rectTransform: (r) =>
          new DOMRect(r.left + r.width * 0.4, r.top, r.width * 0.6, r.height),
        target: { type: "dropOnFolder", groupId: props.groupId }
      })
    } else {
      dragContext.unregisterDropZone(`folder-${props.groupId}`)
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone(`folder-${props.groupId}`)
  })

  // Size classes matching InstanceTile sizes
  const sizeClasses = () => TILE_SIZES[props.size].container
  const iconSizeClasses = () => TILE_SIZES[props.size].icon

  // Context menu handler - opens batch folder deletion modal with unlink/delete options
  const handleManageFolder = () => {
    const g = group()
    if (!g) return
    modalsContext?.openModal(
      { name: "confirmBatchFolderDeletion" },
      { folders: [g], onComplete: () => {} }
    )
  }

  return (
    <ContextMenu
      onOpenChange={(open) => {
        setIsMenuOpen(open)
        if (open && !props.isSelected && props.onSelectExclusive) {
          props.onSelectExclusive()
        }
      }}
    >
      <ContextMenuContent>
        <Show
          when={props.isSelected && (props.selectedCount ?? 0) > 1}
          fallback={
            <ContextMenuGroup>
              <ContextMenuGroupLabel>{groupName()}</ContextMenuGroupLabel>
              <ContextMenuSeparator />
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleManageFolder}
              >
                <div class="i-hugeicons:folder-remove h-4 w-4" />
                {t("instances:_trn_unlink_folder")}
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem
                class="flex items-center gap-2 text-red-400"
                onClick={handleManageFolder}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4" />
                {t("instances:_trn_delete_folder_all")}
              </ContextMenuItem>
            </ContextMenuGroup>
          }
        >
          <ContextMenuGroup>
            <ContextMenuGroupLabel>
              {t("content:_trn_selected_count", { count: props.selectedCount })}
            </ContextMenuGroupLabel>
            <ContextMenuSeparator />
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={() => props.onBatchDelete?.()}
            >
              <div class="i-hugeicons:delete-02 h-4 w-4" />
              {t("content:_trn_delete_selected")}
            </ContextMenuItem>
          </ContextMenuGroup>
        </Show>
      </ContextMenuContent>

      <ContextMenuTrigger>
        <div class="relative">
          <SelectionBorder
            isSelected={props.isSelected ?? false}
            size={props.size}
          />
          <div class="box-border overflow-hidden rounded-2xl p-[2px]">
            <div
              ref={ref}
              onMouseEnter={() => setIsHovering(true)}
              onMouseLeave={() => setIsHovering(false)}
              onClick={(e) => {
                e.stopPropagation()
                // Don't toggle if we just finished a drag operation
                if (dragContext.justDropped()) return
                props.onToggle()
              }}
              onPointerDown={(e) => {
                if (e.button === 0 && isMenuOpen()) {
                  document.body.dispatchEvent(
                    new PointerEvent("pointerdown", { bubbles: true })
                  )
                }
                handleDragStart(e)
              }}
              data-folder-tile
              class={`bg-darkSlate-700 hover:bg-darkSlate-600 group relative flex cursor-pointer flex-col overflow-hidden rounded-2xl transition-all duration-200 ${sizeClasses()}`}
              classList={{
                "opacity-0": isBeingDragged() || props.isOpen,
                "!bg-darkSlate-600": isMenuOpen()
              }}
              style={
                shouldSetViewTransition()
                  ? { "view-transition-name": "folder-tile" }
                  : {}
              }
            >
              <div class="relative flex-1">
                {/* 2x2 preview grid */}
                <div class="absolute inset-0 grid grid-cols-2 grid-rows-2 gap-1 p-2">
                  <For each={[0, 1, 2, 3]}>
                    {(index) => {
                      const instance = () => previewInstances()[index]
                      return (
                        <div
                          class={`bg-darkSlate-800 flex items-center justify-center overflow-hidden rounded ${iconSizeClasses()}`}
                          style={{
                            ...(shouldSetViewTransition() && instance()
                              ? {
                                  "view-transition-name": `folder-preview-${index}`
                                }
                              : {}),
                            ...(!instance() && shouldSetViewTransition()
                              ? {
                                  animation:
                                    "fadeInEmptySlot 200ms ease-out 100ms forwards"
                                }
                              : {})
                          }}
                        >
                          <Show when={instance()}>
                            {(inst) => {
                              const iconRev = () =>
                                "icon_revision" in inst()
                                  ? (inst() as ListInstance).icon_revision
                                  : (inst() as ListServer).iconRevision
                              return (
                                <img
                                  src={
                                    iconRev() !== null &&
                                    iconRev() !== undefined
                                      ? getInstanceImageUrl(
                                          inst().id,
                                          iconRev()!
                                        )
                                      : DefaultImg
                                  }
                                  alt=""
                                  class="h-full w-full object-cover"
                                />
                              )
                            }}
                          </Show>
                        </div>
                      )
                    }}
                  </For>
                </div>

                {/* Folder name overlay */}
                <div
                  class="z-4 absolute bottom-0 left-0 right-0 rounded-b-2xl bg-gradient-to-t from-black/80 via-black/40 to-transparent px-3 pb-6 pt-3"
                  style={
                    shouldSetViewTransition()
                      ? { "view-transition-name": "folder-name" }
                      : {}
                  }
                >
                  <h4 class="m-0 truncate text-sm font-semibold text-white">
                    {groupName()}
                  </h4>
                </div>
              </div>

              {/* Drag handle */}
              <div
                class={`i-ri:drag-move-2-line text-darkSlate-400 hover:text-lightSlate-400 absolute left-1 top-1 cursor-grab text-sm transition-colors group-hover:opacity-100 ${isMenuOpen() ? "opacity-100" : "opacity-0"}`}
                onPointerDown={handleDragStart}
                onClick={(e) => e.stopPropagation()}
              />

              {/* Selection checkbox */}
              <Show when={props.onToggleSelection}>
                <div
                  class="ease-spring absolute left-2 top-2 z-10 transition-all duration-200"
                  classList={{
                    "translate-x-0 opacity-100":
                      props.isSelected ||
                      (isHovering() && !dragContext.isDragging()),
                    "-translate-x-3 opacity-0":
                      !props.isSelected &&
                      (!isHovering() || dragContext.isDragging())
                  }}
                  onPointerDown={(e) => {
                    e.stopPropagation()
                    if (isMenuOpen()) {
                      document.body.dispatchEvent(
                        new PointerEvent("pointerdown", { bubbles: true })
                      )
                    }
                  }}
                  onClick={(e) => {
                    e.stopPropagation()
                    e.preventDefault()
                    props.onToggleSelection?.()
                  }}
                >
                  <Checkbox checked={props.isSelected} hover={false} />
                </div>
              </Show>

              {/* Folder icon indicator - only when open */}
              <Show when={props.isOpen}>
                <div class="absolute right-1 top-1">
                  <div class="i-hugeicons:folder-01 text-primary-400 text-sm" />
                </div>
              </Show>

              {/* Drop indicator when hovering */}
              <DropOverlayIndicator
                isVisible={isOver()}
                icon="i-hugeicons:add-circle"
              />

              {/* Hidden stubs for visible instances 5+ to animate to/from */}
              <Show when={shouldSetViewTransition()}>
                <For each={visibleFolderIndices().filter((i) => i >= 4)}>
                  {(index) => (
                    <div
                      class="pointer-events-none absolute left-1/2 top-1/2 h-0 w-0"
                      style={{
                        "view-transition-name": `folder-preview-${index}`
                      }}
                    />
                  )}
                </For>
              </Show>
            </div>
          </div>
        </div>
      </ContextMenuTrigger>
    </ContextMenu>
  )
}

export default FolderTile
