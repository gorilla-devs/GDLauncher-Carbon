import { setTaskId } from "@/utils/import";
import { taskIds } from "@/utils/import";
import { isProgressFailed } from "@/utils/instances";
import { rspcFetch } from "@/utils/rspcClient";
import { Progressbar } from "@gd/ui";
import { Match, Switch, createEffect, createSignal } from "solid-js";

const [isDownloaded, setIsDownloaded] = createSignal(false);
export { isDownloaded };

const SingleImport = (props: {
  instanceIndex: number;
  instanceName: string;
  taskId?: number;
  importState: string;
}) => {
  const [progress, setProgress] = createSignal(0);
  const [state, setState] = createSignal("idle");
  createEffect(() => {
    async function runner() {
      if (taskIds() !== undefined) {
        const task: any = (await rspcFetch(() => [
          "vtask.getTask",
          props.taskId as number
        ])) as any;

        if (task.data && task.data.progress) {
          if (task.data.progress.Known) {
            setProgress(Math.floor(task.data.progress.Known * 100));
          }
        }
        const isFailed = task.data && isProgressFailed(task.data.progress);
        const isDownloaded = task.data === null && progress() !== 0;
        if (isDownloaded || isFailed) {
          setTaskId(undefined);
        }
        if (isFailed) {
          setState("failed");
        } else if (isDownloaded) {
          setState("completed");
          setIsDownloaded(true);
        }
      }
    }
    try {
      if (props.importState !== "error") {
        runner();
      }
    } catch (err) {
      console.error(err);
    }
  });

  return (
    <div class="flex gap-2 px-4 justify-between rounded-md">
      <span class="font-semibold">{props.instanceName}</span>
      <Switch>
        <Match when={state() === "idle"}>
          <div class="flex w-30 items-center gap-4">
            <Progressbar percentage={progress()} />
            <div class="font-semibold">{progress()}%</div>
          </div>
        </Match>
        <Match when={state() === "failed" || props.importState === "error"}>
          <div>
            <div class="i-ph:x-bold text-2xl text-red-600" />
          </div>
        </Match>
        <Match when={state() === "completed"}>
          <div class="i-ic:round-check text-2xl text-green-600" />
        </Match>
      </Switch>
    </div>
  );
};
export default SingleImport;
