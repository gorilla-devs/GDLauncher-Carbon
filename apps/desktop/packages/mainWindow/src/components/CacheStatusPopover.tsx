import {
  Component,
  Show,
  For,
  onMount,
  onCleanup,
  createSignal
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  Badge,
  Tabs,
  TabList,
  Tab,
  TabPanel,
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
  const [selectedTab, setSelectedTab] = createSignal(0)

  // Data queries
  const cacheStatusQuery = rspc.createQuery(() => ({
    queryKey: ["cache.getCacheStatus"]
  }))

  const cacheHistoryQuery = rspc.createQuery(() => ({
    queryKey: ["cache.getCacheHistory"]
  }))

  const cacheStatsQuery = rspc.createQuery(() => ({
    queryKey: ["cache.getCacheStats"]
  }))

  // Clear history mutation
  const clearHistoryMutation = rspc.createMutation(() => ({
    mutationKey: ["cache.clearCacheHistory"],
    onSuccess: () => {
      cacheHistoryQuery.refetch()
      cacheStatsQuery.refetch()
    }
  }))

  // Auto-refresh for active tasks
  let intervalId: NodeJS.Timeout | undefined

  onMount(() => {
    intervalId = setInterval(() => {
      if (
        cacheStatusQuery.data?.currentTasks &&
        cacheStatusQuery.data.currentTasks.length > 0
      ) {
        cacheStatusQuery.refetch()
        cacheStatsQuery.refetch()
      }
    }, 2000)
  })

  onCleanup(() => {
    if (intervalId) clearInterval(intervalId)
  })

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
    return `${(ms / 60000).toFixed(1)}m`
  }

  const formatTaskType = (taskType: any): string => {
    console.log("formatTaskType called with:", taskType)
    
    if (typeof taskType === "string") {
      console.log("taskType is string:", taskType)
      return taskType
    }

    const key = Object.keys(taskType)[0]
    const data = taskType[key]
    
    console.log("taskType key:", key, "data:", data)

    try {
      switch (key) {
        case "FullInstanceScan":
          return t("cache.task.full_instance_scan", data) as string
        case "SingleFileCache":
          return t("cache.task.single_file_cache", data) as string
        case "ImageExtraction":
          return t("cache.task.image_extraction", {
            addon_name:
              data.addonName ||
              data.filename?.replace(".jar", "") ||
              "Unknown mod"
          })
        case "PlatformDetection":
          return t("cache.task.platform_detection", {
            addon_name:
              data.addonName ||
              data.filename?.replace(".jar", "") ||
              "Unknown mod",
            platform_type: data.platformType || "platforms"
          })
        case "UpdateCheck":
          return t("cache.task.update_check", {
            addon_name:
              data.addonName ||
              data.filename?.replace(".jar", "") ||
              "Unknown mod"
          })
        case "CacheClear":
          return t("cache.task.cache_clear")
        case "StartupScan":
          return t("cache.task.startup_scan")
        default:
          return "Unknown task"
      }
    } catch {
      return "Unknown task"
    }
  }

  const renderCurrentTask = (task: any) => (
    <div class="border border-darkSlate-600 rounded-lg p-3 bg-darkSlate-800/50 backdrop-blur-sm">
      <div class="flex items-center justify-between mb-2">
        <div class="font-medium text-lightSlate-50 text-sm">
          {formatTaskType(task.taskType)}
        </div>
        <Badge
          variant="default"
          class="text-xs bg-blue-500/20 text-blue-300 border-blue-500/30"
        >
          <Trans key="cache.status.running" />
        </Badge>
      </div>
      <div class="text-xs text-lightSlate-400 mb-2">
        <Trans
          key="cache.status.started_at"
          options={{
            time: new Date(task.startedAt).toLocaleTimeString()
          }}
        />
      </div>
      <Show when={task.status?.Running?.progress}>
        <div class="mb-2">
          <Progressbar
            percentage={Math.round(
              (task.status.Running.progress.current /
                task.status.Running.progress.total) *
                100
            )}
          />
          <div class="text-xs text-lightSlate-500 mt-1">
            {task.status.Running.progress.current} /{" "}
            {task.status.Running.progress.total}
          </div>
        </div>
      </Show>
      <Show when={task.status?.Running?.stage}>
        <div class="text-xs text-lightSlate-500 font-mono">
          {task.status.Running.stage}
        </div>
      </Show>
    </div>
  )

  const renderHistoryTask = (task: any) => (
    <div class="border border-darkSlate-600 rounded-lg p-3 bg-darkSlate-800/30">
      <div class="flex items-center justify-between mb-2">
        <div class="font-medium text-lightSlate-100 text-sm">
          {formatTaskType(task.taskType)}
        </div>
        <Badge variant={task.success ? "success" : "error"} class="text-xs">
          {task.success ? t("cache.status.success") : t("cache.status.failed")}
        </Badge>
      </div>
      <div class="flex justify-between text-xs text-lightSlate-400 mb-1">
        <span class="text-emerald-400 font-mono">
          {formatDuration(task.durationMs)}
        </span>
        <span>{new Date(task.completedAt).toLocaleString()}</span>
      </div>
      <Show when={task.errorMessage}>
        <div class="text-xs text-red-400 mt-2 p-2 bg-red-950/30 border border-red-800/30 rounded">
          {task.errorMessage}
        </div>
      </Show>
      <Show when={task.details}>
        <div class="text-xs text-lightSlate-500 mt-1">{task.details}</div>
      </Show>
    </div>
  )

  return (
    <Popover placement="bottom-end">
      <PopoverTrigger>{props.children}</PopoverTrigger>
      <PopoverContent class="w-[600px] max-h-[500px] bg-darkSlate-900 border-darkSlate-600 shadow-2xl">
        <div class="flex flex-col h-full">
          {/* Header */}
          <div class="px-4 py-3 border-b border-darkSlate-700">
            <h3 class="text-lg font-semibold text-lightSlate-50 flex items-center gap-2">
              <div class="i-ri:database-2-line text-blue-400" />
              <Trans key="cache.status.modal.title" />
            </h3>
          </div>

          {/* Content */}
          <div class="flex-1 overflow-hidden">
            <Tabs index={selectedTab()} onChange={setSelectedTab}>
              <div class="px-4 pt-3">
                <TabList>
                  <Tab>
                    <span class="text-sm">
                      <Trans key="cache.status.current_tasks" />
                    </span>
                    <Show
                      when={
                        cacheStatusQuery.data?.currentTasks &&
                        cacheStatusQuery.data.currentTasks.length > 0
                      }
                    >
                      <Badge
                        variant="default"
                        class="ml-2 text-xs bg-blue-500/20 text-blue-300 border-blue-500/30"
                      >
                        {cacheStatusQuery.data!.currentTasks.length}
                      </Badge>
                    </Show>
                  </Tab>
                  <Tab>
                    <span class="text-sm">
                      <Trans key="cache.status.task_history" />
                    </span>
                  </Tab>
                  <Tab>
                    <span class="text-sm">
                      <Trans key="cache.status.statistics" />
                    </span>
                  </Tab>
                </TabList>
              </div>

              <div class="px-4 pb-4">
                <TabPanel>
                  <div class="space-y-3 mt-4 max-h-[300px] overflow-y-auto pr-1">
                    <Show
                      when={
                        cacheStatusQuery.data?.currentTasks &&
                        cacheStatusQuery.data.currentTasks.length > 0
                      }
                      fallback={
                        <div class="text-center text-lightSlate-500 py-8">
                          <div class="i-ri:database-2-line text-3xl mb-3 text-lightSlate-600" />
                          <div class="text-sm">
                            <Trans key="cache.status.no_current_tasks" />
                          </div>
                        </div>
                      }
                    >
                      <For each={cacheStatusQuery.data!.currentTasks}>
                        {renderCurrentTask}
                      </For>
                    </Show>
                  </div>
                </TabPanel>

                <TabPanel>
                  <div class="space-y-3 mt-4">
                    <div class="flex justify-between items-center">
                      <h4 class="text-sm font-medium text-lightSlate-200">
                        <Trans key="cache.status.task_history" />
                      </h4>
                      <Show
                        when={
                          cacheHistoryQuery.data?.tasks &&
                          cacheHistoryQuery.data.tasks.length > 0
                        }
                      >
                        <Button
                          size="small"
                          variant="secondary"
                          onClick={() => clearHistoryMutation.mutate(undefined)}
                          disabled={clearHistoryMutation.isPending}
                          class="text-xs px-2 py-1"
                        >
                          <Trans key="cache.status.clear_history" />
                        </Button>
                      </Show>
                    </div>

                    <div class="max-h-[250px] overflow-y-auto pr-1">
                      <Show
                        when={
                          cacheHistoryQuery.data?.tasks &&
                          cacheHistoryQuery.data.tasks.length > 0
                        }
                        fallback={
                          <div class="text-center text-lightSlate-500 py-6">
                            <div class="i-ri:history-line text-3xl mb-3 text-lightSlate-600" />
                            <div class="text-sm">
                              <Trans key="cache.status.no_history" />
                            </div>
                          </div>
                        }
                      >
                        <div class="space-y-2">
                          <For each={cacheHistoryQuery.data!.tasks}>
                            {renderHistoryTask}
                          </For>
                        </div>
                      </Show>
                    </div>
                  </div>
                </TabPanel>

                <TabPanel>
                  <div class="grid grid-cols-2 gap-4 mt-4">
                    <div class="space-y-3">
                      <h4 class="text-sm font-medium text-lightSlate-200">
                        <Trans key="cache.status.overview" />
                      </h4>
                      <div class="space-y-2">
                        <div class="flex justify-between items-center p-2 bg-darkSlate-800/40 rounded text-xs">
                          <span class="text-lightSlate-400">
                            <Trans key="cache.stats.current_tasks" />
                          </span>
                          <span class="text-blue-400 font-mono">
                            {cacheStatsQuery.data?.currentTasks || 0}
                          </span>
                        </div>
                        <div class="flex justify-between items-center p-2 bg-darkSlate-800/40 rounded text-xs">
                          <span class="text-lightSlate-400">
                            <Trans key="cache.stats.total_completed" />
                          </span>
                          <span class="text-lightSlate-300 font-mono">
                            {cacheStatsQuery.data?.totalCompleted || 0}
                          </span>
                        </div>
                        <div class="flex justify-between items-center p-2 bg-darkSlate-800/40 rounded text-xs">
                          <span class="text-lightSlate-400">
                            <Trans key="cache.stats.successful" />
                          </span>
                          <span class="text-emerald-400 font-mono">
                            {cacheStatsQuery.data?.successful || 0}
                          </span>
                        </div>
                        <div class="flex justify-between items-center p-2 bg-darkSlate-800/40 rounded text-xs">
                          <span class="text-lightSlate-400">
                            <Trans key="cache.stats.failed" />
                          </span>
                          <span class="text-red-400 font-mono">
                            {cacheStatsQuery.data?.failed || 0}
                          </span>
                        </div>
                      </div>
                    </div>

                    <div class="space-y-3">
                      <h4 class="text-sm font-medium text-lightSlate-200">
                        <Trans key="cache.stats.performance" />
                      </h4>
                      <div class="space-y-2">
                        <div class="flex justify-between items-center p-2 bg-darkSlate-800/40 rounded text-xs">
                          <span class="text-lightSlate-400">
                            <Trans key="cache.stats.average_duration" />
                          </span>
                          <span class="text-amber-400 font-mono">
                            {formatDuration(
                              cacheStatsQuery.data?.averageDurationMs || 0
                            )}
                          </span>
                        </div>
                        <Show
                          when={
                            cacheStatsQuery.data &&
                            cacheStatsQuery.data.totalCompleted > 0
                          }
                        >
                          <div class="flex justify-between items-center p-2 bg-darkSlate-800/40 rounded text-xs">
                            <span class="text-lightSlate-400">
                              <Trans key="cache.stats.success_rate" />
                            </span>
                            <span class="text-emerald-400 font-mono">
                              {Math.round(
                                (cacheStatsQuery.data!.successful /
                                  cacheStatsQuery.data!.totalCompleted) *
                                  100
                              )}
                              %
                            </span>
                          </div>
                        </Show>
                      </div>
                    </div>
                  </div>
                </TabPanel>
              </div>
            </Tabs>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  )
}

export default CacheStatusPopover
