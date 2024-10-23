import { getTitleByDays } from "@/utils/helpers";
import { GameLogEntry } from "@gd/core_module/bindings";
import { Collapsable } from "@gd/ui";
import { For, Show } from "solid-js";
import formatDateTime from "./formatDateTime";

type LogsByTimespan = Record<string, Array<GameLogEntry>>;

export type LogsCollapsableProps = {
  title: string;
  logGroup: Array<GameLogEntry>;
  setSelectedLog: (_: number | undefined) => void;
  selectedLog: number | undefined;
};

const LogsCollapsable = (props: LogsCollapsableProps) => {
  const sortedLogs = () => {
    return props.logGroup
      .filter((log) => !log.active)
      .sort((a, b) => {
        return parseInt(b.timestamp, 10) - parseInt(a.timestamp, 10);
      });
  };

  return (
    <Show when={sortedLogs().length > 0}>
      <Collapsable
        title={props.title}
        noPadding
        class="bg-darkSlate-600 rounded-md px-4 py-1 mb-2"
      >
        <For each={sortedLogs()}>
          {(log) => (
            <div
              class="relative text-darkSlate-100 py-3.5 hover:bg-darkSlate-700 rounded-md w-full box-border"
              onClick={() => props.setSelectedLog(log.id)}
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
};

const LogsSidebar = (props: LogsSidebarProps) => {
  const logGroups = () => {
    const logsByTimespan: LogsByTimespan = {};

    for (const log of props.availableLogEntries) {
      const timeDiff: number = Date.now() - parseInt(log.timestamp, 10);
      const days = Math.floor(timeDiff / 1000) / 60 / 60 / 24;

      const dateText = getTitleByDays(days.toString());

      if (!logsByTimespan[dateText]) {
        logsByTimespan[dateText] = [];
      }

      logsByTimespan[dateText].push(log);
    }

    return logsByTimespan;
  };

  const activeLog = () => {
    return props.availableLogEntries.find((log) => log.active);
  };

  return (
    <div class="flex flex-col w-50 box-border pr-6 h-full">
      <div class="h-10 px-4 py-2 flex items-center">All Logs</div>

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
            />
          )}
        </For>
      </div>
    </div>
  );
};

export default LogsSidebar;
