import { rspc } from "@/utils/rspcClient"
import { createSignal, createEffect, onCleanup, Accessor } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import {
  addonInstallCompletion,
  addonInstallFailure
} from "../addonInstallScope"
import { resolveTaskPoll } from "./resolveTaskPoll"
import type { InstanceInstallTask } from "./useModInstallation"

export const useTaskProgress = (
  instanceTasks: Accessor<Map<number, InstanceInstallTask>>,
  clearInstanceLoadingState: (instanceId: number) => void
) => {
  const [t] = useTransContext()
  const ctx = rspc.useContext()
  const [loading, setLoading] = createSignal(false)
  const [progress, setProgress] = createSignal<number | null>(null)
  const watched = new Map<number, number>()
  let disposed = false
  onCleanup(() => {
    disposed = true
  })

  createEffect(() => {
    instanceTasks().forEach((accepted, instanceId) => {
      const { taskId, type } = accepted
      if (watched.get(instanceId) === taskId) return
      watched.set(instanceId, taskId)
      const title =
        accepted.title || t("notifications:_trn_addon_fallback_name")
      const current = () =>
        !disposed && instanceTasks().get(instanceId)?.taskId === taskId
      void (async () => {
        // Direct queries start AFTER this captured attempt was accepted. A stale
        // cached getTasks list from before acceptance cannot certify completion.
        while (current()) {
          try {
            const task = await ctx.client.query([
              "vtask.getTaskOutcome",
              taskId
            ])
            if (!current()) return
            const { action } = resolveTaskPoll(task, null, true)
            if (action.kind === "failed") {
              clearInstanceLoadingState(instanceId)
              toast.error(
                addonInstallFailure(t, title),
                action.message ? { description: action.message } : undefined
              )
              return // Toast delivery does not acknowledge or dismiss Failed.
            }
            if (action.kind === "dismissed") {
              clearInstanceLoadingState(instanceId)
              return
            }
            if (action.kind === "completed") {
              clearInstanceLoadingState(instanceId)
              toast.success(addonInstallCompletion(t, title, type), {
                duration: 2000
              })
              return
            }
          } catch {
            /* A failed read is not a completed installation; retry. */
          }
          await new Promise((resolve) => setTimeout(resolve, 500))
        }
      })()
    })
  })
  return { loading, setLoading, progress, setProgress }
}
