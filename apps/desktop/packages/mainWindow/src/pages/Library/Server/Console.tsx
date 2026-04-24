import { port, rspc } from "@/utils/rspcClient"
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  on,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import { createStore, produce } from "solid-js/store"
import { Trans, useTransContext } from "@gd/i18n"
import { VList } from "@/components/VirtuaWrapper"
import type { VirtualizerHandle } from "virtua/lib/solid"
import {
  Button,
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
  Input,
  Popover,
  PopoverContent,
  PopoverTrigger
} from "@gd/ui"
import useKeyboardShortcut from "@/hooks/useKeyboardShortcut"
import { isConsoleFullScreen, setIsConsoleFullScreen } from "./Tabs/ConsoleTab"

interface ConsoleProps {
  serverId: number
  isRunning: boolean
}

interface ParsedLogLine {
  raw: string
  timestamp?: string
  thread?: string
  level?: "INFO" | "WARN" | "ERROR" | "DEBUG" | "FATAL"
  message: string
}

interface SearchResult {
  lineIndex: number
  pos: number
  len: number
}

type LogDensity = "low" | "medium" | "high"

interface Columns {
  timestamp: boolean
  level: boolean
  thread: boolean
}

const LEVEL_COLORS: Record<string, string> = {
  INFO: "text-lightSlate-300",
  WARN: "text-yellow-400",
  ERROR: "text-red-400",
  DEBUG: "text-lightSlate-600",
  FATAL: "text-red-500 font-bold"
}

const LEVEL_BG: Record<string, string> = {
  WARN: "bg-yellow-500/5",
  ERROR: "bg-red-500/8",
  FATAL: "bg-red-500/12"
}

/**
 * Parse a Minecraft server log line.
 * Format: [HH:MM:SS] [Thread/LEVEL]: message
 * Or: [HH:MM:SS INFO]: message (some servers)
 */
function parseLogLine(raw: string): ParsedLogLine {
  const match1 =
    /^\[(\d{2}:\d{2}:\d{2})\]\s*\[([^/\]]+)\/(\w+)\]:\s*(.*)$/.exec(raw)
  if (match1) {
    return {
      raw,
      timestamp: match1[1],
      thread: match1[2],
      level: match1[3] as ParsedLogLine["level"],
      message: match1[4]
    }
  }

  const match2 = /^\[(\d{2}:\d{2}:\d{2})\s+(\w+)\]:\s*(.*)$/.exec(raw)
  if (match2) {
    return {
      raw,
      timestamp: match2[1],
      level: match2[2] as ParsedLogLine["level"],
      message: match2[3]
    }
  }

  return { raw, message: raw }
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

// --- Sub-components ---

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

function ConsoleSearch(props: {
  open: boolean
  setOpen: (_: boolean) => void
  searchQuery: string
  setSearchQuery: (_: string) => void
  matchCase: boolean
  setMatchCase: (_: boolean) => void
  matchWholeWord: boolean
  setMatchWholeWord: (_: boolean) => void
  useRegex: boolean
  setUseRegex: (_: boolean) => void
  searchResults: SearchResult[]
  currentResultIndex: number | null
  onNavigate: (_: "up" | "down") => void
  setSearchInputRef: (_: HTMLInputElement) => void
}) {
  const [t] = useTransContext()

  return (
    <Popover open={props.open} gutter={4} placement="bottom">
      <PopoverTrigger
        as="div"
        onClick={() => props.setOpen(!props.open)}
        class="animate-icons-on-hover cursor-pointer"
        classList={{ "bg-darkSlate-700": props.open }}
      >
        <div
          class={`i-hugeicons:search-01 h-5 w-5 transition-all duration-200 ease-spring ${
            props.open
              ? "rotate-12 scale-110 bg-lightSlate-50"
              : "bg-lightSlate-800"
          }`}
        />
      </PopoverTrigger>
      <PopoverContent
        hideCloseButton
        class="border-darkSlate-700 mr-40 w-fit border border-solid bg-darkSlate-900 p-2"
      >
        <div class="flex items-center justify-between gap-4">
          <Input
            ref={(ref) => props.setSearchInputRef(ref)}
            icon={
              <div class="-mr-4 flex h-full items-center gap-0.5">
                <div
                  class="group flex h-full items-center rounded-md px-2 transition-colors duration-200 ease-spring"
                  classList={{
                    "bg-darkSlate-500": props.matchCase,
                    "hover:bg-darkSlate-800": !props.matchCase
                  }}
                  onClick={() => props.setMatchCase(!props.matchCase)}
                >
                  <div
                    class={`i-codicon:case-sensitive transition-colors duration-200 ease-spring group-hover:bg-lightSlate-50 ${
                      props.matchCase ? "bg-lightSlate-50" : "bg-lightSlate-800"
                    }`}
                  />
                </div>
                <div
                  class="group flex h-full items-center rounded-md px-2 transition-colors duration-200 ease-spring"
                  classList={{
                    "bg-darkSlate-500": props.matchWholeWord,
                    "hover:bg-darkSlate-800": !props.matchWholeWord
                  }}
                  onClick={() => props.setMatchWholeWord(!props.matchWholeWord)}
                >
                  <div
                    class={`i-codicon:whole-word transition-colors duration-200 ease-spring group-hover:bg-lightSlate-50 ${
                      props.matchWholeWord
                        ? "bg-lightSlate-50"
                        : "bg-lightSlate-800"
                    }`}
                  />
                </div>
                <div
                  class="group flex h-full items-center rounded-md px-2 transition-colors duration-200 ease-spring"
                  classList={{
                    "bg-darkSlate-500": props.useRegex,
                    "hover:bg-darkSlate-800": !props.useRegex
                  }}
                  onClick={() => props.setUseRegex(!props.useRegex)}
                >
                  <div
                    class={`i-codicon:regex transition-colors duration-200 ease-spring group-hover:bg-lightSlate-50 ${
                      props.useRegex ? "bg-lightSlate-50" : "bg-lightSlate-800"
                    }`}
                  />
                </div>
              </div>
            }
            class="h-6"
            placeholder={t("placeholders:_trn_find_logs")}
            value={props.searchQuery}
            onInput={(e) => props.setSearchQuery(e.target.value)}
          />
          <div class="text-sm text-lightSlate-800">
            <Switch>
              <Match when={!props.searchResults.length}>
                <div class="w-24">
                  <Trans key="ui:_trn_no_results" />
                </div>
              </Match>
              <Match when={props.searchResults.length > 0}>
                <div class="w-24">
                  {props.currentResultIndex !== null
                    ? `${props.currentResultIndex + 1} of ${props.searchResults.length}`
                    : `1 of ${props.searchResults.length}`}
                </div>
              </Match>
            </Switch>
          </div>
          <div
            class="i-hugeicons:arrow-up-01 h-4 w-4 cursor-pointer bg-lightSlate-800 transition-colors duration-200 ease-spring hover:bg-lightSlate-50"
            onClick={() => props.onNavigate("up")}
          />
          <div
            class="i-hugeicons:arrow-down-01 h-4 w-4 cursor-pointer bg-lightSlate-800 transition-colors duration-200 ease-spring hover:bg-lightSlate-50"
            onClick={() => props.onNavigate("down")}
          />
          <div
            class="i-hugeicons:cancel-01 h-4 w-4 cursor-pointer bg-lightSlate-800 transition-colors duration-200 ease-spring hover:bg-lightSlate-50"
            onClick={() => props.setOpen(false)}
          />
        </div>
      </PopoverContent>
    </Popover>
  )
}

function ConsoleOptions(props: {
  logsDensity: LogDensity
  setLogsDensity: (_: LogDensity) => void
  columns: Columns
  setColumns: (_: Columns) => void
  fontMultiplier: 0 | 1 | 2
  setFontMultiplier: (_: 0 | 1 | 2) => void
  autoFollowPreference: boolean
  setAutoFollowPreference: (_: boolean) => void
}) {
  return (
    <DropdownMenu placement="left">
      <DropdownMenuTrigger class="animate-icons-on-hover b-0 bg-transparent p-0">
        <div class="i-hugeicons:settings-01 h-5 w-5 bg-lightSlate-800 transition-colors duration-200 ease-spring" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Trans key="logs:_trn_logs_density" />
          </DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownMenuRadioGroup
                value={props.logsDensity}
                onChange={(value) => props.setLogsDensity(value as LogDensity)}
              >
                <DropdownMenuRadioItem value="low">
                  <Trans key="logs:_trn_logs_density.low" />
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="medium">
                  <Trans key="logs:_trn_logs_density.comfortable" />
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="high">
                  <Trans key="logs:_trn_logs_density.compact" />
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Trans key="logs:_trn_font_size" />
          </DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownMenuRadioGroup
                value={props.fontMultiplier.toString()}
                onChange={(value) =>
                  props.setFontMultiplier(parseInt(value) as 0 | 1 | 2)
                }
              >
                <DropdownMenuRadioItem class="text-xs" value="0">
                  {"aAaAaA"}
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem class="text-sm" value="1">
                  {"aAaAaA"}
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem class="text-base" value="2">
                  {"aAaAaA"}
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Trans key="logs:_trn_columns" />
          </DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownMenuCheckboxItem
                checked={props.columns.timestamp}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    timestamp: !props.columns.timestamp
                  })
                }
              >
                <Trans key="logs:_trn_columns.timestamp" />
              </DropdownMenuCheckboxItem>
              <DropdownMenuCheckboxItem
                checked={props.columns.level}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    level: !props.columns.level
                  })
                }
              >
                <Trans key="logs:_trn_columns.level" />
              </DropdownMenuCheckboxItem>
              <DropdownMenuCheckboxItem
                checked={props.columns.thread}
                onChange={() =>
                  props.setColumns({
                    ...props.columns,
                    thread: !props.columns.thread
                  })
                }
              >
                <Trans key="logs:_trn_columns.thread_name" />
              </DropdownMenuCheckboxItem>
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuSeparator />
        <DropdownMenuCheckboxItem
          checked={props.autoFollowPreference}
          onChange={() =>
            props.setAutoFollowPreference(!props.autoFollowPreference)
          }
        >
          <Trans key="logs:_trn_autofollow" />
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

// --- Main Component ---

const Console = (props: ConsoleProps) => {
  const [t] = useTransContext()
  const [logs, setLogs] = createStore<ParsedLogLine[]>([])
  const [command, setCommand] = createSignal("")
  const [commandHistory, setCommandHistory] = createSignal<string[]>([])
  const [historyIndex, setHistoryIndex] = createSignal(-1)

  // Search state
  const [searchOpen, setSearchOpen] = createSignal(false)
  const [searchQuery, setSearchQuery] = createSignal("")
  const [matchCase, setMatchCase] = createSignal(false)
  const [matchWholeWord, setMatchWholeWord] = createSignal(false)
  const [useRegex, setUseRegex] = createSignal(false)
  const [currentResultIndex, setCurrentResultIndex] = createSignal<
    number | null
  >(null)

  // Display state
  const [logsDensity, setLogsDensity] = createSignal<LogDensity>("low")
  const [fontMultiplier, setFontMultiplier] = createSignal<0 | 1 | 2>(1)
  const [columns, setColumns] = createSignal<Columns>({
    timestamp: true,
    level: true,
    thread: true
  })
  const [autoFollowPreference, setAutoFollowPreference] = createSignal(true)
  const [autoFollow, setAutoFollow] = createSignal(true)
  const [lastSeenLogCount, setLastSeenLogCount] = createSignal(0)

  let virtualizerRef: VirtualizerHandle | undefined
  let inputRef: HTMLInputElement | undefined
  let searchInputRef: HTMLInputElement | undefined

  const newLogsCount = createMemo(() =>
    Math.max(0, logs.length - lastSeenLogCount())
  )

  // Client-side search
  const searchResults = createMemo(() => {
    const q = searchQuery()
    if (!q) return []

    const results: SearchResult[] = []
    let regex: RegExp

    try {
      const flags = matchCase() ? "g" : "gi"
      if (useRegex()) {
        regex = new RegExp(q, flags)
      } else if (matchWholeWord()) {
        regex = new RegExp(`\\b${escapeRegex(q)}\\b`, flags)
      } else {
        regex = new RegExp(escapeRegex(q), flags)
      }
    } catch {
      return []
    }

    for (let i = 0; i < logs.length; i++) {
      const text = logs[i].message
      regex.lastIndex = 0
      let match: RegExpExecArray | null
      while ((match = regex.exec(text)) !== null) {
        results.push({ lineIndex: i, pos: match.index, len: match[0].length })
        if (!match[0].length) regex.lastIndex++
      }
    }

    return results
  })

  // Reset result navigation when search params change
  createEffect(
    on(
      [searchQuery, matchCase, matchWholeWord, useRegex],
      () => {
        setCurrentResultIndex(null)
      },
      { defer: true }
    )
  )

  const sendCommandMutation = rspc.createMutation(() => ({
    mutationKey: ["server.sendConsoleCommand"]
  }))

  // WebSocket connection
  createEffect(() => {
    if (!props.isRunning) {
      setLogs([])
      setLastSeenLogCount(0)
      return
    }

    const wsConnection = new WebSocket(
      `ws://127.0.0.1:${port}/server/log?id=${props.serverId}`
    )

    wsConnection.onmessage = (event) => {
      const data = JSON.parse(event.data)

      if (data.error) {
        setLogs(
          produce((prev) => {
            prev.push(parseLogLine(`[Error] ${data.error}`))
          })
        )
        return
      }

      if (Array.isArray(data)) {
        setLogs(data.map(parseLogLine))
      } else if (typeof data === "string") {
        setLogs(
          produce((prev) => {
            prev.push(parseLogLine(data))
          })
        )
      }
    }

    wsConnection.onerror = () => {
      setLogs(
        produce((prev) => {
          prev.push(parseLogLine("[Connection error]"))
        })
      )
    }

    onCleanup(() => {
      if (wsConnection && wsConnection.readyState === wsConnection.OPEN) {
        wsConnection.close()
      }
    })
  })

  // Auto-follow on new logs
  createEffect(() => {
    const count = logs.length
    if (count === 0) return

    if (autoFollowPreference() && autoFollow()) {
      requestAnimationFrame(() => {
        virtualizerRef?.scrollToIndex(count - 1)
      })
      setLastSeenLogCount(count)
    }
  })

  const handleWheel = () => {
    requestAnimationFrame(() => {
      if (!virtualizerRef) return
      const isAtBottom =
        virtualizerRef.scrollOffset + virtualizerRef.viewportSize >=
        virtualizerRef.scrollSize - 30
      setAutoFollow(isAtBottom)
      if (isAtBottom) {
        setLastSeenLogCount(logs.length)
      }
    })
  }

  const scrollToBottom = () => {
    if (virtualizerRef && logs.length > 0) {
      virtualizerRef.scrollToIndex(logs.length - 1)
      setAutoFollow(true)
      setLastSeenLogCount(logs.length)
    }
  }

  const handleSearchNavigate = (direction: "up" | "down") => {
    const results = searchResults()
    if (!results.length) return

    let newIndex: number
    const current = currentResultIndex()
    if (current === null) {
      newIndex = 0
    } else {
      const delta = direction === "down" ? 1 : -1
      newIndex = (current + delta + results.length) % results.length
    }

    const result = results[newIndex]
    if (!result || !virtualizerRef) return

    virtualizerRef.scrollToIndex(result.lineIndex, {})
    setCurrentResultIndex(newIndex)

    requestAnimationFrame(() => {
      const el = document.getElementById(`finder-result-${newIndex}`)
      if (el && virtualizerRef) {
        virtualizerRef.scrollToIndex(result.lineIndex, {
          offset: el.offsetTop - 100,
          smooth: true
        })
      }
    })
  }

  const handleSendCommand = () => {
    const cmd = command().trim()
    if (!cmd) return

    sendCommandMutation.mutate({
      id: props.serverId,
      command: cmd
    })

    setCommandHistory((prev) => [
      cmd,
      ...prev.filter((c) => c !== cmd).slice(0, 49)
    ])
    setHistoryIndex(-1)
    setCommand("")
    inputRef?.focus()
  }

  const handleCommandKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSendCommand()
    } else if (e.key === "ArrowUp") {
      e.preventDefault()
      const history = commandHistory()
      const newIndex = Math.min(historyIndex() + 1, history.length - 1)
      setHistoryIndex(newIndex)
      if (history[newIndex]) setCommand(history[newIndex])
    } else if (e.key === "ArrowDown") {
      e.preventDefault()
      const newIndex = historyIndex() - 1
      setHistoryIndex(newIndex)
      if (newIndex < 0) {
        setCommand("")
      } else {
        const history = commandHistory()
        if (history[newIndex]) setCommand(history[newIndex])
      }
    }
  }

  // Keyboard shortcuts
  useKeyboardShortcut(
    [navigator.platform.includes("Mac") ? "Meta" : "Control", "F"],
    () => {
      if (searchOpen() && searchInputRef) {
        searchInputRef.focus()
      } else {
        setSearchOpen(true)
      }
    }
  )

  useKeyboardShortcut(["Escape"], () => {
    if (searchOpen()) {
      setSearchOpen(false)
      setSearchQuery("")
    }
  })

  return (
    <div class="relative flex h-full flex-col overflow-hidden rounded-xl border border-darkSlate-600 bg-darkSlate-900">
      {/* Toolbar */}
      <div class="box-border flex h-8 w-full shrink-0 items-center justify-between gap-4 bg-darkSlate-800 px-4 py-6">
        <div />
        <div class="flex items-center gap-4">
          <ConsoleSearch
            open={searchOpen()}
            setOpen={setSearchOpen}
            searchQuery={searchQuery()}
            setSearchQuery={setSearchQuery}
            matchCase={matchCase()}
            setMatchCase={setMatchCase}
            matchWholeWord={matchWholeWord()}
            setMatchWholeWord={setMatchWholeWord}
            useRegex={useRegex()}
            setUseRegex={setUseRegex}
            searchResults={searchResults()}
            currentResultIndex={currentResultIndex()}
            onNavigate={handleSearchNavigate}
            setSearchInputRef={(ref) => {
              searchInputRef = ref
            }}
          />
          <ConsoleOptions
            logsDensity={logsDensity()}
            setLogsDensity={setLogsDensity}
            columns={columns()}
            setColumns={setColumns}
            fontMultiplier={fontMultiplier()}
            setFontMultiplier={setFontMultiplier}
            autoFollowPreference={autoFollowPreference()}
            setAutoFollowPreference={setAutoFollowPreference}
          />
          <div
            class="animate-icons-on-hover cursor-pointer"
            onClick={() => setIsConsoleFullScreen((v) => !v)}
          >
            <div
              class={`h-5 w-5 bg-lightSlate-800 transition-colors duration-200 ease-spring ${
                isConsoleFullScreen()
                  ? "i-hugeicons:minimize-screen"
                  : "i-hugeicons:maximize-screen"
              }`}
            />
          </div>
        </div>
      </div>

      {/* LIVE badge */}
      <Show when={props.isRunning}>
        <div class="z-1 absolute right-6 top-20 flex h-10 w-fit items-center rounded-3xl bg-darkSlate-700 px-4 text-lightSlate-700">
          <div class="animate-liveCirclePulse mr-2 h-3 w-3 rounded-full bg-red-400 text-red-400" />
          <Trans key="ui:_trn_live" />
        </div>
      </Show>

      {/* Floating scroll-to-bottom button */}
      <Show when={!autoFollow() && props.isRunning && logs.length > 0}>
        <div class="absolute bottom-14 left-0 right-0 z-20 flex justify-center">
          <div class="w-72">
            <ScrollBottomButton
              onClick={scrollToBottom}
              newLogsCount={newLogsCount()}
            />
          </div>
        </div>
      </Show>

      {/* Log content */}
      <div
        class="relative box-border w-full flex-1 overflow-auto bg-darkSlate-900 py-2 pl-4 pr-2"
        id="server_console_container"
      >
        <Switch>
          <Match when={!props.isRunning}>
            <div class="flex h-full select-none items-center justify-center text-center text-xl text-lightSlate-600">
              <Trans key="logs:_trn_server_not_running" />
            </div>
          </Match>
          <Match when={props.isRunning && logs.length === 0}>
            <div class="flex h-full select-none items-center justify-center text-center text-xl text-lightSlate-600">
              <Trans key="logs:_trn_waiting_for_output" />
            </div>
          </Match>
          <Match when={logs.length > 0}>
            <VList
              data={logs}
              ref={(handle) => {
                if (handle) {
                  virtualizerRef = handle
                }
              }}
              onWheel={handleWheel}
              bufferSize={10}
            >
              {(line, index) => {
                const rowResults = () =>
                  searchOpen()
                    ? searchResults().filter((r) => r.lineIndex === index())
                    : undefined

                const baseIndex = () =>
                  searchResults().findIndex((r) => r.lineIndex === index())

                const relativeCurrentResultIndex = () => {
                  const currIdx = currentResultIndex()
                  if (
                    currIdx !== null &&
                    searchResults()[currIdx]?.lineIndex === index()
                  ) {
                    const results = rowResults()
                    if (results) {
                      return results.findIndex(
                        (r) => r === searchResults()[currIdx]
                      )
                    }
                  }
                  return -1
                }

                return (
                  <div
                    class="border-b-solid border-darkSlate-600 relative w-full break-words border-b px-3"
                    classList={{
                      "py-3": logsDensity() === "low",
                      "py-2": logsDensity() === "medium",
                      "py-1": logsDensity() === "high",
                      [LEVEL_BG[line.level ?? ""] ?? ""]:
                        !!LEVEL_BG[line.level ?? ""]
                    }}
                  >
                    {/* Timestamp */}
                    <Show when={columns().timestamp && line.timestamp}>
                      <span
                        class="z-10 rounded-md bg-darkSlate-900 py-2 pr-2 font-thin italic text-lightSlate-600"
                        classList={{
                          "text-xs": fontMultiplier() === 0,
                          "text-sm": fontMultiplier() === 1,
                          "text-base": fontMultiplier() === 2
                        }}
                      >
                        {line.timestamp}
                        <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
                      </span>
                    </Show>

                    {/* Level */}
                    <Show when={columns().level && line.level}>
                      <span
                        class={`z-10 rounded-md bg-darkSlate-900 py-2 pr-2 font-bold italic ${
                          LEVEL_COLORS[line.level ?? "INFO"]
                        }`}
                        classList={{
                          "text-xs": fontMultiplier() === 0,
                          "text-sm": fontMultiplier() === 1,
                          "text-base": fontMultiplier() === 2
                        }}
                      >
                        [{line.level}]
                        <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
                      </span>
                    </Show>

                    {/* Thread */}
                    <Show when={columns().thread && line.thread}>
                      <span
                        class="z-10 rounded-md bg-darkSlate-900 py-2 pr-2 font-thin italic"
                        classList={{
                          "text-xs": fontMultiplier() === 0,
                          "text-sm": fontMultiplier() === 1,
                          "text-base": fontMultiplier() === 2
                        }}
                      >
                        [{line.thread}]
                        <div class="absolute bottom-0 right-0 top-0 w-2 select-none bg-transparent" />
                      </span>
                    </Show>

                    {/* Message content with search highlighting */}
                    <span
                      class="whitespace-pre-wrap"
                      classList={{
                        "text-lightSlate-50":
                          !line.level ||
                          line.level === "INFO" ||
                          line.level === "DEBUG",
                        [LEVEL_COLORS[line.level ?? ""] ?? ""]:
                          !!line.level &&
                          line.level !== "INFO" &&
                          line.level !== "DEBUG",
                        "text-xs": fontMultiplier() === 0,
                        "text-sm": fontMultiplier() === 1,
                        "text-base": fontMultiplier() === 2
                      }}
                    >
                      <Show when={rowResults()?.length} fallback={line.message}>
                        <For each={rowResults()}>
                          {(result, i) => (
                            <>
                              {i() === 0 &&
                                result.pos > 0 &&
                                line.message.slice(0, result.pos)}
                              {i() > 0 &&
                                line.message.slice(
                                  rowResults()![i() - 1].pos +
                                    rowResults()![i() - 1].len,
                                  result.pos
                                )}
                              <span
                                id={`finder-result-${
                                  (baseIndex() >= 0 ? baseIndex() : 0) + i()
                                }`}
                                class="bg-yellow-500/50"
                                classList={{
                                  "outline outline-2 outline-yellow-500":
                                    relativeCurrentResultIndex() === i()
                                }}
                              >
                                {line.message.slice(
                                  result.pos,
                                  result.pos + result.len
                                )}
                              </span>
                              {i() === (rowResults()?.length ?? 0) - 1 &&
                                line.message.slice(result.pos + result.len)}
                            </>
                          )}
                        </For>
                      </Show>
                    </span>
                  </div>
                )
              }}
            </VList>
          </Match>
        </Switch>
      </div>

      {/* Command input */}
      <div class="border-t border-darkSlate-600 p-2">
        <div class="flex items-center gap-2">
          <span class="font-mono text-xs text-lightSlate-600">&gt;</span>
          <input
            ref={inputRef}
            type="text"
            class="flex-1 border-0 bg-transparent font-mono text-xs text-lightSlate-200 placeholder-lightSlate-800 outline-none"
            placeholder={
              props.isRunning
                ? t("logs:_trn_type_command")
                : t("logs:_trn_start_server_to_send")
            }
            disabled={!props.isRunning}
            value={command()}
            onInput={(e) => setCommand(e.target.value)}
            onKeyDown={handleCommandKeyDown}
          />
          <button
            class="rounded-md bg-darkSlate-700 px-3 py-1 text-xs text-lightSlate-400 transition-colors hover:bg-darkSlate-600 hover:text-lightSlate-100 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!props.isRunning || !command().trim()}
            onClick={handleSendCommand}
          >
            <Trans key="logs:_trn_send" />
          </button>
        </div>
      </div>
    </div>
  )
}

export default Console
