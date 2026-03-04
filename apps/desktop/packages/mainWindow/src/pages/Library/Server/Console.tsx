import { port, rspc } from "@/utils/rspcClient"
import {
  createEffect,
  createSignal,
  onCleanup,
  Show
} from "solid-js"
import { createStore } from "solid-js/store"

interface ConsoleProps {
  serverId: number
  isRunning: boolean
}

interface LogLine {
  timestamp?: string
  message: string
}

const Console = (props: ConsoleProps) => {
  const [logs, setLogs] = createStore<string[]>([])
  const [command, setCommand] = createSignal("")
  const [autoFollow, setAutoFollow] = createSignal(true)
  let logsContainerRef: HTMLDivElement | undefined
  let inputRef: HTMLInputElement | undefined

  const sendCommandMutation = rspc.createMutation(() => ({
    mutationKey: ["server.sendConsoleCommand"]
  }))

  createEffect(() => {
    if (!props.isRunning) {
      setLogs([])
      return
    }

    const wsConnection = new WebSocket(
      `ws://127.0.0.1:${port}/server/log?id=${props.serverId}`
    )

    wsConnection.onmessage = (event) => {
      const data = JSON.parse(event.data)

      if (data.error) {
        setLogs((prev) => [...prev, `[Error] ${data.error}`])
        return
      }

      if (Array.isArray(data)) {
        setLogs(data)
      } else if (typeof data === "string") {
        setLogs((prev) => [...prev, data])
      }

      if (autoFollow() && logsContainerRef) {
        requestAnimationFrame(() => {
          logsContainerRef!.scrollTop = logsContainerRef!.scrollHeight
        })
      }
    }

    wsConnection.onerror = () => {
      setLogs((prev) => [...prev, "[Connection error]"])
    }

    onCleanup(() => {
      if (wsConnection && wsConnection.readyState === wsConnection.OPEN) {
        wsConnection.close()
      }
    })
  })

  const handleScroll = () => {
    if (!logsContainerRef) return
    const isAtBottom =
      Math.abs(
        logsContainerRef.scrollHeight -
          logsContainerRef.scrollTop -
          logsContainerRef.clientHeight
      ) < 2
    setAutoFollow(isAtBottom)
  }

  const handleSendCommand = () => {
    const cmd = command().trim()
    if (!cmd) return

    sendCommandMutation.mutate({
      id: props.serverId,
      command: cmd
    })
    setCommand("")
    inputRef?.focus()
  }

  const scrollToBottom = () => {
    if (logsContainerRef) {
      logsContainerRef.scrollTop = logsContainerRef.scrollHeight
      setAutoFollow(true)
    }
  }

  return (
    <div class="flex h-full flex-col rounded-xl border border-darkSlate-600 bg-darkSlate-900">
      <div class="flex items-center justify-between border-b border-darkSlate-600 px-4 py-2">
        <span class="text-xs font-medium text-lightSlate-600">Console</span>
        <div class="flex items-center gap-2">
          <Show when={!autoFollow() && props.isRunning}>
            <button
              class="flex items-center gap-1 rounded-md bg-darkSlate-700 px-2 py-1 text-xs text-lightSlate-400 hover:text-lightSlate-100 transition-colors"
              onClick={scrollToBottom}
            >
              <div class="i-hugeicons:arrow-down-01 h-3 w-3" />
              Scroll to bottom
            </button>
          </Show>
          <span class="text-xs text-lightSlate-800">
            {logs.length} lines
          </span>
        </div>
      </div>

      <div
        ref={logsContainerRef}
        class="flex-1 overflow-y-auto overflow-x-hidden p-3 font-mono text-xs leading-5"
        onScroll={handleScroll}
      >
        <Show
          when={props.isRunning}
          fallback={
            <div class="flex h-full items-center justify-center text-lightSlate-700">
              Server is not running
            </div>
          }
        >
          <Show
            when={logs.length > 0}
            fallback={
              <div class="flex h-full items-center justify-center text-lightSlate-700">
                Waiting for output...
              </div>
            }
          >
            {logs.map((line) => (
              <div class="whitespace-pre-wrap break-all text-lightSlate-300 hover:bg-darkSlate-800/50">
                {line}
              </div>
            ))}
          </Show>
        </Show>
      </div>

      <div class="border-t border-darkSlate-600 p-2">
        <div class="flex items-center gap-2">
          <span class="text-lightSlate-600 text-xs font-mono">&gt;</span>
          <input
            ref={inputRef}
            type="text"
            class="flex-1 bg-transparent border-0 text-xs font-mono text-lightSlate-200 placeholder-lightSlate-800 outline-none"
            placeholder={
              props.isRunning
                ? "Type a command..."
                : "Start the server to send commands"
            }
            disabled={!props.isRunning}
            value={command()}
            onInput={(e) => setCommand(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleSendCommand()
              }
            }}
          />
          <button
            class="rounded-md bg-darkSlate-700 px-3 py-1 text-xs text-lightSlate-400 hover:bg-darkSlate-600 hover:text-lightSlate-100 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            disabled={!props.isRunning || !command().trim()}
            onClick={handleSendCommand}
          >
            Send
          </button>
        </div>
      </div>
    </div>
  )
}

export default Console
