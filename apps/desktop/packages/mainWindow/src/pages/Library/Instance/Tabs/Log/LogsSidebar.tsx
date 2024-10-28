import { GameLogEntry } from "@gd/core_module/bindings";
import { Collapsable, Spinner } from "@gd/ui";
import { createSignal, For, Match, Show, Switch } from "solid-js";
import formatDateTime from "./formatDateTime";
import { Trans } from "@gd/i18n";

type LogsByTimespan = Record<string, Array<GameLogEntry>>;

export type LogsCollapsableProps = {
  title: string;
  logGroup: Array<GameLogEntry>;
  setSelectedLog: (_: number | undefined) => void;
  selectedLog: number | undefined;
  sortDirection: "asc" | "desc";
};

const LogsCollapsable = (props: LogsCollapsableProps) => {
  const sortedLogs = () => {
    return props.logGroup
      .filter((log) => !log.active)
      .sort((a, b) => {
        if (props.sortDirection === "asc") {
          return parseInt(b.timestamp, 10) - parseInt(a.timestamp, 10);
        } else {
          return parseInt(a.timestamp, 10) - parseInt(b.timestamp, 10);
        }
      });
  };

  const groupTitle = () => {
    const logDate = new Date(props.title);

    const today = new Date();
    today.setHours(0, 0, 0, 0);

    const diffTime = Math.abs(today.getTime() - logDate.getTime());
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));

    let dateText: string;

    if (diffDays === 0) {
      dateText = "Today";
    } else if (diffDays === 1) {
      dateText = "Yesterday";
    } else if (diffDays < 7) {
      dateText = logDate.toLocaleDateString(undefined, { weekday: "long" });
    } else {
      dateText = props.title;
    }

    return dateText;
  };

  return (
    <Show when={sortedLogs().length > 0}>
      <Collapsable
        title={groupTitle()}
        noPadding
        class="bg-darkSlate-600 rounded-md px-4 py-1 mb-2"
      >
        <For each={sortedLogs()}>
          {(log) => (
            <div
              class="relative text-darkSlate-100 py-3.5 px-4 hover:bg-darkSlate-700 rounded-md w-full box-border"
              onClick={() => {
                props.setSelectedLog(log.id);
              }}
            >
              {formatDateTime(new Date(parseInt(log.timestamp, 10)))}
              <Show when={props.selectedLog === log.id}>
                <div class="absolute right-0 top-0 w-1 h-full bg-primary-400" />
              </Show>
            </div>
          )}
        </For>
      </Collapsable>
    </Show>
  );
};

export type LogsSidebarProps = {
  availableLogEntries: GameLogEntry[];
  setSelectedLog: (_: number | undefined) => void;
  selectedLog: number | undefined;
  isLoading: boolean;
};

const LogsSidebar = (props: LogsSidebarProps) => {
  const [sortDirection, setSortDirection] = createSignal<"asc" | "desc">("asc");

  const logGroups = () => {
    const logsByTimespan: LogsByTimespan = {};

    for (const log of props.availableLogEntries) {
      const logDate = new Date(parseInt(log.timestamp, 10));
      logDate.setHours(0, 0, 0, 0);

      const dateText = logDate.toDateString();

      if (!logsByTimespan[dateText]) {
        logsByTimespan[dateText] = [];
      }

      logsByTimespan[dateText].push(log);
    }

    const sortedGroups = Object.entries(logsByTimespan).sort(
      ([dateA], [dateB]) => {
        const timeA = new Date(dateA).getTime();
        const timeB = new Date(dateB).getTime();

        return sortDirection() === "asc" ? timeB - timeA : timeA - timeB;
      }
    );

    return Object.fromEntries(sortedGroups);
  };

  const activeLog = () => {
    return props.availableLogEntries.find((log) => log.active);
  };

  return (
    <div class="flex flex-col w-50 box-border pr-6 h-full">
      <div class="h-10 px-4 py-4 flex items-center justify-between">
        <div>
          <Trans key="logs.all_sessions" />
        </div>
        <div
          class="w-6 h-6 text-lightSlate-600 hover:text-lightSlate-50 duration-100 ease-in-out"
          classList={{
            "i-ri:sort-asc": sortDirection() === "asc",
            "i-ri:sort-desc": sortDirection() === "desc"
          }}
          onClick={() => {
            if (sortDirection() === "asc") {
              setSortDirection("desc");
            } else {
              setSortDirection("asc");
            }
          }}
        />
      </div>

      <Switch>
        <Match when={props.isLoading}>
          <div class="h-full w-full flex items-center justify-center">
            <Spinner />
          </div>
        </Match>
        <Match when={props.availableLogEntries.length > 0}>
          <div class="relative overflow-y-auto h-full">
            <Show when={activeLog()}>
              <div
                class="z-1 sticky top-0 bg-darkSlate-800 w-full h-10 text-lightSlate-50 rounded-b-md rounded-t-none"
                onClick={() => props.setSelectedLog(activeLog()?.id)}
              >
                <div class="relative w-full h-full flex items-center px-4 py-1 box-border bg-darkSlate-600 rounded-md">
                  <div class="bg-red-400 rounded-full text-red-400 w-4 h-4 mr-2 animate-liveCirclePulse" />
                  <div>LIVE</div>
                  <Show when={props.selectedLog === activeLog()?.id}>
                    <div class="absolute right-0 top-0 w-1 h-full bg-primary-400" />
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
          <div class="h-full flex items-center justify-center text-lightSlate-600 text-center">
            <Trans key="logs.no_log_sessions_available" />
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export default LogsSidebar;
