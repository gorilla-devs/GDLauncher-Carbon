import { Trans } from "@gd/i18n"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger
} from "@gd/ui"

export type LogDensity = "low" | "medium" | "high"

export interface Columns {
  timestamp: boolean
  logger: boolean
  sourceKind: boolean
  threadName: boolean
  level: boolean
}

interface Props {
  logsDensity: LogDensity
  setLogsDensity: (_: LogDensity) => void
  columns: Columns
  setColumns: (_: Columns) => void
  fontMultiplier: 0 | 1 | 2
  setFontMultiplier: (_: 0 | 1 | 2) => void
  autoFollowPreference: boolean
  setAutoFollowPreference: (_: boolean) => void
  startLogMessageOnNewLine: boolean
  setStartLogMessageOnNewLine: (_: boolean) => void
}

export default function LogsOptions(props: Props) {
  return (
    <DropdownMenu placement="left">
      <DropdownMenuTrigger class="b-0 bg-transparent p-0">
        <div class="i-hugeicons:settings-01 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out h-6 w-6" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Trans key="logs:_trn_logs_density" />
          </DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownMenuRadioGroup
                value={props.logsDensity}
                onChange={(value) => props.setLogsDensity(value as LogDensity)}
              >
                <DropdownMenuRadioItem value="low">
                  <Trans key="logs:_trn_logs_density.low" />
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="medium">
                  <Trans key="logs:_trn_logs_density.comfortable" />
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="high">
                  <Trans key="logs:_trn_logs_density.compact" />
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Trans key="logs:_trn_font_size" />
          </DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownMenuRadioGroup
                value={props.fontMultiplier.toString()}
                onChange={(value) =>
                  props.setFontMultiplier(parseInt(value) as 0 | 1 | 2)
                }
              >
                <DropdownMenuRadioItem class="text-xs" value="0">
                  {"aAaAaA"}
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem class="text-sm" value="1">
                  {"aAaAaA"}
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem class="text-base" value="2">
                  {"aAaAaA"}
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Trans key="logs:_trn_columns" />
          </DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownMenuCheckboxItem
                checked={props.columns.timestamp}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    timestamp: !props.columns.timestamp
                  })
                }
              >
                <Trans key="logs:_trn_columns.timestamp" />
              </DropdownMenuCheckboxItem>
              <DropdownMenuCheckboxItem
                checked={props.columns.logger}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    logger: !props.columns.logger
                  })
                }
              >
                <Trans key="logs:_trn_columns.logger" />
              </DropdownMenuCheckboxItem>
              <DropdownMenuCheckboxItem
                checked={props.columns.sourceKind}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    sourceKind: !props.columns.sourceKind
                  })
                }
              >
                <Trans key="logs:_trn_columns.source_kind" />
              </DropdownMenuCheckboxItem>
              <DropdownMenuCheckboxItem
                checked={props.columns.threadName}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    threadName: !props.columns.threadName
                  })
                }
              >
                <Trans key="logs:_trn_columns.thread_name" />
              </DropdownMenuCheckboxItem>
              <DropdownMenuCheckboxItem
                checked={props.columns.level}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    level: !props.columns.level
                  })
                }
              >
                <Trans key="logs:_trn_columns.level" />
              </DropdownMenuCheckboxItem>
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuSeparator />
        <DropdownMenuCheckboxItem
          checked={props.autoFollowPreference}
          onChange={() =>
            props.setAutoFollowPreference(!props.autoFollowPreference)
          }
        >
          <Trans key="logs:_trn_autofollow" />
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={props.startLogMessageOnNewLine}
          onChange={() =>
            props.setStartLogMessageOnNewLine(!props.startLogMessageOnNewLine)
          }
        >
          <Trans key="logs:_trn_start_log_message_on_new_line" />
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
