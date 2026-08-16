import { rspc } from "@/utils/rspcClient"
import { createSignal, createEffect, Accessor } from "solid-js"
import { Progress } from "@gd/core_module/bindings"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { resolveTaskPoll } from "./resolveTaskPoll"

export const useTaskProgress = (
  instanceTaskIds: Accessor<Map<number, number>>,
  clearInstanceLoadingState: (instanceId: number) => void,
  addon?: { title?: string; type?: string }
) => {
  const [t] = useTransContext()
  const [loading, setLoading] = createSignal(false)
  const [progress, setProgress] = createSignal<number | null>(null)

  // Monitor all tasks for completion
  const allTasksQuery = rspc.createQuery(() => ({
    queryKey: ["vtask.getTasks"]
  }))

  const dismissTaskMutation = rspc.createMutation(() => ({
    mutationKey: ["vtask.dismissTask"]
  }))

  // Last non-`Indeterminate`/`null` progress seen per instance's tracked
  // task, mirroring the single-button flow's `lastProgress` (see
  // `resolveTaskPoll`) — kept per instance since this hook tracks every
  // instance in the dropdown install path at once.
  //
  // A plain Map, and it has to stay one: the poll effect below reads this
  // back on the pass after it writes it, so making it reactive would make
  // the effect a dependency of its own writes. A tracked task sitting at one
  // `Known` progress resolves to the same `"progress"` action on every pass,
  // so the effect would re-queue itself without end — and Solid nests a
  // `completeUpdates` -> `runUpdates` -> `runEffects` frame per re-queue, so
  // the renderer dies with `RangeError: Maximum call stack size exceeded`
  // rather than merely spinning. The effect's reactive inputs are
  // `instanceTaskIds()` and `allTasksQuery.data`, nothing else.
  const lastProgressByInstance = new Map<number, Progress | null>()

  createEffect(() => {
    const taskIds = instanceTaskIds()
    const allTasks = allTasksQuery.data

    taskIds.forEach((taskId, instanceId) => {
      // `allTasks` is `undefined` while `vtask.getTasks` hasn't resolved
      // yet — not evidence the task is gone, unlike a resolved list that
      // no longer contains it.
      const taskData =
        allTasks === undefined
          ? undefined
          : (allTasks.find((task) => task.id === taskId) ?? null)

      const previousProgress = lastProgressByInstance.get(instanceId) ?? null
      const { action, nextLastProgress } = resolveTaskPoll(
        taskData,
        previousProgress,
        addon?.type === "world"
      )

      if (action.kind === "noop") return

      if (action.kind === "progress") {
        lastProgressByInstance.set(instanceId, nextLastProgress)
        return
      }

      // Failed or completed: the tracked task is done either way, so this
      // instance's entry is retired from both maps together.
      lastProgressByInstance.delete(instanceId)

      if (action.kind === "failed") {
        clearInstanceLoadingState(instanceId)
        toast.error(
          t("notifications:_trn_addon_install_failed", {
            title: addon?.title || t("notifications:_trn_addon_fallback_name")
          }),
          action.message ? { description: action.message } : undefined
        )
        dismissTaskMutation.mutate(taskId)
      } else {
        // Clear loading state when task completes.
        // For regular mods, InstanceDropdown will also handle this via installation detection
        // For worlds, this is the only way to clear loading state since they don't appear in mods list
        clearInstanceLoadingState(instanceId)

        // Show success toast for dropdown installations — but not for a
        // task whose last observed state before disappearing was Failed
        // (already toasted and dismissed above; ignoring `resolveTaskPoll`'s
        // own `showSuccessToast` here since it's world-only for the
        // single-button flow, whereas this hook toasts for every addon
        // type).
        if (addon && previousProgress?.type !== "Failed") {
          toast.success(`${addon.title || "Addon"} installed successfully`, {
            duration: 2000
          })
        }
      }
    })
  })

  return {
    loading,
    setLoading,
    progress,
    setProgress
  }
}
