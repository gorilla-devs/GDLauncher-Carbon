import { rspc } from "@/utils/rspcClient"
import { createSignal, createEffect, Accessor } from "solid-js"
import { toast } from "@gd/ui"

export const useTaskProgress = (
  instanceTaskIds: Accessor<Map<number, number>>,
  clearInstanceLoadingState: (instanceId: number) => void,
  addon?: { title?: string; type?: string }
) => {
  const [loading, setLoading] = createSignal(false)
  const [progress, setProgress] = createSignal<number | null>(null)

  // Monitor all tasks for completion
  const allTasksQuery = rspc.createQuery(() => ({
    queryKey: ["vtask.getTasks"]
  }))

  createEffect(() => {
    const taskIds = instanceTaskIds()
    const allTasks = allTasksQuery.data || []
    const activeTaskIds = new Set(allTasks.map((task) => task.id))

    taskIds.forEach((taskId, instanceId) => {
      if (!activeTaskIds.has(taskId)) {
        // Clear loading state when task completes
        // For regular mods, InstanceDropdown will also handle this via installation detection
        // For worlds, this is the only way to clear loading state since they don't appear in mods list
        clearInstanceLoadingState(instanceId)

        // Show success toast for dropdown installations
        if (addon) {
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
