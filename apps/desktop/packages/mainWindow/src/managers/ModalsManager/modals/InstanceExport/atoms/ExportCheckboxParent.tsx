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
      <ExportCheckbox initialData={explore.data} folder={{ path: [] }} />
    </>
  );
};
export default ExportCheckboxParent;
