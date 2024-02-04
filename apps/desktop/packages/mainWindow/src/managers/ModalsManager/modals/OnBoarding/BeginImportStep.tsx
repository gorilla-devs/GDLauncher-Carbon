import { For, createSignal } from "solid-js";
import SingleImport from "./SingleImport";
import { rspc } from "@/utils/rspcClient";
import { setTaskIds, taskIds } from "@/utils/import";
import { globalInstances } from "./SingleEntity";

const BeginImportStep = (props: {
  singleInstance?: string;
  instances?: Array<string>;
}) => {
  const [state, setState] = createSignal<string[]>([]);
  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(taskId) {
        setTaskIds([...(taskIds() || []), taskId]);
      },
      onError() {
        setTaskIds([...(taskIds() || []), undefined]);
      }
    }
  );

  async function createMutations() {
    for (let i = 0; i < props.instances!.length; i++) {
      try {
        await importInstanceMutation.mutateAsync({
          name: props.instances![i],
          index: globalInstances().findIndex(
            (x) => x.instance_name === props.instances![i]
          )
        });
        setState([...state(), "success"]);
      } catch (error) {
        setState([...state(), "error"]);
        continue;
      }
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  if (taskIds().every((x) => x === undefined)) createMutations();
  return (
    <div class="w-full overflow-y-auto p-2 h-[240px]">
      <For each={props.instances}>
        {(instance, index) => (
          <SingleImport
            instanceIndex={index()}
            instanceName={instance}
            taskId={taskIds()[index()]}
            importState={state()[index()]}
          />
        )}
      </For>
    </div>
  );
};
export default BeginImportStep;
