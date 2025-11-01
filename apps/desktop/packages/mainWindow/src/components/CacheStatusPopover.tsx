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
      <div class="from-darkSlate-800 to-darkSlate-700 border-darkSlate-600/50 relative rounded-xl border bg-gradient-to-r p-4">
        <div class="mb-3 flex items-start justify-between">
          <div class="min-w-0 flex-1">
            <h4 class="text-lightSlate-50 mb-1 truncate text-sm font-medium">
              {formatTaskName(task)}
            </h4>
            <Show
              when={task.active_subtasks && task.active_subtasks.length > 0}
            >
              <p class="text-lightSlate-400 truncate text-xs">
                {formatSubtaskName(task.active_subtasks[0])}
              </p>
            </Show>
          </div>

          <div class="ml-3 flex items-center gap-2">
            <Show when={task.progress?.type === "Failed"}>
              <div class="flex items-center gap-1 rounded-md bg-red-500/20 px-2 py-1 text-xs text-red-300">
                <div class="h-1.5 w-1.5 rounded-full bg-red-400" />
                <Trans key="TaskStatusFailed" />
              </div>
            </Show>
            <Show when={task.progress?.type !== "Failed"}>
              <div class="flex items-center gap-1 rounded-md bg-blue-500/20 px-2 py-1 text-xs text-blue-300">
                <div class="h-1.5 w-1.5 animate-pulse rounded-full bg-blue-400" />
                <Trans key="TaskStatusRunning" />
              </div>
            </Show>
          </div>
        </div>

        <Show when={task.progress?.type === "Known"}>
          <div class="mb-3">
            <div class="mb-1 flex items-center justify-between">
              <span class="text-lightSlate-400 text-xs">Progress</span>
              <span class="text-lightSlate-300 text-xs font-medium">
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
          <div class="border-darkSlate-600/50 mt-3 border-t pt-3">
            <div class="text-lightSlate-400 mb-2 text-xs">Subtasks</div>
            <div class="space-y-2">
              <For each={task.active_subtasks}>
                {(subtask) => (
                  <div class="flex items-center justify-between py-1 text-xs">
                    <span class="text-lightSlate-400 flex items-center gap-2">
                      <div class="bg-lightSlate-500 h-1 w-1 rounded-full" />
                      {formatSubtaskName(subtask)}
                    </span>
                    <Show when={getSubtaskProgress(subtask)}>
                      <span class="text-lightSlate-300 bg-darkSlate-600/50 rounded px-1.5 py-0.5 font-mono">
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
          <div class="mt-3 rounded-lg border border-red-500/20 bg-red-500/10 p-2">
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
      <PopoverContent class="bg-darkSlate-900/95 border-darkSlate-600/50 w-[420px] shadow-2xl backdrop-blur-xl">
        <div class="p-5">
          <div class="mb-6 flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="rounded-lg bg-blue-500/10 p-2">
                <div class="i-hugeicons:tick-02 text-lg text-blue-400" />
              </div>
              <div>
                <h3 class="text-lightSlate-50 text-lg font-semibold">
                  <Trans key="TaskStatusTitle" />
                </h3>
                <Show when={tasksQuery.data && tasksQuery.data.length > 0}>
                  <p class="text-lightSlate-400 mt-0.5 text-xs">
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
              <div class="flex items-center gap-1 rounded-md bg-green-500/10 px-2 py-1 text-xs text-green-400">
                <div class="h-1.5 w-1.5 animate-pulse rounded-full bg-green-400" />
                Active
              </div>
            </Show>
          </div>

          <div class="space-y-3">
            <Show
              when={tasksQuery.data && tasksQuery.data.length > 0}
              fallback={
                <div class="py-12 text-center">
                  <div class="mb-4 flex justify-center">
                    <div class="bg-lightSlate-800/30 rounded-full p-4">
                      <div class="i-hugeicons:tick-double-02 text-3xl text-lightSlate-400" />
                    </div>
                  </div>
                  <h4 class="text-lightSlate-300 mb-2 font-medium">
                    <Trans key="TaskStatusNoActiveTasks" />
                  </h4>
                  <p class="text-lightSlate-500 mx-auto max-w-xs text-xs leading-relaxed">
                    <Trans key="TaskStatusOperationsInfo" />
                  </p>
                </div>
              }
            >
              <div class="max-h-96 space-y-3 overflow-y-auto">
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
