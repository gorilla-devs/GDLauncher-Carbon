import { useTransContext } from "@gd/i18n";
import LoadingGif from "/assets/images/image.gif";
import { Progressbar } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { createEffect, createSignal } from "solid-js";
import { setTaskId, taskId } from "@/utils/import";
import { setExportStep } from "..";
export default function Exporting() {
  const [t] = useTransContext();
  const [progress, setProgress] = createSignal(0);
  const rspcContext = rspc.useContext();

  createEffect(() => {
    async function runner() {
      if (taskId() !== undefined) {
        const task = await rspcContext.client.query([
          "vtask.getTask",
          taskId() || null
        ]);

        if (task && task?.progress) {
          if (task.progress.type === "Known") {
            setProgress(Math.floor(task.progress.value * 100));
          }
        }
        const isFailed = task && task.progress;
        const isDownloaded = task === null;
        if (isDownloaded || isFailed) {
          setTaskId(undefined);
        }
        if (isDownloaded) {
          setExportStep(2);
        }
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
      <img src={LoadingGif} class="h-40 w-40" alt="loading" />
      <span>{t("instance.exporting_instance")}</span>
      <Progressbar color="bg-primary-500" percentage={progress()} />
      <span>{`${progress()}% ${t("instance.export_completed")}`}</span>
    </div>
  );
}
