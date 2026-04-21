/**
 * EndOfGroupDropZone Component
 *
 * Drop zone at the end of a group for placing instances.
 */

import { Accessor, createEffect, onCleanup, Show } from "solid-js"
import { useDragContext } from "../DragContext"
import DropPreviewTile from "./DropPreviewTile"
import { TILE_SIZES, TileSize } from "../constants"

interface EndOfGroupDropZoneProps {
  groupId: number
  instanceCount?: number
  zoneIdPrefix?: string
  tileSize: Accessor<number>
  scope?: string // Optional scope for filtering (e.g., "folder-123")
}

export function EndOfGroupDropZone(props: EndOfGroupDropZoneProps) {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  const zoneIdPrefix = () => props.zoneIdPrefix ?? "end-of-group"
  const sizeConfig = () => TILE_SIZES[(props.tileSize() as TileSize) || 2]

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "endOfGroup" && target.groupId === props.groupId
  }

  // Register drop zone
  createEffect(() => {
    const zoneId = `${zoneIdPrefix()}-${props.groupId}`
    if (
      dragContext.isDragging() &&
      (dragContext.dragType() === "instance" ||
        dragContext.dragType() === "server") &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: zoneId,
        rect,
        element: ref,
        target: { type: "endOfGroup", groupId: props.groupId },
        scope: props.scope
      })
    } else {
      dragContext.unregisterDropZone(zoneId)
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone(`${zoneIdPrefix()}-${props.groupId}`)
  })

  return (
    <>
      <Show when={isOver()}>
        <DropPreviewTile
          tileSize={props.tileSize}
          dropTarget={{ type: "endOfGroup", groupId: props.groupId }}
          scope={props.scope}
        />
      </Show>
      <Show when={!isOver()}>
        <div
          ref={ref}
          class={`relative flex items-center justify-center rounded-lg transition-all duration-200 border-2 border-dashed border-darkSlate-500 ${sizeConfig().container}`}
        >
          <div class="i-hugeicons:plus text-lg text-darkSlate-500" />
        </div>
      </Show>
    </>
  )
}

/**
 * EndOfGroupsDropZone Component
 *
 * Drop zone at the end of all groups for folder reordering.
 */

interface EndOfGroupsDropZoneProps {
  tileSize: Accessor<number>
}

export function EndOfGroupsDropZone(props: EndOfGroupsDropZoneProps) {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined
  const sizeConfig = () => TILE_SIZES[(props.tileSize() as TileSize) || 2]

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "endOfGroups"
  }

  // Register drop zone
  createEffect(() => {
    if (dragContext.isDragging() && dragContext.dragType() === "group" && ref) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: "end-of-groups",
        rect,
        element: ref,
        target: { type: "endOfGroups" }
      })
    } else {
      dragContext.unregisterDropZone("end-of-groups")
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone("end-of-groups")
  })

  return (
    <>
      <Show when={isOver()}>
        <DropPreviewTile
          tileSize={props.tileSize}
          dropTarget={{ type: "endOfGroups" }}
        />
      </Show>
      <Show when={!isOver()}>
        <div
          ref={ref}
          class={`relative flex items-center justify-center rounded-lg transition-all duration-200 border-2 border-dashed border-darkSlate-500 ${sizeConfig().container}`}
        >
          <div class="i-hugeicons:plus text-lg text-darkSlate-500" />
        </div>
      </Show>
    </>
  )
}
