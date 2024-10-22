import { Input } from "@gd/ui";
import { For } from "solid-js";
import { isFullScreen, setIsFullScreen } from ".";
import { LogEntry, LogEntryLevel } from "@/utils/logs";

type Props = {
  logs: LogEntry[];
};

function formatDateTime(
  date: Date,
  locale: string = navigator.language
): string {
  const day: string = date.getDate().toString().padStart(2, "0");
  const month: string = (date.getMonth() + 1).toString().padStart(2, "0");
  const hours: string = date.getHours().toString().padStart(2, "0");
  const minutes: string = date.getMinutes().toString().padStart(2, "0");
  const seconds: string = date.getSeconds().toString().padStart(2, "0");
  const time: string = `${hours}:${minutes}:${seconds}`;

  const isUSFormat: boolean = /^en-US/i.test(locale);
  const dateStr: string = isUSFormat ? `${month}.${day}` : `${day}.${month}`;

  return `${dateStr} ${time}`;
}

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
        class="bg-darkSlate-900 flex-1 overflow-y-scroll px-4 py-2 w-full box-border"
        id="instance_logs_container"
      >
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
      </div>
    </div>
  );
};

export default LogsContent;
