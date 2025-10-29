import type { Component, ComponentProps } from "solid-js"
import { splitProps, mergeProps } from "solid-js"
import { cn } from "../util"

interface AnimatedImageIconProps extends ComponentProps<"img"> {
  /**
   * Image source (URL, imported PNG/SVG, or SVG string)
   */
  src: string
  /**
   * Alt text for accessibility
   */
  alt?: string
  /**
   * Whether the icon should have interactive animations (hover/click)
   * @default true
   */
  interactive?: boolean
}

/**
 * AnimatedImageIcon - A wrapper component for custom image icons with bouncy rotation animation
 * Use this for modloader icons (Forge, Fabric, Quilt) and platform icons (CurseForge, Modrinth)
 *
 * @example
 * ```tsx
 * <AnimatedImageIcon src={forgeIcon} alt="Forge" class="h-4 w-4" />
 * <AnimatedImageIcon src={modrinthLogo} alt="Modrinth" class="h-6 w-6" />
 * <AnimatedImageIcon src={customIcon} interactive={false} /> // No animation
 * ```
 */
const AnimatedImageIcon: Component<AnimatedImageIconProps> = (props) => {
  const merged = mergeProps({ interactive: true, alt: "" }, props)
  const [local, others] = splitProps(merged, ["interactive", "class"])

  return (
    <img
      {...others}
      class={cn(
        local.interactive && "icon-interactive",
        local.class
      )}
    />
  )
}

export { AnimatedImageIcon }
export type { AnimatedImageIconProps }
