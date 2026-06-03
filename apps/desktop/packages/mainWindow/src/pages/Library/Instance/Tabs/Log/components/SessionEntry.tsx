import { GameLogEntry } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { PRESS_CLASSES } from "@gd/ui"
import { Show } from "solid-js"
import formatDateTime from "../formatDateTime"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { Trans } from "@gd/i18n"

export interface SessionEntryProps {
  log: GameLogEntry
  instanceId: number
  isSelected: boolean
  onClick: () => void
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const SessionEntry = (props: SessionEntryProps) => {
  const openLogInFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openLogInFolder"]
  }))

  const handleOpenInFolder = async (e: MouseEvent) => {
    e.stopPropagation()
    const path = await openLogInFolderMutation.mutateAsync({
      instance_id: props.instanceId,
      log_id: props.log.id
    })
    window.openFolder(path)
  }

  return (
    <div
      class={`group relative box-border flex items-center gap-2 w-full rounded-md px-3 py-2 cursor-pointer ${PRESS_CLASSES} ${props.isSelected ? "bg-darkSlate-600" : "hover:bg-darkSlate-700"}`}
      onClick={props.onClick}
    >
      {/* Icon */}
      <div
        class={`i-hugeicons:play-circle h-4 w-4 flex-shrink-0 transition-colors duration-150 ${props.isSelected ? "text-primary-400" : "text-lightSlate-700 group-hover:text-lightSlate-500"}`}
      />

      {/* Time and file size */}
      <div class="flex flex-col flex-1 min-w-0">
        <span
          class={`text-sm truncate transition-colors duration-150 ${props.isSelected ? "text-lightSlate-100" : "text-lightSlate-500 group-hover:text-lightSlate-300"}`}
        >
          {formatDateTime(new Date(parseInt(props.log.timestamp, 10)))}
        </span>
        <Show when={props.log.file_size !== null}>
          <span class="text-xs text-lightSlate-700">
            {formatFileSize(props.log.file_size!)}
          </span>
        </Show>
      </div>

      {/* Open in folder button - shown on hover */}
      <Tooltip>
        <TooltipTrigger>
          <div
            class="i-hugeicons:folder-02 h-4 w-4 flex-shrink-0 text-lightSlate-700 opacity-0 group-hover:opacity-100 hover:text-lightSlate-400 transition-all duration-150"
            onClick={handleOpenInFolder}
          />
        </TooltipTrigger>
        <TooltipContent>
          <Trans key="logs:_trn_open_in_folder" />
        </TooltipContent>
      </Tooltip>

      {/* Selection indicator */}
      <Show when={props.isSelected}>
        <div class="absolute right-0 top-1/2 -translate-y-1/2 w-1 h-5 bg-primary-400 rounded-l-full" />
      </Show>
    </div>
  )
}

export default SessionEntry
