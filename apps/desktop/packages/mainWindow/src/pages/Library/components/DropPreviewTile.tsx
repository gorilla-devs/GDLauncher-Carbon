/**
 * DropPreviewTile Component
 *
 * Invisible layout spacer that reserves grid space for the dragged item.
 * Reports its bounding rect to DragContext so the DragGhost can snap to it.
 * Self-registers as a drop zone to prevent oscillation when items shift.
 */

import { Accessor, createEffect, onCleanup } from "solid-js"
import { useDragContext, DropTarget } from "../DragContext"
import { TILE_SIZES, TileSize } from "../constants"

interface DropPreviewTileProps {
  tileSize: Accessor<number>
  dropTarget: DropTarget
  scope?: string
}

const DropPreviewTile = (props: DropPreviewTileProps) => {
  const dragContext = useDragContext()
  let previewRef: HTMLDivElement | undefined

  const size = () => (props.tileSize() as TileSize) || 2
  const sizeConfig = () => TILE_SIZES[size()]

  // Self-register as drop zone to prevent oscillation.
  // Extend horizontally by the column-gap (16px) on each side so there are
  // no dead zones between the preview and adjacent tile zones.  The trigger
  // zones (e.g. beforeInstance) extend 8px past tile edges via rectTransform,
  // but the preview div doesn't — without padding the cursor can land in a
  // gap between the preview zone and adjacent tile zones, causing oscillation.
  const ZONE_PADDING = 16
  let lastRegisteredZoneId: string | null = null

  createEffect(() => {
    if (previewRef && dragContext.isDragging()) {
      const id = `drop-preview-${props.dropTarget.type}-${JSON.stringify(props.dropTarget)}`
      lastRegisteredZoneId = id
      const rawRect = previewRef.getBoundingClientRect()
      const rect = new DOMRect(
        rawRect.left - ZONE_PADDING,
        rawRect.top,
        rawRect.width + ZONE_PADDING * 2,
        rawRect.height
      )
      dragContext.registerDropZone({
        id,
        rect,
        element: previewRef,
        rectTransform: (r) =>
          new DOMRect(r.left - ZONE_PADDING, r.top, r.width + ZONE_PADDING * 2, r.height),
        target: props.dropTarget,
        scope: props.scope
      })

      // Report position so the DragGhost can snap here
      dragContext.setDropPreviewRect(previewRef.getBoundingClientRect())
    }
  })

  onCleanup(() => {
    if (lastRegisteredZoneId) {
      dragContext.unregisterDropZone(lastRegisteredZoneId)
    }
    dragContext.setDropPreviewRect(null)
  })

  return (
    <div
      ref={previewRef}
      data-drop-preview
      class={`${sizeConfig().container} pointer-events-none`}
    />
  )
}

export default DropPreviewTile
