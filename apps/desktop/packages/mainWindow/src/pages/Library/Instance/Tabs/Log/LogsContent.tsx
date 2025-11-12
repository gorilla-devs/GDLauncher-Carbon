import { createSignal, For, Match, Show, Switch } from "solid-js"

import { isFullScreen, LogQuery, setIsFullScreen } from "."
import { LogEntry, LogEntryLevel, LogEntrySourceKind } from "@/utils/logs"
import formatDateTime from "./formatDateTime"
import FullscreenToggle from "./components/FullscreenToggle"
import LogsOptions, { Columns, LogDensity } from "./components/LogsOptions"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { VList } from "@/components/VirtuaWrapper"
import { VirtualizerHandle } from "virtua/lib/solid"
import { SetStoreFunction } from "solid-js/store"
import LogsFinders from "./LogsFinders"
import { FESearchResult } from "@gd/core_module/bindings"
import useKeyboardShortcut from "@/hooks/useKeyboardShortcut"
import { ScrollToIndexOpts } from "virtua/lib/core"

interface Props {
  logs: LogEntry[]
  query: LogQuery
  setQuery: SetStoreFunction<LogQuery>
  logSearchResults: FESearchResult[] | undefined
  isActive: boolean
  isLoading: boolean
  scrollToBottom: () => void
  onScroll: () => void
  assignScrollBottomRef: (ref: HTMLDivElement) => void
  assignLogsContentRef: (ref: HTMLDivElement) => void
  assignVirtualizerRef: (ref: VirtualizerHandle) => void
  newLogsCount: number
  autoFollowPreference: boolean
  setAutoFollowPreference: (_: boolean) => void
  scrollToIndex: (index: number, opts: ScrollToIndexOpts) => void
  isIndexLoaded: (index: number) => boolean
}

const color = {
  Trace: "text-purple-500",
  Debug: "text-blue-500",
  Info: "text-green-500",
  Warn: "text-yellow-500",
  Error: "text-red-500",
  Fatal: "text-red-500",
  Unknown: "text-red-500"
}

function DateTimeFormatter(props: {
  timestamp: number
  fontMultiplier: 0 | 1 | 2
}) {
  return (
    <span
      class="text-lightSlate-600 bg-darkSlate-900 z-10 rounded-md py-2 pr-2 font-thin italic"
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      {formatDateTime(new Date(props.timestamp))}
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
    </span>
  )
}

function LoggerFormatter(props: { logger: string; fontMultiplier: 0 | 1 | 2 }) {
  return (
    <span
      class={`bg-darkSlate-900 z-10 rounded-md py-2 pr-2 font-thin italic`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      [{props.logger.toUpperCase()}]
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
    </span>
  )
}

function SourceKindFormatter(props: {
  sourceKind: LogEntrySourceKind
  fontMultiplier: 0 | 1 | 2
}) {
  return (
    <span
      class={`bg-darkSlate-900 z-10 rounded-md py-2 pr-2 font-thin italic`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2,
        "text-primary-400": props.sourceKind === LogEntrySourceKind._System
      }}
    >
      [{props.sourceKind.toUpperCase()}]
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
    </span>
  )
}

function ThreadNameFormatter(props: {
  threadName: string
  fontMultiplier: 0 | 1 | 2
}) {
  return (
    <span
      class={`bg-darkSlate-900 z-10 rounded-md py-2 pr-2 font-thin italic`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      [{props.threadName}]
      {/* These absolute dividers are used to interrupt text selection to this column, as it selects the largest continuous block of text it can find */}
      <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
    </span>
  )
}

function LevelFormatter(props: {
  level: LogEntryLevel
  fontMultiplier: 0 | 1 | 2
}) {
  return (
    <span
      class={`py-2 pr-2 font-bold ${color[props.level]} bg-darkSlate-900 z-10 rounded-md italic`}
      classList={{
        "text-xs": props.fontMultiplier === 0,
        "text-sm": props.fontMultiplier === 1,
        "text-base": props.fontMultiplier === 2
      }}
    >
      [{props.level.toUpperCase()}]
      <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
    </span>
  )
}

function ContentFormatter(props: {
  relativeCurrentResultIndex: number | null
  baseIndex: number | null
  searchResults: FESearchResult[] | undefined
  level: LogEntryLevel
  sourceKind: LogEntrySourceKind
  message: string
  fontMultiplier: 0 | 1 | 2
  startLogMessageOnNewLine: boolean
}) {
  const defaultColor = () =>
    props.level === LogEntryLevel.Info ||
    props.level === LogEntryLevel.Debug ||
    props.level === LogEntryLevel.Trace

  const isSystemLog = () => props.sourceKind === LogEntrySourceKind._System

  return (
    <span
      class="whitespace-pre-wrap"
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
      {props.searchResults?.length ? (
        <>
          <For each={props.searchResults}>
            {(result, i) => (
              <>
                {i() === 0 &&
                  result.pos > 0 &&
                  props.message.slice(0, result.pos)}
                {i() > 0 &&
                  props.message.slice(
                    props.searchResults![i() - 1].pos +
                      props.searchResults![i() - 1].len,
                    result.pos
                  )}
                <span
                  id={`finder-result-${(props.baseIndex ?? 0) + i()}`}
                  class="bg-yellow-500/50"
                  classList={{
                    "outline outline-2 outline-yellow-500":
                      props.relativeCurrentResultIndex === i()
                  }}
                >
                  {props.message.slice(result.pos, result.pos + result.len)}
                </span>
                {i() === (props.searchResults?.length ?? 0) - 1 &&
                  props.message.slice(result.pos + result.len)}
              </>
            )}
          </For>
        </>
      ) : (
        props.message
      )}
    </span>
  )
}

function ScrollBottomButton(props: {
  onClick: () => void
  newLogsCount: number
}) {
  const [isHovered, setIsHovered] = createSignal(false)

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
                <div class="i-hugeicons:arrow-down-01" />
                <Trans
                  key="logs:_trn_new_logs"
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
                <div class="i-hugeicons:arrow-down-01" />
                <Trans key="logs:_trn_see_new_logs" />
              </div>
            </Match>
          </Switch>
        </Match>
        <Match when={!isHovered()}>
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:pause h-4 w-4" />
            <Trans key="logs:_trn_logs_paused_due_to_scroll" />
          </div>
        </Match>
      </Switch>
    </Button>
  )
}

const LogsContent = (props: Props) => {
  const [open, setOpen] = createSignal(false)
  const [logsDensity, setLogsDensity] = createSignal<LogDensity>("low")
  const [columns, setColumns] = createSignal<Columns>({
    timestamp: true,
    logger: true,
    sourceKind: false,
    threadName: true,
    level: true
  })
  const [fontMultiplier, setFontMultiplier] = createSignal<0 | 1 | 2>(1)
  const [startLogMessageOnNewLine, setStartLogMessageOnNewLine] =
    createSignal(false)
  const [currentResultIndex, setCurrentResultIndex] = createSignal<
    number | null
  >(null)

  let searchInputRef: HTMLInputElement | undefined

  const setSearchInputRef = (ref: HTMLInputElement) => {
    searchInputRef = ref
  }

  useKeyboardShortcut(
    [navigator.platform.includes("Mac") ? "Meta" : "Control", "F"],
    () => {
      if (open() && searchInputRef) {
        searchInputRef.focus()
      } else {
        setOpen(true)
      }
    }
  )

  useKeyboardShortcut(["Escape"], () => {
    setOpen(false)
  })

  return (
    <div class="border-darkSlate-700 border-l-solid relative flex min-w-0 flex-1 flex-col border">
      <div class="bg-darkSlate-800 box-border flex h-10 w-full shrink-0 items-center justify-between gap-4 px-4 py-8">
        <div />
        <div class="flex items-center gap-4">
          <LogsFinders
            setSearchInputRef={setSearchInputRef}
            open={open()}
            setOpen={setOpen}
            query={props.query}
            setQuery={props.setQuery}
            logSearchResults={props.logSearchResults}
            currentResultIndex={currentResultIndex()}
            setCurrentResultIndex={setCurrentResultIndex}
            scrollToIndex={props.scrollToIndex}
            isIndexLoaded={props.isIndexLoaded}
          />
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
        <div class="z-1 bg-darkSlate-700 text-lightSlate-700 absolute right-6 top-20 flex h-10 w-fit items-center rounded-3xl px-4">
          <div class="animate-liveCirclePulse mr-2 h-3 w-3 rounded-full bg-red-400 text-red-400" />
          <div>
            <Trans key="ui:_trn_live" />
          </div>
        </div>
      </Show>
      <div
        class="hidden justify-center"
        ref={(el) => props.assignScrollBottomRef(el)}
      >
        <div class="fixed bottom-6 z-20 flex w-60 justify-center">
          <ScrollBottomButton
            onClick={props.scrollToBottom}
            newLogsCount={props.newLogsCount}
          />
        </div>
      </div>
      <div
        class="bg-darkSlate-900 relative mb-4 box-border w-full flex-1 overflow-auto py-2 pl-4 pr-2"
        ref={props.assignLogsContentRef}
        id="instance_logs_container" // used to override user select and cursor in index.html
      >
        <Switch>
          <Match when={props.isLoading}>
            <div />
          </Match>
          <Match when={props.logs.length === 0}>
            <div class="text-lightSlate-600 flex h-full select-none items-center justify-center text-center text-xl">
              <Trans key="logs:_trn_no_logs" />
            </div>
          </Match>
          <Match when={props.logs.length > 0}>
            <VList
              data={props.logs}
              ref={(virtuaHandle) => {
                if (virtuaHandle) {
                  props.assignVirtualizerRef(virtuaHandle)
                }
              }}
              onWheel={props.onScroll}
              overscan={10}
            >
              {(log, index) => {
                const rowSearchResult = open()
                  ? props.logSearchResults?.filter(
                      (result) => result.entry_index === index()
                    )
                  : undefined

                const baseIndex = props.logSearchResults?.findIndex(
                  (result) => result.entry_index === index()
                )

                let relativeCurrentResultIndex = -1

                const currResultIndex = currentResultIndex()

                if (
                  currResultIndex !== null &&
                  props.logSearchResults?.[currResultIndex]?.entry_index ===
                    index()
                ) {
                  // Find which result within this row matches the current global index
                  const rowResultIndex = rowSearchResult?.findIndex(
                    (result) =>
                      result === props.logSearchResults?.[currResultIndex]
                  )
                  if (rowResultIndex !== undefined && rowResultIndex !== -1) {
                    relativeCurrentResultIndex = rowResultIndex
                  }
                }

                return (
                  <div
                    class="border-b-solid border-darkSlate-600 relative w-full break-words border-b"
                    classList={{
                      "py-3": logsDensity() === "low",
                      "py-2": logsDensity() === "medium",
                      "py-1": logsDensity() === "high"
                    }}
                  >
                    <span>{index()}</span>
                    <Show when={columns().timestamp}>
                      <DateTimeFormatter
                        timestamp={log.timestamp}
                        fontMultiplier={fontMultiplier()}
                      />
                    </Show>
                    <Show
                      when={
                        columns().sourceKind &&
                        log.sourceKind !== LogEntrySourceKind._System
                      }
                    >
                      <SourceKindFormatter
                        sourceKind={log.sourceKind}
                        fontMultiplier={fontMultiplier()}
                      />
                    </Show>
                    <Show
                      when={
                        columns().level && log.level !== LogEntryLevel.Unknown
                      }
                    >
                      <LevelFormatter
                        level={log.level}
                        fontMultiplier={fontMultiplier()}
                      />
                    </Show>
                    <Show when={columns().logger && log.logger !== "N/A"}>
                      <LoggerFormatter
                        logger={log.logger}
                        fontMultiplier={fontMultiplier()}
                      />
                    </Show>
                    <Show when={columns().threadName && log.thread !== "N/A"}>
                      <ThreadNameFormatter
                        threadName={log.thread}
                        fontMultiplier={fontMultiplier()}
                      />
                    </Show>
                    <ContentFormatter
                      relativeCurrentResultIndex={relativeCurrentResultIndex}
                      baseIndex={baseIndex ?? null}
                      searchResults={rowSearchResult}
                      message={log.message}
                      level={log.level}
                      sourceKind={log.sourceKind}
                      fontMultiplier={fontMultiplier()}
                      startLogMessageOnNewLine={startLogMessageOnNewLine()}
                    />
                  </div>
                )
              }}
            </VList>
          </Match>
        </Switch>
      </div>
    </div>
  )
}

export default LogsContent
