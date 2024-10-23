import { Button, Input, ButtonGroup } from "@gd/ui";
import { createSignal, For, Match, Show, Switch } from "solid-js";
import { isFullScreen, setIsFullScreen } from ".";
import { LogEntry, LogEntryLevel } from "@/utils/logs";
import formatDateTime from "./formatDateTime";
import FullscreenToggle from "./components/FullscreenToggle";
import LogsOptions from "./components/LogsOptions";

type Props = {
  logs: LogEntry[];
  isActive: boolean;
};

const color = {
  Trace: "text-purple-500",
  Debug: "text-blue-500",
  Info: "text-green-500",
  Warn: "text-yellow-500",
  Error: "text-red-500"
};

function DateTimeFormatter(props: { timestamp: number }) {
  return (
    <span class="text-lightSlate-600 text-sm font-thin mr-2">
      {formatDateTime(new Date(props.timestamp))}
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function LevelFormatter(props: { level: LogEntryLevel }) {
  return (
    <span class={`mr-2 text-sm font-thin ${color[props.level]}`}>
      [{props.level.toUpperCase()}]
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function ContentFormatter(props: { level: LogEntryLevel; message: string }) {
  const defaultColor = () =>
    props.level === LogEntryLevel.Info ||
    props.level === LogEntryLevel.Debug ||
    props.level === LogEntryLevel.Trace;

  return (
    <span
      class="text-sm"
      classList={{
        "text-lightSlate-50": defaultColor(),
        [color[props.level]]: !defaultColor()
      }}
    >
      {props.message}
    </span>
  );
}

const LogsContent = (props: Props) => {
  return (
    <div class="relative flex-1 min-w-0 flex flex-col border border-darkSlate-700 border-l-solid">
      <div class="flex-shrink-0 flex justify-between items-center gap-4 w-full h-10 bg-darkSlate-800 py-8 px-4 box-border">
        <Input icon={<div class="i-ri:search-line" />} placeholder="Search" />
        <div class="flex items-center gap-4">
          <LogsOptions />
          <FullscreenToggle
            isFullScreen={isFullScreen}
            setIsFullScreen={setIsFullScreen}
          />
        </div>
      </div>
      <Show when={props.isActive}>
        <div class="z-1 absolute top-20 right-6 w-fit h-10 bg-darkSlate-700 text-lightSlate-800 flex items-center px-4 rounded-3xl">
          <div class="bg-red-400 rounded-full text-red-400 w-3 h-3 mr-2 animate-liveCirclePulse" />
          <div>LIVE</div>
        </div>
      </Show>
      <div
        class="relative bg-darkSlate-900 flex-1 overflow-auto px-4 py-2 w-full box-border mb-4"
        id="instance_logs_container"
      >
        <Switch>
          <Match when={props.logs.length === 0}>
            <div class="flex h-full justify-center items-center text-center text-lightSlate-600 text-2xl select-none">
              No logs available
            </div>
          </Match>
          <Match when={props.logs.length > 0}>
            <For each={props.logs}>
              {(log) => (
                <div class="w-full py-3 break-words border-b border-b-solid border-darkSlate-600 relative">
                  <DateTimeFormatter timestamp={log.timestamp} />
                  <LevelFormatter level={log.level} />
                  <ContentFormatter message={log.message} level={log.level} />
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
