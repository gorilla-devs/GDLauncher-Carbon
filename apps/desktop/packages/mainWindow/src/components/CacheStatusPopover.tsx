import { Component, Show, For, onMount, onCleanup } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { Progress, Popover, PopoverTrigger, PopoverContent } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"

interface CacheStatusPopoverProps {
  children: any
}

const CacheStatusPopover: Component<CacheStatusPopoverProps> = (props) => {
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
    if (intervalId) clearInterval(intervalId)
  })

  const formatTaskName = (task: any): string => {
    if (!task?.name) return "Unknown task"

    if (typeof task.name === "object") {
      const translation = task.name.translation
      const args = task.name.args

      if (translation === "CacheTaskLocal" && args?.instance_name) {
        return `🗂️ Local cache: ${args.instance_name}`
      }
      if (translation === "CacheTaskCurseForge" && args?.instance_name) {
        return `🔥 CurseForge: ${args.instance_name}`
      }
      if (translation === "CacheTaskModrinth" && args?.instance_name) {
        return `🟢 Modrinth: ${args.instance_name}`
      }

      if (translation) {
        return translation
          .replace(/([A-Z])/g, " $1")
          .replace(/^./, (str: string) => str.toUpperCase())
          .trim()
      }
    }

    return String(task.name)
  }

  const formatSubtaskName = (subtask: any): string => {
    if (!subtask?.name) return "Unknown subtask"

    if (typeof subtask.name === "object") {
      const translation = subtask.name.translation
      const args = subtask.name.args

      if (translation === "CacheSubtaskScanningFiles") {
        return t("CacheSubtaskScanningDirectories")
      }
      if (translation === "CacheSubtaskQueryingPlatform" && args?.platform) {
        return t("CacheSubtaskQueryingPlatformDatabase", {
          platform: args.platform
        })
      }
      if (translation === "CacheSubtaskDownloadingImages") {
        return t("CacheSubtaskDownloadingThumbnails")
      }
      if (translation === "CacheSubtaskFinalizingCache") {
        return t("CacheSubtaskProcessingFiles")
      }

      if (translation) {
        return translation
          .replace(/([A-Z])/g, " $1")
          .replace(/^./, (str: string) => str.toUpperCase())
          .trim()
      }
    }

    return String(subtask.name)
  }

  const getSubtaskProgress = (subtask: any): string => {
    if (!subtask?.progress) return ""

    const progress = subtask.progress

    if (progress.current !== undefined && progress.total !== undefined) {
      return `${progress.current}/${progress.total}`
    }

    if (
      progress.Item &&
      progress.Item.current !== undefined &&
      progress.Item.total !== undefined
    ) {
      return `${progress.Item.current}/${progress.Item.total}`
    }

    if (
      progress.item &&
      progress.item.current !== undefined &&
      progress.item.total !== undefined
    ) {
      const current = progress.item.current
      const total = progress.item.total

      if (typeof subtask.name === "object" && subtask.name.translation) {
        const translation = subtask.name.translation
        if (translation === "CacheSubtaskScanningFiles") {
          return t("CacheProgressDirectories", { current, total })
        }
        if (translation === "CacheSubtaskFinalizingCache") {
          return t("CacheProgressFiles", { current, total })
        }
      }
      return `${current}/${total}`
    }

    if (progress.downloaded !== undefined && progress.total !== undefined) {
      return `${Math.round(progress.downloaded / 1024 / 1024)}MB / ${Math.round(progress.total / 1024 / 1024)}MB`
    }

    if (
      progress.Download &&
      progress.Download.downloaded !== undefined &&
      progress.Download.total !== undefined
    ) {
      return `${Math.round(progress.Download.downloaded / 1024 / 1024)}MB / ${Math.round(progress.Download.total / 1024 / 1024)}MB`
    }

    if (
      progress.download &&
      progress.download.downloaded !== undefined &&
      progress.download.total !== undefined
    ) {
      return `${Math.round(progress.download.downloaded / 1024 / 1024)}MB / ${Math.round(progress.download.total / 1024 / 1024)}MB`
    }

    return ""
  }

  const getProgressPercentage = (task: any): number => {
    if (!task?.progress) return 0

    if (task.progress.type === "Known") {
      return Math.round(task.progress.value * 100)
    }

    return 0
  }

  const renderCurrentTask = (task: any) => {
    return (
      <div class="relative bg-gradient-to-r from-darkSlate-800 to-darkSlate-700 rounded-xl p-4 border border-darkSlate-600/50">
        <div class="flex items-start justify-between mb-3">
          <div class="flex-1 min-w-0">
            <h4 class="font-medium text-lightSlate-50 text-sm truncate mb-1">
              {formatTaskName(task)}
            </h4>
            <Show
              when={task.active_subtasks && task.active_subtasks.length > 0}
            >
              <p class="text-xs text-lightSlate-400 truncate">
                {formatSubtaskName(task.active_subtasks[0])}
              </p>
            </Show>
          </div>

          <div class="flex items-center gap-2 ml-3">
            <Show when={task.progress?.type === "Failed"}>
              <div class="flex items-center gap-1 px-2 py-1 bg-red-500/20 text-red-300 rounded-md text-xs">
                <div class="w-1.5 h-1.5 bg-red-400 rounded-full" />
                <Trans key="TaskStatusFailed" />
              </div>
            </Show>
            <Show when={task.progress?.type !== "Failed"}>
              <div class="flex items-center gap-1 px-2 py-1 bg-blue-500/20 text-blue-300 rounded-md text-xs">
                <div class="w-1.5 h-1.5 bg-blue-400 rounded-full animate-pulse" />
                <Trans key="TaskStatusRunning" />
              </div>
            </Show>
          </div>
        </div>

        <Show when={task.progress?.type === "Known"}>
          <div class="mb-3">
            <div class="flex items-center justify-between mb-1">
              <span class="text-xs text-lightSlate-400">Progress</span>
              <span class="text-xs text-lightSlate-300 font-medium">
                {getProgressPercentage(task)}%
              </span>
            </div>
            <Progress value={getProgressPercentage(task)} />
          </div>
        </Show>

        <Show when={task.downloaded && task.download_total}>
          <div class="mb-3">
            <div class="flex items-center justify-between text-xs">
              <span class="text-lightSlate-400">Downloaded</span>
              <span class="text-lightSlate-300 font-mono">
                {Math.round(task.downloaded / 1024 / 1024)}MB /{" "}
                {Math.round(task.download_total / 1024 / 1024)}MB
              </span>
            </div>
          </div>
        </Show>

        <Show when={task.active_subtasks && task.active_subtasks.length > 0}>
          <div class="mt-3 pt-3 border-t border-darkSlate-600/50">
            <div class="text-xs text-lightSlate-400 mb-2">Subtasks</div>
            <div class="space-y-2">
              <For each={task.active_subtasks}>
                {(subtask) => (
                  <div class="flex items-center justify-between text-xs py-1">
                    <span class="text-lightSlate-400 flex items-center gap-2">
                      <div class="w-1 h-1 bg-lightSlate-500 rounded-full" />
                      {formatSubtaskName(subtask)}
                    </span>
                    <Show when={getSubtaskProgress(subtask)}>
                      <span class="text-lightSlate-300 font-mono bg-darkSlate-600/50 px-1.5 py-0.5 rounded">
                        {getSubtaskProgress(subtask)}
                      </span>
                    </Show>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>

        <Show when={task.progress?.type === "Failed"}>
          <div class="mt-3 p-2 bg-red-500/10 border border-red-500/20 rounded-lg">
            <div class="text-xs text-red-300">
              {task.progress?.value?.message || t("CacheErrorGeneric")}
            </div>
          </div>
        </Show>
      </div>
    )
  }

  return (
    <Popover>
      <PopoverTrigger>{props.children}</PopoverTrigger>
      <PopoverContent class="w-[420px] bg-darkSlate-900/95 backdrop-blur-xl border-darkSlate-600/50 shadow-2xl">
        <div class="p-5">
          <div class="flex items-center justify-between mb-6">
            <div class="flex items-center gap-3">
              <div class="p-2 bg-blue-500/10 rounded-lg">
                <div class="i-ri:task-line text-blue-400 text-lg" />
              </div>
              <div>
                <h3 class="text-lg font-semibold text-lightSlate-50">
                  <Trans key="TaskStatusTitle" />
                </h3>
                <Show when={tasksQuery.data && tasksQuery.data.length > 0}>
                  <p class="text-xs text-lightSlate-400 mt-0.5">
                    <Trans
                      key={
                        tasksQuery.data?.length === 1
                          ? "TaskStatusActiveCount_one"
                          : "TaskStatusActiveCount_other"
                      }
                      options={{ count: tasksQuery.data?.length }}
                    />
                  </p>
                </Show>
              </div>
            </div>

            <Show when={tasksQuery.data && tasksQuery.data.length > 0}>
              <div class="flex items-center gap-1 px-2 py-1 bg-green-500/10 text-green-400 rounded-md text-xs">
                <div class="w-1.5 h-1.5 bg-green-400 rounded-full animate-pulse" />
                Active
              </div>
            </Show>
          </div>

          <div class="space-y-3">
            <Show
              when={tasksQuery.data && tasksQuery.data.length > 0}
              fallback={
                <div class="text-center py-12">
                  <div class="flex justify-center mb-4">
                    <div class="p-4 bg-lightSlate-800/30 rounded-full">
                      <div class="i-ri:checkbox-circle-line text-3xl text-lightSlate-400" />
                    </div>
                  </div>
                  <h4 class="text-lightSlate-300 font-medium mb-2">
                    <Trans key="TaskStatusNoActiveTasks" />
                  </h4>
                  <p class="text-xs text-lightSlate-500 max-w-xs mx-auto leading-relaxed">
                    <Trans key="TaskStatusOperationsInfo" />
                  </p>
                </div>
              }
            >
              <div class="space-y-3 max-h-96 overflow-y-auto">
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
