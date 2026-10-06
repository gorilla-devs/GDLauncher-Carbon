import { useTransContext } from "@gd/i18n"
import LoadingGif from "/assets/images/image.gif"
import { Progress } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { batch, createEffect, createSignal, Show } from "solid-js"
import { setTaskId, taskId } from "@/utils/import"
import { setExportStep } from ".."
import { exportFailureMessage } from "./exportFailureMessage"

const [failedMsg, setFailedMsg] = createSignal<string | undefined>(undefined)
export { failedMsg, setFailedMsg }

export default function Exporting() {
  const [t] = useTransContext()
  const [progress, setProgress] = createSignal(0)

  const activeTask = taskId()
  const vtask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTaskOutcome", activeTask ?? null],
    enabled: activeTask !== undefined
  }))

  createEffect(() => {
    if (activeTask === undefined || taskId() !== activeTask) return
    const outcome = vtask.data
    if (!outcome) return // Pending/absence cannot certify successful export.
    const task =
      outcome.type === "Running" || outcome.type === "Failed"
        ? outcome.value
        : undefined
    if (task?.progress.type === "Known")
      setProgress(Math.floor(task.progress.value * 100))
    if (task?.progress.type === "Indeterminate") setProgress(-1)
    const failed =
      task?.progress.type === "Failed" ? task.progress.value : undefined
    if (
      failed ||
      outcome.type === "Failed" ||
      outcome.type === "Unavailable" ||
      outcome.type === "Dismissed"
    ) {
      const message = exportFailureMessage(
        failed?.cause,
        t("instances:_trn_export_failed")
      )
      batch(() => {
        setFailedMsg(message)
        setTaskId(undefined)
        setExportStep(2)
      })
    } else if (outcome.type === "Succeeded") {
      batch(() => {
        setFailedMsg(undefined)
        setTaskId(undefined)
        setExportStep(2)
      })
    }
  })

  return (
    <div class="flex h-full w-full flex-col items-center justify-center gap-4">
      <img src={LoadingGif} class="h-40 w-40" alt="loading" />
      <span>{t("instances:_trn_exporting_instance")}</span>
      <Progress color="bg-primary-500" value={progress()} />
      <span>
        <Show when={progress() === -1}>
          {t("instances:_trn_preparing_export")}
        </Show>
        <Show when={progress() !== -1}>
          {`${progress()}% ${t("instances:_trn_export_completed")}`}
        </Show>
      </span>
    </div>
  )
}
