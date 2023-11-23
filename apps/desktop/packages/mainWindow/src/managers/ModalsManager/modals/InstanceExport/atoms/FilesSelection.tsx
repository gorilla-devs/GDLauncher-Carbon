import { useTransContext } from "@gd/i18n";
import { Checkbox } from "@gd/ui";
import { For, createSignal } from "solid-js";
import SingleCheckBox from "../../OnBoarding/SingleCheckBox";

const FilesSelection = () => {
  const [t] = useTransContext();
  const [files, setFiles] = createSignal([]);
  const mockData = [
    {
      name: "config",
      size: "1.2 MB",
      type: "folder",
      children: [
        {
          name: "config.json",
          size: "1.2 MB",
          type: "file",
          children: false
        }
      ]
    },
    {
      name: "mods",
      size: "1.2 MB",
      type: "file",
      children: false
    },
    {
      name: "resourcepacks",
      size: "1.2 MB",
      type: "file",
      children: false
    },
    {
      name: "saves",
      size: "1.2 MB",
      type: "folder",
      children: [
        {
          name: "config.json",
          size: "1.2 MB",
          type: "file"
        }
      ]
    },
    {
      name: "screenshots",
      size: "1.2 MB",
      type: "folder",
      children: [
        {
          name: "config.json",
          size: "1.2 MB",
          type: "file"
        }
      ]
    },
    {
      name: "libraries",
      size: "1.2 MB",
      type: "folder",
      children: [
        {
          name: "config.json",
          size: "1.2 MB",
          type: "file"
        }
      ]
    },
    {
      name: "versions",
      size: "1.2 MB",
      type: "folder",
      children: [
        {
          name: "config.json",
          size: "1.2 MB",
          type: "file"
        }
      ]
    },
    {
      name: "logs",
      size: "1.2 MB",
      type: "folder",
      children: [
        {
          name: "config.json",
          size: "1.2 MB",
          type: "file"
        }
      ]
    }
  ];
  return (
    <div class="w-full flex flex-col gap-2 pt-2">
      <span>{t("instance.select_files_text")}</span>
      <div class="w-full rounded-md bg-darkSlate-900 h-44 overflow-y-scroll">
        <For each={mockData}>
          {(item) => (
            <div class="flex flex-row justify-between items-center h-10 px-2">
              <SingleCheckBox title={item.name} setList={setFiles} />
              <span>{item.size}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};
export default FilesSelection;
