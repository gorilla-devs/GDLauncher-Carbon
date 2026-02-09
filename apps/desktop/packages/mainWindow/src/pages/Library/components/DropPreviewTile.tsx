/**
 * DropPreviewTile Component
 *
 * Renders a translucent ghost tile preview of the item being dragged.
 * Used as a drop indicator instead of a vertical line.
 * Self-registers as a drop zone to prevent oscillation when items shift.
 */

import { Accessor, createEffect, createMemo, For, onCleanup, Show } from "solid-js"
import { useDragContext, DropTarget } from "../DragContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getInstanceImageUrl } from "@/utils/instances"
import { TILE_SIZES, TileSize } from "../constants"
import DefaultImg from "/assets/images/default-instance-img.png"

interface DropPreviewTileProps {
  tileSize: Accessor<number>
  dropTarget: DropTarget
  scope?: string
}

const DropPreviewTile = (props: DropPreviewTileProps) => {
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  let previewRef: HTMLDivElement | undefined

  const size = () => (props.tileSize() as TileSize) || 2
  const sizeConfig = () => TILE_SIZES[size()]

  const draggedIds = () => dragContext.draggedIds()
  const dragType = () => dragContext.dragType()

  // Look up first dragged instance
  const firstInstance = createMemo(() => {
    if (dragType() !== "instance") return null
    const ids = draggedIds()
    if (ids.length === 0) return null
    return globalStore.instances.data?.find((i) => ids.includes(i.id)) ?? null
  })

  // Look up first dragged group
  const firstGroup = createMemo(() => {
    if (dragType() !== "group") return null
    const ids = draggedIds()
    if (ids.length === 0) return null
    return globalStore.instanceGroups.data?.find((g) => ids.includes(g.id)) ?? null
  })

  // Get instances for the group preview (2x2 grid)
  const groupPreviewInstances = createMemo(() => {
    const group = firstGroup()
    if (!group) return []
    return (globalStore.instances.data || [])
      .filter((i) => i.group_id === group.id)
      .slice(0, 4)
  })

  // Instance image URL for the first dragged instance
  const instanceImageUrl = createMemo(() => {
    const inst = firstInstance()
    if (!inst) return undefined
    return inst.icon_revision
      ? getInstanceImageUrl(inst.id, inst.icon_revision)
      : undefined
  })

  // Count badge
  const count = () => draggedIds().length

  // Self-register as drop zone to prevent oscillation
  let lastRegisteredZoneId: string | null = null

  createEffect(() => {
    if (previewRef && dragContext.isDragging()) {
      const id = `drop-preview-${props.dropTarget.type}-${JSON.stringify(props.dropTarget)}`
      lastRegisteredZoneId = id
      const rect = previewRef.getBoundingClientRect()
      dragContext.registerDropZone({
        id,
        rect,
        element: previewRef,
        target: props.dropTarget,
        scope: props.scope
      })
    }
  })

  onCleanup(() => {
    if (lastRegisteredZoneId) {
      dragContext.unregisterDropZone(lastRegisteredZoneId)
    }
  })

  return (
    <Show when={dragType() === "instance"} fallback={
      // Group/folder drag preview
      <div
        ref={previewRef}
        class={`relative rounded-lg bg-darkSlate-700 opacity-50 border-2 border-dashed border-primary-400 flex flex-col overflow-hidden ${sizeConfig().container}`}
      >
        {/* 2x2 grid of instance icons */}
        <div class="flex-1 p-2 grid grid-cols-2 grid-rows-2 gap-1">
          <For each={[0, 1, 2, 3]}>
            {(index) => {
              const instance = () => groupPreviewInstances()[index]
              return (
                <div class={`rounded bg-darkSlate-600 flex items-center justify-center overflow-hidden ${sizeConfig().icon}`}>
                  <Show when={instance()}>
                    {(inst) => (
                      <img
                        src={
                          inst().icon_revision
                            ? getInstanceImageUrl(inst().id, inst().icon_revision!)
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

        {/* Folder icon overlay bottom-right */}
        <div class="absolute bottom-6 right-1">
          <div class="i-hugeicons:folder-01 text-sm text-darkSlate-400" />
        </div>

        {/* Group name at bottom */}
        <Show when={firstGroup()}>
          {(group) => (
            <div class="px-2 pb-2 text-center">
              <span class="text-xs text-lightSlate-200 truncate block">
                {group().name}
              </span>
            </div>
          )}
        </Show>

        {/* Count badge */}
        <Show when={count() > 1}>
          <div class="absolute top-1 right-1 bg-primary-500 text-white text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center z-10">
            {count()}
          </div>
        </Show>
      </div>
    }>
      {/* Instance drag preview */}
      <div
        ref={previewRef}
        class={`relative overflow-hidden rounded-2xl opacity-50 border-2 border-dashed border-primary-400 ${sizeConfig().container}`}
      >
        {/* Background image */}
        <div
          class="absolute inset-0 bg-darkSlate-800 bg-cover bg-center"
          style={{
            "background-image": instanceImageUrl()
              ? `url("${instanceImageUrl()}")`
              : `url("${DefaultImg}")`
          }}
        />

        {/* Bottom gradient */}
        <div class="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent" />

        {/* Instance name */}
        <Show when={firstInstance()}>
          {(inst) => (
            <div class="absolute bottom-0 left-0 right-0 p-2">
              <span class="text-xs text-white truncate block">
                {inst().name}
              </span>
            </div>
          )}
        </Show>

        {/* Count badge */}
        <Show when={count() > 1}>
          <div class="absolute top-1 right-1 bg-primary-500 text-white text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center z-10">
            {count()}
          </div>
        </Show>
      </div>
    </Show>
  )
}

export default DropPreviewTile
