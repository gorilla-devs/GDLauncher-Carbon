import { Input } from "@gd/ui";
import { For, Match, Switch } from "solid-js";
import { isFullScreen, setIsFullScreen } from ".";
import { LogEntry, LogEntryLevel } from "@/utils/logs";
import formatDateTime from "./formatDateTime";

type Props = {
  logs: LogEntry[];
};

function LevelFormatter(props: { level: LogEntryLevel }) {
  const color = {
    Trace: "text-purple-500",
    Debug: "text-blue-500",
    Info: "text-green-500",
    Warn: "text-yellow-500",
    Error: "text-red-500"
  };

  return <div class={color[props.level]}>[{props.level.toUpperCase()}]</div>;
}

const LogsContent = (props: Props) => {
  return (
    <div class="flex-1 flex flex-col border border-darkSlate-700 border-l-solid">
      <div class="flex justify-between items-center gap-4 w-full h-10 bg-darkSlate-800 py-8 px-4 box-border">
        <Input icon={<div class="i-ri:search-line" />} placeholder="Search" />
        <div
          class="w-6 h-6 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out"
          classList={{
            "i-ri:fullscreen-line": !isFullScreen(),
            "i-ri:fullscreen-exit-line": isFullScreen()
          }}
          onClick={() => {
            setIsFullScreen(!isFullScreen());
          }}
        />
      </div>
      <div
        class="bg-darkSlate-900 flex-1 overflow-y-scroll px-4 py-2 w-full box-border mb-4"
        id="instance_logs_container"
      >
        <Switch>
          <Match when={props.logs.length === 0}>
            <div class="flex h-full justify-center items-center text-center text-lightSlate-600 text-2xl">
              No logs available
            </div>
          </Match>
          <Match when={props.logs.length > 0}>
            <For each={props.logs}>
              {(log) => (
                <div class="flex gap-2 items-center w-full">
                  <div class="text-lightSlate-600 text-sm font-thin min-w-fit">
                    {formatDateTime(new Date(log.timestamp))}
                  </div>
                  <div class="text-lightSlate-600 text-sm font-thin min-w-fit">
                    <LevelFormatter level={log.level} />
                  </div>
                  <div class="text-lightSlate-50 text-sm border-t border-t-solid border-darkSlate-600 py-2 flex-1 flex-wrap break-words w-0">
                    {log.message}
                  </div>
                </div>
              )}
            </For>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default LogsContent;
