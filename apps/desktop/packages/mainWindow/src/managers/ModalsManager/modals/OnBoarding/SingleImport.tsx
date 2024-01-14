import { setTaskId } from "@/utils/import";
import { taskIds } from "@/utils/import";
import { rspc } from "@/utils/rspcClient";
import { Progressbar } from "@gd/ui";
import { Match, Switch, createEffect, createSignal } from "solid-js";

const [isDownloaded, setIsDownloaded] = createSignal(false);
export { isDownloaded };

const SingleImport = (props: {
  instanceIndex: number;
  instanceName: string;
  taskId?: number;
}) => {
  const [progress, setProgress] = createSignal(0);
  const [state, setState] = createSignal("idle");
  const rspcContext = rspc.useContext();

  createEffect(() => {
    async function runner() {
      if (taskIds() !== undefined) {
        const task = await rspcContext.client.query([
          "vtask.getTask",
          props.taskId || null
        ]);

        if (task && task.progress) {
          if (task.progress.type == "Known") {
            setProgress(Math.floor(task.progress.value * 100));
          }
        }
        const isFailed = task && task.progress.type === "Failed";
        const isDownloaded = task === null && progress() !== 0;
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
      runner();
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
        <Match when={state() === "failed"}>
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
