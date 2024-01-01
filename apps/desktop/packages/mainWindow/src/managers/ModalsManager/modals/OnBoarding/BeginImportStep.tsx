import { For, createEffect, createSignal } from "solid-js";
import SingleImport from "./SingleImport";
import { rspc } from "@/utils/rspcClient";
import { setTaskIds, taskIds } from "@/utils/import";

const BeginImportStep = (props: {
  singleInstance?: string;
  instances?: Array<string>;
}) => {
  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(taskId) {
        setTaskIds([...(taskIds() || []), taskId]);
      }
    }
  );

  async function createMutations() {
    for (let i = 0; i < props.instances!.length; i++) {
      await importInstanceMutation.mutateAsync({
        name: props.instances![i],
        index: i
      });
    }
  }
  createMutations();
  return (
    <div class=" w-full h-full p-2">
      <For each={props.instances}>
        {(instance, index) => (
          <SingleImport
            instanceIndex={index()}
            instanceName={instance}
            taskId={taskIds()[index()]}
          />
        )}
      </For>
    </div>
  );
};
export default BeginImportStep;
