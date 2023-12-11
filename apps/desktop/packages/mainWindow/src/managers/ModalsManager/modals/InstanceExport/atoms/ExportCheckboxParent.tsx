import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { createSignal } from "solid-js";
import ExportCheckbox from "./ExportCheckbox";

const [checkedFiles, setCheckedFiles] = createSignal<Array<Array<string>>>([]);
export { checkedFiles, setCheckedFiles };

export function buildNestedObject(paths: Array<Array<string>>) {
  const root: any = {};

  paths.forEach((path) => {
    let current = root;

    path.forEach((item, index) => {
      if (!current[item]) {
        current[item] = { entries: index === path.length - 1 ? null : {} };
      } else if (current[item].entries === null) {
        current[item].entries = index === path.length - 1 ? null : {};
      }
      current = current[item].entries;
    });
  });

  return root;
}
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
