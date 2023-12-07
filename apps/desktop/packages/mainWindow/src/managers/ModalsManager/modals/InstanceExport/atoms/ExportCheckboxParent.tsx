import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { For, createSignal } from "solid-js";
import ExportCheckbox from "./ExportCheckbox";
import { ExportEntry } from "@gd/core_module/bindings";

const [checkedFiles, setCheckedFiles] = createSignal<Array<Array<string>>>([]);
export { checkedFiles, setCheckedFiles };

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
      <ExportCheckbox initialData={explore.data} folder={{ path: [] }} />
    </>
  );
};
export default ExportCheckboxParent;
