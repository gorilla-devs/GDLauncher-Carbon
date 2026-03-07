import { Show, For, createMemo } from "solid-js"
import { Portal } from "solid-js/web"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance } from "@gd/core_module/bindings"
import DefaultImg from "/assets/images/default-instance-img.png"
import { getInstanceImageUrl } from "@/utils/instances"
import { getModloaderIcon } from "@/utils/sidebar"

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

const clamp = (v: number, min: number, max: number) =>
  Math.min(Math.max(v, min), max)

const smoothstep = (t: number) => t * t * (3 - 2 * t)

const lerp = (a: number, b: number, t: number) => a + (b - a) * t

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

  const blendFactor = createMemo(() => {
    const rect = dragContext.dropPreviewRect()
    if (!rect) return 0

    const cursor = dragContext.ghostPosition()
    const halfW = rect.width / 2
    const halfH = rect.height / 2
    if (halfW === 0 || halfH === 0) return 0

    const dx = (cursor.x - (rect.left + halfW)) / halfW
    const dy = (cursor.y - (rect.top + halfH)) / halfH
    const normalizedDist = Math.sqrt(dx * dx + dy * dy)
    const raw = clamp(1 - normalizedDist, 0, 1)

    return smoothstep(Math.pow(raw, 0.4))
  })

  const ghostPosition = createMemo(() => {
    const pos = dragContext.ghostPosition()
    const dim = getTileDimensions(props.tileSize)
    const blend = blendFactor()

    const cursorX = pos.x - dim.width * 0.15
    const cursorY = pos.y - dim.height * 0.15

    if (blend === 0) {
      return { left: `${cursorX}px`, top: `${cursorY}px` }
    }

    const rect = dragContext.dropPreviewRect()!
    const snapX = rect.left + rect.width / 2
    const snapY = rect.top + rect.height / 2

    return {
      left: `${lerp(cursorX, snapX, blend)}px`,
      top: `${lerp(cursorY, snapY, blend)}px`
    }
  })

  const isOverGroup = createMemo(() => {
    const target = dragContext.dropTarget()
    if (!target) return false
    return target.type === "dropOnFolder" || target.type === "createFolder"
  })

  const isOverFavorites = createMemo(() => {
    const target = dragContext.dropTarget()
    return target?.type === "favorites"
  })

  const dropAnim = createMemo(() => dragContext.dropAnimating())

  const count = () => draggedItems().length

  return (
    <Portal>
      <Show when={dragContext.isDragging() && count() > 0 && (dragContext.dragDetached() || dragContext.dropAnimating() !== null)}>
        <div
          class="fixed z-[10002] pointer-events-none motion-reduce:transition-none"
          style={{
            left: dropAnim()
              ? `${dropAnim()!.targetX}px`
              : ghostPosition().left,
            top: dropAnim()
              ? `${dropAnim()!.targetY}px`
              : ghostPosition().top,
            transform: dropAnim()
              ? dropAnim()!.type === "settle"
                ? "translate(-50%, -50%) scale(1)"
                : "translate(-50%, -50%) scale(0.05) scaleX(0.3)"
              : `translate(-50%, -50%) scale(${isOverFavorites() ? 0.25 : isOverGroup() ? 0.45 : 1})`,
            opacity: dropAnim()
              ? dropAnim()!.type === "settle"
                ? 1
                : 0
              : 1,
            transition: dropAnim()
              ? dropAnim()!.type === "settle"
                ? "left 200ms cubic-bezier(0.25, 1, 0.5, 1), top 200ms cubic-bezier(0.25, 1, 0.5, 1), transform 200ms cubic-bezier(0.25, 1, 0.5, 1)"
                : "left 250ms cubic-bezier(0.4, 0, 1, 0.4), top 250ms cubic-bezier(0.4, 0, 1, 0.4), transform 250ms cubic-bezier(0.4, 0, 1, 0.4), opacity 200ms ease-in 50ms"
              : "left 0s, top 0s, transform 150ms cubic-bezier(0.34, 1.56, 0.64, 1)"
          }}
        >
          <div class="relative" style={{ opacity: dropAnim() ? 1 : 1 - 0.5 * blendFactor() }}>
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
            {/* Dashed border overlay when snapping to preview position */}
            <div
              class="absolute inset-0 rounded-2xl border-2 border-dashed border-primary-400 pointer-events-none"
              style={{ opacity: dropAnim() ? 0 : blendFactor() }}
            />
            {/* Primary tint when over a group */}
            <div
              class="absolute inset-0 rounded-xl bg-primary-500 pointer-events-none transition-opacity duration-150 ease-out motion-reduce:transition-none"
              classList={{
                "opacity-30": !dropAnim() && isOverGroup(),
                "opacity-0": !!dropAnim() || !isOverGroup()
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

  const validFirstInstance = () => {
    const inst = firstInstance()
    if (!inst) return undefined
    return inst.status.status === "valid" ? inst.status.value : undefined
  }

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
        {/* Bottom gradient with name and subtitle */}
        <div class="absolute bottom-0 left-0 right-0 flex flex-col gap-1 p-3 bg-gradient-to-t from-black/80 via-black/40 to-transparent rounded-b-2xl">
          <h4 class="m-0 text-left text-sm font-semibold text-white truncate">
            {firstInstance()?.name || ""}
          </h4>
          <Show when={validFirstInstance()}>
            <div class="flex items-center gap-2 text-xs text-white/70">
              <Show when={validFirstInstance()?.modloader}>
                <img
                  class="h-3 w-3"
                  src={getModloaderIcon(validFirstInstance()!.modloader!)}
                />
              </Show>
              <span>{validFirstInstance()?.mc_version}</span>
            </div>
          </Show>
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
                <div class="rounded bg-darkSlate-800 overflow-hidden">
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
        <div class="absolute bottom-0 left-0 right-0 px-3 pt-3 pb-6 bg-gradient-to-t from-black/80 via-black/40 to-transparent">
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
