import { Badge } from "@gd/ui"
import { JSX } from "solid-js"

interface Props {
  onClick: (e: MouseEvent) => void
  class?: string
  style?: JSX.CSSProperties
  children: JSX.Element
}

export function FilterBadge(props: Props) {
  return (
    <Badge
      variant="secondary"
      class={`relative group flex h-8 items-center gap-2 px-3 text-sm hover:bg-darkSlate-600 transition-colors ${props.class ?? ""}`}
      style={props.style}
      onClick={(e) => {
        e.preventDefault()
        e.stopPropagation()
        props.onClick(e)
      }}
    >
      {props.children}
      <i class="i-ri:close-fill text-lightSlate-600 group-hover:text-lightSlate-300 h-4 w-4 transition-colors" />
    </Badge>
  )
}
