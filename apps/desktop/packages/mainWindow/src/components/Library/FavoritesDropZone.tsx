import { createMemo, createEffect, onCleanup } from "solid-js"
import { Trans } from "@gd/i18n"
import { useDragContext } from "@/pages/Library/DragContext"
import { ListInstance } from "@gd/core_module/bindings"

interface FavoritesDropZoneProps {
  instances: ListInstance[]
  containerRef: HTMLDivElement | undefined
}

const FavoritesDropZone = (props: FavoritesDropZoneProps) => {
  const dragContext = useDragContext()

  const isOver = createMemo(() => {
    const target = dragContext.dropTarget()
    return target?.type === "favorites"
  })

  // Calculate if all dragged instances are already favorites
  const allDraggedAreFavorite = createMemo(() => {
    const draggedIds = dragContext.draggedIds()
    if (draggedIds.length === 0) return false

    const draggedInstances = props.instances.filter((i) =>
      draggedIds.includes(i.id)
    )
    return draggedInstances.every((i) => i.favorite)
  })

  // Register drop zone
  createEffect(() => {
    if (dragContext.isDragging() && dragContext.dragType() === "instance") {
      const container = props.containerRef
      if (!container) return

      const rect = container.getBoundingClientRect()
      dragContext.registerDropZone({
        id: "favorites-drop-zone",
        rect,
        element: container,
        target: { type: "favorites" }
      })
    } else {
      dragContext.unregisterDropZone("favorites-drop-zone")
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone("favorites-drop-zone")
  })

  return (
    <div
      class="w-full h-full flex items-center justify-center gap-3 rounded-full transition-all duration-200 ease-out"
      classList={{
        "bg-darkSlate-700 border-2 border-dashed border-darkSlate-500":
          !isOver(),
        "bg-primary-500/20 border-2 border-solid border-primary-500": isOver()
      }}
    >
      <div
        class="text-2xl transition-transform duration-200"
        classList={{
          "i-ri:star-fill text-yellow-500": !allDraggedAreFavorite(),
          "i-ri:star-line text-lightSlate-400": allDraggedAreFavorite(),
          "scale-125": isOver()
        }}
      />
      <span
        class="text-sm font-medium transition-colors duration-200"
        classList={{
          "text-lightSlate-300": !isOver(),
          "text-lightSlate-50": isOver()
        }}
      >
        {allDraggedAreFavorite() ? (
          <Trans key="instances:_trn_drop_to_unfavorite" />
        ) : (
          <Trans key="instances:_trn_drop_to_favorite" />
        )}
      </span>
    </div>
  )
}

export default FavoritesDropZone
