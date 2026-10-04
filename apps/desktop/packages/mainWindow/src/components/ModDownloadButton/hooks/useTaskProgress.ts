import { rspc } from "@/utils/rspcClient"
import { createSignal, createEffect, onCleanup, Accessor } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import {
  addonInstallCompletion,
  addonInstallFailure
} from "../addonInstallScope"
import { resolveTaskPoll } from "./resolveTaskPoll"

export const useTaskProgress = (
  instanceTaskIds: Accessor<Map<number, number>>,
  clearInstanceLoadingState: (instanceId: number) => void,
  addon?: { title?: string; type?: string }
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
    instanceTaskIds().forEach((taskId, instanceId) => {
      if (watched.get(instanceId) === taskId) return
      watched.set(instanceId, taskId)
      const title = addon?.title || "Addon"
      const type = addon?.type
      const current = () =>
        !disposed && instanceTaskIds().get(instanceId) === taskId
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
