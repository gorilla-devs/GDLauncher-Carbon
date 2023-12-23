import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";
import { createNotification } from "@gd/ui";
import { createSignal } from "solid-js";
import RowContainer, { Props } from "@/components/Browser/RowContainer";

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
    // const icon = props.isCurseforge
    //   ? props.project.logo?.url
    //   : props.project.icon_url;

    const modpack = props.isCurseforge
      ? {
          Curseforge: {
            project_id: props.modVersion.id,
            file_id: props.modVersion.fileId
          }
        }
      : {
          Modrinth: {
            project_id: props.modVersion.id,
            version_id: props.modVersion.fileId
          }
        };

    // if (icon) {
    //   loadIconMutation.mutate(icon);
    // }

    setLoading(true);
    createInstanceMutation.mutate({
      group: defaultGroup.data || 1,
      use_loaded_icon: true,
      notes: "",
      name: props.modVersion.name,
      version: {
        Modpack: modpack
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
