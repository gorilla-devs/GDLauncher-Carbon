import { setTaskId, setTaskIds } from "@/utils/import";
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
  importState: string;
}) => {
  const [progress, setProgress] = createSignal(0);
  const [state, setState] = createSignal("idle");
  // const rspcContext = rspc.useContext();

  createEffect(() => {
    async function runner() {
      if (taskIds() !== undefined) {
        rspc.createQuery(() => ["vtask.getTask", props.taskId || null], {
          onSuccess: (task) => {
            if (task && task.progress) {
              if (task.progress.type == "Known") {
                setProgress(Math.floor(task.progress.value * 100));
              }
            }
            const isFailed = task && task.progress.type === "Failed";
            const isDownloaded = task === null && progress() !== 0;
            if (isDownloaded || isFailed) {
              const taskIdsArray = taskIds();
              taskIdsArray[props.instanceIndex] = undefined;
              setTaskIds(taskIdsArray);
            }
            if (isFailed) {
              setState("failed");
            } else if (isDownloaded) {
              setState("completed");
              setIsDownloaded(true);
            }
          }
        });
        // const task = await rspcContext.client.query([
        //   "vtask.getTask",
        //   props.taskId || null
        // ]);
        // console.log(task);
        // if (task && task.progress) {
        //   if (task.progress.type == "Known") {
        //     setProgress(Math.floor(task.progress.value * 100));
        //   }
        // }
        // const isFailed = task && task.progress.type === "Failed";
        // const isDownloaded = task === null && progress() !== 0;
        // if (isDownloaded || isFailed) {
        //   setTaskId(undefined);
        // }
        // if (isFailed) {
        //   setState("failed");
        // } else if (isDownloaded) {
        //   setState("completed");
        //   setIsDownloaded(true);
        // }
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
        <Match when={state() === "failed" || props.importState === "error"}>
          <div>
            <div class="text-2xl i-ph:x-bold text-red-600" />
          </div>
        </Match>
        <Match when={state() === "idle"}>
          <div class="flex w-30 items-center gap-4">
            <Progressbar percentage={progress()} />
            <div class="font-semibold">{progress()}%</div>
          </div>
        </Match>

        <Match when={state() === "completed"}>
          <div class="text-2xl i-ic:round-check text-green-600" />
        </Match>
      </Switch>
    </div>
  );
};
export default SingleImport;
