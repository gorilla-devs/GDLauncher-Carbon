import { useTransContext } from "@gd/i18n";
import LoadingGif from "/assets/images/image.gif";
import { Progressbar } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { createEffect, createSignal } from "solid-js";
import { setTaskId, taskId } from "@/utils/import";
import { setExportStep } from "..";
const [failedMsg, setFailedMsg] = createSignal<string | undefined>(undefined);
export { failedMsg, setFailedMsg };
export default function Exporting() {
  const [t] = useTransContext();
  const [progress, setProgress] = createSignal(0);

  createEffect(() => {
    async function runner() {
      if (taskId() !== undefined) {
        rspc.createQuery(() => ["vtask.getTask", taskId() || null], {
          onSuccess: (task) => {
            if (task && task.progress) {
              if (task.progress.type == "Known") {
                setProgress(Math.floor(task.progress.value * 100));
              }
              if (task.progress.type === "Failed") {
                setFailedMsg(task.progress.value.cause[1].display as string);
                setExportStep(2);
              }
            }
            const isFailed = task && task.progress.type === "Failed";
            const isDownloaded = task === null && progress() !== 0;
            if (isDownloaded || isFailed) {
              setTaskId(undefined);
            }
            if (isDownloaded) {
              setExportStep(2);
            }
          }
        });
      }
    }
    try {
      runner();
    } catch (err) {
      console.error(err);
    }
  });

  return (
    <div class="w-full gap-4 h-full flex flex-col items-center justify-center">
      <img src={LoadingGif} class="w-40 h-40" alt="loading" />
      <span>{t("instance.exporting_instance")}</span>
      <Progressbar color="bg-primary-500" percentage={progress()} />
      <span>{`${progress()}% ${t("instance.export_completed")}`}</span>
    </div>
  );
}
