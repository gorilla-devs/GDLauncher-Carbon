import { GameLogEntry } from "@gd/core_module/bindings"
import { PRESS_CLASSES } from "@gd/ui"
import { Show } from "solid-js"
import formatDateTime from "../formatDateTime"

export interface SessionEntryProps {
  log: GameLogEntry
  isSelected: boolean
  onClick: () => void
}

const SessionEntry = (props: SessionEntryProps) => {
  return (
    <div
      class={`group relative box-border flex items-center gap-2 w-full rounded-md px-3 py-2 cursor-pointer ${PRESS_CLASSES} ${props.isSelected ? "bg-darkSlate-600" : "hover:bg-darkSlate-700"}`}
      onClick={props.onClick}
    >
      {/* Icon */}
      <div
        class={`i-hugeicons:play-circle h-4 w-4 flex-shrink-0 transition-colors duration-150 ${props.isSelected ? "text-primary-400" : "text-lightSlate-700 group-hover:text-lightSlate-500"}`}
      />

      {/* Time */}
      <span
        class={`text-sm truncate transition-colors duration-150 ${props.isSelected ? "text-lightSlate-100" : "text-lightSlate-500 group-hover:text-lightSlate-300"}`}
      >
        {formatDateTime(new Date(parseInt(props.log.timestamp, 10)))}
      </span>

      {/* Selection indicator */}
      <Show when={props.isSelected}>
        <div class="absolute right-0 top-1/2 -translate-y-1/2 w-1 h-5 bg-primary-400 rounded-l-full" />
      </Show>
    </div>
  )
}

export default SessionEntry
