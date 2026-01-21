import { Show, createMemo, createEffect } from "solid-js"
import { useDragContext, DropTarget } from "@/pages/Library/DragContext"

interface DropIndicatorProps {
  target: DropTarget
  orientation?: "horizontal" | "vertical"
  position?: { x: number; y: number; width?: number; height?: number }
}

const DropIndicator = (props: DropIndicatorProps) => {
  const dragContext = useDragContext()

  const isActive = createMemo(() => {
    const currentTarget = dragContext.dropTarget()
    if (!currentTarget || !dragContext.isDragging()) return false

    // Compare targets
    if (currentTarget.type !== props.target.type) return false

    switch (currentTarget.type) {
      case "favorites":
        return props.target.type === "favorites"
      case "beforeInstance":
        return (
          props.target.type === "beforeInstance" &&
          currentTarget.instanceId === props.target.instanceId
        )
      case "endOfGroup":
        return (
          props.target.type === "endOfGroup" &&
          currentTarget.groupId === props.target.groupId
        )
      case "beforeGroup":
        return (
          props.target.type === "beforeGroup" &&
          currentTarget.groupId === props.target.groupId
        )
      case "endOfGroups":
        return props.target.type === "endOfGroups"
      default:
        return false
    }
  })

  const orientation = () => props.orientation ?? "horizontal"

  return (
    <Show when={isActive()}>
      <div
        class="absolute z-50 transition-all duration-150 ease-out"
        classList={{
          "h-0.5 left-0 right-0": orientation() === "horizontal",
          "w-0.5 top-0 bottom-0": orientation() === "vertical"
        }}
        style={
          props.position
            ? {
                left: `${props.position.x}px`,
                top: `${props.position.y}px`,
                width: props.position.width ? `${props.position.width}px` : undefined,
                height: props.position.height ? `${props.position.height}px` : undefined
              }
            : undefined
        }
      >
        {/* Main line */}
        <div
          class="absolute inset-0 bg-primary-500 rounded-full"
          classList={{
            "h-0.5": orientation() === "horizontal",
            "w-0.5": orientation() === "vertical"
          }}
        />

        {/* Glow effect */}
        <div
          class="absolute inset-0 bg-primary-500 rounded-full blur-sm opacity-60"
          classList={{
            "h-1 -top-0.25": orientation() === "horizontal",
            "w-1 -left-0.25": orientation() === "vertical"
          }}
        />

        {/* End caps */}
        <Show when={orientation() === "horizontal"}>
          <div class="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-1 w-2 h-2 rounded-full bg-primary-500" />
          <div class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1 w-2 h-2 rounded-full bg-primary-500" />
        </Show>
      </div>
    </Show>
  )
}

interface InstanceDropIndicatorProps {
  instanceId: number
  groupId: number
  position: "before" | "after"
}

export const InstanceDropIndicator = (props: InstanceDropIndicatorProps) => {
  const target: DropTarget =
    props.position === "before"
      ? { type: "beforeInstance", instanceId: props.instanceId, groupId: props.groupId }
      : { type: "endOfGroup", groupId: props.groupId }

  return <DropIndicator target={target} orientation="horizontal" />
}

interface GroupDropIndicatorProps {
  groupId: number
  position: "before" | "after"
}

export const GroupDropIndicator = (props: GroupDropIndicatorProps) => {
  const target: DropTarget =
    props.position === "before"
      ? { type: "beforeGroup", groupId: props.groupId }
      : { type: "endOfGroups" }

  return <DropIndicator target={target} orientation="horizontal" />
}

export default DropIndicator
