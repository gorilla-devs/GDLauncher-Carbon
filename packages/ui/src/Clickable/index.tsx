import { createMemo, mergeProps, splitProps, JSX } from "solid-js"
import { cn } from "../util"

/** Press effect classes as a string (for use with cn() or class attribute) */
export const PRESS_CLASSES =
  "transition-all duration-100 ease-spring active:scale-95"
export const PRESS_CLASSES_DISABLED = "transition-all duration-100 ease-spring"

/**
 * Returns press effect classes as an object for use with classList
 * @param disabled - Whether the element is disabled
 */
export const getPressEffectClasses = (disabled: boolean) =>
  ({
    "transition-all": true,
    "duration-100": true,
    "ease-spring": true,
    "active:scale-95": !disabled
  }) as const

export interface ClickableRenderProps {
  onClick: (e: MouseEvent) => void
  onKeyDown: (e: KeyboardEvent) => void
  class: string
  tabIndex: number
  role?: string
  "aria-disabled"?: boolean
}

export interface ClickableProps {
  /** Render function that receives clickable props to spread on the target element */
  children: (props: ClickableRenderProps) => JSX.Element

  /** Click handler */
  onClick?: (event: MouseEvent) => void

  /** Additional keyboard event handler */
  onKeyDown?: (event: KeyboardEvent) => void

  /** Whether the clickable element is disabled */
  disabled?: boolean

  /** Additional classes to merge with press effect classes */
  class?: string

  /**
   * ARIA role for accessibility. Defaults to "button" for non-button elements.
   * Set to null to skip adding role.
   */
  role?: string | null

  /**
   * Tab index. Defaults to 0 for keyboard navigation.
   * Set to -1 to remove from tab order.
   */
  tabIndex?: number
}

function Clickable(props: ClickableProps) {
  const merged = mergeProps(
    {
      tabIndex: 0,
      role: "button" as string | null
    },
    props
  )

  const [local] = splitProps(merged, [
    "children",
    "onClick",
    "onKeyDown",
    "disabled",
    "class",
    "role",
    "tabIndex"
  ])

  const handleClick = (e: MouseEvent) => {
    if (local.disabled) {
      e.preventDefault()
      return
    }
    local.onClick?.(e)
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (local.disabled) {
      e.preventDefault()
      return
    }

    // Trigger click on Enter or Space
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault()

      // Call the custom onKeyDown if provided
      local.onKeyDown?.(e)

      // Simulate click for Enter/Space
      if (local.onClick) {
        const target = e.currentTarget as HTMLElement
        const clickEvent = new MouseEvent("click", {
          bubbles: true,
          cancelable: true,
          view: window
        })
        target.dispatchEvent(clickEvent)
      }
    } else {
      local.onKeyDown?.(e)
    }
  }

  const pressClasses = createMemo(() => {
    return local.disabled ? PRESS_CLASSES_DISABLED : PRESS_CLASSES
  })

  const renderProps = createMemo((): ClickableRenderProps => {
    const baseProps: ClickableRenderProps = {
      onClick: handleClick,
      onKeyDown: handleKeyDown,
      class: cn(pressClasses(), local.class),
      tabIndex: local.disabled ? -1 : local.tabIndex
    }

    // Add role if not null
    if (local.role !== null) {
      baseProps.role = local.role
    }

    // Add aria-disabled when disabled
    if (local.disabled) {
      baseProps["aria-disabled"] = true
    }

    return baseProps
  })

  return <>{local.children(renderProps())}</>
}

export { Clickable }
