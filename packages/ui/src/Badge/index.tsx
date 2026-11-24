import type { Component, ComponentProps } from "solid-js"
import { splitProps } from "solid-js"

import type { VariantProps } from "class-variance-authority"
import { cva } from "class-variance-authority"
import { cn } from "../util"
import { gdlHooks } from "../hooks"

const badgeVariants = cva(
  "w=fit focus:ring-ring inline-flex items-center rounded-md px-2.5 py-0.5 text-xs font-semibold transition-colors duration-100 ease-spring focus:outline-none focus:ring-2 focus:ring-offset-2",
  {
    variants: {
      variant: {
        default:
          "bg-primary-500 hover:bg-primary-600 text-lightSlate-50 border-0 border-transparent",
        secondary:
          "bg-darkSlate-800 hover:bg-darkSlate-700 hover:outline-darkSlate-600 text-lightSlate-50 border-0 border-transparent outline-none",
        outline:
          "text-lightSlate-50 bg-darkSlate-800 hover:bg-darkSlate-700 hover:outline-darkSlate-600 border-0 border-transparent outline-none",
        success:
          "text-lightSlate-50 border-0 border-green-500 bg-green-500 hover:bg-green-600",
        warning:
          "text-darkSlate-900 border-0 border-yellow-500 bg-yellow-500 hover:bg-yellow-600",
        error:
          "text-lightSlate-50 border-0 border-red-500 bg-red-500 hover:bg-red-600"
      }
    },
    defaultVariants: {
      variant: "default"
    }
  }
)

type BadgeProps = ComponentProps<"div"> &
  VariantProps<typeof badgeVariants> & {
    round?: boolean
  }

const Badge: Component<BadgeProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "variant",
    "round",
    "data-gdl" as keyof BadgeProps
  ])
  const merged = () =>
    cn(
      badgeVariants({ variant: local.variant }),
      local.round && "rounded-full",
      local.class
    )
  // `badge.default` marks a badge that really is gold: the default variant
  // whose merged classes still have `bg-primary-500` (a caller's own
  // background class replaces it).
  const isGold = () => merged().split(/\s+/).includes("bg-primary-500")
  // On the gold fill the words read `--gdl-on-primary` (style.css), unless a
  // caller's own text colour replaced the default one.
  const classes = () =>
    isGold()
      ? merged().replace(/(^|\s)text-lightSlate-50(?=\s|$)/, "$1gdl-on-primary")
      : merged()
  return (
    <div
      data-gdl={gdlHooks(
        isGold() ? "badge badge.default" : "badge",
        (local as Record<string, unknown>)["data-gdl"]
      )}
      class={classes()}
      {...others}
    />
  )
}

export type { BadgeProps }
export { Badge, badgeVariants }
