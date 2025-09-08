import { rspc } from "@/utils/rspcClient"
import { createSignal, createEffect, Accessor } from "solid-js"
import useSearchContext from "@/components/SearchInputContext"

export const useTaskProgress = (
  instanceTaskIds: Accessor<Map<number, number>>,
  clearInstanceLoadingState: (instanceId: number) => void
) => {
  const [loading, setLoading] = createSignal(false)
  const [progress, setProgress] = createSignal<number | null>(null)
  const searchContext = useSearchContext()

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
        // Don't clear loading state immediately - let the InstanceDropdown handle it
        // when it detects the mod is actually installed via the reactive effect
        // This prevents the gap between "Installing" and "Installed"
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