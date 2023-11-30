import { useTransContext } from "@gd/i18n";
import { Checkbox } from "@gd/ui";
import { For, createEffect, createSignal } from "solid-js";
import SingleCheckBox from "../../OnBoarding/SingleCheckBox";
import { rspc } from "@/utils/rspcClient";
import { payload } from "..";
import { instanceId } from "@/utils/browser";
import { ExploreEntry } from "@gd/core_module/bindings";

const FilesSelection = () => {
  const [t] = useTransContext();
  const [filesToSelect, setFilesToSelect] = createSignal<ExploreEntry[]>([]);

  createEffect(() => {
    const exploreRecursively = (path: string[] = []) => {
      const explore = rspc.createQuery(() => [
        "instance.explore",
        {
          instance_id: instanceId() as number,
          path: path
        }
      ]);

      const data = explore.data;
      const files = data || [];

      for (const file of files) {
        console.log(file);
        if (file.type === "Directory") {
          exploreRecursively([...path, file.name]);
        } else {
          setFilesToSelect([...filesToSelect(), file]);
        }
      }
    };
    //exploreRecursively();
    // const explore = rspc.createQuery(() => [
    //   "instance.explore",
    //   {
    //     instance_id: instanceId() as number,
    //     path: []
    //   }
    // ]);
    // console.log(explore.data);
    // console.log(filesToSelect());
  });

  return (
    <div class="w-full flex flex-col gap-2 pt-2">
      <span>{t("instance.select_files_text")}</span>
      <div class="w-full rounded-md bg-darkSlate-900 h-44 overflow-y-scroll">
        {/* <For each={mockData}>
          {(item) => (
            <div class="flex flex-row justify-between items-center h-10 px-2">
              <SingleCheckBox title={item.name} setList={setFiles} />
              <span>{item.size}</span>
            </div>
          )}
        </For> */}
      </div>
    </div>
  );
};
export default FilesSelection;
