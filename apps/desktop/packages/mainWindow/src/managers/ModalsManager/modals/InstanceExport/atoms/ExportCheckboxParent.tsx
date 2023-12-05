import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { For } from "solid-js";
import ExportCheckbox from "./ExportCheckbox";

const ExportCheckboxParent = () => {
  const explore = rspc.createQuery(() => [
    "instance.explore",
    {
      instance_id: instanceId() as number,
      path: []
    }
  ]);
  console.log(explore.data);
  return (
    <>
      <For each={explore.data}>
        {(item) => (
          // <div class="flex flex-row justify-between items-center h-10 px-2">

          //   {/* <span>{item.type["File"].size} kb</span> */}
          // </div>
          <ExportCheckbox
            title={item.name}
            folder={{ ...item, name: item.name }}
            canExpand={item.type === "Directory"}
          />
        )}
      </For>
    </>
  );
};
export default ExportCheckboxParent;
