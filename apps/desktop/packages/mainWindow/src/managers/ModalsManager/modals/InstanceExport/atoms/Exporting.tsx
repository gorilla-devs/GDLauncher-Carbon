import { useTransContext } from "@gd/i18n"
import LoadingGif from "/assets/images/image.gif"
import { Progress } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { createEffect, createSignal } from "solid-js"
import { setTaskId, taskId } from "@/utils/import"
import { setExportStep } from ".."

const [failedMsg, setFailedMsg] = createSignal<string | undefined>(undefined)
export { failedMsg, setFailedMsg }

export default function Exporting() {
  const [t] = useTransContext()
  const [progress, setProgress] = createSignal(0)

  const vtask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", taskId() || null]
  }))

  createEffect(() => {
    if (vtask.data?.progress) {
      if (vtask.data.progress.type == "Known") {
        setProgress(Math.floor((vtask.data.progress.value || 1) * 100))
      }
      if (vtask.data.progress.type === "Failed") {
        setFailedMsg(vtask.data.progress.value.cause[1].display)
        setExportStep(2)
      }
    }
    const isFailed = vtask.data && vtask.data.progress.type === "Failed"
    const isDownloaded = vtask.data === null && progress() !== 0
    if (isDownloaded || isFailed) {
      setTaskId(undefined)
    }
    if (isDownloaded) {
      setExportStep(2)
    }
  })

  return (
    <div class="flex h-full w-full flex-col items-center justify-center gap-4">
      <img src={LoadingGif} class="h-40 w-40" alt="loading" />
      <span>{t("instances:_trn_exporting_instance")}</span>
      <Progress color="bg-primary-500" value={progress()} />
      <span>{`${progress()}% ${t("instances:_trn_export_completed")}`}</span>
    </div>
  )
}
