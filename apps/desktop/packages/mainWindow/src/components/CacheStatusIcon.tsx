import { Show, onCleanup, onMount } from "solid-js"
import { Tab, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { Trans, useTransContext } from "@gd/i18n"
import CacheStatusPopover from "./CacheStatusPopover"

export const CacheStatusIcon = () => {
  const [t] = useTransContext()

  const tasksQuery = rspc.createQuery(() => ({
    queryKey: ["vtask.getTasks"]
  }))

  let intervalId: NodeJS.Timeout | undefined

  onMount(() => {
    intervalId = setInterval(() => {
      if (tasksQuery.data && tasksQuery.data.length > 0) {
        tasksQuery.refetch()
      }
    }, 2000)
  })

  onCleanup(() => {
    if (intervalId) {
      clearInterval(intervalId)
    }
  })

  const getCurrentTasksCount = () => {
    return tasksQuery.data?.length || 0
  }

  const hasActiveTasks = () => getCurrentTasksCount() > 0

  const getCacheTaskSummary = () => {
    if (!tasksQuery.data || tasksQuery.data.length === 0) return null

    const cacheTaskCount = tasksQuery.data.filter((task) => {
      if (typeof task.name === "object" && task.name.translation) {
        const translation = task.name.translation
        return translation.startsWith("CacheTask")
      }
      return false
    }).length

    const otherTaskCount = tasksQuery.data.length - cacheTaskCount

    if (cacheTaskCount > 0) {
      return otherTaskCount > 0
        ? `${cacheTaskCount} cache + ${otherTaskCount} other tasks`
        : cacheTaskCount === 1
          ? t("TaskStatusActiveTasks_one")
          : t("TaskStatusActiveTasks_other", { count: cacheTaskCount })
    }

    return null
  }

  return (
    <Show when={tasksQuery.data || tasksQuery.isLoading}>
      <CacheStatusPopover>
        <Tab ignored>
          <Tooltip>
            <TooltipTrigger>
              <div class="hover:text-lightSlate-100 relative cursor-pointer text-2xl transition-colors duration-200">
                <div class="i-hugeicons:database-01 h-6 w-6" />
                <Show when={hasActiveTasks()}>
                  <div class="absolute bottom-0 left-0 text-sm text-white">
                    <div class="i-hugeicons:refresh h-4 w-4 animate-spin" />
                  </div>
                </Show>
              </div>
            </TooltipTrigger>
            <TooltipContent>
              <Show
                when={hasActiveTasks()}
                fallback={<Trans key="TaskStatusIdle" />}
              >
                <Show
                  when={getCacheTaskSummary()}
                  fallback={
                    <Trans
                      key={
                        getCurrentTasksCount() === 1
                          ? "TaskStatusActiveTasks_one"
                          : "TaskStatusActiveTasks_other"
                      }
                      options={{ count: getCurrentTasksCount() }}
                    />
                  }
                >
                  {getCacheTaskSummary()}
                </Show>
              </Show>
            </TooltipContent>
          </Tooltip>
        </Tab>
      </CacheStatusPopover>
    </Show>
  )
}
