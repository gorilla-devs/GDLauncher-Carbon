import { createEffect, createMemo, createSignal, onCleanup, Show, For } from "solid-js"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance } from "@gd/core_module/bindings"
import { getInstanceImageUrl } from "@/utils/instances"
import DefaultImg from "/assets/images/default-instance-img.png"

// Module-level signal for tracking which folder is being animated (like clickedInstanceId pattern)
export const [clickedFolderId, setClickedFolderId] = createSignal<number | null>(null)

interface FolderTileProps {
  group: { id: number; name: string; instances: ListInstance[] }
  isOpen: boolean
  onToggle: () => void
  size: 1 | 2 | 3 | 4 | 5
}

const FolderTile = (props: FolderTileProps) => {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  // Check if this folder should have view-transition-name for animation
  // Only set when this folder is clicked AND not open (to avoid duplicate with ExpandedFolderContent)
  const shouldSetViewTransition = () => clickedFolderId() === props.group.id && !props.isOpen

  // Check if this folder is being hovered during drag
  const isOver = createMemo(() => {
    const target = dragContext.dropTarget()
    return target?.type === "dropOnFolder" && target.groupId === props.group.id
  })

  // Check if this folder is being dragged
  const isBeingDragged = createMemo(() =>
    dragContext.isDragging() &&
    dragContext.dragType() === "group" &&
    dragContext.draggedIds().includes(props.group.id)
  )

  // Handle drag start for folder reordering
  const handleDragStart = (e: PointerEvent) => {
    e.stopPropagation()
    // Don't call preventDefault - let clicks work normally
    // DragContext's 5px threshold handles click vs drag distinction
    dragContext.startDrag("group", [props.group.id], e)
  }

  // Register as drop target when dragging instances
  createEffect(() => {
    if (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: `folder-${props.group.id}`,
        rect,
        target: { type: "dropOnFolder", groupId: props.group.id }
      })
    } else {
      dragContext.unregisterDropZone(`folder-${props.group.id}`)
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone(`folder-${props.group.id}`)
  })

  // Get the first 4 instances for the preview grid
  const previewInstances = createMemo(() => props.group.instances.slice(0, 4))

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

  return (
    <div
      ref={ref}
      onClick={(e) => {
        e.stopPropagation()
        props.onToggle()
      }}
      onPointerDown={handleDragStart}
      data-folder-tile
      class={`group relative cursor-pointer rounded-lg bg-darkSlate-700 hover:bg-darkSlate-600 transition-all duration-200 flex flex-col overflow-hidden ${sizeClasses()}`}
      classList={{
        "ring-2 ring-primary-500 bg-primary-500/10": isOver(),
        "opacity-50": isBeingDragged()
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
                style={shouldSetViewTransition() ? { "view-transition-name": `folder-preview-${index}` } : {}}
              >
                <Show
                  when={instance()}
                  fallback={
                    <div class="w-full h-full bg-darkSlate-600" />
                  }
                >
                  {(inst) => (
                    <img
                      src={
                        inst().icon_revision
                          ? getInstanceImageUrl(inst().id, inst().icon_revision)
                          : DefaultImg
                      }
                      alt=""
                      class="w-full h-full object-cover"
                      loading="lazy"
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
          {props.group.name}
        </span>
        <span class="text-xs text-darkSlate-400">
          {props.group.instances.length} {props.group.instances.length === 1 ? "instance" : "instances"}
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
    </div>
  )
}

export default FolderTile
