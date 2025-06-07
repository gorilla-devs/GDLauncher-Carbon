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
      class={`group flex h-10 items-center gap-4 px-4 text-sm ${props.class ?? ""}`}
      style={props.style}
      onClick={(e) => {
        e.preventDefault()
        e.stopPropagation()
        props.onClick(e)
      }}
    >
      {props.children}
      <i class="i-ri:close-fill text-lightSlate-700 group-hover:text-lightSlate-50 h-5 w-5 transition-colors" />
    </Badge>
  )
}
