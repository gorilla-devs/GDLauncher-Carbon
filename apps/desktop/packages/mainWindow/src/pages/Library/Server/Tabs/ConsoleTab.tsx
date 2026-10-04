import { useParams } from "@solidjs/router"
import { Show, createSignal, onCleanup } from "solid-js"
import { Trans } from "@gd/i18n"
import useServerData from "../server.data"
import Console from "../Console"
import Metrics from "../Metrics"

// Controls the outer layout: collapses header and prevents scrolling
export const [isConsoleFullScreen, setIsConsoleFullScreen] = createSignal(false)

const ConsoleTab = () => {
  const params = useParams<{ id: string }>()
  const routeData = useServerData()

  const serverId = () => parseInt(params.id, 10)
  const details = () => routeData.serverDetails.data
  const isRunning = () => details()?.state?.status === "running"
  // Started by an earlier session: this launcher never had its console.
  const isRecoveredRunning = () =>
    details()?.state?.status === "recoveredRunning"

  onCleanup(() => {
    setIsConsoleFullScreen(false)
  })

  return (
    <div
      class="flex w-full gap-4"
      classList={{
        "h-full": isConsoleFullScreen()
      }}
      style={{
        height: isConsoleFullScreen() ? undefined : "calc(100vh - 412px)"
      }}
    >
      <div class="min-h-0 flex-1">
        <Show
          when={isRecoveredRunning()}
          fallback={<Console serverId={serverId()} isRunning={isRunning()} />}
        >
          <div
            class="bg-darkSlate-800 text-lightSlate-400 flex h-full items-center justify-center gap-2 rounded-xl p-6 text-center text-sm"
            data-testid="server-recovered-no-console"
          >
            <div class="i-hugeicons:computer-terminal-01 h-5 w-5 shrink-0" />
            <Trans key="instances:_trn_server_recovered_console_unavailable" />
          </div>
        </Show>
      </div>
      <div class="w-64 shrink-0">
        <Metrics
          serverId={serverId()}
          isRunning={isRunning()}
          isRecovered={isRecoveredRunning()}
          xmx={details()?.xmx ?? 2048}
        />
      </div>
    </div>
  )
}

export default ConsoleTab
