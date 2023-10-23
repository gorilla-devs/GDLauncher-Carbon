import { setTaskId, taskId } from "@/utils/import";
import { isProgressFailed } from "@/utils/instances";
import { rspc } from "@/utils/rspcClient";
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
  createEffect(() => {
    if (taskId() !== undefined) {
      const task = rspc.createQuery(() => [
        "vtask.getTask",
        taskId() as number
      ]);
      const isFailed = task.data && isProgressFailed(task.data.progress);
      const isDownloaded = task.data === null;

      const currentInstance = props.instanceName;
      if (!currentInstance) return;
      const instanceIndex = props.instanceIndex;
    }
  });
  return <div>singleImport</div>;
};
export default SingleImport;
