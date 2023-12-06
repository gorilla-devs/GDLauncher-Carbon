// export default ExportCheckbox;
import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { ExportEntry } from "@gd/core_module/bindings";
import { Checkbox } from "@gd/ui";
import { createEffect, createSignal, For, Match, Show, Switch } from "solid-js";

// Define the structure for your files and folders
const [checkedFiles, setCheckedFiles] = createSignal<ExportEntry>({
  entries: {}
});
type FileFolder = {
  name?: string;
  type?: "file" | "folder";
  path?: Array<string>;
};

const FolderDropdown = (props: {
  folder: FileFolder;
  initialData: any | undefined;
}) => {
  const [isOpen, setIsOpen] = createSignal(false);
  const [contents, setContents] = createSignal<any[]>([]);
  // const [path, setPath] = createSignal<string[]>([]);

  // Mock function to fetch contents. Replace this with your actual API call.
  // const fetchContents = (folderName: string) => {
  //   // Here you would make an API call to fetch the contents of the folder.
  //   // This is just a placeholder for demonstration purposes.

  //   setPath((prevPath) => [...prevPath, folderName]);
  //   return explore.data;
  // };

  createEffect(() => {
    if (!isOpen() && contents().length === 0) {
      rspc.createQuery(
        () => [
          "instance.explore",
          {
            instance_id: instanceId() as number,
            path: props.folder.path as Array<string>
          }
        ],
        {
          onSuccess: (data) => {
            setContents(data as any);
          }
        }
      );
      // setContents(explore.data as any);
    }
    // setIsOpen(!isOpen());
  });

  // const toggleFolder = () => {
  //   if (!isOpen()) {
  //     const explore = rspc.createQuery(() => [
  //       "instance.explore",
  //       {
  //         instance_id: instanceId() as number,
  //         path: ["config"]
  //       }
  //     ]);
  //     setContents(explore.data as any);
  //   }
  //   setIsOpen(!isOpen());
  // };
  createEffect(() => {
    console.log(checkedFiles());
  });
  return (
    <div class="flex flex-col p-1">
      <Show when={props.folder.name}>
        <div class="flex items-center gap-2">
          <div
            onClick={() => {
              setIsOpen(!isOpen());
            }}
            class={`${
              isOpen()
                ? "i-ep:arrow-down-bold"
                : "i-ep:arrow-down-bold rotate-[270deg]"
            } bg-darkSlate-500`}
          ></div>
          <Checkbox
            checked={props.folder.name! in checkedFiles().entries}
            onChange={() => {
              console.log(props.folder.path);
              setCheckedFiles(() => {
                if (props.folder.name! in checkedFiles().entries) {
                  const newCheckedFiles = { ...checkedFiles() };
                  delete newCheckedFiles.entries[props.folder.name!];
                  return newCheckedFiles;
                } else {
                  return {
                    entries: {
                      ...checkedFiles().entries,
                      [props.folder.name!]: {
                        entries: null
                      }
                    }
                  };
                }
              });
            }}
            children={<span>{props.folder.name}</span>}
          />
        </div>
      </Show>
      <div style={{ "margin-left": !props.initialData ? "20px" : "" }}>
        <Show when={isOpen() || props.initialData}>
          {/* <For each={contents()}>
            {(item) =>
              item.type === "folder" ? (
                <FolderDropdown folder={item} />
              ) : (
                <div>{item.name}</div>
              )
            }
          </For> */}
          <For each={(props.initialData as any) || contents()}>
            {(item) => (
              <div class="flex flex-row justify-between items-center ">
                <Switch>
                  <Match when={item.type === "Directory"}>
                    <FolderDropdown
                      initialData={undefined}
                      folder={{
                        name: item.name,
                        type: item.type,
                        path: [
                          ...(props.folder.path as Array<string>),
                          item.name
                        ]
                      }}
                    />
                  </Match>
                  <Match when={item.type !== "Directory"}>
                    <div class="flex items-center gap-2 p-1">
                      <div class="w-[16px] h-[16px]"></div>
                      <Checkbox
                        onChange={() => {}}
                        children={<span>{item.name}</span>}
                      />
                    </div>
                  </Match>
                </Switch>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
};

export default FolderDropdown;
