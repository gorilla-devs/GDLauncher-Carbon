import { useParams } from "@solidjs/router"
import { createSignal, onCleanup } from "solid-js"
import useServerData from "../server.data"
import Console from "../Console"
import Metrics from "../Metrics"

// Controls the outer layout: collapses header and prevents scrolling
export const [isConsoleFullScreen, setIsConsoleFullScreen] = createSignal(false)

const ConsoleTab = () => {
  const params = useParams()
  const routeData = useServerData()

  const serverId = () => parseInt(params.id!, 10)
  const details = () => routeData.serverDetails.data
  const isRunning = () => details()?.state?.status === "running"

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
        <Console serverId={serverId()} isRunning={isRunning()} />
      </div>
      <div class="w-64 shrink-0">
        <Metrics
          serverId={serverId()}
          isRunning={isRunning()}
          xmx={details()?.xmx ?? 2048}
        />
      </div>
    </div>
  )
}

export default ConsoleTab
