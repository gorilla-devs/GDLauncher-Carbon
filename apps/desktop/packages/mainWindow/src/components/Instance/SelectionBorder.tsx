import { Show, createSignal, createEffect, onCleanup } from "solid-js"
import { TILE_SIZES } from "@/pages/Library/constants"

interface SelectionBorderProps {
  isSelected: boolean
  /** Draw at this tile size's fixed box (the picture plus 2px round it)
   *  instead of filling the parent: for a folder, whose frame is as wide as
   *  its grid cell rather than its picture. */
  size?: 1 | 2 | 3 | 4 | 5
}

/** The dash pattern's length: the rect's `pathLength`, so drawing it round
 *  works for any size and corner radius. */
const PATH_LENGTH = 1000

/**
 * The outline drawn round a selected tile (`library.tile.outline`). On an
 * instance or server tile it fills the tile's frame (`library.tile.frame`),
 * whatever size a theme gives it. On a folder it sits outside the frame, in
 * the root, at the picture's fixed size plus 2px round it (`size`). Its
 * corners follow `--gdl-tile-radius`, like the frame and the picture. The stroke is set on
 * the SVG and inherited, so a theme's `stroke` or `stroke-width` on the hook
 * reaches it. With reduced motion it is drawn whole at once (index.html).
 */
const SelectionBorder = (props: SelectionBorderProps) => {
  // Track visibility and animation state
  const [isVisible, setIsVisible] = createSignal(props.isSelected)
  const [isExiting, setIsExiting] = createSignal(false)

  // Handle selection changes with exit animation
  createEffect(() => {
    const selected = props.isSelected
    if (selected) {
      setIsExiting(false)
      setIsVisible(true)
    } else if (isVisible()) {
      // Start exit animation
      setIsExiting(true)
      const timer = setTimeout(() => {
        setIsVisible(false)
        setIsExiting(false)
      }, 100) // Match undraw animation duration
      onCleanup(() => clearTimeout(timer))
    }
  })

  // The box: the parent's, or a tile size's fixed one.
  const outer = () =>
    props.size ? `${TILE_SIZES[props.size].widthPx + 4}px` : "100%"
  const inner = () =>
    props.size ? `${TILE_SIZES[props.size].widthPx}px` : "calc(100% - 4px)"

  return (
    <Show when={isVisible()}>
      {/* The rect sits 2px inside the frame, on the picture's edge, so the
          2px stroke lies half over the picture and half over the frame. */}
      <svg
        data-gdl="library.tile.outline"
        class="absolute pointer-events-none z-10"
        fill="none"
        stroke="rgb(var(--primary-500))"
        stroke-width="2"
        style={{
          top: "0px",
          left: "0px",
          width: outer(),
          height: outer()
        }}
      >
        <rect
          pathLength={PATH_LENGTH}
          class={
            isExiting() ? "selection-border-undraw" : "selection-border-draw"
          }
          // SVG geometry as CSS (Solid's style typings don't list it), so
          // the rect follows the frame's size and the tile's radius.
          style={[
            "x: 2px",
            "y: 2px",
            `width: ${inner()}`,
            `height: ${inner()}`,
            "rx: var(--gdl-tile-radius, calc(1rem * var(--radius-scale, 1)))",
            `--border-perimeter: ${PATH_LENGTH}`
          ].join("; ")}
        />
      </svg>
    </Show>
  )
}

export default SelectionBorder
