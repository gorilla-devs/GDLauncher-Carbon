import { createEffect, createMemo, createSignal, onCleanup, Show, For } from "solid-js"
import { useTransContext } from "@gd/i18n"
import { useDragContext } from "@/pages/Library/DragContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getInstanceImageUrl } from "@/utils/instances"
import { rspc } from "@/utils/rspcClient"
import { useModal } from "@/managers/ModalsManager"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger
} from "@gd/ui"
import DefaultImg from "/assets/images/default-instance-img.png"

// Module-level signal for tracking which folder is being animated (like clickedInstanceId pattern)
export const [clickedFolderId, setClickedFolderId] = createSignal<number | null>(null)

// Track which instance indices are visible in the expanded folder viewport
export const [visibleFolderIndices, setVisibleFolderIndices] = createSignal<number[]>([])

// Module-level style element reference for dynamic CSS injection
let dynamicStyleElement: HTMLStyleElement | null = null

export function injectFolderTransitionCSS(indices: number[], direction: "open" | "close") {
  // Remove existing dynamic styles
  if (dynamicStyleElement) {
    dynamicStyleElement.remove()
  }

  if (indices.length === 0) return

  const groupSelectors = indices.map(i => `::view-transition-group(folder-preview-${i})`).join(",\n")
  const oldNewSelectors = indices.flatMap(i => [
    `::view-transition-old(folder-preview-${i})`,
    `::view-transition-new(folder-preview-${i})`
  ]).join(",\n")

  // Direction-specific CSS for folder-tile old/new snapshots
  const folderTileCSS = direction === "close"
    ? `
      /* On close: keep old (expanded) visible while morphing */
      ::view-transition-old(folder-tile) {
        opacity: 1 !important;
      }
      ::view-transition-new(folder-tile) {
        opacity: 0;
      }
    `
    : `
      /* On open: fade out old (collapsed), show new (expanded) */
      ::view-transition-old(folder-tile) {
        opacity: 0;
      }
      ::view-transition-new(folder-tile) {
        opacity: 1;
      }
    `

  // Keyframes for empty slot fade-in during close animation
  const emptySlotKeyframes = direction === "close" ? `
    @keyframes fadeInEmptySlot {
      from { opacity: 0; }
      to { opacity: 1; }
    }
  ` : ""

  const css = `
    ${emptySlotKeyframes}
    ${groupSelectors} {
      animation-duration: 300ms;
      animation-timing-function: cubic-bezier(0.32, 0.72, 0, 1);
      z-index: 1;
    }
    ${oldNewSelectors} {
      animation-duration: 300ms;
      animation-timing-function: cubic-bezier(0.32, 0.72, 0, 1);
      mix-blend-mode: normal;
    }
    ${folderTileCSS}
  `

  dynamicStyleElement = document.createElement("style")
  dynamicStyleElement.textContent = css
  document.head.appendChild(dynamicStyleElement)
}

export function removeFolderTransitionCSS() {
  if (dynamicStyleElement) {
    dynamicStyleElement.remove()
    dynamicStyleElement = null
  }
}

interface FolderTileProps {
  groupId: number
  isOpen: boolean
  onToggle: () => void
  size: 1 | 2 | 3 | 4 | 5
}

const FolderTile = (props: FolderTileProps) => {
  const [t] = useTransContext()
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  let ref: HTMLDivElement | undefined

  const deleteGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroup"]
  }))

  const deleteGroupWithInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroupWithInstances"]
  }))

  // Look up group from globalStore - creates reactive dependency
  const group = createMemo(() =>
    globalStore.instanceGroups.data?.find((g) => g.id === props.groupId)
  )

  const groupName = createMemo(() => {
    const g = group()
    if (!g) return ""
    return g.name === "localize➽default" ? t("general:_trn_default") : g.name
  })

  // Get instances for this group - REACTIVE DEPENDENCY on globalStore.instances
  const groupInstances = createMemo(() =>
    (globalStore.instances.data || []).filter(
      (i) => i.group_id === props.groupId && !i.favorite
    )
  )

  // Preview instances - now reactive via groupInstances()
  const previewInstances = createMemo(() => groupInstances().slice(0, 4))

  const instanceCount = createMemo(() => groupInstances().length)

  // Check if this folder should have view-transition-name for animation
  // Only set when this folder is clicked AND not open (to avoid duplicate with ExpandedFolderContent)
  const shouldSetViewTransition = () => clickedFolderId() === props.groupId && !props.isOpen

  // Check if this folder is being hovered during drag
  const isOver = createMemo(() => {
    const target = dragContext.dropTarget()
    return target?.type === "dropOnFolder" && target.groupId === props.groupId
  })

  // Check if this folder is being dragged
  const isBeingDragged = createMemo(() =>
    dragContext.isDragging() &&
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

  // Register as drop target when dragging instances
  createEffect(() => {
    if (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      // Shrink drop zone to right 75% (excludes left edge for beforeInstanceAtFolder)
      const dropRect = new DOMRect(
        rect.left + rect.width * 0.25,
        rect.top,
        rect.width * 0.75,
        rect.height
      )
      dragContext.registerDropZone({
        id: `folder-${props.groupId}`,
        rect: dropRect,
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
  const sizeClasses = createMemo(() => {
    switch (props.size) {
      case 1:
        return "h-24 w-24"
      case 2:
        return "h-46 w-46"
      case 3:
        return "h-60 w-60"
      case 4:
        return "h-84 w-84"
      case 5:
        return "h-120 w-120"
      default:
        return "h-46 w-46"
    }
  })

  const iconSizeClasses = createMemo(() => {
    switch (props.size) {
      case 1:
        return "w-10 h-10"
      case 2:
        return "w-20 h-20"
      case 3:
        return "w-26 h-26"
      case 4:
        return "w-38 h-38"
      case 5:
        return "w-56 h-56"
      default:
        return "w-20 h-20"
    }
  })

  // Context menu handlers
  const handleUnlinkAndDelete = () => {
    modalsContext?.openModal(
      { name: "notification" },
      {
        title: t("instances:_trn_unlink_folder_title"),
        message: t("instances:_trn_unlink_folder_message", { count: instanceCount() }),
        type: "warning",
        onConfirm: () => deleteGroupMutation.mutate(props.groupId)
      }
    )
  }

  const handleDeleteAll = () => {
    modalsContext?.openModal(
      { name: "notification" },
      {
        title: t("instances:_trn_delete_folder_all_title"),
        message: t("instances:_trn_delete_folder_all_message", { count: instanceCount() }),
        type: "warning",
        onConfirm: () => deleteGroupWithInstancesMutation.mutate(props.groupId)
      }
    )
  }

  return (
    <ContextMenu>
      <ContextMenuContent>
        <ContextMenuGroup>
          <ContextMenuGroupLabel>{groupName()}</ContextMenuGroupLabel>
          <ContextMenuSeparator />

          <ContextMenuItem class="flex items-center gap-2" onClick={handleUnlinkAndDelete}>
            <div class="i-hugeicons:folder-remove h-4 w-4" />
            {t("instances:_trn_unlink_folder")}
          </ContextMenuItem>

          <ContextMenuSeparator />

          <ContextMenuItem class="flex items-center gap-2 text-red-400" onClick={handleDeleteAll}>
            <div class="i-hugeicons:delete-02 h-4 w-4" />
            {t("instances:_trn_delete_folder_all")}
          </ContextMenuItem>
        </ContextMenuGroup>
      </ContextMenuContent>

      <ContextMenuTrigger>
        <div
          ref={ref}
          onClick={(e) => {
            e.stopPropagation()
            // Don't toggle if we just finished a drag operation
            if (dragContext.justDropped()) return
            props.onToggle()
          }}
          onPointerDown={handleDragStart}
          data-folder-tile
          class={`group relative cursor-pointer rounded-lg bg-darkSlate-700 hover:bg-darkSlate-600 transition-all duration-200 flex flex-col overflow-hidden ${sizeClasses()}`}
          classList={{
            "ring-2 ring-primary-500 bg-primary-500/10": isOver(),
            "opacity-50": isBeingDragged(),
            "opacity-0": props.isOpen && !isBeingDragged()
          }}
          style={shouldSetViewTransition() ? { "view-transition-name": "folder-tile" } : {}}
        >
          {/* 2x2 preview grid */}
          <div class="flex-1 p-2 grid grid-cols-2 grid-rows-2 gap-1">
            <For each={[0, 1, 2, 3]}>
              {(index) => {
                const instance = () => previewInstances()[index]
                return (
                  <div
                    class={`rounded bg-darkSlate-600 flex items-center justify-center overflow-hidden ${iconSizeClasses()}`}
                    style={{
                      ...(shouldSetViewTransition() && instance() ? { "view-transition-name": `folder-preview-${index}` } : {}),
                      ...(!instance() && shouldSetViewTransition() ? {
                        animation: "fadeInEmptySlot 200ms ease-out 100ms forwards"
                      } : {})
                    }}
                  >
                    <Show when={instance()}>
                      {(inst) => (
                        <img
                          src={
                            inst().icon_revision
                              ? getInstanceImageUrl(inst().id, inst().icon_revision)
                              : DefaultImg
                          }
                          alt=""
                          class="w-full h-full object-cover"
                        />
                      )}
                    </Show>
                  </div>
                )
              }}
            </For>
          </div>

          {/* Folder name */}
          <div class="px-2 pb-2 text-center">
            <span class="text-xs text-lightSlate-200 truncate block">
              {groupName()}
            </span>
            <span class="text-xs text-darkSlate-400">
              {instanceCount()} {instanceCount() === 1 ? "instance" : "instances"}
            </span>
          </div>

          {/* Drag handle */}
          <div
            class="absolute top-1 left-1 i-ri:drag-move-2-line text-sm text-darkSlate-400 hover:text-lightSlate-400 cursor-grab transition-colors opacity-0 group-hover:opacity-100"
            onPointerDown={handleDragStart}
            onClick={(e) => e.stopPropagation()}
          />

          {/* Folder icon indicator */}
          <div class="absolute top-1 right-1">
            <div
              class="i-hugeicons:folder-01 text-sm"
              classList={{
                "text-primary-400": props.isOpen,
                "text-darkSlate-400": !props.isOpen
              }}
            />
          </div>

          {/* Drop indicator when hovering */}
          <Show when={isOver()}>
            <div class="absolute inset-0 border-2 border-primary-500 rounded-lg bg-primary-500/20 pointer-events-none flex items-center justify-center">
              <div class="i-hugeicons:add-circle text-primary-400 text-2xl" />
            </div>
          </Show>

          {/* Hidden stubs for visible instances 5+ to animate to/from */}
          <Show when={shouldSetViewTransition()}>
            <For each={visibleFolderIndices().filter(i => i >= 4)}>
              {(index) => (
                <div
                  class="absolute top-1/2 left-1/2 w-0 h-0 pointer-events-none"
                  style={{ "view-transition-name": `folder-preview-${index}` }}
                />
              )}
            </For>
          </Show>
        </div>
      </ContextMenuTrigger>
    </ContextMenu>
  )
}

export default FolderTile
