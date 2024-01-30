import { Button, Dropdown, Select } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { rspc } from "@/utils/rspcClient";
import { instanceId } from "@/utils/browser";
import { Show, createSignal } from "solid-js";
import {
  CurseforgeModpack,
  FEInstanceId,
  Modpack,
  ModrinthModpack
} from "@gd/core_module/bindings";

const ModPackVersionUpdate = (props: ModalProps) => {
  const [currentPlatform, setCurrentPlatform] = createSignal<string>("");
  const [selectedVersion, setSelectedVersion] = createSignal<string>("");
  const modalContext = useModal();
  const instance = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    instanceId() as number
  ]);
  const changeModpackMutation = rspc.createMutation(
    ["instance.changeModpack"],
    {
      onSuccess(taskId) {
        modalContext?.closeModal();
      }
    }
  );
  const getProjectId = () => {
    const modpack = instance.data?.modpack?.modpack;
    if (modpack) {
      if ("Curseforge" in modpack) {
        const curseforgeModpack: CurseforgeModpack = modpack.Curseforge;
        setCurrentPlatform("curseforge");
        return {
          projectId: curseforgeModpack.project_id,
          fileId: curseforgeModpack.file_id
        };
      } else {
        const modrinthModpack: ModrinthModpack = modpack.Modrinth;
        setCurrentPlatform("modrinth");
        return {
          projectId: modrinthModpack.project_id,
          fileId: modrinthModpack.version_id
        };
      }
    }
    return undefined;
  };
  const response = rspc.createQuery(() => [
    "modplatforms.curseforge.getModFiles",
    {
      modId: getProjectId()?.projectId as number,
      query: {
        pageSize: 300
      }
    }
  ]);
  const handleUpdate = () => {
    const obj = {
      instance: instanceId() as FEInstanceId,
      modpack:
        currentPlatform() === "curseforge"
          ? { Curseforge: JSON.parse(selectedVersion()) }
          : { Modrinth: JSON.parse(selectedVersion()) }
    };
    console.log(obj);
    changeModpackMutation.mutate(obj);
  };
  console.log(response.data);
  console.log(instance.data);
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
      scrollable="overflow-y-scroll scrollbar-hide"
    >
      <div class="flex flex-col p-4 w-120 gap-4">
        <Show when={response.isLoading || instance.isLoading}>loading ...</Show>
        <Show when={!response.isLoading && !instance.isLoading}>
          <Dropdown
            class="bg-darkSlate-800 w-full"
            options={
              response.data?.data.map((file) => ({
                label: (
                  <div class="flex justify-between w-full">
                    <span>{file.displayName}</span>
                    <Show when={file.id === getProjectId()?.fileId}>
                      <span class="text-green-500">[ Current ]</span>
                    </Show>
                  </div>
                ),
                key:
                  currentPlatform() === "curseforge"
                    ? JSON.stringify({
                        file_id: file.id,
                        project_id: file.modId
                      })
                    : JSON.stringify({
                        version_id: file.id,
                        project_id: file.modId
                      })
              })) || []
            }
            onChange={(option) => {
              setSelectedVersion(option.key as string);
            }}
          />

          <div class="flex justify-between">
            <Button
              type="outline"
              onClick={() => {
                modalContext?.closeModal();
              }}
            >
              Cancel
            </Button>
            <Button type="primary" onClick={handleUpdate}>
              Update
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  );
};

export default ModPackVersionUpdate;
