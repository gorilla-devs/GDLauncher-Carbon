import { Show, For, createMemo } from "solid-js"
import { Portal } from "solid-js/web"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance } from "@gd/core_module/bindings"
import DefaultImg from "/assets/images/default-instance-img.png"
import { getInstanceImageUrl } from "@/utils/instances"

interface DragGhostProps {
  instances: ListInstance[]
  groups: { id: number; name: string; instances: ListInstance[] }[]
  tileSize: 1 | 2 | 3 | 4 | 5
}

const getTileDimensions = (size: 1 | 2 | 3 | 4 | 5) => {
  const sizeMap = {
    1: { width: 96, height: 96 }, // h-24
    2: { width: 184, height: 184 }, // h-46
    3: { width: 240, height: 240 }, // h-60
    4: { width: 336, height: 336 }, // h-84
    5: { width: 480, height: 480 } // h-120
  }
  return sizeMap[size]
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
    const dim = getTileDimensions(props.tileSize)
    // Offset so cursor appears at top-left area of tile
    return {
      left: `${pos.x - dim.width * 0.15}px`,
      top: `${pos.y - dim.height * 0.15}px`
    }
  })

  const isOverGroup = createMemo(() => {
    const target = dragContext.dropTarget()
    if (!target) return false
    return target.type === "dropOnFolder" || target.type === "createFolder"
  })

  const count = () => draggedItems().length

  return (
    <Portal>
      <Show when={dragContext.isDragging() && count() > 0}>
        <div
          class="fixed z-[10001] pointer-events-none transition-transform duration-150 ease-[cubic-bezier(0.34,1.56,0.64,1)] motion-reduce:transition-none"
          style={{
            left: ghostPosition().left,
            top: ghostPosition().top,
            transform: `translate(-50%, -50%) scale(${isOverGroup() ? 0.8 : 1})`
          }}
        >
          <div class="relative">
            <Show when={dragContext.dragType() === "instance"}>
              <InstanceGhost
                instances={draggedItems() as ListInstance[]}
                tileSize={props.tileSize}
              />
            </Show>
            <Show when={dragContext.dragType() === "group"}>
              <GroupGhost
                groups={
                  draggedItems() as {
                    id: number
                    name: string
                    instances: ListInstance[]
                  }[]
                }
                tileSize={props.tileSize}
              />
            </Show>
            {/* Primary tint when over a group */}
            <div
              class="absolute inset-0 rounded-xl bg-primary-500 pointer-events-none transition-opacity duration-150 ease-out motion-reduce:transition-none"
              classList={{
                "opacity-30": isOverGroup(),
                "opacity-0": !isOverGroup()
              }}
            />
          </div>
        </div>
      </Show>
    </Portal>
  )
}

interface InstanceGhostProps {
  instances: ListInstance[]
  tileSize: 1 | 2 | 3 | 4 | 5
}

const InstanceGhost = (props: InstanceGhostProps) => {
  const firstInstance = () => props.instances[0]
  const count = () => props.instances.length
  const dim = () => getTileDimensions(props.tileSize)

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
          class="absolute rounded-xl bg-darkSlate-700 opacity-60"
          style={{
            width: `${dim().width}px`,
            height: `${dim().height}px`,
            right: "-4px",
            bottom: "-4px"
          }}
        />
        <div
          class="absolute rounded-xl bg-darkSlate-600 opacity-80"
          style={{
            width: `${dim().width}px`,
            height: `${dim().height}px`,
            right: "-2px",
            bottom: "-2px"
          }}
        />
      </Show>

      {/* Main ghost tile - dynamic size */}
      <div
        class="relative rounded-2xl overflow-hidden shadow-2xl bg-darkSlate-800"
        style={{
          width: `${dim().width}px`,
          height: `${dim().height}px`,
          "background-image": `url("${imageUrl()}")`,
          "background-size": "cover",
          "background-position": "center"
        }}
      >
        {/* Bottom gradient with name */}
        <div class="absolute bottom-0 left-0 right-0 px-3 pt-6 pb-3 bg-gradient-to-t from-black/80 via-black/40 to-transparent">
          <h4 class="m-0 text-sm font-semibold text-white truncate">
            {firstInstance()?.name || ""}
          </h4>
        </div>
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
  groups: { id: number; name: string; instances: ListInstance[] }[]
  tileSize: 1 | 2 | 3 | 4 | 5
}

const GroupGhost = (props: GroupGhostProps) => {
  const firstGroup = () => props.groups[0]
  const previewInstances = () => firstGroup()?.instances.slice(0, 4) || []
  const count = () => props.groups.length
  const dim = () => getTileDimensions(props.tileSize)

  return (
    <div class="relative">
      {/* Stacked effect for multiple groups */}
      <Show when={count() > 1}>
        <div
          class="absolute rounded-lg bg-darkSlate-700 opacity-60"
          style={{
            width: `${dim().width}px`,
            height: `${dim().height}px`,
            right: "-4px",
            bottom: "-4px"
          }}
        />
        <div
          class="absolute rounded-lg bg-darkSlate-600 opacity-80"
          style={{
            width: `${dim().width}px`,
            height: `${dim().height}px`,
            right: "-2px",
            bottom: "-2px"
          }}
        />
      </Show>

      {/* Main ghost - dynamic size, 2x2 preview grid */}
      <div
        class="relative rounded-2xl bg-darkSlate-700 overflow-hidden p-2"
        style={{
          width: `${dim().width}px`,
          height: `${dim().height}px`
        }}
      >
        <div class="grid grid-cols-2 grid-rows-2 gap-1 h-full">
          <For each={[0, 1, 2, 3]}>
            {(index) => {
              const instance = () => previewInstances()[index]
              return (
                <div class="rounded-sm bg-darkSlate-600 overflow-hidden">
                  <Show when={instance()}>
                    {(inst) => (
                      <img
                        src={
                          inst().icon_revision
                            ? getInstanceImageUrl(
                                inst().id,
                                inst().icon_revision!
                              )
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
        {/* Folder name overlay at bottom */}
        <div class="absolute bottom-0 left-0 right-0 px-3 pt-3 pb-2 bg-gradient-to-t from-black/80 via-black/40 to-transparent">
          <h4 class="m-0 text-sm font-semibold text-white truncate">
            {firstGroup()?.name || ""}
          </h4>
        </div>
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

export default DragGhost
