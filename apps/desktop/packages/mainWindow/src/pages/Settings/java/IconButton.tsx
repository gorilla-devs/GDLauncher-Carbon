import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import type { JSX } from "solid-js"

export const ICON_BUTTON_CLASS =
  "flex h-6 w-6 shrink-0 items-center justify-center rounded-md outline-none transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"

/** A tooltip that shows a path: narrow enough to stay clear of the ads. */
export const PATH_TOOLTIP_CLASS = "max-w-[32rem]"

/** The path itself, which has no spaces to wrap at, so it breaks anywhere.
 *  Only on the path: a sentence beside it must not break mid-word. */
export const PATH_TEXT_CLASS = "break-all"

/** An icon-only button with a tooltip that also opens on keyboard focus.
 *  `disabled` keeps it focusable, so the tooltip can say why. */
const IconButton = (props: {
  label: string
  tooltip: string
  onClick?: () => void
  disabled?: boolean
  busy?: boolean
  class?: string
  ref?: (el: HTMLButtonElement) => void
  children: JSX.Element
}) => (
  <Tooltip>
    <TooltipTrigger
      ref={props.ref}
      type="button"
      aria-label={props.label}
      aria-disabled={props.disabled || undefined}
      aria-busy={props.busy || undefined}
      class={`${ICON_BUTTON_CLASS} ${props.class ?? ""}`}
      onClick={() => {
        if (!props.disabled) props.onClick?.()
      }}
    >
      {props.children}
    </TooltipTrigger>
    <TooltipContent>{props.tooltip}</TooltipContent>
  </Tooltip>
)

export default IconButton
