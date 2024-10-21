import { getTitleByDays } from "@/utils/helpers";
import { GameLogEntry } from "@gd/core_module/bindings";
import { Collapsable } from "@gd/ui";
import { For } from "solid-js";

type LogsByTimespan = Record<string, Array<GameLogEntry>>;

export type LogsCollapsableProps = {
  title: string;
  logs: Array<GameLogEntry>;
};

const LogsCollapsable = (props: LogsCollapsableProps) => {
  return (
    <Collapsable
      title={props.title}
      noPadding
      class="bg-darkSlate-600 rounded-md px-4 py-1"
    >
      <For each={props.logs}>
        {(log) => (
          <div class="text-darkSlate-100 py-4">
            {new Date(parseInt(log.timestamp, 10)).toLocaleString()}
          </div>
        )}
      </For>
    </Collapsable>
  );
};

export type LogsSidebarProps = {
  logs: GameLogEntry[];
};

const LogsSidebar = (props: LogsSidebarProps) => {
  const logs = () => {
    const logsByTimespan: LogsByTimespan = {};

    for (const log of props.logs) {
      const timeDiff: number = Date.now() - parseInt(log.timestamp, 10);
      const days = Math.floor(timeDiff / 1000) / 60 / 60 / 24;

      console.log(
        log.timestamp,
        timeDiff,
        days,
        getTitleByDays(days.toString())
      );

      const dateText = getTitleByDays(days.toString());

      if (!logsByTimespan[dateText]) {
        logsByTimespan[dateText] = [];
      }

      logsByTimespan[dateText].push(log);
    }

    return logsByTimespan;
  };

  return (
    <div class="w-50 box-border pr-6">
      <div class="h-10 px-4 py-2 flex items-center">All Logs</div>
      <div class="flex items-center bg-darkSlate-600 rounded-md px-4 py-1 w-full h-10 box-border text-lightSlate-50">
        <div class="bg-red-400 rounded-full text-red-400 w-4 h-4 mr-2 animate-liveCirclePulse" />
        <div>LIVE</div>
      </div>

      <For each={Object.keys(logs())}>
        {(key) => <LogsCollapsable title={key} logs={logs()[key]} />}
      </For>
    </div>
  );
};

export default LogsSidebar;
