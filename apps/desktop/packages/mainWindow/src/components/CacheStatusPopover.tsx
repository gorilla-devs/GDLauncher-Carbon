import {
  Component,
  Show,
  For,
  onMount,
  onCleanup
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  Badge,
  Progressbar,
  Popover,
  PopoverTrigger,
  PopoverContent
} from "@gd/ui"
import { rspc } from "@/utils/rspcClient"

interface CacheStatusPopoverProps {
  children: any
}

const CacheStatusPopover: Component<CacheStatusPopoverProps> = (props) => {
  const [t] = useTransContext()

  // Data queries - using available endpoints
  const tasksQuery = rspc.createQuery(() => ({
    queryKey: ["vtask.getTasks"]
  }))

  // Auto-refresh for active tasks
  let intervalId: NodeJS.Timeout | undefined

  onMount(() => {
    intervalId = setInterval(() => {
      if (tasksQuery.data && tasksQuery.data.length > 0) {
        tasksQuery.refetch()
      }
    }, 2000)
  })

  onCleanup(() => {
    if (intervalId) clearInterval(intervalId)
  })

  const formatTaskName = (task: any): string => {
    if (!task?.name) return "Unknown task"
    
    // Handle translation object
    if (typeof task.name === 'object' && task.name.translation) {
      return task.name.translation
    }
    
    return String(task.name)
  }

  const getProgressPercentage = (task: any): number => {
    if (!task?.progress) return 0
    
    if (task.progress.type === "Known") {
      return Math.round(task.progress.value * 100)
    }
    
    return 0
  }

  const renderCurrentTask = (task: any) => (
    <div class="border border-darkSlate-600 rounded-lg p-3 bg-darkSlate-800/50 backdrop-blur-sm">
      <div class="flex items-center justify-between mb-2">
        <div class="font-medium text-lightSlate-50 text-sm">
          {formatTaskName(task)}
        </div>
        <Badge
          variant="default"
          class="text-xs bg-blue-500/20 text-blue-300 border-blue-500/30"
        >
          <Trans key="cache.status.running" />
        </Badge>
      </div>
      
      <Show when={task.progress?.type === "Known"}>
        <div class="mb-2">
          <Progressbar
            percentage={getProgressPercentage(task)}
          />
          <div class="text-xs text-lightSlate-500 mt-1">
            {getProgressPercentage(task)}%
          </div>
        </div>
      </Show>
      
      <Show when={task.downloaded && task.download_total}>
        <div class="text-xs text-lightSlate-400">
          Downloaded: {Math.round(task.downloaded / 1024 / 1024)}MB / {Math.round(task.download_total / 1024 / 1024)}MB
        </div>
      </Show>
    </div>
  )

  return (
    <Popover>
      <PopoverTrigger>
        {props.children}
      </PopoverTrigger>
      <PopoverContent class="w-96 bg-darkSlate-900 border-darkSlate-600">
        <div class="p-4">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-lightSlate-50">
              <Trans key="cache.status.title" />
            </h3>
          </div>

          <div class="space-y-3">
            <Show
              when={tasksQuery.data && tasksQuery.data.length > 0}
              fallback={
                <div class="text-center py-8">
                  <div class="i-ri:database-2-line text-4xl text-lightSlate-400 mb-2" />
                  <p class="text-lightSlate-400">
                    <Trans key="cache.status.no_active_tasks" />
                  </p>
                </div>
              }
            >
              <div class="space-y-2">
                <h4 class="text-sm font-medium text-lightSlate-200 mb-2">
                  <Trans 
                    key="cache.status.active_tasks" 
                    options={{ count: tasksQuery.data?.length || 0 }}
                  />
                </h4>
                <For each={tasksQuery.data}>
                  {(task) => renderCurrentTask(task)}
                </For>
              </div>
            </Show>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  )
}

export default CacheStatusPopover