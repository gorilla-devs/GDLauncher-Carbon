import { GameLogEntry } from "@gd/core_module/bindings"
import { Collapsable, Skeleton } from "@gd/ui"
import { createSignal, For, Match, Show, Switch } from "solid-js"
import formatDateTime from "./formatDateTime"
import { Trans, useTransContext } from "@gd/i18n"

type LogsByTimespan = Record<string, GameLogEntry[]>

export interface LogsCollapsableProps {
  title: string
  logGroup: GameLogEntry[]
  setSelectedLog: (_: number | undefined) => void
  selectedLog: number | undefined
  sortDirection: "asc" | "desc"
}

const LogsCollapsable = (props: LogsCollapsableProps) => {
  const [t] = useTransContext()

  const sortedLogs = () => {
    return props.logGroup
      .filter((log) => !log.active)
      .sort((a, b) => {
        if (props.sortDirection === "asc") {
          return parseInt(b.timestamp, 10) - parseInt(a.timestamp, 10)
        } else {
          return parseInt(a.timestamp, 10) - parseInt(b.timestamp, 10)
        }
      })
  }

  const groupTitle = () => {
    const logDate = new Date(props.title)

    const today = new Date()
    today.setHours(0, 0, 0, 0)

    const diffTime = Math.abs(today.getTime() - logDate.getTime())
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24))

    let dateText: string

    if (diffDays === 0) {
      dateText = "Today"
    } else if (diffDays === 1) {
      dateText = "Yesterday"
    } else if (diffDays < 7) {
      dateText = t("tracking:_trn_x_days_ago", { count: diffDays })
    } else {
      dateText = new Date(logDate).toLocaleDateString(undefined, {
        dateStyle: "short"
      })
    }

    return dateText
  }

  return (
    <Show when={sortedLogs().length > 0}>
      <Collapsable
        title={groupTitle()}
        noPadding
        class="bg-darkSlate-600 mb-2 rounded-md px-4 py-1"
      >
        <For each={sortedLogs()}>
          {(log) => (
            <div
              class="text-lightSlate-700 hover:bg-darkSlate-700 relative box-border w-full rounded-md px-4 py-3.5"
              onClick={() => {
                props.setSelectedLog(log.id)
              }}
            >
              {formatDateTime(new Date(parseInt(log.timestamp, 10)))}
              <Show when={props.selectedLog === log.id}>
                <div class="bg-primary-400 absolute right-0 top-0 h-full w-1" />
              </Show>
            </div>
          )}
        </For>
      </Collapsable>
    </Show>
  )
}

export interface LogsSidebarProps {
  availableLogEntries: GameLogEntry[]
  setSelectedLog: (_: number | undefined) => void
  selectedLog: number | undefined
  isLoading: boolean
}

const LogsSidebar = (props: LogsSidebarProps) => {
  const [sortDirection, setSortDirection] = createSignal<"asc" | "desc">("asc")

  const logGroups = () => {
    const logsByTimespan: LogsByTimespan = {}

    for (const log of props.availableLogEntries) {
      const logDate = new Date(parseInt(log.timestamp, 10))
      logDate.setHours(0, 0, 0, 0)

      const dateText = logDate.toDateString()

      if (!logsByTimespan[dateText]) {
        logsByTimespan[dateText] = []
      }

      logsByTimespan[dateText].push(log)
    }

    const sortedGroups = Object.entries(logsByTimespan).sort(
      ([dateA], [dateB]) => {
        const timeA = new Date(dateA).getTime()
        const timeB = new Date(dateB).getTime()

        return sortDirection() === "asc" ? timeB - timeA : timeA - timeB
      }
    )

    return Object.fromEntries(sortedGroups)
  }

  const activeLog = () => {
    return props.availableLogEntries.find((log) => log.active)
  }

  return (
    <div class="w-50 box-border flex h-full flex-col pr-6">
      <div class="flex h-10 items-center justify-between px-4 py-4">
        <div>
          <Trans key="logs:_trn_all_sessions" />
        </div>
        <div
          class={`h-6 w-6 text-lightSlate-600 hover:text-lightSlate-50 duration-100 ease-in-out ${
            sortDirection() === "asc"
              ? "i-hugeicons:sort-by-up-01"
              : "i-hugeicons:sort-by-down-01"
          }`}
          onClick={() => {
            if (sortDirection() === "asc") {
              setSortDirection("desc")
            } else {
              setSortDirection("asc")
            }
          }}
        />
      </div>

      <Switch>
        <Match when={props.isLoading}>
          <Skeleton.logsList />
        </Match>
        <Match when={props.availableLogEntries.length > 0}>
          <div class="relative h-full overflow-y-auto">
            <Show when={activeLog()}>
              <div
                class="z-1 bg-darkSlate-800 text-lightSlate-50 sticky top-0 h-10 w-full rounded-b-md rounded-t-none"
                onClick={() => props.setSelectedLog(activeLog()?.id)}
              >
                <div class="bg-darkSlate-600 relative box-border flex h-full w-full items-center rounded-md px-4 py-1">
                  <div class="animate-liveCirclePulse mr-2 h-4 w-4 rounded-full bg-red-400 text-red-400" />
                  <div>
                    <Trans key="ui:_trn_live" />
                  </div>
                  <Show when={props.selectedLog === activeLog()?.id}>
                    <div class="bg-primary-400 absolute right-0 top-0 h-full w-1" />
                  </Show>
                </div>
              </div>
            </Show>

            <For each={Object.keys(logGroups())}>
              {(key) => (
                <LogsCollapsable
                  title={key}
                  logGroup={logGroups()[key]}
                  setSelectedLog={props.setSelectedLog}
                  selectedLog={props.selectedLog}
                  sortDirection={sortDirection()}
                />
              )}
            </For>
          </div>
        </Match>
        <Match when={props.availableLogEntries.length === 0}>
          <div class="text-lightSlate-600 flex h-full items-center justify-center text-center">
            <Trans key="logs:_trn_no_log_sessions_available" />
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default LogsSidebar
