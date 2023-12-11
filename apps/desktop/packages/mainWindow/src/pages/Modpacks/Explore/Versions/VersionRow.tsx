import {
  CFFEFile,
  CFFEMod,
  MRFEProject,
  MRFEVersion
} from "@gd/core_module/bindings";
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
    const icon = props.isCurseforge
      ? (props.project as CFFEMod).logo?.url
      : (props.project as MRFEProject).icon_url;

    const modpack = props.isCurseforge
      ? {
          Curseforge: {
            file_id: (props.modVersion as CFFEFile).id,
            project_id: (props.modVersion as CFFEFile).modId
          }
        }
      : {
          Modrinth: {
            project_id: (props.modVersion as MRFEVersion).project_id,
            version_id: (props.modVersion as MRFEVersion).id
          }
        };

    if (icon) {
      loadIconMutation.mutate(icon);
    }
    setLoading(true);
    createInstanceMutation.mutate({
      group: defaultGroup.data || 1,
      use_loaded_icon: true,
      notes: "",
      name: props.isCurseforge
        ? (props.modVersion as CFFEFile).displayName
        : (props.project as MRFEProject).title,
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
