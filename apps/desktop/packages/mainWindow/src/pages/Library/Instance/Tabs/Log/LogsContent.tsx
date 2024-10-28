import { createSignal, For, Match, Show, Switch } from "solid-js";
import { isFullScreen, setIsFullScreen } from ".";
import { LogEntry, LogEntryLevel, LogEntrySourceKind } from "@/utils/logs";
import formatDateTime from "./formatDateTime";
import FullscreenToggle from "./components/FullscreenToggle";
import LogsOptions, { Columns, LogDensity } from "./components/LogsOptions";
import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";

type Props = {
  logs: LogEntry[];
  isActive: boolean;
  scrollToBottom: () => void;
  assignScrollBottomRef: (ref: HTMLDivElement) => void;
  assignLogsContentRef: (ref: HTMLDivElement) => void;
  newLogsCount: number;
  autoFollowPreference: boolean;
  setAutoFollowPreference: (_: boolean) => void;
};

const color = {
  Trace: "text-purple-500",
  Debug: "text-blue-500",
  Info: "text-green-500",
  Warn: "text-yellow-500",
  Error: "text-red-500"
};

function DateTimeFormatter(props: {
  timestamp: number;
  fontMultiplier: 0 | 1 | 2;
}) {
  return (
    <span
      class="text-lightSlate-600 font-thin mr-2"
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      {formatDateTime(new Date(props.timestamp))}
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function LoggerFormatter(props: { logger: string; fontMultiplier: 0 | 1 | 2 }) {
  return (
    <span
      class={`mr-2 font-thin`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      [{props.logger.toUpperCase()}]
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function SourceKindFormatter(props: {
  sourceKind: LogEntrySourceKind;
  fontMultiplier: 0 | 1 | 2;
}) {
  return (
    <span
      class={`mr-2 font-thin`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2,
        "text-primary-400": props.sourceKind === LogEntrySourceKind._System
      }}
    >
      [{props.sourceKind.toUpperCase()}]
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function ThreadNameFormatter(props: {
  threadName: string;
  fontMultiplier: 0 | 1 | 2;
}) {
  return (
    <span
      class={`mr-2 font-thin`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      [{props.threadName}]
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function LevelFormatter(props: {
  level: LogEntryLevel;
  fontMultiplier: 0 | 1 | 2;
}) {
  return (
    <span
      class={`mr-2 font-thin ${color[props.level]}`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      [{props.level.toUpperCase()}]
      <div class="absolute top-0 bottom-0 right-0 w-2 bg-transparent select-none" />
    </span>
  );
}

function ContentFormatter(props: {
  level: LogEntryLevel;
  sourceKind: LogEntrySourceKind;
  message: string;
  fontMultiplier: 0 | 1 | 2;
  startLogMessageOnNewLine: boolean;
}) {
  const defaultColor = () =>
    props.level === LogEntryLevel.Info ||
    props.level === LogEntryLevel.Debug ||
    props.level === LogEntryLevel.Trace;

  const isSystemLog = () => props.sourceKind === LogEntrySourceKind._System;

  return (
    <span
      classList={{
        "text-lightSlate-50": defaultColor() && !isSystemLog(),
        [color[props.level]]: !defaultColor() && !isSystemLog(),
        "text-primary-400": isSystemLog(),
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2,
        "block w-full pt-2": props.startLogMessageOnNewLine
      }}
    >
      {props.message}
    </span>
  );
}

function ScrollBottomButton(props: {
  onClick: () => void;
  newLogsCount: number;
}) {
  const [isHovered, setIsHovered] = createSignal(false);

  return (
    <Button
      size="small"
      type="secondary"
      fullWidth
      onClick={props.onClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <Switch>
        <Match when={isHovered()}>
          <Switch>
            <Match when={props.newLogsCount > 0}>
              <div class="flex items-center gap-2">
                <div class="i-ri:arrow-down-s-line" />
                <Trans
                  key="logs.new_logs"
                  options={{
                    logsCount:
                      props.newLogsCount > 999
                        ? "999+"
                        : props.newLogsCount.toString()
                  }}
                />
              </div>
            </Match>
            <Match when={props.newLogsCount === 0}>
              <div class="flex items-center gap-2">
                <div class="i-ri:arrow-down-s-line" />
                <Trans key="logs.see_new_logs" />
              </div>
            </Match>
          </Switch>
        </Match>
        <Match when={!isHovered()}>
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 i-ri:pause-fill" />
            <Trans key="logs.logs_paused_due_to_scroll" />
          </div>
        </Match>
      </Switch>
    </Button>
  );
}

const LogsContent = (props: Props) => {
  const [logsDensity, setLogsDensity] = createSignal<LogDensity>("low");
  const [columns, setColumns] = createSignal<Columns>({
    timestamp: true,
    logger: true,
    sourceKind: true,
    threadName: true,
    level: true
  });
  const [fontMultiplier, setFontMultiplier] = createSignal<0 | 1 | 2>(1);
  const [startLogMessageOnNewLine, setStartLogMessageOnNewLine] =
    createSignal(false);

  return (
    <div class="relative flex-1 min-w-0 flex flex-col border border-darkSlate-700 border-l-solid">
      <div class="flex-shrink-0 flex justify-between items-center gap-4 w-full h-10 bg-darkSlate-800 py-8 px-4 box-border">
        {/* <Input icon={<div class="i-ri:search-line" />} placeholder="Search" /> */}
        <div />
        <div class="flex items-center gap-4">
          <LogsOptions
            logsDensity={logsDensity()}
            setLogsDensity={setLogsDensity}
            columns={columns()}
            setColumns={setColumns}
            fontMultiplier={fontMultiplier()}
            setFontMultiplier={setFontMultiplier}
            autoFollowPreference={props.autoFollowPreference}
            setAutoFollowPreference={props.setAutoFollowPreference}
            startLogMessageOnNewLine={startLogMessageOnNewLine()}
            setStartLogMessageOnNewLine={setStartLogMessageOnNewLine}
          />
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
        class="justify-center hidden"
        ref={(el) => props.assignScrollBottomRef(el)}
      >
        <div class="w-60 z-1 flex justify-center fixed bottom-6">
          <ScrollBottomButton
            onClick={props.scrollToBottom}
            newLogsCount={props.newLogsCount}
          />
        </div>
      </div>
      <div
        class="relative bg-darkSlate-900 flex-1 overflow-auto px-4 py-2 w-full box-border mb-4"
        ref={props.assignLogsContentRef}
        id="instance_logs_container" // used to override user select and cursor in index.html
      >
        <Switch>
          <Match when={props.logs.length === 0}>
            <div class="flex h-full justify-center items-center text-center text-lightSlate-600 text-xl select-none">
              <Trans key="logs.no_logs" />
            </div>
          </Match>
          <Match when={props.logs.length > 0}>
            <For each={props.logs}>
              {(log) => (
                <div
                  class="w-full break-words border-b border-b-solid border-darkSlate-600 relative"
                  classList={{
                    "py-3": logsDensity() === "low",
                    "py-2": logsDensity() === "medium",
                    "py-1": logsDensity() === "high"
                  }}
                >
                  <Show when={columns().timestamp}>
                    <DateTimeFormatter
                      timestamp={log.timestamp}
                      fontMultiplier={fontMultiplier()}
                    />
                  </Show>
                  <Show when={columns().sourceKind}>
                    <SourceKindFormatter
                      sourceKind={log.sourceKind}
                      fontMultiplier={fontMultiplier()}
                    />
                  </Show>
                  <Show when={columns().level}>
                    <LevelFormatter
                      level={log.level}
                      fontMultiplier={fontMultiplier()}
                    />
                  </Show>
                  <Show when={columns().logger}>
                    <LoggerFormatter
                      logger={log.logger}
                      fontMultiplier={fontMultiplier()}
                    />
                  </Show>
                  <Show when={columns().threadName}>
                    <ThreadNameFormatter
                      threadName={log.thread}
                      fontMultiplier={fontMultiplier()}
                    />
                  </Show>
                  <ContentFormatter
                    message={log.message}
                    level={log.level}
                    sourceKind={log.sourceKind}
                    fontMultiplier={fontMultiplier()}
                    startLogMessageOnNewLine={startLogMessageOnNewLine()}
                  />
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
