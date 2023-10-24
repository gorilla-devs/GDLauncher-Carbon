import { setTaskId, taskId } from "@/utils/import";
import { isProgressFailed } from "@/utils/instances";
import { rspc, rspcFetch } from "@/utils/rspcClient";
import { FETask } from "@gd/core_module/bindings";
import { createEffect } from "solid-js";

const SingleImport = (props: {
  instanceIndex: number;
  instanceName: string;
}) => {
  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(taskId) {
        setTaskId(taskId);
      }
    }
  );
  createEffect(async () => {
    if (taskId() !== undefined) {
      const task: FETask = (await rspcFetch(() => [
        "vtask.getTask",
        taskId() as number
      ])) as FETask;
      const isFailed = task && isProgressFailed(task.progress);
      const isDownloaded = task === null;

      const currentInstance = props.instanceName;
      if (!currentInstance) return;
      const instanceIndex = props.instanceIndex;
    } else {
      importInstanceMutation.mutate({
        name: props.instanceName,
        index: props.instanceIndex
      });
    }
  });
  return <div>singleImport</div>;
};
export default SingleImport;
