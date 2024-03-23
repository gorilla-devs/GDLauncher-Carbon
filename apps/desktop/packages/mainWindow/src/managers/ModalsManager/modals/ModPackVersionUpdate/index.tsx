import { Button, Dropdown, Spinner } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { rspc } from "@/utils/rspcClient";
import { instanceId } from "@/utils/browser";
import { Show, createEffect, createSignal } from "solid-js";
import { FEInstanceId, Modpack } from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";
import { useTransContext } from "@gd/i18n";

const ModPackVersionUpdate = (props: ModalProps) => {
  const [t] = useTransContext();
  const [currentPlatform, setCurrentPlatform] = createSignal<
    "modrinth" | "curseforge" | null
  >(null);
  const [selectedVersion, setSelectedVersion] = createSignal<string | null>(
    null
  );
  const navigate = useGDNavigate();
  const modalContext = useModal();
  const instance = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId() as number]
  }));

  const changeModpackMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.changeModpack"]
  }));

  const getProjectId = () => {
    const modpack = instance.data?.modpack?.modpack;
    if (modpack) {
      if (modpack.type === "curseforge") {
        setCurrentPlatform("curseforge");
        return {
          projectId: modpack.value.project_id,
          fileId: modpack.value.file_id
        };
      } else {
        setCurrentPlatform("modrinth");
        return {
          projectId: modpack.value.project_id,
          fileId: modpack.value.version_id
        };
      }
    }

    return undefined;
  };

  createEffect(() => {
    if (!selectedVersion()) {
      setSelectedVersion(getProjectId()?.fileId?.toString() || "");
    }
  });

  const responseCF = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.curseforge.getModFiles",
      {
        modId: getProjectId()?.projectId as number,
        query: {
          pageSize: 300
        }
      }
    ],
    enabled: false
  }));

  const responseModrinth = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.modrinth.getProjectVersions",
      {
        project_id: getProjectId()?.projectId.toString()!
      }
    ],
    enabled: false
  }));

  createEffect(() => {
    if (currentPlatform() === "curseforge") {
      responseCF.refetch();
    } else if (currentPlatform() === "modrinth") {
      responseModrinth.refetch();
    }
  });

  const response = () =>
    currentPlatform() === "curseforge" ? responseCF : responseModrinth;

  const options = () => {
    if (currentPlatform() === "curseforge") {
      return (
        responseCF.data?.data.map((file) => ({
          label: (
            <div class="flex justify-between w-full">
              <span>{file.displayName}</span>
              <Show when={file.id === getProjectId()?.fileId}>
                <span class="text-green-500">{`[ Current ]`}</span>
              </Show>
            </div>
          ),
          key: file.id.toString()
        })) || []
      );
    }

    return (
      responseModrinth.data?.map((file) => ({
        label: (
          <div class="flex justify-between w-full">
            <span>{file.name}</span>
            <Show when={file.id === getProjectId()?.fileId}>
              <span class="text-green-500">{`[ Current ]`}</span>
            </Show>
          </div>
        ),
        key: file.id.toString()
      })) || []
    );
  };

  const handleUpdate = async () => {
    if (!selectedVersion()) return;

    await changeModpackMutation.mutateAsync({
      instance: instanceId() as FEInstanceId,
      modpack: {
        type: currentPlatform(),
        value:
          currentPlatform() === "curseforge"
            ? {
                project_id: getProjectId()?.projectId as number,
                file_id: parseInt(selectedVersion()!)
              }
            : {
                project_id: getProjectId()?.projectId.toString() as string,
                version_id: selectedVersion()
              }
      } as Modpack
    });

    modalContext?.closeModal();
    navigate("/library");
  };

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
      scrollable="overflow-y-scroll scrollbar-hide"
    >
      <div class="flex flex-col p-4 w-120 gap-4">
        <Show when={response().isLoading || instance.isLoading}>
          <Spinner />
        </Show>
        <Show when={!response().isLoading && !instance.isLoading}>
          <Dropdown
            class="bg-darkSlate-800 w-full"
            options={options()}
            value={selectedVersion()}
            onChange={(option) => {
              setSelectedVersion(option.key.toString());
            }}
          />

          <div class="flex justify-between">
            <Button
              type="outline"
              onClick={() => {
                modalContext?.closeModal();
              }}
            >
              {t("instance.cancel_export")}
            </Button>
            <Button
              type="primary"
              onClick={handleUpdate}
              disabled={!selectedVersion()}
            >
              {t("instance.instance_modal_instance_update")}
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  );
};

export default ModPackVersionUpdate;
