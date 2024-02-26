import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";
import { createNotification } from "@gd/ui";
import { createSignal } from "solid-js";
import RowContainer, { Props } from "@/components/Browser/RowContainer";
import { Modpack } from "@gd/core_module/bindings";

const VersionRow = (props: Props) => {
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const [loading, setLoading] = createSignal(false);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess(_data) {
        setLoading(true);
        addNotification("Instance successfully created.");
      },
      onError() {
        setLoading(false);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        setLoading(false);
        navigate(`/library`);
      }
    }
  );

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onSuccess(instanceId) {
        prepareInstanceMutation.mutate(instanceId);
      },
      onError() {
        addNotification("Error while downloading the modpack.", "error");
      }
    }
  );

  const onPrimaryAction = () => {
    if (props.modVersion.mainThumbnail) {
      loadIconMutation.mutate(props.modVersion.mainThumbnail);
    }

    setLoading(true);
    createInstanceMutation.mutate({
      group: defaultGroup.data || 1,
      use_loaded_icon: true,
      notes: "",
      name: props.modVersion.name,
      version: {
        Modpack: {
          type: props.isCurseforge ? "curseforge" : "modrinth",
          value: props.isCurseforge
            ? {
                project_id: parseInt(props.modVersion.id, 10),
                file_id: parseInt(props.modVersion.fileId, 10)
              }
            : {
                project_id: props.modVersion.id,
                version_id: props.modVersion.fileId
              }
        } as Modpack
      }
    });
  };

  return (
    <RowContainer
      {...props}
      loading={loading()}
      disabled={false}
      onPrimaryAction={onPrimaryAction}
    />
  );
};

export default VersionRow;
