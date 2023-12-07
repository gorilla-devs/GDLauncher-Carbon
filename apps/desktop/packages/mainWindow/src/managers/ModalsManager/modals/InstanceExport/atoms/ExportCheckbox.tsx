// export default ExportCheckbox;
import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { ExportEntry } from "@gd/core_module/bindings";
import { Checkbox } from "@gd/ui";
import {
  createEffect,
  createSignal,
  For,
  Match,
  onMount,
  Show,
  Switch
} from "solid-js";
import { checkedFiles, setCheckedFiles } from "./ExportCheckboxParent";
import { set } from "date-fns";
import _ from "lodash";
import { is } from "date-fns/locale";

// Define the structure for your files and folders

type FileFolder = {
  name?: string;
  type?: "file" | "folder" | "Directory";
  path?: Array<string>;
};
export function isSubsetOf(needle: Array<string>, haystack: Array<string>) {
  return needle.every((val, idx) => haystack[idx] === val);
}

const FileCheckbox = (props: { file: FileFolder; name: string }) => {
  const handleChange = (checked: boolean, path: Array<string>) => {
    if (checked) {
      setCheckedFiles((prev) => [...prev, path]);
      return;
    }
    setCheckedFiles((prev) =>
      prev.filter((item) => {
        return !isSubsetOf(item, path);
      })
    );
  };

  createEffect(() => {
    console.log(props.file);
    const path = [...(props.file.path as Array<string>), props.name as string];

    const isAlreadyInList = checkedFiles().some((item) =>
      _.isEqual(item, path)
    );

    if (isAlreadyInList) {
      return;
    }

    const isAreadyChecked = checkedFiles().some((item) =>
      isSubsetOf(item, path)
    );

    if (isAreadyChecked) {
      setCheckedFiles((prev) => [...prev, path]);
    }
  });

  return (
    <Checkbox
      checked={checkedFiles().some((item) =>
        _.isEqual(item, [...(props.file.path as Array<string>), props.name])
      )}
      onChange={(checked: boolean) => {
        handleChange(checked, [
          ...(props.file.path as Array<string>),
          props.name as string
        ]);
      }}
      children={<span>{props.name}</span>}
    />
  );
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
  createEffect(() => {
    console.log(checkedFiles());
  });

  const handleChange = (checked: boolean, path: Array<string>) => {
    if (checked) {
      setCheckedFiles((prev) => [...prev, path]);
      return;
    }
    setCheckedFiles((prev) =>
      prev.filter((item) => !isSubsetOf(item, path) && !isSubsetOf(path, item))
    );
  };

  createEffect(() => {
    const path = _.cloneDeep(props.folder.path as Array<string>);

    const isAlreadyInList = checkedFiles().some((item) =>
      _.isEqual(item, path)
    );

    if (isAlreadyInList) {
      return;
    }

    const isAreadyChecked = checkedFiles().some((item) =>
      isSubsetOf(item, path)
    );

    const isAllChildrenChecked =
      !isAreadyChecked &&
      checkedFiles().filter(
        (item) => item.length - path.length === 1 && isSubsetOf(path, item)
      ).length === contents().length &&
      contents().length !== 0;

    if (isAreadyChecked || isAllChildrenChecked) {
      setCheckedFiles((prev) => [...prev, path]);
    }
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
            indeterminate={checkedFiles().some((item) =>
              isSubsetOf(props.folder.path as Array<string>, item)
            )}
            checked={checkedFiles().some((item) =>
              _.isEqual(item, props.folder.path as Array<string>)
            )}
            onChange={(checked: boolean) => {
              handleChange(checked, props.folder.path as Array<string>);
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
                      <FileCheckbox name={item.name} file={props.folder} />
                      {/* <Checkbox
                        checked={checkedFiles().some((checkedItem) =>
                          _.isEqual(checkedItem, [
                            ...(props.folder.path as Array<string>),
                            item.name
                          ])
                        )}
                        onChange={(checked: boolean) => {
                          handleChange(checked, [
                            ...(props.folder.path as Array<string>),
                            item.name
                          ] as Array<string>);
                        }}
                        children={<span>{item.name}</span>}
                      /> */}
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
