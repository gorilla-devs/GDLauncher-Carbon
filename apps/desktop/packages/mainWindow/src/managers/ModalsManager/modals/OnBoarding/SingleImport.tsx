import { setTaskId, taskId } from "@/utils/import";
import { setTaskIds, taskIds } from "@/utils/import";
import { isProgressFailed } from "@/utils/instances";
import { rspc, rspcFetch } from "@/utils/rspcClient";
import { FETask } from "@gd/core_module/bindings";
import { createEffect, createSignal, onCleanup } from "solid-js";

const SingleImport = (props: {
  instanceIndex: number;
  instanceName: string;
}) => {
  const [progress, setProgress] = createSignal(0);
  const [state, setState] = createSignal("idle");
  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(taskId) {
        setTaskIds((prev) => ({ ...prev, [props.instanceName]: taskId }));
      }
    }
  );
  createEffect(() => {
    async function runner() {
      if (taskIds() !== undefined) {
        const taskId = (taskIds() as any)[props.instanceName];
        const task: any = (await rspcFetch(() => [
          "vtask.getTask",
          taskId as number
        ])) as any;

        if (task.data && task.data.progress) {
          if (task.data.progress.Known) {
            setProgress(Math.floor(task.data.progress.Known * 100));
          }
        }
        const isFailed = task.data && isProgressFailed(task.data.progress);
        const isDownloaded = task.data === null;
        console.log("isFailed", isFailed);
        console.log("isDownloaded", isDownloaded);
        if (isDownloaded || isFailed) {
          setTaskId(undefined);
        }
        if (isFailed) {
          setState("failed");
        } else if (isDownloaded) {
          setState("completed");
        }
      }
    }
    runner();
  });
  createEffect(() => {
    if (taskIds()) {
      if ((taskIds() as any)[props.instanceName]) {
        return;
      }
    }
    importInstanceMutation.mutate({
      name: props.instanceName,
      index: props.instanceIndex
    });
  });
  createEffect(() => {
    console.log("ids", taskIds());
  });
  return (
    <div class="flex gap-2 border-2 border-solid shadow-md border-neutral-800 p-4 justify-between rounded-md bg-gray-900 bg-opacity-60 backdrop-blur-lg">
      <span class="font-semibold">{props.instanceName}</span>
      <span class="font-semibold">
        {state() !== "idle" ? state() : `${progress()}%`}
      </span>
    </div>
  );
};
export default SingleImport;
