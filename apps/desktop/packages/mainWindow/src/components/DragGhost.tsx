import { Show, For, createMemo } from "solid-js"
import { Portal } from "solid-js/web"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance } from "@gd/core_module/bindings"
import DefaultImg from "/assets/images/default-instance-img.png"
import { getInstanceImageUrl } from "@/utils/instances"

interface DragGhostProps {
  instances: ListInstance[]
  groups: { id: number; name: string }[]
}

const DragGhost = (props: DragGhostProps) => {
  const dragContext = useDragContext()

  const draggedItems = createMemo(() => {
    const type = dragContext.dragType()
    const ids = dragContext.draggedIds()

    if (type === "instance") {
      return props.instances.filter((i) => ids.includes(i.id))
    }

    if (type === "group") {
      return props.groups.filter((g) => ids.includes(g.id))
    }

    return []
  })

  const ghostPosition = createMemo(() => {
    const pos = dragContext.ghostPosition()
    return {
      left: `${pos.x - 10}px`,
      top: `${pos.y - 10}px`
    }
  })

  const count = () => draggedItems().length

  return (
    <Portal>
      <Show when={dragContext.isDragging() && count() > 0}>
        <div
          class="fixed z-[10001] pointer-events-none"
          style={{
            left: ghostPosition().left,
            top: ghostPosition().top,
            transform: "translate(-50%, -50%)"
          }}
        >
          <Show when={dragContext.dragType() === "instance"}>
            <InstanceGhost
              instances={draggedItems() as ListInstance[]}
            />
          </Show>
          <Show when={dragContext.dragType() === "group"}>
            <GroupGhost
              groups={draggedItems() as { id: number; name: string }[]}
            />
          </Show>
        </div>
      </Show>
    </Portal>
  )
}

interface InstanceGhostProps {
  instances: ListInstance[]
}

const InstanceGhost = (props: InstanceGhostProps) => {
  const firstInstance = () => props.instances[0]
  const count = () => props.instances.length

  const imageUrl = createMemo(() => {
    const instance = firstInstance()
    if (!instance) return DefaultImg
    return instance.icon_revision
      ? getInstanceImageUrl(instance.id, instance.icon_revision)
      : DefaultImg
  })

  return (
    <div class="relative">
      {/* Stacked effect for multiple items */}
      <Show when={count() > 1}>
        <div
          class="absolute -right-1 -bottom-1 w-16 h-16 rounded-xl bg-darkSlate-700 opacity-60"
        />
        <div
          class="absolute -right-0.5 -bottom-0.5 w-16 h-16 rounded-xl bg-darkSlate-600 opacity-80"
        />
      </Show>

      {/* Main ghost tile */}
      <div
        class="relative w-16 h-16 rounded-xl overflow-hidden shadow-lg opacity-90 bg-darkSlate-800"
        style={{
          "background-image": `url("${imageUrl()}")`,
          "background-size": "cover",
          "background-position": "center"
        }}
      >
        {/* Overlay gradient */}
        <div class="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent" />
      </div>

      {/* Count badge */}
      <Show when={count() > 1}>
        <div class="absolute -top-2 -right-2 min-w-6 h-6 px-1.5 rounded-full bg-primary-500 text-white text-sm font-bold flex items-center justify-center shadow-md">
          {count()}
        </div>
      </Show>
    </div>
  )
}

interface GroupGhostProps {
  groups: { id: number; name: string }[]
}

const GroupGhost = (props: GroupGhostProps) => {
  const firstGroup = () => props.groups[0]

  return (
    <div class="bg-darkSlate-700 rounded-lg px-4 py-2 shadow-lg opacity-90 flex items-center gap-2">
      <div class="i-hugeicons:folder-01 h-5 w-5 text-lightSlate-400" />
      <span class="text-lightSlate-100 font-medium text-sm truncate max-w-32">
        {firstGroup()?.name}
      </span>
      <Show when={props.groups.length > 1}>
        <span class="text-lightSlate-500 text-xs">
          +{props.groups.length - 1}
        </span>
      </Show>
    </div>
  )
}

export default DragGhost
