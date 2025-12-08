import { GameLogEntry } from "@gd/core_module/bindings"
import { Skeleton } from "@gd/ui"
import { createSignal, For, Match, Show, Switch } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import LiveSessionCard from "./components/LiveSessionCard"
import SessionEntry from "./components/SessionEntry"
import SessionGroup from "./components/SessionGroup"

type LogsByTimespan = Record<string, GameLogEntry[]>

export interface LogsSidebarProps {
  availableLogEntries: GameLogEntry[]
  setSelectedLog: (_: number | undefined) => void
  selectedLog: number | undefined
  isLoading: boolean
}

const LogsSidebar = (props: LogsSidebarProps) => {
  const [t] = useTransContext()
  const [sortDirection, setSortDirection] = createSignal<"asc" | "desc">("asc")

  const logGroups = () => {
    const logsByTimespan: LogsByTimespan = {}

    for (const log of props.availableLogEntries) {
      if (log.active) continue // Skip active logs, they're shown separately

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

  const getGroupTitle = (dateString: string) => {
    const logDate = new Date(dateString)

    const today = new Date()
    today.setHours(0, 0, 0, 0)

    const diffTime = Math.abs(today.getTime() - logDate.getTime())
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24))

    if (diffDays === 0) {
      return "Today"
    } else if (diffDays === 1) {
      return "Yesterday"
    } else if (diffDays < 7) {
      return t("tracking:_trn_x_days_ago", { count: diffDays })
    } else {
      return new Date(logDate).toLocaleDateString(undefined, {
        dateStyle: "short"
      })
    }
  }

  const getSortedLogsForGroup = (logs: GameLogEntry[]) => {
    return logs.sort((a, b) => {
      if (sortDirection() === "asc") {
        return parseInt(b.timestamp, 10) - parseInt(a.timestamp, 10)
      } else {
        return parseInt(a.timestamp, 10) - parseInt(b.timestamp, 10)
      }
    })
  }

  const totalCount = () => props.availableLogEntries.length

  return (
    <div class="w-50 box-border flex h-full flex-col pr-6">
      {/* Header */}
      <div class="box-border flex h-8 shrink-0 items-center justify-between px-4 py-6">
        <div class="flex items-center gap-1.5">
          <span class="text-lightSlate-400 text-sm font-medium">
            <Trans key="logs:_trn_all_sessions" />
          </span>
          <Show when={!props.isLoading && totalCount() > 0}>
            <span class="text-lightSlate-700 text-xs bg-darkSlate-700 px-1.5 py-0.5 rounded-full">
              {totalCount()}
            </span>
          </Show>
        </div>
        <div
          class="animate-icons-on-hover cursor-pointer"
          onClick={() => {
            if (sortDirection() === "asc") {
              setSortDirection("desc")
            } else {
              setSortDirection("asc")
            }
          }}
        >
          <div
            class={`h-5 w-5 bg-lightSlate-800 transition-colors duration-200 ease-in-out ${
              sortDirection() === "asc"
                ? "i-hugeicons:sort-by-up-01"
                : "i-hugeicons:sort-by-down-01"
            }`}
          />
        </div>
      </div>

      <Switch>
        <Match when={props.isLoading}>
          <Skeleton.logsList />
        </Match>
        <Match when={props.availableLogEntries.length > 0}>
          <div class="relative h-full overflow-y-auto">
            {/* Live session */}
            <Show when={activeLog()}>
              {(log) => (
                <LiveSessionCard
                  log={log()}
                  isSelected={props.selectedLog === log().id}
                  onClick={() => props.setSelectedLog(log().id)}
                />
              )}
            </Show>

            {/* Past sessions grouped by date */}
            <For each={Object.keys(logGroups())}>
              {(key) => {
                const logs = () => getSortedLogsForGroup(logGroups()[key])
                return (
                  <SessionGroup
                    title={getGroupTitle(key)}
                    count={logs().length}
                    defaultOpen={true}
                  >
                    <For each={logs()}>
                      {(log) => (
                        <SessionEntry
                          log={log}
                          isSelected={props.selectedLog === log.id}
                          onClick={() => props.setSelectedLog(log.id)}
                        />
                      )}
                    </For>
                  </SessionGroup>
                )
              }}
            </For>
          </div>
        </Match>
        <Match when={props.availableLogEntries.length === 0}>
          <div class="flex flex-col items-center justify-center h-full gap-2 px-4 text-center">
            <div class="i-hugeicons:file-not-found h-8 w-8 text-lightSlate-700" />
            <p class="text-lightSlate-600 text-sm">
              <Trans key="logs:_trn_no_log_sessions_available" />
            </p>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default LogsSidebar
