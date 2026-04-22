/**
 * useDragLayoutAnimation Hook
 *
 * Manual FLIP (First-Last-Invert-Play) animation for grid tiles during drag-and-drop.
 * Provides smooth iOS-like tile rearrangement when the drop target changes.
 *
 * Flow:
 * 1. DragContext calls capturePositions (via layoutCaptureCallback) BEFORE setDropTarget
 * 2. capturePositions reads visual positions (including in-progress animation transforms)
 * 3. setDropTarget updates → SolidJS updates DOM synchronously
 * 4. Effect fires → reads new layout positions → computes deltas → animates
 *
 * For rapid target changes, in-progress animations are canceled and the visual position
 * from the interrupted animation becomes the start of the new animation (no snapping).
 */

import { createEffect, on, onCleanup, Accessor } from "solid-js"
import { useDragContext } from "../DragContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { ANIMATION } from "../constants"

export function useDragLayoutAnimation(
  gridRef: Accessor<HTMLDivElement | undefined>
) {
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  let savedPositions = new Map<Element, DOMRect>()
  const activeAnimations = new Map<Element, Animation>()

  const capturePositions = () => {
    const grid = gridRef()
    if (!grid) return

    // 1. Read VISUAL positions (includes in-progress animation transforms)
    //    This is where tiles appear to the user right now
    const visualPositions = new Map<Element, DOMRect>()
    for (const child of grid.children) {
      if (child instanceof HTMLElement) {
        visualPositions.set(child, child.getBoundingClientRect())
      }
    }

    // 2. Cancel running animations (elements snap to layout positions)
    for (const anim of activeAnimations.values()) anim.cancel()
    activeAnimations.clear()

    // 3. Store visual positions (these represent where tiles LOOK right now)
    savedPositions = visualPositions
  }

  // Register the capture callback
  dragContext.addLayoutCaptureCallback(capturePositions)

  // Animate after DOM update
  createEffect(
    on(
      () => dragContext.dropTarget(),
      () => {
        const reducedMotion = globalStore.settings.data?.reducedMotion ?? false
        if (
          reducedMotion ||
          !dragContext.isDragging() ||
          savedPositions.size === 0
        ) {
          return
        }

        const grid = gridRef()
        if (!grid) return

        // Phase 1: batch READS. Reading getBoundingClientRect between writes
        // thrashes layout (O(n) reflows); reading all first keeps it to one.
        const pending: { el: HTMLElement; dx: number; dy: number }[] = []
        for (const child of grid.children) {
          if (!(child instanceof HTMLElement)) continue
          if (child.dataset.dropPreview !== undefined) continue
          if (child.classList.contains("hidden")) continue

          const oldRect = savedPositions.get(child)
          if (!oldRect) continue

          const newRect = child.getBoundingClientRect()
          const dx = oldRect.left - newRect.left
          const dy = oldRect.top - newRect.top
          if (Math.abs(dx) < 2 && Math.abs(dy) < 2) continue
          pending.push({ el: child, dx, dy })
        }

        // Phase 2: batch WRITES.
        for (const { el, dx, dy } of pending) {
          const anim = el.animate(
            [
              { transform: `translate(${dx}px, ${dy}px)` },
              { transform: "translate(0, 0)" }
            ],
            {
              duration: ANIMATION.DRAG_LAYOUT_DURATION,
              easing: ANIMATION.DRAG_LAYOUT_EASING
            }
          )
          activeAnimations.set(el, anim)
          anim.onfinish = () => activeAnimations.delete(el)
          anim.oncancel = () => activeAnimations.delete(el)
        }

        savedPositions = new Map()
      },
      { defer: false }
    )
  )

  // Clean up animations when drag ends
  createEffect(
    on(
      () => dragContext.isDragging(),
      (dragging) => {
        if (!dragging) {
          for (const anim of activeAnimations.values()) anim.cancel()
          activeAnimations.clear()
          savedPositions.clear()
        }
      }
    )
  )

  onCleanup(() => {
    for (const anim of activeAnimations.values()) anim.cancel()
    activeAnimations.clear()
    dragContext.removeLayoutCaptureCallback(capturePositions)
  })
}
