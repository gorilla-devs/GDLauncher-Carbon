import { useParams } from "@solidjs/router"
import useServerData from "../server.data"
import Console from "../Console"
import Metrics from "../Metrics"

const ConsoleTab = () => {
  const params = useParams()
  const routeData = useServerData()

  const serverId = () => parseInt(params.id, 10)
  const details = () => routeData.serverDetails.data
  const isRunning = () => details()?.state?.status === "running"

  return (
    <div class="flex h-full w-full gap-4">
      <div class="flex-1">
        <Console
          serverId={serverId()}
          isRunning={isRunning()}
        />
      </div>
      <div class="w-64 flex-shrink-0">
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
