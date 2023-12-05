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
import { set } from "date-fns";
import { createSignal, For, Show } from "solid-js";

// Define the structure for your files and folders
type FileFolder = {
  name: string;
  type: "file" | "folder";
  children?: FileFolder[];
};

const FolderDropdown = (props: { folder: FileFolder }) => {
  const [isOpen, setIsOpen] = createSignal(false);
  const [contents, setContents] = createSignal<FileFolder[]>([]);
  const [path, setPath] = createSignal<string[]>([]);

  // Mock function to fetch contents. Replace this with your actual API call.
  const fetchContents = (folderName: string) => {
    // Here you would make an API call to fetch the contents of the folder.
    // This is just a placeholder for demonstration purposes.
    const explore = rspc.createQuery(() => [
      "instance.explore",
      {
        instance_id: instanceId() as number,
        path: path()
      }
    ]);
    setPath((prevPath) => [...prevPath, folderName]);
    return explore.data;
  };

  const toggleFolder = () => {
    if (!isOpen()) {
      const fetchedContents = fetchContents(props.folder.name);
      setContents(fetchedContents as any);
    }
    setIsOpen(!isOpen());
  };

  return (
    <div>
      <div onClick={toggleFolder}>
        {props.folder.name} {isOpen() ? "▼" : "►"}
      </div>
      <Show when={isOpen()}>
        <div style={{ "margin-left": "20px" }}>
          <For each={contents()}>
            {(item) =>
              item.type === "folder" ? (
                <FolderDropdown folder={item} />
              ) : (
                <div>{item.name}</div>
              )
            }
          </For>
        </div>
      </Show>
    </div>
  );
};

export default FolderDropdown;
