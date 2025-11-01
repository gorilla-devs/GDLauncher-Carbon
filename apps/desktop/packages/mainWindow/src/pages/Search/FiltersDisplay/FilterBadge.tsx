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
      class={`group relative flex h-8 cursor-pointer items-center gap-2 px-3 text-sm ${props.class ?? ""}`}
      style={props.style}
      onClick={(e) => {
        e.preventDefault()
        e.stopPropagation()
        props.onClick(e)
      }}
    >
      {props.children}
      <div class="i-hugeicons:cancel-01 text-lightSlate-600 group-hover:text-lightSlate-300 h-4 w-4 transition-colors" />
    </Badge>
  )
}
