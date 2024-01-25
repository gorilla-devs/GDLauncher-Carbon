import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { createEffect, createSignal } from "solid-js";
import ExportCheckbox from "./ExportCheckbox";
import { Checkbox } from "@gd/ui";
import { useTransContext } from "@gd/i18n";
import _ from "lodash";
import { set } from "date-fns";

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
  const [allSelected, setAllSelected] = createSignal(false);
  const [someSelected, setSomeSelected] = createSignal(false);
  const [t] = useTransContext();
  const explore = rspc.createQuery(() => [
    "instance.explore",
    {
      instance_id: instanceId() as number,
      path: []
    }
  ]);

  createEffect(() => {
    if (!explore.data) return;
    const allChecked: boolean = explore.data?.every((item) => {
      return checkedFiles().some((checkedItem) => {
        return _.isEqual(checkedItem, [item.name]);
      });
    });

    const someChecked: boolean = explore.data?.some((item) => {
      return checkedFiles().some((checkedItem) => {
        return _.isEqual(checkedItem, [item.name]);
      });
    });

    setAllSelected(allChecked);
    setSomeSelected(!allChecked && someChecked);
  });

  const handleSelectAll = (checked: boolean) => {
    if (checked) {
      if (!explore.data) return;
      const paths: Array<Array<string>> = explore.data.map((item) => [
        item.name
      ]);
      setCheckedFiles(paths);
      setAllSelected(true);
      return;
    }
    setCheckedFiles([]);
    setAllSelected(false);
  };

  return (
    <>
      <div class="flex items-center gap-2 pt-2">
        <div class="w-6 h-6 "></div>
        <Checkbox
          onChange={handleSelectAll}
          checked={allSelected()}
          indeterminate={someSelected()}
          children={
            <span class="text-sm text-[#8A8B8F]">
              {t("instance.select_all_mods")}
            </span>
          }
        />
      </div>
      <ExportCheckbox initialData={explore.data} folder={{ path: [] }} />
    </>
  );
};
export default ExportCheckboxParent;
