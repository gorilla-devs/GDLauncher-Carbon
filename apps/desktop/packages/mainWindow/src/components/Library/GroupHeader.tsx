import {
  createSignal,
  Show,
  createEffect,
  onCleanup,
  createMemo
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
  Input
} from "@gd/ui"
import { useDragContext } from "@/pages/Library/DragContext"
import { rspc } from "@/utils/rspcClient"
import { useModal } from "@/managers/ModalsManager"

interface GroupHeaderProps {
  groupId: number
  name: string
  isDefault: boolean
  onToggleCollapse: () => void
  isCollapsed: boolean
}

const GroupHeader = (props: GroupHeaderProps) => {
  const [t] = useTransContext()
  const dragContext = useDragContext()
  const modals = useModal()

  const [isEditing, setIsEditing] = createSignal(false)
  const [editValue, setEditValue] = createSignal("")
  let inputRef: HTMLInputElement | undefined

  const renameGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.renameGroup"]
  }))

  const deleteGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroup"]
  }))

  const moveGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.moveGroup"]
  }))

  // Check if this group is being dragged
  const isBeingDragged = createMemo(
    () =>
      dragContext.isDragging() &&
      dragContext.dragType() === "group" &&
      dragContext.draggedIds().includes(props.groupId)
  )

  // Start editing on double click
  const handleDoubleClick = (e: MouseEvent) => {
    if (props.isDefault) return // Can't rename default group
    e.stopPropagation()
    setEditValue(props.name)
    setIsEditing(true)
  }

  // Focus input when editing starts
  createEffect(() => {
    if (isEditing() && inputRef) {
      inputRef.focus()
      inputRef.select()
    }
  })

  // Handle save
  const handleSave = () => {
    const newName = editValue().trim()
    if (newName && newName !== props.name) {
      renameGroupMutation.mutate({
        group: props.groupId,
        name: newName
      })
    }
    setIsEditing(false)
  }

  // Handle cancel
  const handleCancel = () => {
    setIsEditing(false)
    setEditValue("")
  }

  // Handle key events
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSave()
    } else if (e.key === "Escape") {
      handleCancel()
    }
  }

  // Handle delete
  const handleDelete = () => {
    if (props.isDefault) return

    // Confirm deletion
    modals?.openModal(
      {
        name: "notification"
      },
      {
        title: t("instances:_trn_delete_group_title"),
        message: t("instances:_trn_delete_group_message"),
        type: "warning",
        onConfirm: () => {
          deleteGroupMutation.mutate(props.groupId)
        }
      }
    )
  }

  // Handle drag start for group reordering
  const handleDragStart = (e: PointerEvent) => {
    if (props.isDefault) return // Can't drag default group
    e.stopPropagation()
    e.preventDefault()
    dragContext.startDrag("group", [props.groupId], e)
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          class="w-fit h-8 flex gap-2 items-center cursor-pointer select-none transition-opacity duration-150"
          classList={{
            "opacity-50": isBeingDragged()
          }}
          onClick={props.onToggleCollapse}
        >
          {/* Drag handle - only for non-default groups */}
          <Show when={!props.isDefault}>
            <div
              class="i-ri:drag-move-2-line min-w-4 min-h-4 text-lightSlate-600 hover:text-lightSlate-400 cursor-grab transition-colors"
              onPointerDown={handleDragStart}
              onClick={(e) => e.stopPropagation()}
            />
          </Show>

          {/* Collapse arrow */}
          <div
            class="i-hugeicons:arrow-right-01 min-w-4 min-h-4 transition ease-spring text-lightSlate-700"
            classList={{
              "rotate-90": !props.isCollapsed
            }}
          />

          {/* Group name or edit input */}
          <Show
            when={!isEditing()}
            fallback={
              <Input
                ref={inputRef}
                value={editValue()}
                onInput={(e) => setEditValue(e.currentTarget.value)}
                onKeyDown={handleKeyDown}
                onBlur={handleSave}
                onClick={(e) => e.stopPropagation()}
                class="h-6 text-sm py-0"
              />
            }
          >
            <p
              class="m-0 text-lightSlate-700 flex items-center uppercase text-ellipsis max-w-full text-left text-md"
              onDblClick={handleDoubleClick}
            >
              {props.name}
            </p>
          </Show>
        </div>
      </ContextMenuTrigger>

      <ContextMenuContent>
        <ContextMenuGroup>
          <ContextMenuGroupLabel>{props.name}</ContextMenuGroupLabel>
          <ContextMenuSeparator />

          <Show when={!props.isDefault}>
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={() => {
                setEditValue(props.name)
                setIsEditing(true)
              }}
            >
              <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
              <Trans key="instances:_trn_rename_group" />
            </ContextMenuItem>
          </Show>

          <Show when={!props.isDefault}>
            <ContextMenuItem
              class="flex items-center gap-2 text-red-400"
              onClick={handleDelete}
            >
              <div class="i-hugeicons:delete-02 h-4 w-4" />
              <Trans key="instances:_trn_delete_group" />
            </ContextMenuItem>
          </Show>

          <Show when={props.isDefault}>
            <ContextMenuItem class="flex items-center gap-2 opacity-50" disabled>
              <div class="i-hugeicons:information-circle h-4 w-4" />
              <Trans key="instances:_trn_default_group_info" />
            </ContextMenuItem>
          </Show>
        </ContextMenuGroup>
      </ContextMenuContent>
    </ContextMenu>
  )
}

export default GroupHeader
