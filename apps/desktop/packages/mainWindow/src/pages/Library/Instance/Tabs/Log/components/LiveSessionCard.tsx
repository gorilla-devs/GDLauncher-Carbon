import { GameLogEntry } from "@gd/core_module/bindings"
import { Show } from "solid-js"
import { Trans } from "@gd/i18n"
import formatDateTime from "../formatDateTime"

export interface LiveSessionCardProps {
  log: GameLogEntry
  isSelected: boolean
  onClick: () => void
}

const LiveSessionCard = (props: LiveSessionCardProps) => {
  return (
    <div class="z-1 bg-darkSlate-800 sticky top-0 w-full pb-2">
      <div
        class={`relative box-border flex items-center gap-2.5 bg-darkSlate-600 rounded-md px-3 py-2.5 cursor-pointer transition-all duration-150 ease-spring hover:bg-darkSlate-500 ${props.isSelected ? "bg-darkSlate-500" : ""}`}
        onClick={props.onClick}
      >
        <div class="animate-liveCirclePulse h-3 w-3 rounded-full bg-red-400 flex-shrink-0" />
        <div class="flex flex-col min-w-0 flex-1">
          <span class="text-lightSlate-50 text-sm">
            <Trans key="ui:_trn_live" />
          </span>
          <span class="text-lightSlate-600 text-xs truncate">
            {formatDateTime(new Date(parseInt(props.log.timestamp, 10)))}
          </span>
        </div>
        <Show when={props.isSelected}>
          <div class="absolute right-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-primary-400 rounded-l-full" />
        </Show>
      </div>
    </div>
  )
}

export default LiveSessionCard
