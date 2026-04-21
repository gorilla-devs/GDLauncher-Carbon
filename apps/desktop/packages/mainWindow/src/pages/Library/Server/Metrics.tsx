import { port } from "@/utils/rspcClient"
import { Progress } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { createEffect, createSignal, onCleanup, Show } from "solid-js"

interface MetricsProps {
  serverId: number
  isRunning: boolean
  xmx: number
}

interface MetricsData {
  cpuPercent: number
  memoryMb: number
  uptimeSeconds: number
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  const mins = Math.floor(seconds / 60)
  if (mins < 60) return `${mins}m ${seconds % 60}s`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}h ${mins % 60}m`
  const days = Math.floor(hours / 24)
  return `${days}d ${hours % 24}h`
}

const Metrics = (props: MetricsProps) => {
  const [metrics, setMetrics] = createSignal<MetricsData | null>(null)

  createEffect(() => {
    if (!props.isRunning) {
      setMetrics(null)
      return
    }

    const wsConnection = new WebSocket(
      `ws://127.0.0.1:${port}/server/metrics?id=${props.serverId}`
    )

    wsConnection.onmessage = (event) => {
      const data = JSON.parse(event.data) as MetricsData
      setMetrics(data)
    }

    wsConnection.onerror = () => {
      setMetrics(null)
    }

    onCleanup(() => {
      if (wsConnection && wsConnection.readyState === wsConnection.OPEN) {
        wsConnection.close()
      }
    })
  })

  const cpuPercent = () => metrics()?.cpuPercent ?? 0
  const memoryMb = () => metrics()?.memoryMb ?? 0
  const memoryPercent = () =>
    props.xmx > 0 ? Math.min((memoryMb() / props.xmx) * 100, 100) : 0
  const uptime = () => metrics()?.uptimeSeconds ?? 0

  return (
    <div class="flex flex-col gap-4 rounded-xl border border-darkSlate-600 bg-darkSlate-900 p-4">
      <h3 class="m-0 text-sm font-medium text-lightSlate-400">
        <Trans key="instances:_trn_server_metrics_title" />
      </h3>

      <Show
        when={props.isRunning}
        fallback={
          <div class="flex items-center justify-center py-8 text-sm text-lightSlate-700">
            <Trans key="instances:_trn_server_metrics_start_hint" />
          </div>
        }
      >
        {/* CPU */}
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs">
            <span class="text-lightSlate-500">CPU</span>
            <span class="font-mono text-lightSlate-300">
              {cpuPercent().toFixed(1)}%
            </span>
          </div>
          <Progress
            value={cpuPercent()}
            max={100}
            size="small"
            color={
              cpuPercent() > 80
                ? "bg-red-500"
                : cpuPercent() > 50
                  ? "bg-yellow-500"
                  : "bg-green-500"
            }
          />
        </div>

        {/* Memory */}
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs">
            <span class="text-lightSlate-500">
              <Trans key="instances:_trn_server_metrics_memory" />
            </span>
            <span class="font-mono text-lightSlate-300">
              <Trans
                key="instances:_trn_server_metrics_memory_value"
                options={{ used: memoryMb(), total: props.xmx }}
              />
            </span>
          </div>
          <Progress
            value={memoryPercent()}
            max={100}
            size="small"
            color={
              memoryPercent() > 90
                ? "bg-red-500"
                : memoryPercent() > 70
                  ? "bg-yellow-500"
                  : "bg-primary-500"
            }
          />
        </div>

        {/* Uptime */}
        <div class="flex items-center justify-between text-xs">
          <span class="text-lightSlate-500">
            <Trans key="instances:_trn_server_metrics_uptime" />
          </span>
          <span class="font-mono text-lightSlate-300">
            {formatUptime(uptime())}
          </span>
        </div>
      </Show>
    </div>
  )
}

export default Metrics
