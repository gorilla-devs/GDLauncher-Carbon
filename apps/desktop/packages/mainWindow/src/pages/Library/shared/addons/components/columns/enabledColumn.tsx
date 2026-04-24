import { Switch, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { Show, JSX } from "solid-js"
import { useTransContext } from "@gd/i18n"
import { createColumnHelper } from "@tanstack/solid-table"

interface EnabledColumnConfig {
  onToggle: (row: any) => void
  isDisabled?: () => boolean
  disabledTooltip?: JSX.Element
}

export const createEnabledColumn = (config: EnabledColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<any>()

  return columnHelper.accessor("enabled", {
    header: () => (
      <span class="hidden md:inline">{t("content:_trn_table.status")}</span>
    ),
    size: 100,
    cell: (props) => {
      const row = props.row.original
      return (
        <div class="hidden md:flex">
          <Show
            when={!config.isDisabled?.()}
            fallback={
              <Show
                when={config.disabledTooltip}
                fallback={<Switch checked={props.getValue()} disabled />}
              >
                <Tooltip>
                  <TooltipTrigger>
                    <Switch checked={props.getValue()} disabled />
                  </TooltipTrigger>
                  <TooltipContent>{config.disabledTooltip}</TooltipContent>
                </Tooltip>
              </Show>
            }
          >
            <div class="group" onMouseDown={(e) => e.stopPropagation()}>
              <Switch
                checked={row.enabled}
                onChange={() => config.onToggle(row)}
              />
            </div>
          </Show>
        </div>
      )
    }
  })
}
