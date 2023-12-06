// // Necessary imports
// import { createEffect, createSignal, For, Show } from "solid-js";
// // Assuming these are your custom imports
// import { instanceId } from "@/utils/browser";
// import { rspc } from "@/utils/rspcClient";
// import { Checkbox } from "@gd/ui";

// interface Props {
//   canExpand?: boolean;
//   title?: string;
//   onChange?: (_checked: boolean) => void;
//   indeterminate?: boolean;
//   checked?: boolean;
//   file: {
//     name: string;
//     type: string;
//     // Add other necessary file properties here
//   };
// }

// const ExportCheckbox = (props: Props) => {
//   const [path, setPath] = createSignal<string[]>([]);
//   const [data, setData] = createSignal<any>({});
//   // const [opened, setOpened] = createSignal(false);
//   // const [expand, setExpand] = createSignal<{ [key: string]: boolean }>({});

//   console.log("data: ", data());
//   // Fetching data when a folder is expanded
//   createEffect(() => {
//     console.log("createEffect");
//     if (!data().isOpen && data().isexpanding) {
//       setPath((prevPath) => [...prevPath, props.file.name]);
//       fetchData();
//     }
//   });

//   // Function to fetch data from the backend
//   const fetchData = async () => {
//     console.log("fetching data");
//     const explore = rspc.createQuery(() => [
//       "instance.explore",
//       {
//         instance_id: instanceId() as number,
//         path: path()
//       }
//     ]);
//     console.log(explore.data);
//     setData({ data: explore.data, isOpen: true });
//   };

//   // Toggle folder open/close
//   const toggleFolder = () => {
//     console.log("toggleFolder");
//     setData({ ...data(), isOpen: false, isexpanding: !data().isexpanding });
//   };

//   return (
//     <div>
//       <div class="flex items-center gap-2 h-10 px-2">
//         <Show when={props.canExpand}>
//           <div
//             class={`${
//               data().isexpanding ? "i-ep:arrow-up" : "i-ep:arrow-down"
//             }`}
//             onClick={toggleFolder}
//           ></div>
//         </Show>
//         <Checkbox
//           children={<span class="text-sm">{props.title}</span>}
//           checked={props.checked}
//           onChange={props.onChange}
//           indeterminate={props.indeterminate}
//         />
//       </div>
//       <Show when={data().isexpanding}>
//         <div class="pl-4">
//           <For each={data().data}>
//             {(item) => (
//               <ExportCheckbox
//                 title={item.name}
//                 file={item}
//                 canExpand={item.type === "Directory"}
//               />
//             )}
//           </For>
//         </div>
//       </Show>
//     </div>
//   );
// };

// export default ExportCheckbox;
import { instanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";
import { Checkbox } from "@gd/ui";
import { createEffect, createSignal, For, Match, Show, Switch } from "solid-js";

// Define the structure for your files and folders
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
  console.log("name", props.folder.name);
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
          <Checkbox children={<span>{props.folder.name}</span>} />
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
                      <Checkbox children={<span>{item.name}</span>} />
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
