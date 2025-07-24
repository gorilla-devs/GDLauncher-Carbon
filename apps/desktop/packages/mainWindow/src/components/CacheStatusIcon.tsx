import { Show, onCleanup, onMount } from "solid-js"
import { Tab, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import CacheStatusPopover from "./CacheStatusPopover"

export const CacheStatusIcon = () => {
  // Use RSPC query for cache status
  const cacheStatusQuery = rspc.createQuery(() => ({
    queryKey: ["cache.getCacheStatus"]
  }))

  let intervalId: NodeJS.Timeout | undefined

  onMount(() => {
    // Poll for updates every 2 seconds when there are active tasks
    intervalId = setInterval(() => {
      if (
        cacheStatusQuery.data?.currentTasks &&
        cacheStatusQuery.data.currentTasks.length > 0
      ) {
        cacheStatusQuery.refetch()
      }
    }, 2000)
  })

  onCleanup(() => {
    if (intervalId) {
      clearInterval(intervalId)
    }
  })

  const getCurrentTasksCount = () => {
    return cacheStatusQuery.data?.currentTasks?.length || 0
  }

  const hasActiveTasks = () => getCurrentTasksCount() > 0

  return (
    <Show when={cacheStatusQuery.data || cacheStatusQuery.isLoading}>
      <CacheStatusPopover>
        <Tab ignored>
          <Tooltip>
            <TooltipTrigger>
              <div
                class="text-2xl cursor-pointer transition-colors duration-200"
                classList={{
                  "i-ri:database-2-fill text-blue-400": hasActiveTasks(),
                  "i-ri:database-2-line text-lightSlate-300": !hasActiveTasks(),
                  "hover:text-blue-300": hasActiveTasks(),
                  "hover:text-lightSlate-100": !hasActiveTasks(),
                  "animate-spin": hasActiveTasks()
                }}
              />
            </TooltipTrigger>
            <TooltipContent>
              <Show
                when={hasActiveTasks()}
                fallback={<Trans key="cache.status.idle" />}
              >
                <Trans
                  key="cache.status.active_tasks"
                  options={{ count: getCurrentTasksCount() }}
                />
              </Show>
            </TooltipContent>
          </Tooltip>
        </Tab>
      </CacheStatusPopover>
    </Show>
  )
}
