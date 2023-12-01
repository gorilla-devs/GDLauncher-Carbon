import { useTransContext } from "@gd/i18n";
import { Checkbox } from "@gd/ui";
import { For, createEffect, createSignal } from "solid-js";
import SingleCheckBox from "../../OnBoarding/SingleCheckBox";
import { rspc } from "@/utils/rspcClient";
import { payload } from "..";
import { instanceId } from "@/utils/browser";
import { ExploreEntry } from "@gd/core_module/bindings";
import { isWithinInterval } from "date-fns";

const FilesSelection = () => {
  const [t] = useTransContext();
  const [filesToSelect, setFilesToSelect] = createSignal<ExploreEntry[]>([]);
  const [files, setFiles] = createSignal<never[]>([]);

  if (typeof instanceId() === "number") {
    console.log(instanceId());
    // const explore = (path: string[]) => {
    //   return rspc.createQuery(() => [
    //     "instance.explore",
    //     {
    //       instance_id: instanceId() as number,
    //       path: path
    //     }
    //   ]);
    // };

    // const exploreDirectories = (path: string[]) => {
    //   const root = explore(path);

    //   const toBeExplored = root.data ?? [];
    //   console.log("toBeExplored: ", toBeExplored, toBeExplored.length);

    //   while (toBeExplored.length !== 0) {
    //     const current = toBeExplored.pop();
    //     if (!current) continue;
    //     if (current.type === "Directory") {
    //       console.log([...path, current.name]);
    //       const sub = explore([...path, current.name]);
    //       const subFiles = sub.data ?? [];
    //       toBeExplored.push(...subFiles);
    //     } else {
    //       setFilesToSelect([...filesToSelect(), current]);
    //     }
    //   }
    // };

    const exploreRecursively = (path: string[]) => {
      const explore = rspc.createQuery(() => [
        "instance.explore",
        {
          instance_id: instanceId() as number,
          path: path
        }
      ]);

      const files = explore.data ?? [];

      for (const file of files) {
        // console.log("file: ", file.name);
        // console.log("type: ", file.type);
        // console.log("path: ", [...path, file.name].join("/"));
        if (file.type === "Directory") {
          console.log([...path, file.name]);
          exploreRecursively([...path, file.name]);
        } else {
          setFilesToSelect([...filesToSelect(), file]);
        }
      }
    };
    exploreRecursively([]);

    // const explore = rspc.createQuery(() => [
    //   "instance.explore",
    //   {
    //     instance_id: instanceId() as number,
    //     path: []
    //   }
    // ]);
    // console.log(explore.data);
    // console.log(filesToSelect());
  }

  return (
    <div class="w-full flex flex-col gap-2 pt-2">
      <span>{t("instance.select_files_text")}</span>
      <div class="w-full rounded-md bg-darkSlate-900 h-44 overflow-y-scroll">
        <For each={filesToSelect()}>
          {(item) => (
            <div class="flex flex-row justify-between items-center h-10 px-2">
              <SingleCheckBox
                items={files()}
                title={item.name}
                setList={setFiles}
              />
              <span>{item.type["File"].size} kb</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};
export default FilesSelection;
