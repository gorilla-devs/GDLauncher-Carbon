import {
  Show,
  createMemo,
  createSignal,
  createEffect,
  onCleanup
} from "solid-js"

interface SelectionBorderProps {
  isSelected: boolean
  size: 1 | 2 | 3 | 4 | 5
}

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

  // Tile dimensions based on size (from Tile.tsx)
  // UnoCSS uses 0.25rem per unit, so h-24 = 24 * 0.25rem = 6rem = 96px
  const tileDimension = createMemo(() => {
    const sizes = {
      1: 24 * 4, // h-24 = 6rem = 96px
      2: 46 * 4, // h-46 = 11.5rem = 184px
      3: 60 * 4, // h-60 = 15rem = 240px
      4: 84 * 4, // h-84 = 21rem = 336px
      5: 120 * 4 // h-120 = 30rem = 480px
    }
    return sizes[props.size] || sizes[2]
  })

  const borderRadius = 16 // rounded-2xl = 1rem = 16px
  const strokeWidth = 2

  // SVG is 4px larger than tile to accommodate stroke
  const svgSize = createMemo(() => tileDimension() + 4)

  // Rect is inset by 1px on each side (for stroke centering)
  const rectSize = createMemo(() => svgSize() - 2)

  // Calculate perimeter for rounded rectangle
  // P = 2(w - 2r) + 2(h - 2r) + 2πr = 2w + 2h - 8r + 2πr
  const perimeter = createMemo(() => {
    const d = rectSize()
    return 2 * d + 2 * d - 8 * borderRadius + 2 * Math.PI * borderRadius
  })

  return (
    <Show when={isVisible()}>
      <svg
        class="absolute pointer-events-none z-10"
        style={{
          top: "-2px",
          left: "-2px",
          width: `${svgSize()}px`,
          height: `${svgSize()}px`
        }}
      >
        <rect
          x="1"
          y="1"
          width={rectSize()}
          height={rectSize()}
          rx={borderRadius}
          ry={borderRadius}
          fill="none"
          stroke="rgb(var(--primary-500))"
          stroke-width={strokeWidth}
          class={
            isExiting() ? "selection-border-undraw" : "selection-border-draw"
          }
          style={{ "--border-perimeter": `${perimeter()}px` }}
        />
      </svg>
    </Show>
  )
}

export default SelectionBorder
