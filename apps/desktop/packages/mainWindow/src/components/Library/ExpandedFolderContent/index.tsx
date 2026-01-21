import {
  For,
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount
} from "solid-js"
import { Portal } from "solid-js/web"
import { Button, Input } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { ListInstance } from "@gd/core_module/bindings"
import InstanceTile from "@/components/InstanceTile"
import { useDragContext } from "@/pages/Library/DragContext"
import { rspc } from "@/utils/rspcClient"
import adSize from "@/utils/adhelper"
import { clickedFolderId, setClickedFolderId } from "../FolderTile"
import { useGlobalStore } from "@/components/GlobalStoreContext"

interface ExpandedFolderContentProps {
  group: { id: number; name: string; instances: ListInstance[] }
  onClose: () => void
  tileSize: 1 | 2 | 3 | 4 | 5
  isDefaultGroup: boolean
  selectedIds: Set<number>
  onToggleSelection: (id: number) => void
  onDragStart: (
    instanceId: number,
    isSelected: boolean,
    e: PointerEvent
  ) => void
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
        <div class="absolute -left-2 top-0 bottom-0 w-1 bg-primary-500 rounded-full z-50">
          <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
          <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
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
  const [isEditing, setIsEditing] = createSignal(false)
  const [editValue, setEditValue] = createSignal("")
  let inputRef: HTMLInputElement | undefined

  const renameGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.renameGroup"]
  }))

  // Handle close with view transition
  const handleClose = () => {
    const shouldTransition =
      !globalStore.settings.data?.reducedMotion && document.startViewTransition

    if (shouldTransition) {
      setClickedFolderId(props.group.id)
      const transition = document.startViewTransition(() => props.onClose())
      transition.finished.then(() => setClickedFolderId(null))
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
          onClick={handleClose}
        >
          {/* Backdrop */}
          <div
            class="absolute inset-0 bg-black/50"
            onClick={(e) => {
              e.stopPropagation()
              handleClose()
            }}
          />

          {/* Overlay content - centered via parent flex */}
          <div
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

            {/* Instance grid */}
            <div
              class="flex flex-wrap gap-4"
              classList={{
                "gap-y-4": props.tileSize === 1,
                "gap-y-6": props.tileSize === 2,
                "gap-y-8": props.tileSize === 3,
                "gap-y-10": props.tileSize === 4,
                "gap-y-12": props.tileSize === 5
              }}
            >
              <For each={props.group.instances}>
                {(instance, index) => {
                  const isBeingDragged = () =>
                    dragContext.isDragging() &&
                    dragContext.dragType() === "instance" &&
                    dragContext.draggedIds().includes(instance.id)

                  const isSelected = () => props.selectedIds.has(instance.id)

                  // Only first 4 instances get folder-preview transition names
                  const getFolderPreviewStyle = () => {
                    if (clickedFolderId() === props.group.id && index() < 4) {
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
          </div>
        </div>

        {/* Ad sidebar placeholder - matches the ad area width for proper centering */}
        <div
          class="h-screen bg-black/50 animate-fadeIn"
          style={{ width: `${adSize.width}px` }}
          onClick={handleClose}
        />
      </div>
    </Portal>
  )
}

export default ExpandedFolderContent
