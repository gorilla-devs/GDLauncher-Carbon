import type { Component, ComponentProps } from "solid-js"
import { splitProps } from "solid-js"

import type { VariantProps } from "class-variance-authority"
import { cva } from "class-variance-authority"
import { cn } from "../util"

const badgeVariants = cva(
  "w=fit focus:ring-ring inline-flex items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2",
  {
    variants: {
      variant: {
        default:
          "bg-primary-500 hover:bg-primary-600 text-lightSlate-50 border-transparent",
        secondary:
          "bg-darkSlate-500 hover:bg-darkSlate-600 text-lightSlate-50 border-transparent",
        outline:
          "text-darkSlate-900 hover:bg-darkSlate-100 border-darkSlate-200",
        success:
          "text-lightSlate-50 border-green-500 bg-green-500 hover:bg-green-600",
        warning:
          "text-darkSlate-900 border-yellow-500 bg-yellow-500 hover:bg-yellow-600",
        error: "text-lightSlate-50 border-red-500 bg-red-500 hover:bg-red-600"
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
  const [local, others] = splitProps(props, ["class", "variant", "round"])
  return (
    <div
      class={cn(
        badgeVariants({ variant: local.variant }),
        local.round && "rounded-full",
        local.class
      )}
      {...others}
    />
  )
}

export type { BadgeProps }
export { Badge, badgeVariants }
