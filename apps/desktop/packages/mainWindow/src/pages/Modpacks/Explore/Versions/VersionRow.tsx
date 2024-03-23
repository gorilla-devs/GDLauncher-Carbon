import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";
import { createNotification } from "@gd/ui";
import { createSignal } from "solid-js";
import RowContainer, { Props } from "@/components/Browser/RowContainer";
import { Modpack } from "@gd/core_module/bindings";

const VersionRow = (props: Props) => {
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const loadIconMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.loadIconUrl"]
  }));

  const defaultGroup = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }));

  const [loading, setLoading] = createSignal(false);

  const prepareInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.prepareInstance"]
  }));

  const createInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.createInstance"]
  }));

  const onPrimaryAction = async () => {
    if (props.modVersion.mainThumbnail) {
      loadIconMutation.mutate(props.modVersion.mainThumbnail);
    }

    setLoading(true);
    try {
      const instanceId = await createInstanceMutation.mutateAsync({
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

      await prepareInstanceMutation.mutateAsync(instanceId);

      setLoading(true);
      addNotification("Instance successfully created.");
    } catch (err) {
      setLoading(false);
      addNotification("Error while downloading the modpack.", "error");
    } finally {
      setLoading(false);
      navigate(`/library`);
    }
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
