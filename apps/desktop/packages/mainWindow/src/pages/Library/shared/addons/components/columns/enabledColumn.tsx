import { Switch, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { Show, JSX } from "solid-js"
import { useTransContext } from "@gd/i18n"
import { createColumnHelper } from "@tanstack/solid-table"

interface EnabledColumnConfig {
  onToggle: (row: any) => void
  isDisabled?: () => boolean
  /** Per-row: render no control at all for this row (as opposed to
   *  `isDisabled`, which renders a greyed-out Switch with a tooltip). */
  isHidden?: (row: any) => boolean
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
      if (config.isHidden?.(row)) return null
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
            <div
              class="group"
              data-testid="mod-row-toggle"
              onMouseDown={(e) => e.stopPropagation()}
            >
              {/* The anchor lives on this wrapping div rather than as a prop
                  on Switch: Switch spreads all its props onto the native
                  `<input type="checkbox">` it renders, but that input is
                  zero-size (`w-0 h-0`) — the visible, clickable surface is
                  the enclosing `<label>`. This div hugs that label's size
                  (it's an unstretched flex item), so it's clickable. */}
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
