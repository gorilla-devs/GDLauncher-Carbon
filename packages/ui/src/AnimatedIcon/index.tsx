import type { Component, ComponentProps } from "solid-js"
import { splitProps, mergeProps } from "solid-js"
import { cn } from "../util"

interface AnimatedIconProps extends ComponentProps<"div"> {
  /**
   * The Iconify icon class (e.g., "i-hugeicons:search-01")
   */
  icon: string
  /**
   * Whether the icon should have interactive animations (hover/click)
   * @default true
   */
  interactive?: boolean
  /**
   * Size classes (e.g., "h-5 w-5", "text-lg")
   */
  size?: string
}

/**
 * AnimatedIcon - A wrapper component for Iconify icons with bouncy rotation animation
 *
 * @example
 * ```tsx
 * <AnimatedIcon icon="i-hugeicons:search-01" class="text-primary-500" />
 * <AnimatedIcon icon="i-hugeicons:settings-01" size="h-6 w-6" />
 * <AnimatedIcon icon="i-hugeicons:loading-03" interactive={false} /> // No animation
 * ```
 */
const AnimatedIcon: Component<AnimatedIconProps> = (props) => {
  const merged = mergeProps({ interactive: true }, props)
  const [local, others] = splitProps(merged, ["icon", "interactive", "size", "class"])

  return (
    <div
      {...others}
      class={cn(
        local.icon,
        local.size,
        local.interactive && "icon-interactive",
        local.class
      )}
    />
  )
}

export { AnimatedIcon }
export type { AnimatedIconProps }
