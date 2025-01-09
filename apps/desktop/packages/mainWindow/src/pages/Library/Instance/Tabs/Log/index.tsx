import { LogEntry } from "@/utils/logs"
import { port, rspc } from "@/utils/rspcClient.js"
import { useParams } from "@solidjs/router"
import { createEffect, createSignal, onCleanup } from "solid-js"
import LogsSidebar from "./LogsSidebar"
import LogsContent from "./LogsContent"
import { createStore } from "solid-js/store"
import { VirtualizerHandle } from "virtua/lib/solid"

export const [isFullScreen, setIsFullScreen] = createSignal(false)

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
  const params = useParams()
  const [newLogsCount, setNewLogsCount] = createSignal(0)

  const availableLogEntries = rspc.createQuery(() => ({
    queryKey: ["instance.getLogs", parseInt(params.id, 10)]
  }))

  const isActive = () =>
    availableLogEntries.data?.find((log) => log.id === selectedLog())?.active

  createEffect(() => {
    if (!availableLogEntries.data) return
    const activeLogId = availableLogEntries.data.find((log) => log.active)?.id

    if (activeLogId !== undefined) setSelectedLog(activeLogId)
  })

  createEffect(() => {
    if (selectedLog() === undefined) return

    const wsConnection = new WebSocket(
      `ws://127.0.0.1:${port}/instance/log?id=${selectedLog()}`
    )

    wsConnection.onmessage = (event) => {
      setIsLoading(false)
      const newLogs = JSON.parse(event.data) as LogEntry[]
      setLogs((prev) => [...prev, ...newLogs])

      if (!autoFollowPreference() || !virtualizerRef) return

      if (autoFollow()) {
        virtualizerRef.scrollToIndex(logs.length - 1)
        setNewLogsCount(0)
      } else {
        setNewLogsCount((prev) => prev + 1)
      }
    }

    onCleanup(() => {
      setLogs([])
      setIsLoading(true)

      if (wsConnection && wsConnection.readyState === wsConnection.OPEN) {
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
        setSelectedLog={setSelectedLog}
        selectedLog={selectedLog()}
        isLoading={availableLogEntries.isLoading}
      />
      <LogsContent
        logs={logs}
        isActive={isActive() || false}
        isLoading={isLoading()}
        scrollToBottom={scrollToBottom}
        onScroll={handleScroll}
        assignScrollBottomRef={assignScrollBottomRef}
        assignLogsContentRef={assignLogsContentRef}
        assignVirtualizerRef={assignVirtualizerRef}
        newLogsCount={newLogsCount()}
        autoFollowPreference={autoFollowPreference()}
        setAutoFollowPreference={setAutoFollowPreference}
      />
    </div>
  )
}

export default Logs
