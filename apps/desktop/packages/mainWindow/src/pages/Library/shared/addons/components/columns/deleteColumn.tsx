import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { Show, JSX } from "solid-js"
import { Trans } from "@gd/i18n"
import { createColumnHelper } from "@tanstack/solid-table"

interface DeleteColumnConfig {
  onDelete: (row: any) => void
  isDisabled?: () => boolean
  disabledTooltip?: JSX.Element
}

export const createDeleteColumn = (config: DeleteColumnConfig) => {
  const columnHelper = createColumnHelper<any>()

  return columnHelper.display({
    id: "delete",
    header: "",
    size: 48,
    cell: (props) => {
      const row = props.row.original
      return (
        <div
          class="flex items-center justify-center"
          onMouseDown={(e) => e.stopPropagation()}
        >
          <Show
            when={!config.isDisabled?.()}
            fallback={
              <Show
                when={config.disabledTooltip}
                fallback={
                  <div class="i-hugeicons:delete-02 text-lightSlate-700 h-5 w-5 cursor-not-allowed" />
                }
              >
                <Tooltip>
                  <TooltipTrigger>
                    <div class="i-hugeicons:delete-02 text-lightSlate-700 h-5 w-5 cursor-not-allowed" />
                  </TooltipTrigger>
                  <TooltipContent>{config.disabledTooltip}</TooltipContent>
                </Tooltip>
              </Show>
            }
          >
            <Tooltip>
              <TooltipTrigger>
                <div
                  data-testid="mod-row-delete"
                  class="i-hugeicons:delete-02 text-lightSlate-500 h-5 w-5 cursor-pointer transition-colors hover:text-red-400"
                  onClick={() => config.onDelete(row)}
                />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="content:_trn_delete_mod" />
              </TooltipContent>
            </Tooltip>
          </Show>
        </div>
      )
    },
    enableSorting: false
  })
}
