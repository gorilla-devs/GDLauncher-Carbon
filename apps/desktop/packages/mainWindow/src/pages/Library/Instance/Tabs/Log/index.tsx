import { LogEntry } from "@/utils/logs"
import { apiWsUrl, rspc } from "@/utils/rspcClient.js"
import { useParams } from "@solidjs/router"
import { createEffect, createSignal, onCleanup, untrack } from "solid-js"
import LogsSidebar from "./LogsSidebar"
import LogsContent from "./LogsContent"
import { createStore } from "solid-js/store"
import type { FESearchResult } from "@gd/core_module/bindings"
import { VirtualizerHandle } from "virtua/lib/solid"

export const [isFullScreen, setIsFullScreen] = createSignal(false)

export interface LogQuery {
  query: string | null
  matchCase: boolean
  matchWholeWord: boolean
  useRegex: boolean
}

const Logs = () => {
  let logsContentRef: HTMLDivElement | undefined
  let scrollBottomRef: HTMLDivElement | undefined
  let virtualizerRef: VirtualizerHandle | undefined
  const [logs, setLogs] = createStore<LogEntry[]>([])
  const [isLoading, setIsLoading] = createSignal(false)
  const [selectedLog, setSelectedLog] = createSignal<number | undefined>(
    undefined
  )
  const [autoFollowPreference, setAutoFollowPreference] = createSignal(true)
  const [autoFollow, setAutoFollow] = createSignal(true)
  const params = useParams<{ id: string }>()
  const [newLogsCount, setNewLogsCount] = createSignal(0)
  const [finderOpen, setFinderOpen] = createSignal(false)
  const [searchResults, setSearchResults] = createSignal<FESearchResult[]>()
  let markSearchDirty = () => {}
  const [query, setQuery] = createStore<LogQuery>({
    query: null,
    matchCase: false,
    matchWholeWord: false,
    useRegex: false
  })

  const logSearchResults = rspc.createQuery(() => ({
    queryKey: [
      "instance.searchLogs",
      {
        log_id: selectedLog()!,
        query: query.query!,
        match_case: query.matchCase,
        match_whole_word: query.matchWholeWord,
        use_regex: query.useRegex
      }
    ],
    enabled:
      finderOpen() &&
      !!query.query &&
      selectedLog() !== undefined &&
      selectedLog() !== null
  }))

  const availableLogEntries = rspc.createQuery(() => ({
    queryKey: ["instance.getLogs", parseInt(params.id, 10)]
  }))

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)]
  }))

  /** Running a game a previous launcher session started. That session owned
   *  the stdout pipe and took it with it, so there is no live log to select —
   *  which would otherwise read as the log simply having failed to load. */
  const isAdoptedRunning = () => {
    const state = instanceDetails.data?.state
    return state?.state === "running" && state.value.adopted
  }

  const isActive = () =>
    availableLogEntries.data?.find((log) => log.id === selectedLog())?.active

  createEffect((previousKey: string | undefined) => {
    const key = JSON.stringify([
      params.id,
      selectedLog(),
      query.query,
      query.matchCase,
      query.matchWholeWord,
      query.useRegex,
      finderOpen()
    ])
    const active = isActive()
    if (key !== previousKey) setSearchResults(undefined)
    markSearchDirty = () => {}
    if (!finderOpen() || !query.query || selectedLog() == null) return key

    let current = true
    let dirty = false
    let inFlight = false
    let timer: ReturnType<typeof setTimeout> | undefined
    const queue = () => {
      if (!current) return
      dirty = true
      if (inFlight || timer !== undefined) return
      // A fixed batch window still fires during continuous output.
      timer = setTimeout(() => {
        timer = undefined
        refresh()
      }, 250)
    }
    const refresh = () => {
      if (!current || inFlight) return
      // A request started before opening may contain an older snapshot.
      // Join it, then obtain one current snapshot after it settles.
      dirty = logSearchResults.isFetching
      inFlight = true
      // Join existing work instead of cancelling it on each chunk.
      void logSearchResults
        .refetch({ cancelRefetch: false })
        .then(
          (result) => {
            if (current) setSearchResults(result.data)
          },
          () => {
            // The query retains its failure; obsolete results have no authority.
            if (current) setSearchResults(undefined)
          }
        )
        .finally(() => {
          inFlight = false
          if (current && dirty) queue()
        })
    }
    markSearchDirty = () => {
      if (active) queue()
    }
    onCleanup(() => {
      current = false
      if (timer !== undefined) clearTimeout(timer)
      markSearchDirty = () => {}
    })
    // Opening, changing the search, or retiring a live session obtains
    // the current snapshot. Finite owners do not schedule growth refreshes.
    untrack(refresh)
    return key
  })

  createEffect(() => {
    if (!availableLogEntries.data) return
    const activeLogId = availableLogEntries.data.find((log) => log.active)?.id

    if (activeLogId !== undefined) setSelectedLog(activeLogId)
  })

  createEffect(() => {
    if (selectedLog() === undefined) return

    let current = true

    const wsConnection = new WebSocket(
      apiWsUrl(`/instance/log?id=${selectedLog()}`)
    )

    wsConnection.onmessage = (event) => {
      if (!current) return
      setIsLoading(false)
      const newLogs = JSON.parse(event.data) as LogEntry[]
      setLogs((prev) => [...prev, ...newLogs])
      if (newLogs.length > 0) markSearchDirty()

      if (!autoFollowPreference() || !virtualizerRef) return

      if (autoFollow()) {
        virtualizerRef.scrollToIndex(logs.length - 1)
        setNewLogsCount(0)
      } else {
        setNewLogsCount((prev) => prev + 1)
      }
    }

    onCleanup(() => {
      // Revoke already-queued callbacks before transport cancellation.
      current = false
      wsConnection.onmessage = null
      setLogs([])
      setIsLoading(true)

      if (
        wsConnection.readyState === wsConnection.CONNECTING ||
        wsConnection.readyState === wsConnection.OPEN
      ) {
        wsConnection.close()
      }
    })
  })

  onCleanup(() => {
    setIsLoading(false)
    setSelectedLog(undefined)
  })

  createEffect(() => {
    // autoFollowPreference call NEEDS to be here for scrollToBottom to be called when it changes
    autoFollowPreference()
    // selectedLog call NEEDS to be here for scrollToBottom to be called when it changes
    selectedLog()
    setNewLogsCount(0)
    handleScroll()
  })

  const handleScroll = () => {
    if (!logsContentRef || !virtualizerRef) return

    // This also accounts for sub-pixel rounding errors
    const isAtBottom =
      Math.abs(
        virtualizerRef.viewportSize +
          virtualizerRef.scrollOffset -
          virtualizerRef.scrollSize
      ) < 1

    if (scrollBottomRef && (!autoFollowPreference() || !isActive())) {
      scrollBottomRef.style.display = "none"
      return
    }

    if (isAtBottom) {
      setAutoFollow(true)
      if (scrollBottomRef && autoFollowPreference()) {
        scrollBottomRef.style.display = "none"
      }
    } else {
      setAutoFollow(false)
      if (scrollBottomRef && autoFollowPreference()) {
        scrollBottomRef.style.display = "flex"
      }
    }
  }

  createEffect(() => {
    if (isFullScreen() && logsContentRef) {
      logsContentRef.scrollIntoView({
        block: "start",
        inline: "end"
      })
    }
  })

  onCleanup(() => {
    setIsFullScreen(false)
  })

  const scrollToBottom = () => {
    if (logsContentRef && virtualizerRef) {
      virtualizerRef.scrollToIndex(logs.length - 1)
      setAutoFollow(true)
      setNewLogsCount(0)
      if (scrollBottomRef) {
        scrollBottomRef.style.display = "none"
      }
    }
  }

  function assignScrollBottomRef(ref: HTMLDivElement) {
    scrollBottomRef = ref
  }

  function assignLogsContentRef(ref: HTMLDivElement) {
    logsContentRef = ref
  }

  function assignVirtualizerRef(ref: VirtualizerHandle) {
    virtualizerRef = ref
  }

  return (
    <div class="border-darkSlate-600 border-t-solid flex h-full w-full overflow-hidden border">
      <LogsSidebar
        availableLogEntries={availableLogEntries.data || []}
        instanceId={parseInt(params.id, 10)}
        setSelectedLog={setSelectedLog}
        selectedLog={selectedLog()}
        isLoading={availableLogEntries.isLoading}
      />
      <LogsContent
        logs={logs}
        query={query}
        setQuery={setQuery}
        logSearchResults={searchResults()}
        onFinderVisibilityChange={setFinderOpen}
        isActive={isActive() || false}
        adopted={isAdoptedRunning()}
        isLoading={isLoading()}
        scrollToBottom={scrollToBottom}
        onScroll={handleScroll}
        assignScrollBottomRef={assignScrollBottomRef}
        assignLogsContentRef={assignLogsContentRef}
        assignVirtualizerRef={assignVirtualizerRef}
        newLogsCount={newLogsCount()}
        autoFollowPreference={autoFollowPreference()}
        setAutoFollowPreference={setAutoFollowPreference}
        scrollToIndex={virtualizerRef?.scrollToIndex ?? (() => {})}
        isIndexLoaded={(index: number) => {
          const startIndex =
            (virtualizerRef?.findItemIndex(virtualizerRef.scrollOffset) ?? 0) -
            10
          const endIndex =
            (virtualizerRef?.findItemIndex(
              virtualizerRef.scrollOffset + virtualizerRef.viewportSize
            ) ?? 0) + 10
          return index >= startIndex && index <= endIndex
        }}
      />
    </div>
  )
}

export default Logs
